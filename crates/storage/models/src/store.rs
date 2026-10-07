use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use antenna_core::{Artifact, ModelFiles};

use crate::ModelError;
use crate::install::Installer;
use crate::layout::{ArtifactPaths, default_root, project_dirs};
use crate::progress::{DownloadProgress, Reporter};
use crate::retry::RETRY_DELAYS;
use crate::source::{DirectoryFetch, HubFetch, Source};

const MODEL_DIR_ENV: &str = "ANTENNA_MODEL_DIR";
const DISK_MARGIN_BYTES: u64 = 1_073_741_824;

/// The local directory that contains the downloaded artifacts.
#[derive(Clone, Debug)]
pub struct ModelStore {
    pub(crate) root: PathBuf,
    source: Source,
    retry_delays: [Duration; 3],
}

impl ModelStore {
    /// Opens the model store of the platform, which downloads from Hugging Face.
    ///
    /// The environment variable `ANTENNA_MODEL_DIR` replaces the platform directory.
    ///
    /// # Errors
    ///
    /// Returns [`ModelError::NoDataDir`] if the variable is not set and the platform has no data
    /// directory, or [`ModelError::Io`] if the root cannot be created.
    pub fn open_default() -> Result<Self, ModelError> {
        let root = default_root(env::var_os(MODEL_DIR_ENV), project_dirs())?;
        Self::open(root, Source::Hub { endpoint: None })
    }

    /// Opens the model store in a root directory and creates the directory if it does not exist.
    ///
    /// # Errors
    ///
    /// Returns [`ModelError::Io`] if the root cannot be created.
    pub fn open(root: impl Into<PathBuf>, source: Source) -> Result<Self, ModelError> {
        let root = root.into();
        fs::create_dir_all(&root).map_err(ModelError::io(&root))?;
        Ok(Self {
            root,
            source,
            retry_delays: RETRY_DELAYS,
        })
    }

    /// Returns the root directory of the store.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Replaces the delays before the three retries of a failed download.
    #[must_use]
    pub fn with_retry_delays(self, delays: [Duration; 3]) -> Self {
        Self {
            retry_delays: delays,
            ..self
        }
    }

    /// Installs the artifacts that are not installed and returns the local path of each artifact.
    ///
    /// A download continues from the partial file of an earlier call. `progress` gets at most one
    /// report in each 100 ms, and one report at the end.
    ///
    /// # Errors
    ///
    /// Returns [`ModelError::DuplicateKey`] if two artifacts have the same key,
    /// [`ModelError::DiskSpace`] if the disk is too small, [`ModelError::Cancelled`] if `cancel`
    /// becomes `true`, and the other [`ModelError`] variants if a download, a check or a file
    /// operation fails.
    pub fn ensure(
        &self,
        artifacts: impl IntoIterator<Item = &'static Artifact>,
        progress: &(dyn Fn(DownloadProgress) + Sync),
        cancel: &AtomicBool,
    ) -> Result<ModelFiles, ModelError> {
        let artifacts = unique_keys(artifacts)?;
        let missing: Vec<_> = artifacts
            .iter()
            .copied()
            .filter(|artifact| !self.has_artifact(artifact))
            .collect();
        if !missing.is_empty() {
            self.install(&missing, progress, cancel)?;
        }
        Ok(self.model_files(&artifacts))
    }

    fn install(
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
                let fetcher = HubFetch::new(endpoint.as_deref());
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
