use std::time::Duration;

use antenna_core::SampleRate;

const NANOS_PER_SECOND: u128 = 1_000_000_000;

fn frames_in(duration: Duration, rate: SampleRate) -> i64 {
    let frames = duration.as_nanos() * u128::from(rate.hz()) / NANOS_PER_SECOND;
    i64::try_from(frames).unwrap_or(i64::MAX)
}

pub(crate) fn sample_count(duration: Duration, rate: SampleRate) -> usize {
    usize::try_from(frames_in(duration, rate)).unwrap_or(usize::MAX)
}

pub(crate) fn rescale(frames: i64, from: SampleRate, to: SampleRate) -> i64 {
    let numerator = i128::from(frames) * i128::from(to.hz());
    let denominator = i128::from(from.hz());
    let rounded = (2 * numerator + denominator).div_euclid(2 * denominator);
    i64::try_from(rounded).unwrap_or(if frames < 0 { i64::MIN } else { i64::MAX })
}

#[cfg(test)]
mod tests {
    use super::*;

    const FROM: SampleRate = SampleRate::HZ_24000;
    const TO: SampleRate = SampleRate::HZ_44100;

    #[test]
    fn frames_in_rounds_down_when_duration_not_whole() {
        let frames = frames_in(Duration::from_micros(41_666), FROM);

        assert_eq!(frames, 999);
    }

    #[test]
    fn frames_in_counts_whole_seconds() {
        assert_eq!(
            frames_in(Duration::from_secs(2), SampleRate::HZ_48000),
            96_000
        );
    }

    #[test]
    fn sample_count_saturates_when_duration_is_huge() {
        assert_eq!(
            sample_count(Duration::MAX, FROM),
            usize::try_from(i64::MAX).unwrap()
        );
    }

    #[test]
    fn rescale_converts_whole_ratio() {
        assert_eq!(rescale(24_000, FROM, TO), 44_100);
    }

    #[test]
    fn rescale_mirrors_when_frames_negative() {
        assert_eq!(rescale(-24_000, FROM, TO), -44_100);
    }

    #[test]
    fn rescale_rounds_half_up_when_frames_negative_or_positive() {
        assert_eq!(rescale(1, SampleRate::HZ_48000, SampleRate::HZ_24000), 1);
        assert_eq!(rescale(-1, SampleRate::HZ_48000, SampleRate::HZ_24000), 0);
    }

    #[test]
    fn rescale_saturates_when_result_overflows() {
        assert_eq!(rescale(i64::MAX, FROM, TO), i64::MAX);
        assert_eq!(rescale(i64::MIN, FROM, TO), i64::MIN);
    }
}
