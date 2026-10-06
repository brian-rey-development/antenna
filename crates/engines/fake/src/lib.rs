//! The `fake` engine of Antenna. It makes deterministic sine tones, so the pipeline, the apps and
//! their tests run with no model download.

mod descriptor;
mod engine;
mod error;
mod factory;

pub use factory::{Factory, Fault};
