use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::LibraryError;

pub(crate) const TEMP_MARKER: &str = ".tmp-";

static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

/// A temporary file next to its target. The file is deleted when the value drops, so each
/// error path cleans up.
#[derive(Debug)]
pub(crate) struct TempFile {
    path: PathBuf,
}

impl TempFile {
    pub(crate) fn beside(target: &Path) -> Self {
        let mut name = target.file_name().unwrap_or_default().to_owned();
        let counter = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        name.push(format!("{TEMP_MARKER}{}-{counter}", process::id()));
        Self {
            path: target.with_file_name(name),
        }
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn persist(&self, target: &Path) -> Result<(), LibraryError> {
        fs::rename(&self.path, target).map_err(LibraryError::io(target))
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        // Drop cannot return the error. A temporary file that stays is deleted by collect_garbage.
        drop(fs::remove_file(&self.path));
    }
}

pub(crate) fn sync_path(path: &Path) -> Result<(), LibraryError> {
    let file = File::options()
        .write(true)
        .open(path)
        .map_err(LibraryError::io(path))?;
    file.sync_all().map_err(LibraryError::io(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn temp_file_name_has_marker_and_differs_when_made_twice() {
        let target = Path::new("dir").join("document.toml");

        let first = TempFile::beside(&target);
        let second = TempFile::beside(&target);

        let name = first.path().file_name().unwrap().to_str().unwrap();
        assert!(name.starts_with(&format!("document.toml{TEMP_MARKER}")));
        assert_eq!(first.path().parent(), Some(Path::new("dir")));
        assert_ne!(first.path(), second.path());
    }

    #[test]
    fn temp_file_is_deleted_when_dropped() {
        let dir = tempfile::tempdir().unwrap();
        let temp_file = TempFile::beside(&dir.path().join("target"));
        fs::write(temp_file.path(), b"partial").unwrap();
        let path = temp_file.path().to_owned();

        drop(temp_file);

        assert!(!path.exists());
    }

    #[test]
    fn temp_file_replaces_target_when_persisted() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("target");
        fs::write(&target, b"old").unwrap();
        let temp_file = TempFile::beside(&target);
        fs::write(temp_file.path(), b"new").unwrap();

        temp_file.persist(&target).unwrap();
        let temp_path = temp_file.path().to_owned();
        drop(temp_file);

        assert_eq!(fs::read(&target).unwrap(), b"new");
        assert!(!temp_path.exists());
    }
}
