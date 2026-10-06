use std::collections::BTreeSet;
use std::fmt::{self, Debug, Formatter};
use std::sync::Arc;

use antenna_core::{EngineFactory, Language, VoiceDescriptor, VoiceId, check_descriptor};

use crate::defaults::{Defaults, voices_of};
use crate::{RegistryError, engines};

/// The engine factories of the build, in registration sequence, and the default voices.
pub struct Registry {
    factories: Vec<Arc<dyn EngineFactory>>,
    defaults: Defaults,
}

impl Registry {
    /// Makes the registry of the engines of the enabled features.
    ///
    /// # Errors
    ///
    /// Returns [`RegistryError::Descriptor`] if a descriptor has a defect,
    /// [`RegistryError::DuplicateEngine`] if two engines have the same id, and
    /// [`RegistryError::NoVoice`] if a language has no voice.
    pub fn new() -> Result<Self, RegistryError> {
        Self::from_parts(engines::factories(), engines::DEFAULT_VOICES)
    }

    /// Makes a registry from factories in registration sequence and default voice ids. If no
    /// default id names a registered voice of a language, the default voice of that language is
    /// its first registered voice.
    pub(crate) fn from_parts(
        factories: Vec<Arc<dyn EngineFactory>>,
        default_ids: &[VoiceId],
    ) -> Result<Self, RegistryError> {
        let mut ids = BTreeSet::new();
        for descriptor in factories.iter().map(|factory| factory.descriptor()) {
            check_descriptor(descriptor).map_err(RegistryError::Descriptor)?;
            if !ids.insert(descriptor.id) {
                return Err(RegistryError::DuplicateEngine(descriptor.id));
            }
        }
        let defaults = Defaults::new(&factories, default_ids)?;
        Ok(Self {
            factories,
            defaults,
        })
    }

    /// Returns the factories in registration sequence.
    pub fn factories(&self) -> impl Iterator<Item = &Arc<dyn EngineFactory>> {
        self.factories.iter()
    }

    /// Returns the factory of an engine from the text of its id, for example "qwen3".
    ///
    /// # Errors
    ///
    /// Returns [`RegistryError::UnknownEngine`] with the text if no factory has the id.
    pub fn engine(&self, id: &str) -> Result<&Arc<dyn EngineFactory>, RegistryError> {
        self.factories()
            .find(|factory| factory.descriptor().id.as_str() == id)
            .ok_or_else(|| RegistryError::UnknownEngine(id.to_owned()))
    }

    /// Returns the voices of a language, in registration sequence.
    pub fn voices(&self, language: Language) -> impl Iterator<Item = &'static VoiceDescriptor> {
        voices_of(&self.factories, language)
    }

    /// Returns a voice from the `Display` text of its id, for example "qwen3/es-lucia".
    ///
    /// # Errors
    ///
    /// Returns [`RegistryError::UnknownVoice`] with the text if no voice has the id.
    pub fn voice(&self, id: &str) -> Result<&'static VoiceDescriptor, RegistryError> {
        self.factories()
            .flat_map(|factory| factory.descriptor().voices)
            .find(|voice| voice.id.to_string() == id)
            .ok_or_else(|| RegistryError::UnknownVoice(id.to_owned()))
    }

    /// Returns the default voice of a language.
    pub fn default_voice(&self, language: Language) -> &'static VoiceDescriptor {
        self.defaults.get(language)
    }
}

impl Debug for Registry {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let engines: Vec<_> = self
            .factories()
            .map(|factory| factory.descriptor().id)
            .collect();
        formatter
            .debug_struct("Registry")
            .field("engines", &engines)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use antenna_core::{
        CoreError, Defect, Engine, EngineDescriptor, EngineError, EngineId, Localized, ModelFiles,
        ModelLicense, Quality, SampleRate, Variant, Variants,
    };

    use super::*;

    const TEST: EngineId = EngineId::new("test");
    const OTHER: EngineId = EngineId::new("other");
    const VARIANT: Variant = Variant {
        parameters: "0",
        artifacts: &[],
    };
    const EN_A: VoiceDescriptor = voice(TEST, "en-a", Language::En);
    const EN_B: VoiceDescriptor = voice(TEST, "en-b", Language::En);
    const ES_A: VoiceDescriptor = voice(TEST, "es-a", Language::Es);
    const PT_A: VoiceDescriptor = voice(TEST, "pt-a", Language::Pt);
    const FR_A: VoiceDescriptor = voice(TEST, "fr-a", Language::Fr);
    const IT_A: VoiceDescriptor = voice(TEST, "it-a", Language::It);
    const DE_A: VoiceDescriptor = voice(TEST, "de-a", Language::De);
    const ES_Z: VoiceDescriptor = voice(OTHER, "es-z", Language::Es);
    static FULL: EngineDescriptor = descriptor(TEST, &[EN_A, EN_B, ES_A, PT_A, FR_A, IT_A, DE_A]);
    static NO_SPANISH: EngineDescriptor = descriptor(TEST, &[EN_A, PT_A, FR_A, IT_A, DE_A]);
    static SECOND: EngineDescriptor = descriptor(OTHER, &[ES_Z]);
    static EMPTY: EngineDescriptor = descriptor(OTHER, &[]);

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

