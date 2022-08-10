use std::path::PathBuf;

pub enum Content {
    File { path: PathBuf },
    Bytes { path: PathBuf, data: Vec<u8> },
}
