use antenna_core::{CoreError, Defect, EngineError};

/// Returns the condition of the conformance suite that a descriptor error breaks.
pub(crate) fn of_descriptor(error: CoreError) -> &'static str {
    match error {
        CoreError::InvalidDescriptor { defect, .. } => of_defect(defect),
        CoreError::EmptyDocument | CoreError::ZeroSampleRate | CoreError::InvalidTextHash => {
            "check_descriptor accepts the descriptor"
        }
    }
}

fn of_defect(defect: Defect) -> &'static str {
    match defect {
        Defect::NoVoices => "the engine has a voice",
        Defect::DuplicateVoice(_) => "each voice id is unique",
        Defect::ForeignVoice(_) => "each voice id has the engine id of the descriptor",
        Defect::EmptyName(_) => "each voice has a name",
        Defect::EmptyParameters(_) => "each variant has a parameters text",
        Defect::Revision { .. } => "each artifact revision is 40 lowercase hex characters",
        Defect::Sha256 { .. } => "each artifact SHA-256 is 64 lowercase hex characters",
        Defect::EmptyExtent { .. } => "each artifact extent has more than 0 bytes",
        Defect::DuplicateKey { .. } => "each artifact key is unique in the request of a voice",
    }
}

/// Returns the condition of the conformance suite that an engine error breaks.
pub(crate) fn of_engine(error: &EngineError) -> &'static str {
    match error {
        EngineError::MissingFile { .. } => "the model files contain each key that the engine reads",
        EngineError::Load(_) => "the engine loads",
        EngineError::Inference(_) => "the synthesis succeeds",
        EngineError::Runaway { .. } => "the generation ends before its step limit",
    }
}
