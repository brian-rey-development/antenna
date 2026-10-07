use std::io;
use std::path::PathBuf;

use thiserror::Error;

/// An error of the player or of an export.
#[derive(Debug, Error)]
pub enum AudioError {
    /// The system has no default output device.
    #[error("the system has no default audio output device")]
    NoDevice,
    /// The default output device has no usable stream configuration.
    #[error("cannot read the stream configuration of the output device")]
    DeviceConfig(#[source] cpal::Error),
    /// The output device reports a configuration that the player cannot use.
    #[error("the output device reports {channels} channels at {sample_rate} Hz")]
    InvalidDeviceConfig {
        /// The channel count of the device.
        channels: u16,
        /// The sample rate of the device in hertz.
        sample_rate: u32,
    },
    /// The output device uses a sample format that the player does not support.
    #[error("the output device uses the sample format {0}, and the player does not support it")]
    UnsupportedFormat(cpal::SampleFormat),
    /// The output stream cannot be built.
    #[error("cannot build the output stream")]
    BuildStream(#[source] cpal::Error),
    /// The output stream cannot start.
    #[error("cannot start the output stream")]
    PlayStream(#[source] cpal::Error),
    /// The feed thread of the player cannot start.
    #[error("cannot start the thread of the player")]
    SpawnThread(#[source] io::Error),
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
