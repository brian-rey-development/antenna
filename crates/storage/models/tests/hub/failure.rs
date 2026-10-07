use std::fs;
use std::sync::atomic::{AtomicBool, Ordering};

use antenna_core::{Artifact, Extent};
use antenna_models::{DownloadProgress, ModelError};

use crate::server::Behavior;
use crate::setup::{FILE_BYTES, RESUME_BYTES, Setup, route};
use crate::support::{leak, pattern, range, whole};

const BLOCK_BYTES: u64 = 65_536;
const EXTRA_BYTES: usize = 10;

fn setup_with(behavior: Behavior) -> (Setup, &'static Artifact, Vec<u8>) {
    let bytes = pattern(FILE_BYTES);
    let artifact = whole("weights", &bytes);
    let setup = Setup::new(vec![route(artifact, &bytes, behavior)]);
    (setup, artifact, bytes)
}

#[test]
fn ensure_succeeds_when_server_fails_three_times() {
    let (setup, artifact, bytes) = setup_with(Behavior::FailFirst(3));

    let installed = setup.install(artifact);

    assert_eq!(installed, bytes);
    assert_eq!(setup.request_count(artifact), 4);
}

#[test]
fn ensure_fails_when_server_fails_four_times() {
    let (setup, artifact, _) = setup_with(Behavior::FailFirst(4));

    let result = setup.ensure(artifact);

    assert!(matches!(
        result,
        Err(ModelError::Network {
            artifact: "weights",
            attempts: 4,
            ..
        })
    ));
    assert_eq!(setup.request_count(artifact), 4);
}

#[test]
fn ensure_does_not_retry_when_not_found() {
    let (setup, artifact, _) = setup_with(Behavior::Status(404));

    let result = setup.ensure(artifact);

    assert!(matches!(
        result,
        Err(ModelError::NotFound {
            artifact: "weights"
        })
    ));
    assert_eq!(setup.request_count(artifact), 1);
}

#[test]
fn ensure_does_not_retry_when_access_denied() {
    for status in [401, 403] {
        let (setup, artifact, _) = setup_with(Behavior::Status(status));

        let result = setup.ensure(artifact);

        assert!(matches!(result, Err(ModelError::NotFound { .. })));
        assert_eq!(setup.request_count(artifact), 1);
    }
}

#[test]
fn ensure_retries_when_status_is_transient() {
    for status in [408, 429, 500, 502, 504] {
        let (setup, artifact, _) = setup_with(Behavior::Status(status));

        let result = setup.ensure(artifact);

        assert!(matches!(
            result,
            Err(ModelError::Network { attempts: 4, .. })
        ));
        assert_eq!(setup.request_count(artifact), 4);
    }
}

#[test]
fn ensure_does_not_retry_when_status_is_not_transient() {
    let (setup, artifact, _) = setup_with(Behavior::Status(400));

    let result = setup.ensure(artifact);

    assert!(matches!(
        result,
        Err(ModelError::Network { attempts: 1, .. })
    ));
    assert_eq!(setup.request_count(artifact), 1);
}

#[test]
fn ensure_returns_cancelled_without_request_when_cancel_is_set() {
    let (setup, artifact, _) = setup_with(Behavior::FailFirst(4));
    let cancel = AtomicBool::new(true);

    let result = setup
        .store
        .ensure([artifact], &|_: DownloadProgress| {}, &cancel);

    assert!(matches!(result, Err(ModelError::Cancelled)));
    assert_eq!(setup.request_count(artifact), 0);
}

#[test]
fn ensure_stops_after_one_block_when_cancelled_during_download() {
    let (setup, artifact, _) = setup_with(Behavior::Serve);
    let cancel = AtomicBool::new(false);
    let cancel_on_progress = |_: DownloadProgress| cancel.store(true, Ordering::Relaxed);

    let result = setup.store.ensure([artifact], &cancel_on_progress, &cancel);

    let kept = fs::metadata(setup.partial(artifact)).unwrap().len();
    assert!(matches!(result, Err(ModelError::Cancelled)));
    assert!((1..=BLOCK_BYTES).contains(&kept));
}

fn declared_shorter(artifact: &Artifact, bytes: &[u8]) -> &'static Artifact {
    let shorter = &bytes[..bytes.len() - EXTRA_BYTES];
    leak(Artifact {
        extent: Extent::Whole {
            bytes: shorter.len() as u64,
        },
        ..*artifact
    })
}

#[test]
fn ensure_fails_on_hash_when_server_sends_more_bytes_than_declared() {
    let (setup, artifact, bytes) = setup_with(Behavior::Serve);
    let declared = declared_shorter(artifact, &bytes);

    let result = setup.ensure(declared);

    assert!(matches!(
        result,
        Err(ModelError::Hash {
            artifact: "weights",
            ..
        })
    ));
    assert!(!setup.file(artifact).exists());
    assert!(!setup.partial(artifact).exists());
}

#[test]
fn ensure_installs_declared_bytes_when_server_sends_more_bytes() {
    let (setup, artifact, bytes) = setup_with(Behavior::Serve);
    let shorter = &bytes[..bytes.len() - EXTRA_BYTES];
    let declared = whole("weights", shorter);

    let installed = setup.install(declared);

    assert_eq!(installed, shorter);
    assert_eq!(setup.request_count(artifact), 1);
}

#[test]
fn ensure_fails_when_server_answers_other_range_than_requested() {
    let archive = pattern(FILE_BYTES);
    let artifact = range("prompt", &archive, 100_000, 50_000);
    let setup = Setup::new(vec![route(artifact, &archive, Behavior::MisplacedRange)]);

    let result = setup.ensure(artifact);

    assert!(matches!(
        result,
        Err(ModelError::RangeUnsupported { artifact: "prompt" })
    ));
    assert_eq!(setup.request_count(artifact), 1);
}

#[test]
fn ensure_keeps_partial_file_when_server_answers_other_range_on_resume() {
    let (setup, artifact, bytes) = setup_with(Behavior::MisplacedRange);
    setup.write_partial(artifact, &bytes[..RESUME_BYTES]);

    let result = setup.ensure(artifact);

    let kept = fs::metadata(setup.partial(artifact)).unwrap().len();
    assert!(matches!(result, Err(ModelError::RangeUnsupported { .. })));
    assert_eq!(kept, RESUME_BYTES as u64);
}
