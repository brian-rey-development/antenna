//! The documents, the segment store and the search of Antenna.
//!
//! The library keeps each document with its text and its metadata. The segment store keeps the
//! audio of each synthesized segment. Both live in one data directory.

mod atomic;
mod document;
mod error;
mod paths;
mod segments;

pub use document::DocumentId;
pub use error::LibraryError;
pub use segments::{SegmentKey, SegmentStore, SegmentWriter, StoredSegment};
