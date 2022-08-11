use std::{
    fs::{self, File},
    io::{self, BufReader},
    path::PathBuf,
};

use clap::{Parser, Subcommand};
use rpalib::Archive;

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
    // Extract files with full paths
    X {
        /// Paths to archives to extract.
        paths: Vec<PathBuf>,

        /// Root output directory. The default is current directory.
        #[clap(short, long)]
        out: Option<PathBuf>,
    },

    // List contents of archive
    L {
        /// Path to archive.
        path: PathBuf,
    },
}

fn main() -> io::Result<()> {
    let args = Cli::parse();

    match args.command {
        Command::X { paths, out } => {
            let out = out.unwrap_or_else(|| PathBuf::new());

            for archive_path in paths {
                let reader = BufReader::new(File::open(archive_path)?);
                let mut archive = Archive::from_reader(reader)?;

                for (output, index) in archive.indexes.iter() {
                    let output = out.join(output);
                    if let Some(parent) = output.parent() {
                        if !parent.exists() {
                            fs::create_dir_all(parent)?;
                        }
                    }

                    let mut file = File::create(output)?;
                    index.copy_to(&mut archive.reader, &mut file)?;
                }
            }

            Ok(())
        }
        Command::L { path } => {
            let reader = BufReader::new(File::open(path)?);
            let archive = Archive::from_reader(reader)?;

            for path in archive.indexes.keys() {
                println!("{path}");
            }

            Ok(())
        }
    }
}
