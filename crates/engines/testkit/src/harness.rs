use std::fmt::{self, Debug, Formatter};
use std::ops::ControlFlow;

use antenna_core::{
    Engine, EngineError, EngineFactory, Language, ModelFiles, PcmChunk, Quality, Segment,
    SegmentIndex, VoiceDescriptor, check_descriptor,
};

use crate::{Cause, Condition, Violation};

const SHORT_SEGMENT: &str = "Hello.";
const LONG_SEGMENT_SOURCE: &str = "The quick brown fox jumps over the lazy dog. ";
const EDGE_SEGMENTS: [&str; 4] = ["Hi", "...", "1 2 3", "Ok \u{1F399}"];
const MAX_PEAK: f32 = 1.0;

/// The function that gives the model files of a voice at a quality.
pub type FilesFn<'a> = &'a dyn Fn(&VoiceDescriptor, Quality) -> ModelFiles;

/// The conformance suite of one engine factory.
///
/// Each check except [`Harness::check_descriptor`] runs on the first voice of each language,
/// one time for each [`Quality`].
pub struct Harness<'a> {
    factory: &'a dyn EngineFactory,
    files: FilesFn<'a>,
}

impl Debug for Harness<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Harness")
            .field("engine", &self.factory.descriptor().id)
            .finish_non_exhaustive()
    }
}

impl<'a> Harness<'a> {
    /// Makes the suite of a factory. `files` gives the model files of each case.
    pub fn new(factory: &'a dyn EngineFactory, files: FilesFn<'a>) -> Self {
        Self { factory, files }
    }

    /// Checks that `antenna_core::check_descriptor` accepts the descriptor.
    ///
    /// # Errors
    ///
    /// Returns a [`Violation`] with no voice and no quality if the descriptor has a defect.
    pub fn check_descriptor(&self) -> Result<(), Violation> {
        check_descriptor(self.factory.descriptor()).map_err(|error| Violation {
            voice: None,
            quality: None,
            condition: Condition::Descriptor,
            source: Some(Cause::Descriptor(error)),
        })
    }

    /// Checks the audio of a short segment and of a segment of `max_segment_chars` characters.
    /// Each segment must give samples, all samples must be finite and the peak must be 1.0 or
    /// less.
    ///
    /// # Errors
    ///
    /// Returns a [`Violation`] for the first case that fails.
    pub fn check_audio(&self) -> Result<(), Violation> {
        let chars = self.factory.descriptor().max_segment_chars.get();
        let long: String = LONG_SEGMENT_SOURCE.chars().cycle().take(chars).collect();
        self.for_each_case(|case| {
            let mut engine = case.load()?;
            for text in [SHORT_SEGMENT, long.as_str()] {
                check_samples(&synthesize(engine.as_mut(), text)?)?;
            }
            Ok(())
        })
    }

    /// Checks that `synthesize` returns `Ok` and does not call `emit` again after `emit`
    /// returns `Break` on the first chunk.
    ///
    /// # Errors
    ///
    /// Returns a [`Violation`] for the first case that fails.
    pub fn check_break(&self) -> Result<(), Violation> {
        self.for_each_case(|case| {
            let calls = synthesize_until_break(case.load()?.as_mut())?;
            if calls > 1 {
                return Err(Condition::Break.into());
            }
            Ok(())
        })
    }

    /// Checks that two calls with the same segment give the same samples, bit for bit.
    ///
    /// # Errors
    ///
    /// Returns a [`Violation`] for the first case that fails.
    pub fn check_determinism(&self) -> Result<(), Violation> {
        self.for_each_case(|case| {
            let mut engine = case.load()?;
            let first = synthesize(engine.as_mut(), SHORT_SEGMENT)?;
            let second = synthesize(engine.as_mut(), SHORT_SEGMENT)?;
            if !is_same(&first, &second) {
                return Err(Condition::Determinism.into());
            }
            Ok(())
        })
    }

    /// Checks that a call after a `Break` gives the same samples as a call on a new engine.
    ///
    /// # Errors
    ///
    /// Returns a [`Violation`] for the first case that fails.
    pub fn check_reset(&self) -> Result<(), Violation> {
        self.for_each_case(|case| {
            let mut stopped = case.load()?;
            synthesize_until_break(stopped.as_mut())?;
            let after_break = synthesize(stopped.as_mut(), SHORT_SEGMENT)?;
            let fresh = synthesize(case.load()?.as_mut(), SHORT_SEGMENT)?;
            if !is_same(&after_break, &fresh) {
                return Err(Condition::Reset.into());
            }
            Ok(())
        })
    }

