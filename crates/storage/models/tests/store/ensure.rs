use std::fs::{self, File, OpenOptions};
use std::sync::atomic::{AtomicBool, Ordering};

use antenna_core::{Artifact, Extent};
use antenna_models::{DownloadProgress, ModelError};

use crate::fixture::{Fixture, NOT_CANCELLED, ignore_progress};
use crate::support::{leak, pattern, whole};

const BLOCK_BYTES: usize = 65_536;
const FILE_BYTES: usize = 10 * BLOCK_BYTES;
const WRONG_HASH: &str = "0000000000000000000000000000000000000000000000000000000000000000";

fn published(fixture: &Fixture, key: &str, len: usize) -> (&'static Artifact, Vec<u8>) {
    let bytes = pattern(len);
    let artifact = whole(key, &bytes);
    fixture.publish(artifact, &bytes);
    (artifact, bytes)
}

#[test]
fn ensure_installs_artifact_when_source_has_it() {
    let fixture = Fixture::new();
    let (artifact, bytes) = published(&fixture, "weights", FILE_BYTES);

    let files = fixture.ensure(&[artifact]).unwrap();

    assert_eq!(fs::read(files.path("weights").unwrap()).unwrap(), bytes);
    assert!(fixture.store.has_artifact(artifact));
}

#[test]
fn ensure_creates_root_when_directory_missing() {
    let fixture = Fixture::new();
    let (artifact, bytes) = published(&fixture, "weights", FILE_BYTES);
    let parent = tempfile::tempdir().unwrap();
    let store = fixture.store_at(&parent.path().join("models").join("nested"));

    let files = store
        .ensure([artifact], &ignore_progress, &NOT_CANCELLED)
        .unwrap();

    assert_eq!(fs::read(files.path("weights").unwrap()).unwrap(), bytes);
}

#[test]
fn ensure_skips_fetch_when_artifact_installed() {
    let fixture = Fixture::new();
    let (artifact, bytes) = published(&fixture, "weights", FILE_BYTES);
    fixture.ensure(&[artifact]).unwrap();
    fixture.unpublish(artifact);

    let files = fixture.ensure(&[artifact]).unwrap();

    assert_eq!(fs::read(files.path("weights").unwrap()).unwrap(), bytes);
}

#[test]
fn ensure_fails_and_deletes_when_hash_differs() {
    let fixture = Fixture::new();
    let (artifact, _) = published(&fixture, "weights", FILE_BYTES);
    let wrong = leak(Artifact {
        sha256: WRONG_HASH,
        ..*artifact
    });

    let result = fixture.ensure(&[wrong]);

    let Err(ModelError::Hash {
        artifact: key,
        expected,
        actual,
    }) = result
    else {
        panic!("expected a hash error");
    };
    assert_eq!(
        (key, expected, actual.as_str()),
        ("weights", WRONG_HASH, artifact.sha256)
    );
    assert!(!fixture.file(artifact).exists());
    assert!(!fixture.partial(artifact).exists());
}

#[test]
fn ensure_fails_and_deletes_when_size_differs() {
    let fixture = Fixture::new();
    let (artifact, _) = published(&fixture, "weights", FILE_BYTES);
    let declared = leak(Artifact {
        extent: Extent::Whole {
            bytes: FILE_BYTES as u64 + 10,
        },
        ..*artifact
    });

    let result = fixture.ensure(&[declared]);

    assert!(matches!(
        result,
        Err(ModelError::Size {
            artifact: "weights",
            expected_bytes,
            actual_bytes,
        }) if expected_bytes == FILE_BYTES as u64 + 10 && actual_bytes == FILE_BYTES as u64
    ));
    assert!(!fixture.file(artifact).exists());
    assert!(!fixture.partial(artifact).exists());
}

#[test]
fn ensure_fetches_again_when_installed_file_truncated() {
    let fixture = Fixture::new();
    let (artifact, bytes) = published(&fixture, "weights", FILE_BYTES);
    fixture.ensure(&[artifact]).unwrap();
    OpenOptions::new()
        .write(true)
        .open(fixture.file(artifact))
        .unwrap()
        .set_len(10)
        .unwrap();

    let files = fixture.ensure(&[artifact]).unwrap();

    assert_eq!(fs::read(files.path("weights").unwrap()).unwrap(), bytes);
}

#[test]
fn ensure_fails_when_disk_space_insufficient() {
    let fixture = Fixture::new();
    let huge = leak(Artifact {
        extent: Extent::Whole {
            bytes: u64::MAX / 2,
        },
        ..*whole("huge", b"x")
    });

    let result = fixture.ensure(&[huge]);

    assert!(matches!(result, Err(ModelError::DiskSpace { .. })));
    assert!(!fixture.file(huge).exists());
}

