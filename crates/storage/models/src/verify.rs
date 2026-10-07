use std::fs::{self, File, OpenOptions};
use std::io::Read;
use std::path::Path;
use std::sync::atomic::AtomicBool;

use antenna_core::Artifact;
use sha2::{Digest, Sha256};

use crate::layout::ArtifactPaths;
use crate::{ModelError, retry};

const HASH_BUFFER_BYTES: usize = 1_048_576;

/// Checks the partial file of an artifact and makes it the installed file.
///
/// The file is at its final path only after the size check and the hash check pass, and after the
/// operating system wrote its bytes to the disk. If a check fails, this function deletes the
/// partial file. If `cancel` becomes `true` during the hash check, it keeps the partial file.
pub(crate) fn commit(
    artifact: &'static Artifact,
    paths: &ArtifactPaths,
    cancel: &AtomicBool,
) -> Result<(), ModelError> {
    let actual_bytes = paths.partial_len();
    let expected_bytes = artifact.extent.bytes();
    if actual_bytes != expected_bytes {
        discard(paths)?;
        return Err(ModelError::Size {
            artifact: artifact.key,
            expected_bytes,
            actual_bytes,
        });
    }
    let actual = sha256_hex(&paths.partial, cancel)?;
    if actual != artifact.sha256 {
        discard(paths)?;
        return Err(ModelError::Hash {
            artifact: artifact.key,
            expected: artifact.sha256,
            actual,
        });
    }
    sync(&paths.partial)?;
    fs::rename(&paths.partial, &paths.file).map_err(ModelError::io(&paths.file))?;
    fs::write(&paths.verified, actual).map_err(ModelError::io(&paths.verified))
}

fn discard(paths: &ArtifactPaths) -> Result<(), ModelError> {
    fs::remove_file(&paths.partial).map_err(ModelError::io(&paths.partial))
}

fn sync(path: &Path) -> Result<(), ModelError> {
    let file = OpenOptions::new().append(true).open(path);
    file.and_then(|file| file.sync_all())
        .map_err(ModelError::io(path))
}

fn sha256_hex(path: &Path, cancel: &AtomicBool) -> Result<String, ModelError> {
    let mut file = File::open(path).map_err(ModelError::io(path))?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; HASH_BUFFER_BYTES];
    loop {
        retry::check(cancel)?;
        let count = file.read(&mut buffer).map_err(ModelError::io(path))?;
        if count == 0 {
            return Ok(to_hex(&hasher.finalize()));
        }
        hasher.update(buffer.split_at(count).0);
    }
}

fn to_hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .fold(String::new(), |hex, byte| hex + &format!("{byte:02x}"))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use antenna_core::Extent;

    use super::*;

    static NOT_CANCELLED: AtomicBool = AtomicBool::new(false);
    static CANCELLED: AtomicBool = AtomicBool::new(true);

    const ABC: Artifact = Artifact {
        key: "abc",
        repo: "antenna/test",
        revision: "0123456789abcdef0123456789abcdef01234567",
        path: "abc.bin",
        extent: Extent::Whole { bytes: 3 },
        sha256: ABC_HASH,
    };
    const EMPTY_HASH: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
    const ABC_HASH: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

    #[test]
    fn sha256_hex_matches_known_value_when_file_is_empty() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("empty");
        fs::write(&path, b"").unwrap();

        let hash = sha256_hex(&path, &NOT_CANCELLED).unwrap();

        assert_eq!(hash, EMPTY_HASH);
    }

    #[test]
    fn sha256_hex_matches_known_value_when_file_has_abc() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("abc");
        fs::write(&path, b"abc").unwrap();

        let hash = sha256_hex(&path, &NOT_CANCELLED).unwrap();

        assert_eq!(hash, ABC_HASH);
    }

    #[test]
    fn sha256_hex_matches_one_shot_hash_when_file_exceeds_buffer() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("large");
        let bytes = vec![7_u8; HASH_BUFFER_BYTES + 1];
        fs::write(&path, &bytes).unwrap();

        let hash = sha256_hex(&path, &NOT_CANCELLED).unwrap();

        assert_eq!(hash, to_hex(&Sha256::digest(&bytes)));
    }

    #[test]
    fn sha256_hex_fails_when_file_missing() {
        let directory = tempfile::tempdir().unwrap();

        let result = sha256_hex(&directory.path().join("missing"), &NOT_CANCELLED);

        assert!(result.is_err());
    }

    #[test]
    fn sha256_hex_returns_cancelled_when_flag_is_set() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("abc");
        fs::write(&path, b"abc").unwrap();

        let result = sha256_hex(&path, &CANCELLED);

        assert!(matches!(result, Err(ModelError::Cancelled)));
    }

    #[test]
    fn commit_moves_partial_file_to_final_path_when_hash_matches() {
        let directory = tempfile::tempdir().unwrap();
        let paths = ArtifactPaths::new(directory.path(), &ABC);
        fs::create_dir_all(paths.directory()).unwrap();
        fs::write(&paths.partial, b"abc").unwrap();

        commit(&ABC, &paths, &NOT_CANCELLED).unwrap();

        assert_eq!(fs::read(&paths.file).unwrap(), b"abc");
        assert_eq!(fs::read_to_string(&paths.verified).unwrap(), ABC_HASH);
        assert!(!paths.partial.exists());
    }

    #[test]
    fn commit_keeps_partial_file_when_cancelled() {
        let directory = tempfile::tempdir().unwrap();
        let paths = ArtifactPaths::new(directory.path(), &ABC);
        fs::create_dir_all(paths.directory()).unwrap();
        fs::write(&paths.partial, b"abc").unwrap();

        let result = commit(&ABC, &paths, &CANCELLED);

        assert!(matches!(result, Err(ModelError::Cancelled)));
        assert!(paths.partial.is_file());
        assert!(!paths.file.exists());
    }
}
