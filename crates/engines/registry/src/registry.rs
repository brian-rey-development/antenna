use std::fmt::{self, Debug, Formatter};
use std::sync::Arc;

use antenna_core::{EngineFactory, Language, VoiceDescriptor, VoiceId, check_descriptor};

use crate::RegistryError;
use crate::defaults::{DEFAULT_VOICES, Defaults, voices_of};

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
    /// Returns [`RegistryError::Descriptor`] if a descriptor has a defect, and
    /// [`RegistryError::NoVoice`] if a language has no voice.
    pub fn new() -> Result<Self, RegistryError> {
        let factories: Vec<Arc<dyn EngineFactory>> = vec![
            #[cfg(feature = "engine-fake")]
            Arc::new(antenna_engine_fake::FakeFactory::default()),
        ];
        Self::from_parts(factories, DEFAULT_VOICES)
    }

    /// Makes a registry from factories in registration sequence and default voice ids. If no
    /// default id names a registered voice of a language, the default voice of that language is
    /// its first registered voice.
    pub(crate) fn from_parts(
        factories: Vec<Arc<dyn EngineFactory>>,
        default_ids: &[VoiceId],
    ) -> Result<Self, RegistryError> {
        for factory in &factories {
            check_descriptor(factory.descriptor()).map_err(RegistryError::Descriptor)?;
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
    use antenna_core::{CoreError, Defect};

    use super::*;
    use crate::test_factory::{EMPTY, FULL, OTHER, SECOND, TEST, registry};

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
            .map(|voice| voice.id.key())
            .collect();
        let engines: Vec<_> = registry
            .factories()
            .map(|factory| factory.descriptor().id)
            .collect();

        assert_eq!(spanish, ["es-a", "es-z"]);
        assert_eq!(engines, [TEST, OTHER]);
    }

    #[test]
    fn registry_debug_shows_engine_ids() {
        let registry = registry(&[&FULL, &SECOND], &[]).unwrap();

        let text = format!("{registry:?}");

        assert_eq!(
            text,
            r#"Registry { engines: [EngineId("test"), EngineId("other")], .. }"#
        );
    }
}
