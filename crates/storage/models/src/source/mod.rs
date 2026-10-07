mod directory;
mod hub;
mod stall;

use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use antenna_core::{Artifact, Extent};

use crate::ModelError;

pub(crate) use directory::DirectoryFetch;
pub(crate) use hub::{HubFetch, READ_TIMEOUT};

const READ_BLOCK_BYTES: usize = 65_536;

/// Where the model store gets the bytes of an artifact.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Source {
    /// A Hugging Face server.
    Hub {
        /// The base URL of the server. `None` selects `https://huggingface.co`.
        endpoint: Option<String>,
    },
    /// A local directory with the layout `<owner>/<name>/<revision>/<path>`.
    Directory(PathBuf),
}

/// A way to fill the partial file of an artifact.
pub(crate) trait Fetch {
    /// Appends the missing bytes of the artifact to the partial file. The `u64` that `progress`
    /// gets is the number of bytes that this call wrote so far.
    fn fetch(
        &self,
        artifact: &'static Artifact,
        partial: &Path,
        progress: &dyn Fn(u64),
        cancel: &AtomicBool,
    ) -> Result<(), FetchError>;
}

pub(crate) enum FetchError {
    Transient(ureq::Error),
    Permanent(ModelError),
    Cancelled,
}

impl FetchError {
    pub(crate) fn io(path: &Path) -> impl FnOnce(io::Error) -> Self + '_ {
        |source| Self::Permanent(ModelError::io(path)(source))
    }
}

enum CopyEnd {
    Finished,
    Cancelled,
}

enum CopyError {
    Read(io::Error),
    Write(io::Error),
}

fn start_offset(extent: Extent) -> u64 {
    match extent {
        Extent::Whole { .. } => 0,
        Extent::Range { offset, .. } => offset,
    }
}

fn copy_blocks(
    reader: &mut impl Read,
    output: &mut impl Write,
    progress: &dyn Fn(u64),
    cancel: &AtomicBool,
) -> Result<CopyEnd, CopyError> {
    let mut block = vec![0_u8; READ_BLOCK_BYTES];
    let mut written = 0_u64;
    loop {
        let count = reader.read(&mut block).map_err(CopyError::Read)?;
        if count == 0 {
            return Ok(CopyEnd::Finished);
        }
        output
            .write_all(block.split_at(count).0)
            .map_err(CopyError::Write)?;
        written += count as u64;
        progress(written);
        if cancel.load(Ordering::Relaxed) {
            return Ok(CopyEnd::Cancelled);
        }
    }
}

fn finish_copy(
    result: Result<CopyEnd, CopyError>,
    partial: &Path,
    read_error: impl FnOnce(io::Error) -> FetchError,
) -> Result<(), FetchError> {
    match result {
        Ok(CopyEnd::Finished) => Ok(()),
        Ok(CopyEnd::Cancelled) => Err(FetchError::Cancelled),
        Err(CopyError::Read(source)) => Err(read_error(source)),
        Err(CopyError::Write(source)) => Err(FetchError::io(partial)(source)),
    }
}
