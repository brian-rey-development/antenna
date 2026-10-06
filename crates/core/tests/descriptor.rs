//! Tests of `check_descriptor` and of the voice queries of a descriptor.

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use antenna_core::{
        Artifact, CoreError, Defect, EngineDescriptor, EngineId, Extent, Language, Localized,
        ModelLicense, Quality, SampleRate, Variant, Variants, VoiceDescriptor, VoiceId,
        check_descriptor,
    };

    const ID: EngineId = EngineId::new("test");
    const TEXT: Localized = Localized {
        en: "Test",
        es: "Prueba",
    };
    const WEIGHTS: Artifact = Artifact {
        key: "weights",
        repo: "antenna/test",
        revision: "0123456789abcdef0123456789abcdef01234567",
        path: "model.safetensors",
        extent: Extent::Whole { bytes: 10 },
        sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    };
    const PROMPT: Artifact = Artifact {
        key: "prompt",
        extent: Extent::Range {
            offset: 512,
            bytes: 64,
        },
        ..WEIGHTS
    };
    const VARIANT: Variant = Variant {
        parameters: "1B",
        artifacts: &[WEIGHTS],
    };
    const ALBA: VoiceDescriptor = VoiceDescriptor {
        id: VoiceId::new(ID, "en-alba"),
        language: Language::En,
        name: "Alba",
        description: TEXT,
        artifacts: &[PROMPT],
    };
    const BRUNO: VoiceDescriptor = VoiceDescriptor {
        id: VoiceId::new(ID, "es-bruno"),
        language: Language::Es,
        name: "Bruno",
        ..ALBA
    };
    const CLARA: VoiceDescriptor = VoiceDescriptor {
        id: VoiceId::new(ID, "en-clara"),
        name: "Clara",
        ..ALBA
    };
    const VALID: EngineDescriptor = EngineDescriptor {
        id: ID,
        name: "Test",
        version: "1",
        summary: TEXT,
        license: ModelLicense {
            name: "MIT",
            url: "https://mit-license.org",
            attribution: "",
        },
        sample_rate: SampleRate::HZ_24000,
        max_segment_chars: NonZeroUsize::new(400).unwrap(),
        variants: Variants {
            fast: VARIANT,
            balanced: VARIANT,
            max: VARIANT,
        },
        voices: &[ALBA, BRUNO, CLARA],
    };

    fn defect_of(descriptor: &EngineDescriptor) -> Defect {
        match check_descriptor(descriptor) {
            Err(CoreError::InvalidDescriptor { engine, defect }) if engine == ID => defect,
            other => panic!("expected a defect, got {other:?}"),
        }
    }

    fn with_artifact(artifact: Artifact) -> EngineDescriptor {
        let voice = VoiceDescriptor {
            artifacts: Box::leak(Box::new([artifact])),
            ..ALBA
        };
        EngineDescriptor {
            voices: Box::leak(Box::new([voice])),
            ..VALID
        }
    }

    fn with_variant_artifact(artifact: Artifact) -> EngineDescriptor {
        let balanced = Variant {
            artifacts: Box::leak(Box::new([artifact])),
            ..VARIANT
        };
        EngineDescriptor {
            variants: Variants {
                balanced,
                ..VALID.variants
            },
            ..VALID
        }
    }

    #[test]
    fn check_descriptor_passes_when_descriptor_valid() {
        assert_eq!(check_descriptor(&VALID), Ok(()));
    }

    #[test]
    fn check_descriptor_fails_when_no_voices() {
        let descriptor = EngineDescriptor {
            voices: &[],
            ..VALID
        };

        assert_eq!(defect_of(&descriptor), Defect::NoVoices);
    }

    #[test]
    fn check_descriptor_fails_when_duplicate_voice() {
        let descriptor = EngineDescriptor {
            voices: &[ALBA, BRUNO, ALBA],
            ..VALID
        };

        assert_eq!(defect_of(&descriptor), Defect::DuplicateVoice(ALBA.id));
    }

    #[test]
    fn check_descriptor_fails_when_foreign_voice() {
        const OTHER: VoiceId = VoiceId::new(EngineId::new("other"), "en-alba");
        let descriptor = EngineDescriptor {
            voices: &[ALBA, VoiceDescriptor { id: OTHER, ..ALBA }],
            ..VALID
        };

        assert_eq!(defect_of(&descriptor), Defect::ForeignVoice(OTHER));
    }

    #[test]
    fn check_descriptor_fails_when_empty_name() {
        let descriptor = EngineDescriptor {
            voices: &[ALBA, VoiceDescriptor { name: " ", ..BRUNO }],
            ..VALID
        };

        assert_eq!(defect_of(&descriptor), Defect::EmptyName(BRUNO.id));
    }

    #[test]
    fn check_descriptor_fails_when_empty_parameters() {
        let descriptor = EngineDescriptor {
            variants: Variants {
                max: Variant {
                    parameters: "",
                    ..VARIANT
                },
                ..VALID.variants
            },
            ..VALID
        };

        assert_eq!(
            defect_of(&descriptor),
            Defect::EmptyParameters(Quality::Max)
        );
    }

    #[test]
    fn check_descriptor_fails_when_revision() {
        for revision in ["", "0123456789ABCDEF0123456789abcdef01234567", "main"] {
            let in_voice = with_artifact(Artifact { revision, ..PROMPT });
            let in_variant = with_variant_artifact(Artifact {
                revision,
                ..WEIGHTS
            });

            assert_eq!(
                defect_of(&in_voice),
                Defect::Revision { artifact: "prompt" }
            );
            assert_eq!(
                defect_of(&in_variant),
                Defect::Revision {
                    artifact: "weights"
                }
            );
        }
    }

    #[test]
    fn check_descriptor_fails_when_sha256() {
        let sha256 = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b85";

        let in_voice = with_artifact(Artifact { sha256, ..PROMPT });
        let in_variant = with_variant_artifact(Artifact { sha256, ..WEIGHTS });

        assert_eq!(defect_of(&in_voice), Defect::Sha256 { artifact: "prompt" });
        assert_eq!(
            defect_of(&in_variant),
            Defect::Sha256 {
                artifact: "weights"
            }
        );
    }

    #[test]
    fn check_descriptor_fails_when_empty_extent() {
        let extent = Extent::Range {
            offset: 512,
            bytes: 0,
        };

        let in_voice = with_artifact(Artifact { extent, ..PROMPT });
        let in_variant = with_variant_artifact(Artifact { extent, ..WEIGHTS });

        assert_eq!(
            defect_of(&in_voice),
            Defect::EmptyExtent { artifact: "prompt" }
        );
        assert_eq!(
            defect_of(&in_variant),
            Defect::EmptyExtent {
                artifact: "weights"
            }
        );
    }

    #[test]
    fn check_descriptor_fails_when_duplicate_key() {
        let descriptor = with_artifact(Artifact {
            key: "weights",
            ..PROMPT
        });

        assert_eq!(
            defect_of(&descriptor),
            Defect::DuplicateKey {
                artifact: "weights"
            }
        );
    }

    #[test]
    fn check_descriptor_reports_first_defect_in_sequence() {
        let descriptor = EngineDescriptor {
            voices: &[ALBA, VoiceDescriptor { name: "", ..ALBA }],
            ..VALID
        };

        assert_eq!(defect_of(&descriptor), Defect::DuplicateVoice(ALBA.id));
    }

    #[test]
    fn core_error_shows_engine_and_defect() {
        let error = CoreError::InvalidDescriptor {
            engine: ID,
            defect: Defect::EmptyExtent { artifact: "prompt" },
        };

        let text = error.to_string();

        let expected =
            "the descriptor of engine test is not valid: the extent of artifact prompt has 0 bytes";
        assert_eq!(text, expected);
    }

    #[test]
    fn voices_for_returns_voices_of_language_in_sequence() {
        let english: Vec<_> = VALID
            .voices_for(Language::En)
            .map(|voice| voice.id)
            .collect();

        assert_eq!(english, [ALBA.id, CLARA.id]);
        assert_eq!(VALID.voices_for(Language::De).count(), 0);
    }
}
