use std::num::NonZeroU32;
use std::time::Duration;

use crate::CoreError;

/// The scale of a 16-bit stored sample. The segment store writes a sample of 1.0 as 32767.
pub const PCM_SCALE: f32 = 32_767.0;

const NANOS_PER_SECOND: u64 = 1_000_000_000;

/// The number of samples in one second of audio.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SampleRate(NonZeroU32);

impl SampleRate {
    /// 16 kHz, the input rate of Whisper.
    pub const HZ_16000: Self = Self::constant(16_000);
    /// 22.05 kHz.
    pub const HZ_22050: Self = Self::constant(22_050);
    /// 24 kHz.
    pub const HZ_24000: Self = Self::constant(24_000);
    /// 44.1 kHz.
    pub const HZ_44100: Self = Self::constant(44_100);
    /// 48 kHz.
    pub const HZ_48000: Self = Self::constant(48_000);

    /// Makes a sample rate for a constant.
    pub const fn new(hz: NonZeroU32) -> Self {
        Self(hz)
    }

    #[expect(
        clippy::panic,
        reason = "only the rate constants call this function, so a zero is a compile error"
    )]
    const fn constant(hz: u32) -> Self {
        match NonZeroU32::new(hz) {
            Some(hz) => Self(hz),
            None => panic!("the sample rate is zero"),
        }
    }

    /// Returns the rate in hertz.
    pub const fn hz(self) -> u32 {
        self.0.get()
    }

    /// Returns the duration of a number of samples at this rate, rounded down to the nanosecond.
    pub fn duration_of(self, samples: u64) -> Duration {
        let hz = u64::from(self.hz());
        let nanos = samples % hz * NANOS_PER_SECOND / hz;
        Duration::from_secs(samples / hz) + Duration::from_nanos(nanos)
    }
}

impl TryFrom<u32> for SampleRate {
    type Error = CoreError;

    /// Parses a sample rate from outside the program, for example a device rate.
    fn try_from(hz: u32) -> Result<Self, Self::Error> {
        NonZeroU32::new(hz)
            .map(Self)
            .ok_or(CoreError::ZeroSampleRate)
    }
}

/// A sequence of mono samples that an engine emits in one call of the emit callback.
#[derive(Clone, Debug, PartialEq)]
pub struct PcmChunk(Vec<f32>);

impl PcmChunk {
    /// Makes a chunk from mono samples at the sample rate of the engine.
    pub fn new(samples: Vec<f32>) -> Self {
        Self(samples)
    }

    /// Returns the samples.
    pub fn samples(&self) -> &[f32] {
        &self.0
    }

    /// Returns the samples and drops the chunk.
    pub fn into_samples(self) -> Vec<f32> {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_rate_try_from_fails_when_zero() {
        let result = SampleRate::try_from(0);

        assert_eq!(result, Err(CoreError::ZeroSampleRate));
    }

    #[test]
    fn sample_rate_try_from_keeps_hz_when_not_zero() {
        let rate = SampleRate::try_from(44_100).unwrap();

        assert_eq!(rate, SampleRate::HZ_44100);
        assert_eq!(rate.hz(), 44_100);
    }

    #[test]
    fn sample_rate_duration_of_divides_by_rate() {
        let rate = SampleRate::HZ_24000;

        assert_eq!(rate.duration_of(0), Duration::ZERO);
        assert_eq!(rate.duration_of(960), Duration::from_millis(40));
        assert_eq!(rate.duration_of(36_000), Duration::from_millis(1_500));
        assert_eq!(
            SampleRate::HZ_22050.duration_of(1),
            Duration::from_nanos(45_351)
        );
    }

    #[test]
    fn pcm_chunk_keeps_samples() {
        let chunk = PcmChunk::new(vec![0.5, -0.5]);

        assert_eq!(chunk.samples(), [0.5, -0.5]);
        assert_eq!(chunk.into_samples(), vec![0.5, -0.5]);
    }

    #[test]
    fn sample_rate_duration_of_does_not_overflow_when_samples_maximum() {
        let duration = SampleRate::HZ_16000.duration_of(u64::MAX);

        assert_eq!(duration.as_secs(), u64::MAX / 16_000);
    }
}
