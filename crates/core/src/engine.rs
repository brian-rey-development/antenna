use std::ops::ControlFlow;

use crate::{
    EngineDescriptor, EngineError, ModelFiles, PcmChunk, Quality, Segment, VoiceDescriptor,
};

/// The callback that receives each chunk of a segment. It returns `ControlFlow::Break` to stop the
/// synthesis.
pub type Emit<'a> = &'a mut dyn FnMut(PcmChunk) -> ControlFlow<()>;

/// An object that describes an engine type and loads engines. This trait is an extension point.
///
/// ```
/// use std::num::NonZeroUsize;
///
/// use antenna_core::{
///     Emit, Engine, EngineDescriptor, EngineError, EngineFactory, EngineId, Language,
///     Localized, ModelFiles, ModelLicense, Quality, SampleRate, Segment, Variant, Variants,
///     VoiceDescriptor, VoiceId,
/// };
///
/// const ID: EngineId = EngineId::new("silence");
/// const VARIANT: Variant = Variant { parameters: "0", artifacts: &[] };
/// const TEXT: Localized = Localized { en: "Silence", es: "Silencio" };
///
/// static DESCRIPTOR: EngineDescriptor = EngineDescriptor {
///     id: ID,
///     name: "Silence",
///     version: "1",
///     summary: TEXT,
///     license: ModelLicense { name: "MIT", url: "https://mit-license.org", attribution: "" },
///     sample_rate: SampleRate::HZ_24000,
///     max_segment_chars: NonZeroUsize::new(400).unwrap(),
///     variants: Variants { fast: VARIANT, balanced: VARIANT, max: VARIANT },
///     voices: &[VoiceDescriptor {
///         id: VoiceId::new(ID, "en-quiet"),
///         language: Language::En,
///         name: "Quiet",
///         description: TEXT,
///         artifacts: &[],
///     }],
/// };
///
/// struct SilenceFactory;
/// struct Silence;
///
/// impl EngineFactory for SilenceFactory {
///     fn descriptor(&self) -> &'static EngineDescriptor {
///         &DESCRIPTOR
///     }
///
///     fn load(
///         &self,
///         _: &VoiceDescriptor,
///         _: Quality,
///         _: &ModelFiles,
///     ) -> Result<Box<dyn Engine>, EngineError> {
///         Ok(Box::new(Silence))
///     }
/// }
///
/// impl Engine for Silence {
///     fn synthesize(&mut self, _: &Segment, _: Emit<'_>) -> Result<(), EngineError> {
///         Ok(())
///     }
/// }
///
/// let factory = SilenceFactory;
/// let voice = &factory.descriptor().voices[0];
/// assert!(factory.load(voice, Quality::Fast, &ModelFiles::default()).is_ok());
/// ```
pub trait EngineFactory: Send + Sync + 'static {
    /// Returns the static descriptor of the engine type.
    fn descriptor(&self) -> &'static EngineDescriptor;

    /// Loads an engine for one voice at one quality. The function reads the weights, builds the
    /// model and runs one warm-up synthesis.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::MissingFile`] if `files` has no path for an artifact, and
    /// [`EngineError::Load`] if the engine cannot load.
    fn load(
        &self,
        voice: &VoiceDescriptor,
        quality: Quality,
        files: &ModelFiles,
    ) -> Result<Box<dyn Engine>, EngineError>;
}

/// An object that converts segments into chunks with one loaded voice. This trait is an
/// extension point.
///
/// The engine calls `emit` for each chunk as soon as the chunk is available. If `emit` returns
/// `ControlFlow::Break`, the engine stops and returns `Ok(())` before it calls `emit` again. The
/// engine is deterministic, does no I/O and resets its generation state at the start of each
/// segment.
///
/// ```
/// use std::ops::ControlFlow;
///
/// use antenna_core::{Emit, Engine, EngineError, PcmChunk, Segment, SegmentIndex};
///
/// const CHUNK_SAMPLES: usize = 480;
///
/// struct Silence;
///
/// impl Engine for Silence {
///     fn synthesize(&mut self, segment: &Segment, emit: Emit<'_>) -> Result<(), EngineError> {
///         for _ in segment.text().chars() {
///             if emit(PcmChunk::new(vec![0.0; CHUNK_SAMPLES])).is_break() {
///                 return Ok(());
///             }
///         }
///         Ok(())
///     }
/// }
///
/// let mut samples = 0;
/// let segment = Segment::new(SegmentIndex::new(0), "Hi", 0..2);
/// Silence
///     .synthesize(&segment, &mut |chunk| {
///         samples += chunk.samples().len();
///         ControlFlow::Continue(())
///     })
///     .unwrap();
/// assert_eq!(samples, 2 * CHUNK_SAMPLES);
/// ```
pub trait Engine: Send {
    /// Synthesizes one segment and sends each chunk to `emit`.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Inference`] if the model fails, and [`EngineError::Runaway`] if the
    /// generation passes its step limit.
    fn synthesize(&mut self, segment: &Segment, emit: Emit<'_>) -> Result<(), EngineError>;
}
