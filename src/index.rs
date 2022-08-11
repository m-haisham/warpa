use std::io::{self, Read, Seek, SeekFrom, Take, Write};

use encoding::{all::ISO_8859_1, Encoding};
use serde_pickle::Value;

use crate::{RpaError, RpaResult};

#[derive(Clone, Debug)]
pub struct Index {
    pub start: u64,
    pub length: u64,
    pub prefix: Option<String>,
}

impl Index {
    pub fn new(start: u64, length: u64, prefix: Option<String>, key: Option<u64>) -> Self {
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

    // TODO: optimize by moving values rather than borrowing vectors.
    pub fn from_value(value: Value, key: Option<u64>) -> RpaResult<Self> {
        match value {
            Value::List(values) => match values.as_slice() {
                [Value::List(values)] => match values.as_slice() {
                    [Value::I64(start), Value::I64(length)] => {
                        Ok(Self::new(*start as u64, *length as u64, None, key))
                    }
                    [Value::I64(start), Value::I64(length), Value::String(prefix)] => Ok(
                        Self::new(*start as u64, *length as u64, Some(prefix.clone()), key),
                    ),
                    _ => Err(RpaError::FormatIndex),
                },
                _ => Err(RpaError::FormatIndex),
            },
            _ => Err(RpaError::FormatIndex),
        }
    }

    pub fn into_value(&self) -> Value {
        let mut values = vec![
            Value::I64(self.start as i64),
            Value::I64(self.length as i64),
        ];

        if let Some(prefix) = self.prefix.as_ref() {
            values.push(Value::String(prefix.clone()))
        }

        Value::List(vec![Value::List(values)])
    }
}

impl Index {
    /// The actual length of the indexed file with prefix taken into account.
    fn actual_length(&self) -> u64 {
        self.length - self.prefix.as_ref().map(|v| v.len()).unwrap_or(0) as u64
    }

    /// Encode the prefix with latin1 and return the bytes.
    pub fn encoded_prefix(&self) -> RpaResult<Option<Vec<u8>>> {
        match self.prefix.as_ref() {
            Some(prefix) => match ISO_8859_1.encode(prefix, encoding::EncoderTrap::Strict) {
                Ok(bytes) => Ok(Some(bytes)),
                Err(e) => return Err(RpaError::EncodePrefix(e.to_string())),
            },
            None => Ok(None),
        }
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
        let mut scope = self.scope(reader)?;

        // Append prefix to output
        if let Some(prefix) = self.encoded_prefix()? {
            writer.write(&prefix[..])?;
        }

        Ok(io::copy(&mut scope, writer)?)
    }
}
