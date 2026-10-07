use std::fs::{self, File, OpenOptions};
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

use antenna_core::Artifact;

use crate::layout::ArtifactPaths;
use crate::progress::Reporter;
use crate::retry::{self, CANCEL_POLL, RETRY_COUNT};
use crate::source::{Fetch, FetchError};
use crate::{ModelError, verify};

/// Installs the missing artifacts of one request with one fetcher.
pub(crate) struct Installer<'a, F> {
    fetcher: &'a F,
    retry_delays: [Duration; RETRY_COUNT],
    reporter: &'a Reporter<'a>,
    cancel: &'a AtomicBool,
}

impl<'a, F: Fetch> Installer<'a, F> {
    pub(crate) fn new(
        fetcher: &'a F,
        retry_delays: [Duration; RETRY_COUNT],
        reporter: &'a Reporter<'a>,
        cancel: &'a AtomicBool,
    ) -> Self {
        Self {
            fetcher,
            retry_delays,
            reporter,
            cancel,
        }
    }

    pub(crate) fn install_all(
        &self,
        root: &Path,
        artifacts: &[&'static Artifact],
    ) -> Result<(), ModelError> {
        for artifact in artifacts {
            self.install(artifact, &ArtifactPaths::new(root, artifact))?;
        }
        self.reporter.finish();
        Ok(())
    }

    fn install(
        &self,
        artifact: &'static Artifact,
        paths: &ArtifactPaths,
    ) -> Result<(), ModelError> {
        let extent = artifact.extent;
        let share = paths.missing_bytes(extent);
        let kept_bytes = paths.resumable_len(extent);
        fs::create_dir_all(paths.directory()).map_err(ModelError::io(paths.directory()))?;
        let _lock = self.lock(paths)?;
        paths.reclaim_unverified(extent)?;
        if !paths.is_installed(extent) {
            self.fetch_with_retry(artifact, paths, kept_bytes)?;
            verify::commit(artifact, paths, self.cancel)?;
        }
        self.reporter.complete_file(share);
        Ok(())
    }

    fn lock(&self, paths: &ArtifactPaths) -> Result<File, ModelError> {
        loop {
            if let Some(lock) = paths.try_lock()? {
                return Ok(lock);
            }
            retry::wait(CANCEL_POLL, self.cancel)?;
        }
    }

    fn fetch_with_retry(
        &self,
        artifact: &'static Artifact,
        paths: &ArtifactPaths,
        kept_bytes: u64,
    ) -> Result<(), ModelError> {
        retry::check(self.cancel)?;
        let mut delays = self.retry_delays.into_iter();
        let mut attempts = 0;
        loop {
            attempts += 1;
            let source = match self.fetch_once(artifact, paths, kept_bytes) {
                Ok(()) => return Ok(()),
                Err(FetchError::Cancelled) => return Err(ModelError::Cancelled),
                Err(FetchError::Permanent(error)) => return Err(error),
                Err(FetchError::Transient(source)) => source,
            };
            let Some(delay) = delays.next() else {
                let artifact = artifact.key;
                return Err(ModelError::Network {
                    artifact,
                    attempts,
                    source,
                });
            };
            retry::wait(delay, self.cancel)?;
        }
    }

    fn fetch_once(
        &self,
        artifact: &'static Artifact,
        paths: &ArtifactPaths,
        kept_bytes: u64,
    ) -> Result<(), FetchError> {
        let bytes = artifact.extent.bytes();
        if paths.partial_len() > bytes {
            fs::remove_file(&paths.partial).map_err(FetchError::io(&paths.partial))?;
        }
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(&paths.partial)
            .map_err(FetchError::io(&paths.partial))?;
        let have = paths.partial_len();
        if have == bytes {
            return Ok(());
        }
        let earlier_bytes = have.saturating_sub(kept_bytes);
        let share = bytes - kept_bytes;
        let progress = |written: u64| {
            let file_bytes = (earlier_bytes + written).min(share);
            self.reporter.advance(file_bytes, Instant::now());
        };
        self.fetcher
            .fetch(artifact, &paths.partial, &progress, self.cancel)
    }
}
