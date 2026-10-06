//! The conformance suite of the `fake` engine.

use antenna_core::ModelFiles;
use antenna_engine_fake::Factory;

antenna_engine_testkit::conformance!(Factory::default(), &|_, _| ModelFiles::default());
