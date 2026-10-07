use antenna_core::{Output, SampleRate, SegmentIndex, Sink, SinkError};
use flume::Sender;

use crate::TrackId;
use crate::player::{FeedCommand, send};

/// An output that sends the chunks of a job to a track of the [`Player`](crate::Player).
///
/// The sink of the output completes its segment when it is dropped. The pipeline has one worker,
/// so an old sink cannot complete a segment of a newer job.
#[derive(Debug)]
pub struct LiveOutput {
    commands: Sender<FeedCommand>,
    id: TrackId,
    rate: SampleRate,
}

impl LiveOutput {
    pub(crate) fn new(commands: Sender<FeedCommand>, id: TrackId, rate: SampleRate) -> Self {
        Self { commands, id, rate }
    }
}

impl Output for LiveOutput {
    /// Opens the sink of the track.
    ///
    /// # Errors
    ///
    /// Returns [`SinkError::RateMismatch`] if `rate` is not the sample rate of the track.
    fn open(self: Box<Self>, rate: SampleRate) -> Result<Box<dyn Sink>, SinkError> {
        if rate != self.rate {
            return Err(SinkError::RateMismatch {
                expected: self.rate,
                actual: rate,
            });
        }
        Ok(Box::new(LiveSink {
            commands: self.commands,
            id: self.id,
        }))
    }
}

#[derive(Debug)]
struct LiveSink {
    commands: Sender<FeedCommand>,
    id: TrackId,
}

impl Sink for LiveSink {
    fn begin_segment(&mut self, index: SegmentIndex) -> Result<(), SinkError> {
        send(
            &self.commands,
            FeedCommand::LiveBegin {
                id: self.id,
                segment: index,
            },
        );
        Ok(())
    }

    fn write(&mut self, samples: &[f32]) -> Result<(), SinkError> {
        let samples = samples.to_vec();
        send(
            &self.commands,
            FeedCommand::LiveChunk {
                id: self.id,
                samples,
            },
        );
        Ok(())
    }

    fn finish(self: Box<Self>) -> Result<(), SinkError> {
        Ok(())
    }
}

impl Drop for LiveSink {
    fn drop(&mut self) {
        // A job that stops early drops its sink, and the partial segment must still complete.
        send(&self.commands, FeedCommand::LiveEnd { id: self.id });
    }
}

#[cfg(test)]
mod tests {
    use antenna_core::Output;

    use super::*;
    use crate::Track;
    use crate::player::PlayerState;
    use crate::player::harness::{Harness, RATE, track};
    use crate::test_support::pcm_ramp;

    fn sink(harness: &Harness, id: TrackId) -> Box<dyn Sink> {
        let output = LiveOutput::new(harness.commands.clone(), id, RATE);
        Box::new(output).open(RATE).unwrap()
    }

    #[test]
    fn live_output_fails_when_rate_differs() {
        let (commands, _receiver) = flume::unbounded();
        let output = Box::new(LiveOutput::new(commands, TrackId::new(1), RATE));

        let result = output.open(SampleRate::HZ_48000);

        let expected = SinkError::RateMismatch {
            expected: RATE,
            actual: SampleRate::HZ_48000,
        };
        assert_eq!(result.err(), Some(expected));
    }

    #[test]
    fn live_sink_drops_samples_when_track_replaced() {
        let harness = Harness::start(RATE);
        let old = harness.load(1, Track::new(RATE, [None]));
        let mut old_sink = sink(&harness, old.id);
        let current = harness.load(2, Track::new(RATE, [None]));

        let began = old_sink.begin_segment(SegmentIndex::new(0));
        let wrote = old_sink.write(&pcm_ramp(0, 100));
        harness.play(current.id);
        let mut consumer = harness.next_stream();
        harness.wait_for(current.id, PlayerState::Buffering);

        assert_eq!((began, wrote), (Ok(()), Ok(())));
        assert!(harness.drain(&mut consumer).is_empty());
    }

    #[test]
    fn live_sink_returns_ok_when_player_stopped() {
        let (commands, receiver) = flume::unbounded();
        let output = Box::new(LiveOutput::new(commands, TrackId::new(1), RATE));
        let mut sink = output.open(RATE).unwrap();
        drop(receiver);

        let wrote = sink.write(&pcm_ramp(0, 10));

        assert_eq!(wrote, Ok(()));
    }

    #[test]
    fn live_sink_completes_segment_when_finished() {
        let harness = Harness::start(RATE);
        let loaded = harness.load(1, Track::new(RATE, [None]));
        let mut sink = sink(&harness, loaded.id);
        harness.play(loaded.id);
        let mut consumer = harness.next_stream();

        sink.begin_segment(SegmentIndex::new(0)).unwrap();
        sink.write(&pcm_ramp(0, 40)).unwrap();
        sink.write(&pcm_ramp(40, 60)).unwrap();
        sink.finish().unwrap();

        let samples = harness.drain_until(&mut consumer, loaded.id, PlayerState::Ended);
        assert_eq!(samples, pcm_ramp(0, 100));
    }

    #[test]
    fn live_sink_completes_segment_when_dropped() {
        let directory = tempfile::tempdir().unwrap();
        let (live, stored) = (pcm_ramp(0, 50), pcm_ramp(50, 50));
        let harness = Harness::start(RATE);
        let loaded = harness.load(1, track(directory.path(), RATE, &[None, Some(&stored)]));
        let mut sink = sink(&harness, loaded.id);
        harness.play(loaded.id);
        let mut consumer = harness.next_stream();

        sink.begin_segment(SegmentIndex::new(0)).unwrap();
        sink.write(&live).unwrap();
        drop(sink);

        let samples = harness.drain_until(&mut consumer, loaded.id, PlayerState::Ended);
        assert_eq!(samples, pcm_ramp(0, 100));
    }
}
