use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use flume::RecvTimeoutError;

use super::command::send;
use super::current::Clock;
use super::feed::Feed;
use super::stream::Stream;
use super::{PlayerEvent, PlayerState};
use crate::TrackPosition;

const FLUSH_POLL: Duration = Duration::from_millis(1);

struct FlushUnanswered;

impl<S> Feed<S> {
    pub(super) fn seek(&mut self, position: TrackPosition) {
        let Some(track) = self.track.as_ref() else {
            return;
        };
        let (segment, offset) = track.target(position);
        let base_frame = self.flush_ring_buffer();
        self.reposition(segment, offset, base_frame);
        if self.state() == PlayerState::Ended {
            self.transition(PlayerState::Paused);
        }
    }

    /// Discards the audio in the ring buffer. Returns the device frame of the first discarded
    /// sample, or the played frames if no stream exists. A flush that the callback does not
    /// answer in time drops the stream.
    pub(super) fn flush_ring_buffer(&mut self) -> u64 {
        if self.stream.is_some() {
            self.atomics.is_buffering.store(true, Ordering::Relaxed);
            if self.request_flush().is_err() {
                self.lose_stream();
                send(&self.events, PlayerEvent::DeviceLost);
            }
        }
        match self.stream {
            Some(_) => self.atomics.flushed_at_frame.load(Ordering::Relaxed),
            None => self.atomics.played_frames.load(Ordering::Acquire),
        }
    }

    pub(super) fn reposition(&mut self, segment: usize, offset: usize, base_frame: u64) {
        let Some(track) = self.track.as_mut() else {
            return;
        };
        let device = self
            .stream
            .as_ref()
            .map(Stream::rate)
            .or_else(|| self.atomics.device_rate())
            .unwrap_or(track.rate());
        track.restart(segment, offset, Clock::new(base_frame, device));
        if let Some(stream) = self.stream.as_mut() {
            stream.reset();
        }
        self.prefill();
        self.publish_flags();
    }

