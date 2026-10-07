use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs::{self, DirEntry};
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

#[derive(Clone, Copy, Debug)]
enum Kind {
    Temp,
    Segment(SegmentKey),
}

impl Kind {
    fn of(name: &OsStr) -> Option<Self> {
        if name.to_string_lossy().contains(TEMP_MARKER) {
            return Some(Self::Temp);
        }
        let path = Path::new(name);
        if path.extension()? != SEGMENT_EXTENSION {
            return None;
        }
        path.file_stem()?.to_str()?.parse().ok().map(Self::Segment)
    }

    fn is_expired(self, stat: &Stat) -> bool {
        let limit = match self {
            Self::Temp => STALE_TEMP_AGE,
            Self::Segment(_) => GC_MIN_AGE,
        };
        stat.age > limit
    }

    fn is_used(self, used: &HashSet<SegmentKey>) -> bool {
        matches!(self, Self::Segment(key) if used.contains(&key))
    }
}

struct Candidate {
    path: PathBuf,
    kind: Kind,
}

struct Stat {
    age: Duration,
    bytes: u64,
}

#[derive(Default)]
struct Failures {
    first: Option<LibraryError>,
}

impl Failures {
    fn keep<T>(&mut self, result: Result<T, LibraryError>) -> Option<T> {
        match result {
            Ok(value) => Some(value),
            Err(error) => {
                self.first.get_or_insert(error);
                None
            }
        }
    }
}

/// Deletes the old unused segments and the stale temporary files under the directories.
///
/// The first pass lists the candidates. The key set is read after it, and each candidate is
/// checked again just before it is deleted, so a segment that a document uses now, or that a
/// job touched in between, stays. A failure on one file or directory does not stop the pass.
pub(crate) fn collect(
    directories: &[PathBuf],
    used_keys: impl FnOnce() -> HashSet<SegmentKey>,
    now: SystemTime,
) -> Result<GcReport, LibraryError> {
    let mut failures = Failures::default();
    let mut candidates = Vec::new();
    for directory in directories {
        for entry in files_under(directory, &mut failures) {
            candidates.extend(candidate_of(&entry, now, &mut failures));
        }
    }
    let used = used_keys();
    let mut report = GcReport::default();
    for candidate in candidates.iter().filter(|c| !c.kind.is_used(&used)) {
        if let Some(bytes) = delete(candidate, now, &mut failures) {
            report.deleted_files += 1;
            report.deleted_bytes += bytes;
        }
    }
    failures.first.map_or(Ok(report), Err)
}

fn candidate_of(entry: &DirEntry, now: SystemTime, failures: &mut Failures) -> Option<Candidate> {
    let kind = Kind::of(&entry.file_name())?;
    let path = entry.path();
    let stat = stat(&path, now, failures)?;
    kind.is_expired(&stat).then_some(Candidate { path, kind })
}

fn delete(candidate: &Candidate, now: SystemTime, failures: &mut Failures) -> Option<u64> {
    let stat = stat(&candidate.path, now, failures)?;
    if !candidate.kind.is_expired(&stat) {
        return None;
    }
    let removed = present(fs::remove_file(&candidate.path));
    let removed = failures.keep(removed.map_err(LibraryError::io(&candidate.path)));
    removed.flatten().map(|()| stat.bytes)
}

fn stat(path: &Path, now: SystemTime, failures: &mut Failures) -> Option<Stat> {
    let read = read_stat(path, now).map_err(LibraryError::io(path));
    failures.keep(read).flatten()
}

fn read_stat(path: &Path, now: SystemTime) -> io::Result<Option<Stat>> {
    let Some(metadata) = present(fs::metadata(path))? else {
        return Ok(None);
    };
    let age = now.duration_since(metadata.modified()?).unwrap_or_default();
    Ok(Some(Stat {
        age,
        bytes: metadata.len(),
    }))
}

fn files_under(root: &Path, failures: &mut Failures) -> Vec<DirEntry> {
    let mut files = Vec::new();
    let mut pending = vec![root.to_owned()];
    while let Some(directory) = pending.pop() {
        for entry in entries_of(&directory, failures) {
            let kind = entry.file_type().map_err(LibraryError::io(&entry.path()));
            let Some(kind) = failures.keep(kind) else {
                continue;
            };
            if kind.is_dir() {
                pending.push(entry.path());
            } else if kind.is_file() {
                files.push(entry);
            }
        }
    }
    files
}

fn entries_of(directory: &Path, failures: &mut Failures) -> Vec<DirEntry> {
    let listing = present(fs::read_dir(directory)).map_err(LibraryError::io(directory));
    let entries = failures.keep(listing).flatten().into_iter().flatten();
    entries
        .filter_map(|entry| failures.keep(entry.map_err(LibraryError::io(directory))))
        .collect()
}

fn present<T>(result: io::Result<T>) -> io::Result<Option<T>> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use std::fs::File;

    use tempfile::TempDir;

    use super::*;

    const AGE: Duration = Duration::from_secs(7_200);

    fn now() -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(10_000_000)
    }

    fn key() -> SegmentKey {
        "a".repeat(64).parse().unwrap()
    }

    fn set_modified(path: &Path, time: SystemTime) {
        let file = File::options().write(true).open(path).unwrap();
        file.set_modified(time).unwrap();
    }

    fn old_segment() -> (TempDir, PathBuf) {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(format!("{}.wav", key()));
        fs::write(&path, "audio").unwrap();
        set_modified(&path, now() - AGE);
        (directory, path)
    }

    #[test]
    fn collect_deletes_old_segment_when_key_unused() {
        let (directory, path) = old_segment();

        let report = collect(&[directory.path().to_owned()], HashSet::new, now()).unwrap();

        assert!(!path.exists());
        assert_eq!(report.deleted_files, 1);
    }

    #[test]
    fn collect_keeps_segment_when_key_is_used_after_listing() {
        let (directory, path) = old_segment();
        let used = || HashSet::from([key()]);

        let report = collect(&[directory.path().to_owned()], used, now()).unwrap();

        assert!(path.is_file());
        assert_eq!(report, GcReport::default());
    }

    #[test]
    fn collect_keeps_segment_when_file_is_touched_after_listing() {
        let (directory, path) = old_segment();
        let touch = || {
            set_modified(&path, now());
            HashSet::new()
        };

        let report = collect(&[directory.path().to_owned()], touch, now()).unwrap();

        assert!(path.is_file());
        assert_eq!(report, GcReport::default());
    }

    #[test]
    fn collect_succeeds_when_file_vanishes_after_listing() {
        let (directory, path) = old_segment();
        let remove = || {
            fs::remove_file(&path).unwrap();
            HashSet::new()
        };

        let report = collect(&[directory.path().to_owned()], remove, now());

        assert_eq!(report.unwrap(), GcReport::default());
    }
}
