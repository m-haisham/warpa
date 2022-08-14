use std::{
    fs::{self, File},
    io::{BufRead, Cursor, Seek},
    path::{Path, PathBuf},
    process::exit,
};

use clap::{Parser, Subcommand};
use log::error;
use memmap::MmapOptions;
use rayon::prelude::*;
use simplelog::{ColorChoice, Config, LevelFilter, TermLogger};
use std::io;
use warpalib::{RenpyArchive, RpaError, RpaResult};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    /// Provide additional information (default only shows errors).
    #[clap(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// The encryption key used for creating v3 archives (default=0xDEADBEEF).
    #[clap(short, long)]
    key: Option<u64>,

    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Add files to existing or new archive
    Add {
        /// Path to archive.
        path: PathBuf,

        /// Files to be added.
        files: Vec<PathBuf>,
    },

    /// Extract files with full paths
    Extract {
        /// Paths to archives to extract.
        archives: Vec<PathBuf>,

        /// Root output directory. The default is current directory.
        #[clap(short, long)]
        out: Option<PathBuf>,
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
    },
}

macro_rules! io_error {
    ($($arg:tt)*) => {
        Err(RpaError::Io(io::Error::new(io::ErrorKind::Other, format!($($arg)+))))
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

fn run(args: Cli) -> Result<(), RpaError> {
    match args.command {
        Command::Add { path, files } => {
            fn add_files<R: Seek + BufRead>(
                path: &Path,
                mut archive: RenpyArchive<R>,
                files: Vec<PathBuf>,
                temp_path: &Path,
                key: Option<u64>,
            ) -> RpaResult<()> {
                if let Some(key) = key {
                    archive.key = Some(key);
                }

                for file in files {
                    archive.add_file(file);
                }

                replace_archive(archive, path, temp_path)?;
                Ok(())
            }

            temp_scope(&path, |temp| {
                if path.exists() && path.is_file() {
                    let archive = RenpyArchive::open(&path)?;
                    add_files(&path, archive, files, temp, args.key)
                } else if path.exists() {
                    io_error!("Expected an archive or empty path: {}", path.display())
                } else {
                    add_files(&path, RenpyArchive::new(), files, temp, args.key)
                }
            })
        }
        Command::Extract {
            archives: paths,
            out,
        } => {
            let out = out.unwrap_or_else(|| PathBuf::new());

            paths
                .into_par_iter()
                .map(|path| {
                    let file = File::open(path)?;
                    let reader = unsafe { MmapOptions::new().map(&file)? };
                    let archive = RenpyArchive::read(Cursor::new(reader))?;

                    archive
                        .content
                        .into_par_iter()
                        .map_init(
                            || Cursor::new(archive.reader.get_ref()),
                            |mut reader, (path, content)| {
                                let output = out.join(path);
                                if let Some(parent) = output.parent() {
                                    if !parent.exists() {
                                        fs::create_dir_all(parent)?;
                                    }
                                }

                                let mut file = File::create(output)?;
                                content.copy_to(&mut reader, &mut file)?;
                                Ok(())
                            },
                        )
                        .collect::<RpaResult<Vec<()>>>()?;

                    Ok(())
                })
                .collect::<RpaResult<Vec<()>>>()
                .map(|_| ())
        }
        Command::List { archive } => {
            let archive = RenpyArchive::open(&archive)?;

            for path in archive.content.keys() {
                println!("{}", path.display());
            }

            Ok(())
        }
        Command::Remove {
            archive: path,
            files,
        } => {
            let mut archive = RenpyArchive::open(&path)?;
            if let Some(key) = args.key {
                archive.key = Some(key);
            }

            for file in files {
                if let None = archive.content.remove(file.as_path()) {
                    return io_error!("File not found in archive: '{}'", file.display());
                }
            }

            temp_scope(&path, |temp_path| {
                replace_archive(archive, &path, temp_path)
            })
        }
    }
}

fn replace_archive<R: Seek + BufRead>(
    archive: RenpyArchive<R>,
    path: &Path,
    temp_path: &Path,
) -> RpaResult<()> {
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
        fs::remove_file(&temp_path)?;
    }

    result
}
