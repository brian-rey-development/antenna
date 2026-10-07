use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use antenna_core::{Quality, SampleRate};
use antenna_library::{DocumentId, GcReport, Library, LibraryError, SegmentKey, text_hash};
use tempfile::TempDir;

use super::support::{at, document_dir, key, library, plain, voice};

const HOUR: Duration = Duration::from_secs(3_600);

fn now() -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(10_000_000)
}

fn set_age(path: &Path, age: Duration) {
    let file = File::options().write(true).open(path).unwrap();
    file.set_modified(now() - age).unwrap();
}

fn store_segment(library: &Library, key: SegmentKey, age: Duration) -> PathBuf {
    let mut writer = library
        .segments()
        .writer(key, SampleRate::HZ_24000)
        .unwrap();
    writer.write(&[0.5; 100]).unwrap();
    writer.commit().unwrap();
    let path = library.segments().path(&key);
    set_age(&path, age);
    path
}

fn aged_file(directory: &Path, name: &str, age: Duration) -> PathBuf {
    let path = directory.join(name);
    fs::write(&path, "partial").unwrap();
    set_age(&path, age);
    path
}

fn library_with_document() -> (TempDir, Library, DocumentId) {
    let (root, library) = library();
    let id = library.create("Notes", &plain("Hello"), at(100)).unwrap();
    (root, library, id)
}

fn record_keys(library: &Library, id: DocumentId, keys: Vec<SegmentKey>) {
    let alba = voice("en-alba", Quality::Balanced);
    library.set_voice(id, alba.clone(), at(200)).unwrap();
    library
        .record_segments(id, text_hash(&plain("Hello")), &alba, keys)
        .unwrap();
}

#[test]
fn gc_deletes_temp_files_when_older_than_one_hour() {
    let (root, library, id) = library_with_document();
    let directory = document_dir(&root, id);
    let old = aged_file(&directory, "document.toml.tmp-1-0", HOUR * 2);
    let young = aged_file(&directory, "text.txt.tmp-1-1", HOUR / 2);
    let exact = aged_file(&directory, "text.txt.tmp-1-2", HOUR);
    let old_segment = aged_file(&root.path().join("segments"), "a.wav.tmp-2-0", HOUR * 3);

    let report = library.collect_garbage(now()).unwrap();

    assert!(!old.exists() && !old_segment.exists());
    assert!(young.is_file() && exact.is_file());
    assert!(directory.join("document.toml").is_file());
    assert_eq!(report.deleted_files, 2);
}

#[test]
fn gc_deletes_only_unused_old_segments() {
    let (_root, library, id) = library_with_document();
    record_keys(&library, id, vec![key('1')]);
    let old_used = store_segment(&library, key('1'), HOUR * 2);
    let old_unused = store_segment(&library, key('2'), HOUR * 2);
    let young_unused = store_segment(&library, key('3'), HOUR / 2);
    let exact_unused = store_segment(&library, key('4'), HOUR);
    let size = fs::metadata(&old_unused).unwrap().len();

    let report = library.collect_garbage(now()).unwrap();

    assert!(old_used.is_file() && young_unused.is_file() && exact_unused.is_file());
    assert!(!old_unused.exists());
    assert_eq!(
        report,
        GcReport {
            deleted_files: 1,
            deleted_bytes: size
        }
    );
}

#[test]
fn gc_deletes_segments_of_deleted_document_when_old() {
    let (_root, library, id) = library_with_document();
    record_keys(&library, id, vec![key('1')]);
    let segment = store_segment(&library, key('1'), HOUR * 2);
    library.delete(id).unwrap();
    let was_present = segment.is_file();

    library.collect_garbage(now()).unwrap();

    assert!(was_present);
    assert!(!segment.exists());
}

#[test]
fn gc_keeps_files_when_not_segments() {
    let (root, library, _id) = library_with_document();
    let segments = root.path().join("segments");
    let other = aged_file(&segments, "notes.wav", HOUR * 5);
    let readme = aged_file(&segments, "README", HOUR * 5);

    let report = library.collect_garbage(now()).unwrap();

    assert!(other.is_file() && readme.is_file());
    assert_eq!(report, GcReport::default());
}

#[test]
fn gc_keeps_segment_when_modified_after_now() {
    let (_root, library, _id) = library_with_document();
    let segment = store_segment(&library, key('1'), Duration::ZERO);
    File::options()
        .write(true)
        .open(&segment)
        .unwrap()
        .set_modified(now() + HOUR)
        .unwrap();

    library.collect_garbage(now()).unwrap();

    assert!(segment.is_file());
}

#[cfg(unix)]
#[test]
fn gc_continues_and_reports_first_error_when_directory_unreadable() {
    use std::fs::Permissions;
    use std::os::unix::fs::PermissionsExt;

    let (root, library, _id) = library_with_document();
    let old_segment = store_segment(&library, key('1'), HOUR * 2);
    let old_temp = aged_file(&root.path().join("library"), "text.txt.tmp-1-0", HOUR * 2);
    let locked = root.path().join("segments").join("ff");
    fs::create_dir(&locked).unwrap();
    fs::set_permissions(&locked, Permissions::from_mode(0o000)).unwrap();

    let result = library.collect_garbage(now());
    fs::set_permissions(&locked, Permissions::from_mode(0o755)).unwrap();

    assert!(matches!(&result, Err(LibraryError::Io { path, .. }) if *path == locked));
    assert!(!old_segment.exists() && !old_temp.exists());
}
