use std::{
    fs::{self, File},
    path::{Path, PathBuf},
};

use warpalib::{RenpyArchive, RpaError, RpaResult};

fn main() -> RpaResult<()> {
    // Assuming there is an archive named "archive.rpa" in current directory
    let path = Path::new("archive.rpa");
    let mut archive = RenpyArchive::open(path)?;

    // Make a change to the archive.
    archive.add_file(PathBuf::from("README.md"));

    // Saving and replacing the archive is a bit complicated since
    // we cannot read from and write to the same file at the same time
    // for data corruption reasons. So, first we write to a temp file.
    let temp_path = Path::new("archive.rpa.temp");

    // A new block is created for two reasons.
    // 1. Drop handle to temp_file.
    // 2. Capture any errors to ensure that next step is executed.
    let result = {
        // Create and open temp file.
        let mut temp_file = File::create(temp_path)?;

        // Write to the opened temp file.
        archive.flush(&mut temp_file)?;

        // Move and replace the archive with temp.
        fs::rename(temp_path, path).map_err(|e| RpaError::Io(e))
    };

    // Delete temp file if it still exists for cleanup.
    // The file may still be present if archive writing failed.
    // or, if the file move failed.
    if temp_path.exists() {
        fs::remove_file(temp_path)?;
    }

    // While delayed, we should never discard a possible error,
    result
}
