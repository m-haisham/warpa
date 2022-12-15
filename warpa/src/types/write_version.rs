use std::{fmt::Display, str::FromStr};

use warpalib::RpaVersion;

/// Defines archive versions that support write.
#[derive(Clone, Default, Debug)]
pub enum WriteVersion {
    #[default]
    V3,
    V2,
}

impl FromStr for WriteVersion {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "3" => Ok(WriteVersion::V3),
            "2" => Ok(WriteVersion::V2),
            _ => Err(format!(
                "'{s}' not recognized or supported as a write version."
            )),
        }
    }
}

impl Display for WriteVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WriteVersion::V3 => write!(f, "3"),
            WriteVersion::V2 => write!(f, "2"),
        }
    }
}

impl From<WriteVersion> for RpaVersion {
    fn from(version: WriteVersion) -> Self {
        match version {
            WriteVersion::V3 => RpaVersion::V3_0,
            WriteVersion::V2 => RpaVersion::V2_0,
        }
    }
}

impl From<&WriteVersion> for RpaVersion {
    fn from(version: &WriteVersion) -> Self {
        match version {
            WriteVersion::V3 => RpaVersion::V3_0,
            WriteVersion::V2 => RpaVersion::V2_0,
        }
    }
}
