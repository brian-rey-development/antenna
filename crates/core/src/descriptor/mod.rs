mod check;

use std::num::NonZeroUsize;

use crate::{EngineId, Language, Quality, SampleRate, VoiceId};

pub use check::check_descriptor;

/// A text in each interface language.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Localized {
    /// The English text.
    pub en: &'static str,
    /// The Spanish text.
    pub es: &'static str,
}

/// The license of the weights of an engine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModelLicense {
    /// The name of the license, for example "Apache-2.0".
    pub name: &'static str,
    /// The URL of the license text.
    pub url: &'static str,
    /// The attribution text that the license needs, or an empty text.
    pub attribution: &'static str,
}

/// One model file that Antenna downloads from Hugging Face.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Artifact {
    /// The key of the file in [`ModelFiles`](crate::ModelFiles). It is unique in the files of one
    /// voice and variant.
    pub key: &'static str,
    /// The Hugging Face repository, for example "Qwen/Qwen3-TTS-12Hz-1.7B-Base".
    pub repo: &'static str,
    /// The commit SHA of the repository, 40 lowercase hex characters.
    pub revision: &'static str,
    /// The path of the file in the repository.
    pub path: &'static str,
    /// The bytes of the file that Antenna downloads.
    pub extent: Extent,
    /// The SHA-256 of the downloaded bytes, 64 lowercase hex characters.
    pub sha256: &'static str,
}

/// The bytes of a file in a repository that an artifact contains.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Extent {
    /// The complete file.
    Whole {
        /// The size of the file.
        bytes: u64,
    },
    /// A part of the file, for a member of an uncompressed archive.
    Range {
        /// The position of the first byte in the file.
        offset: u64,
        /// The size of the part.
        bytes: u64,
    },
}

impl Extent {
    /// Returns the number of bytes that Antenna downloads.
    pub const fn bytes(self) -> u64 {
        match self {
            Self::Whole { bytes } | Self::Range { bytes, .. } => bytes,
        }
    }
}

/// The files and the model size of an engine for one quality.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Variant {
    /// The model size for the Models screen, for example "1.7B".
    pub parameters: &'static str,
    /// The files that all voices need in this variant.
    pub artifacts: &'static [Artifact],
}

/// The variants of an engine, one for each [`Quality`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Variants {
    /// The variant of [`Quality::Fast`].
    pub fast: Variant,
    /// The variant of [`Quality::Balanced`].
    pub balanced: Variant,
    /// The variant of [`Quality::Max`].
    pub max: Variant,
}

impl Variants {
    /// Returns the variant of a quality.
    pub const fn get(&self, quality: Quality) -> &Variant {
        match quality {
            Quality::Fast => &self.fast,
            Quality::Balanced => &self.balanced,
            Quality::Max => &self.max,
        }
    }
}

/// One speaker identity of an engine for one language.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VoiceDescriptor {
    /// The id of the voice.
    pub id: VoiceId,
    /// The language that the voice speaks.
    pub language: Language,
    /// A person name, for example "Lucía".
    pub name: &'static str,
    /// The sound of the voice, for example "Warm, mid register, calm".
    pub description: Localized,
    /// The files that only this voice needs.
    pub artifacts: &'static [Artifact],
}

/// The static data that tells the capabilities of an engine type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EngineDescriptor {
    /// The id of the engine.
    pub id: EngineId,
    /// The name for the user, for example "Qwen3-TTS".
    pub name: &'static str,
    /// An integer text that increases with each change to the audio. It is part of each segment
    /// key.
    pub version: &'static str,
    /// One line for the Models screen.
    pub summary: Localized,
    /// The license of the weights.
    pub license: ModelLicense,
    /// The sample rate of the audio that the engine emits.
    pub sample_rate: SampleRate,
    /// The maximum number of characters in a segment. The text crate makes no longer segment.
    pub max_segment_chars: NonZeroUsize,
    /// The variant of each quality.
    pub variants: Variants,
    /// The voices of the engine.
    pub voices: &'static [VoiceDescriptor],
}

impl EngineDescriptor {
    /// Returns the voices of a language, in declaration sequence.
    pub fn voices_for(&self, language: Language) -> impl Iterator<Item = &'static VoiceDescriptor> {
        self.voices
            .iter()
            .filter(move |voice| voice.language == language)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EMPTY: Variant = Variant {
        parameters: "0",
        artifacts: &[],
    };

    #[test]
    fn variants_get_returns_field_when_quality_given() {
        let variants = Variants {
            fast: Variant {
                parameters: "0.6B",
                ..EMPTY
            },
            balanced: Variant {
                parameters: "1.7B",
                ..EMPTY
            },
            max: Variant {
                parameters: "1.7B f32",
                ..EMPTY
            },
        };

        let parameters = Quality::ALL.map(|quality| variants.get(quality).parameters);

        assert_eq!(parameters, ["0.6B", "1.7B", "1.7B f32"]);
    }

    #[test]
    fn extent_bytes_returns_size_of_each_kind() {
        assert_eq!(Extent::Whole { bytes: 10 }.bytes(), 10);
        assert_eq!(
            Extent::Range {
                offset: 512,
                bytes: 7
            }
            .bytes(),
            7
        );
    }
}
