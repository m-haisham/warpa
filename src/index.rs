use std::io::{self, Read, Seek, SeekFrom, Take};

use encoding::{all::ISO_8859_1, Encoding};
use serde_pickle::Value;

#[derive(Clone, Debug)]
pub struct Index {
    pub start: u64,
    pub length: u64,
    pub prefix: Option<String>,
}

#[derive(Debug)]
pub struct InvalidPickleFormat;

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

    pub fn from_value(value: Value, key: Option<u64>) -> Result<Self, InvalidPickleFormat> {
        let values = match value {
            Value::List(values) => match &values[..] {
                [Value::List(values)] => Ok(values),
                _ => Err(InvalidPickleFormat),
            },
            _ => Err(InvalidPickleFormat),
        }?;

        match values[..] {
            [Value::I64(start), Value::I64(length)] => {
                Ok(Self::new(start as u64, length as u64, None, key))
            }
            [Value::I64(start), Value::I64(length), Value::String(prefix)] => {
                Ok(Self::new(start as u64, length as u64, Some(prefix), key))
            }
            _ => Err(InvalidPickleFormat),
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
    /// Actual length of index by considering prefix.
    fn capacity(&self) -> u64 {
        self.length - self.prefix.as_ref().map(|v| v.len()).unwrap_or(0) as u64
    }

    pub fn scope<'i, 'r, R: Seek + Read>(
        &'i self,
        reader: &'r mut R,
    ) -> io::Result<Take<&'r mut R>> {
        reader.seek(SeekFrom::Start(self.start))?;

        let take = reader.by_ref().take(self.capacity());
        Ok(take)
    }

    pub fn encoded_prefix(&self) -> io::Result<Option<Vec<u8>>> {
        match self.prefix.as_ref() {
            Some(prefix) => match ISO_8859_1.encode(prefix, encoding::EncoderTrap::Strict) {
                Ok(bytes) => Ok(Some(bytes)),
                Err(e) => return Err(io::Error::new(io::ErrorKind::Other, e)),
            },
            None => Ok(None),
        }
    }
}
