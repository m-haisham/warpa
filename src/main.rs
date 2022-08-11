use std::{
    fs::File,
    io::{self, BufReader},
    path::PathBuf,
    rc::Rc,
};

use rpalib::{Archive, Content, ContentKind};

macro_rules! debug {
    ($label:ident) => {
        println!("{} = {:?}", stringify!($label), $label)
    };
    ($label:ident = $value:expr) => {
        println!("{} = {:?}", stringify!($label), $value)
    };
}

fn main() -> io::Result<()> {
    let file = File::open("test.rpa")?;
    let mut reader = BufReader::new(file);

    let mut archive = Archive::from_reader(&mut reader)?;

    {
        let path = Rc::from(PathBuf::from("audio/log.txt"));
        let content = Content::new(Rc::clone(&path), ContentKind::File);
        archive.content.insert(path, content);
    }

    let mut output = File::create("output.rpa")?;
    // let mut writer = Cursor::new(Vec::new());

    let result = archive.flush(&mut output)?;
    result.into_archive(BufReader::new(output));

    // for (path, index) in archive.indexes.iter() {
    //     let path = PathBuf::from(path);
    //     match path.parent() {
    //         Some(parent) => {
    //             if !parent.exists() {
    //                 fs::create_dir_all(parent)?;
    //             }
    //         }
    //         None => (),
    //     }

    //     let mut file = OpenOptions::new()
    //         .write(true)
    //         .truncate(true)
    //         .create(true)
    //         .open(&path)?;

    //     println!(
    //         "path = {}, start = {}, length = {}",
    //         path.display(),
    //         index.start,
    //         index.length,
    //     );

    //     let mut scope = index.scope(&mut archive.reader)?;
    //     let written = io::copy(&mut scope, &mut file)?;

    //     debug!(written);
    // }

    Ok(())
}
