use std::{
    collections::{BTreeMap, HashMap},
    fs::File,
    io::{self, BufRead, BufReader, Cursor, Read, Seek, SeekFrom, Write},
    path::Path,
    rc::Rc,
};

use flate2::{read::ZlibDecoder, write::ZlibEncoder, Compression};
use serde_pickle::{DeOptions, HashableValue, SerOptions, Value};

use crate::{content::Content, index::Index, version::RpaVersion, RpaError, RpaResult};

#[derive(Debug)]
pub struct RenpyArchive<R: Seek + BufRead> {
    pub reader: R,

    pub key: Option<u64>,
    pub offset: u64,

    pub version: RpaVersion,
    pub indexes: HashMap<String, Index>,
    pub content: HashMap<Rc<Path>, Content>,
}

impl RenpyArchive<Cursor<Vec<u8>>> {
    pub fn new() -> Self {
        Self {
            reader: Cursor::new(Vec::new()),
            offset: 0,
            version: RpaVersion::V3_0,
            indexes: HashMap::new(),
            key: Some(0xDEADBEEF),
            content: HashMap::new(),
        }
    }
}

impl RenpyArchive<BufReader<File>> {
    /// Open archive from file.
    pub fn open(path: &Path) -> RpaResult<Self> {
        Self::read(BufReader::new(File::open(path)?))
    }
}

impl<R> RenpyArchive<R>
where
    R: Seek + BufRead,
{
    pub fn read(mut reader: R) -> RpaResult<Self> {
        let mut version = String::new();
        reader.by_ref().take(7).read_to_string(&mut version)?;

        // FIXME: Doesnt quite support version yet.
        let version = RpaVersion::identify("", &version).ok_or(RpaError::IdentifyVersion)?;

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
        version: &RpaVersion,
    ) -> RpaResult<(u64, Option<u64>, HashMap<String, Index>)> {
        let mut first_line = String::new();
        reader.read_line(&mut first_line)?;

        let metadata = first_line[..(first_line.len() - 1)]
            .split(" ")
            .collect::<Vec<_>>();

        let offset = u64::from_str_radix(metadata[1], 16).map_err(|_| RpaError::ParseOffset)?;

        let key = match version {
            RpaVersion::V3_0 => {
                let mut key = 0;
                for subkey in &metadata[2..] {
                    key ^= u64::from_str_radix(subkey, 16).map_err(|_| RpaError::ParseKey)?;
                }
                Some(key)
            }
            RpaVersion::V3_2 => {
                let mut key = 0;
                for subkey in &metadata[3..] {
                    key ^= u64::from_str_radix(subkey, 16).map_err(|_| RpaError::ParseKey)?;
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
        let mut decoder = ZlibDecoder::new(Cursor::new(contents));
        let mut contents = Vec::new();
        io::copy(&mut decoder, &mut contents)?;

        // Deserialize indexes using pickle.
        let options = DeOptions::default();
        let raw_indexes: HashMap<String, Value> = serde_pickle::from_slice(&contents[..], options)
            .map_err(|_| RpaError::DeserializeIndex)?;

        // Map indexes to an easier format.
        let mut indexes = HashMap::new();
        for (path, value) in raw_indexes.into_iter() {
            let value = Index::from_value(value, key)?;

            indexes.insert(path, value);
        }

        Ok((offset, key, indexes))
    }
}

impl<R> RenpyArchive<R>
where
    R: Seek + BufRead,
{
    pub fn add_content(&mut self, content: Content) -> Option<Content> {
        self.content.insert(Rc::clone(&content.path), content)
    }
}

impl<R> RenpyArchive<R>
where
    R: Seek + BufRead,
{
    pub fn copy_file<W: Write>(&mut self, path: &str, writer: &mut W) -> RpaResult<u64> {
        if let Some(index) = self.indexes.get(path) {
            return index.copy_to(&mut self.reader, writer);
        };

        if let Some(content) = self.content.get(Path::new(path)) {
            return content.copy_to(writer);
        }

        Err(RpaError::NotFound(path.to_string()))
    }
}

impl<R> RenpyArchive<R>
where
    R: Seek + BufRead,
{
    pub fn flush<W: Seek + Write>(mut self, writer: &mut W) -> RpaResult<FlushResult> {
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

            indexes.insert(path, Index::new(offset, length, None, self.key));
            offset += length;
        }

        // Copy data from content.
        for (path, content) in self.content.into_iter() {
            let length = content.copy_to(writer)?;
            let path = path.as_os_str().to_string_lossy().to_string();

            indexes.insert(path, Index::new(offset, length, None, self.key));
            offset += length;
        }

        {
            // Convert indexes into serializable values.
            let values = Value::Dict(BTreeMap::from_iter(
                indexes
                    .iter()
                    .map(|(k, v)| (HashableValue::String(k.clone()), v.into_value())),
            ));

            // Serialize indexes with picke protocol 2.
            let mut buffer = Vec::new();
            let options = SerOptions::new().proto_v2();
            match serde_pickle::value_to_writer(&mut buffer, &values, options) {
                Ok(_) => Ok(()),
                Err(serde_pickle::Error::Io(e)) => Err(RpaError::Io(e)),
                Err(_) => Err(RpaError::SerializeIndex),
            }?;

            // Compress serialized data with zlib.
            let mut input = Cursor::new(buffer);
            let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
            io::copy(&mut input, &mut encoder)?;

            // Write compressed data to writer.
            let compressed = encoder.finish()?;
            let mut cursor = Cursor::new(compressed);
            io::copy(&mut cursor, writer)?;
        }

        // Back to start, time to write the header.
        writer.rewind()?;

        let key = self.key.unwrap_or(0);
        let header = match self.version {
            RpaVersion::V3_0 => format!("RPA-3.0 {:016x} {:08x}\n", offset, key),
            RpaVersion::V2_0 => format!("RPA-2.0 {:016x}\n", offset),
            v @ (RpaVersion::V3_2 | RpaVersion::V1_0) => {
                return Err(RpaError::WritingNotSupported(v))
            }
        };
        writer.write(&header.into_bytes())?;

        // And done.
        writer.flush()?;

        Ok(FlushResult {
            key: self.key,
            offset,
            version: self.version,
            indexes,
        })
    }
}

pub struct FlushResult {
    key: Option<u64>,
    offset: u64,
    version: RpaVersion,
    indexes: HashMap<String, Index>,
}

impl FlushResult {
    pub fn into_archive<R>(self, reader: R) -> RenpyArchive<R>
    where
        R: Seek + BufRead,
    {
        RenpyArchive {
            reader,
            key: self.key,
            offset: self.offset,
            version: self.version,
            indexes: self.indexes,
            content: HashMap::new(),
        }
    }
}
