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

pub(crate) fn read_segment(path: &Path) -> Result<Vec<f32>, AudioError> {
    SegmentFile::open(path)?.into_samples()
}

#[cfg(test)]
mod tests {
    use hound::{WavSpec, WavWriter};

    use super::*;

    fn write(path: &Path, spec: WavSpec, samples: &[i16]) {
        let mut writer = WavWriter::create(path, spec).unwrap();
        for sample in samples {
            writer.write_sample(*sample).unwrap();
        }
        writer.finalize().unwrap();
    }

    fn spec(channels: u16, bits_per_sample: u16) -> WavSpec {
        WavSpec {
            channels,
            sample_rate: 24_000,
            bits_per_sample,
            sample_format: SampleFormat::Int,
        }
    }

    fn file(spec: WavSpec, samples: &[i16]) -> (tempfile::TempDir, PathBuf) {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("a.wav");
        write(&path, spec, samples);
        (directory, path)
    }

    #[test]
    fn read_segment_scales_when_sample_is_max() {
        let (_directory, path) = file(spec(1, 16), &[i16::MAX, 0, -i16::MAX]);

        let samples = read_segment(&path).unwrap();

        assert_eq!(samples, [1.0, 0.0, -1.0]);
    }

    #[test]
    fn read_segment_returns_nothing_when_file_has_no_samples() {
        let (_directory, path) = file(spec(1, 16), &[]);

        assert!(read_segment(&path).unwrap().is_empty());
    }

    #[test]
    fn segment_file_reports_rate_when_opened() {
        let (_directory, path) = file(spec(1, 16), &[1]);

        let segment = SegmentFile::open(&path).unwrap();

        assert_eq!(segment.rate(), SampleRate::HZ_24000);
    }

    #[test]
    fn read_segment_fails_when_file_is_stereo() {
        let (_directory, path) = file(spec(2, 16), &[1, 2]);

        let error = read_segment(&path).unwrap_err();

        assert!(matches!(error, AudioError::UnsupportedSegment { .. }));
    }

    #[test]
    fn read_segment_fails_when_file_is_24_bit() {
        let (_directory, path) = file(spec(1, 24), &[1]);

        let error = read_segment(&path).unwrap_err();

        assert!(matches!(error, AudioError::UnsupportedSegment { .. }));
    }

    #[test]
    fn read_segment_fails_when_file_is_float() {
        let float = WavSpec {
            sample_format: SampleFormat::Float,
            bits_per_sample: 32,
            ..spec(1, 32)
        };
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("float.wav");
        let mut writer = WavWriter::create(&path, float).unwrap();
        writer.write_sample(0.5_f32).unwrap();
        writer.finalize().unwrap();

        let error = read_segment(&path).unwrap_err();

        assert!(matches!(error, AudioError::UnsupportedSegment { .. }));
    }

    #[test]
    fn read_segment_fails_when_file_missing() {
        let directory = tempfile::tempdir().unwrap();

        let error = read_segment(&directory.path().join("missing.wav")).unwrap_err();

        assert!(matches!(error, AudioError::ReadSegment { .. }));
    }
}
