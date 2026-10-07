use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use antenna_core::{PCM_SCALE, SampleRate};
use hound::{SampleFormat, WavReader};

use crate::AudioError;

pub(crate) const PCM_BITS_PER_SAMPLE: u16 = 16;

pub(crate) struct SegmentFile {
    path: PathBuf,
    rate: SampleRate,
    reader: WavReader<BufReader<File>>,
}

impl SegmentFile {
    pub(crate) fn open(path: &Path) -> Result<Self, AudioError> {
        let reader = WavReader::open(path).map_err(|source| AudioError::ReadSegment {
            path: path.to_owned(),
            source,
        })?;
        let spec = reader.spec();
        let unsupported = || AudioError::UnsupportedSegment {
            path: path.to_owned(),
            spec,
        };
        if spec.channels != 1
            || spec.bits_per_sample != PCM_BITS_PER_SAMPLE
            || spec.sample_format != SampleFormat::Int
        {
            return Err(unsupported());
        }
        let rate = SampleRate::try_from(spec.sample_rate)
            .ok()
            .ok_or_else(unsupported)?;
        Ok(Self {
            path: path.to_owned(),
            rate,
            reader,
        })
    }

    pub(crate) fn rate(&self) -> SampleRate {
        self.rate
    }

    pub(crate) fn into_samples(mut self) -> Result<Vec<f32>, AudioError> {
        self.reader
            .samples::<i16>()
            .map(|sample| sample.map(|pcm| f32::from(pcm) / PCM_SCALE))
            .collect::<Result<_, _>>()
            .map_err(|source| AudioError::ReadSegment {
                path: self.path,
                source,
            })
    }
}

#[cfg(test)]
mod tests {
    use hound::{WavSpec, WavWriter};

    use super::*;

    fn spec(channels: u16, bits_per_sample: u16) -> WavSpec {
        WavSpec {
            channels,
            sample_rate: 24_000,
            bits_per_sample,
            sample_format: SampleFormat::Int,
        }
    }

    fn file(spec: WavSpec) -> (tempfile::TempDir, PathBuf) {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("a.wav");
        let mut writer = WavWriter::create(&path, spec).unwrap();
        writer.write_sample(1_i16).unwrap();
        writer.write_sample(1_i16).unwrap();
        writer.finalize().unwrap();
        (directory, path)
    }

    #[test]
    fn segment_file_reads_samples_when_opened() {
        let (_directory, path) = file(spec(1, 16));

        let samples = SegmentFile::open(&path).unwrap().into_samples().unwrap();

        assert_eq!(samples.len(), 2);
    }

    #[test]
    fn segment_file_reports_rate_when_opened() {
        let (_directory, path) = file(spec(1, 16));

        let segment = SegmentFile::open(&path).unwrap();

        assert_eq!(segment.rate(), SampleRate::HZ_24000);
    }

    #[test]
    fn segment_file_fails_when_file_is_stereo() {
        let (_directory, path) = file(spec(2, 16));

        let error = SegmentFile::open(&path).err().unwrap();

        assert!(matches!(error, AudioError::UnsupportedSegment { .. }));
    }

    #[test]
    fn segment_file_fails_when_file_is_24_bit() {
        let (_directory, path) = file(spec(1, 24));

        let error = SegmentFile::open(&path).err().unwrap();

        assert!(matches!(error, AudioError::UnsupportedSegment { .. }));
    }

    #[test]
    fn segment_file_fails_when_file_missing() {
        let directory = tempfile::tempdir().unwrap();

        let error = SegmentFile::open(&directory.path().join("missing.wav"))
            .err()
            .unwrap();

        assert!(matches!(error, AudioError::ReadSegment { .. }));
    }
}
