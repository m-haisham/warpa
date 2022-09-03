use std::{
    fs::{self, File},
    io::{Cursor, Read, Seek},
    path::{Path, PathBuf},
};

use glob::Pattern;
use log::info;
use memmap2::{Advice, Mmap};
use rayon::prelude::ParallelIterator;
use warpalib::{Content, ContentMap, RenpyArchive, RpaResult};

pub struct MemArchive {
    #[allow(dead_code)]
    file: File,
    pub archive: RenpyArchive<Cursor<Mmap>>,
}

impl MemArchive {
    /// Read a file into memory and open an archive
    pub fn open(path: &Path) -> RpaResult<MemArchive> {
        let file = File::open(path)?;
        let mmap = unsafe { Mmap::map(&file)? };
        #[cfg(unix)]
        mmap.advise(Advice::WillNeed)?;

        let archive = RenpyArchive::read(Cursor::new(mmap))?;
        Ok(MemArchive { file, archive })
    }
}

pub fn filter_content<'a>(
    content: ContentMap,
    files: &'a [PathBuf],
    pattern: Option<&'a Pattern>,
) -> Box<dyn Iterator<Item = (PathBuf, Content)> + 'a> {
    match (files, pattern) {
        (f, Some(pattern)) if f.is_empty() => Box::new(
            content
                .into_iter()
                .filter(|(path, _)| pattern.matches_path(path)),
        ),
        (f, Some(pattern)) => Box::new(
            content
                .into_iter()
                .filter(|(path, _)| pattern.matches_path(path) || f.contains(path)),
        ),
        (f, None) if f.is_empty() => Box::new(content.into_iter()),
        (f, None) => Box::new(content.into_iter().filter(|(path, _)| f.contains(&path))),
    }
}

pub fn extract_archive<'a, R: Seek + Read>(
    reader: &mut R,
    content_iter: Box<dyn Iterator<Item = (PathBuf, Content)> + 'a>,
    out_dir: &Path,
) -> RpaResult<()> {
    for (output, content) in content_iter {
        extract_content(reader, &output, &content, out_dir)?;
    }

    Ok(())
}

pub fn extract_archive_threaded<'p, P>(reader: Mmap, content: P, out_dir: &Path) -> RpaResult<()>
where
    P: ParallelIterator<Item = (&'p PathBuf, &'p Content)>,
{
    content
        .map_init(
            || Cursor::new(&reader),
            |reader, (output, content)| extract_content(reader, output, content, out_dir),
        )
        .collect::<RpaResult<()>>()
}

pub fn extract_content<R: Seek + Read>(
    reader: &mut R,
    output: &Path,
    content: &Content,
    out_dir: &Path,
) -> RpaResult<()> {
    info!("Extracting {}", output.display());

    let output = out_dir.join(output);
    if let Some(parent) = output.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }

    let mut file = File::create(output)?;
    content.copy_to(reader, &mut file)?;
    Ok(())
}
