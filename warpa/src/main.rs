mod extract;
mod types;

use std::{
    collections::HashMap,
    fs::{self, File},
    io::{BufRead, Seek},
    mem,
    path::{Path, PathBuf},
    process::exit,
    str::FromStr,
};

use clap::{Parser, Subcommand};
use extract::{extract_archive, extract_archive_threaded, filter_content, MemArchive};
use glob::{glob, Pattern};
use log::{debug, error, info, warn};
use rayon::prelude::*;
use simplelog::{ColorChoice, Config, LevelFilter, TermLogger};
use std::io;
use types::{HexKey, MappedPath, WriteVersion};
use warpalib::{Content, RenpyArchive, RpaError, RpaResult};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    /// Provide additional information (default only shows errors).
    #[clap(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// The encryption key used for creating v3 archives (default=0xDEADBEEF).
    #[clap(short, long)]
    key: Option<HexKey>,

    /// The write version of archives.
    #[clap(short, long)]
    write_version: Option<WriteVersion>,

    /// Override with default write version (3) if archive version does not support write.
    #[clap(short, long)]
    override_version: bool,

    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Add files to existing or create a new archive
    Add {
        /// Path to existing or new archive file.
        path: PathBuf,

        /// Mapped files to be added to the archive.
        files: Vec<MappedPath>,

        /// Add files matching this glob pattern.
        #[clap(short, long)]
        pattern: Option<String>,
    },

    /// Extract files with full paths
    Extract {
        /// Paths to archives to extract.
        archives: Vec<PathBuf>,

        /// Find archives using the glob pattern.
        #[clap(short, long)]
        archive_pattern: Option<String>,

        /// Root output directory. The default is parent of archive.
        #[clap(short, long)]
        out: Option<PathBuf>,

        /// Files to be extracted.
        #[clap(short, long)]
        files: Vec<PathBuf>,

        /// Extract files matching the given glob pattern
        #[clap(short, long)]
        pattern: Option<String>,

        /// Load archive into memory and read using multiple threads. This is experimental.
        #[clap(short, long)]
        memory: bool,
    },

    /// List contents of archive
    List {
        /// Path to archive.
        archive: PathBuf,
    },

    /// Delete files from archive
    Remove {
        /// Path to archive.
        archive: PathBuf,

        /// Files to be deleted
        files: Vec<PathBuf>,

        /// Remove archive files matching this glob pattern.
        #[clap(short, long)]
        pattern: Option<String>,

        /// Keep files matching the pattern.
        #[clap(short, long)]
        keep: bool,
    },

    /// Update existing archive by reading from filesystem.
    Update {
        /// Path to archive.
        archive: PathBuf,

        /// Files in archive to be updated.
        files: Vec<PathBuf>,

        /// Update archive files matching this glob pattern.
        #[clap(short, long)]
        pattern: Option<String>,

        /// Find files relative to directory. The default is archive directory.
        #[clap(short, long)]
        relative: Option<PathBuf>,
    },
}

macro_rules! io_error {
    ($($arg:tt)*) => {
        Err(RpaError::Io(io::Error::new(io::ErrorKind::Other, format!($($arg)+))))
    };
}

macro_rules! not_found {
    ($($arg:tt)*) => {
        Err(RpaError::Io(io::Error::new(io::ErrorKind::NotFound, format!($($arg)+))))
    };
}

fn main() {
    let args = Cli::parse();

    let level = match args.verbose {
        0 => LevelFilter::Error,
        1 => LevelFilter::Warn,
        2 => LevelFilter::Info,
        3 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    };

    TermLogger::init(
        level,
        Config::default(),
        simplelog::TerminalMode::Mixed,
        ColorChoice::Auto,
    )
    .unwrap();

    if let Err(e) = run(args) {
        error!("{e}");
        exit(1);
    }
}

/// A capture of config from cli.
struct CliConfig {
    pub key: Option<HexKey>,
    pub write_version: Option<WriteVersion>,
    pub override_version: bool,
}

impl CliConfig {
    fn update_archive<R: BufRead + Seek>(&self, archive: &mut RenpyArchive<R>) {
        if let Some(version) = self.write_version.as_ref() {
            archive.version = version.into()
        } else if self.override_version {
            archive.version = WriteVersion::default().into()
        }

        if let Some(key) = self.key.as_ref() {
            archive.key = Some(key.0);
        }
    }
}

