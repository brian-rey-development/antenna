use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU8, AtomicU32, AtomicU64, Ordering};

use antenna_core::{SampleRate, SegmentIndex};

use super::PlayerState;
use crate::{TrackPosition, Volume};

const UNSCHEDULED: i64 = i64::MAX;
const STATES: [PlayerState; 4] = [
    PlayerState::Paused,
    PlayerState::Playing,
    PlayerState::Buffering,
    PlayerState::Ended,
];

#[derive(Debug)]
pub(crate) struct PlayerAtomics {
    pub(crate) played_frames: AtomicU64,
    pub(crate) flush_epoch: AtomicU64,
    pub(crate) flushed_epoch: AtomicU64,
    pub(crate) flushed_at_frame: AtomicU64,
    pub(crate) paused: AtomicBool,
    pub(crate) is_buffering: AtomicBool,
    pub(crate) device_rate_hz: AtomicU32,
    pub(crate) underruns: AtomicU64,
    pub(crate) volume_bits: AtomicU32,
}

impl PlayerAtomics {
    pub(crate) fn new() -> Self {
        Self {
            played_frames: AtomicU64::new(0),
            flush_epoch: AtomicU64::new(0),
            flushed_epoch: AtomicU64::new(0),
            flushed_at_frame: AtomicU64::new(0),
            paused: AtomicBool::new(true),
            is_buffering: AtomicBool::new(false),
            device_rate_hz: AtomicU32::new(0),
            underruns: AtomicU64::new(0),
            volume_bits: AtomicU32::new(Volume::FULL.to_bits()),
        }
    }

    pub(crate) fn device_rate(&self) -> Option<SampleRate> {
        SampleRate::try_from(self.device_rate_hz.load(Ordering::Relaxed)).ok()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Located {
    pub(crate) segment: usize,
    pub(crate) offset_frames: u64,
}

#[derive(Debug)]
pub(crate) struct TrackAtomics {
    starts: Box<[AtomicI64]>,
    first_scheduled: AtomicU32,
    state: AtomicU8,
}

impl TrackAtomics {
    pub(crate) fn new(segments: usize) -> Self {
        Self {
            starts: (0..segments).map(|_| AtomicI64::new(UNSCHEDULED)).collect(),
            first_scheduled: AtomicU32::new(0),
            state: AtomicU8::new(PlayerState::Paused as u8),
        }
    }

    pub(crate) fn restart(&self, first_scheduled: usize) {
        for start in &self.starts {
            start.store(UNSCHEDULED, Ordering::Relaxed);
        }
        let first = u32::try_from(first_scheduled).unwrap_or(u32::MAX);
        self.first_scheduled.store(first, Ordering::Relaxed);
    }

    pub(crate) fn schedule(&self, segment: usize, frame: i64) {
        if let Some(start) = self.starts.get(segment) {
            start.store(frame, Ordering::Relaxed);
        }
    }

    pub(crate) fn state(&self) -> PlayerState {
        let raw = self.state.load(Ordering::Relaxed);
        STATES
            .into_iter()
            .find(|state| *state as u8 == raw)
            .unwrap_or(PlayerState::Paused)
    }

    pub(crate) fn set_state(&self, state: PlayerState) {
        self.state.store(state as u8, Ordering::Relaxed);
    }

    pub(crate) fn locate(&self, played_frames: u64) -> Option<Located> {
        let first = self.first_scheduled.load(Ordering::Relaxed) as usize;
        let scheduled = self.starts.get(first..)?;
        let played = i64::try_from(played_frames).unwrap_or(i64::MAX);
        let started = scheduled.partition_point(|start| start.load(Ordering::Relaxed) <= played);
        let current = scheduled.get(started.checked_sub(1)?)?;
        Some(Located {
            segment: first + started - 1,
            offset_frames: u64::try_from(played.checked_sub(current.load(Ordering::Relaxed))?)
                .ok()?,
        })
    }

    pub(crate) fn position(&self, player: &PlayerAtomics) -> Option<TrackPosition> {
        let located = self.locate(player.played_frames.load(Ordering::Acquire))?;
        let rate = player.device_rate()?;
        Some(TrackPosition {
            segment: SegmentIndex::new(u32::try_from(located.segment).ok()?),
            offset: rate.duration_of(located.offset_frames),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    const RATE_HZ: u32 = 1_000;

    fn device() -> PlayerAtomics {
        let atomics = PlayerAtomics::new();
        atomics.device_rate_hz.store(RATE_HZ, Ordering::Relaxed);
        atomics
    }

    fn position_at(
        track: &TrackAtomics,
        device: &PlayerAtomics,
        frames: u64,
    ) -> Option<(u32, Duration)> {
        device.played_frames.store(frames, Ordering::Release);
        track
            .position(device)
            .map(|at| (at.segment.get(), at.offset))
    }

    #[test]
    fn position_tracks_played_frames() {
        let (track, player) = (TrackAtomics::new(3), device());
        track.schedule(0, 0);
        track.schedule(1, 500);
        track.schedule(2, 1_500);

        assert_eq!(position_at(&track, &player, 0), Some((0, Duration::ZERO)));
        assert_eq!(
            position_at(&track, &player, 250),
            Some((0, Duration::from_millis(250)))
        );
        assert_eq!(position_at(&track, &player, 500), Some((1, Duration::ZERO)));
        assert_eq!(
            position_at(&track, &player, 1_499),
            Some((1, Duration::from_millis(999)))
        );
        assert_eq!(
            position_at(&track, &player, 9_000),
            Some((2, Duration::from_millis(7_500)))
        );
    }

    #[test]
    fn position_is_none_when_no_segment_started() {
        let (track, player) = (TrackAtomics::new(2), device());
        track.schedule(0, 100);

        assert_eq!(position_at(&track, &player, 99), None);
        assert_eq!(position_at(&TrackAtomics::new(0), &player, 5), None);
    }

    #[test]
    fn position_restarts_at_seek_target() {
        let (track, player) = (TrackAtomics::new(3), device());
        track.schedule(0, 0);
        track.schedule(1, 500);
        track.restart(2);
        track.schedule(2, 1_000);

        assert_eq!(
            position_at(&track, &player, 1_000),
            Some((2, Duration::ZERO))
        );
        assert_eq!(
            position_at(&track, &player, 1_250),
            Some((2, Duration::from_millis(250)))
        );
    }

    #[test]
    fn position_is_correct_when_seek_offset_exceeds_played_frames() {
        let (track, player) = (TrackAtomics::new(2), device());
        track.restart(1);
        track.schedule(1, 100 - 700);

        assert_eq!(
            position_at(&track, &player, 100),
            Some((1, Duration::from_millis(700)))
        );
        assert_eq!(
            position_at(&track, &player, 150),
            Some((1, Duration::from_millis(750)))
        );
    }

    #[test]
    fn position_is_none_when_device_rate_unknown() {
        let (track, player) = (TrackAtomics::new(1), PlayerAtomics::new());
        track.schedule(0, 0);

        assert_eq!(position_at(&track, &player, 10), None);
    }

    #[test]
    fn track_state_round_trips_when_stored() {
        let track = TrackAtomics::new(0);

        track.set_state(PlayerState::Buffering);

        assert_eq!(track.state(), PlayerState::Buffering);
    }
}
