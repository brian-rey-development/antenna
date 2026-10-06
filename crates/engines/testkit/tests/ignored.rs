//! A conformance suite with the `ignore` form, for an engine that needs downloaded models.

use std::num::NonZeroUsize;

use antenna_core::{
    Emit, Engine, EngineDescriptor, EngineError, EngineFactory, EngineId, Language, Localized,
    ModelFiles, ModelLicense, PcmChunk, Quality, SampleRate, Segment, Variant, Variants,
    VoiceDescriptor, VoiceId,
};

const ID: EngineId = EngineId::new("ignored");
const CHUNK_SAMPLES: usize = 240;
const TEXT: Localized = Localized {
    en: "Silence",
    es: "Silencio",
};
const VARIANT: Variant = Variant {
    parameters: "0",
    artifacts: &[],
};
static DESCRIPTOR: EngineDescriptor = EngineDescriptor {
    id: ID,
    name: "Silence",
    version: "1",
    summary: TEXT,
    license: ModelLicense {
        name: "MIT",
        url: "https://mit-license.org",
        attribution: "",
    },
    sample_rate: SampleRate::HZ_24000,
    max_segment_chars: NonZeroUsize::new(200).unwrap(),
    variants: Variants {
        fast: VARIANT,
        balanced: VARIANT,
        max: VARIANT,
    },
    voices: &[VoiceDescriptor {
        id: VoiceId::new(ID, "en-quiet"),
        language: Language::En,
        name: "Quiet",
        description: TEXT,
        artifacts: &[],
    }],
};

struct SilenceFactory;

struct Silence;

impl EngineFactory for SilenceFactory {
    fn descriptor(&self) -> &'static EngineDescriptor {
        &DESCRIPTOR
    }

    fn load(
        &self,
        _: &VoiceDescriptor,
        _: Quality,
        _: &ModelFiles,
    ) -> Result<Box<dyn Engine>, EngineError> {
        Ok(Box::new(Silence))
    }
}

impl Engine for Silence {
    fn synthesize(&mut self, segment: &Segment, emit: Emit<'_>) -> Result<(), EngineError> {
        for _ in segment.text().chars() {
            if emit(PcmChunk::new(vec![0.0; CHUNK_SAMPLES])).is_break() {
                return Ok(());
            }
        }
        Ok(())
    }
}

antenna_engine_testkit::conformance!(
    SilenceFactory,
    &|_, _| ModelFiles::default(),
    ignore = "needs models"
);
