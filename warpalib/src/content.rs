use std::{
    fs::File,
    io::{self, Cursor, Read, Seek, Write},
    path::Path,
    rc::Rc,
};

use log::debug;

use crate::{Index, RpaResult};

#[derive(Debug)]
pub enum Content {
    Index(Index),
    File(Rc<Path>),
    Raw(Vec<u8>),
}

impl Content {
    /// Copy data from the content into the `writer`.
    ///
    /// - `File` - Data is read from the file into the writer.
    /// - `Raw` - The raw buffer is copied into the writer.
    pub fn copy_to<'a, 'r, 'w, R, W>(
        &'a self,
        reader: &'r mut R,
        writer: &'w mut W,
    ) -> RpaResult<u64>
    where
        R: Seek + Read,
        W: Write,
    {
        match self {
            Content::Index(index) => index.copy_to(reader, writer),
            Content::File(path) => {
                debug!("Copying file content: {}", path.display());

                let mut file = File::open(path)?;
                io::copy(&mut file, writer).map_err(|e| e.into())
            }
            Content::Raw(data) => {
                debug!("Copying raw content: {} bytes", data.len());

                let mut cursor = Cursor::new(data);
                io::copy(&mut cursor, writer).map_err(|e| e.into())
            }
        }
    }
}
