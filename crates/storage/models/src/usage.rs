use std::collections::BTreeSet;
use std::fs::{self, Metadata};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use antenna_core::{Artifact, EngineDescriptor, Quality, VoiceDescriptor};

use crate::layout::{
    ArtifactPaths, engine_artifacts, relative_path, unique_files, voice_artifacts,
};
use crate::{ModelError, ModelStore};

impl ModelStore {
    /// Returns `true` if the file of the artifact and its `.verified` record exist and the file has
    /// the declared size.
    pub fn has_artifact(&self, artifact: &Artifact) -> bool {
        ArtifactPaths::new(&self.root, artifact).is_installed(artifact.extent)
    }

    /// Returns `true` if all artifacts of a voice at a quality are installed.
    pub fn is_installed(
        &self,
        descriptor: &EngineDescriptor,
        voice: &VoiceDescriptor,
        quality: Quality,
    ) -> bool {
        voice_artifacts(descriptor, voice, quality)
            .into_iter()
            .all(|artifact| self.has_artifact(artifact))
    }

    /// Returns `true` if each voice of an engine is installed at a quality.
    pub fn is_engine_installed(&self, descriptor: &EngineDescriptor, quality: Quality) -> bool {
        descriptor
            .voices
            .iter()
            .all(|voice| self.is_installed(descriptor, voice, quality))
    }

    /// Returns the bytes that a download of the artifacts still needs. A partial file counts as
    /// downloaded.
    pub fn missing_bytes(&self, artifacts: impl IntoIterator<Item = &'static Artifact>) -> u64 {
        artifacts
            .into_iter()
            .map(|artifact| ArtifactPaths::new(&self.root, artifact).missing_bytes(artifact.extent))
            .sum()
    }

    /// Returns the bytes of the installed artifacts of an engine. The function reads no directory.
    pub fn engine_disk_usage(&self, descriptor: &EngineDescriptor) -> u64 {
        engine_artifacts(descriptor)
            .into_iter()
            .filter(|artifact| self.has_artifact(artifact))
            .map(|artifact| artifact.extent.bytes())
            .sum()
    }

    /// Returns the bytes of all files in the store, partial files included.
    ///
    /// # Errors
    ///
    /// Returns [`ModelError::Io`] if a directory or a file cannot be read.
    pub fn disk_usage(&self) -> Result<u64, ModelError> {
        let mut total = 0;
        let mut pending = vec![self.root.clone()];
        while let Some(directory) = pending.pop() {
            for (path, metadata) in list_directory(&directory)? {
                if metadata.is_dir() {
                    pending.push(path);
                } else if metadata.is_file() {
                    total += metadata.len();
                }
            }
        }
        Ok(total)
    }

    /// Deletes the files of the artifacts of an engine, except the artifacts in `keep`. Then it
    /// deletes the directories that are empty. Returns the number of deleted bytes.
    ///
    /// # Errors
    ///
    /// Returns [`ModelError::Busy`] if a download holds the lock of an artifact, or
    /// [`ModelError::Io`] if a file cannot be deleted.
    pub fn remove_engine(
        &self,
        descriptor: &EngineDescriptor,
        keep: impl IntoIterator<Item = &'static Artifact>,
    ) -> Result<u64, ModelError> {
        let kept: BTreeSet<PathBuf> = keep.into_iter().map(relative_path).collect();
        let mut deleted_bytes = 0;
        for artifact in engine_artifacts(descriptor) {
            if !kept.contains(&relative_path(artifact)) {
                deleted_bytes += self.remove_artifact(artifact)?;
            }
        }
        Ok(deleted_bytes)
    }

    fn remove_artifact(&self, artifact: &Artifact) -> Result<u64, ModelError> {
        let paths = ArtifactPaths::new(&self.root, artifact);
        if !paths.directory().exists() {
            return Ok(0);
        }
        let Some(lock) = paths.try_lock()? else {
            return Err(ModelError::Busy {
                artifact: artifact.key,
            });
        };
        let mut deleted_bytes = 0;
        for path in [&paths.file, &paths.verified, &paths.partial] {
            deleted_bytes += remove_counted(path)?;
        }
        drop(lock);
        deleted_bytes += remove_counted(&paths.lock)?;
        prune_empty_directories(&self.root, paths.directory());
        Ok(deleted_bytes)
    }
}

/// Returns the bytes of the variant of a quality and of all voices of an engine. Each local file
/// counts one time.
pub fn engine_download_bytes(descriptor: &EngineDescriptor, quality: Quality) -> u64 {
    let variant = descriptor.variants.get(quality).artifacts;
    let voices = descriptor.voices.iter().flat_map(|voice| voice.artifacts);
    unique_files(variant.iter().chain(voices))
        .into_iter()
        .map(|artifact| artifact.extent.bytes())
        .sum()
}

/// Returns the artifacts that `remove_engine` must keep, which are the artifacts of the other
/// engines and the artifacts of `extra`, for example the review model.
pub fn keep_artifacts<'a>(
    other_engines: impl IntoIterator<Item = &'a EngineDescriptor>,
    extra: impl IntoIterator<Item = &'static Artifact>,
) -> Vec<&'static Artifact> {
    other_engines
        .into_iter()
        .flat_map(engine_artifacts)
        .chain(extra)
        .collect()
}

fn list_directory(directory: &Path) -> Result<Vec<(PathBuf, Metadata)>, ModelError> {
    let entries = fs::read_dir(directory).map_err(ModelError::io(directory))?;
    entries
        .map(|entry| {
            let entry = entry.map_err(ModelError::io(directory))?;
            let metadata = entry.metadata().map_err(ModelError::io(&entry.path()))?;
            Ok((entry.path(), metadata))
        })
        .collect()
}

fn remove_counted(path: &Path) -> Result<u64, ModelError> {
    let bytes = match fs::metadata(path) {
        Ok(metadata) => metadata.len(),
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(0),
        Err(source) => return Err(ModelError::io(path)(source)),
    };
    fs::remove_file(path).map_err(ModelError::io(path))?;
    Ok(bytes)
}

fn prune_empty_directories(root: &Path, start: &Path) {
    // `remove_dir` fails on a directory that is not empty. That failure ends the walk.
    for directory in start.ancestors().take_while(|directory| *directory != root) {
        if fs::remove_dir(directory).is_err() {
            return;
        }
    }
}
