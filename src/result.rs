use std::io;

use encoding::codec::error;
use thiserror::Error;

use crate::Version;

pub type RpaResult<T> = Result<T, RpaError>;

#[derive(Error, Debug)]
pub enum RpaError {
    #[error("{0}")]
    Io(#[from] io::Error),

    #[error("failed to identify archive version")]
    IdentifyVersion,

    #[error("failed to parse index offset")]
    ParseOffset,

    #[error("failed to parse index deobfuscation key")]
    ParseKey,

    #[error("file not found in indexes or content: '{0}'")]
    NotFound(String),

    #[error("writing archive not supported for version {0}")]
    WritingNotSupported(Version),

    #[error("failed to serialize archive index")]
    SerializeIndex,

    #[error("failed to deserialize archive index")]
    DeserializeIndex,

    #[error("failed to format archive index")]
    FormatIndex,

    #[error("failed to encode prefix to latin1")]
    EncodePrefix(String),
}
