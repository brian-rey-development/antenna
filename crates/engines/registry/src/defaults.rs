use std::sync::Arc;

use antenna_core::{EngineFactory, Language, VoiceDescriptor, VoiceId};

use crate::RegistryError;

/// The default voice id of each language. Each entry has the `engine-<name>` feature of its
/// engine, so a build without that engine has no entry for it.
pub(crate) const DEFAULT_VOICES: &[VoiceId] = &[];

/// The default voice of each language. The struct has a field for each language, so
/// `Registry::default_voice` cannot fail.
pub(crate) struct Defaults {
    en: &'static VoiceDescriptor,
    es: &'static VoiceDescriptor,
    pt: &'static VoiceDescriptor,
    fr: &'static VoiceDescriptor,
    it: &'static VoiceDescriptor,
    de: &'static VoiceDescriptor,
}

impl Defaults {
    /// Selects the default voice of each language. The default voice is the voice of the first id
    /// that names a registered voice of the language. If no id names one, it is the first
    /// registered voice of the language.
    pub(crate) fn new(
        factories: &[Arc<dyn EngineFactory>],
        ids: &[VoiceId],
    ) -> Result<Self, RegistryError> {
        let select = |language| select_voice(factories, ids, language);
        Ok(Self {
            en: select(Language::En)?,
            es: select(Language::Es)?,
            pt: select(Language::Pt)?,
            fr: select(Language::Fr)?,
            it: select(Language::It)?,
            de: select(Language::De)?,
        })
    }

    pub(crate) fn get(&self, language: Language) -> &'static VoiceDescriptor {
        match language {
            Language::En => self.en,
            Language::Es => self.es,
            Language::Pt => self.pt,
            Language::Fr => self.fr,
            Language::It => self.it,
            Language::De => self.de,
        }
    }
}

/// Returns the voices of a language, in registration sequence.
pub(crate) fn voices_of(
    factories: &[Arc<dyn EngineFactory>],
    language: Language,
) -> impl Iterator<Item = &'static VoiceDescriptor> {
    factories
        .iter()
        .flat_map(move |factory| factory.descriptor().voices_for(language))
}

fn select_voice(
    factories: &[Arc<dyn EngineFactory>],
    ids: &[VoiceId],
    language: Language,
) -> Result<&'static VoiceDescriptor, RegistryError> {
    let voices = || voices_of(factories, language);
    ids.iter()
        .find_map(|id| voices().find(|voice| voice.id == *id))
        .or_else(|| voices().next())
        .ok_or(RegistryError::NoVoice(language))
}

#[cfg(test)]
mod tests {
    use antenna_core::EngineId;

    use crate::test_factory::{FULL, NO_SPANISH, SECOND, registry};

    use super::*;

    #[cfg(feature = "engine-fake")]
    #[test]
    fn registry_has_default_voice_when_fake_enabled() {
        let registry = crate::Registry::new().unwrap();

        for language in Language::ALL {
            let voice = registry.default_voice(language);

            assert_eq!(voice.language, language);
            assert_eq!(voice.id.to_string(), format!("fake/{language}-alba"));
        }
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

        assert_eq!(
            registry.default_voice(Language::Es).id.to_string(),
            "test/es-a"
        );
    }

    #[test]
    fn registry_uses_default_entry_when_voice_registered() {
        let spanish = SECOND.voices[0].id;

        let registry = registry(&[&FULL, &SECOND], &[spanish]).unwrap();

        assert_eq!(registry.default_voice(Language::Es).id, spanish);
        assert_eq!(
            registry.default_voice(Language::En).id.to_string(),
            "test/en-a"
        );
    }
}
