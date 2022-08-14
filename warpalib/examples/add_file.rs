use std::{io::Cursor, path::PathBuf};

use warpalib::{RenpyArchive, RpaResult};

fn main() -> RpaResult<()> {
    // Open an in memory archive.
    let mut archive = RenpyArchive::new();

    // Add readme into archive.
    archive.add_file(PathBuf::from("README.md"));

    // Write the current to a buffer.
    let mut buffer = Cursor::new(Vec::new());
    archive.flush(&mut buffer)?;

    Ok(())
}