    /// Checks that short, punctuation, digit and pictograph segments each return `Ok`.
    ///
    /// # Errors
    ///
    /// Returns a [`Violation`] for the first case that fails.
    pub fn check_edge_segments(&self) -> Result<(), Violation> {
        self.for_each_case(|case| {
            let mut engine = case.load()?;
            EDGE_SEGMENTS
                .iter()
                .try_for_each(|text| synthesize(engine.as_mut(), text).map(drop))
        })
    }

    /// Runs a check on the first voice of each language at each quality, and stops at the first
    /// failed condition.
    fn for_each_case(
        &self,
        check: impl Fn(&Case<'_>) -> Result<(), Failure>,
    ) -> Result<(), Violation> {
        let descriptor = self.factory.descriptor();
        let voices = Language::ALL
            .into_iter()
            .filter_map(|language| descriptor.voices_for(language).next());
        for voice in voices {
            for quality in Quality::ALL {
                let case = Case {
                    harness: self,
                    voice,
                    quality,
                };
                check(&case).map_err(|failure| Violation {
                    voice: Some(voice.id),
                    quality: Some(quality),
                    condition: failure.condition,
                    source: failure.source.map(Cause::Engine),
                })?;
            }
        }
        Ok(())
    }
}

/// The failed condition of one case, and the error that broke it.
struct Failure {
    condition: Condition,
    source: Option<EngineError>,
}

impl Failure {
    fn with_error(condition: Condition, error: EngineError) -> Self {
        Self {
            condition,
            source: Some(error),
        }
    }
}

impl From<Condition> for Failure {
    fn from(condition: Condition) -> Self {
        Self {
            condition,
            source: None,
        }
    }
}

/// One voice at one quality.
struct Case<'a> {
    harness: &'a Harness<'a>,
    voice: &'static VoiceDescriptor,
    quality: Quality,
}

impl Case<'_> {
    fn load(&self) -> Result<Box<dyn Engine>, Failure> {
        let files = (self.harness.files)(self.voice, self.quality);
        self.harness
            .factory
            .load(self.voice, self.quality, &files)
            .map_err(|error| Failure::with_error(Condition::Load, error))
    }
}

fn segment(text: &str) -> Segment {
    Segment::new(SegmentIndex::new(0), text, 0..text.len())
}

fn synthesize(engine: &mut dyn Engine, text: &str) -> Result<Vec<f32>, Failure> {
    let mut samples = Vec::new();
    let mut emit = |chunk: PcmChunk| {
        samples.extend(chunk.into_samples());
        ControlFlow::Continue(())
    };
    engine
        .synthesize(&segment(text), &mut emit)
        .map_err(|error| Failure::with_error(Condition::Synthesis, error))?;
    Ok(samples)
}

/// Synthesizes the short segment with an `emit` that returns `Break`, and returns the number of
/// calls of `emit`.
fn synthesize_until_break(engine: &mut dyn Engine) -> Result<usize, Failure> {
    let mut calls = 0;
    let mut emit = |_| {
        calls += 1;
        ControlFlow::Break(())
    };
    engine
        .synthesize(&segment(SHORT_SEGMENT), &mut emit)
        .map_err(|error| Failure::with_error(Condition::Synthesis, error))?;
    Ok(calls)
}

fn check_samples(samples: &[f32]) -> Result<(), Failure> {
    if samples.is_empty() {
        return Err(Condition::NonEmptyAudio.into());
    }
    if !samples.iter().all(|sample| sample.is_finite()) {
        return Err(Condition::FiniteSamples.into());
    }
    if samples.iter().any(|sample| sample.abs() > MAX_PEAK) {
        return Err(Condition::Peak.into());
    }
    Ok(())
}

/// Compares the samples bit for bit, so two NaN values with the same bits are the same.
fn is_same(first: &[f32], second: &[f32]) -> bool {
    first.len() == second.len()
        && first
            .iter()
            .zip(second)
            .all(|(left, right)| left.to_bits() == right.to_bits())
}
