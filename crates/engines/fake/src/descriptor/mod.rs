mod voices;

use std::num::NonZeroUsize;

use antenna_core::{
    EngineDescriptor, EngineId, Localized, ModelLicense, SampleRate, Variant, Variants,
};

pub(crate) use voices::{Timbre, timbre_of};

const ID: EngineId = EngineId::new("fake");

pub(crate) const SAMPLE_RATE: SampleRate = SampleRate::HZ_24000;

const MAX_SEGMENT_CHARS: NonZeroUsize = NonZeroUsize::new(400).unwrap();

/// The fake engine has no model files, so each variant is the same.
const VARIANT: Variant = Variant {
    parameters: "0",
    artifacts: &[],
};

pub(crate) static DESCRIPTOR: EngineDescriptor = EngineDescriptor {
    id: ID,
    name: "Fake",
    version: "1",
    summary: Localized {
        en: "Test tones with no model download",
        es: "Tonos de prueba sin descarga de modelos",
    },
    license: ModelLicense {
        name: "GPL-3.0-or-later",
        url: "https://www.gnu.org/licenses/gpl-3.0.html",
        attribution: "",
    },
    sample_rate: SAMPLE_RATE,
    max_segment_chars: MAX_SEGMENT_CHARS,
    variants: Variants {
        fast: VARIANT,
        balanced: VARIANT,
        max: VARIANT,
    },
    voices: &voices::VOICES,
};
