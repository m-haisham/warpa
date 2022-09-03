use std::{fmt::Display, str::FromStr};

use warpalib::RpaVersion;

/// Defines archive versions that support write.
#[derive(Debug)]
pub enum WriteVersion {
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

impl Default for WriteVersion {
    fn default() -> Self {
        WriteVersion::V3
    }
}

impl Into<RpaVersion> for &WriteVersion {
    fn into(self) -> RpaVersion {
        match self {
            WriteVersion::V3 => RpaVersion::V3_0,
            WriteVersion::V2 => RpaVersion::V2_0,
        }
    }
}

impl Into<RpaVersion> for WriteVersion {
    #[inline]
    fn into(self) -> RpaVersion {
        (&self).into()
    }
}

/// Defines where an operation must be relative to.
#[derive(Debug)]
pub enum RelativeTo {
    Archive,
    Current,
}

impl FromStr for RelativeTo {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "archive" => Ok(RelativeTo::Archive),
            "current" => Ok(RelativeTo::Current),
            _ => Err(format!("unrecognised relative format '{s}'.")),
        }
    }
}
