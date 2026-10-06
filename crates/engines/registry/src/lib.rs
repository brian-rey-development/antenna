//! The registry of Antenna: the engine factories of the build and the default voice of each
//! language.
//!
//! The feature `engine-<name>` of this crate adds the engine `<name>`. The crate has no default
//! features, so each app selects its engines.

mod defaults;
mod engines;
mod error;
mod registry;

pub use error::RegistryError;
pub use registry::Registry;
