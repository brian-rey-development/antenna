//! The model store of Antenna.
//!
//! [`ModelStore::ensure`] installs the artifacts of a voice as local files with a correct SHA-256.
//! A download has progress, resume, byte ranges, retries and cancellation. The store never gives an
//! engine a file that failed a check.

mod error;
mod install;
mod layout;
mod progress;
mod retry;
mod source;
mod store;
mod usage;
mod verify;

pub use error::ModelError;
pub use layout::{engine_artifacts, voice_artifacts};
pub use progress::DownloadProgress;
pub use source::Source;
pub use store::ModelStore;
pub use usage::{engine_download_bytes, keep_artifacts};
