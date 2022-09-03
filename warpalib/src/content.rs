use std::{
    collections::{hash_map, HashMap},
    fs::File,
    io::{self, Cursor, Read, Seek, Write},
    ops::{Deref, DerefMut},
    path::PathBuf,
};

use log::debug;

use crate::Record;

/// Represents contents of an archive mapped to their path
#[derive(Default, Debug)]
pub struct ContentMap(HashMap<PathBuf, Content>);

impl From<HashMap<PathBuf, Content>> for ContentMap {
    fn from(value: HashMap<PathBuf, Content>) -> Self {
        ContentMap(value)
    }
}

impl Deref for ContentMap {
    type Target = HashMap<PathBuf, Content>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ContentMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl IntoIterator for ContentMap {
    type Item = (PathBuf, Content);

    type IntoIter = hash_map::IntoIter<PathBuf, Content>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl ContentMap {
    /// Add a file to the archive. The file will be indexed in the
    /// archive with the same path.
    ///
    /// The data is not written into the archive until `flush` is called.
    pub fn insert_file<P>(&mut self, path: P) -> Option<Content>
    where
        P: Into<PathBuf>,
    {
        fn inner(map: &mut ContentMap, path: PathBuf) -> Option<Content> {
            map.0.insert(path.clone(), Content::File(path))
        }
        inner(self, path.into())
    }

    /// Add a file to the archive. The file will be indexed by `archive_path` in the archive
    /// with `file_path` used to track in filesystem.
    ///
    /// Use [`insert_file`] when adding files that have the same relative path in archive and
    /// in filesystem.
    ///
    /// The data is not written into the archive until `flush` is called.
    pub fn insert_file_mapped<P>(&mut self, archive_path: P, file_path: P) -> Option<Content>
    where
        P: Into<PathBuf>,
    {
        fn inner(map: &mut ContentMap, key: PathBuf, value: PathBuf) -> Option<Content> {
            map.0.insert(key, Content::File(value))
        }
        inner(self, archive_path.into(), file_path.into())
    }

    /// Add raw bytes to archive.
    ///
    /// The data is not written into the archive until `flush` is called.
    pub fn insert_raw<P>(&mut self, path: P, bytes: Vec<u8>) -> Option<Content>
    where
        P: Into<PathBuf>,
    {
        fn inner(map: &mut ContentMap, path: PathBuf, bytes: Vec<u8>) -> Option<Content> {
            map.0.insert(path, Content::Raw(bytes))
        }
        inner(self, path.into(), bytes)
    }
}

/// Represents data stored in archive.
#[derive(Debug)]
pub enum Content {
    /// Points to a slice in archive.
    Record(Record),

    /// A file in the storage.
    File(PathBuf),

    /// Bytes in memory.
    Raw(Vec<u8>),
}

impl Content {
    /// Copy data from the content into the `writer`.
    ///
    /// - `Record` - Data is copied from the archive (reader).
    /// - `File` - Data is copied from the file.
    /// - `Raw` - Raw in-memory buffer is copied.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::io::Cursor;
    /// use warpalib::Content;
    ///    
    /// let bytes = vec![25u8; 256];
    /// let content = Content::Raw(bytes.clone());
    ///
    /// let mut reader = Cursor::new(Vec::new());
    /// let mut buffer = vec![];
    ///
    /// content.copy_to(&mut reader, &mut buffer)
    ///     .expect("Failed to copy content to buffer.");
    ///
    /// assert_eq!(bytes, buffer);
    /// ```
    pub fn copy_to<R, W>(&self, reader: &mut R, writer: &mut W) -> io::Result<u64>
    where
        R: Seek + Read,
        W: Write,
    {
        match self {
            Content::Record(record) => record.copy_section(reader, writer),
            Content::File(path) => {
                debug!("Copying file content: {}", path.display());

                let mut file = File::open(path)?;
                io::copy(&mut file, writer)
            }
            Content::Raw(data) => {
                debug!("Copying raw content: {} bytes", data.len());

                let mut cursor = Cursor::new(data);
                io::copy(&mut cursor, writer)
            }
        }
    }
}
