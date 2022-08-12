use std::{
    fs::File,
    io::{self, Cursor, Write},
    path::Path,
    rc::Rc,
};

use crate::RpaResult;

#[derive(Debug)]
pub struct Content {
    pub path: Rc<Path>,
    pub kind: ContentKind,
}

#[derive(Debug)]
pub enum ContentKind {
    File,
    Raw(Vec<u8>),
}

impl Content {
    pub fn new(path: Rc<Path>, kind: ContentKind) -> Self {
        Content { path, kind }
    }
}

impl Content {
    /// Copy data from the content into the `writer`.
    ///
    /// * `File` - Data is read from the file into the writer.
    /// * `Raw` - The raw buffer is copied into the writer.
    pub fn copy_to<W: Write>(&self, writer: &mut W) -> RpaResult<u64> {
        Ok(match &self.kind {
            ContentKind::File => {
                let mut file = File::open(&self.path)?;
                io::copy(&mut file, writer)
            }
            ContentKind::Raw(data) => {
                let mut cursor = Cursor::new(data);
                io::copy(&mut cursor, writer)
            }
        }?)
    }
}
