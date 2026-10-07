use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::LibraryError;

pub(crate) const TEMP_MARKER: &str = ".tmp-";

// A process can hold two stores or libraries, so the process id alone does not make a name unique.
static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

/// A temporary file next to its target. The file is deleted when the value drops, so each
/// error path cleans up.
#[derive(Debug)]
pub(crate) struct TempFile {
    directory: PathBuf,
    target: PathBuf,
    path: PathBuf,
}

impl TempFile {
    pub(crate) fn new(directory: &Path, file_name: &str) -> Self {
        let counter = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let temp_name = format!("{file_name}{TEMP_MARKER}{}-{counter}", process::id());
        Self {
            directory: directory.to_owned(),
            target: directory.join(file_name),
            path: directory.join(temp_name),
        }
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn target(&self) -> &Path {
        &self.target
    }

    pub(crate) fn persist(&self) -> Result<(), LibraryError> {
        fs::rename(&self.path, &self.target).map_err(LibraryError::io(&self.target))?;
        sync_parent(&self.directory)
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        // Drop cannot return the error. A temporary file that stays is deleted by collect_garbage.
        drop(fs::remove_file(&self.path));
    }
}

pub(crate) fn write(directory: &Path, file_name: &str, bytes: &[u8]) -> Result<(), LibraryError> {
    let temp_file = TempFile::new(directory, file_name);
    write_synced(temp_file.path(), bytes).map_err(LibraryError::io(temp_file.path()))?;
    temp_file.persist()
}

fn write_synced(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = File::create(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

/// Makes a rename or a new entry in `directory` durable. The file sync alone does not, because
/// a crash can lose the directory entry of a synced file.
pub(crate) fn sync_parent(directory: &Path) -> Result<(), LibraryError> {
    // Windows cannot open a directory as a file, so the call does nothing there.
    if !cfg!(unix) {
        return Ok(());
    }
    File::open(directory)
        .and_then(|handle| handle.sync_all())
        .map_err(LibraryError::io(directory))
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
        let directory = Path::new("dir");

        let first = TempFile::new(directory, "document.toml");
        let second = TempFile::new(directory, "document.toml");

        let name = first.path().file_name().unwrap().to_str().unwrap();
        assert!(name.starts_with(&format!("document.toml{TEMP_MARKER}")));
        assert_eq!(first.path().parent(), Some(directory));
        assert_eq!(first.target(), directory.join("document.toml"));
        assert_ne!(first.path(), second.path());
    }

    #[test]
    fn temp_file_is_deleted_when_dropped() {
        let dir = tempfile::tempdir().unwrap();
        let temp_file = TempFile::new(dir.path(), "target");
        fs::write(temp_file.path(), b"partial").unwrap();
        let path = temp_file.path().to_owned();

        drop(temp_file);

        assert!(!path.exists());
    }

    #[test]
    fn temp_file_replaces_target_when_persisted() {
        let dir = tempfile::tempdir().unwrap();
        let temp_file = TempFile::new(dir.path(), "target");
        fs::write(temp_file.target(), b"old").unwrap();
        fs::write(temp_file.path(), b"new").unwrap();

        temp_file.persist().unwrap();
        let temp_path = temp_file.path().to_owned();
        let target = temp_file.target().to_owned();
        drop(temp_file);

        assert_eq!(fs::read(&target).unwrap(), b"new");
        assert!(!temp_path.exists());
    }

    #[test]
    fn write_replaces_target_and_leaves_no_temp_file() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("document.toml");
        fs::write(&target, b"old").unwrap();

        write(dir.path(), "document.toml", b"new").unwrap();

        assert_eq!(fs::read(&target).unwrap(), b"new");
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[test]
    fn write_fails_when_directory_missing() {
        let dir = tempfile::tempdir().unwrap();

        let result = write(&dir.path().join("missing"), "document.toml", b"new");

        assert!(matches!(result, Err(LibraryError::Io { .. })));
    }

    #[test]
    fn sync_parent_succeeds_when_directory_exists() {
        let dir = tempfile::tempdir().unwrap();

        let result = sync_parent(dir.path());

        assert!(result.is_ok());
    }

    #[cfg(unix)]
    #[test]
    fn sync_parent_fails_when_directory_missing() {
        let dir = tempfile::tempdir().unwrap();

        let result = sync_parent(&dir.path().join("missing"));

        assert!(matches!(result, Err(LibraryError::Io { .. })));
    }
}
