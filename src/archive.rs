use std::{
    collections::{BTreeMap, HashMap},
    io::{self, BufRead, Cursor, Read, Seek, SeekFrom, Write},
    path::Path,
    rc::Rc,
};

use libflate::zlib::{self, Encoder};
use serde_pickle::{DeOptions, HashableValue, SerOptions, Value};

use crate::{content::Content, index::Index, version::Version};

#[derive(Debug)]
pub struct Archive<R: Seek + BufRead> {
    pub reader: R,

    pub key: Option<u64>,
    pub offset: u64,

    pub version: Version,
    pub indexes: HashMap<String, Index>,
    pub content: HashMap<Rc<Path>, Content>,
}

impl<R> Archive<R>
where
    R: Seek + BufRead,
{
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            offset: 0,
            version: Version::V3_2,
            indexes: HashMap::new(),
            key: Some(0xDEADBEEF),
            content: HashMap::new(),
        }
    }

    pub fn from_reader(mut reader: R) -> io::Result<Self> {
        let mut version = String::new();
        reader.by_ref().take(7).read_to_string(&mut version)?;

        let version = Version::identify("", &version).ok_or(io::Error::new(
            io::ErrorKind::Other,
            "Cannot identify archive version",
        ))?;

        let (offset, key, indexes) = Self::metadata(&mut reader, &version)?;

        Ok(Self {
            reader,
            offset,
            version,
            indexes,
            key,
            content: HashMap::new(),
        })
    }

    pub fn metadata<'b>(
        reader: &'b mut R,
        version: &Version,
    ) -> io::Result<(u64, Option<u64>, HashMap<String, Index>)> {
        let mut first_line = String::new();
        reader.read_line(&mut first_line)?;

        let metadata = first_line[..(first_line.len() - 1)]
            .split(" ")
            .collect::<Vec<_>>();

        let offset = u64::from_str_radix(metadata[1], 16)
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "Failed to parse index offset."))?;

        let key = match version {
            Version::V3_0 => {
                let mut key = 0;
                for subkey in &metadata[2..] {
                    key ^= u64::from_str_radix(subkey, 16).map_err(|_| {
                        io::Error::new(io::ErrorKind::Other, "Failed to parse key.")
                    })?;
                }
                Some(key)
            }
            Version::V3_2 => {
                let mut key = 0;
                for subkey in &metadata[3..] {
                    key ^= u64::from_str_radix(subkey, 16).map_err(|_| {
                        io::Error::new(io::ErrorKind::Other, "Failed to parse key.")
                    })?;
                }
                Some(key)
            }
            _ => None,
        };

        // Retrieve indexes.
        reader.seek(SeekFrom::Start(offset))?;
        let mut contents = Vec::new();
        reader.read_to_end(&mut contents)?;

        // Decode indexes data.
        let mut decoder = zlib::Decoder::new(Cursor::new(contents))?;
        let mut contents = Vec::new();
        io::copy(&mut decoder, &mut contents)?;

        // Deserialize indexes using pickle.
        let options = DeOptions::default();
        let raw_indexes: HashMap<String, Value> = serde_pickle::from_slice(&contents[..], options)
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "Failed to deserialize indexes."))?;

        // Map indexes to an easier format.
        let mut indexes = HashMap::new();
        for (path, value) in raw_indexes.into_iter() {
            let value = Index::from_value(value, key)
                .map_err(|_| io::Error::new(io::ErrorKind::Other, "Failed to format indexes."))?;

            indexes.insert(path, value);
        }

        Ok((offset, key, indexes))
    }
}

impl<R> Archive<R>
where
    R: Seek + BufRead,
{
    pub fn copy_file<W: Write>(&mut self, path: &str, writer: &mut W) -> io::Result<u64> {
        if let Some(content) = self.content.get(Path::new(path)) {
            return content.copy_to(writer);
        }

        if let Some(index) = self.indexes.get(path) {
            let mut scope = index.scope(&mut self.reader)?;
            return io::copy(&mut scope, writer);
        };

        Err(io::Error::new(
            io::ErrorKind::NotFound,
            "File not found in archive or content.",
        ))
    }
}

impl<R> Archive<R>
where
    R: Seek + BufRead,
{
    pub fn flush<W: Seek + Write>(mut self, writer: &mut W) -> io::Result<()> {
        let mut offset: u64 = 0;

        // Write a placeholder header to be filled later.
        // Not using seek since writer might not have any data.
        let header = vec![0u8; self.version.header_length()?];
        offset += writer.write(&header)? as u64;

        // Build indexes while writing to the archive.
        let mut indexes = HashMap::new();

        // Copy data from existing archive (indexes).
        for (path, index) in self.indexes.into_iter() {
            let mut scope = index.scope(&mut self.reader)?;
            let length = io::copy(&mut scope, writer)?;

            indexes.insert(path, Index::new(offset, length, self.key));
            offset += length;
        }

        // Copy data from content.
        for (path, content) in self.content.into_iter() {
            let length = content.copy_to(writer)?;
            let path = path.as_os_str().to_string_lossy().to_string();

            indexes.insert(path, Index::new(offset, length, self.key));
            offset += length;
        }

        {
            // Convert indexes into serializable values.
            let values = Value::Dict(BTreeMap::from_iter(
                indexes
                    .into_iter()
                    .map(|(k, v)| (HashableValue::String(k), v.into_value())),
            ));

            // Serialize indexes with picke protocol 2.
            let mut buffer = Vec::new();
            let options = SerOptions::new().proto_v2();
            match serde_pickle::value_to_writer(&mut buffer, &values, options) {
                Ok(_) => Ok(()),
                Err(serde_pickle::Error::Io(e)) => Err(e),
                Err(_) => Err(io::Error::new(
                    io::ErrorKind::Other,
                    "Failed to serialize archive indexes.",
                )),
            }?;

            // Compress serialized data with zlib.
            let mut input = Cursor::new(buffer);
            let mut encoder = Encoder::new(Vec::new())?;
            io::copy(&mut input, &mut encoder)?;

            // Write compressed data to writer.
            let compressed = encoder.finish().into_result()?;
            let mut cursor = Cursor::new(compressed);
            io::copy(&mut cursor, writer)?;
        }

        // Back to start, time to write the header.
        writer.rewind()?;

        let key = self.key.unwrap_or(0);
        let header = match self.version {
            Version::V3_2 => format!("RPA-3.2 {:016x} {:08x}\n", offset, key),
            Version::V3_0 => format!("RPA-3.0 {:016x} {:08x}\n", offset, key),
            Version::V2_0 => format!("RPA-2.0 {:016x}\n", offset),
            Version::V1_0 => {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "version not supported",
                ))
            }
        };
        writer.write(&header.into_bytes())?;

        // And done.
        writer.flush()?;

        Ok(())
    }
}
