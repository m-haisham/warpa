use std::{
    collections::HashMap,
    fs::{self, File},
    io::{BufRead, Seek},
    mem,
    path::{Path, PathBuf},
    process::exit,
    rc::Rc,
    str::FromStr,
};

use clap::{Parser, Subcommand};
use glob::{glob, Pattern};
use log::{error, info};
use rayon::prelude::*;
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
    Add {
        /// Path to archive.
        path: PathBuf,

        /// Files to be added.
        files: Vec<PathBuf>,

        /// Add files matching this pattern.
        #[clap(short, long)]
        pattern: Option<String>,
    },

    /// Extract files with full paths
    Extract {
        /// Paths to archives to extract.
        archives: Vec<PathBuf>,

        /// Root output directory. The default is current directory.
        #[clap(short, long)]
        out: Option<PathBuf>,

        /// Extract files matching the given glob pattern
        #[clap(short, long)]
        pattern: Option<String>,
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

        /// Remove files matching this glob pattern.
        #[clap(short, long)]
        pattern: Option<String>,

        /// Keep files matching the pattern.
        #[clap(short, long)]
        keep: bool,
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
        Command::Add {
            path,
            files,
            pattern,
        } => {
            fn add_files<R: Seek + BufRead>(
                path: &Path,
                files: Vec<PathBuf>,
                pattern: Option<String>,
                mut archive: RenpyArchive<R>,
                temp_path: &Path,
                key: Option<u64>,
            ) -> RpaResult<()> {
                // Override key.
                if let Some(key) = key {
                    archive.key = Some(key);
                }

                // Add manual specified files.
                for file in files {
                    info!("Adding {}", file.display());
                    archive.add_file(&file);
                }

                // Add glob pattern specified files.
                if let Some(pattern) = pattern {
                    for file in glob(&pattern)? {
                        let file = file.expect("Failed glob iteration");
                        info!("Adding {}", file.display());
                        archive.add_file(&file);
                    }
                }

                // Write and replace archive.
                replace_archive(archive, path, temp_path)?;

                Ok(())
            }

            temp_scope(&path, |temp_path| {
                if path.exists() && path.is_file() {
                    let archive = RenpyArchive::open(&path)?;
                    add_files(&path, files, pattern, archive, temp_path, args.key)
                } else if path.exists() {
                    io_error!("Expected an archive or empty path: {}", path.display())
                } else {
                    let archive = RenpyArchive::new();
                    add_files(&path, files, pattern, archive, temp_path, args.key)
                }
            })
        }
        Command::Extract {
            archives: paths,
            out,
            pattern,
        } => {
            let out = out.unwrap_or_default();

            paths
                .into_par_iter()
                .map(|path| {
                    let mut archive = RenpyArchive::open(&path)?;

                    let iter: Box<dyn Iterator<Item = (&Rc<Path>, &Content)>> = match &pattern {
                        Some(pattern) => Box::new(archive.content.glob(&pattern)?),
                        None => Box::new(archive.content.iter()),
                    };

                    for (output, content) in iter {
                        info!("Extracting {}", output.display());

                        let output = out.join(output);
                        if let Some(parent) = output.parent() {
                            if !parent.exists() {
                                fs::create_dir_all(parent)?;
                            }
                        }

                        let mut file = File::create(output)?;
                        content.copy_to(&mut archive.reader, &mut file)?;
                    }

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
            pattern,
            keep,
        } => {
            let mut archive = RenpyArchive::open(&path)?;
            if let Some(key) = args.key {
                archive.key = Some(key);
            }

            for file in files {
                info!("Removing {}", file.display());
                if archive.content.remove(file.as_path()).is_none() {
                    return io_error!("File not found in archive: '{}'", file.display());
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
                            info!("Removing {}", path.display());
                        }
                        keep
                    })
                    .collect::<HashMap<_, _>>()
                    .into();
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
