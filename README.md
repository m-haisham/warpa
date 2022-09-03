# Warpa

![Crates.io](https://img.shields.io/crates/v/warpalib)
![Crates.io](https://img.shields.io/crates/d/warpalib)
![Crates.io](https://img.shields.io/crates/l/warpalib)
![docs.rs](https://img.shields.io/docsrs/warpalib)

Warpa is a command-line tool used to create and extract from renpy archives (rpa).

The program fully supports v3.0 and v2.0 and reading v3.2.

## Features

- **Fast threaded extraction.** Extract files from multiple archives at the same time using threads. Use `-m` to enable multi-threaded extraction for a single archive by lazy reading file into memory as needed.
- **Built-in glob pattern support.** Built-in support for glob pattern matching allows adding and removing files, and extracting and updating archives using patterns.
- **Minimal memory footprint.** Warpa does not read archive into memory. It copies segments from the archive into specified location (extracting file or temporary archive depending on command).

## Install

```bash
cargo install --git https://github.com/mensch272/warpa
```

## Usage

```text
USAGE:
    warpa [OPTIONS] <SUBCOMMAND>

OPTIONS:
    -h, --help         Print help information
    -k, --key <KEY>    The encryption key used for creating v3 archives (default=0xDEADBEEF)
    -v, --verbose      Provide additional information (default only shows errors)
    -V, --version      Print version information

SUBCOMMANDS:
    add        Add files to existing or new archive
    extract    Extract files with full paths
    help       Print this message or the help of the given subcommand(s)
    list       List contents of archive
    remove     Delete files from archive
    update     Update existing archive by reading from filesystem
```

[Examples](warpalib/examples) in warpalib.

## License

This tool and library is licensed under [MIT License](LICENSE).

## Disclaimer

This tool is intended for use with files on which the authors allow modification of and/or extraction. Unpermitted use on files where such consent was not given is highly discouraged.
