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

pub(crate) fn pcm_exact(samples: &[f32]) -> Vec<f32> {
    samples
        .iter()
        .map(|sample| (sample * PCM_SCALE).round() / PCM_SCALE)
        .collect()
}
