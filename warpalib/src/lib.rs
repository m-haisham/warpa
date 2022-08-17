#![warn(missing_docs)]

//! An unopiniated api for parsing renpy archives.

mod archive;
mod content;
mod error;
mod record;
mod version;

pub use archive::RenpyArchive;
pub use content::Content;
pub use error::{RpaError, RpaResult};
pub use record::Record;
pub use version::RpaVersion;
