use antenna_core::SampleRate;
use rtrb::Producer;

use crate::resample::Conversion;

/// The parts of an open output stream that the feed thread needs.
pub(crate) struct StreamParts<S> {
    pub(crate) stream: S,
    pub(crate) rate: SampleRate,
    pub(crate) producer: Producer<f32>,
}

/// The output side of the feed thread. It converts samples to the device rate and pushes them
/// into the ring buffer.
pub(crate) struct Stream<S> {
    _device: S,
    producer: Producer<f32>,
    conversion: Conversion,
    rate: SampleRate,
}

impl<S> Stream<S> {
    pub(crate) fn new(parts: StreamParts<S>, engine: SampleRate) -> Self {
        Self {
            _device: parts.stream,
            conversion: Conversion::new(engine, parts.rate),
            producer: parts.producer,
            rate: parts.rate,
        }
    }

    pub(crate) fn rate(&self) -> SampleRate {
        self.rate
    }

    pub(crate) fn retune(&mut self, engine: SampleRate) {
        self.conversion = Conversion::new(engine, self.rate);
    }

    pub(crate) fn reset(&mut self) {
        self.conversion.reset();
    }

    pub(crate) fn buffered_frames(&self) -> usize {
        self.producer.buffer().capacity() - self.producer.slots()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.buffered_frames() == 0
    }

    pub(crate) fn push(&mut self, block: &[f32], scratch: &mut Vec<f32>) {
        scratch.clear();
        self.conversion.convert(block, scratch);
        self.write(scratch);
    }

    pub(crate) fn finish(&mut self, scratch: &mut Vec<f32>) {
        scratch.clear();
        self.conversion.finish(scratch);
        self.write(scratch);
    }

    fn write(&mut self, samples: &[f32]) {
        if self.producer.push_entire_slice(samples).is_err() {
            tracing::warn!(
                dropped = samples.len(),
                "the ring buffer is full, so the player drops samples"
            );
        }
    }
}
