use std::{
    fs::File,
    io::{self, Cursor, Read, Seek, Write},
    path::Path,
    rc::Rc,
};

use log::debug;

use crate::Record;

/// Represents data stored in archive.
#[derive(Debug)]
pub enum Content {
    /// Points to a slice in archive.
    Record(Record),

    /// A file in the storage.
    File(Rc<Path>),

    /// Bytes in memory.
    Raw(Vec<u8>),
}

impl Content {
    /// Copy data from the content into the `writer`.
    ///
    /// - `Index` - Data is copied from the archive (reader).
    /// - `File` - Data is copied from the file.
    /// - `Raw` - Raw in-memory buffer is copied.
    pub fn copy_to<R, W>(&self, reader: &mut R, writer: &mut W) -> io::Result<u64>
    where
        R: Seek + Read,
        W: Write,
    {
        match self {
            Content::Record(index) => index.copy_section(reader, writer),
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
