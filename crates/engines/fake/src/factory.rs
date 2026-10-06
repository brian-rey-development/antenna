use antenna_core::{
    Engine, EngineDescriptor, EngineError, EngineFactory, ModelFiles, Quality, SegmentIndex,
    VoiceDescriptor,
};

use crate::descriptor::{DESCRIPTOR, timbre_of};
use crate::engine::FakeEngine;
use crate::error::FakeError;

/// The factory of the `fake` engine. It has four voices for each language and no model files.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Factory {
    fault: Option<Fault>,
}

/// A failure that the engines of a [`Factory`] make on purpose, to test the pipeline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fault {
    /// The engine panics when it synthesizes this segment.
    PanicAt {
        /// The index of the segment.
        segment: SegmentIndex,
    },
    /// The engine returns `EngineError::Inference` when it synthesizes this segment.
    FailAt {
        /// The index of the segment.
        segment: SegmentIndex,
    },
}

impl Factory {
    /// Makes a factory whose engines fail with a fault.
    pub fn with_fault(fault: Fault) -> Self {
        Self { fault: Some(fault) }
    }
}

impl EngineFactory for Factory {
    fn descriptor(&self) -> &'static EngineDescriptor {
        &DESCRIPTOR
    }

    fn load(
        &self,
        voice: &VoiceDescriptor,
        _: Quality,
        _: &ModelFiles,
    ) -> Result<Box<dyn Engine>, EngineError> {
        let timbre = timbre_of(voice.id).ok_or_else(|| {
            EngineError::Load(Box::new(FakeError::UnknownVoice { voice: voice.id }))
        })?;
        Ok(Box::new(FakeEngine::new(timbre, self.fault)))
    }
}
