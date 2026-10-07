//! Playback and export of audio.
//!
//! The [`Player`] plays a [`Track`] of stored, live and pending segments. The [`export`] function
//! writes stored segments to an MP3, WAV or Ogg Opus file.

mod error;
mod export;
mod frames;
mod live;
mod peaks;
mod player;
mod resample;
mod segment_file;
#[cfg(test)]
mod test_support;
mod track;
mod volume;

#[cfg(test)]
#[global_allocator]
static ALLOCATOR: assert_no_alloc::AllocDisabler = assert_no_alloc::AllocDisabler;

pub use error::AudioError;
pub use export::{EpisodeTags, ExportRate, ExportSpec, ExportSummary, Loudness, export};
pub use live::LiveOutput;
pub use peaks::{PEAKS_PER_SECOND, PeakCache};
pub use player::{Player, PlayerEvent, PlayerState, TrackHandle};
pub use resample::resample;
pub use track::{Track, TrackId, TrackPosition};
pub use volume::Volume;
