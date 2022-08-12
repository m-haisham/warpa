use std::{
    fs::{self, File},
    io::{BufRead, Seek},
    path::{Path, PathBuf},
    rc::Rc,
};

use clap::{Parser, Subcommand};
use rpalib::{Content, RenpyArchive, RpaResult};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    #[clap(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    // Add files to archive
    A {
        /// Path to archive.
        path: PathBuf,

        /// Files to be added.
        files: Vec<PathBuf>,
    },

    // Extract files with full paths
    X {
        /// Paths to archives to extract.
        archives: Vec<PathBuf>,

        /// Root output directory. The default is current directory.
        #[clap(short, long)]
        out: Option<PathBuf>,
    },

    // List contents of archive
    L {
        /// Path to archive.
        archive: PathBuf,
    },
}

fn main() -> RpaResult<()> {
    let args = Cli::parse();

    match args.command {
        Command::A { path, files } => {
            fn add_files<R: Seek + BufRead>(
                path: PathBuf,
                mut archive: RenpyArchive<R>,
                files: Vec<PathBuf>,
                temp_path: &Path,
            ) -> RpaResult<()> {
                for file in files {
                    let file = Rc::from(file);
                    archive
                        .content
                        .insert(Rc::clone(&file), Content::File(file));
                }

                {
                    let mut temp_file = File::create(&temp_path)?;
                    archive.flush(&mut temp_file)?;
                }

                fs::rename(temp_path, path)?;
                Ok(())
            }

            let mut temp_path = path.clone();
            let mut temp_name = path.file_name().unwrap().to_os_string();
            temp_name.push(".temp");
            temp_path.set_file_name(temp_name);

            let result = if path.exists() && path.is_file() {
                let archive = RenpyArchive::open(&path)?;
                add_files(path, archive, files, &temp_path)
            } else if path.exists() {
                panic!("Expected an archive or empty path: {}", path.display());
            } else {
                add_files(path, RenpyArchive::new(), files, &temp_path)
            };

            // Delete the temporary file in case something went wrong
            if temp_path.exists() {
                fs::remove_file(temp_path)?;
            }

            result
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
    }
}
