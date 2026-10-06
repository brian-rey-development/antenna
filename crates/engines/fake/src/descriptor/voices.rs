use antenna_core::{Language, Localized, VoiceDescriptor, VoiceId};

use super::ID;

/// The name, the tone and the description that one voice has in each language.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Timbre {
    name: &'static str,
    /// The frequency of the tone of segment 0.
    pub(crate) base_hz: u32,
    description: Localized,
}

const ALBA: Timbre = Timbre {
    name: "Alba",
    base_hz: 220,
    description: Localized {
        en: "Test tone, low pitch",
        es: "Tono de prueba, grave",
    },
};
const BRUNO: Timbre = Timbre {
    name: "Bruno",
    base_hz: 247,
    description: Localized {
        en: "Test tone, mid-low pitch",
        es: "Tono de prueba, medio grave",
    },
};
const CLARA: Timbre = Timbre {
    name: "Clara",
    base_hz: 262,
    description: Localized {
        en: "Test tone, mid-high pitch",
        es: "Tono de prueba, medio agudo",
    },
};
const DARIO: Timbre = Timbre {
    name: "Dario",
    base_hz: 294,
    description: Localized {
        en: "Test tone, high pitch",
        es: "Tono de prueba, agudo",
    },
};
const TIMBRES: [Timbre; 4] = [ALBA, BRUNO, CLARA, DARIO];

const fn voice(language: Language, key: &'static str, timbre: Timbre) -> VoiceDescriptor {
    VoiceDescriptor {
        id: VoiceId::new(ID, key),
        language,
        name: timbre.name,
        description: timbre.description,
        artifacts: &[],
    }
}

/// Makes the four voices of each language, in the sequence of `TIMBRES`. The key of a voice is the
/// language code and the lowercase name, for example "es-alba".
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
    let position = VOICES.iter().position(|candidate| candidate.id == voice)?;
    TIMBRES.get(position % TIMBRES.len()).copied()
}
