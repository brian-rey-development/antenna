use antenna_core::{CoreError, EngineId, Language};
use thiserror::Error;

/// An error of the registry.
#[derive(Debug, Error)]
pub enum RegistryError {
    /// No registered engine has a voice for the language.
    #[error("no engine has a voice for the language {0}")]
    NoVoice(Language),
    /// No registered engine has a voice with this `Display` text.
    #[error("no engine has the voice {0}")]
    UnknownVoice(String),
    /// No registered engine has this id.
    #[error("no engine has the id {0}")]
    UnknownEngine(String),
    /// Two registered engines have this id.
    #[error("two engines have the id {0}")]
    DuplicateEngine(EngineId),
    /// The descriptor of a registered engine has a defect.
    #[error("an engine descriptor is not valid")]
    Descriptor(#[source] CoreError),
}
