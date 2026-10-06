//! The domain types and the extension traits of Antenna.
//!
//! The traits [`EngineFactory`], [`Engine`], [`Output`] and [`Sink`] are the extension points of
//! the system. All other crates use the vocabulary of this crate.

pub mod app_dirs;
mod audio;
mod descriptor;
mod document;
mod engine;
mod error;
mod export_format;
mod files;
mod hex;
mod id;
mod language;
mod output;
mod quality;
mod text_hash;

pub use audio::{PCM_SCALE, PcmChunk, SampleRate};
pub use descriptor::{
    Artifact, EngineDescriptor, Extent, Localized, ModelLicense, Variant, Variants,
    VoiceDescriptor, check_descriptor,
};
pub use document::{Document, Segment, SegmentIndex, TextFormat};
pub use engine::{Emit, Engine, EngineFactory};
pub use error::{CoreError, Defect, EngineError, SinkError};
pub use export_format::ExportFormat;
pub use files::ModelFiles;
pub use id::{EngineId, VoiceId};
pub use language::Language;
pub use output::{Output, Sink};
pub use quality::Quality;
pub use text_hash::TextHash;
