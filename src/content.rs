use std::{
    fs::File,
    io::{self, Cursor, Write},
    path::Path,
    rc::Rc,
};

pub enum Content {
    File { path: Rc<Path> },
    Raw { path: Rc<Path>, data: Vec<u8> },
}

impl Content {
    pub fn copy_to<W: Write>(&self, writer: &mut W) -> io::Result<u64> {
        match self {
            Content::File { path } => {
                let mut file = File::open(path)?;
                io::copy(&mut file, writer)
            }
            Content::Raw { path, data } => {
                let mut cursor = Cursor::new(data);
                io::copy(&mut cursor, writer)
            }
        }
    }
}