    const fn voice(engine: EngineId, key: &'static str, language: Language) -> VoiceDescriptor {
        VoiceDescriptor {
            id: VoiceId::new(engine, key),
            language,
            name: key,
            description: Localized { en: key, es: key },
            artifacts: &[],
        }
    }

    const fn descriptor(id: EngineId, voices: &'static [VoiceDescriptor]) -> EngineDescriptor {
        EngineDescriptor {
            id,
            name: id.as_str(),
            version: "1",
            summary: Localized { en: "", es: "" },
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

    fn registry(
        descriptors: &[&'static EngineDescriptor],
        defaults: &[VoiceId],
    ) -> Result<Registry, RegistryError> {
        let factories = descriptors
            .iter()
            .map(|descriptor| Arc::new(TestFactory(descriptor)) as Arc<dyn EngineFactory>)
            .collect();
        Registry::from_parts(factories, defaults)
    }

    #[test]
    fn registry_finds_voice_when_given_display_text() {
        let registry = registry(&[&FULL, &SECOND], &[]).unwrap();

        for voice in FULL.voices.iter().chain(SECOND.voices) {
            assert_eq!(registry.voice(&voice.id.to_string()).unwrap(), voice);
        }
    }

    #[test]
    fn registry_fails_when_voice_unknown() {
        let registry = registry(&[&FULL], &[]).unwrap();

        for text in ["test/xx-none", "test", "en-a", ""] {
            let result = registry.voice(text);

            assert!(matches!(result, Err(RegistryError::UnknownVoice(unknown)) if unknown == text));
        }
    }

    #[test]
    fn registry_fails_when_descriptor_invalid() {
        let result = registry(&[&FULL, &EMPTY], &[]);

        let expected = CoreError::InvalidDescriptor {
            engine: OTHER,
            defect: Defect::NoVoices,
        };
        assert!(matches!(result, Err(RegistryError::Descriptor(error)) if error == expected));
    }

    #[test]
    fn registry_fails_when_engine_id_duplicate() {
        let result = registry(&[&FULL, &SECOND, &NO_SPANISH], &[]);

        assert!(matches!(result, Err(RegistryError::DuplicateEngine(TEST))));
    }

    #[test]
    fn registry_finds_engine_when_given_id_text() {
        let registry = registry(&[&FULL, &SECOND], &[]).unwrap();

        let engine = registry.engine("other").unwrap();

        assert_eq!(engine.descriptor().id, OTHER);
    }

    #[test]
    fn registry_fails_when_engine_unknown() {
        let registry = registry(&[&FULL], &[]).unwrap();

        let result = registry.engine("qwen3");

        assert!(matches!(result, Err(RegistryError::UnknownEngine(unknown)) if unknown == "qwen3"));
    }

    #[test]
    fn registry_lists_voices_in_registration_sequence() {
        let registry = registry(&[&FULL, &SECOND], &[]).unwrap();

        let spanish: Vec<_> = registry
            .voices(Language::Es)
            .map(|voice| voice.id)
            .collect();
        let engines: Vec<_> = registry
            .factories()
            .map(|factory| factory.descriptor().id)
            .collect();

        assert_eq!(spanish, [ES_A.id, ES_Z.id]);
        assert_eq!(engines, [TEST, OTHER]);
    }

    #[test]
    fn registry_debug_shows_engine_ids() {
        let registry = registry(&[&FULL, &SECOND], &[]).unwrap();

        let text = format!("{registry:?}");

        let expected = r#"Registry { engines: [EngineId("test"), EngineId("other")], .. }"#;
        assert_eq!(text, expected);
    }

    #[test]
    fn registry_fails_when_language_has_no_voice() {
        let result = registry(&[&NO_SPANISH], &[]);

        assert!(matches!(result, Err(RegistryError::NoVoice(Language::Es))));
    }

    #[test]
    fn registry_uses_first_voice_when_default_entry_has_no_factory() {
        let absent = VoiceId::new(EngineId::new("absent"), "es-a");

        let registry = registry(&[&FULL, &SECOND], &[absent]).unwrap();

        assert_eq!(registry.default_voice(Language::Es).id, ES_A.id);
    }

    #[test]
    fn registry_uses_default_entry_when_voice_registered() {
        let registry = registry(&[&FULL, &SECOND], &[ES_Z.id]).unwrap();

        assert_eq!(registry.default_voice(Language::Es).id, ES_Z.id);
        assert_eq!(registry.default_voice(Language::En).id, EN_A.id);
    }
}
