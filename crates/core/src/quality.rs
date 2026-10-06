use strum::{Display, EnumString, IntoStaticStr};

/// The quality variant of an engine. A higher quality uses a larger or more precise model.
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Display,
    EnumString,
    IntoStaticStr,
)]
#[strum(serialize_all = "lowercase")]
pub enum Quality {
    /// The fastest variant of an engine.
    Fast,
    /// The default variant, with a balance of speed and quality.
    #[default]
    Balanced,
    /// The variant with the best quality, for example at full precision.
    Max,
}

impl Quality {
    /// All qualities, from the fastest to the best.
    pub const ALL: [Self; 3] = [Self::Fast, Self::Balanced, Self::Max];
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quality_round_trips_when_text_parsed() {
        let texts = Quality::ALL.map(<&str>::from);

        let parsed = texts.map(|text| text.parse::<Quality>().unwrap());

        assert_eq!(texts, ["fast", "balanced", "max"]);
        assert_eq!(parsed, Quality::ALL);
        assert_eq!(Quality::Max.to_string(), "max");
    }

    #[test]
    fn quality_is_balanced_by_default() {
        assert_eq!(Quality::default(), Quality::Balanced);
    }
}
