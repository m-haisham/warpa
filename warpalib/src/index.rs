use std::io::{self, Read, Seek, SeekFrom, Take, Write};

use log::debug;
use serde_pickle::Value;

use crate::{RpaError, RpaResult};

#[derive(Clone, Debug)]
pub struct Index {
    pub start: u64,
    pub length: u64,
    pub prefix: Option<Vec<u8>>,
}

impl Index {
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
    pub fn from_value(value: Value, key: Option<u64>) -> RpaResult<Self> {
        debug!("Parsing index from value: {value:?}");

        let mut iter = match value {
            Value::List(values) => {
                let mut iter = values.into_iter();
                match iter.next() {
                    Some(Value::List(values)) => values.into_iter(),
                    _ => return Err(RpaError::FormatIndex),
                }
            }
            _ => return Err(RpaError::FormatIndex),
        };

        match (iter.next(), iter.next(), iter.next()) {
            (Some(Value::I64(start)), Some(Value::I64(length)), None) => {
                Ok(Self::new(start as u64, length as u64, None, key))
            }
            (Some(Value::I64(start)), Some(Value::I64(length)), Some(Value::Bytes(prefix))) => {
                Ok(Self::new(start as u64, length as u64, Some(prefix), key))
            }
            _ => Err(RpaError::FormatIndex),
        }
    }

    pub fn into_value(&self) -> Value {
        debug!(
            "Creating value from index: [{}, {}, {:?}]",
            self.start, self.length, self.prefix
        );

        let mut values = vec![
            Value::I64(self.start as i64),
            Value::I64(self.length as i64),
        ];

        if let Some(prefix) = self.prefix.as_ref() {
            // TODO: optimize by using move.
            values.push(Value::Bytes(prefix.clone()));
        }

        Value::List(vec![Value::List(values)])
    }
}

impl Index {
    /// The actual length of the indexed file with prefix taken into account.
    fn actual_length(&self) -> u64 {
        self.length - self.prefix.as_ref().map(|v| v.len()).unwrap_or(0) as u64
    }

    /// Return a reader with limited scope into only the data specified
    /// by this index.
    pub fn scope<'i, 'r, R: Seek + Read>(
        &'i self,
        reader: &'r mut R,
    ) -> io::Result<Take<&'r mut R>> {
        reader.seek(SeekFrom::Start(self.start))?;
        let take = reader.by_ref().take(self.actual_length());
        Ok(take)
    }

    /// Copy data specified by this index into the writer.
    pub fn copy_to<'r, 'w, R, W>(&'r self, reader: &'r mut R, writer: &'w mut W) -> RpaResult<u64>
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
            writer.write(&prefix[..])?;
        }

        Ok(io::copy(&mut scope, writer)?)
    }
}
