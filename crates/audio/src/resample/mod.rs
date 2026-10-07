mod fft;

use antenna_core::SampleRate;

use fft::FftConversion;

pub(crate) const RESAMPLER_CHUNK_FRAMES: usize = 1_024;

/// Converts the samples of one rate to another rate, with a state for streaming.
#[derive(Debug)]
pub(crate) enum Conversion {
    Direct,
    Resample(Box<FftConversion>),
}

impl Conversion {
    pub(crate) fn new(from: SampleRate, to: SampleRate) -> Self {
        if from == to {
            return Self::Direct;
        }
        Self::Resample(Box::new(FftConversion::new(from, to)))
    }

    pub(crate) fn convert(&mut self, input: &[f32], output: &mut Vec<f32>) {
        match self {
            Self::Direct => output.extend_from_slice(input),
            Self::Resample(fft) => fft.convert(input, output),
        }
    }

    pub(crate) fn finish(&mut self, output: &mut Vec<f32>) {
        match self {
            Self::Direct => {}
            Self::Resample(fft) => fft.finish(output),
        }
    }

    pub(crate) fn reset(&mut self) {
        match self {
            Self::Direct => {}
            Self::Resample(fft) => fft.reset(),
        }
    }
}

/// Converts mono samples from one sample rate to another.
///
/// The output has `ceil(samples.len() * to / from)` samples, and the same start time as the input.
pub fn resample(samples: &[f32], from: SampleRate, to: SampleRate) -> Vec<f32> {
    let mut conversion = Conversion::new(from, to);
    let mut output = Vec::with_capacity(samples.len() * to.hz() as usize / from.hz() as usize + 1);
    conversion.convert(samples, &mut output);
    conversion.finish(&mut output);
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{rms, sine};

    const SINE_HZ: f32 = 1_000.0;

    fn expected_length(samples: usize, from: SampleRate, to: SampleRate) -> usize {
        (samples * to.hz() as usize).div_ceil(from.hz() as usize)
    }

    fn resampled_length(samples: usize, from: SampleRate, to: SampleRate) -> usize {
        let input = sine(SINE_HZ, from, samples, 0.5);

        resample(&input, from, to).len()
    }

    fn level_difference_db(from: SampleRate) -> f32 {
        let input = sine(SINE_HZ, from, from.hz() as usize, 0.5);

        let output = resample(&input, from, SampleRate::HZ_48000);

        let (_, middle) = output.split_at(output.len() / 4);
        20.0 * (rms(middle) / rms(&input)).log10()
    }

    #[test]
    fn resample_keeps_length_when_24000_to_48000() {
        let (from, to) = (SampleRate::HZ_24000, SampleRate::HZ_48000);

        assert_eq!(
            resampled_length(24_000, from, to),
            expected_length(24_000, from, to)
        );
    }

    #[test]
    fn resample_keeps_length_when_22050_to_48000() {
        let (from, to) = (SampleRate::HZ_22050, SampleRate::HZ_48000);

        assert_eq!(
            resampled_length(22_050, from, to),
            expected_length(22_050, from, to)
        );
    }

    #[test]
    fn resample_rounds_length_up_when_ratio_is_not_whole() {
        let (from, to) = (SampleRate::HZ_22050, SampleRate::HZ_48000);

        assert_eq!(resampled_length(1_000, from, to), 2_177);
    }

    #[test]
    fn resample_keeps_length_when_input_is_one_sample() {
        let (from, to) = (SampleRate::HZ_44100, SampleRate::HZ_16000);

        assert_eq!(resampled_length(1, from, to), 1);
    }

    #[test]
    fn resample_keeps_length_when_input_is_one_chunk() {
        let (from, to) = (SampleRate::HZ_24000, SampleRate::HZ_44100);

        let length = resampled_length(RESAMPLER_CHUNK_FRAMES, from, to);

        assert_eq!(length, expected_length(RESAMPLER_CHUNK_FRAMES, from, to));
    }

    #[test]
    fn resample_keeps_length_when_input_is_one_chunk_plus_one() {
        let (from, to) = (SampleRate::HZ_24000, SampleRate::HZ_44100);

        let length = resampled_length(RESAMPLER_CHUNK_FRAMES + 1, from, to);

        assert_eq!(
            length,
            expected_length(RESAMPLER_CHUNK_FRAMES + 1, from, to)
        );
    }

    #[test]
    fn resample_keeps_level_when_sine() {
        assert!(level_difference_db(SampleRate::HZ_24000).abs() <= 0.5);
    }

    #[test]
    fn resample_keeps_level_when_sine_from_22050() {
        assert!(level_difference_db(SampleRate::HZ_22050).abs() <= 0.5);
    }

    #[test]
    fn resample_keeps_phase_when_sine() {
        let input = sine(SINE_HZ, SampleRate::HZ_24000, 24_000, 0.5);

        let output = resample(&input, SampleRate::HZ_24000, SampleRate::HZ_48000);

        let expected = sine(SINE_HZ, SampleRate::HZ_48000, output.len(), 0.5);
        let middle = output.len() / 4..output.len() / 2;
        let error = rms(&output[middle.clone()]
            .iter()
            .zip(&expected[middle])
            .map(|(actual, wanted)| actual - wanted)
            .collect::<Vec<_>>());
        assert!(error < 0.01, "error {error}");
    }

    #[test]
    fn resample_returns_nothing_when_input_empty() {
        assert!(resample(&[], SampleRate::HZ_24000, SampleRate::HZ_48000).is_empty());
    }

    #[test]
    fn conversion_copies_samples_when_rates_equal() {
        let mut conversion = Conversion::new(SampleRate::HZ_24000, SampleRate::HZ_24000);
        let mut output = Vec::new();

        conversion.convert(&[0.1, 0.2], &mut output);
        conversion.finish(&mut output);

        assert!(matches!(conversion, Conversion::Direct));
        assert_eq!(output, [0.1, 0.2]);
    }

    fn convert_in_pieces(piece: usize) -> Vec<f32> {
        let input = sine(SINE_HZ, SampleRate::HZ_22050, 5_000, 0.5);
        let mut conversion = Conversion::new(SampleRate::HZ_22050, SampleRate::HZ_48000);
        let mut output = Vec::new();

        input
            .chunks(piece)
            .for_each(|part| conversion.convert(part, &mut output));
        conversion.finish(&mut output);
        output
    }

    fn one_shot() -> Vec<f32> {
        let input = sine(SINE_HZ, SampleRate::HZ_22050, 5_000, 0.5);

        resample(&input, SampleRate::HZ_22050, SampleRate::HZ_48000)
    }

    #[test]
    fn conversion_matches_one_shot_when_input_is_split() {
        assert_eq!(convert_in_pieces(777), one_shot());
    }

    #[test]
    fn conversion_matches_one_shot_when_input_is_one_sample_pieces() {
        assert_eq!(convert_in_pieces(1), one_shot());
    }

    #[test]
    fn conversion_restarts_when_reset() {
        let input = sine(SINE_HZ, SampleRate::HZ_24000, 3_000, 0.5);
        let expected = resample(&input, SampleRate::HZ_24000, SampleRate::HZ_48000);
        let mut conversion = Conversion::new(SampleRate::HZ_24000, SampleRate::HZ_48000);
        conversion.convert(&input[..1_500], &mut Vec::new());
        conversion.reset();
        let mut output = Vec::new();

        conversion.convert(&input, &mut output);
        conversion.finish(&mut output);

        assert_eq!(output, expected);
    }

    #[test]
    fn conversion_restarts_when_reset_after_finish() {
        let input = sine(SINE_HZ, SampleRate::HZ_24000, 3_000, 0.5);
        let expected = resample(&input, SampleRate::HZ_24000, SampleRate::HZ_48000);
        let mut conversion = Conversion::new(SampleRate::HZ_24000, SampleRate::HZ_48000);
        conversion.convert(&input, &mut Vec::new());
        conversion.finish(&mut Vec::new());
        conversion.reset();
        let mut output = Vec::new();

        conversion.convert(&input, &mut output);
        conversion.finish(&mut output);

        assert_eq!(output, expected);
    }
}
