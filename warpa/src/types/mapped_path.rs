use std::{fmt::Display, path::PathBuf, str::FromStr};

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub struct MappedPath {
    key: PathBuf,
    value: Option<PathBuf>,
}

impl From<MappedPath> for (PathBuf, PathBuf) {
    fn from(mapped: MappedPath) -> Self {
        match mapped.value {
            Some(value) => (mapped.key, value),
            None => (mapped.key.clone(), mapped.key),
        }
    }
}

impl FromStr for MappedPath {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.split_once("=") {
            Some((k, v)) => MappedPath {
                key: PathBuf::from(k),
                value: Some(PathBuf::from(v)),
            },
            None => MappedPath {
                key: PathBuf::from(s),
                value: None,
            },
        })
    }
}

impl Display for MappedPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.key.display())?;
        if let Some(value) = self.value.as_ref() {
            write!(f, "={}", value.display())?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_parse_entry_with_value_from_target() {
        let entry = "left/path=right/path".parse::<MappedPath>().unwrap();
        let expected = MappedPath {
            key: PathBuf::from("left/path"),
            value: Some(PathBuf::from("right/path")),
        };

        assert_eq!(entry, expected);
    }

    #[test]
    fn should_parse_entry_from_string() {
        let entry = "only/path".parse::<MappedPath>().unwrap();
        let expected = MappedPath {
            key: PathBuf::from("only/path"),
            value: None,
        };

        assert_eq!(entry, expected);
    }
}
