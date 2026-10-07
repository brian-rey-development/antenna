use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs::TryLockError;
use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};

use antenna_core::{Artifact, EngineDescriptor, Extent, Quality, VoiceDescriptor, app_dirs};
use directories::ProjectDirs;

use crate::ModelError;

const MODELS_DIR_NAME: &str = "models";
const PARTIAL_SUFFIX: &str = ".part";
const VERIFIED_SUFFIX: &str = ".verified";
const LOCK_SUFFIX: &str = ".lock";

/// The local files of one artifact.
#[derive(Debug)]
pub(crate) struct ArtifactPaths {
    pub(crate) file: PathBuf,
    pub(crate) partial: PathBuf,
    pub(crate) verified: PathBuf,
    pub(crate) lock: PathBuf,
    directory: PathBuf,
}

impl ArtifactPaths {
    pub(crate) fn new(root: &Path, artifact: &Artifact) -> Self {
        let file = root.join(relative_path(artifact));
        Self {
            partial: with_suffix(&file, PARTIAL_SUFFIX),
            verified: with_suffix(&file, VERIFIED_SUFFIX),
            lock: with_suffix(&file, LOCK_SUFFIX),
            directory: file.parent().unwrap_or(root).to_owned(),
            file,
        }
    }

    pub(crate) fn directory(&self) -> &Path {
        &self.directory
    }

    pub(crate) fn is_installed(&self, extent: Extent) -> bool {
        self.has_declared_size(extent) && self.verified.is_file()
    }

    fn has_declared_size(&self, extent: Extent) -> bool {
        fs::metadata(&self.file)
            .is_ok_and(|metadata| metadata.is_file() && metadata.len() == extent.bytes())
    }

    pub(crate) fn partial_bytes(&self) -> u64 {
        file_bytes(&self.partial)
    }

    /// Returns the bytes of the partial file that a download can keep.
    pub(crate) fn resumable_bytes(&self, extent: Extent) -> u64 {
        let kept = self.partial_bytes();
        if kept <= extent.bytes() { kept } else { 0 }
    }

    /// Turns a file with the declared size but no `.verified` record into a partial file. The
    /// next hash check can then install it with no download.
    pub(crate) fn reclaim_unverified(&self, extent: Extent) -> Result<(), ModelError> {
        if !self.has_declared_size(extent) || self.verified.is_file() || self.partial_bytes() > 0 {
            return Ok(());
        }
        fs::rename(&self.file, &self.partial).map_err(ModelError::io(&self.file))
    }

    pub(crate) fn missing_bytes(&self, extent: Extent) -> u64 {
        if self.is_installed(extent) {
            return 0;
        }
        extent.bytes() - self.resumable_bytes(extent)
    }

    /// Takes the exclusive lock of the artifact. Returns `None` if another holder has it.
    ///
    /// The function creates the directory of the artifact on each call, because a removal can
    /// delete an empty directory while a download waits for the lock.
    pub(crate) fn try_lock(&self) -> Result<Option<File>, ModelError> {
        fs::create_dir_all(&self.directory).map_err(ModelError::io(&self.directory))?;
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&self.lock)
            .map_err(ModelError::io(&self.lock))?;
        match file.try_lock() {
            Ok(()) => Ok(Some(file)),
            Err(TryLockError::WouldBlock) => Ok(None),
            Err(TryLockError::Error(source)) => Err(ModelError::io(&self.lock)(source)),
        }
    }
}

/// Returns the size of a file, or 0 if the file does not exist.
pub(crate) fn file_bytes(path: &Path) -> u64 {
    fs::metadata(path).map_or(0, |metadata| metadata.len())
}

/// Returns the path of the local file of an artifact, relative to the root of the store.
pub(crate) fn relative_path(artifact: &Artifact) -> PathBuf {
    let local_name = match artifact.extent {
        Extent::Whole { .. } => artifact.path.to_owned(),
        Extent::Range { offset, bytes } => format!("{}@{offset}+{bytes}", artifact.path),
    };
    Path::new(artifact.repo)
        .join(artifact.revision)
        .join(local_name)
}

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

/// Removes the artifacts that have the same local file as an earlier artifact.
pub(crate) fn unique_files(
    artifacts: impl IntoIterator<Item = &'static Artifact>,
) -> Vec<&'static Artifact> {
    let mut seen = BTreeSet::new();
    artifacts
        .into_iter()
        .filter(|artifact| seen.insert(relative_path(artifact)))
        .collect()
}

/// Returns the root of the model store.
///
/// The environment value, if it is not empty, replaces the `models` directory in the platform data
/// directory.
pub(crate) fn default_root(
    env: Option<OsString>,
    dirs: Option<ProjectDirs>,
) -> Result<PathBuf, ModelError> {
    if let Some(root) = env.filter(|root| !root.is_empty()) {
        return Ok(PathBuf::from(root));
    }
    let dirs = dirs.ok_or(ModelError::NoDataDir)?;
    Ok(dirs.data_dir().join(MODELS_DIR_NAME))
}