fn run(args: Cli) -> Result<(), RpaError> {
    let config = CliConfig {
        key: args.key,
        write_version: args.write_version,
        override_version: args.override_version,
    };

    match args.command {
        Command::Add {
            path,
            files,
            pattern,
        } => {
            fn add_files<R: Seek + BufRead>(
                path: &Path,
                files: Vec<MappedPath>,
                pattern: Option<String>,
                mut archive: RenpyArchive<R>,
                temp_path: &Path,
            ) -> RpaResult<()> {
                // Add manual specified files.
                for file_map in files {
                    info!("Adding {}...", &file_map);
                    let (archive_path, file_path) = file_map.into();
                    let removed = archive
                        .content
                        .insert_file_mapped(archive_path.clone(), file_path);
                    if removed.is_some() {
                        warn!("Removed previous content in {}.", archive_path.display());
                    }
                }

                // Add glob pattern specified files.
                if let Some(pattern) = pattern {
                    for file in glob(&pattern)? {
                        let file = file.expect("Failed glob iteration");
                        info!("Adding {}...", file.display());
                        if archive.content.insert_file(file.clone()).is_some() {
                            warn!("Removed previous content in {}.", file.display());
                        }
                    }
                }

                // Write and replace archive.
                replace_archive(archive, path, temp_path)?;

                Ok(())
            }

            temp_scope(&path, |temp_path| {
                if path.exists() && path.is_file() {
                    let mut archive = RenpyArchive::open(&path)?;
                    config.update_archive(&mut archive);
                    add_files(&path, files, pattern, archive, temp_path)
                } else if path.exists() {
                    io_error!("Expected an archive or empty path: {}", path.display())
                } else {
                    let mut archive = RenpyArchive::new();
                    config.update_archive(&mut archive);
                    add_files(&path, files, pattern, archive, temp_path)
                }
            })
        }
        Command::Extract {
            mut archives,
            archive_pattern: archives_pattern,
            out,
            files,
            pattern,
            memory,
        } => {
            if let Some(pattern) = archives_pattern {
                info!("Adding archives from glob pattern '{}'...", pattern);
                for file in glob(&pattern)? {
                    let file = file.expect("Failed glob iteration");
                    archives.push(file);
                }
            }

            archives
                .into_par_iter()
                .map(|path| {
                    let out_dir = get_out_or_parent(out.as_ref(), &path)?;

                    let pattern = pattern
                        .as_ref()
                        .map(|s| Pattern::from_str(s))
                        .map_or(Ok(None), |r| r.map(Some))?;

                    if memory {
                        let mmap = MemArchive::open(&path)?;
                        if files.is_empty() && pattern.is_none() {
                            // Convert the map into a parralel iter skipping iter collection.
                            extract_archive_threaded(
                                mmap.archive.reader.into_inner(),
                                mmap.archive.content.par_iter(),
                                out_dir,
                            )
                        } else {
                            // Filter and collect results so parallelization will be affective.
                            let content =
                                filter_content(mmap.archive.content, &files, pattern.as_ref())
                                    .collect::<Vec<_>>();

                            extract_archive_threaded(
                                mmap.archive.reader.into_inner(),
                                content.par_iter().map(|(p, c)| (p, c)),
                                out_dir,
                            )
                        }
                    } else {
                        let mut archive = RenpyArchive::open(&path)?;
                        let content_iter =
                            filter_content(archive.content, &files, pattern.as_ref());
                        extract_archive(&mut archive.reader, content_iter, out_dir)
                    }
                })
                .collect::<RpaResult<()>>()
        }
        Command::List { archive } => {
            let archive = RenpyArchive::open(&archive)?;

            for path in archive.content.keys() {
                println!("{}", path.display());
            }

            Ok(())
        }
        Command::Remove {
            archive: archive_path,
            files,
            pattern,
            keep,
        } => {
            let mut archive = RenpyArchive::open(&archive_path)?;
            config.update_archive(&mut archive);

            for file in files {
                info!("Removing {}...", file.display());
                if archive.content.remove(file.as_path()).is_none() {
                    return io_error!("File {} not found in the archive.", file.display());
                }
            }

            if let Some(pattern_str) = pattern {
                let pattern = Pattern::from_str(&pattern_str)?;

                let content = mem::take(&mut archive.content);
                archive.content = content
                    .into_iter()
                    .filter(move |(path, _)| {
                        let keep = pattern.matches_path(path) ^ keep;
                        if !keep {
                            info!("Removing {}...", path.display());
                        }
                        keep
                    })
                    .collect::<HashMap<_, _>>()
                    .into();
            }

            temp_scope(&archive_path, |temp_path| {
                replace_archive(archive, &archive_path, temp_path)
            })
        }
        Command::Update {
            archive: archive_path,
            files,
            pattern,
            relative,
        } => {
            // Resolve the target directory and make sure its valid before reading archive.
            let dir = match relative.as_ref() {
                None => match archive_path.parent() {
                    Some(p) => p,
                    None => return not_found!("unable to access archive directory."),
                },
                Some(p) => {
                    if !p.exists() {
                        return not_found!(
                            "relative directory target not found. '{}' does not exist.",
                            p.display(),
                        );
                    } else if !p.is_dir() {
                        return not_found!(
                            "relative directory target not found. '{}' not a directory.",
                            p.display(),
                        );
                    } else {
                        p
                    }
                }
            };

            let mut archive = RenpyArchive::open(&archive_path)?;
            config.update_archive(&mut archive);

            // Update all if no specifics are defined.
            if files.is_empty() && pattern.is_none() {
                debug!("Updating all files in archive, no specifics defined.");
                archive.content = archive
                    .content
                    .into_iter()
                    .map(|(path, _)| {
                        let file = Content::File(dir.join(&path));
                        info!("Updating {}...", path.display());
                        (path, file)
                    })
                    .collect::<HashMap<_, _>>()
                    .into();
            } else {
                debug!("Updating files defined by pattern in archive.");
                if let Some(pattern) = pattern {
                    let pattern = Pattern::from_str(&pattern)?;
                    archive.content = archive
                        .content
                        .into_iter()
                        .map(|(path, content)| {
                            if pattern.matches_path(&path) {
                                info!("Updating {}...", path.display());
                                let file = Content::File(dir.join(&path));
                                (path, file)
                            } else {
                                (path, content)
                            }
                        })
                        .collect::<HashMap<_, _>>()
                        .into();
                }

                debug!("Updating files defined by path in archive.");
                for path in files {
                    match archive.content.get_mut(&path) {
                        Some(content @ Content::Record(_)) => {
                            info!("Updating {}...", path.display());
                            *content = Content::File(dir.join(path))
                        }
                        Some(_) => (),
                        None => {
                            return io_error!("File not found in archive: '{}'", path.display())
                        }
                    }
                }
            }

            temp_scope(&archive_path, |temp_path| {
                replace_archive(archive, &archive_path, temp_path)
            })
        }
    }
}

