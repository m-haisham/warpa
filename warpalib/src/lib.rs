#![warn(missing_docs)]

//! An unopiniated api for parsing renpy archives.
//!
//! # Installation
//!
//! ```toml
//! [dependencies]
//! warpalib = "0.1.0"
//! ```

mod archive;
mod content;
mod error;
mod record;
mod version;

#[cfg(feature = "glob")]
mod glob;

pub use archive::RenpyArchive;
pub use content::{Content, ContentMap};
pub use error::{RpaError, RpaResult};
pub use record::Record;
pub use version::RpaVersion;
