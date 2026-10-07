use std::path::PathBuf;
use std::time::Duration;

use antenna_core::{SampleRate, SegmentIndex};

/// The segments of a document that the player plays, at the sample rate of the engine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Track {
    rate: SampleRate,
    segments: Vec<TrackSegment>,
}

/// A segment of a track. A pending segment has no audio yet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum TrackSegment {
    Stored { path: PathBuf, duration: Duration },
    Pending,
}

impl Track {
    /// Makes a track with a stored segment for each `Some` entry and a pending segment for each
    /// `None` entry.
    pub fn new(
        rate: SampleRate,
        segments: impl IntoIterator<Item = Option<(PathBuf, Duration)>>,
    ) -> Self {
        let segments = segments
            .into_iter()
            .map(|entry| match entry {
                Some((path, duration)) => TrackSegment::Stored { path, duration },
                None => TrackSegment::Pending,
            })
            .collect();
        Self { rate, segments }
    }

    pub(crate) fn segment_count(&self) -> usize {
        self.segments.len()
    }

    pub(crate) fn rate(&self) -> SampleRate {
        self.rate
    }

    pub(crate) fn into_parts(self) -> (SampleRate, Vec<TrackSegment>) {
        (self.rate, self.segments)
    }
}

/// The segment and the time in the segment that the player plays now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrackPosition {
    /// The segment that plays.
    pub segment: SegmentIndex,
    /// The time from the start of the segment.
    pub offset: Duration,
}

/// The identifier that the player gives to each loaded track.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TrackId(u64);

impl TrackId {
    pub(crate) const fn new(id: u64) -> Self {
        Self(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn track_marks_segment_pending_when_entry_none() {
        let stored = (PathBuf::from("a.wav"), Duration::from_secs(1));

        let track = Track::new(SampleRate::HZ_24000, [Some(stored.clone()), None]);

        let (rate, segments) = track.into_parts();
        let expected = TrackSegment::Stored {
            path: stored.0,
            duration: stored.1,
        };
        assert_eq!(rate, SampleRate::HZ_24000);
        assert_eq!(segments, [expected, TrackSegment::Pending]);
    }
}