/// Finds the platform directories of Antenna.
pub(crate) fn project_dirs() -> Option<ProjectDirs> {
    ProjectDirs::from(
        app_dirs::QUALIFIER,
        app_dirs::ORGANIZATION,
        app_dirs::APPLICATION,
    )
}

/// Returns the artifacts of the variant of a quality, then the artifacts of a voice.
pub fn voice_artifacts(
    descriptor: &EngineDescriptor,
    voice: &VoiceDescriptor,
    quality: Quality,
) -> Vec<&'static Artifact> {
    let variant = descriptor.variants.get(quality).artifacts;
    variant.iter().chain(voice.artifacts).collect()
}

/// Returns the artifacts of all voices of an engine.
pub(crate) fn voice_files(
    descriptor: &EngineDescriptor,
) -> impl Iterator<Item = &'static Artifact> {
    descriptor.voices.iter().flat_map(|voice| voice.artifacts)
}

/// Returns the artifacts of the three variants and of all voices of an engine. Each local file
/// occurs one time.
pub fn engine_artifacts(descriptor: &EngineDescriptor) -> Vec<&'static Artifact> {
    let variants = Quality::ALL
        .into_iter()
        .flat_map(|quality| descriptor.variants.get(quality).artifacts);
    unique_files(variants.chain(voice_files(descriptor)))
}

#[cfg(test)]
mod tests {
    use super::*;

    const ARCHIVE: Artifact = Artifact {
        key: "archive",
        repo: "antenna/test",
        revision: "0123456789abcdef0123456789abcdef01234567",
        path: "model/archive.nemo",
        extent: Extent::Range {
            offset: 512,
            bytes: 64,
        },
        sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    };

    #[test]
    fn layout_separates_ranges_of_one_file() {
        let other = Artifact {
            extent: Extent::Range {
                offset: 1024,
                bytes: 64,
            },
            ..ARCHIVE
        };

        let first = ArtifactPaths::new(Path::new("root"), &ARCHIVE);
        let second = ArtifactPaths::new(Path::new("root"), &other);

        assert_ne!(first.file, second.file);
        assert!(first.file.ends_with("model/archive.nemo@512+64"));
        assert!(second.file.ends_with("model/archive.nemo@1024+64"));
    }

    #[test]
    fn layout_shares_file_when_range_is_the_same() {
        let renamed = Artifact {
            key: "other-key",
            ..ARCHIVE
        };

        let first = ArtifactPaths::new(Path::new("root"), &ARCHIVE);
        let second = ArtifactPaths::new(Path::new("root"), &renamed);

        assert_eq!(first.file, second.file);
    }

    #[test]
    fn layout_places_files_next_to_the_artifact_when_whole() {
        let whole = Artifact {
            extent: Extent::Whole { bytes: 10 },
            ..ARCHIVE
        };

        let paths = ArtifactPaths::new(Path::new("root"), &whole);

        let directory =
            Path::new("root/antenna/test/0123456789abcdef0123456789abcdef01234567/model");
        assert_eq!(paths.file, directory.join("archive.nemo"));
        assert_eq!(paths.partial, directory.join("archive.nemo.part"));
        assert_eq!(paths.verified, directory.join("archive.nemo.verified"));
        assert_eq!(paths.lock, directory.join("archive.nemo.lock"));
        assert_eq!(paths.directory(), directory);
    }

    #[test]
    fn try_lock_creates_directory_when_it_is_missing() {
        let root = tempfile::tempdir().unwrap();
        let paths = ArtifactPaths::new(root.path(), &ARCHIVE);

        let lock = paths.try_lock().unwrap();

        assert!(lock.is_some());
        assert!(paths.lock.is_file());
    }

    #[test]
    fn default_root_uses_env_when_set() {
        let root = default_root(Some(OsString::from("/models")), project_dirs());

        assert_eq!(root.unwrap(), PathBuf::from("/models"));
    }

    #[test]
    fn default_root_uses_data_dir_when_env_absent() {
        let dirs = project_dirs().unwrap();
        let expected = dirs.data_dir().join("models");

        let root = default_root(None, Some(dirs));

        assert_eq!(root.unwrap(), expected);
    }

    #[test]
    fn default_root_uses_data_dir_when_env_empty() {
        let dirs = project_dirs().unwrap();
        let expected = dirs.data_dir().join("models");

        let root = default_root(Some(OsString::new()), Some(dirs));

        assert_eq!(root.unwrap(), expected);
    }

    #[test]
    fn default_root_fails_when_env_and_data_dir_absent() {
        let root = default_root(None, None);

        assert!(matches!(root, Err(ModelError::NoDataDir)));
    }
}
