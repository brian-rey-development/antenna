use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::EngineError;

/// The local paths of the artifacts of one voice at one quality.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ModelFiles(BTreeMap<&'static str, PathBuf>);

impl ModelFiles {
    /// Makes model files from a map of artifact keys to local paths.
    pub fn new(files: BTreeMap<&'static str, PathBuf>) -> Self {
        Self(files)
    }

    /// Returns the local path of an artifact.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::MissingFile`] if no path has the key.
    pub fn path(&self, key: &'static str) -> Result<&Path, EngineError> {
        self.0
            .get(key)
            .map(PathBuf::as_path)
            .ok_or(EngineError::MissingFile { key })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_files_returns_path_when_key_known() {
        let files = ModelFiles::new(BTreeMap::from([("weights", PathBuf::from("a/b.gguf"))]));

        let path = files.path("weights").unwrap();

        assert_eq!(path, Path::new("a/b.gguf"));
    }

    #[test]
    fn model_files_fails_when_key_unknown() {
        let files = ModelFiles::default();

        let result = files.path("weights");

        assert!(matches!(
            result,
            Err(EngineError::MissingFile { key: "weights" })
        ));
    }
}
