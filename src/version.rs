use std::fmt::Display;

use crate::{RpaError, RpaResult};

#[derive(Clone, Debug)]
#[repr(u8)]
pub enum RpaVersion {
    V3_2,
    V3_0,
    V2_0,
    V1_0,
}

impl RpaVersion {
    pub fn identify(file_name: &str, version: &str) -> Option<Self> {
        match version {
            "RPA-3.2" => Some(Self::V3_2),
            "RPA-3.0" => Some(Self::V3_0),
            "RPA-2.0" => Some(Self::V2_0),
            _ if file_name.ends_with("rpi") => Some(Self::V1_0),
            _ => None,
        }
    }

    pub fn header_length(&self) -> RpaResult<usize> {
        match self {
            RpaVersion::V3_0 => Ok(34),
            RpaVersion::V2_0 => Ok(25),
            v @ (RpaVersion::V3_2 | RpaVersion::V1_0) => {
                Err(RpaError::WritingNotSupported(v.clone()))
            }
        }
    }

    pub fn write_support(&self) -> bool {
        match self {
            RpaVersion::V3_2 => false,
            RpaVersion::V3_0 => true,
            RpaVersion::V2_0 => true,
            RpaVersion::V1_0 => false,
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
