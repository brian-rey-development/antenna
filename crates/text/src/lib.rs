//! Converts a document into speakable segments with exact source ranges, and identifies the
//! language of a document.
//!
//! [`prepare`] is the entry point of the segmentation. It reads Markdown or plain text, splits the
//! prose into sentences with the SRX rules of `LanguageTool`, and splits long sentences into
//! clauses. [`identify_language`] reads the same prose.

mod boundary;
mod error;
mod fragment;
mod language;
mod limits;
mod markdown;
mod prepare;
mod prose;
mod sentences;
mod source_map;
mod split;
mod srx_rules;

pub use error::TextError;
pub use language::identify_language;
pub use limits::{FIRST_SEGMENT_CHARS, SegmentLimits};
pub use prepare::prepare;
