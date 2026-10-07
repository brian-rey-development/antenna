use std::io;
use std::path::{Path, PathBuf};

use thiserror::Error;

/// The error of the model store.
#[derive(Debug, Error)]
pub enum ModelError {
    /// The disk has less free space than the download needs.
    #[error(
        "not enough disk space in {root}: {needed_bytes} bytes needed, {available_bytes} bytes available"
    )]
    DiskSpace {
        /// The root of the model store.
        root: PathBuf,
        /// The bytes to download plus the safety margin.
        needed_bytes: u64,
        /// The bytes that the disk of the root has free.
        available_bytes: u64,
    },
    /// The SHA-256 of a downloaded file is different from the declared hash.
    #[error("the hash of {artifact} is {actual}, but it must be {expected}")]
    Hash {
        /// The key of the artifact.
        artifact: &'static str,
        /// The declared SHA-256.
        expected: &'static str,
        /// The SHA-256 of the downloaded bytes.
        actual: String,
    },
    /// A downloaded file has a different size than the declared size.
    #[error("{artifact} has {actual_bytes} bytes, but it must have {expected_bytes} bytes")]
    Size {
        /// The key of the artifact.
        artifact: &'static str,
        /// The declared size.
        expected_bytes: u64,
        /// The size of the downloaded file.
        actual_bytes: u64,
    },
    /// The download failed and no retry is left, or the error is not transient.
    #[error("the download of {artifact} failed, attempt count {attempts}")]
    Network {
        /// The key of the artifact.
        artifact: &'static str,
        /// The number of consecutive attempts without progress.
        attempts: u32,
        /// The error of the last attempt.
        #[source]
        source: ureq::Error,
    },
    /// The source has no file for the artifact.
    #[error("the source has no file for {artifact}")]
    NotFound {
        /// The key of the artifact.
        artifact: &'static str,
    },
    /// The server did not answer the byte range that the download asked for.
    #[error("the server does not support the byte range of {artifact}")]
    RangeUnsupported {
        /// The key of the artifact.
        artifact: &'static str,
    },
    /// Two artifacts of one request have the same key.
    #[error("two artifacts have the key {key}")]
    DuplicateKey {
        /// The key of the artifacts.
        key: &'static str,
    },
    /// A download holds the lock of the artifact.
    #[error("a download of {artifact} is in progress")]
    Busy {
        /// The key of the artifact.
        artifact: &'static str,
    },
    /// A file operation failed.
    #[error("cannot access {path}")]
    Io {
        /// The path of the file or directory.
        path: PathBuf,
        /// The error of the operating system.
        #[source]
        source: io::Error,
    },
    /// The environment variable is not set and the platform has no data directory.
    #[error("the platform has no data directory, and ANTENNA_MODEL_DIR is not set")]
    NoDataDir,
    /// The caller cancelled the operation.
    #[error("the operation was cancelled")]
    Cancelled,
}

impl ModelError {
    pub(crate) fn io(path: &Path) -> impl FnOnce(io::Error) -> Self + '_ {
        |source| Self::Io {
            path: path.to_owned(),
            source,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn network_message_names_attempt_count_when_count_is_one() {
        let error = ModelError::Network {
            artifact: "weights",
            attempts: 1,
            source: ureq::Error::ConnectionFailed,
        };

        assert_eq!(
            error.to_string(),
            "the download of weights failed, attempt count 1"
        );
    }
}
