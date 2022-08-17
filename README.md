# Warpa

![Crates.io](https://img.shields.io/crates/v/warpalib)
![Crates.io](https://img.shields.io/crates/d/warpalib)
![Crates.io](https://img.shields.io/crates/l/warpalib)
![docs.rs](https://img.shields.io/docsrs/warpalib)

Warpa is a command-line tool used to create and extract from renpy archives (rpa).

## Support

| Version | Read | Write |
| ------- | ---- | ----- |
| 3.2     | Yes  | No    |
| 3.0     | Yes  | Yes   |
| 2.0     | Yes  | Yes   |
| 1.0     | No   | No    |

## Usage

### CLI

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
```

[Examples](warpalib/examples) in warpalib.

## License

This tool and library is licensed under [MIT License](LICENSE).

## Disclaimer

This tool is intended for use with files on which the authors allow modification of and/or extraction. Unpermitted use on files where such consent was not given is highly discouraged.
