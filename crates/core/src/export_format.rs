use strum::{Display, EnumString, IntoStaticStr};

/// The file format of an export. The text form is the file extension, for example "mp3".
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Display, EnumString, IntoStaticStr)]
#[strum(serialize_all = "lowercase")]
pub enum ExportFormat {
    /// MP3, the default format.
    #[default]
    Mp3,
    /// WAV with 16-bit PCM.
    Wav,
    /// Ogg Opus.
    Ogg,
}

impl ExportFormat {
    /// All export formats.
    pub const ALL: [Self; 3] = [Self::Mp3, Self::Wav, Self::Ogg];
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_format_round_trips_when_text_parsed() {
        let texts = ExportFormat::ALL.map(<&str>::from);

        let parsed = texts.map(|text| text.parse::<ExportFormat>().unwrap());

        assert_eq!(texts, ["mp3", "wav", "ogg"]);
        assert_eq!(parsed, ExportFormat::ALL);
        assert_eq!(ExportFormat::Ogg.to_string(), "ogg");
    }

    #[test]
    fn export_format_is_mp3_by_default() {
        assert_eq!(ExportFormat::default(), ExportFormat::Mp3);
    }
}
