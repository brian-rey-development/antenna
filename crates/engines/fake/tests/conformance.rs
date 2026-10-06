//! The conformance suite of the `fake` engine.

use antenna_core::ModelFiles;
use antenna_engine_fake::FakeFactory;

antenna_engine_testkit::conformance!(FakeFactory::default(), &|_, _| ModelFiles::default());