    fn request_flush(&mut self) -> Result<(), FlushUnanswered> {
        let epoch = self.atomics.flush_epoch.fetch_add(1, Ordering::AcqRel) + 1;
        let deadline = Instant::now() + self.flush_wait;
        while self.atomics.flushed_epoch.load(Ordering::Acquire) < epoch {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(FlushUnanswered);
            }
            match self.commands.recv_timeout(remaining.min(FLUSH_POLL)) {
                Ok(command) => self.backlog.push_back(command),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return Err(FlushUnanswered),
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::Ordering;
    use std::time::Duration;

    use antenna_core::SegmentIndex;
    use rtrb::Consumer;

    use super::super::command::FeedCommand;
    use super::super::harness::{Harness, RATE, TestTrack, track};
    use super::*;
    use crate::test_support::pcm_ramp;

    const SECOND_SEGMENT_START: usize = 6_000;
    const OFFSET_MS: u64 = 100;
    const OFFSET_SAMPLES: usize = 2_400;

    fn offset() -> Duration {
        Duration::from_millis(OFFSET_MS)
    }

    /// Seeks while the track is paused, and returns after the feed thread prefilled the target.
    fn seek_while_paused(
        harness: &Harness,
        loaded: &TestTrack,
        consumer: &mut Consumer<f32>,
        target: (u32, Duration),
    ) {
        harness.pause(loaded.id);
        harness.wait_for(loaded.id, PlayerState::Paused);
        let epoch = harness.atomics.flush_epoch.load(Ordering::Acquire) + 1;
        harness.seek(loaded.id, target.0, target.1);
        harness.drain_until_flushed(consumer, epoch);
        harness.play(loaded.id);
        harness.wait_for(loaded.id, PlayerState::Playing);
    }

    #[test]
    fn seek_delivers_target_sample_in_first_drained_buffer() {
        let directory = tempfile::tempdir().unwrap();
        let harness = Harness::start(RATE);
        let (loaded, mut consumer) = harness.play_two_segments(directory.path());
        harness.drain_samples(&mut consumer, 100);

        seek_while_paused(&harness, &loaded, &mut consumer, (1, offset()));

        let first_buffer = harness.drain(&mut consumer);
        let target = pcm_ramp(SECOND_SEGMENT_START + OFFSET_SAMPLES, 1);
        assert_eq!(first_buffer.first(), target.first());
    }

    #[test]
    fn seek_positions_target_when_offset_exceeds_played_frames() {
        let directory = tempfile::tempdir().unwrap();
        let harness = Harness::start(RATE);
        let (loaded, mut consumer) = harness.play_two_segments(directory.path());
        harness.drain_samples(&mut consumer, 100);
        seek_while_paused(&harness, &loaded, &mut consumer, (1, offset()));

        let drained = harness.drain(&mut consumer).len() as u64;

        let position = loaded.atomics.position(&harness.atomics).unwrap();
        let expected = RATE.duration_of(OFFSET_SAMPLES as u64 + drained);
        assert_eq!(position.segment, SegmentIndex::new(1));
        assert_eq!(position.offset, expected);
    }

    #[test]
    fn seek_clamps_offset_when_beyond_stored_duration() {
        let directory = tempfile::tempdir().unwrap();
        let harness = Harness::start(RATE);
        let (loaded, mut consumer) = harness.play_two_segments(directory.path());
        harness.drain_samples(&mut consumer, 100);
        seek_while_paused(
            &harness,
            &loaded,
            &mut consumer,
            (0, Duration::from_secs(60)),
        );

        let first_buffer = harness.drain(&mut consumer);

        let target = pcm_ramp(SECOND_SEGMENT_START, 1);
        assert_eq!(first_buffer.first(), target.first());
        let flushed = harness.atomics.flushed_at_frame.load(Ordering::Relaxed);
        let located = loaded.atomics.locate(flushed - 1).unwrap();
        assert_eq!((located.segment, located.offset_frames), (0, 5_999));
    }

    #[test]
    fn seek_pauses_track_when_track_ended() {
        let directory = tempfile::tempdir().unwrap();
        let harness = Harness::start(RATE);
        let loaded = harness.load(1, track(directory.path(), RATE, &[Some(&pcm_ramp(0, 100))]));
        harness.play(loaded.id);
        let mut consumer = harness.next_stream();
        harness.drain_until(&mut consumer, loaded.id, PlayerState::Ended);

        harness.seek(loaded.id, 0, Duration::ZERO);
        harness.drain_until_flushed(&mut consumer, 1);

        harness.wait_for(loaded.id, PlayerState::Paused);
    }

    #[test]
    fn seek_buffers_when_target_segment_pending() {
        let directory = tempfile::tempdir().unwrap();
        let harness = Harness::start(RATE);
        let loaded = harness.load(
            1,
            track(directory.path(), RATE, &[Some(&pcm_ramp(0, 9_000)), None]),
        );
        harness.play(loaded.id);
        let mut consumer = harness.next_stream();
        harness.wait_for(loaded.id, PlayerState::Playing);

        harness.seek(loaded.id, 1, Duration::ZERO);
        harness.drain_until_flushed(&mut consumer, 1);

        harness.wait_for(loaded.id, PlayerState::Buffering);
    }

    #[test]
    fn seek_skips_flush_when_device_lost() {
        let directory = tempfile::tempdir().unwrap();
        let harness = Harness::start(RATE);
        let (loaded, mut consumer) = harness.play_two_segments(directory.path());
        harness.drain_samples(&mut consumer, 100);
        harness.send(FeedCommand::DeviceLost);
        harness.wait_for(loaded.id, PlayerState::Paused);

        harness.seek(loaded.id, 1, Duration::ZERO);
        harness.play(loaded.id);
        let mut reopened = harness.next_stream();

        let first = harness.drain_samples(&mut reopened, 1);
        assert_eq!(harness.atomics.flush_epoch.load(Ordering::Relaxed), 0);
        assert_eq!(first.first(), pcm_ramp(SECOND_SEGMENT_START, 1).first());
    }

    #[test]
    fn seek_drops_stream_when_flush_unanswered() {
        let directory = tempfile::tempdir().unwrap();
        let harness = Harness::start_with_flush_wait(RATE, Duration::ZERO);
        let (loaded, _consumer) = harness.play_two_segments(directory.path());
        harness.wait_for(loaded.id, PlayerState::Playing);

        harness.seek(loaded.id, 1, Duration::ZERO);

        harness.wait_for(loaded.id, PlayerState::Paused);

        assert_eq!(harness.next_event(), PlayerEvent::DeviceLost);
        assert!(harness.atomics.paused.load(Ordering::Relaxed));
    }

    #[test]
    fn seek_keeps_prefill_when_stream_reopened_after_unanswered_flush() {
        let directory = tempfile::tempdir().unwrap();
        let harness = Harness::start_with_flush_wait(RATE, Duration::ZERO);
        let (loaded, _consumer) = harness.play_two_segments(directory.path());
        harness.wait_for(loaded.id, PlayerState::Playing);
        harness.seek(loaded.id, 1, Duration::ZERO);
        harness.wait_for(loaded.id, PlayerState::Paused);

        harness.play(loaded.id);
        let mut reopened = harness.next_stream();

        let first = harness.drain_samples(&mut reopened, 1);
        assert_eq!(first.first(), pcm_ramp(SECOND_SEGMENT_START, 1).first());
    }

    #[test]
    fn seek_applies_queued_commands_when_flush_completes() {
        let directory = tempfile::tempdir().unwrap();
        let harness = Harness::start(RATE);
        let (loaded, mut consumer) = harness.play_two_segments(directory.path());
        harness.drain_samples(&mut consumer, 100);
        harness.seek(loaded.id, 1, Duration::ZERO);
        harness.wait_for_flush_request(1);

        harness.seek(loaded.id, 1, offset());
        harness.drain_until_flushed(&mut consumer, 1);
        harness.wait_for_flush_request(2);
        harness.drain_until_flushed(&mut consumer, 2);

        let first = harness.drain_samples(&mut consumer, 1);
        let target = pcm_ramp(SECOND_SEGMENT_START + OFFSET_SAMPLES, 1);
        assert_eq!(first.first(), target.first());
    }
}
