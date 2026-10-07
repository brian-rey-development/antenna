use std::num::{NonZeroU32, NonZeroUsize};

use antenna_core::{
    Artifact, EngineDescriptor, EngineId, Language, Localized, ModelLicense, SampleRate, Variant,
    Variants, VoiceDescriptor, VoiceId,
};

use crate::fixture::Fixture;
use crate::support::leak;

const TEXT: Localized = Localized {
    en: "Test",
    es: "Prueba",
};

pub(crate) struct Parts<'a> {
    pub(crate) fast: &'a [&'static Artifact],
    pub(crate) balanced: &'a [&'static Artifact],
    pub(crate) max: &'a [&'static Artifact],
    pub(crate) voices: &'a [&'a [&'static Artifact]],
}

pub(crate) fn engine(name: &'static str, parts: &Parts<'_>) -> &'static EngineDescriptor {
    let id = EngineId::new(name);
    leak(EngineDescriptor {
        id,
        name,
        version: "1",
        summary: TEXT,
        license: ModelLicense {
            name: "MIT",
            url: "https://example.org/license",
            attribution: "",
        },
        sample_rate: SampleRate::new(NonZeroU32::MIN),
        max_segment_chars: NonZeroUsize::MIN,
        variants: Variants {
            fast: variant(parts.fast),
            balanced: variant(parts.balanced),
            max: variant(parts.max),
        },
        voices: voices(id, parts.voices),
    })
}

pub(crate) fn single_file_engine(
    name: &'static str,
    artifact: &'static Artifact,
) -> &'static EngineDescriptor {
    engine(
        name,
        &Parts {
            fast: &[artifact],
            balanced: &[artifact],
            max: &[artifact],
            voices: &[&[]],
        },
    )
}

fn voices(id: EngineId, voices: &[&[&'static Artifact]]) -> &'static [VoiceDescriptor] {
    let voices: Vec<VoiceDescriptor> = voices
        .iter()
        .enumerate()
        .map(|(index, artifacts)| VoiceDescriptor {
            id: VoiceId::new(id, Box::leak(format!("voice-{index}").into_boxed_str())),
            language: Language::En,
            name: "Voice",
            description: TEXT,
            artifacts: copy(artifacts),
        })
        .collect();
    Box::leak(voices.into_boxed_slice())
}

fn variant(artifacts: &[&'static Artifact]) -> Variant {
    Variant {
        parameters: "1B",
        artifacts: copy(artifacts),
    }
}

fn copy(artifacts: &[&'static Artifact]) -> &'static [Artifact] {
    let artifacts: Vec<Artifact> = artifacts.iter().map(|artifact| **artifact).collect();
    Box::leak(artifacts.into_boxed_slice())
}

pub(crate) struct Alpha {
    pub(crate) descriptor: &'static EngineDescriptor,
    pub(crate) weights: &'static Artifact,
    pub(crate) prompt_a: &'static Artifact,
    pub(crate) prompt_b: &'static Artifact,
}

pub(crate) const WEIGHTS_BYTES: usize = 3_000;
pub(crate) const PROMPT_A_BYTES: usize = 500;
pub(crate) const PROMPT_B_BYTES: usize = 600;

pub(crate) fn alpha(fixture: &Fixture) -> Alpha {
    let blob = |key, length| fixture.published(key, length).0;
    let fast = blob("fast", 100);
    let weights = blob("weights", WEIGHTS_BYTES);
    let max = blob("max", 200);
    let prompt_a = blob("prompt-a", PROMPT_A_BYTES);
    let prompt_b = blob("prompt-b", PROMPT_B_BYTES);
    let parts = Parts {
        fast: &[fast],
        balanced: &[weights],
        max: &[max],
        voices: &[&[prompt_a], &[prompt_b]],
    };
    Alpha {
        descriptor: engine("alpha", &parts),
        weights,
        prompt_a,
        prompt_b,
    }
}
