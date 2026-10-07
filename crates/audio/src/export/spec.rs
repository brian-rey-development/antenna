use std::path::PathBuf;
use std::time::Duration;

use antenna_core::{ExportFormat, Language, SampleRate, VoiceId};
use strum::{Display, EnumString, IntoStaticStr};

/// The sample rate of an exported MP3 or WAV file. The text form is the rate in hertz.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Display, EnumString, IntoStaticStr)]
pub enum ExportRate {
    /// 24 kHz, the rate of the default engine.
    #[strum(serialize = "24000")]
    Hz24000,
    /// 44.1 kHz, the default.
    #[default]
    #[strum(serialize = "44100")]
    Hz44100,
    /// 48 kHz.
    #[strum(serialize = "48000")]
    Hz48000,
}

impl ExportRate {
    /// All export rates.
    pub const ALL: [Self; 3] = [Self::Hz24000, Self::Hz44100, Self::Hz48000];

    /// Returns the rate as a sample rate.
    pub const fn sample_rate(self) -> SampleRate {
        match self {
            Self::Hz24000 => SampleRate::HZ_24000,
            Self::Hz44100 => SampleRate::HZ_44100,
            Self::Hz48000 => SampleRate::HZ_48000,
        }
    }
}

/// Whether an export normalizes the loudness. The text form is "normalize" or "keep".
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Display, EnumString, IntoStaticStr)]
#[strum(serialize_all = "lowercase")]
pub enum Loudness {
    /// Apply one gain to reach -16 LUFS without a true peak above -1 dBTP.
    #[default]
    Normalize,
    /// Keep the level of the segments.
    Keep,
}

impl Loudness {
    pub(crate) const fn passes(self) -> usize {
        match self {
            Self::Normalize => 2,
            Self::Keep => 1,
        }
    }
}

/// The text that an export writes to the tags of the file. The MP3 tags hold only Latin-1
/// characters, and the title has at most 250 bytes. Another character is a question mark.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EpisodeTags {
    /// The title of the document.
    pub title: String,
    /// The voice that spoke the document.
    pub voice: VoiceId,
    /// The language of the document.
    pub language: Language,
}

/// The input of [`export`](crate::export).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportSpec {
    /// The stored segments in the order of the document.
    pub segments: Vec<PathBuf>,
    /// The sample rate of the segments.
    pub source_rate: SampleRate,
    /// The file format.
    pub format: ExportFormat,
    /// The sample rate of the file. An Ogg file at 44.1 kHz has 48 kHz, because Opus has no 44.1 kHz mode.
    pub export_rate: ExportRate,
    /// Whether to normalize the loudness.
    pub loudness: Loudness,
    /// The tags of the file.
    pub tags: EpisodeTags,
    /// The path of the file.
    pub destination: PathBuf,
}

/// The result of an export.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExportSummary {
    /// The duration of the exported audio.
    pub duration: Duration,
    /// The gain that the export applied, in decibels.
    pub gain_db: f64,
    /// The size of the file in bytes.
    pub bytes: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_rates() -> [ExportRate; 3] {
        ExportRate::ALL
            .map(<&str>::from)
            .map(|text| text.parse::<ExportRate>().unwrap())
    }

    #[test]
    fn export_rate_round_trips_when_text_parsed() {
        assert_eq!(parse_rates(), ExportRate::ALL);
    }

    #[test]
    fn export_rate_has_hertz_text_form() {
        assert_eq!(
            ExportRate::ALL.map(<&str>::from),
            ["24000", "44100", "48000"]
        );
    }

    #[test]
    fn export_rate_is_44100_by_default() {
        assert_eq!(ExportRate::default(), ExportRate::Hz44100);
    }

    #[test]
    fn export_rate_converts_to_sample_rate() {
        let rates = ExportRate::ALL.map(ExportRate::sample_rate);

        assert_eq!(rates.map(SampleRate::hz), [24_000, 44_100, 48_000]);
    }

    #[test]
    fn loudness_round_trips_when_text_parsed() {
        let all = [Loudness::Normalize, Loudness::Keep];

        let parsed = all
            .map(<&str>::from)
            .map(|text| text.parse::<Loudness>().unwrap());

        assert_eq!(parsed, all);
    }

    #[test]
    fn loudness_has_lowercase_text_form() {
        assert_eq!(
            [Loudness::Normalize, Loudness::Keep].map(<&str>::from),
            ["normalize", "keep"]
        );
    }

    #[test]
    fn loudness_normalizes_by_default() {
        assert_eq!(Loudness::default(), Loudness::Normalize);
    }

    #[test]
    fn loudness_needs_two_passes_when_normalizing() {
        assert_eq!(
            (Loudness::Normalize.passes(), Loudness::Keep.passes()),
            (2, 1)
        );
    }
}
