/// Generates one test for each check of the conformance suite.
///
/// The first form takes the factory and the function that gives the model files of a voice
/// at a quality. The second form also marks each test as ignored, for an engine that needs
/// downloaded models.
///
/// ```text
/// antenna_engine_testkit::conformance!(FakeFactory::default(), &|_, _| ModelFiles::default());
/// antenna_engine_testkit::conformance!(Factory, &files, ignore = "needs models");
/// ```
#[macro_export]
macro_rules! conformance {
    ($factory:expr, $files:expr) => {
        $crate::conformance!(@suite $factory, $files, []);
    };
    ($factory:expr, $files:expr, ignore = $reason:literal) => {
        $crate::conformance!(@suite $factory, $files, [#[ignore = $reason]]);
    };
    (@suite $factory:expr, $files:expr, [$($ignore:tt)*]) => {
        fn conformance_harness(
            check: fn(&$crate::Harness<'_>) -> ::core::result::Result<(), $crate::Violation>,
        ) -> ::core::result::Result<(), $crate::Violation> {
            let factory = $factory;
            check(&$crate::Harness::new(&factory, $files))
        }

        #[cfg(test)]
        mod conformance {
            $crate::conformance!(@test check_descriptor, [$($ignore)*]);
            $crate::conformance!(@test check_audio, [$($ignore)*]);
            $crate::conformance!(@test check_break, [$($ignore)*]);
            $crate::conformance!(@test check_determinism, [$($ignore)*]);
            $crate::conformance!(@test check_reset, [$($ignore)*]);
            $crate::conformance!(@test check_edge_segments, [$($ignore)*]);
        }
    };
    (@test $check:ident, [$($ignore:tt)*]) => {
        #[test]
        $($ignore)*
        fn $check() -> ::core::result::Result<(), $crate::Violation> {
            super::conformance_harness(|harness| harness.$check())
        }
    };
}