#[test]
fn ensure_stops_after_one_block_when_cancelled() {
    let fixture = Fixture::new();
    let (artifact, _) = published(&fixture, "weights", FILE_BYTES);
    let cancel = AtomicBool::new(false);
    let cancel_on_progress = |_: DownloadProgress| cancel.store(true, Ordering::Relaxed);

    let result = fixture
        .store
        .ensure([artifact], &cancel_on_progress, &cancel);

    let kept = fs::metadata(fixture.partial(artifact)).unwrap().len();
    assert!(matches!(result, Err(ModelError::Cancelled)));
    assert_eq!(kept, BLOCK_BYTES as u64);
    assert!(!fixture.file(artifact).exists());
}

#[test]
fn ensure_returns_cancelled_without_partial_file_when_cancel_is_set() {
    let fixture = Fixture::new();
    let (artifact, _) = published(&fixture, "weights", FILE_BYTES);
    let cancel = AtomicBool::new(true);

    let result = fixture.store.ensure([artifact], &ignore_progress, &cancel);

    assert!(matches!(result, Err(ModelError::Cancelled)));
    assert!(!fixture.partial(artifact).exists());
}

#[test]
fn ensure_returns_cancelled_when_waiting_on_lock() {
    let fixture = Fixture::new();
    let (artifact, _) = published(&fixture, "weights", FILE_BYTES);
    fs::create_dir_all(fixture.file(artifact).parent().unwrap()).unwrap();
    let lock = File::create(fixture.lock(artifact)).unwrap();
    lock.try_lock().unwrap();
    let cancel = AtomicBool::new(true);

    let result = fixture.store.ensure([artifact], &ignore_progress, &cancel);

    assert!(matches!(result, Err(ModelError::Cancelled)));
    assert!(!fixture.partial(artifact).exists());
}

#[test]
fn ensure_fails_when_keys_collide() {
    let fixture = Fixture::new();
    let (first, _) = published(&fixture, "weights", 10);
    let second = leak(Artifact {
        path: "other.bin",
        ..*first
    });

    let result = fixture.ensure(&[first, second]);

    assert!(matches!(
        result,
        Err(ModelError::DuplicateKey { key: "weights" })
    ));
}

#[test]
fn ensure_installs_artifacts_without_descriptor() {
    let fixture = Fixture::new();
    let (first, _) = published(&fixture, "first", 10);
    let (second, _) = published(&fixture, "second", 20);
    let slice: &'static [Artifact] = Box::leak(vec![*first, *second].into_boxed_slice());

    let files = fixture
        .store
        .ensure(slice.iter(), &ignore_progress, &NOT_CANCELLED)
        .unwrap();

    assert!(files.path("first").unwrap().is_file());
    assert!(files.path("second").unwrap().is_file());
}

#[test]
fn ensure_returns_not_found_when_source_lacks_artifact() {
    let fixture = Fixture::new();
    let artifact = whole("weights", b"abc");

    let result = fixture.ensure(&[artifact]);

    assert!(matches!(
        result,
        Err(ModelError::NotFound {
            artifact: "weights"
        })
    ));
}

#[test]
fn final_path_absent_until_hash_checked() {
    let fixture = Fixture::new();
    let (artifact, _) = published(&fixture, "weights", FILE_BYTES);
    let wrong = leak(Artifact {
        sha256: WRONG_HASH,
        ..*artifact
    });
    let (sender, receiver) = flume::unbounded();
    let record = |_: DownloadProgress| sender.send(fixture.file(wrong).exists()).unwrap();

    let result = fixture.store.ensure([wrong], &record, &NOT_CANCELLED);

    let seen: Vec<bool> = receiver.try_iter().collect();
    assert!(result.is_err());
    assert!(!seen.is_empty() && seen.iter().all(|exists| !exists));
    assert!(!fixture.file(wrong).exists());
}

#[test]
fn ensure_reports_total_when_install_finishes() {
    let fixture = Fixture::new();
    let (artifact, _) = published(&fixture, "weights", FILE_BYTES);
    let (sender, receiver) = flume::unbounded();
    let record = |progress| sender.send(progress).unwrap();

    fixture
        .store
        .ensure([artifact], &record, &NOT_CANCELLED)
        .unwrap();

    let reports: Vec<DownloadProgress> = receiver.try_iter().collect();
    let total = FILE_BYTES as u64;
    assert_eq!(
        reports.last(),
        Some(&DownloadProgress {
            done_bytes: total,
            total_bytes: total
        })
    );
    assert!(reports.is_sorted_by_key(|report| report.done_bytes));
}
