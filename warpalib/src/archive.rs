use std::{
    collections::{BTreeMap, HashMap},
    fs::File,
    io::{self, BufRead, BufReader, Cursor, Read, Seek, SeekFrom, Write},
    path::Path,
    rc::Rc,
};

use flate2::{read::ZlibDecoder, write::ZlibEncoder, Compression};
use log::{debug, info};
use serde_pickle::{DeOptions, HashableValue, SerOptions, Value};

use crate::{index::Index, version::RpaVersion, Content, RpaError, RpaResult};

#[derive(Debug)]
pub struct RenpyArchive<R: Seek + BufRead> {
    pub reader: R,

    pub key: Option<u64>,
    pub offset: u64,

    pub version: RpaVersion,
    pub content: HashMap<Rc<Path>, Content>,
}

impl RenpyArchive<Cursor<Vec<u8>>> {
    pub fn new() -> Self {
        info!("Opening new empty in-memory archive");

        Self {
            reader: Cursor::new(Vec::new()),
            offset: 0,
            version: RpaVersion::V3_0,
            key: Some(0xDEADBEEF),
            content: HashMap::new(),
        }
    }
}

impl RenpyArchive<BufReader<File>> {
    /// Open archive from file.
    pub fn open(path: &Path) -> RpaResult<Self> {
        info!("Opening archive from file: {}", path.display());

        let mut reader = BufReader::new(File::open(path)?);

        let version = match path.file_name() {
            Some(name) => Self::version(&mut reader, &name.to_string_lossy())?,
            None => Self::version(&mut reader, "")?,
        };

        let (offset, key, content) = Self::metadata(&mut reader, &version)?;

        Ok(Self {
            reader,
            offset,
            version,
            key,
            content,
        })
    }
}

impl<R> RenpyArchive<R>
where
    R: Seek + BufRead,
{
    pub fn read(mut reader: R) -> RpaResult<Self> {
        info!("Opening archive from reader");

        let version = Self::version(&mut reader, "")?;
        let (offset, key, content) = Self::metadata(&mut reader, &version)?;

        Ok(Self {
            reader,
            offset,
            version,
            key,
            content,
        })
    }

    pub fn version<'r>(reader: &'r mut R, file_name: &str) -> RpaResult<RpaVersion> {
        let mut version = String::new();
        reader.by_ref().take(7).read_to_string(&mut version)?;
        RpaVersion::identify(file_name, &version).ok_or(RpaError::IdentifyVersion)
    }

    pub fn metadata<'r>(
        reader: &'r mut R,
        version: &RpaVersion,
    ) -> RpaResult<(u64, Option<u64>, HashMap<Rc<Path>, Content>)> {
        info!("Parsing metadata from archive version ({version})");

        let mut first_line = String::new();
        reader.read_line(&mut first_line)?;
        debug!("Read first line: {first_line}");

        // Dont't need the newline character
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
        debug!("Parsed the obfuscation key: {key:?}");

        info!("Retrieving indexes");

        // Retrieve indexes.
        reader.seek(SeekFrom::Start(offset))?;
        let mut contents = Vec::new();
        reader.read_to_end(&mut contents)?;
        debug!("Read raw index bytes");

        // Decode indexes data.
        let mut decoder = ZlibDecoder::new(Cursor::new(contents));
        let mut contents = Vec::new();
        io::copy(&mut decoder, &mut contents)?;
        debug!("Decoded index data with zlib");

        // Deserialize indexes using pickle.
        let options = DeOptions::default();
        let raw_indexes: HashMap<String, Value> = serde_pickle::from_slice(&contents[..], options)
            .map_err(|_| RpaError::DeserializeIndex)?;
        debug!("Deserialized index data using pickle");

        // Map indexes to an easier format.
        let mut content = HashMap::new();
        for (path, value) in raw_indexes.into_iter() {
            let value = Index::from_value(value, key)?;
            content.insert(Rc::from(Path::new(&path)), Content::Index(value));
        }
        debug!("Parsed index data to struct");

        Ok((offset, key, content))
    }
}

impl<R> RenpyArchive<R>
where
    R: Seek + BufRead,
{
    pub fn copy_file<W: Write>(&mut self, path: &Path, writer: &mut W) -> RpaResult<u64> {
        if let Some(content) = self.content.get(Path::new(path)) {
            return content.copy_to(&mut self.reader, writer);
        }

        Err(RpaError::NotFound(path.to_path_buf()))
    }
}

impl<R> RenpyArchive<R>
where
    R: Seek + BufRead,
{
    pub fn flush<W: Seek + Write>(mut self, writer: &mut W) -> RpaResult<()> {
        info!("Commencing archive flush");

        let mut offset: u64 = 0;

        // Write a placeholder header to be filled later.
        // Not using seek since writer might not have any data.
        let header_length = self.version.header_length()?;
        let header = vec![0u8; header_length];
        offset += writer.write(&header)? as u64;
        debug!(
            "Written placeholder header for version ({}) length ({} bytes)",
            self.version, header_length,
        );

        // Build indexes while writing to the archive.
        info!("Rebuilding indexes from content");
        let mut indexes = HashMap::new();

        // Copy data from content.
        for (path, content) in self.content.into_iter() {
            let length = content.copy_to(&mut self.reader, writer)?;
            let path = path.as_os_str().to_string_lossy().to_string();
            debug!("Written content from path ({path}) length ({length} bytes)",);

            indexes.insert(path, Index::new(offset, length, None, self.key));
            offset += length;
        }

        {
            info!("Preparing to write indexes");

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
                Err(serde_pickle::Error::Io(e)) => Err(RpaError::Io(e)),
                Err(_) => Err(RpaError::SerializeIndex),
            }?;
            debug!(
                "Encoded indexes using pickle format 2: {} bytes",
                buffer.len()
            );

            // Compress serialized data with zlib.
            let mut input = Cursor::new(buffer);
            let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
            io::copy(&mut input, &mut encoder)?;
            let compressed = encoder.finish()?;
            debug!("Compressed indexes using zlib: {} bytes", compressed.len());

            // Write compressed data to writer.
            let mut cursor = Cursor::new(compressed);
            io::copy(&mut cursor, writer)?;
            debug!("Done writing indexes");
        }

        // Back to start, time to write the header.
        info!("Rewinding and writing archive header");
        writer.rewind()?;

        let key = self.key.unwrap_or(0);
        let header = match self.version {
            RpaVersion::V3_0 => format!("RPA-3.0 {:016x} {:08x}\n", offset, key),
            RpaVersion::V2_0 => format!("RPA-2.0 {:016x}\n", offset),
            v @ (RpaVersion::V3_2 | RpaVersion::V1_0) => {
                return Err(RpaError::WritingNotSupported(v))
            }
        };

        {
            let header = header.into_bytes();
            writer.write(&header)?;
            debug!("Written header ({} bytes) key ({})", header.len(), key);
        }

        // And done.
        writer.flush()?;
        debug!("Done writing archive");

        Ok(())
    }
}
