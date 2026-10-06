//! Tests of the voices, the tones and the faults of the `fake` engine.

#[cfg(test)]
mod tests {
    use std::ops::ControlFlow;
    use std::time::Duration;

    use antenna_core::{
        EngineError, EngineFactory, EngineId, Language, ModelFiles, Quality, SampleRate, Segment,
        SegmentIndex, VoiceDescriptor, VoiceId,
    };
    use antenna_engine_fake::{FakeFactory, Fault};

    const RATE: SampleRate = SampleRate::HZ_24000;
    const ONE_SECOND_TEXT: &str = "abcdefghijklmnopqrstuvwxy";

    fn voice(key: &str) -> &'static VoiceDescriptor {
        FakeFactory::default()
            .descriptor()
            .voices
            .iter()
            .find(|voice| voice.id.key() == key)
            .unwrap()
    }

    fn chunks(
        factory: FakeFactory,
        voice_key: &str,
        quality: Quality,
        index: u32,
        text: &str,
    ) -> Result<Vec<Vec<f32>>, EngineError> {
        let mut engine = factory.load(voice(voice_key), quality, &ModelFiles::default())?;
        let segment = Segment::new(SegmentIndex::new(index), text, 0..text.len());
        let mut chunks = Vec::new();
        engine.synthesize(&segment, &mut |chunk| {
            chunks.push(chunk.into_samples());
            ControlFlow::Continue(())
        })?;
        Ok(chunks)
    }

    fn samples(voice_key: &str, index: u32, text: &str) -> Vec<f32> {
        chunks(
            FakeFactory::default(),
            voice_key,
            Quality::Balanced,
            index,
            text,
        )
        .unwrap()
        .concat()
    }

    fn upward_crossings(samples: &[f32]) -> usize {
        samples
            .windows(2)
            .filter(|pair| matches!(pair, [before, after] if *before < 0.0 && *after >= 0.0))
            .count()
    }

    #[test]
    fn fake_has_four_named_voices_for_each_language() {
        let descriptor = FakeFactory::default().descriptor();

        for language in Language::ALL {
            let voices: Vec<_> = descriptor.voices_for(language).collect();

            let names: Vec<_> = voices.iter().map(|voice| voice.name).collect();
            assert_eq!(names, ["Alba", "Bruno", "Clara", "Dario"]);
            for voice in voices {
                let key = format!("fake/{language}-{}", voice.name.to_lowercase());
                assert_eq!(voice.id.to_string(), key);
                assert!(!voice.description.en.is_empty() && !voice.description.es.is_empty());
            }
        }
        assert_eq!(descriptor.voices.len(), 24);
    }

    #[test]
    fn fake_tone_lasts_40_ms_for_each_character() {
        let samples = samples("es-bruno", 0, "Hola, ¿qué tal?");

        let duration = RATE.duration_of(samples.len() as u64);

        assert_eq!(duration, Duration::from_millis(40 * 15));
    }

    #[test]
    fn fake_tone_frequency_follows_segment_index() {
        let cases = [
            ("en-alba", 0, 220),
            ("en-alba", 1, 440),
            ("en-alba", 3, 880),
            ("en-alba", 5, 440),
            ("de-dario", 2, 882),
        ];

        for (key, index, expected_hz) in cases {
            let crossings = upward_crossings(&samples(key, index, ONE_SECOND_TEXT));

            assert!(
                crossings.abs_diff(expected_hz) <= 1,
                "{key} segment {index}: {crossings} Hz"
            );
        }
    }

    #[test]
    fn fake_tone_has_amplitude_of_one_quarter() {
        let samples = samples("fr-clara", 0, ONE_SECOND_TEXT);

        let peak = samples
            .iter()
            .fold(0.0_f32, |peak, sample| peak.max(sample.abs()));

        assert!((peak - 0.25).abs() < 1e-4, "peak {peak}");
    }

    #[test]
    fn fake_emits_chunks_of_480_samples() {
        let chunks = chunks(
            FakeFactory::default(),
            "en-alba",
            Quality::Fast,
            0,
            "Hello.",
        )
        .unwrap();

        assert_eq!(chunks.len(), 12);
        assert!(chunks.iter().all(|chunk| chunk.len() == 480));
    }

    #[test]
    fn fake_audio_is_same_when_quality_changes() {
        let audio = Quality::ALL
            .map(|quality| chunks(FakeFactory::default(), "en-alba", quality, 1, "Hi").unwrap());

        assert_eq!(audio[0], audio[1]);
        assert_eq!(audio[1], audio[2]);
    }

    #[test]
    fn fake_returns_error_when_fault_fail_at_segment() {
        let factory = FakeFactory::with_fault(Fault::FailAt {
            segment: SegmentIndex::new(2),
        });

        let before = chunks(factory, "en-alba", Quality::Balanced, 1, "Hi");
        let result = chunks(factory, "en-alba", Quality::Balanced, 2, "Hi");

        assert!(before.is_ok());
        assert!(matches!(result, Err(EngineError::Inference(_))));
    }

    #[test]
    #[should_panic(expected = "the fake engine panics at segment 3 on purpose")]
    fn fake_panics_when_fault_panic_at_segment() {
        let factory = FakeFactory::with_fault(Fault::PanicAt {
            segment: SegmentIndex::new(3),
        });
        chunks(factory, "en-alba", Quality::Balanced, 2, "Hi").unwrap();

        chunks(factory, "en-alba", Quality::Balanced, 3, "Hi").unwrap();
    }

    #[test]
    fn fake_load_fails_when_voice_unknown() {
        let foreign = VoiceDescriptor {
            id: VoiceId::new(EngineId::new("other"), "en-alba"),
            ..*voice("en-alba")
        };

        let result = FakeFactory::default().load(&foreign, Quality::Fast, &ModelFiles::default());

        assert!(matches!(result, Err(EngineError::Load(_))));
    }
}
