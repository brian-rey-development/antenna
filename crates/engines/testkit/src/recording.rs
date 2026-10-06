use antenna_core::{Output, SampleRate, SegmentIndex, Sink, SinkError};
use flume::{Receiver, Sender};

/// A message from a recording output or its sink to the recording.
#[derive(Debug)]
enum Event {
    Open(SampleRate),
    Begin(SegmentIndex),
    Samples(Vec<f32>),
    Finish,
}

/// An output that sends all that it receives to its [`Recording`].
#[derive(Debug)]
pub struct RecordingOutput {
    sender: Sender<Event>,
}

impl RecordingOutput {
    /// Makes an output and the recording that receives its events.
    pub fn new() -> (Self, Recording) {
        let (sender, receiver) = flume::unbounded();
        (Self { sender }, Recording { receiver })
    }
}

impl Output for RecordingOutput {
    fn open(self: Box<Self>, rate: SampleRate) -> Result<Box<dyn Sink>, SinkError> {
        send(&self.sender, Event::Open(rate))?;
        Ok(Box::new(RecordingSink {
            sender: self.sender,
        }))
    }
}

#[derive(Debug)]
struct RecordingSink {
    sender: Sender<Event>,
}

impl Sink for RecordingSink {
    fn begin_segment(&mut self, index: SegmentIndex) -> Result<(), SinkError> {
        send(&self.sender, Event::Begin(index))
    }

    fn write(&mut self, samples: &[f32]) -> Result<(), SinkError> {
        send(&self.sender, Event::Samples(samples.to_vec()))
    }

    fn finish(self: Box<Self>) -> Result<(), SinkError> {
        send(&self.sender, Event::Finish)
    }
}

fn send(sender: &Sender<Event>, event: Event) -> Result<(), SinkError> {
    sender
        .send(event)
        .map_err(|flume::SendError(_)| SinkError::Closed)
}

/// The receiver of the events of a [`RecordingOutput`].
#[derive(Debug)]
pub struct Recording {
    receiver: Receiver<Event>,
}

impl Recording {
    /// Collects the events that the sink sent until now into one recording.
    ///
    /// # Panics
    ///
    /// Panics if the test did not open the output, because a recording has no rate before the
    /// open.
    pub fn collect(self) -> RecordedAudio {
        let mut rate = None;
        let mut samples = Vec::new();
        let mut segment_starts = Vec::new();
        let mut is_finished = false;
        for event in self.receiver.drain() {
            match event {
                Event::Open(opened) => rate = Some(opened),
                Event::Begin(index) => segment_starts.push((index, samples.len())),
                Event::Samples(chunk) => samples.extend(chunk),
                Event::Finish => is_finished = true,
            }
        }
        RecordedAudio {
            rate: expect_rate(rate),
            samples,
            segment_starts,
            is_finished,
        }
    }
}

#[expect(
    clippy::expect_used,
    reason = "a test that collects an output that it never opened has a defect, and the panic fails it"
)]
fn expect_rate(rate: Option<SampleRate>) -> SampleRate {
    rate.expect("the test collected a recording whose output it did not open")
}

/// The audio that a [`RecordingOutput`] received.
#[derive(Clone, Debug, PartialEq)]
pub struct RecordedAudio {
    /// The sample rate of the open.
    pub rate: SampleRate,
    /// All samples, in the sequence of the writes.
    pub samples: Vec<f32>,
    /// Each segment index and the index of its first sample in `samples`.
    pub segment_starts: Vec<(SegmentIndex, usize)>,
    /// `true` if the sink received `finish`.
    pub is_finished: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recording_marks_segment_starts_when_segments_begin() {
        let (output, recording) = RecordingOutput::new();
        let mut sink = Box::new(output).open(SampleRate::HZ_24000).unwrap();

        sink.begin_segment(SegmentIndex::new(0)).unwrap();
        sink.write(&[0.1, 0.2]).unwrap();
        sink.write(&[0.3]).unwrap();
        sink.begin_segment(SegmentIndex::new(1)).unwrap();
        sink.write(&[0.4]).unwrap();
        sink.finish().unwrap();

        let audio = recording.collect();
        assert_eq!(audio.rate, SampleRate::HZ_24000);
        assert_eq!(audio.samples, [0.1, 0.2, 0.3, 0.4]);
        assert_eq!(
            audio.segment_starts,
            [(SegmentIndex::new(0), 0), (SegmentIndex::new(1), 3)]
        );
        assert!(audio.is_finished);
    }

    #[test]
    fn recording_is_not_finished_when_sink_dropped() {
        let (output, recording) = RecordingOutput::new();
        let mut sink = Box::new(output).open(SampleRate::HZ_22050).unwrap();
        sink.write(&[0.5]).unwrap();

        drop(sink);

        let audio = recording.collect();
        assert_eq!(audio.samples, [0.5]);
        assert!(!audio.is_finished);
    }

    #[test]
    fn sink_fails_when_recording_dropped() {
        let (output, recording) = RecordingOutput::new();
        let mut sink = Box::new(output).open(SampleRate::HZ_24000).unwrap();

        drop(recording);

        assert_eq!(sink.write(&[0.5]), Err(SinkError::Closed));
    }

    #[test]
    #[should_panic(expected = "whose output it did not open")]
    fn recording_panics_when_output_never_opened() {
        let (_output, recording) = RecordingOutput::new();

        recording.collect();
    }
}
