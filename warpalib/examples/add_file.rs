use std::io::Cursor;

use warpalib::{RenpyArchive, RpaResult};

fn main() -> RpaResult<()> {
    // Open an in memory archive.
    let mut archive = RenpyArchive::new();

    // Add readme into archive.
    archive.content.insert_file("README.md");

    // Write the current to a buffer.
    let mut buffer = Cursor::new(Vec::new());
    archive.flush(&mut buffer)?;

    Ok(())
}
