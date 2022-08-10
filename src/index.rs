use std::io::{self, Read, Seek, SeekFrom, Take};

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
    pub fn new(data: (u64, u64), key: Option<u64>) -> Self {
        let (start, length) = match key {
            Some(key) => (data.0 ^ key, data.0 ^ key),
            None => data,
        };

        Self {
            start,
            length,
            prefix: None,
        }
    }

    pub fn from_value(value: Value, key: Option<u64>) -> Result<Self, InvalidPickleFormat> {
        let data = match value {
            Value::List(values) => match &values[..] {
                [Value::List(values)] => match values[..] {
                    [Value::I64(start), Value::I64(end)] => Ok((start as u64, end as u64)),
                    _ => Err(InvalidPickleFormat),
                },
                _ => Err(InvalidPickleFormat),
            },
            _ => Err(InvalidPickleFormat),
        }?;

        Ok(Self::new(data, key))
    }
}

impl Index {
    pub fn scope<'i, 'r, R: Seek + Read>(
        &'i self,
        reader: &'r mut R,
    ) -> io::Result<Take<&'r mut R>> {
        reader.seek(SeekFrom::Start(self.start))?;
        let take = reader.by_ref().take(self.length);
        Ok(take)
    }
}
