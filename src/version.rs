#[derive(Debug)]
pub enum Version {
    V3_2,
    V3_0,
    V2_0,
    V1_0,
}

impl Version {
    pub fn identify(file_name: &str, version: &str) -> Option<Self> {
        match version {
            "RPA-3.2" => Some(Self::V3_2),
            "RPA-3.0" => Some(Self::V3_0),
            "RPA-2.0" => Some(Self::V2_0),
            _ if file_name.ends_with("rpi") => Some(Self::V1_0),
            _ => None,
        }
    }
}
