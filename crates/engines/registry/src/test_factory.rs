//! Test engine factories with static descriptors, for the unit tests of the registry.

use std::num::NonZeroUsize;
use std::sync::Arc;

use antenna_core::{
    Engine, EngineDescriptor, EngineError, EngineFactory, EngineId, Language, Localized,
    ModelFiles, ModelLicense, Quality, SampleRate, Variant, Variants, VoiceDescriptor, VoiceId,
};

use crate::{Registry, RegistryError};

pub(crate) const TEST: EngineId = EngineId::new("test");
pub(crate) const OTHER: EngineId = EngineId::new("other");
const TEXT: Localized = Localized {
    en: "Test",
    es: "Prueba",
};
const VARIANT: Variant = Variant {
    parameters: "0",
    artifacts: &[],
};

const fn voice(engine: EngineId, key: &'static str, language: Language) -> VoiceDescriptor {
    VoiceDescriptor {
        id: VoiceId::new(engine, key),
        language,
        name: key,
        description: TEXT,
        artifacts: &[],
    }
}

const fn descriptor(id: EngineId, voices: &'static [VoiceDescriptor]) -> EngineDescriptor {
    EngineDescriptor {
        id,
        name: "Test",
        version: "1",
        summary: TEXT,
        license: ModelLicense {
            name: "MIT",
            url: "https://mit-license.org",
            attribution: "",
        },
        sample_rate: SampleRate::HZ_24000,
        max_segment_chars: NonZeroUsize::MIN,
        variants: Variants {
            fast: VARIANT,
            balanced: VARIANT,
            max: VARIANT,
        },
        voices,
    }
}

/// An engine with two English voices and one voice for each other language.
pub(crate) static FULL: EngineDescriptor = descriptor(
    TEST,
    &[
        voice(TEST, "en-a", Language::En),
        voice(TEST, "en-b", Language::En),
        voice(TEST, "es-a", Language::Es),
        voice(TEST, "pt-a", Language::Pt),
        voice(TEST, "fr-a", Language::Fr),
        voice(TEST, "it-a", Language::It),
        voice(TEST, "de-a", Language::De),
    ],
);

/// An engine with one voice for each language except Spanish.
pub(crate) static NO_SPANISH: EngineDescriptor = descriptor(
    TEST,
    &[
        voice(TEST, "en-a", Language::En),
        voice(TEST, "pt-a", Language::Pt),
        voice(TEST, "fr-a", Language::Fr),
        voice(TEST, "it-a", Language::It),
        voice(TEST, "de-a", Language::De),
    ],
);

/// A second engine with one Spanish voice.
pub(crate) static SECOND: EngineDescriptor =
    descriptor(OTHER, &[voice(OTHER, "es-z", Language::Es)]);

/// An engine whose descriptor has no voices.
pub(crate) static EMPTY: EngineDescriptor = descriptor(OTHER, &[]);

struct TestFactory(&'static EngineDescriptor);

impl EngineFactory for TestFactory {
    fn descriptor(&self) -> &'static EngineDescriptor {
        self.0
    }

    fn load(
        &self,
        _: &VoiceDescriptor,
        _: Quality,
        _: &ModelFiles,
    ) -> Result<Box<dyn Engine>, EngineError> {
        Err(EngineError::Load("a test factory does not load".into()))
    }
}

/// Makes a registry from test factories in sequence and default voice ids.
pub(crate) fn registry(
    descriptors: &[&'static EngineDescriptor],
    defaults: &[VoiceId],
) -> Result<Registry, RegistryError> {
    let factories = descriptors
        .iter()
        .map(|descriptor| Arc::new(TestFactory(descriptor)) as Arc<dyn EngineFactory>)
        .collect();
    Registry::from_parts(factories, defaults)
}
