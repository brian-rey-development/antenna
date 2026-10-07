/// The gain of the output, from 0.0 (silence) to 1.0 (the level of the samples).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Volume(f32);

impl Volume {
    pub(crate) const FULL: Self = Self(1.0);

    /// Makes a volume. A gain outside the range is the nearest limit, and a gain that is not a
    /// number is 0.0.
    pub fn new(gain: f32) -> Self {
        if gain.is_nan() {
            return Self(0.0);
        }
        Self(gain.clamp(0.0, 1.0))
    }

    pub(crate) fn to_bits(self) -> u32 {
        self.0.to_bits()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn volume_clamps_to_one_when_gain_above_range() {
        assert_eq!(Volume::new(1.5), Volume::FULL);
    }

    #[test]
    fn volume_clamps_to_zero_when_gain_below_range() {
        assert_eq!(Volume::new(-0.5), Volume::new(0.0));
    }

    #[test]
    fn volume_is_zero_when_gain_not_a_number() {
        assert_eq!(Volume::new(f32::NAN), Volume::new(0.0));
    }

    #[test]
    fn volume_keeps_gain_when_inside_range() {
        assert_eq!(Volume::new(0.25).to_bits(), 0.25_f32.to_bits());
    }
}
