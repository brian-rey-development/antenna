use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use antenna_core::{Artifact, ModelFiles};
use antenna_models::{DownloadProgress, ModelError, ModelStore, Source};
use tempfile::TempDir;

use crate::server::{Behavior, Route, TestServer, hub_path};
use crate::support::{self, FILE_BYTES, NOT_CANCELLED};

pub(crate) const RESUME_BYTES: usize = 100_000;

const NO_DELAYS: [Duration; 3] = [Duration::ZERO; 3];

pub(crate) struct Setup {
    pub(crate) server: TestServer,
    pub(crate) store: ModelStore,
    _root: TempDir,
}

impl Setup {
    pub(crate) fn new(routes: Vec<Route>) -> Self {
        let server = TestServer::start(routes);
        let root = tempfile::tempdir().unwrap();
        let source = Source::Hub {
            endpoint: Some(server.endpoint().to_owned()),
        };
        let store = ModelStore::open(root.path(), source).with_retry_delays(NO_DELAYS);
        Self {
            server,
            store,
            _root: root,
        }
    }

    pub(crate) fn ensure(&self, artifact: &'static Artifact) -> Result<ModelFiles, ModelError> {
        self.store
            .ensure([artifact], &|_: DownloadProgress| {}, &NOT_CANCELLED)
    }

    pub(crate) fn install(&self, artifact: &'static Artifact) -> Vec<u8> {
        let files = self.ensure(artifact).unwrap();
        fs::read(files.path(artifact.key).unwrap()).unwrap()
    }

    pub(crate) fn file(&self, artifact: &Artifact) -> PathBuf {
        support::file(self.store.root(), artifact)
    }

    pub(crate) fn partial(&self, artifact: &Artifact) -> PathBuf {
        support::partial(self.store.root(), artifact)
    }

    pub(crate) fn write_partial(&self, artifact: &Artifact, bytes: &[u8]) {
        let path = self.partial(artifact);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }

    pub(crate) fn ranges(&self, path: &str) -> Vec<Option<String>> {
        self.server
            .requests(path)
            .into_iter()
            .map(|request| request.range)
            .collect()
    }

    pub(crate) fn request_count(&self, artifact: &Artifact) -> usize {
        self.server.requests(&hub_path(artifact)).len()
    }
}

pub(crate) fn route(artifact: &Artifact, body: &[u8], behavior: Behavior) -> Route {
    Route {
        path: hub_path(artifact),
        body: body.to_vec(),
        behavior,
    }
}

pub(crate) fn resume_range(start: usize) -> String {
    format!("bytes={start}-{}", FILE_BYTES - 1)
}
