use std::{
    fs::{self, File},
    io::{BufRead, Seek},
    path::{Path, PathBuf},
    rc::Rc,
};

use clap::{Parser, Subcommand};
use log::error;
use simplelog::{ColorChoice, Config, LevelFilter, TermLogger};
use std::io;
use warpalib::{Content, RenpyArchive, RpaError, RpaResult};

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
    A {
        /// Path to archive.
        path: PathBuf,

        /// Files to be added.
        files: Vec<PathBuf>,
    },

    /// Extract files with full paths
    X {
        /// Paths to archives to extract.
        archives: Vec<PathBuf>,

        /// Root output directory. The default is current directory.
        #[clap(short, long)]
        out: Option<PathBuf>,
    },

    /// List contents of archive
    L {
        /// Path to archive.
        archive: PathBuf,
    },

    /// Delete files from archive
    D {
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
    }
}

fn run(args: Cli) -> Result<(), RpaError> {
    match args.command {
        Command::A { path, files } => {
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
                    let file = Rc::from(file);
                    archive
                        .content
                        .insert(Rc::clone(&file), Content::File(file));
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
        Command::X {
            archives: paths,
            out,
        } => {
            let out = out.unwrap_or_else(|| PathBuf::new());

            for archive_path in paths {
                let mut archive = RenpyArchive::open(&archive_path)?;

                for (output, content) in archive.content.iter() {
                    let output = out.join(output);
                    if let Some(parent) = output.parent() {
                        if !parent.exists() {
                            fs::create_dir_all(parent)?;
                        }
                    }

                    let mut file = File::create(output)?;
                    content.copy_to(&mut archive.reader, &mut file)?;
                }
            }

            Ok(())
        }
        Command::L { archive } => {
            let archive = RenpyArchive::open(&archive)?;

            for path in archive.content.keys() {
                println!("{}", path.display());
            }

            Ok(())
        }
        Command::D {
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
