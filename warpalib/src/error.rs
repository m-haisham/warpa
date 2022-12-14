use std::{io, path::PathBuf};

use thiserror::Error;

use crate::RpaVersion;

/// Type alias for a result with an `RpaError`.
pub type RpaResult<T> = Result<T, RpaError>;

/// Represents errors that the library can raise.
#[derive(Error, Debug)]
pub enum RpaError {
    /// Wraps [`io::Error`].
    #[error("{0}")]
    Io(#[from] io::Error),

    /// Wraps [`glob::PatternError`]
    #[cfg(feature = "glob")]
    #[error("{0}")]
    GlobPattern(#[from] glob::PatternError),

    /// Failed to identify archive version.
    #[error("failed to identify archive version")]
    IdentifyVersion,

    /// Failed to parse index offset.
    #[error("failed to parse index offset")]
    ParseOffset,

    /// Failed to parse index obfuscation key.
    #[error("failed to parse index deobfuscation key")]
    ParseKey,

    /// File not found in dexes or content.
    #[error("file not found in indexes or content: '{0}'")]
    NotFound(PathBuf),

    /// Creating archive not supported for a specific version.
    #[error("writing archive not supported for {0}")]
    WritingNotSupported(RpaVersion),

    /// Failed to serialize archive index.
    #[error("failed to serialize archive index")]
    SerializeRecord,

    /// Failed to deserialize archive index.
    #[error("failed to deserialize archive index")]
    DeserializeRecord,

    /// Failed to format archive index.
    #[error("failed to format archive index")]
    FormatRecord,
}
