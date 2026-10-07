use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use antenna_models::DownloadProgress;

use crate::server::{Behavior, Route, hub_path};
use crate::setup::{FILE_BYTES, NOT_CANCELLED, RESUME_BYTES, Setup, resume_range, route};
use crate::support::{pattern, range, whole};

#[test]
fn ensure_sends_antenna_user_agent_when_downloading() {
    let bytes = pattern(FILE_BYTES);
    let artifact = whole("weights", &bytes);
    let setup = Setup::new(vec![route(artifact, &bytes, Behavior::Serve)]);

    setup.install(artifact);

    let requests = setup.server.requests(&hub_path(artifact));
    let agent = requests[0].user_agent.as_deref().unwrap();
    assert_eq!(agent, concat!("antenna/", env!("CARGO_PKG_VERSION")));
}

#[test]
fn ensure_resumes_when_partial_file_exists() {
    let bytes = pattern(FILE_BYTES);
    let artifact = whole("weights", &bytes);
    let setup = Setup::new(vec![route(artifact, &bytes, Behavior::Serve)]);
    setup.write_partial(artifact, &bytes[..RESUME_BYTES]);

    let installed = setup.install(artifact);

    assert_eq!(installed, bytes);
    let expected = Some(resume_range(RESUME_BYTES));
    assert_eq!(setup.ranges(&hub_path(artifact)), [expected]);
}

#[test]
fn ensure_resumes_when_connection_breaks() {
    let bytes = pattern(FILE_BYTES);
    let artifact = whole("weights", &bytes);
    let behavior = Behavior::CloseAfter(RESUME_BYTES);
    let setup = Setup::new(vec![route(artifact, &bytes, behavior)]);

    let installed = setup.install(artifact);

    assert_eq!(installed, bytes);
    let resumed = Some(resume_range(RESUME_BYTES));
    assert_eq!(setup.ranges(&hub_path(artifact)), [None, None, resumed]);
}

#[test]
fn ensure_reports_increasing_progress_when_connection_breaks() {
    let bytes = pattern(FILE_BYTES);
    let artifact = whole("weights", &bytes);
    let behavior = Behavior::CloseAfter(RESUME_BYTES);
    let setup = Setup::new(vec![route(artifact, &bytes, behavior)]);
    let (sender, receiver) = flume::unbounded();
    let record = |progress| sender.send(progress).unwrap();

    setup
        .store
        .ensure([artifact], &record, &NOT_CANCELLED)
        .unwrap();

    let reports: Vec<DownloadProgress> = receiver.try_iter().collect();
    assert!(reports.is_sorted_by_key(|report| report.done_bytes));
    assert!(
        reports
            .iter()
            .all(|report| report.total_bytes == FILE_BYTES as u64)
    );
    assert_eq!(
        reports.last().map(|report| report.done_bytes),
        Some(FILE_BYTES as u64)
    );
}

#[test]
fn ensure_reports_remaining_bytes_as_total_when_partial_file_exists() {
    let bytes = pattern(FILE_BYTES);
    let artifact = whole("weights", &bytes);
    let setup = Setup::new(vec![route(artifact, &bytes, Behavior::Serve)]);
    setup.write_partial(artifact, &bytes[..RESUME_BYTES]);
    let (sender, receiver) = flume::unbounded();
    let record = |progress| sender.send(progress).unwrap();

    setup
        .store
        .ensure([artifact], &record, &NOT_CANCELLED)
        .unwrap();

    let first = receiver.try_iter().next().unwrap();
    assert_eq!(first.total_bytes, (FILE_BYTES - RESUME_BYTES) as u64);
}

#[test]
fn ensure_downloads_only_range_when_extent_is_range() {
    let archive = pattern(FILE_BYTES);
    let artifact = range("prompt", &archive, 100_000, 50_000);
    let setup = Setup::new(vec![route(artifact, &archive, Behavior::Serve)]);

    let installed = setup.install(artifact);

    assert_eq!(installed, &archive[100_000..150_000]);
    let expected = Some("bytes=100000-149999".to_owned());
    assert_eq!(setup.ranges(&hub_path(artifact)), [expected]);
}

#[test]
fn ensure_fails_when_server_ignores_range_for_extent_range() {
    let archive = pattern(FILE_BYTES);
    let artifact = range("prompt", &archive, 100_000, 50_000);
    let setup = Setup::new(vec![route(artifact, &archive, Behavior::IgnoreRange)]);

    let result = setup.ensure(artifact);

    assert!(matches!(
        result,
        Err(antenna_models::ModelError::RangeUnsupported { artifact: "prompt" })
    ));
    assert_eq!(setup.request_count(artifact), 1);
}

#[test]
fn ensure_restarts_when_server_ignores_range_on_resume() {
    let bytes = pattern(FILE_BYTES);
    let artifact = whole("weights", &bytes);
    let setup = Setup::new(vec![route(artifact, &bytes, Behavior::IgnoreRange)]);
    setup.write_partial(artifact, &bytes[..RESUME_BYTES]);

    let installed = setup.install(artifact);

    assert_eq!(installed, bytes);
    assert_eq!(setup.request_count(artifact), 1);
}

#[test]
fn ensure_restarts_when_partial_file_is_longer_than_artifact() {
    let bytes = pattern(FILE_BYTES);
    let artifact = whole("weights", &bytes);
    let setup = Setup::new(vec![route(artifact, &bytes, Behavior::Serve)]);
    setup.write_partial(artifact, &pattern(FILE_BYTES + 5));

    let installed = setup.install(artifact);

    assert_eq!(installed, bytes);
    assert_eq!(setup.ranges(&hub_path(artifact)), [None]);
}

#[test]
fn ensure_keeps_range_when_redirected() {
    let bytes = pattern(FILE_BYTES);
    let artifact = whole("weights", &bytes);
    let mirror = "/mirror/weights.bin";
    let redirect = Behavior::Redirect(mirror.to_owned());
    let setup = Setup::new(vec![
        route(artifact, &bytes, redirect),
        Route {
            path: mirror.to_owned(),
            body: bytes.clone(),
            behavior: Behavior::Serve,
        },
    ]);
    setup.write_partial(artifact, &bytes[..RESUME_BYTES]);

    let installed = setup.install(artifact);

    assert_eq!(installed, bytes);
    let expected = Some(resume_range(RESUME_BYTES));
    assert_eq!(setup.ranges(mirror), [expected]);
}

#[test]
fn ensure_waits_on_lock_when_other_call_fetches() {
    let bytes = pattern(FILE_BYTES);
    let artifact = whole("weights", &bytes);
    let setup = Setup::new(vec![route(artifact, &bytes, Behavior::Serve)]);
    let (started, running) = flume::bounded(1);
    let first_report = AtomicBool::new(true);
    // The first call holds the lock for 300 ms, about three lock polls of the second call. The
    // assertions hold for any timing, because a second call that comes late finds the file.
    let hold = |_: DownloadProgress| {
        if first_report.swap(false, Ordering::Relaxed) {
            started.send(()).unwrap();
            thread::park_timeout(Duration::from_millis(300));
        }
    };

    let results = thread::scope(|scope| {
        let first = thread::Builder::new()
            .name("antenna-test-first".to_owned())
            .spawn_scoped(scope, || {
                setup.store.ensure([artifact], &hold, &NOT_CANCELLED)
            })
            .unwrap();
        running.recv().unwrap();
        let second = setup.ensure(artifact);
        (first.join().unwrap(), second)
    });

    assert!(results.0.is_ok());
    assert!(results.1.is_ok());
    assert_eq!(setup.request_count(artifact), 1);
}
