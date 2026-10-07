use std::sync::Arc;
use std::sync::atomic::Ordering;

use super::atomics::TrackAtomics;
use super::command::{FeedCommand, send};
use super::current::{Clock, CurrentTrack};
use super::feed::{Feed, Flow};
use super::stream::Stream;
use super::{PlayerEvent, PlayerState};
use crate::{AudioError, Track, TrackId};

impl<S> Feed<S> {
    pub(super) fn apply(&mut self, command: FeedCommand) -> Flow {
        if self.is_stale(&command) {
            return Flow::Continue;
        }
        match command {
            FeedCommand::Load { id, track, atomics } => self.load(id, track, atomics),
            FeedCommand::Play { .. } => self.play(),
            FeedCommand::Pause { .. } => self.pause(),
            FeedCommand::Seek { position, .. } => self.seek(position),
            FeedCommand::Stored {
                segment,
                path,
                duration,
                ..
            } => self.edit_track(|track| track.set_stored(usize::from(segment), path, duration)),
            FeedCommand::LiveBegin { segment, .. } => {
                self.edit_track(|track| track.live_begin(usize::from(segment)));
            }
            FeedCommand::LiveChunk { samples, .. } => {
                self.edit_track(|track| track.live_chunk(&samples));
            }
            FeedCommand::LiveEnd { .. } => self.edit_track(CurrentTrack::live_end),
            FeedCommand::DeviceLost => self.lose_stream(),
            FeedCommand::Shutdown => return Flow::Stop,
        }
        Flow::Continue
    }

    fn is_stale(&self, command: &FeedCommand) -> bool {
        let current = self.track.as_ref().map(CurrentTrack::id);
        command.track().is_some_and(|id| Some(id) != current)
    }

    fn edit_track(&mut self, edit: impl FnOnce(&mut CurrentTrack)) {
        if let Some(track) = self.track.as_mut() {
            edit(track);
        }
    }

    fn load(&mut self, id: TrackId, track: Track, atomics: Arc<TrackAtomics>) {
        let engine = track.rate();
        self.transition(PlayerState::Paused);
        let base_frame = self.flush_ring_buffer();
        let device = self.stream.as_ref().map_or(engine, Stream::rate);
        match self.stream.as_mut() {
            Some(stream) => stream.retune(engine),
            None => self
                .atomics
                .device_rate_hz
                .store(engine.hz(), Ordering::Relaxed),
        }
        let clock = Clock::new(base_frame, device);
        self.track = Some(CurrentTrack::new(id, track, atomics, clock));
        self.atomics.paused.store(true, Ordering::Relaxed);
        self.atomics.is_buffering.store(false, Ordering::Relaxed);
    }

    fn play(&mut self) {
        if self.state() != PlayerState::Paused {
            return;
        }
        if self.stream.is_none()
            && let Err(error) = self.open()
        {
            tracing::warn!(?error, "cannot open the output device");
            send(&self.events, PlayerEvent::DeviceLost);
            return;
        }
        self.transition(PlayerState::Playing);
    }

    fn pause(&mut self) {
        if self.is_active() {
            self.transition(PlayerState::Paused);
        }
    }

    pub(super) fn lose_stream(&mut self) {
        self.stream = None;
        // No callback answers a flush request any more, and a new stream must not see it.
        let requested = self.atomics.flush_epoch.load(Ordering::Acquire);
        self.atomics
            .flushed_epoch
            .store(requested, Ordering::Release);
        self.atomics.paused.store(true, Ordering::Relaxed);
        if self.is_active() {
            self.transition(PlayerState::Paused);
        }
    }

    fn open(&mut self) -> Result<(), AudioError> {
        let Some(track) = self.track.as_ref() else {
            return Ok(());
        };
        let engine = track.rate();
        let previous = self.atomics.device_rate().unwrap_or(engine);
        let played = self.atomics.played_frames.load(Ordering::Acquire);
        let (segment, offset) = track.resume_point(played, previous);
        let parts = (self.open_stream)()?;
        self.atomics
            .device_rate_hz
            .store(parts.rate.hz(), Ordering::Relaxed);
        self.stream = Some(Stream::new(parts, engine));
        self.reposition(segment, offset, played);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::harness::{Harness, RATE, track};
    use super::*;
    use crate::test_support::pcm_ramp;

    #[test]
    fn feed_reopens_stream_when_play_after_device_lost() {
        let directory = tempfile::tempdir().unwrap();
        let harness = Harness::start(RATE);
        let loaded = harness.load(
            1,
            track(directory.path(), RATE, &[Some(&pcm_ramp(0, 9_000))]),
        );
        harness.play(loaded.id);
        let mut first = harness.next_stream();
        let played = harness.drain_samples(&mut first, 700).len();

        harness.send(FeedCommand::DeviceLost);
        harness.wait_for(loaded.id, PlayerState::Paused);
        harness.play(loaded.id);
        let mut second = harness.next_stream();

        let resumed = harness.drain_samples(&mut second, 1);
        assert_eq!(harness.opens.load(Ordering::Relaxed), 2);
        assert_eq!(resumed.first(), pcm_ramp(played, 1).first());
    }

    #[test]
    fn feed_stays_paused_when_open_fails() {
        let directory = tempfile::tempdir().unwrap();
        let harness = Harness::start_without_device();
        let loaded = harness.load(1, track(directory.path(), RATE, &[Some(&pcm_ramp(0, 100))]));

        harness.play(loaded.id);

        assert_eq!(harness.next_event(), PlayerEvent::DeviceLost);
        assert_eq!(loaded.atomics.state(), PlayerState::Paused);
    }

    #[test]
    fn feed_flushes_old_audio_when_track_loaded() {
        let directory = tempfile::tempdir().unwrap();
        let harness = Harness::start(RATE);
        let old = harness.load(
            1,
            track(directory.path(), RATE, &[Some(&pcm_ramp(0, 9_000))]),
        );
        harness.play(old.id);
        let mut consumer = harness.next_stream();
        harness.drain_samples(&mut consumer, 100);
        let current = harness.load(
            2,
            track(directory.path(), RATE, &[Some(&pcm_ramp(20_000, 50))]),
        );
        harness.drain_until_flushed(&mut consumer, 1);

        harness.play(current.id);
        let samples = harness.drain_until(&mut consumer, current.id, PlayerState::Ended);

        assert_eq!(samples, pcm_ramp(20_000, 50));
    }

    #[test]
    fn feed_pauses_old_track_when_track_loaded() {
        let directory = tempfile::tempdir().unwrap();
        let harness = Harness::start(RATE);
        let old = harness.load(
            1,
            track(directory.path(), RATE, &[Some(&pcm_ramp(0, 9_000))]),
        );
        harness.play(old.id);
        let mut consumer = harness.next_stream();
        harness.wait_for(old.id, PlayerState::Playing);

        harness.load(2, track(directory.path(), RATE, &[Some(&pcm_ramp(0, 50))]));
        harness.drain_until_flushed(&mut consumer, 1);

        harness.wait_for(old.id, PlayerState::Paused);
        assert_eq!(old.atomics.state(), PlayerState::Paused);
    }
}
