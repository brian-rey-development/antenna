use std::path::PathBuf;

use antenna_core::{PCM_SCALE, SampleRate};
use hound::{SampleFormat, WavSpec, WavWriter};

use super::part_file::PartFile;
use crate::AudioError;
use crate::segment_file::PCM_BITS_PER_SAMPLE;

pub(super) struct WavEncoder<'a> {
    writer: WavWriter<&'a mut PartFile>,
    path: PathBuf,
}

impl<'a> WavEncoder<'a> {
    pub(super) fn new(part: &'a mut PartFile, rate: SampleRate) -> Result<Self, AudioError> {
        let path = part.destination().to_owned();
        let spec = WavSpec {
            channels: 1,
            sample_rate: rate.hz(),
            bits_per_sample: PCM_BITS_PER_SAMPLE,
            sample_format: SampleFormat::Int,
        };
        let writer = WavWriter::new(part, spec).map_err(|source| AudioError::WriteWav {
            path: path.clone(),
            source,
        })?;
        Ok(Self { writer, path })
    }

    fn wav_error(&self, source: hound::Error) -> AudioError {
        AudioError::WriteWav {
            path: self.path.clone(),
            source,
        }
    }
}

impl WavEncoder<'_> {
    pub(super) fn write(&mut self, samples: &[f32]) -> Result<(), AudioError> {
        for sample in samples {
            self.writer
                .write_sample(to_pcm(*sample))
                .map_err(|source| self.wav_error(source))?;
        }
        Ok(())
    }

    pub(super) fn finish(self) -> Result<(), AudioError> {
        let Self { writer, path } = self;
        writer
            .finalize()
            .map_err(|source| AudioError::WriteWav { path, source })
    }
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "the clamped sample times the scale is at most 32767 in magnitude"
)]
fn to_pcm(sample: f32) -> i16 {
    (sample.clamp(-1.0, 1.0) * PCM_SCALE).round() as i16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_pcm_scales_when_sample_in_range() {
        assert_eq!(to_pcm(1.0), i16::MAX);
        assert_eq!(to_pcm(-1.0), -i16::MAX);
        assert_eq!(to_pcm(0.5), 16_384);
    }

    #[test]
    fn to_pcm_clamps_when_sample_out_of_range() {
        assert_eq!(to_pcm(3.0), i16::MAX);
        assert_eq!(to_pcm(-3.0), -i16::MAX);
    }

    #[test]
    fn to_pcm_is_zero_when_sample_not_a_number() {
        assert_eq!(to_pcm(f32::NAN), 0);
    }
}
