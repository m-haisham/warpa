use glob::{Pattern, PatternError};
use std::{path::Path, rc::Rc, str::FromStr};

use crate::{Content, ContentMap};

impl ContentMap {
    /// Return an iterator that produces all the contents in the archive
    /// that match the given pattern.
    ///
    /// # Errors
    ///
    /// This may return an error if the pattern is invalid.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::path::Path;
    /// use warpalib::RenpyArchive;
    ///
    /// // Create a new archive and add sample files.
    /// let mut archive = RenpyArchive::new();
    /// archive.add_raw(Path::new("silk.png"), vec![]);
    /// archive.add_raw(Path::new("cherry.png"), vec![]);
    /// archive.add_raw(Path::new("yucca.jpg"), vec![]);
    ///
    /// // Retrieve files with png extension.
    /// let paths = archive.content
    ///     .glob("*.png")
    ///     .expect("Failed to compile pattern")
    ///     .map(|(path, _)| path.as_ref())
    ///     .collect::<Vec<_>>();
    ///
    /// assert!(paths.contains(&Path::new("silk.png")));
    /// assert!(paths.contains(&Path::new("cherry.png")));
    /// ```
    pub fn glob(
        &self,
        pattern: &str,
    ) -> Result<impl Iterator<Item = (&Rc<Path>, &Content)>, PatternError> {
        let pattern = Pattern::from_str(pattern)?;

        let iter = self
            .iter()
            .filter(move |(path, _)| pattern.matches_path(path));

        Ok(iter)
    }
}
