use std::error::Error;
use std::fmt::{self, Display, Formatter};

use thiserror::Error;

use crate::{EngineId, Quality, SampleRate, SegmentIndex, VoiceId};

/// An error of a function of the core.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Error)]
pub enum CoreError {
    /// The text of a document is empty or contains only whitespace.
    #[error("the document has no text")]
    EmptyDocument,
    /// A sample rate is zero.
    #[error("the sample rate is zero")]
    ZeroSampleRate,
    /// A text hash is not 64 lowercase hex characters.
    #[error("the text hash is not 64 lowercase hex characters")]
    InvalidTextHash,
    /// An engine descriptor has a defect.
    #[error("the descriptor of engine {engine} is not valid: {defect}")]
    InvalidDescriptor {
        /// The id of the engine.
        engine: EngineId,
        /// The first defect that the check found.
        defect: Defect,
    },
}

/// A defect of an engine descriptor that [`check_descriptor`](crate::check_descriptor) finds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Defect {
    /// The engine has no voices.
    NoVoices,
    /// Two voices have this id.
    DuplicateVoice(VoiceId),
    /// The engine id of this voice id is not the id of the descriptor.
    ForeignVoice(VoiceId),
    /// The voice with this id has an empty name.
    EmptyName(VoiceId),
    /// The variant of this quality has an empty `parameters` text.
    EmptyParameters(Quality),
    /// The revision of an artifact is not 40 lowercase hex characters.
    Revision {
        /// The key of the artifact.
        artifact: &'static str,
    },
    /// The SHA-256 of an artifact is not 64 lowercase hex characters.
    Sha256 {
        /// The key of the artifact.
        artifact: &'static str,
    },
    /// The extent of an artifact has 0 bytes.
    EmptyExtent {
        /// The key of the artifact.
        artifact: &'static str,
    },
    /// Two artifacts in the request of one voice at one quality have this key.
    DuplicateKey {
        /// The key of the artifacts.
        artifact: &'static str,
    },
}

impl Display for Defect {
    #[expect(
        clippy::too_many_lines,
        reason = "the match has one arm for each of the nine defects, and rustfmt puts each long arm on three lines"
    )]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoVoices => write!(formatter, "the engine has no voices"),
            Self::DuplicateVoice(voice) => write!(formatter, "two voices have the id {voice}"),
            Self::ForeignVoice(voice) => {
                write!(formatter, "the voice {voice} has another engine id")
            }
            Self::EmptyName(voice) => write!(formatter, "the voice {voice} has no name"),
            Self::EmptyParameters(quality) => {
                write!(formatter, "the variant {quality} has no parameters text")
            }
            Self::Revision { artifact } => {
                write!(
                    formatter,
                    "the revision of artifact {artifact} is not 40 hex characters"
                )
            }
            Self::Sha256 { artifact } => {
                write!(
                    formatter,
                    "the SHA-256 of artifact {artifact} is not 64 hex characters"
                )
            }
            Self::EmptyExtent { artifact } => {
                write!(formatter, "the extent of artifact {artifact} has 0 bytes")
            }
            Self::DuplicateKey { artifact } => {
                write!(
                    formatter,
                    "two artifacts of one voice request have the key {artifact}"
                )
            }
        }
    }
}

/// An error of an engine factory or an engine.
#[derive(Debug, Error)]
pub enum EngineError {
    /// The model files have no path for an artifact key.
    #[error("the model file {key} is missing")]
    MissingFile {
        /// The key of the artifact.
        key: &'static str,
    },
    /// The engine cannot load. The source is the error of the engine.
    #[error("the engine cannot load")]
    Load(#[source] Box<dyn Error + Send + Sync>),
    /// The synthesis of a segment failed. The source is the error of the engine.
    #[error("the synthesis failed")]
    Inference(#[source] Box<dyn Error + Send + Sync>),
    /// The generation of a segment passed its step limit after the first chunk.
    #[error("the generation of segment {segment} passed its step limit")]
    Runaway {
        /// The index of the segment.
        segment: SegmentIndex,
    },
}

/// An error of an output or a sink.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Error)]
pub enum SinkError {
    /// The output cannot accept the sample rate of the engine.
    #[error("the sample rate is {} Hz, but the output needs {} Hz", .actual.hz(), .expected.hz())]
    RateMismatch {
        /// The rate that the output needs.
        expected: SampleRate,
        /// The rate of the engine.
        actual: SampleRate,
    },
    /// The receiver of the sink is gone.
    #[error("the receiver of the sink is gone")]
    Closed,
}
