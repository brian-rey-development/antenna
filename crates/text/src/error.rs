use std::sync::Arc;

use thiserror::Error;

/// An error of a function of the text crate.
#[derive(Debug, Error)]
pub enum TextError {
    /// The vendored sentence rules do not parse. The rules are the same for all calls, so all
    /// calls share one error.
    #[error("cannot parse the sentence rules in segment.srx")]
    Rules(#[source] Arc<srx::Error>),
    /// The document has no text that an engine can speak, for example a document with only a code
    /// block.
    #[error("the document has no text to speak")]
    NoSpeech,
}
