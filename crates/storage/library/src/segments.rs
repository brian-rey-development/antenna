use std::fmt::{self, Debug, Display, Formatter};
use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Duration;

use antenna_core::{EngineDescriptor, PCM_SCALE, Quality, SampleRate, TextHash, VoiceId};
use hound::{SampleFormat, WavReader, WavSpec, WavWriter};
use sha2::{Digest, Sha256};

use crate::atomic::{self, TempFile};
use crate::{LibraryError, paths};

const KEY_BYTES: usize = 32;
const PCM_BITS: u16 = 16;

/// The name of a stored segment. It is the SHA-256 of the engine, the engine version, the voice,
/// the quality and the segment text, so a change of any of them gives a new key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SegmentKey([u8; KEY_BYTES]);

impl SegmentKey {
    /// Makes the key of a segment text for one engine, voice and quality.
    ///
    /// Each field enters the hash as its length (a `u64` in little-endian byte order) and its
    /// UTF-8 bytes, so two different field lists never give the same bytes.
    pub fn new(
        descriptor: &EngineDescriptor,
        voice: VoiceId,
        quality: Quality,
        text: &str,
    ) -> Self {
        debug_assert_eq!(voice.engine(), descriptor.id);
        let voice_text = voice.to_string();
        let quality_text = quality.to_string();
        let fields = [
            descriptor.id.as_str(),
            descriptor.version,
            &voice_text,
            &quality_text,
            text,
        ];
        let mut hasher = Sha256::new();
        for field in fields {
            hasher.update((field.len() as u64).to_le_bytes());
            hasher.update(field.as_bytes());
        }
        Self(hasher.finalize().into())
    }
}

impl Display for SegmentKey {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&TextHash::new(self.0), formatter)
    }
}

impl FromStr for SegmentKey {
    type Err = LibraryError;

    /// Parses 64 lowercase hex characters.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        // A key has the text form of a text hash, so the core parser is the only hex parser.
        text.parse::<TextHash>()
            .ok()
            .map(|hash| Self(*hash.as_bytes()))
            .ok_or_else(|| LibraryError::InvalidSegmentKey {
                text: text.to_owned(),
            })
    }
}

/// The audio of a stored segment.
#[derive(Clone, Debug, PartialEq)]
pub struct StoredSegment {
    /// The sample rate of the audio.
    pub rate: SampleRate,
    /// The mono samples.
    pub samples: Vec<f32>,
}

/// The directory that keeps the audio of each synthesized segment, in 16-bit WAV files.
///
/// A clone of the store uses the same directory.
#[derive(Clone, Debug)]
pub struct SegmentStore {
    directory: PathBuf,
}

impl SegmentStore {
    /// Opens the segment store in the platform data directory, or in the directory of the
    /// environment variable `ANTENNA_DATA_DIR`.
    ///
    /// # Errors
    ///
    /// Returns [`LibraryError::NoDataDir`] if no data directory is known, and
    /// [`LibraryError::Io`] if the directory cannot be made.
    pub fn open_default() -> Result<Self, LibraryError> {
        Self::open(paths::data_root()?)
    }

    /// Opens the segment store of a data root. The store uses the directory `segments` of the
    /// root, and makes it if it does not exist.
    ///
    /// # Errors
    ///
    /// Returns [`LibraryError::Io`] if the directory cannot be made.
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, LibraryError> {
        let directory = paths::segments_dir(&root.into());
        fs::create_dir_all(&directory).map_err(LibraryError::io(&directory))?;
        Ok(Self { directory })
    }

    /// Returns `true` if the store has a file for the key.
    pub fn contains(&self, key: &SegmentKey) -> bool {
        self.path(key).is_file()
    }

    /// Returns the path of the file for the key. The file can be absent.
    pub fn path(&self, key: &SegmentKey) -> PathBuf {
        paths::segment_path(&self.directory, key)
    }

    /// Starts a segment file for the key. The file is not visible until [`SegmentWriter::commit`].
    ///
    /// # Errors
    ///
    /// Returns [`LibraryError::Io`] or [`LibraryError::Wav`] if the temporary file cannot be made.
    pub fn writer(&self, key: SegmentKey, rate: SampleRate) -> Result<SegmentWriter, LibraryError> {
        let target = self.path(&key);
        let parent = target.parent().unwrap_or(&self.directory);
        fs::create_dir_all(parent).map_err(LibraryError::io(parent))?;
        let temp_file = TempFile::beside(&target);
        let spec = WavSpec {
            channels: 1,
            sample_rate: rate.hz(),
            bits_per_sample: PCM_BITS,
            sample_format: SampleFormat::Int,
        };
        let wav = WavWriter::create(temp_file.path(), spec)
            .map_err(LibraryError::wav(temp_file.path()))?;
        Ok(SegmentWriter {
            wav,
            temp: temp_file,
            target,
            rate,
        })
    }

