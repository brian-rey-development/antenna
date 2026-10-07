use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use antenna_core::{Artifact, ModelFiles};

use crate::ModelError;
use crate::install::Installer;
use crate::layout::{ArtifactPaths, default_root, project_dirs, unique_files};
use crate::progress::{DownloadProgress, Reporter};
use crate::retry::{RETRY_COUNT, RETRY_DELAYS};
use crate::source::{DirectoryFetch, HubFetch, READ_TIMEOUT, Source};

const MODEL_DIR_ENV: &str = "ANTENNA_MODEL_DIR";
const DISK_MARGIN_BYTES: u64 = 1_073_741_824;

/// The local directory that contains the downloaded artifacts.
#[derive(Clone, Debug)]
pub struct ModelStore {
    pub(crate) root: PathBuf,
    source: Source,
    retry_delays: [Duration; RETRY_COUNT],
}

impl ModelStore {
    /// Opens the model store of the platform, which downloads from Hugging Face.
    ///
    /// The environment variable `ANTENNA_MODEL_DIR` replaces the platform directory. An empty
    /// variable counts as not set.
    ///
    /// # Errors
    ///
    /// Returns [`ModelError::NoDataDir`] if the variable is not set and the platform has no data
    /// directory.
    pub fn open_default() -> Result<Self, ModelError> {
        let root = default_root(env::var_os(MODEL_DIR_ENV), project_dirs())?;
        Ok(Self::open(root, Source::Hub { endpoint: None }))
    }

    /// Opens the model store in a root directory.
    ///
    /// The function does not touch the disk. [`ModelStore::ensure`] creates the root directory
    /// when a download needs it.
    pub fn open(root: impl Into<PathBuf>, source: Source) -> Self {
        Self {
            root: root.into(),
            source,
            retry_delays: RETRY_DELAYS,
        }
    }

    /// Returns the root directory of the store.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Replaces the delays before the three retries of a failed download.
    #[must_use]
    pub fn with_retry_delays(self, delays: [Duration; RETRY_COUNT]) -> Self {
        Self {
            retry_delays: delays,
            ..self
        }
    }

    /// Installs the artifacts that are not installed and returns the local path of each artifact.
    ///
    /// A download continues from the partial file of an earlier call. If a download is necessary,
    /// `progress` gets at most one report in each progress interval and one report at the end.
    /// If all artifacts are installed, `progress` gets no report.
    ///
    /// # Errors
    ///
    /// Returns one of these errors.
    ///
    /// 1. [`ModelError::DuplicateKey`] if two artifacts have the same key.
    /// 2. [`ModelError::DiskSpace`] if the disk is too small.
    /// 3. [`ModelError::Cancelled`] if `cancel` becomes `true`.
    /// 4. Another [`ModelError`] if a download, a check or a file operation fails.
    pub fn ensure(
        &self,
        artifacts: impl IntoIterator<Item = &'static Artifact>,
        progress: &(dyn Fn(DownloadProgress) + Sync),
        cancel: &AtomicBool,
    ) -> Result<ModelFiles, ModelError> {
        let artifacts = unique_keys(artifacts)?;
        let missing = unique_files(
            artifacts
                .iter()
                .copied()
                .filter(|artifact| !self.has_artifact(artifact)),
        );
        if !missing.is_empty() {
            self.install_missing(&missing, progress, cancel)?;
        }
        Ok(self.model_files(&artifacts))
    }

    fn install_missing(
        &self,
        missing: &[&'static Artifact],
        progress: &(dyn Fn(DownloadProgress) + Sync),
        cancel: &AtomicBool,
    ) -> Result<(), ModelError> {
        let total_bytes = self.missing_bytes(missing.iter().copied());
        self.check_disk_space(total_bytes)?;
        let reporter = Reporter::new(progress, total_bytes);
        let retry_delays = self.retry_delays;
        let root = self.root.as_path();
        match &self.source {
            Source::Hub { endpoint } => {
                let fetcher = HubFetch::new(endpoint.as_deref(), READ_TIMEOUT);
                Installer::new(&fetcher, retry_delays, &reporter, cancel).install_all(root, missing)
            }
            Source::Directory(directory) => {
                let fetcher = DirectoryFetch::new(directory);
                Installer::new(&fetcher, retry_delays, &reporter, cancel).install_all(root, missing)
            }
        }
    }

    fn check_disk_space(&self, missing_bytes: u64) -> Result<(), ModelError> {
        let needed_bytes = missing_bytes.saturating_add(DISK_MARGIN_BYTES);
        fs::create_dir_all(&self.root).map_err(ModelError::io(&self.root))?;
        let available_bytes =
            fs4::available_space(&self.root).map_err(ModelError::io(&self.root))?;
        if available_bytes < needed_bytes {
            return Err(ModelError::DiskSpace {
                root: self.root.clone(),
                needed_bytes,
                available_bytes,
            });
        }
        Ok(())
    }

    fn model_files(&self, artifacts: &[&'static Artifact]) -> ModelFiles {
        let files: BTreeMap<_, _> = artifacts
            .iter()
            .map(|artifact| (artifact.key, ArtifactPaths::new(&self.root, artifact).file))
            .collect();
        ModelFiles::new(files)
    }
}

fn unique_keys(
    artifacts: impl IntoIterator<Item = &'static Artifact>,
) -> Result<Vec<&'static Artifact>, ModelError> {
    let mut keys = BTreeSet::new();
    let artifacts: Vec<_> = artifacts.into_iter().collect();
    match artifacts.iter().find(|artifact| !keys.insert(artifact.key)) {
        Some(artifact) => Err(ModelError::DuplicateKey { key: artifact.key }),
        None => Ok(artifacts),
    }
}
