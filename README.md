# Warpa

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

### Library

The following example shows how to open an archive and update it.

```rust
// Open a new archive.
let mut archive = RenpyArchive::new();

// Or, open a file.
let mut archive = RenpyArchive::open("archive.rpa")?

// Add a file.
let path = Rc::from(Path::new("file.txt"));
archive.content.insert(Rc::clone(path), Content::File(path));

// Add raw bytes.
let path = Rc::from(Path::new("raw.txt"));
let bytes = vec![];
archive.content.insert(path, Content::Raw(bytes));

// Remove content.
archive.content.remove(&Path::new("existing.txt"));

// Save the current archive state.
let mut new_archive = File::create("file")?;
archive.flush(&mut new_archive)?;
```

More [examples](warpalib/examples) in warpalib.

## License

This tool and library is licensed under [MIT License](LICENSE).

## Disclaimer

This tool is intended for use with files on which the authors allow modification of and/or extraction. Unpermitted use on files where such consent was not given is highly discouraged.
