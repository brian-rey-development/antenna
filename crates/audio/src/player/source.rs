use std::mem;
use std::path::PathBuf;
use std::time::Duration;

use crate::segment_file::read_segment;
use crate::track::TrackSegment;

/// The audio of one segment of the track, as the feed thread holds it.
#[derive(Debug)]
pub(crate) enum SegmentSource {
    Pending,
    Stored(StoredSegment),
    Live(Vec<f32>),
    LiveComplete(Vec<f32>),
}

#[derive(Debug)]
pub(crate) struct StoredSegment {
    path: PathBuf,
    duration: Duration,
    samples: Option<Vec<f32>>,
}

impl SegmentSource {
    pub(crate) fn from_file(path: PathBuf, duration: Duration) -> Self {
        Self::Stored(StoredSegment {
            path,
            duration,
            samples: None,
        })
    }

    pub(crate) fn duration(&self) -> Option<Duration> {
        match self {
            Self::Stored(stored) => Some(stored.duration),
            Self::Pending | Self::Live(_) | Self::LiveComplete(_) => None,
        }
    }

    pub(crate) fn is_stored(&self) -> bool {
        matches!(self, Self::Stored(_))
    }

    /// A stored segment is complete when its file is loaded. A segment that cannot be read stays
    /// incomplete, so the player waits and does not skip it.
    pub(crate) fn is_complete(&self) -> bool {
        match self {
            Self::Stored(stored) => stored.samples.is_some(),
            Self::LiveComplete(_) => true,
            Self::Pending | Self::Live(_) => false,
        }
    }

    pub(crate) fn samples(&mut self) -> &[f32] {
        match self {
            Self::Pending => &[],
            Self::Stored(stored) => stored.load(),
            Self::Live(samples) | Self::LiveComplete(samples) => samples,
        }
    }

    pub(crate) fn extend_live(&mut self, chunk: &[f32]) {
        if let Self::Live(samples) = self {
            samples.extend_from_slice(chunk);
        }
    }

    pub(crate) fn complete_live(&mut self) {
        if let Self::Live(samples) = self {
            *self = Self::LiveComplete(mem::take(samples));
        }
    }

    pub(crate) fn release(&mut self) {
        if let Self::Stored(stored) = self {
            stored.samples = None;
        }
    }
}

impl From<TrackSegment> for SegmentSource {
    fn from(segment: TrackSegment) -> Self {
        match segment {
            TrackSegment::Stored { path, duration } => Self::from_file(path, duration),
            TrackSegment::Pending => Self::Pending,
        }
    }
}

impl StoredSegment {
    fn load(&mut self) -> &[f32] {
        if self.samples.is_none() {
            self.samples = read_segment(&self.path)
                .inspect_err(|error| {
                    tracing::error!(path = %self.path.display(), ?error, "cannot read the segment");
                })
                .ok();
        }
        self.samples.as_deref().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use antenna_core::SampleRate;

    use super::*;
    use crate::test_support::write_segment;

    fn stored_source(samples: &[f32]) -> (tempfile::TempDir, SegmentSource) {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("a.wav");
        write_segment(&path, SampleRate::HZ_24000, samples).unwrap();
        let source = SegmentSource::from_file(path, Duration::from_secs(1));
        (directory, source)
    }

    #[test]
    fn source_loads_samples_when_stored() {
        let (_directory, mut source) = stored_source(&[1.0, 0.0]);

        let samples = source.samples().to_vec();

        assert_eq!(samples, [1.0, 0.0]);
        assert!(source.is_complete());
    }

    #[test]
    fn source_has_duration_when_stored() {
        let (_directory, source) = stored_source(&[0.0]);

        assert_eq!(source.duration(), Some(Duration::from_secs(1)));
    }

    #[test]
    fn source_waits_when_stored_file_missing() {
        let directory = tempfile::tempdir().unwrap();
        let mut source =
            SegmentSource::from_file(directory.path().join("none.wav"), Duration::ZERO);

        assert!(source.samples().is_empty());
        assert!(!source.is_complete());
    }

    #[test]
    fn source_loads_file_when_it_appears_after_failed_read() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("late.wav");
        let mut source = SegmentSource::from_file(path.clone(), Duration::ZERO);
        assert!(source.samples().is_empty());
        write_segment(&path, SampleRate::HZ_24000, &[1.0]).unwrap();

        let samples = source.samples().to_vec();

        assert_eq!(samples, [1.0]);
    }

    #[test]
    fn source_reloads_samples_when_released() {
        let (_directory, mut source) = stored_source(&[0.0]);
        source.samples();
        source.release();

        let samples = source.samples().to_vec();

        assert_eq!(samples, [0.0]);
    }

    #[test]
    fn source_grows_until_complete_when_live() {
        let mut source = SegmentSource::Live(Vec::new());

        source.extend_live(&[0.1]);
        source.extend_live(&[0.2]);

        assert_eq!(source.samples(), [0.1, 0.2]);
        assert!(!source.is_complete());
    }

    #[test]
    fn source_ignores_samples_when_live_complete() {
        let mut source = SegmentSource::Live(vec![0.1]);
        source.complete_live();

        source.extend_live(&[0.3]);

        assert!(source.is_complete());
        assert_eq!(source.samples(), [0.1]);
    }

    #[test]
    fn source_waits_when_pending() {
        let mut source = SegmentSource::from(TrackSegment::Pending);

        assert!(source.samples().is_empty());
        assert!(!source.is_complete());
        assert!(!source.is_stored());
    }
}
