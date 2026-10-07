//! Test signals and segment files. The unit tests, the integration tests, the benchmark and the
//! example include this file.

use std::f32::consts::TAU;
use std::path::Path;

use antenna_core::{PCM_SCALE, SampleRate};
use hound::{SampleFormat, WavSpec, WavWriter};

#[expect(
    clippy::cast_precision_loss,
    reason = "the signals have fewer than 2^24 frames, and a rate below 2^24 is exact"
)]
pub(crate) fn sine(hz: f32, rate: SampleRate, frames: usize, amplitude: f32) -> Vec<f32> {
    let step = TAU * hz / rate.hz() as f32;
    (0..frames)
        .map(|index| amplitude * (step * index as f32).sin())
        .collect()
}

pub(crate) fn write_segment(
    path: &Path,
    rate: SampleRate,
    samples: &[f32],
) -> Result<(), hound::Error> {
    let spec = WavSpec {
        channels: 1,
        sample_rate: rate.hz(),
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };
    let mut writer = WavWriter::create(path, spec)?;
    for sample in samples {
        writer.write_sample(to_pcm(*sample))?;
    }
    writer.finalize()
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "the clamped sample times the scale is at most 32767 in magnitude"
)]
fn to_pcm(sample: f32) -> i16 {
    (sample.clamp(-1.0, 1.0) * PCM_SCALE).round() as i16
}
