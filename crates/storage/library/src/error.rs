use std::io;
use std::path::{Path, PathBuf};

use thiserror::Error;
use toml::{de, ser};

use crate::DocumentId;

/// The error of the library crate.
#[derive(Debug, Error)]
pub enum LibraryError {
    /// No document has the id.
    #[error("no document has the id {0}")]
    NotFound(DocumentId),
    /// The title of a document is empty or contains only whitespace.
    #[error("the title is empty")]
    EmptyTitle,
    /// A `document.toml` file is damaged or has an unknown version.
    #[error("cannot read the metadata in {path}")]
    MetaRead {
        /// The path of the file.
        path: PathBuf,
        /// The error of the TOML parser.
        #[source]
        source: de::Error,
    },
    /// The metadata of a document cannot be written as TOML.
    #[error("cannot write the metadata for {path}")]
    MetaWrite {
        /// The path of the file.
        path: PathBuf,
        /// The error of the TOML writer.
        #[source]
        source: ser::Error,
    },
    /// A segment file cannot be read or written as a WAV file.
    #[error("cannot process the WAV file {path}")]
    Wav {
        /// The path of the file.
        path: PathBuf,
        /// The error of the WAV library.
        #[source]
        source: hound::Error,
    },
    /// A file or a directory cannot be accessed.
    #[error("cannot access {path}")]
    Io {
        /// The path of the file or the directory.
        path: PathBuf,
        /// The error of the operating system.
        #[source]
        source: io::Error,
    },
    /// A text is not a segment key.
    #[error("{text:?} is not 64 lowercase hex characters")]
    InvalidSegmentKey {
        /// The text that did not parse.
        text: String,
    },
    /// The environment variable `ANTENNA_DATA_DIR` is not set, and the platform has no data
    /// directory.
    #[error("the data directory is unknown. Set ANTENNA_DATA_DIR")]
    NoDataDir,
}

impl LibraryError {
    pub(crate) fn io(path: &Path) -> impl FnOnce(io::Error) -> Self {
        let path = path.to_owned();
        |source| Self::Io { path, source }
    }

    pub(crate) fn wav(path: &Path) -> impl FnOnce(hound::Error) -> Self {
        let path = path.to_owned();
        |source| Self::Wav { path, source }
    }
}
