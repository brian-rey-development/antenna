use std::f32::consts::TAU;

use antenna_core::{Emit, Engine, EngineError, PcmChunk, Segment, SegmentIndex};

use crate::Fault;
use crate::descriptor::{SAMPLE_RATE, Timbre};
use crate::error::FakeError;

const CHUNK_SAMPLES: usize = 480;
const MS_PER_CHAR: u64 = 40;
const MS_PER_SECOND: u64 = 1_000;
const SAMPLES_PER_CHAR: u64 = MS_PER_CHAR * SAMPLE_RATE.hz() as u64 / MS_PER_SECOND;
const AMPLITUDE: f32 = 0.25;
/// The number of segments after which the frequency of a voice repeats.
const HARMONICS: u32 = 4;

/// An engine of the `fake` engine type, for one voice.
#[derive(Debug)]
pub(crate) struct FakeEngine {
    timbre: Timbre,
    fault: Option<Fault>,
}

impl FakeEngine {
    pub(crate) fn new(timbre: Timbre, fault: Option<Fault>) -> Self {
        Self { timbre, fault }
    }

    #[expect(
        clippy::panic,
        reason = "Fault::PanicAt exists to test the panic isolation of the pipeline"
    )]
    fn check_fault(&self, segment: SegmentIndex) -> Result<(), EngineError> {
        match self.fault {
            Some(Fault::PanicAt { segment: target }) if target == segment => {
                panic!("the fake engine panics at segment {segment} on purpose")
            }
            Some(Fault::FailAt { segment: target }) if target == segment => {
                Err(EngineError::Inference(Box::new(FakeError::Fault {
                    segment,
                })))
            }
            Some(Fault::PanicAt { .. } | Fault::FailAt { .. }) | None => Ok(()),
        }
    }
}

impl Engine for FakeEngine {
    fn synthesize(&mut self, segment: &Segment, emit: Emit<'_>) -> Result<(), EngineError> {
        self.check_fault(segment.index())?;
        let tone = Tone::for_segment(self.timbre, segment);
        for chunk in tone.chunks() {
            if emit(chunk).is_break() {
                return Ok(());
            }
        }
        Ok(())
    }
}

/// The sine tone of one segment.
#[derive(Clone, Copy, Debug)]
struct Tone {
    frequency_hz: u64,
    samples: u64,
}

impl Tone {
    /// The frequency is the base frequency of the voice multiplied by `1 + index mod 4`, and the
    /// duration is 40 ms for each character.
    fn for_segment(timbre: Timbre, segment: &Segment) -> Self {
        let harmonic = segment.index().get() % HARMONICS + 1;
        let chars = segment.text().chars().count() as u64;
        Self {
            frequency_hz: u64::from(timbre.base_hz * harmonic),
            samples: chars * SAMPLES_PER_CHAR,
        }
    }

    fn chunks(self) -> impl Iterator<Item = PcmChunk> {
        (0..self.samples).step_by(CHUNK_SAMPLES).map(move |start| {
            let end = self.samples.min(start + CHUNK_SAMPLES as u64);
            PcmChunk::new(
                (start..end)
                    .map(|position| self.sample_at(position))
                    .collect(),
            )
        })
    }

    /// Calculates the phase with integers, so the tone has no drift and no rounding error.
    #[expect(
        clippy::cast_precision_loss,
        reason = "the phase and the rate are below 2^24, so f32 represents them exactly"
    )]
    fn sample_at(self, position: u64) -> f32 {
        let rate = u64::from(SAMPLE_RATE.hz());
        let phase = position % rate * self.frequency_hz % rate;
        AMPLITUDE * (TAU * phase as f32 / rate as f32).sin()
    }
}
