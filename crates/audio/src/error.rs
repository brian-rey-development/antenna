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
    /// A stored segment has another sample rate than the export expects.
    #[error("the segment {path} has {actual} Hz, and the export expects {expected} Hz")]
    RateMismatch {
        /// The path of the segment file.
        path: PathBuf,
        /// The sample rate that the export expects, in hertz.
        expected: u32,
        /// The sample rate of the file, in hertz.
        actual: u32,
    },
    /// A stored segment is not 16-bit mono PCM.
    #[error("the segment {path} is not 16-bit mono PCM, and its format is {spec:?}")]
    UnsupportedSegment {
        /// The path of the segment file.
        path: PathBuf,
        /// The format of the file.
        spec: hound::WavSpec,
    },
    /// A WAV export cannot be written.
    #[error("cannot write the WAV file {path}")]
    WriteWav {
        /// The path of the export file.
        path: PathBuf,
        /// The error of the WAV writer.
        #[source]
        source: hound::Error,
    },
    /// An export file cannot be written.
    #[error("cannot write the file {path}")]
    Write {
        /// The path of the file.
        path: PathBuf,
        /// The error of the file system.
        #[source]
        source: io::Error,
    },
    /// The MP3 encoder cannot be built.
    #[error("cannot build the MP3 encoder")]
    Mp3Build(#[source] mp3lame_encoder::BuildError),
    /// The MP3 encoder failed.
    #[error("the MP3 encoder failed")]
    Mp3Encode(#[source] mp3lame_encoder::EncodeError),
    /// The MP3 tag is not valid.
    #[error("cannot set the MP3 tag, the error is {0:?}")]
    Mp3Tag(mp3lame_encoder::Id3TagError),
    /// The Opus encoder failed.
    #[error("the Opus encoder failed")]
    Opus(#[source] opus::Error),
    /// The loudness meter failed.
    #[error("the loudness meter failed")]
    LoudnessMeter(#[source] ebur128::Error),
    /// The caller cancelled the export.
    #[error("the caller canceled the export")]
    Cancelled,
}