fn replace_archive<R: Seek + BufRead>(
    archive: RenpyArchive<R>,
    path: &Path,
    temp_path: &Path,
) -> RpaResult<()> {
    debug!("Replacing archive in {}.", path.display());

    {
        let mut temp_file = File::create(&temp_path)?;
        archive.flush(&mut temp_file)?;
    }

    fs::rename(temp_path, path)?;
    Ok(())
}

fn temp_scope<F>(path: &Path, f: F) -> RpaResult<()>
where
    F: FnOnce(&Path) -> RpaResult<()>,
{
    let mut temp_path = path.to_path_buf();
    let mut temp_name = path.file_name().unwrap().to_os_string();
    temp_name.push(".temp");
    temp_path.set_file_name(temp_name);

    let result = f(temp_path.as_path());

    if temp_path.exists() {
        warn!("Removing dangling {}.", temp_path.display());
        fs::remove_file(&temp_path)?;
    }

    result
}

/// Returns [out] if given or [parent_of] other path.
///
/// # Errors
///
/// Throws an [`io::ErrorKind::NotFound`] if parent is not found.
fn get_out_or_parent<'a>(out: Option<&'a PathBuf>, parent_of: &'a Path) -> io::Result<&'a Path> {
    match out {
        Some(out) => Ok(out),
        None => match parent_of.parent() {
            Some(parent) => Ok(parent),
            None => Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("parent of {} not found", parent_of.display()),
            )),
        },
    }
}
