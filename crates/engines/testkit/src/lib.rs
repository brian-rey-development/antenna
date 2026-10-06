//! The conformance suite and the test outputs of Antenna engines.
//!
//! Each engine crate runs the suite with [`conformance!`] in `tests/conformance.rs`. Other tests
//! record the output of a job with [`RecordingOutput`].

mod condition;
mod conformance;
mod harness;
mod recording;

pub use harness::{FilesFn, Harness, Violation};
pub use recording::{RecordedAudio, Recording, RecordingOutput};
