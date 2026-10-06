use std::sync::Arc;

use antenna_core::{EngineFactory, Language, VoiceDescriptor, VoiceId};

use crate::RegistryError;

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
