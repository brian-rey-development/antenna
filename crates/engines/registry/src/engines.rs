//! The engines of the build and their default voices. A stage that adds an engine adds its
//! registration line, its `DEFAULT_VOICES` entries and their tests to this module.

use std::sync::Arc;

use antenna_core::{EngineFactory, VoiceId};

/// The default voice id of each language. Each entry has the `engine-<name>` feature of its
/// engine, so a build without that engine has no entry for it.
pub(crate) const DEFAULT_VOICES: &[VoiceId] = &[];

/// Returns the factories of the enabled features, in registration sequence.
pub(crate) fn factories() -> Vec<Arc<dyn EngineFactory>> {
    vec![
        #[cfg(feature = "engine-fake")]
        Arc::new(antenna_engine_fake::Factory::default()),
    ]
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "engine-fake")]
    #[test]
    fn registry_has_default_voice_when_fake_enabled() {
        use antenna_core::Language;

        use crate::Registry;

        let registry = Registry::new().unwrap();

        for language in Language::ALL {
            let voice = registry.default_voice(language);

            assert_eq!(voice.language, language);
            assert_eq!(voice.id.to_string(), format!("fake/{language}-alba"));
        }
    }
}
