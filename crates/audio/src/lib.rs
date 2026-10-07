//! Playback and export of audio.

mod error;
mod peaks;
mod resample;
mod segment_file;
#[cfg(test)]
mod test_support;

pub use error::AudioError;
pub use peaks::{PEAKS_PER_SECOND, PeakCache};
pub use resample::resample;
