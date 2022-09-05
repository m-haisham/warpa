use std::{num::ParseIntError, str::FromStr};

#[derive(Debug)]
pub struct HexKey(pub u64);

impl FromStr for HexKey {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        u64::from_str_radix(s, 16).map(HexKey)
    }
}
