use std::fs::{self, File};
use std::io::{self, Read};
use std::path::Path;

use antenna_core::Artifact;
use sha2::{Digest, Sha256};

use crate::ModelError;
use crate::layout::ArtifactPaths;

const HASH_BUFFER_BYTES: usize = 1_048_576;

/// Checks the partial file of an artifact and makes it the installed file.
///
/// The file is at its final path only after the size check and the hash check pass. If a check
/// fails, this function deletes the partial file.
pub(crate) fn commit(artifact: &'static Artifact, paths: &ArtifactPaths) -> Result<(), ModelError> {
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
    let actual = sha256_hex(&paths.partial).map_err(ModelError::io(&paths.partial))?;
    if actual != artifact.sha256 {
        discard(paths)?;
        return Err(ModelError::Hash {
            artifact: artifact.key,
            expected: artifact.sha256,
            actual,
        });
    }
    fs::rename(&paths.partial, &paths.file).map_err(ModelError::io(&paths.file))?;
    fs::write(&paths.verified, actual).map_err(ModelError::io(&paths.verified))
}

fn discard(paths: &ArtifactPaths) -> Result<(), ModelError> {
    fs::remove_file(&paths.partial).map_err(ModelError::io(&paths.partial))
}

fn sha256_hex(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; HASH_BUFFER_BYTES];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(buffer.split_at(count).0);
    }
    Ok(to_hex(&hasher.finalize()))
}

fn to_hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .fold(String::new(), |hex, byte| hex + &format!("{byte:02x}"))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    const EMPTY_HASH: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
    const ABC_HASH: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

    #[test]
    fn sha256_hex_matches_known_value_when_file_is_empty() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("empty");
        fs::write(&path, b"").unwrap();

        let hash = sha256_hex(&path).unwrap();

        assert_eq!(hash, EMPTY_HASH);
    }

    #[test]
    fn sha256_hex_matches_known_value_when_file_has_abc() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("abc");
        fs::write(&path, b"abc").unwrap();

        let hash = sha256_hex(&path).unwrap();

        assert_eq!(hash, ABC_HASH);
    }

    #[test]
    fn sha256_hex_matches_one_shot_hash_when_file_exceeds_buffer() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("large");
        let bytes = vec![7_u8; HASH_BUFFER_BYTES + 1];
        fs::write(&path, &bytes).unwrap();

        let hash = sha256_hex(&path).unwrap();

        assert_eq!(hash, to_hex(&Sha256::digest(&bytes)));
    }

    #[test]
    fn sha256_hex_fails_when_file_missing() {
        let directory = tempfile::tempdir().unwrap();

        let result = sha256_hex(&directory.path().join("missing"));

        assert!(result.is_err());
    }
}
