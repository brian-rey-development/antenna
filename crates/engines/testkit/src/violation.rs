use antenna_core::{CoreError, EngineError, Quality, VoiceId};
use strum::Display;
use thiserror::Error;

/// A failed condition of the conformance suite.
#[derive(Debug, Error)]
#[error("failed condition: {condition} (voice {}, quality {})", or_none(.voice.as_ref()), or_none(.quality.as_ref()))]
pub struct Violation {
    /// The voice of the failed case, or `None` for a descriptor check.
    pub voice: Option<VoiceId>,
    /// The quality of the failed case, or `None` for a descriptor check.
    pub quality: Option<Quality>,
    /// The condition that the engine did not satisfy.
    pub condition: Condition,
    /// The error that broke the condition, or `None` if the engine gave wrong audio.
    #[source]
    pub source: Option<Cause>,
}

/// A condition of the conformance suite. The text form states the condition.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Display)]
pub enum Condition {
    /// The condition of `check_descriptor`. The source has the defect.
    #[strum(to_string = "check_descriptor accepts the descriptor")]
    Descriptor,
    /// `EngineFactory::load` returns `Ok`. The source has the error of the engine.
    #[strum(to_string = "the engine loads")]
    Load,
    /// `Engine::synthesize` returns `Ok`. The source has the error of the engine.
    #[strum(to_string = "the synthesis succeeds")]
    Synthesis,
    /// A condition of `check_audio` about the sample count.
    #[strum(to_string = "the segment gives more than 0 samples")]
    NonEmptyAudio,
    /// A condition of `check_audio` about the sample values.
    #[strum(to_string = "all samples are finite")]
    FiniteSamples,
    /// A condition of `check_audio` about the sample values.
    #[strum(to_string = "the peak is 1.0 or less")]
    Peak,
    /// The condition of `check_break`.
    #[strum(to_string = "emit is not called after a Break")]
    Break,
    /// The condition of `check_determinism`.
    #[strum(to_string = "two calls with one segment give the same samples")]
    Determinism,
    /// The condition of `check_reset`.
    #[strum(to_string = "a call after a Break gives the samples of a new engine")]
    Reset,
}

/// The error of the core or of the engine that broke a condition.
#[derive(Debug, Error)]
pub enum Cause {
    /// The error of `antenna_core::check_descriptor`, with the defect of the descriptor.
    #[error(transparent)]
    Descriptor(CoreError),
    /// The error of `EngineFactory::load` or `Engine::synthesize`.
    #[error(transparent)]
    Engine(EngineError),
}

fn or_none(value: Option<&impl ToString>) -> String {
    value.map_or_else(|| "none".to_owned(), ToString::to_string)
}
