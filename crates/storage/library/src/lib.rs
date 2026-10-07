//! The documents, the segment store and the search of Antenna.
//!
//! The library keeps each document with its text and its metadata. The segment store keeps the
//! audio of each synthesized segment. Both live in one data directory.

mod atomic;
mod document;
mod error;
mod index;
mod library;
mod meta_file;
mod metadata;
mod paths;
mod period;
mod search;
mod segments;
mod status;

pub use document::{DocumentId, DocumentMeta, ExportRecord, SegmentList, StoredVoice, text_hash};
pub use error::LibraryError;
pub use index::RECENT_LIMIT;
pub use library::Library;
pub use period::{Group, Period};
pub use search::fold;
pub use segments::{SegmentKey, SegmentStore, SegmentWriter, StoredSegment};
pub use status::{DocumentSummary, Filter, Status};
