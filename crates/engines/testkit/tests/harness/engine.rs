//! A test engine with one configurable defect, for the self-tests of the conformance suite.

use std::mem;
use std::num::NonZeroUsize;

use antenna_core::{
    Emit, Engine, EngineDescriptor, EngineError, EngineFactory, EngineId, Language, Localized,
    ModelFiles, ModelLicense, PcmChunk, Quality, SampleRate, Segment, Variant, Variants,
    VoiceDescriptor, VoiceId,
};
use antenna_engine_testkit::Harness;

const ID: EngineId = EngineId::new("test");
const CHUNK_SAMPLES: usize = 240;
const SAMPLES_PER_CHAR: usize = 100;
const AMPLITUDE: f32 = 0.5;
const LOUD: f32 = 1.5;
const TEXT: Localized = Localized {
    en: "Test",
    es: "Prueba",
};
const VARIANT: Variant = Variant {
    parameters: "0",
    artifacts: &[],
};
pub(crate) const EN_FIRST: VoiceDescriptor = VoiceDescriptor {
    id: VoiceId::new(ID, "en-a"),
    language: Language::En,
    name: "A",
    description: TEXT,
    artifacts: &[],
};
pub(crate) const VOICES: [VoiceDescriptor; 3] = [
    EN_FIRST,
    VoiceDescriptor {
        id: VoiceId::new(ID, "en-b"),
        name: "B",
        ..EN_FIRST
    },
    VoiceDescriptor {
        id: VoiceId::new(ID, "es-c"),
        language: Language::Es,
        ..EN_FIRST
    },
];
static DESCRIPTOR: EngineDescriptor = EngineDescriptor {
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
    max_segment_chars: NonZeroUsize::new(200).unwrap(),
    variants: Variants {
        fast: VARIANT,
        balanced: VARIANT,
        max: VARIANT,
    },
    voices: &VOICES,
};
pub(crate) static NO_VOICES: EngineDescriptor = EngineDescriptor {
    voices: &[],
    ..DESCRIPTOR
};

/// The defect of a test engine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Defect {
    None,
    NotFinite,
    TooLoud,
    SilentAt(Quality),
    IgnoresBreak,
    ChangesEachCall,
    KeepsStateAfterBreak,
    FailsOnPictograph,
    NeedsModelFile,
}

pub(crate) struct TestFactory {
    pub(crate) descriptor: &'static EngineDescriptor,
    pub(crate) defect: Defect,
}

struct TestEngine {
    defect: Defect,
    quality: Quality,
    calls: u8,
    resume_at: usize,
}

impl EngineFactory for TestFactory {
    fn descriptor(&self) -> &'static EngineDescriptor {
        self.descriptor
    }

    fn load(
        &self,
        _: &VoiceDescriptor,
        quality: Quality,
        files: &ModelFiles,
    ) -> Result<Box<dyn Engine>, EngineError> {
        let defect = self.defect;
        if defect == Defect::NeedsModelFile {
            files.path("weights")?;
        }
        Ok(Box::new(TestEngine {
            defect,
            quality,
            calls: 0,
            resume_at: 0,
        }))
    }
}

impl Engine for TestEngine {
    fn synthesize(&mut self, segment: &Segment, emit: Emit<'_>) -> Result<(), EngineError> {
        if self.defect == Defect::FailsOnPictograph && !segment.text().is_ascii() {
            return Err(EngineError::Inference("no pictographs".into()));
        }
        self.calls += 1;
        let samples = self.samples(segment.text());
        let start = mem::take(&mut self.resume_at);
        for (index, chunk) in samples.chunks(CHUNK_SAMPLES).enumerate().skip(start) {
            let is_break = emit(PcmChunk::new(chunk.to_vec())).is_break();
            if is_break && self.defect != Defect::IgnoresBreak {
                if self.defect == Defect::KeepsStateAfterBreak {
                    self.resume_at = index + 1;
                }
                return Ok(());
            }
        }
        Ok(())
    }
}

impl TestEngine {
    fn samples(&self, text: &str) -> Vec<f32> {
        let value = match self.defect {
            Defect::NotFinite => f32::NAN,
            Defect::TooLoud => LOUD,
            Defect::SilentAt(quality) if quality == self.quality => return Vec::new(),
            Defect::ChangesEachCall => AMPLITUDE / f32::from(self.calls),
            Defect::None
            | Defect::SilentAt(_)
            | Defect::IgnoresBreak
            | Defect::KeepsStateAfterBreak
            | Defect::FailsOnPictograph
            | Defect::NeedsModelFile => AMPLITUDE,
        };
        vec![value; text.len() * SAMPLES_PER_CHAR]
    }
}

fn files(_: &VoiceDescriptor, _: Quality) -> ModelFiles {
    ModelFiles::default()
}

pub(crate) fn factory(defect: Defect) -> TestFactory {
    TestFactory {
        descriptor: &DESCRIPTOR,
        defect,
    }
}

pub(crate) fn harness_of(factory: &TestFactory) -> Harness<'_> {
    Harness::new(factory, &files)
}
