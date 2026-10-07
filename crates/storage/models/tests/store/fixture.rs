use std::fs;
use std::path::{Path, PathBuf};

use antenna_core::{Artifact, ModelFiles};
use antenna_models::{DownloadProgress, ModelError, ModelStore, Source};
use tempfile::TempDir;

use crate::support::{self, NOT_CANCELLED, pattern, whole};

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

    pub(crate) fn published(&self, key: &str, length: usize) -> (&'static Artifact, Vec<u8>) {
        let bytes = pattern(length);
        let artifact = whole(key, &bytes);
        self.publish(artifact, &bytes);
        (artifact, bytes)
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
        support::file(self.store.root(), artifact)
    }

    pub(crate) fn partial(&self, artifact: &Artifact) -> PathBuf {
        support::partial(self.store.root(), artifact)
    }

    pub(crate) fn lock(&self, artifact: &Artifact) -> PathBuf {
        support::with_suffix(&self.file(artifact), ".lock")
    }

    pub(crate) fn verified(&self, artifact: &Artifact) -> PathBuf {
        support::with_suffix(&self.file(artifact), ".verified")
    }

    fn source_path(&self, artifact: &Artifact) -> PathBuf {
        self.source
            .path()
            .join(artifact.repo)
            .join(artifact.revision)
            .join(artifact.path)
    }
}
