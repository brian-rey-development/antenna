use std::collections::BTreeSet;

use crate::hex::is_lowercase_hex;
use crate::{Artifact, CoreError, Defect, EngineDescriptor, Quality, Variants};

const REVISION_HEX_CHARS: usize = 40;
const SHA256_HEX_CHARS: usize = 64;

/// Checks that an engine descriptor is complete and consistent.
///
/// # Errors
///
/// Returns [`CoreError::InvalidDescriptor`] with the first [`Defect`] that it finds. The checks
/// run in the declaration sequence of the `Defect` variants.
pub fn check_descriptor(descriptor: &EngineDescriptor) -> Result<(), CoreError> {
    match find_defect(descriptor) {
        Some(defect) => Err(CoreError::InvalidDescriptor {
            engine: descriptor.id,
            defect,
        }),
        None => Ok(()),
    }
}

fn find_defect(descriptor: &EngineDescriptor) -> Option<Defect> {
    voice_defect(descriptor)
        .or_else(|| parameters_defect(&descriptor.variants))
        .or_else(|| artifact_defect(descriptor))
        .or_else(|| key_defect(descriptor))
}

fn voice_defect(descriptor: &EngineDescriptor) -> Option<Defect> {
    let voices = descriptor.voices;
    if voices.is_empty() {
        return Some(Defect::NoVoices);
    }
    let mut ids = BTreeSet::new();
    let duplicate = voices.iter().find(|voice| !ids.insert(voice.id));
    let foreign = || {
        voices
            .iter()
            .find(|voice| voice.id.engine() != descriptor.id)
    };
    let unnamed = || voices.iter().find(|voice| voice.name.trim().is_empty());
    duplicate
        .map(|voice| Defect::DuplicateVoice(voice.id))
        .or_else(|| foreign().map(|voice| Defect::ForeignVoice(voice.id)))
        .or_else(|| unnamed().map(|voice| Defect::EmptyName(voice.id)))
}

fn parameters_defect(variants: &Variants) -> Option<Defect> {
    Quality::ALL
        .into_iter()
        .find(|quality| variants.get(*quality).parameters.trim().is_empty())
        .map(Defect::EmptyParameters)
}

fn artifact_defect(descriptor: &EngineDescriptor) -> Option<Defect> {
    let artifacts = || {
        let variants = Quality::ALL.map(|quality| descriptor.variants.get(quality).artifacts);
        let voices = descriptor.voices.iter().map(|voice| voice.artifacts);
        variants.into_iter().chain(voices).flatten()
    };
    let find = |is_defect: fn(&Artifact) -> bool| artifacts().find(|artifact| is_defect(artifact));
    find(|artifact| !is_lowercase_hex(artifact.revision, REVISION_HEX_CHARS))
        .map(|artifact| Defect::Revision {
            artifact: artifact.key,
        })
        .or_else(|| {
            find(|artifact| !is_lowercase_hex(artifact.sha256, SHA256_HEX_CHARS)).map(|artifact| {
                Defect::Sha256 {
                    artifact: artifact.key,
                }
            })
        })
        .or_else(|| {
            find(|artifact| artifact.extent.bytes() == 0).map(|artifact| Defect::EmptyExtent {
                artifact: artifact.key,
            })
        })
}

/// Finds a key that occurs two times in the request of one voice at one quality, which is the
/// artifacts of the variant plus the artifacts of the voice.
fn key_defect(descriptor: &EngineDescriptor) -> Option<Defect> {
    Quality::ALL.into_iter().find_map(|quality| {
        let shared = descriptor.variants.get(quality).artifacts;
        descriptor.voices.iter().find_map(|voice| {
            let mut keys = BTreeSet::new();
            shared
                .iter()
                .chain(voice.artifacts)
                .find(|artifact| !keys.insert(artifact.key))
                .map(|artifact| Defect::DuplicateKey {
                    artifact: artifact.key,
                })
        })
    })
}
