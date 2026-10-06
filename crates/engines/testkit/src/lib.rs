//! The conformance suite and the test outputs of Antenna engines.
//!
//! Each engine crate runs the suite with [`conformance!`] in `tests/conformance.rs`. Other tests
//! record the output of a job with [`RecordingOutput`].

mod conformance;
mod harness;
mod recording;
mod violation;

pub use harness::{FilesFn, Harness};
pub use recording::{RecordedAudio, Recording, RecordingOutput};
pub use violation::{Cause, Condition, Violation};
