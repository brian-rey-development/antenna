use antenna_core::{Language, Localized, VoiceDescriptor, VoiceId};

use super::ID;

/// The name, the tone and the description that one voice has in each language.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Timbre {
    /// The lowercase name, which is the part of the voice key after the language code.
    key: &'static str,
    name: &'static str,
    /// The frequency of the tone of segment 0.
    pub(crate) base_hz: u32,
    description: Localized,
}

const ALBA: Timbre = Timbre {
    key: "alba",
    name: "Alba",
    base_hz: 220,
    description: Localized {
        en: "Test tone, low pitch",
        es: "Tono de prueba, grave",
    },
};
const BRUNO: Timbre = Timbre {
    key: "bruno",
    name: "Bruno",
    base_hz: 247,
    description: Localized {
        en: "Test tone, mid-low pitch",
        es: "Tono de prueba, medio grave",
    },
};
const CLARA: Timbre = Timbre {
    key: "clara",
    name: "Clara",
    base_hz: 262,
    description: Localized {
        en: "Test tone, mid-high pitch",
        es: "Tono de prueba, medio agudo",
    },
};
const DARIO: Timbre = Timbre {
    key: "dario",
    name: "Dario",
    base_hz: 294,
    description: Localized {
        en: "Test tone, high pitch",
        es: "Tono de prueba, agudo",
    },
};
const TIMBRES: [Timbre; 4] = [ALBA, BRUNO, CLARA, DARIO];

/// The separator between the language code and the timbre key in a voice key.
const KEY_SEPARATOR: char = '-';

const fn voice(language: Language, key: &'static str, timbre: Timbre) -> VoiceDescriptor {
    VoiceDescriptor {
        id: VoiceId::new(ID, key),
        language,
        name: timbre.name,
        description: timbre.description,
        artifacts: &[],
    }
}

/// Makes the four voices of each language. The key of a voice is the language code and the
/// lowercase name, for example "es-alba".
macro_rules! voices {
    ($(($language:expr, $code:literal)),+ $(,)?) => {
        [$(
            voice($language, concat!($code, "-alba"), ALBA),
            voice($language, concat!($code, "-bruno"), BRUNO),
            voice($language, concat!($code, "-clara"), CLARA),
            voice($language, concat!($code, "-dario"), DARIO),
        )+]
    };
}

/// The 24 voices of the `fake` engine.
pub(super) static VOICES: [VoiceDescriptor; 24] = voices![
    (Language::En, "en"),
    (Language::Es, "es"),
    (Language::Pt, "pt"),
    (Language::Fr, "fr"),
    (Language::It, "it"),
    (Language::De, "de"),
];

/// Returns the timbre of a voice of the `fake` engine, or `None` for another voice.
pub(crate) fn timbre_of(voice: VoiceId) -> Option<Timbre> {
    if !VOICES.iter().any(|candidate| candidate.id == voice) {
        return None;
    }
    let (_, key) = voice.key().split_once(KEY_SEPARATOR)?;
    TIMBRES.into_iter().find(|timbre| timbre.key == key)
}
