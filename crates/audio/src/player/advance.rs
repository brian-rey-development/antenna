use std::sync::atomic::Ordering;
use std::time::Duration;

use super::PlayerState;
use super::current::Block;
use super::feed::{Feed, Work};
use super::stream::Stream;
use crate::frames::sample_count;

const PREFILL_DURATION_MS: u64 = 50;
const FEED_TARGET_MS: u64 = 300;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Outcome {
    Pushed,
    Full,
    Waiting,
    Done,
}

impl<S> Feed<S> {
    pub(super) fn step(&mut self) -> Work {
        if !self.is_active() {
            return Work::Idle;
        }
        self.advance(Duration::from_millis(FEED_TARGET_MS))
    }

    pub(super) fn prefill(&mut self) {
        while self.advance(Duration::from_millis(PREFILL_DURATION_MS)) == Work::Pending {}
    }

    fn advance(&mut self, target: Duration) -> Work {
        let outcome = self.push_block(target);
        let work = match outcome {
            Outcome::Pushed => Work::Pending,
            Outcome::Full | Outcome::Waiting | Outcome::Done => Work::Idle,
        };
        if !self.is_active() {
            return work;
        }
        match outcome {
            Outcome::Pushed => self.transition(PlayerState::Playing),
            Outcome::Full => {}
            Outcome::Waiting => self.transition(PlayerState::Buffering),
            Outcome::Done => self.drain(),
        }
        work
    }

    fn push_block(&mut self, target: Duration) -> Outcome {
        let (Some(track), Some(stream)) = (self.track.as_mut(), self.stream.as_mut()) else {
            return Outcome::Full;
        };
        let target_frames = sample_count(target, stream.rate());
        if stream.buffered_frames() >= target_frames {
            return Outcome::Full;
        }
        self.block.clear();
        match track.next_block(&mut self.block) {
            Block::Ready => {
                stream.push(&self.block, &mut self.converted);
                Outcome::Pushed
            }
            Block::Waiting => Outcome::Waiting,
            Block::Done => {
                stream.finish(&mut self.converted);
                Outcome::Done
            }
        }
    }

    fn drain(&mut self) {
        // The last callback finds a short buffer, and this is not an underrun.
        if self.stream.as_ref().is_some_and(Stream::is_empty) {
            self.transition(PlayerState::Ended);
        } else {
            self.atomics.is_buffering.store(true, Ordering::Relaxed);
        }
    }
}

#[cfg(test)]
mod tests {
    use antenna_core::SampleRate;

    use super::super::harness::{Harness, RATE, stored, track};
    use super::*;
    use crate::Track;
    use crate::test_support::pcm_ramp;

    #[test]
    fn feed_plays_stored_segments_in_order() {
        let directory = tempfile::tempdir().unwrap();
        let (first, second, third) = (
            pcm_ramp(0, 3_000),
            pcm_ramp(3_000, 1_500),
            pcm_ramp(4_500, 2_000),
        );
        let harness = Harness::start(RATE);
        let loaded = harness.load(
            1,
            track(
                directory.path(),
                RATE,
                &[Some(&first), Some(&second), Some(&third)],
            ),
        );
        harness.play(loaded.id);
        let mut consumer = harness.next_stream();

        let samples = harness.drain_until(&mut consumer, loaded.id, PlayerState::Ended);

        assert_eq!(samples, pcm_ramp(0, 6_500));
    }

    #[test]
    fn feed_plays_live_chunks_then_swaps_to_stored() {
        let directory = tempfile::tempdir().unwrap();
        let (first, second) = (pcm_ramp(0, 3_000), pcm_ramp(3_000, 1_000));
        let harness = Harness::start(RATE);
        let loaded = harness.load(1, track(directory.path(), RATE, &[None, None]));
        harness.play(loaded.id);
        let mut consumer = harness.next_stream();
        harness.live(loaded.id, 0, &first[..1_500]);
        let mut samples = harness.drain_samples(&mut consumer, 1_500);

        harness.store(
            loaded.id,
            0,
            stored(directory.path(), "first.wav", RATE, &first),
        );
        harness.live(loaded.id, 1, &second);
        harness.finish_live(loaded.id);
        samples.extend(harness.drain_until(&mut consumer, loaded.id, PlayerState::Ended));

        assert_eq!(samples, pcm_ramp(0, 4_000));
    }

