use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use antenna_core::{Artifact, Extent};
use sha2::{Digest, Sha256};

pub(crate) const REPO: &str = "antenna/test";
pub(crate) const REVISION: &str = "0123456789abcdef0123456789abcdef01234567";
pub(crate) const BLOCK_BYTES: usize = 65_536;
pub(crate) const FILE_BYTES: usize = 300_000;
pub(crate) static NOT_CANCELLED: AtomicBool = AtomicBool::new(false);

const BYTE_PERIOD: usize = 251;
const PARTIAL_SUFFIX: &str = ".part";

pub(crate) fn leak<T>(value: T) -> &'static T {
    Box::leak(Box::new(value))
}

pub(crate) fn pattern(length: usize) -> Vec<u8> {
    (0..length)
        .map(|index| u8::try_from(index % BYTE_PERIOD).unwrap())
        .collect()
}

pub(crate) fn whole(key: &str, bytes: &[u8]) -> &'static Artifact {
    artifact(
        key,
        &format!("{key}.bin"),
        Extent::Whole {
            bytes: bytes.len() as u64,
        },
        bytes,
    )
}

pub(crate) fn range(key: &str, archive: &[u8], offset: usize, length: usize) -> &'static Artifact {
    let extent = Extent::Range {
        offset: offset as u64,
        bytes: length as u64,
    };
    artifact(
        key,
        "archive.nemo",
        extent,
        &archive[offset..offset + length],
    )
}

/// Returns the local file of an artifact in a store root.
pub(crate) fn file(root: &Path, artifact: &Artifact) -> PathBuf {
    let local_name = match artifact.extent {
        Extent::Whole { .. } => artifact.path.to_owned(),
        Extent::Range { offset, bytes } => format!("{}@{offset}+{bytes}", artifact.path),
    };
    root.join(artifact.repo)
        .join(artifact.revision)
        .join(local_name)
}

pub(crate) fn partial(root: &Path, artifact: &Artifact) -> PathBuf {
    with_suffix(&file(root, artifact), PARTIAL_SUFFIX)
}

pub(crate) fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

fn artifact(key: &str, path: &str, extent: Extent, hashed: &[u8]) -> &'static Artifact {
    let hash = Sha256::digest(hashed)
        .iter()
        .fold(String::new(), |mut hex, byte| {
            write!(hex, "{byte:02x}").unwrap();
            hex
        });
    leak(Artifact {
        key: Box::leak(key.to_owned().into_boxed_str()),
        repo: REPO,
        revision: REVISION,
        path: Box::leak(path.to_owned().into_boxed_str()),
        extent,
        sha256: Box::leak(hash.into_boxed_str()),
    })
}
