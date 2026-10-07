use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use antenna_core::SampleRate;

use super::PlayerState;
use super::atomics::TrackAtomics;
use super::source::SegmentSource;
use crate::frames::{rescale, sample_count};
use crate::resample::RESAMPLER_CHUNK_FRAMES;
use crate::{Track, TrackId, TrackPosition};

#[derive(Clone, Copy, Debug)]
pub(crate) struct Clock {
    base_frame: i64,
    device: SampleRate,
}

impl Clock {
    pub(crate) fn new(base_frame: u64, device: SampleRate) -> Self {
        Self {
            base_frame: i64::try_from(base_frame).unwrap_or(i64::MAX),
            device,
        }
    }
}

#[derive(Debug)]
struct Cursor {
    segment: usize,
    offset: usize,
    is_entered: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Block {
    Ready,
    Waiting,
    Done,
}

#[derive(Debug)]
pub(crate) struct CurrentTrack {
    id: TrackId,
    atomics: Arc<TrackAtomics>,
    rate: SampleRate,
    sources: Vec<SegmentSource>,
    cursor: Cursor,
    clock: Clock,
    fed: i64,
    live: Option<usize>,
    pub(crate) state: PlayerState,
}

impl CurrentTrack {
    pub(crate) fn new(id: TrackId, track: Track, atomics: Arc<TrackAtomics>, clock: Clock) -> Self {
        let (rate, segments) = track.into_parts();
        Self {
            id,
            atomics,
            rate,
            sources: segments.into_iter().map(SegmentSource::from).collect(),
            cursor: Cursor {
                segment: 0,
                offset: 0,
                is_entered: false,
            },
            clock,
            fed: 0,
            live: None,
            state: PlayerState::Paused,
        }
    }

    pub(crate) fn id(&self) -> TrackId {
        self.id
    }

    pub(crate) fn rate(&self) -> SampleRate {
        self.rate
    }

    pub(crate) fn atomics(&self) -> &TrackAtomics {
        &self.atomics
    }

    pub(crate) fn restart(&mut self, segment: usize, offset: usize, clock: Clock) {
        self.sources.iter_mut().for_each(SegmentSource::release);
        let segment = segment.min(self.sources.len());
        self.cursor = Cursor {
            segment,
            offset,
            is_entered: true,
        };
        self.clock = clock;
        self.fed = 0;
        self.atomics.restart(segment);
        self.mark_start();
    }

    pub(crate) fn target(&self, position: TrackPosition) -> (usize, usize) {
        let segment = usize::from(position.segment);
        let offset = match self.sources.get(segment).and_then(SegmentSource::duration) {
            Some(duration) => position.offset.min(duration),
            None => position.offset,
        };
        (segment, sample_count(offset, self.rate))
    }

    pub(crate) fn resume_point(&self, played_frames: u64, device: SampleRate) -> (usize, usize) {
        let Some(located) = self.atomics.locate(played_frames) else {
            return (self.cursor.segment, self.cursor.offset);
        };
        let frames = i64::try_from(located.offset_frames).unwrap_or(i64::MAX);
        let offset = usize::try_from(rescale(frames, device, self.rate)).unwrap_or(0);
        (located.segment, offset)
    }

    pub(crate) fn set_stored(&mut self, segment: usize, path: PathBuf, duration: Duration) {
        if let Some(source) = self.sources.get_mut(segment) {
            *source = SegmentSource::from_file(path, duration);
        }
        if self.live == Some(segment) {
            self.live = None;
        }
    }

    pub(crate) fn live_begin(&mut self, segment: usize) {
        self.live_end();
        let Some(source) = self.sources.get_mut(segment) else {
            return;
        };
        if !source.is_stored() {
            *source = SegmentSource::Live(Vec::new());
            self.live = Some(segment);
        }
    }

    pub(crate) fn live_chunk(&mut self, samples: &[f32]) {
        let live = self.live.and_then(|index| self.sources.get_mut(index));
        if let Some(source) = live {
            source.extend_live(samples);
        }
    }

    pub(crate) fn live_end(&mut self) {
        let live = self
            .live
            .take()
            .and_then(|index| self.sources.get_mut(index));
        if let Some(source) = live {
            source.complete_live();
        }
    }

    pub(crate) fn next_block(&mut self, output: &mut Vec<f32>) -> Block {
        while self.cursor.segment < self.sources.len() {
            if !self.cursor.is_entered {
                self.cursor.is_entered = true;
                self.mark_start();
            }
            match self.read(output) {
                Some(block) => return block,
                None => self.advance(),
            }
        }
        Block::Done
    }

    fn read(&mut self, output: &mut Vec<f32>) -> Option<Block> {
        let source = self.sources.get_mut(self.cursor.segment)?;
        let samples = source
            .samples()
            .get(self.cursor.offset..)
            .unwrap_or_default();
        let block = samples.len().min(RESAMPLER_CHUNK_FRAMES);
        if block > 0 {
            output.extend(samples.iter().take(block));
            self.cursor.offset += block;
            self.fed += signed(block);
            Some(Block::Ready)
        } else if source.is_complete() {
            None
        } else {
            Some(Block::Waiting)
        }
    }

    fn advance(&mut self) {
        if let Some(source) = self.sources.get_mut(self.cursor.segment) {
            source.release();
        }
        self.cursor = Cursor {
            segment: self.cursor.segment + 1,
            offset: 0,
            is_entered: false,
        };
    }

    fn mark_start(&self) {
        // The samples fed since the restart, minus the offset in the segment, are the samples
        // between the start of the segment and the restart point. They can be negative.
        let samples = self.fed - signed(self.cursor.offset);
        let frame = self.clock.base_frame + rescale(samples, self.rate, self.clock.device);
        self.atomics.schedule(self.cursor.segment, frame);
    }
}

fn signed(count: usize) -> i64 {
    i64::try_from(count).unwrap_or(i64::MAX)
}
