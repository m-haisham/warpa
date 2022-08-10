use std::{
    fs::File,
    io::{self, Cursor, Write},
    path::Path,
    rc::Rc,
};

#[derive(Debug)]
pub struct Content {
    pub path: Rc<Path>,
    pub kind: ContentKind,
}

#[derive(Debug)]
pub enum ContentKind {
    File,
    Raw { data: Vec<u8> },
}

impl Content {
    pub fn copy_to<W: Write>(&self, writer: &mut W) -> io::Result<u64> {
        match &self.kind {
            ContentKind::File => {
                let mut file = File::open(&self.path)?;
                io::copy(&mut file, writer)
            }
            ContentKind::Raw { data } => {
                let mut cursor = Cursor::new(data);
                io::copy(&mut cursor, writer)
            }
        }
    }
}
