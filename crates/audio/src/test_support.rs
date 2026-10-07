#[path = "../tests/fixtures/mod.rs"]
mod fixtures;

use antenna_core::PCM_SCALE;

pub(crate) use fixtures::{sine, write_segment};

#[expect(
    clippy::cast_precision_loss,
    reason = "test signals have fewer than 2^24 samples"
)]
pub(crate) fn rms(samples: &[f32]) -> f32 {
    let energy: f32 = samples.iter().map(|sample| sample * sample).sum();
    (energy / samples.len() as f32).sqrt()
}

#[expect(clippy::cast_precision_loss, reason = "the index is below 30000")]
pub(crate) fn pcm_ramp(start: usize, len: usize) -> Vec<f32> {
    (start..start + len)
        .map(|index| (index % 30_000) as f32 / PCM_SCALE)
        .collect()
}

pub(crate) fn pcm_exact(samples: &[f32]) -> Vec<f32> {
    samples
        .iter()
        .map(|sample| (sample * PCM_SCALE).round() / PCM_SCALE)
        .collect()
}

pub(crate) fn assert_same(actual: &[f32], expected: &[f32]) {
    let bits = |samples: &[f32]| {
        samples
            .iter()
            .map(|sample| sample.to_bits())
            .collect::<Vec<_>>()
    };
    assert_eq!(bits(actual), bits(expected));
}