    #[test]
    fn feed_buffers_when_segment_pending() {
        let directory = tempfile::tempdir().unwrap();
        let (first, second) = (pcm_ramp(0, 500), pcm_ramp(500, 500));
        let harness = Harness::start(RATE);
        let loaded = harness.load(1, track(directory.path(), RATE, &[Some(&first), None]));
        harness.play(loaded.id);
        let mut consumer = harness.next_stream();
        harness.wait_for(loaded.id, PlayerState::Buffering);

        harness.live(loaded.id, 1, &second);
        harness.wait_for(loaded.id, PlayerState::Playing);

        let samples = harness.drain_samples(&mut consumer, 1_000);
        assert_eq!(samples, pcm_ramp(0, 1_000));
    }

    #[test]
    fn feed_ends_after_last_segment() {
        let directory = tempfile::tempdir().unwrap();
        let harness = Harness::start(RATE);
        let loaded = harness.load(
            1,
            track(directory.path(), RATE, &[Some(&pcm_ramp(0, 3_000))]),
        );
        harness.play(loaded.id);
        let mut consumer = harness.next_stream();

        let samples = harness.drain_until(&mut consumer, loaded.id, PlayerState::Ended);

        assert_eq!(samples, pcm_ramp(0, 3_000));
        assert!(harness.atomics.paused.load(Ordering::Relaxed));
    }

    #[test]
    fn feed_ends_when_track_has_no_segments() {
        let harness = Harness::start(RATE);
        let loaded = harness.load(1, Track::new(RATE, []));

        harness.play(loaded.id);

        harness.wait_for(loaded.id, PlayerState::Ended);
    }

    #[test]
    fn feed_stops_output_when_paused() {
        let directory = tempfile::tempdir().unwrap();
        let harness = Harness::start(RATE);
        let loaded = harness.load(
            1,
            track(directory.path(), RATE, &[Some(&pcm_ramp(0, 5_000))]),
        );
        harness.play(loaded.id);
        let mut consumer = harness.next_stream();
        harness.drain_samples(&mut consumer, 100);

        harness.pause(loaded.id);
        harness.wait_for(loaded.id, PlayerState::Paused);

        assert!(harness.drain(&mut consumer).is_empty());
    }

    #[test]
    fn feed_resumes_from_same_sample_when_paused_then_played() {
        let directory = tempfile::tempdir().unwrap();
        let harness = Harness::start(RATE);
        let loaded = harness.load(
            1,
            track(directory.path(), RATE, &[Some(&pcm_ramp(0, 5_000))]),
        );
        harness.play(loaded.id);
        let mut consumer = harness.next_stream();
        let mut samples = harness.drain_samples(&mut consumer, 100);
        harness.pause(loaded.id);
        harness.wait_for(loaded.id, PlayerState::Paused);

        harness.play(loaded.id);
        samples.extend(harness.drain_until(&mut consumer, loaded.id, PlayerState::Ended));

        assert_eq!(samples, pcm_ramp(0, 5_000));
    }

    #[test]
    fn feed_resamples_to_device_rate_when_rates_differ() {
        let directory = tempfile::tempdir().unwrap();
        let harness = Harness::start(SampleRate::HZ_48000);
        let loaded = harness.load(
            1,
            track(
                directory.path(),
                RATE,
                &[Some(&pcm_ramp(0, 3_000)), Some(&pcm_ramp(0, 1_000))],
            ),
        );
        harness.play(loaded.id);
        let mut consumer = harness.next_stream();

        let samples = harness.drain_until(&mut consumer, loaded.id, PlayerState::Ended);

        assert_eq!(samples.len(), 8_000);
    }

    #[test]
    fn feed_schedules_segment_start_in_device_frames_when_rates_differ() {
        let directory = tempfile::tempdir().unwrap();
        let harness = Harness::start(SampleRate::HZ_48000);
        let loaded = harness.load(
            1,
            track(
                directory.path(),
                RATE,
                &[Some(&pcm_ramp(0, 3_000)), Some(&pcm_ramp(0, 1_000))],
            ),
        );
        harness.play(loaded.id);
        let mut consumer = harness.next_stream();
        harness.drain_until(&mut consumer, loaded.id, PlayerState::Ended);

        let at_second = loaded.atomics.locate(6_000).unwrap();
        let before_second = loaded.atomics.locate(5_999).unwrap();

        assert_eq!((at_second.segment, at_second.offset_frames), (1, 0));
        assert_eq!(
            (before_second.segment, before_second.offset_frames),
            (0, 5_999)
        );
    }
}
