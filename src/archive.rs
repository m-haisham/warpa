use std::{
    collections::HashMap,
    io::{self, BufRead, Cursor, Read, Seek, SeekFrom, Write},
};

use libflate::zlib;
use serde_pickle::{DeOptions, Value};

use crate::{index::Index, version::Version};

pub struct Archive<'a, R: Seek + BufRead> {
    pub reader: &'a mut R,

    pub key: Option<u64>,
    pub offset: u64,

    pub version: Version,
    pub indexes: HashMap<String, Index>,
}

impl<'a, R> Archive<'a, R>
where
    R: Seek + BufRead,
{
    pub fn new(reader: &'a mut R) -> Self {
        Self {
            reader,
            offset: 0,
            version: Version::V3_2,
            indexes: HashMap::new(),
            key: Some(0xDEADBEEF),
        }
    }

    pub fn from_reader(reader: &'a mut R) -> io::Result<Self> {
        let mut version = String::new();
        reader.by_ref().take(7).read_to_string(&mut version)?;

        let version = Version::identify("", &version).ok_or(io::Error::new(
            io::ErrorKind::Other,
            "Cannot identify archive version",
        ))?;

        let (offset, key, indexes) = Self::metadata(reader, &version)?;

        Ok(Self {
            reader,
            offset,
            version,
            indexes,
            key,
        })
    }

    pub fn metadata<'b>(
        reader: &'b mut R,
        version: &Version,
    ) -> io::Result<(u64, Option<u64>, HashMap<String, Index>)> {
        let mut first_line = String::new();
        reader.read_line(&mut first_line)?;

        let metadata = first_line[..(first_line.len() - 1)]
            .split(" ")
            .collect::<Vec<_>>();

        let offset = u64::from_str_radix(metadata[1], 16)
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "Failed to parse index offset."))?;

        let key = match version {
            Version::V3_0 => {
                let mut key = 0;
                for subkey in &metadata[2..] {
                    key ^= u64::from_str_radix(subkey, 16).map_err(|_| {
                        io::Error::new(io::ErrorKind::Other, "Failed to parse key.")
                    })?;
                }
                Some(key)
            }
            Version::V3_2 => {
                let mut key = 0;
                for subkey in &metadata[3..] {
                    key ^= u64::from_str_radix(subkey, 16).map_err(|_| {
                        io::Error::new(io::ErrorKind::Other, "Failed to parse key.")
                    })?;
                }
                Some(key)
            }
            _ => None,
        };

        // Retrieve indexes.
        reader.seek(SeekFrom::Start(offset))?;
        let mut contents = Vec::new();
        reader.read_to_end(&mut contents)?;

        // Decode indexes data.
        let mut decoder = zlib::Decoder::new(Cursor::new(contents))?;
        let mut contents = Vec::new();
        io::copy(&mut decoder, &mut contents)?;

        // Deserialize indexes using pickle.
        let options = DeOptions::default();
        let raw_indexes: HashMap<String, Value> = serde_pickle::from_slice(&contents[..], options)
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "Failed to deserialize indexes."))?;

        // Map indexes to an easier format.
        let mut indexes = HashMap::new();
        for (path, value) in raw_indexes.into_iter() {
            let value = Index::from_value(value, key)
                .map_err(|_| io::Error::new(io::ErrorKind::Other, "Failed to format indexes."))?;

            indexes.insert(path, value);
        }

        Ok((offset, key, indexes))
    }
}

impl<'a, R> Archive<'a, R>
where
    R: Seek + BufRead,
{
    pub fn copy_file<W: Write>(&mut self, path: &str, writer: &mut W) -> io::Result<u64> {
        let index = self.indexes.get(path).ok_or(io::Error::new(
            io::ErrorKind::NotFound,
            "File not found in archive.",
        ))?;

        let mut scope = index.scope(&mut self.reader)?;
        io::copy(&mut scope, writer)
    }
}
