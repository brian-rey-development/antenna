use std::fs;
use std::sync::atomic::{AtomicBool, Ordering};

use antenna_core::{Artifact, Extent};
use antenna_models::{DownloadProgress, ModelError};

use crate::server::Behavior;
use crate::setup::{FILE_BYTES, Setup, route};
use crate::support::{leak, pattern, whole};

const BLOCK_BYTES: u64 = 65_536;

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
fn ensure_returns_cancelled_when_cancelled_during_retry_wait() {
    let (setup, artifact, _) = setup_with(Behavior::FailFirst(4));
    let cancel = AtomicBool::new(true);

    let result = setup
        .store
        .ensure([artifact], &|_: DownloadProgress| {}, &cancel);

    assert!(matches!(result, Err(ModelError::Cancelled)));
    assert_eq!(setup.request_count(artifact), 1);
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

#[test]
fn ensure_fails_when_server_sends_more_bytes_than_declared() {
    let (setup, artifact, _) = setup_with(Behavior::Serve);
    let declared = leak(Artifact {
        extent: Extent::Whole {
            bytes: FILE_BYTES as u64 - 10,
        },
        ..*artifact
    });

    let result = setup.ensure(declared);

    assert!(matches!(
        result,
        Err(ModelError::Size {
            artifact: "weights",
            ..
        })
    ));
    assert!(!setup.partial(artifact).exists());
}
