use std::path::PathBuf;

use antenna_core::SampleRate;
use ebur128::{EbuR128, Mode};

use super::progress::Progress;
use crate::AudioError;
use crate::segment_file::read_segment_at_rate;

const TARGET_LOUDNESS_LUFS: f64 = -16.0;
const TRUE_PEAK_LIMIT_DBTP: f64 = -1.0;
const DECIBELS_PER_DECADE: f64 = 20.0;

/// Measures the segments with EBU R128 and returns the gain in decibels that normalizes them.
pub(crate) fn measure_gain(
    segments: &[PathBuf],
    rate: SampleRate,
    progress: &mut Progress<'_>,
) -> Result<f64, AudioError> {
    let mut meter =
        EbuR128::new(1, rate.hz(), Mode::I | Mode::TRUE_PEAK).map_err(AudioError::LoudnessMeter)?;
    for path in segments {
        meter
            .add_frames_f32(&read_segment_at_rate(path, rate)?)
            .map_err(AudioError::LoudnessMeter)?;
        progress.complete_step()?;
    }
    let integrated = meter.loudness_global().map_err(AudioError::LoudnessMeter)?;
    let true_peak = meter.true_peak(0).map_err(AudioError::LoudnessMeter)?;
    Ok(gain_db(integrated, DECIBELS_PER_DECADE * true_peak.log10()))
}

/// Returns the gain that moves the integrated loudness to the target without a true peak above
/// the limit. A loudness that is not finite (silence) has no gain.
fn gain_db(integrated_lufs: f64, true_peak_dbtp: f64) -> f64 {
    if !integrated_lufs.is_finite() {
        return 0.0;
    }
    let gain = TARGET_LOUDNESS_LUFS - integrated_lufs;
    gain.min(TRUE_PEAK_LIMIT_DBTP - true_peak_dbtp)
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "the f32 range holds linear gains up to 760 decibels, and a gain is at most the distance between the loudness of the signal and the target"
)]
pub(crate) fn linear_gain(gain_db: f64) -> f32 {
    10_f64.powf(gain_db / DECIBELS_PER_DECADE) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f64 = 1e-9;

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < EPSILON,
            "{actual} is not {expected}"
        );
    }

    #[test]
    fn gain_reaches_target_when_peak_is_low() {
        assert_close(gain_db(-30.0, -20.0), 14.0);
        assert_close(gain_db(-6.0, -3.0), -10.0);
    }

    #[test]
    fn gain_limits_true_peak_when_crest_high() {
        assert_close(gain_db(-20.0, -2.0), 1.0);
    }

    #[test]
    fn gain_is_zero_when_loudness_infinite() {
        assert_close(gain_db(f64::NEG_INFINITY, f64::NEG_INFINITY), 0.0);
    }

    #[test]
    fn linear_gain_doubles_when_gain_is_six_decibels() {
        assert_close(f64::from(linear_gain(6.0206)), 2.0);
        assert_close(f64::from(linear_gain(0.0)), 1.0);
    }
}
