use std::{
    fs::{self, File, OpenOptions},
    io::{self, BufReader},
    path::PathBuf,
};

use rpalib::Archive;

macro_rules! debug {
    ($label:ident) => {
        println!("{} = {:?}", stringify!($label), $label)
    };
    ($label:ident = $value:expr) => {
        println!("{} = {:?}", stringify!($label), $value)
    };
}

fn main() -> io::Result<()> {
    let file = File::open("test.rpa")?;
    let mut reader = BufReader::new(file);

    let mut archive = Archive::from_reader(&mut reader)?;

    for (path, index) in archive.indexes.iter() {
        let path = PathBuf::from(path);
        match path.parent() {
            Some(parent) => {
                if !parent.exists() {
                    fs::create_dir_all(parent)?;
                }
            }
            None => (),
        }

        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(&path)?;

        println!(
            "path = {}, start = {}, length = {}",
            path.display(),
            index.start,
            index.length,
        );

        let mut scope = index.scope(&mut archive.reader)?;
        let written = io::copy(&mut scope, &mut file)?;

        debug!(written);
    }

    Ok(())
}
