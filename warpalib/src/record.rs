use std::io::{self, Read, Seek, SeekFrom, Take, Write};

use log::debug;
use serde_pickle::Value;

use crate::{RpaError, RpaResult};

/// Record contains information required to read a specific
/// file from the archive.
///
/// # Examples
///
/// ```rust
/// use std::{path::Path, io::Cursor};
/// use warpalib::Record;
///
/// let mut reader = Cursor::new(vec![0u8; 2048]);
/// let mut writer = vec![];
///
/// let record = Record::new(1024, 1024, None, None);
/// record.copy_section(&mut reader, &mut writer).unwrap();
///
/// assert_eq!(1024, writer.len());
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record {
    /// Index of starting byte of data.
    pub start: u64,

    /// The length of the data (as defined by archive index table).
    pub length: u64,

    /// An optional prefix added before the data.
    pub prefix: Option<Vec<u8>>,
}

impl Record {
    /// Create an index from values by deobfuscating if necessary (key is provided).
    ///
    /// Deobfuscation is done on `start` and `length` by running xor
    /// operation with `key`.
    ///
    /// Passing deobfuscated values with keys would obfuscate them. This is used
    /// when rebuilding the index for writing to a new archive.
    pub fn new(start: u64, length: u64, prefix: Option<Vec<u8>>, key: Option<u64>) -> Self {
        let (start, length) = match key {
            Some(key) => (start ^ key, length ^ key),
            None => (start, length),
        };

        Self {
            start,
            length,
            prefix,
        }
    }

    /// Create an index from pickle value.
    ///
    /// The current implementation does not use cloning,
    /// hence is better suited for when there is a prefix
    ///
    /// # Errors
    ///
    /// This function will return [`RpaError::FormatRecord`] if the format
    /// of the value could not be recognized.
    pub fn from_value(value: Value, key: Option<u64>) -> RpaResult<Self> {
        debug!("Parsing index from value: {value:?}");

        let mut iter = match value {
            Value::List(values) => {
                let mut iter = values.into_iter();
                match iter.next() {
                    Some(Value::List(values)) => values.into_iter(),
                    _ => return Err(RpaError::FormatRecord),
                }
            }
            _ => return Err(RpaError::FormatRecord),
        };

        match (iter.next(), iter.next(), iter.next()) {
            (Some(Value::I64(start)), Some(Value::I64(length)), None) => {
                Ok(Self::new(start as u64, length as u64, None, key))
            }
            (Some(Value::I64(start)), Some(Value::I64(length)), Some(Value::Bytes(prefix))) => {
                Ok(Self::new(start as u64, length as u64, Some(prefix), key))
            }
            _ => Err(RpaError::FormatRecord),
        }
    }

    /// Consume and convert the index into a pickle value.
    pub fn into_value(self) -> Value {
        debug!(
            "Creating value from index: [{}, {}, {:?}]",
            self.start, self.length, self.prefix
        );

        let mut values = vec![
            Value::I64(self.start as i64),
            Value::I64(self.length as i64),
        ];

        if let Some(prefix) = self.prefix {
            values.push(Value::Bytes(prefix));
        }

        Value::List(vec![Value::List(values)])
    }
}

impl Record {
    /// The actual length of the indexed file.
    ///
    /// This is calculated by subtracting `prefix` length from the `length`.
    fn actual_length(&self) -> u64 {
        self.length - self.prefix.as_ref().map(|v| v.len()).unwrap_or(0) as u64
    }

    /// Return a reader with limited scope into only the data specified
    /// by this index.
    ///
    /// # Errors
    ///
    /// This function forwards errors that occur during `Seek` to `start` offset.
    pub fn scope<'i, 'r, R: Seek + Read>(
        &'i self,
        reader: &'r mut R,
    ) -> io::Result<Take<&'r mut R>> {
        reader.seek(SeekFrom::Start(self.start))?;
        let take = reader.by_ref().take(self.actual_length());
        Ok(take)
    }

    /// Copy the data specified by this record from `reader` into the `writer`.
    ///
    /// The process involves writing prefix if available and copying bytes starting
    /// from the offset `start` and writing a specific `length` of bytes to `writer`.
    ///
    /// # Errors
    ///
    /// This function will forward any errors that occur during `Seek`, `Read`, and `Write`.
    pub fn copy_section<'r, 'w, R, W>(
        &'r self,
        reader: &'r mut R,
        writer: &'w mut W,
    ) -> io::Result<u64>
    where
        R: Seek + Read,
        W: Write,
    {
        debug!(
            "Copying index bytes starting {} of length {}",
            self.start,
            self.actual_length()
        );

        let mut scope = self.scope(reader)?;

        // Append prefix to output
        if let Some(prefix) = self.prefix.as_ref() {
            debug!("Writing prefix: {} bytes", prefix.len());
            writer.write_all(&prefix[..])?;
        }

        io::copy(&mut scope, writer)
    }
}
