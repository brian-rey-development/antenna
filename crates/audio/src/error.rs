use std::path::PathBuf;

use thiserror::Error;

/// An error of the player or of an export.
#[derive(Debug, Error)]
pub enum AudioError {
    /// A stored segment cannot be read.
    #[error("cannot read the segment {path}")]
    ReadSegment {
        /// The path of the segment file.
        path: PathBuf,
        /// The error of the WAV reader.
        #[source]
        source: hound::Error,
    },
    /// A stored segment is not 16-bit mono PCM.
    #[error("the segment {path} is not 16-bit mono PCM, and its format is {spec:?}")]
    UnsupportedSegment {
        /// The path of the segment file.
        path: PathBuf,
        /// The format of the file.
        spec: hound::WavSpec,
    },
}
