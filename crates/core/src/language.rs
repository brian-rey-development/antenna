use strum::{Display, EnumString, IntoStaticStr};

/// A language that Antenna reads aloud. The text form is the ISO 639-1 code, for example "en".
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Display, EnumString, IntoStaticStr,
)]
#[strum(serialize_all = "lowercase")]
pub enum Language {
    /// English.
    En,
    /// Spanish.
    Es,
    /// Portuguese.
    Pt,
    /// French.
    Fr,
    /// Italian.
    It,
    /// German.
    De,
}

impl Language {
    /// All languages, in declaration sequence.
    pub const ALL: [Self; 6] = [Self::En, Self::Es, Self::Pt, Self::Fr, Self::It, Self::De];
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_round_trips_when_text_parsed() {
        let texts = Language::ALL.map(<&str>::from);

        let parsed = texts.map(|text| text.parse::<Language>().unwrap());

        assert_eq!(texts, ["en", "es", "pt", "fr", "it", "de"]);
        assert_eq!(parsed, Language::ALL);
        assert_eq!(Language::Es.to_string(), "es");
    }

    #[test]
    fn language_fails_when_text_unknown() {
        let result = "EN".parse::<Language>();

        assert_eq!(result, Err(strum::ParseError::VariantNotFound));
    }
}