    /// Reads the audio of a stored segment.
    ///
    /// # Errors
    ///
    /// Returns [`LibraryError::Wav`] if the file is absent or is not a valid segment file.
    pub fn read(&self, key: &SegmentKey) -> Result<StoredSegment, LibraryError> {
        let path = self.path(key);
        let (reader, rate) = open_reader(&path)?;
        let samples = reader
            .into_samples::<i16>()
            .map(|sample| sample.map(from_pcm))
            .collect::<Result<Vec<_>, _>>()
            .map_err(LibraryError::wav(&path))?;
        Ok(StoredSegment { rate, samples })
    }

    /// Returns the duration of a stored segment. The function reads only the WAV header.
    ///
    /// # Errors
    ///
    /// Returns [`LibraryError::Wav`] if the file is absent or is not a valid segment file.
    pub fn duration(&self, key: &SegmentKey) -> Result<Duration, LibraryError> {
        let (reader, rate) = open_reader(&self.path(key))?;
        Ok(rate.duration_of(u64::from(reader.duration())))
    }
}

/// A segment file in progress. If the writer drops before [`SegmentWriter::commit`], it deletes
/// its temporary file.
pub struct SegmentWriter {
    wav: WavWriter<BufWriter<File>>,
    temp: TempFile,
    target: PathBuf,
    rate: SampleRate,
}

impl Debug for SegmentWriter {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SegmentWriter")
            .field("target", &self.target)
            .field("rate", &self.rate)
            .finish_non_exhaustive()
    }
}

impl SegmentWriter {
    /// Appends mono samples. Each sample is clamped to the range -1.0 to 1.0.
    ///
    /// # Errors
    ///
    /// Returns [`LibraryError::Wav`] if the file cannot be written.
    pub fn write(&mut self, samples: &[f32]) -> Result<(), LibraryError> {
        samples
            .iter()
            .try_for_each(|&sample| self.wav.write_sample(to_pcm(sample)))
            .map_err(LibraryError::wav(self.temp.path()))
    }

    /// Finishes the file and makes it visible in the store. If the store has a file for the key
    /// already, the writer keeps that file, because engines are deterministic. The function
    /// returns the duration of the audio.
    ///
    /// # Errors
    ///
    /// Returns [`LibraryError::Wav`] or [`LibraryError::Io`] if the file cannot be finished or
    /// renamed.
    pub fn commit(self) -> Result<Duration, LibraryError> {
        let duration = self.rate.duration_of(u64::from(self.wav.duration()));
        self.wav
            .finalize()
            .map_err(LibraryError::wav(self.temp.path()))?;
        atomic::sync_path(self.temp.path())?;
        if !self.target.exists() {
            self.temp.persist(&self.target)?;
        }
        Ok(duration)
    }
}

fn open_reader(path: &Path) -> Result<(WavReader<BufReader<File>>, SampleRate), LibraryError> {
    let reader = WavReader::open(path).map_err(LibraryError::wav(path))?;
    let hz = NonZeroU32::new(reader.spec().sample_rate);
    let rate = hz.map(SampleRate::new).ok_or_else(|| LibraryError::Wav {
        path: path.to_owned(),
        source: hound::Error::FormatError("the sample rate is zero"),
    })?;
    Ok((reader, rate))
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "the clamped sample times PCM_SCALE is in -32767..=32767"
)]
fn to_pcm(sample: f32) -> i16 {
    (sample.clamp(-1.0, 1.0) * PCM_SCALE).round() as i16
}

fn from_pcm(sample: i16) -> f32 {
    f32::from(sample) / PCM_SCALE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_pcm_scales_with_pcm_scale_when_in_range() {
        assert_eq!(to_pcm(0.0), 0);
        assert_eq!(to_pcm(1.0), 32_767);
        assert_eq!(to_pcm(-1.0), -32_767);
        assert_eq!(to_pcm(0.5), 16_384);
    }

    #[test]
    fn to_pcm_clamps_when_sample_out_of_range() {
        assert_eq!(to_pcm(2.5), 32_767);
        assert_eq!(to_pcm(-9.0), -32_767);
        assert_eq!(to_pcm(f32::INFINITY), 32_767);
    }

    #[test]
    fn to_pcm_gives_silence_when_sample_not_a_number() {
        assert_eq!(to_pcm(f32::NAN), 0);
    }

    #[test]
    fn from_pcm_divides_by_pcm_scale() {
        assert!((from_pcm(32_767) - 1.0).abs() <= f32::EPSILON);
        assert!((from_pcm(-32_767) + 1.0).abs() <= f32::EPSILON);
        assert!(from_pcm(0).abs() <= f32::EPSILON);
    }
}
