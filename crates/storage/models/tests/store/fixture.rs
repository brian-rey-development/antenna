use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use antenna_core::{Artifact, ModelFiles};
use antenna_models::{DownloadProgress, ModelError, ModelStore, Source};
use tempfile::TempDir;

use crate::support::{REPO, REVISION};

pub(crate) static NOT_CANCELLED: AtomicBool = AtomicBool::new(false);

pub(crate) fn ignore_progress(_: DownloadProgress) {}

pub(crate) struct Fixture {
    pub(crate) store: ModelStore,
    source: TempDir,
    _root: TempDir,
}

impl Fixture {
    pub(crate) fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let source = tempfile::tempdir().unwrap();
        let store =
            ModelStore::open(root.path(), Source::Directory(source.path().to_owned())).unwrap();
        Self {
            store,
            source,
            _root: root,
        }
    }

    pub(crate) fn store_at(&self, root: &Path) -> ModelStore {
        ModelStore::open(root, Source::Directory(self.source.path().to_owned())).unwrap()
    }

    pub(crate) fn publish(&self, artifact: &Artifact, bytes: &[u8]) {
        let path = self.source_path(artifact);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }

    pub(crate) fn unpublish(&self, artifact: &Artifact) {
        fs::remove_file(self.source_path(artifact)).unwrap();
    }

    pub(crate) fn ensure(&self, artifacts: &[&'static Artifact]) -> Result<ModelFiles, ModelError> {
        self.store
            .ensure(artifacts.iter().copied(), &ignore_progress, &NOT_CANCELLED)
    }

    pub(crate) fn write_partial(&self, artifact: &Artifact, bytes: &[u8]) {
        fs::create_dir_all(self.file(artifact).parent().unwrap()).unwrap();
        fs::write(self.partial(artifact), bytes).unwrap();
    }

    pub(crate) fn file(&self, artifact: &Artifact) -> PathBuf {
        self.store
            .root()
            .join(REPO)
            .join(REVISION)
            .join(artifact.path)
    }

    pub(crate) fn partial(&self, artifact: &Artifact) -> PathBuf {
        self.with_suffix(artifact, ".part")
    }

    pub(crate) fn lock(&self, artifact: &Artifact) -> PathBuf {
        self.with_suffix(artifact, ".lock")
    }

    pub(crate) fn verified(&self, artifact: &Artifact) -> PathBuf {
        self.with_suffix(artifact, ".verified")
    }

    fn with_suffix(&self, artifact: &Artifact, suffix: &str) -> PathBuf {
        let mut name = self.file(artifact).into_os_string();
        name.push(suffix);
        PathBuf::from(name)
    }

    fn source_path(&self, artifact: &Artifact) -> PathBuf {
        self.source
            .path()
            .join(REPO)
            .join(REVISION)
            .join(artifact.path)
    }
}
