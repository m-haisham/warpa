use std::fmt::Display;

use log::{info, trace};

use crate::{RpaError, RpaResult};

/// Represents archive versions.
///
/// # Examples
///
/// ```rust
/// use warpalib::RpaVersion;
///
/// // Identify version from file_name ("") and header ("RPA-3.0")
/// let version = RpaVersion::identify("", "RPA-3.0");
///
/// assert_eq!(Some(RpaVersion::V3_0), version);
/// ```
#[derive(Clone, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum RpaVersion {
    /// Represents v3.2
    V3_2,

    /// Represents v3.0
    V3_0,

    /// Represents v2.0
    V2_0,

    /// Represents v1.0
    V1_0,
}

impl RpaVersion {
    /// Identify version from header string and file name.
    ///
    /// # Versions supported
    ///
    /// | Version | Header  | filename |
    /// | :-----: | :-----: | :------: |
    /// | V3_2    | RPA-3.2 | *        |
    /// | V3_0    | RPA-3.0 | *        |
    /// | V2_0    | RPA-2.0 | *        |
    /// | V1_0    | *       | *.rpi    |
    ///
    /// If none of the above matches, `None` is returned.
    pub fn identify(file_name: &str, version: &str) -> Option<Self> {
        trace!("Identifying version from file name ({file_name}) and identity string ({version})");

        match version {
            "RPA-3.2" => Some(Self::V3_2),
            "RPA-3.0" => Some(Self::V3_0),
            "RPA-2.0" => Some(Self::V2_0),
            _ if file_name.ends_with("rpi") => Some(Self::V1_0),
            _ => None,
        }
    }

    /// The length of the archive header for a specific version
    ///
    /// # Errors
    ///
    /// This function returns `WritingNotSupported` for v3.2 and v1.0.
    pub fn header_length(&self) -> RpaResult<usize> {
        match self {
            RpaVersion::V3_0 => Ok(34),
            RpaVersion::V2_0 => Ok(25),
            RpaVersion::V3_2 | RpaVersion::V1_0 => Err(RpaError::WritingNotSupported(self.clone())),
        }
    }
}

impl Display for RpaVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RpaVersion::V3_2 => write!(f, "v3.2"),
            RpaVersion::V3_0 => write!(f, "v3.0"),
            RpaVersion::V2_0 => write!(f, "v2.0"),
            RpaVersion::V1_0 => write!(f, "v1.0"),
        }
    }
}
