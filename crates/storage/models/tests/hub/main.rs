//! Behavior tests of the model store with a local HTTP server.

#[cfg(test)]
#[path = "../support/mod.rs"]
mod support;

#[cfg(test)]
mod server;

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::thread;
    use std::time::Duration;

    use antenna_core::{Artifact, ModelFiles};
    use antenna_models::{DownloadProgress, ModelError, ModelStore, Source};
    use tempfile::TempDir;

    use crate::server::{Behavior, Route, TestServer, hub_path};
    use crate::support::{pattern, range, whole};

    const FILE_BYTES: usize = 300_000;
    const RESUME_BYTES: usize = 100_000;
    const NO_DELAYS: [Duration; 3] = [Duration::ZERO; 3];
    static NOT_CANCELLED: AtomicBool = AtomicBool::new(false);

    struct Setup {
        server: TestServer,
        store: ModelStore,
        _root: TempDir,
    }

    impl Setup {
        fn new(routes: Vec<Route>) -> Self {
            let server = TestServer::start(routes);
            let root = tempfile::tempdir().unwrap();
            let source = Source::Hub {
                endpoint: Some(server.endpoint().to_owned()),
            };
            let store = ModelStore::open(root.path(), source)
                .unwrap()
                .with_retry_delays(NO_DELAYS);
            Self {
                server,
                store,
                _root: root,
            }
        }

        fn ensure(&self, artifact: &'static Artifact) -> Result<ModelFiles, ModelError> {
            self.store
                .ensure([artifact], &|_: DownloadProgress| {}, &NOT_CANCELLED)
        }

        fn install(&self, artifact: &'static Artifact) -> Vec<u8> {
            let files = self.ensure(artifact).unwrap();
            fs::read(files.path(artifact.key).unwrap()).unwrap()
        }

        fn write_partial(&self, artifact: &Artifact, bytes: &[u8]) {
            let mut name = self
                .store
                .root()
                .join(artifact.repo)
                .join(artifact.revision)
                .join(artifact.path)
                .into_os_string();
            name.push(".part");
            let path = PathBuf::from(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, bytes).unwrap();
        }

        fn ranges(&self, path: &str) -> Vec<Option<String>> {
            self.server
                .requests(path)
                .into_iter()
                .map(|request| request.range)
                .collect()
        }
    }

    fn route(artifact: &Artifact, body: &[u8], behavior: Behavior) -> Route {
        Route {
            path: hub_path(artifact),
            body: body.to_vec(),
            behavior,
        }
    }

    #[test]
    fn ensure_succeeds_when_server_fails_three_times() {
        let bytes = pattern(FILE_BYTES);
        let artifact = whole("weights", &bytes);
        let setup = Setup::new(vec![route(artifact, &bytes, Behavior::FailFirst(3))]);

        let installed = setup.install(artifact);

        assert_eq!(installed, bytes);
        assert_eq!(setup.server.requests(&hub_path(artifact)).len(), 4);
    }

    #[test]
    fn ensure_fails_when_server_fails_four_times() {
        let bytes = pattern(FILE_BYTES);
        let artifact = whole("weights", &bytes);
        let setup = Setup::new(vec![route(artifact, &bytes, Behavior::FailFirst(4))]);

        let result = setup.ensure(artifact);

        assert!(matches!(
            result,
            Err(ModelError::Network {
                artifact: "weights",
                attempts: 4,
                ..
            })
        ));
        assert_eq!(setup.server.requests(&hub_path(artifact)).len(), 4);
    }

    #[test]
    fn ensure_does_not_retry_when_not_found() {
        let bytes = pattern(FILE_BYTES);
        let artifact = whole("weights", &bytes);
        let setup = Setup::new(vec![route(artifact, &bytes, Behavior::Missing)]);

        let result = setup.ensure(artifact);

        assert!(matches!(
            result,
            Err(ModelError::NotFound {
                artifact: "weights"
            })
        ));
        assert_eq!(setup.server.requests(&hub_path(artifact)).len(), 1);
    }

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
    fn ensure_returns_cancelled_when_cancelled_during_retry_wait() {
        let bytes = pattern(FILE_BYTES);
        let artifact = whole("weights", &bytes);
        let setup = Setup::new(vec![route(artifact, &bytes, Behavior::FailFirst(4))]);
        let cancel = AtomicBool::new(true);

        let result = setup
            .store
            .ensure([artifact], &|_: DownloadProgress| {}, &cancel);

        assert!(matches!(result, Err(ModelError::Cancelled)));
        assert_eq!(setup.server.requests(&hub_path(artifact)).len(), 1);
    }

    #[test]
    fn ensure_resumes_when_partial_file_exists() {
        let bytes = pattern(FILE_BYTES);
        let artifact = whole("weights", &bytes);
        let setup = Setup::new(vec![route(artifact, &bytes, Behavior::Serve)]);
        setup.write_partial(artifact, &bytes[..RESUME_BYTES]);

        let installed = setup.install(artifact);

        assert_eq!(installed, bytes);
        let expected = format!("bytes={RESUME_BYTES}-{}", FILE_BYTES - 1);
        assert_eq!(setup.ranges(&hub_path(artifact)), [Some(expected)]);
    }

    #[test]
    fn ensure_resumes_when_connection_breaks() {
        let bytes = pattern(FILE_BYTES);
        let artifact = whole("weights", &bytes);
        let behavior = Behavior::CloseAfter(RESUME_BYTES);
        let setup = Setup::new(vec![route(artifact, &bytes, behavior)]);

        let installed = setup.install(artifact);

        assert_eq!(installed, bytes);
        let resumed = format!("bytes={RESUME_BYTES}-{}", FILE_BYTES - 1);
        assert_eq!(
            setup.ranges(&hub_path(artifact)),
            [None, None, Some(resumed)]
        );
    }

    #[test]
    fn ensure_downloads_only_range_when_extent_is_range() {
        let archive = pattern(FILE_BYTES);
        let artifact = range("prompt", &archive, 100_000, 50_000);
        let setup = Setup::new(vec![route(artifact, &archive, Behavior::Serve)]);

        let installed = setup.install(artifact);

        assert_eq!(installed, &archive[100_000..150_000]);
        assert_eq!(
            setup.ranges(&hub_path(artifact)),
            [Some("bytes=100000-149999".to_owned())]
        );
    }

    #[test]
    fn ensure_fails_when_server_ignores_range_for_extent_range() {
        let archive = pattern(FILE_BYTES);
        let artifact = range("prompt", &archive, 100_000, 50_000);
        let setup = Setup::new(vec![route(artifact, &archive, Behavior::IgnoreRange)]);

        let result = setup.ensure(artifact);

        assert!(matches!(
            result,
            Err(ModelError::RangeUnsupported { artifact: "prompt" })
        ));
        assert_eq!(setup.server.requests(&hub_path(artifact)).len(), 1);
    }

    #[test]
    fn ensure_restarts_when_server_ignores_range_on_resume() {
        let bytes = pattern(FILE_BYTES);
        let artifact = whole("weights", &bytes);
        let setup = Setup::new(vec![route(artifact, &bytes, Behavior::IgnoreRange)]);
        setup.write_partial(artifact, &bytes[..RESUME_BYTES]);

        let installed = setup.install(artifact);

        assert_eq!(installed, bytes);
        assert_eq!(setup.server.requests(&hub_path(artifact)).len(), 1);
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
        let expected = format!("bytes={RESUME_BYTES}-{}", FILE_BYTES - 1);
        assert_eq!(setup.ranges(mirror), [Some(expected)]);
    }

    #[test]
    fn ensure_waits_on_lock_when_other_call_fetches() {
        let bytes = pattern(FILE_BYTES);
        let artifact = whole("weights", &bytes);
        let setup = Setup::new(vec![route(artifact, &bytes, Behavior::Serve)]);
        let (started, running) = flume::bounded(1);
        let first_report = AtomicBool::new(true);
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
        assert_eq!(setup.server.requests(&hub_path(artifact)).len(), 1);
    }
}
