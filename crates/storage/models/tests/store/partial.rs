use std::fs;

use antenna_core::Artifact;
use antenna_models::DownloadProgress;

use crate::fixture::{Fixture, NOT_CANCELLED};
use crate::support::{leak, pattern, range, whole};

const BLOCK_BYTES: usize = 65_536;
const FILE_BYTES: usize = 10 * BLOCK_BYTES;

fn published(fixture: &Fixture, key: &str, len: usize) -> (&'static Artifact, Vec<u8>) {
    let bytes = pattern(len);
    let artifact = whole(key, &bytes);
    fixture.publish(artifact, &bytes);
    (artifact, bytes)
}

#[test]
fn ensure_resumes_when_partial_file_exists_and_source_is_directory() {
    let fixture = Fixture::new();
    let (artifact, bytes) = published(&fixture, "weights", FILE_BYTES);
    fixture.write_partial(artifact, &bytes[..BLOCK_BYTES + 7]);

    let files = fixture.ensure(&[artifact]).unwrap();

    assert_eq!(fs::read(files.path("weights").unwrap()).unwrap(), bytes);
}

#[test]
fn ensure_restarts_when_partial_file_is_longer_than_artifact() {
    let fixture = Fixture::new();
    let (artifact, bytes) = published(&fixture, "weights", FILE_BYTES);
    fixture.write_partial(artifact, &pattern(FILE_BYTES + 5));

    let files = fixture.ensure(&[artifact]).unwrap();

    assert_eq!(fs::read(files.path("weights").unwrap()).unwrap(), bytes);
}

#[test]
fn ensure_installs_without_fetch_when_partial_file_is_complete() {
    let fixture = Fixture::new();
    let (artifact, bytes) = published(&fixture, "weights", FILE_BYTES);
    fixture.write_partial(artifact, &bytes);
    fixture.unpublish(artifact);

    let files = fixture.ensure(&[artifact]).unwrap();

    assert_eq!(fs::read(files.path("weights").unwrap()).unwrap(), bytes);
}

#[test]
fn ensure_installs_range_when_source_is_directory() {
    let fixture = Fixture::new();
    let archive = pattern(FILE_BYTES);
    let part = range("prompt", &archive, 1_000, 5_000);
    fixture.publish(part, &archive);

    let files = fixture.ensure(&[part]).unwrap();

    let installed = fs::read(files.path("prompt").unwrap()).unwrap();
    assert_eq!(installed, &archive[1_000..6_000]);
}

#[test]
fn ensure_installs_empty_artifact_when_extent_is_zero() {
    let fixture = Fixture::new();
    let (artifact, _) = published(&fixture, "empty", 0);

    let files = fixture.ensure(&[artifact]).unwrap();

    assert_eq!(fs::read(files.path("empty").unwrap()).unwrap(), b"");
    assert!(fixture.store.has_artifact(artifact));
}

#[test]
fn ensure_installs_without_fetch_when_verified_record_is_missing() {
    let fixture = Fixture::new();
    let (artifact, bytes) = published(&fixture, "weights", FILE_BYTES);
    fixture.ensure(&[artifact]).unwrap();
    fs::remove_file(fixture.verified(artifact)).unwrap();
    fixture.unpublish(artifact);

    let files = fixture.ensure(&[artifact]).unwrap();

    assert_eq!(fs::read(files.path("weights").unwrap()).unwrap(), bytes);
    assert!(fixture.verified(artifact).is_file());
}

#[test]
fn ensure_writes_hash_to_verified_record_when_installed() {
    let fixture = Fixture::new();
    let (artifact, _) = published(&fixture, "weights", FILE_BYTES);

    fixture.ensure(&[artifact]).unwrap();

    let record = fs::read_to_string(fixture.verified(artifact)).unwrap();
    assert_eq!(record, artifact.sha256);
}

#[test]
fn ensure_installs_shared_file_once_when_two_keys_have_one_file() {
    let fixture = Fixture::new();
    let (first, bytes) = published(&fixture, "first", FILE_BYTES);
    let second = leak(Artifact {
        key: "second",
        ..*first
    });
    let (sender, receiver) = flume::unbounded();
    let record = |progress| sender.send(progress).unwrap();

    let files = fixture
        .store
        .ensure([first, second], &record, &NOT_CANCELLED)
        .unwrap();

    assert_eq!(files.path("first").unwrap(), files.path("second").unwrap());
    assert_eq!(fs::read(files.path("first").unwrap()).unwrap(), bytes);
    let reports: Vec<DownloadProgress> = receiver.try_iter().collect();
    assert!(
        reports
            .iter()
            .all(|report| report.total_bytes == FILE_BYTES as u64)
    );
}

#[test]
fn ensure_reports_sum_of_files_as_total_when_two_artifacts_missing() {
    let fixture = Fixture::new();
    let (first, _) = published(&fixture, "first", FILE_BYTES);
    let (second, _) = published(&fixture, "second", BLOCK_BYTES);
    let (sender, receiver) = flume::unbounded();
    let record = |progress| sender.send(progress).unwrap();

    fixture
        .store
        .ensure([first, second], &record, &NOT_CANCELLED)
        .unwrap();

    let last = receiver.try_iter().last().unwrap();
    let total = (FILE_BYTES + BLOCK_BYTES) as u64;
    assert_eq!((last.done_bytes, last.total_bytes), (total, total));
}
