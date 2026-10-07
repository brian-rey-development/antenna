use std::collections::HashSet;
use std::fs;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use crate::atomic::TEMP_MARKER;
use crate::paths::SEGMENT_EXTENSION;
use crate::{LibraryError, SegmentKey};

const STALE_TEMP_AGE: Duration = Duration::from_secs(3_600);
const GC_MIN_AGE: Duration = Duration::from_secs(3_600);

/// The files that a garbage collection deleted.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GcReport {
    /// The number of deleted files.
    pub deleted_files: u32,
    /// The size of the deleted files.
    pub deleted_bytes: u64,
}

pub(crate) fn collect(
    directories: &[PathBuf],
    used_keys: impl FnOnce() -> HashSet<SegmentKey>,
    now: SystemTime,
) -> Result<GcReport, LibraryError> {
    let mut files = Vec::new();
    for directory in directories {
        files.extend(files_under(directory)?);
    }
    let used = used_keys();
    let mut report = GcReport::default();
    for path in files {
        let deleted = delete_if_garbage(&path, &used, now).map_err(LibraryError::io(&path))?;
        if let Some(bytes) = deleted {
            report.deleted_files += 1;
            report.deleted_bytes += bytes;
        }
    }
    Ok(report)
}

fn files_under(root: &Path) -> Result<Vec<PathBuf>, LibraryError> {
    let mut files = Vec::new();
    let mut pending = vec![root.to_owned()];
    while let Some(directory) = pending.pop() {
        let listing = present(fs::read_dir(&directory)).map_err(LibraryError::io(&directory))?;
        let Some(entries) = listing else {
            continue;
        };
        for entry in entries {
            let entry = entry.map_err(LibraryError::io(&directory))?;
            let kind = entry.file_type().map_err(LibraryError::io(&entry.path()))?;
            if kind.is_dir() {
                pending.push(entry.path());
            } else if kind.is_file() {
                files.push(entry.path());
            }
        }
    }
    Ok(files)
}

fn delete_if_garbage(
    path: &Path,
    used: &HashSet<SegmentKey>,
    now: SystemTime,
) -> io::Result<Option<u64>> {
    let Some(metadata) = present(fs::metadata(path))? else {
        return Ok(None);
    };
    let age = now.duration_since(metadata.modified()?).unwrap_or_default();
    if !is_garbage(path, used, age) {
        return Ok(None);
    }
    Ok(present(fs::remove_file(path))?.map(|()| metadata.len()))
}

fn is_garbage(path: &Path, used: &HashSet<SegmentKey>, age: Duration) -> bool {
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    if name.contains(TEMP_MARKER) {
        return age > STALE_TEMP_AGE;
    }
    age > GC_MIN_AGE && is_unused_segment(path, used)
}

fn is_unused_segment(path: &Path, used: &HashSet<SegmentKey>) -> bool {
    let is_wav = path
        .extension()
        .is_some_and(|extension| extension == SEGMENT_EXTENSION);
    let key = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .and_then(|stem| stem.parse::<SegmentKey>().ok());
    is_wav && key.is_some_and(|key| !used.contains(&key))
}

fn present<T>(result: io::Result<T>) -> io::Result<Option<T>> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}
