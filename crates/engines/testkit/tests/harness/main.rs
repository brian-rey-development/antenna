//! Self-tests of the conformance suite. Each test uses a test engine with one defect.

#[cfg(test)]
mod engine;

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::error::Error;

    use antenna_core::{CoreError, EngineError, ModelFiles, Quality, VoiceDescriptor};
    use antenna_engine_testkit::{Cause, Condition, Harness};

    use crate::engine::{Defect, EN_FIRST, NO_VOICES, TestFactory, VOICES, factory, harness_of};

    #[test]
    fn harness_passes_when_engine_has_no_defect() {
        let factory = factory(Defect::None);
        let harness = harness_of(&factory);

        harness.check_descriptor().unwrap();
        harness.check_audio().unwrap();
        harness.check_break().unwrap();
        harness.check_determinism().unwrap();
        harness.check_reset().unwrap();
        harness.check_edge_segments().unwrap();
    }

    #[test]
    fn harness_check_descriptor_fails_when_engine_has_no_voices() {
        let factory = TestFactory {
            descriptor: &NO_VOICES,
            defect: Defect::None,
        };

        let violation = harness_of(&factory).check_descriptor().unwrap_err();

        assert_eq!((violation.voice, violation.quality), (None, None));
        assert_eq!(violation.condition, Condition::Descriptor);
        assert!(matches!(
            violation.source,
            Some(Cause::Descriptor(CoreError::InvalidDescriptor {
                defect: antenna_core::Defect::NoVoices,
                ..
            }))
        ));
    }

    #[test]
    fn harness_check_audio_fails_when_samples_not_finite() {
        let factory = factory(Defect::NotFinite);

        let violation = harness_of(&factory).check_audio().unwrap_err();

        assert_eq!(violation.condition, Condition::FiniteSamples);
        assert_eq!(violation.voice, Some(EN_FIRST.id));
    }

    #[test]
    fn harness_check_audio_fails_when_peak_above_one() {
        let factory = factory(Defect::TooLoud);

        let violation = harness_of(&factory).check_audio().unwrap_err();

        assert_eq!(violation.condition, Condition::Peak);
    }

    #[test]
    fn harness_check_audio_fails_when_one_quality_is_silent() {
        let factory = factory(Defect::SilentAt(Quality::Max));

        let violation = harness_of(&factory).check_audio().unwrap_err();

        assert_eq!(violation.quality, Some(Quality::Max));
        let message = "failed condition: the segment gives more than 0 samples (voice test/en-a, quality max)";
        assert_eq!(violation.to_string(), message);
    }

    #[test]
    fn harness_check_break_fails_when_engine_ignores_break() {
        let factory = factory(Defect::IgnoresBreak);

        let violation = harness_of(&factory).check_break().unwrap_err();

        assert_eq!(violation.condition, Condition::Break);
    }

    #[test]
    fn harness_check_determinism_fails_when_samples_change_between_calls() {
        let factory = factory(Defect::ChangesEachCall);

        let violation = harness_of(&factory).check_determinism().unwrap_err();

        assert_eq!(violation.condition, Condition::Determinism);
    }

    #[test]
    fn harness_check_reset_fails_when_state_survives_break() {
        let factory = factory(Defect::KeepsStateAfterBreak);

        let violation = harness_of(&factory).check_reset().unwrap_err();

        assert_eq!(violation.condition, Condition::Reset);
    }

    #[test]
    fn harness_check_edge_segments_fails_when_pictograph_fails() {
        let factory = factory(Defect::FailsOnPictograph);

        let violation = harness_of(&factory).check_edge_segments().unwrap_err();

        assert_eq!(violation.condition, Condition::Synthesis);
        assert!(matches!(
            violation.source,
            Some(Cause::Engine(EngineError::Inference(_)))
        ));
    }

    #[test]
    fn harness_check_audio_fails_when_model_file_missing() {
        let factory = factory(Defect::NeedsModelFile);

        let violation = harness_of(&factory).check_audio().unwrap_err();

        assert_eq!(violation.condition, Condition::Load);
        assert!(matches!(
            violation.source,
            Some(Cause::Engine(EngineError::MissingFile { key: "weights" }))
        ));
    }

    #[test]
    fn violation_source_is_engine_error_when_load_fails() {
        let factory = factory(Defect::NeedsModelFile);

        let violation = harness_of(&factory).check_audio().unwrap_err();

        let source = violation.source().unwrap().to_string();
        assert_eq!(source, "the model file weights is missing");
    }

    #[test]
    fn harness_checks_first_voice_of_each_language_at_each_quality() {
        let cases = Cell::new(Vec::new());
        let record = |voice: &VoiceDescriptor, quality: Quality| {
            let mut seen = cases.take();
            seen.push((voice.id, quality));
            cases.set(seen);
            ModelFiles::default()
        };
        let factory = factory(Defect::None);

        Harness::new(&factory, &record).check_break().unwrap();

        let expected: Vec<_> = [VOICES[0].id, VOICES[2].id]
            .into_iter()
            .flat_map(|voice| Quality::ALL.map(|quality| (voice, quality)))
            .collect();
        assert_eq!(cases.take(), expected);
    }
}
