use std::fs::{File, OpenOptions};
use std::io::{ErrorKind, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use antenna_core::Artifact;

use super::{Fetch, FetchError, copy_blocks, finish_copy, start_offset};
use crate::ModelError;
use crate::layout::file_len;

/// Copies artifacts from a local directory with the layout of the model store.
pub(crate) struct DirectoryFetch {
    directory: PathBuf,
}

impl DirectoryFetch {
    pub(crate) fn new(directory: &Path) -> Self {
        Self {
            directory: directory.to_owned(),
        }
    }

    fn open_source(&self, artifact: &'static Artifact) -> Result<(File, PathBuf), FetchError> {
        let path = self
            .directory
            .join(artifact.repo)
            .join(artifact.revision)
            .join(artifact.path);
        match File::open(&path) {
            Ok(file) => Ok((file, path)),
            Err(error) if error.kind() == ErrorKind::NotFound => {
                Err(FetchError::Permanent(ModelError::NotFound {
                    artifact: artifact.key,
                }))
            }
            Err(source) => Err(FetchError::Permanent(ModelError::io(&path)(source))),
        }
    }
}

impl Fetch for DirectoryFetch {
    fn fetch(
        &self,
        artifact: &'static Artifact,
        partial: &Path,
        progress: &dyn Fn(u64),
        cancel: &AtomicBool,
    ) -> Result<(), FetchError> {
        let have = file_len(partial);
        let (mut input, source) = self.open_source(artifact)?;
        let position = start_offset(artifact.extent) + have;
        input
            .seek(SeekFrom::Start(position))
            .map_err(FetchError::io(&source))?;
        let mut reader = input.take(artifact.extent.bytes().saturating_sub(have));
        let mut output = OpenOptions::new()
            .create(true)
            .append(true)
            .open(partial)
            .map_err(FetchError::io(partial))?;
        let result = copy_blocks(&mut reader, &mut output, progress, cancel);
        finish_copy(result, partial, FetchError::io(&source))
    }
}
