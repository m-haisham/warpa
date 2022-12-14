use std::{
    collections::{BTreeMap, HashMap},
    fs::File,
    io::{self, BufRead, BufReader, Cursor, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

use flate2::{read::ZlibDecoder, write::ZlibEncoder, Compression};
use log::{debug, trace};
use serde_pickle::{DeOptions, HashableValue, SerOptions, Value};

use crate::{record::Record, version::RpaVersion, Content, ContentMap, RpaError, RpaResult};

/// Represents a renpy archive.
///
/// This struct does not enforce in-memory or in-storage. It is left upto the
/// use where the data is stored.
///
/// # Examples
///
/// ```rust
/// use warpalib::RenpyArchive;
/// use std::{
///     io::{BufWriter, Cursor},
///     fs::File,
/// };
///
/// // Open in memory archive
/// let mut archive = RenpyArchive::new();
///
/// // Insert new data into archive
/// archive.content.insert_raw("log.txt", vec![0u8; 1024]);
///
/// // or, insert new file
/// // archive.add_file(Path::new("log.txt"));
///
/// // Write archive to a file
/// let mut writer = Cursor::new(vec![]);
/// archive.flush(&mut writer).expect("Failed to write archive");
/// ```
#[derive(Debug)]
pub struct RenpyArchive<R: Seek + BufRead> {
    /// Handle to the archive data.
    pub reader: R,

    /// Key used to encode and decode index locations.
    pub key: Option<u64>,

    /// The offset where index data is stored.
    pub offset: u64,

    /// The version of this archive.
    pub version: RpaVersion,

    /// The content present in this archive.
    pub content: ContentMap,
}

impl RenpyArchive<Cursor<Vec<u8>>> {
    /// Create a new in-memory archive without allocating to heap.
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for RenpyArchive<Cursor<Vec<u8>>> {
    fn default() -> Self {
        Self {
            reader: Cursor::new(Vec::with_capacity(0)),
            offset: 0,
            version: RpaVersion::V3_0,
            key: Some(0xDEADBEEF),
            content: Default::default(),
        }
    }
}

impl RenpyArchive<BufReader<File>> {
    /// Open archive from file.
    pub fn open(path: &Path) -> RpaResult<Self> {
        trace!("Opening archive from file: {}", path.display());

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

type MetaData = (u64, Option<u64>, ContentMap);

impl<R> RenpyArchive<R>
where
    R: Seek + BufRead,
{
    /// Open an archive from bytes.
    pub fn read(mut reader: R) -> RpaResult<Self> {
        trace!("Opening archive from reader");

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

    /// Identify version by reading header and provided filename
    pub fn version(reader: &mut R, file_name: &str) -> RpaResult<RpaVersion> {
        let mut version = String::new();
        reader.by_ref().take(7).read_to_string(&mut version)?;
        RpaVersion::identify(file_name, &version).ok_or(RpaError::IdentifyVersion)
    }

    /// Retrieve `offset`, `key`, and content indexes from the archive
    pub fn metadata(reader: &mut R, version: &RpaVersion) -> RpaResult<MetaData> {
        trace!("Parsing metadata from archive version ({version})");

        let mut first_line = String::new();
        reader.read_line(&mut first_line)?;
        debug!("Read first line: {first_line}");

        // Dont't need the newline character
        let metadata = first_line[..(first_line.len() - 1)]
            .split(' ')
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

        trace!("Commencing index retrieval");

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
            .map_err(|_| RpaError::DeserializeRecord)?;
        debug!("Deserialized index data using pickle");

        // Map indexes to an easier format.
        let mut content = HashMap::new();
        for (path, value) in raw_indexes.into_iter() {
            let value = Record::from_value(value, key)?;
            content.insert(PathBuf::from(path), Content::Record(value));
        }
        debug!("Parsed index data to struct");

        Ok((offset, key, content.into()))
    }
}

impl<R> RenpyArchive<R>
where
    R: Seek + BufRead,
{
    /// Copy content from a file in the archive to the `writer`.
    ///
    /// # Errors
    ///
    /// This function returns `NotFound` error if `path` is not present in
    /// the archive and any errors raised during the copy process.
    pub fn copy_file<W: Write>(&mut self, path: &Path, writer: &mut W) -> RpaResult<u64> {
        if let Some(content) = self.content.get(Path::new(path)) {
            return content
                .copy_to(&mut self.reader, writer)
                .map_err(|e| e.into());
        }

        Err(RpaError::NotFound(path.to_path_buf()))
    }
}

impl<R> RenpyArchive<R>
where
    R: Seek + BufRead,
{
    /// Consume and write the archive to the `writer`.
    ///
    /// The archive is consumed as this rebuilds the indexes and reorgenizes the
    /// stored data.
    ///
    /// This function defers control of data flow by not enforcing that archive
    /// or writer be in-memory. This means that both archive and writer could be
    /// both a file and the program would use minimal memory since they wont be
    /// loaded into memory.
    ///
    /// # Warnings
    ///
    /// Take care not to write to the same archive as being read from.
    pub fn flush<W: Seek + Write>(mut self, writer: &mut W) -> RpaResult<()> {
        trace!("Commencing archive flush");

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
        trace!("Rebuilding indexes from content");
        let mut indexes = HashMap::new();

        // Copy data from content.
        for (path, content) in self.content.into_iter() {
            let length = content.copy_to(&mut self.reader, writer)?;
            let path = path.as_os_str().to_string_lossy().to_string();
            debug!("Written content from path ({path}) length ({length} bytes)",);

            indexes.insert(path, Record::new(offset, length, None, self.key));
            offset += length;
        }

        {
            trace!("Preparing to write indexes");

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
                Err(_) => Err(RpaError::SerializeRecord),
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
        trace!("Rewinding and writing archive header");
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
            writer.write_all(&header)?;
            debug!("Written header ({} bytes) key ({})", header.len(), key);
        }

        // And done.
        writer.flush()?;
        debug!("Done writing archive");

        Ok(())
    }
}
