use antenna_core::{SegmentIndex, VoiceId};
use thiserror::Error;

/// An error of the `fake` engine, boxed into `EngineError`.
#[derive(Debug, Error)]
pub(crate) enum FakeError {
    /// The voice is not a voice of the `fake` engine.
    #[error("the fake engine has no voice {voice}")]
    UnknownVoice { voice: VoiceId },
    /// The engine fails at this segment because of `Fault::FailAt`.
    #[error("the fake engine fails at segment {segment} on purpose")]
    Fault { segment: SegmentIndex },
}
