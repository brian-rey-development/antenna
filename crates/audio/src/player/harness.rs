use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use antenna_core::{SampleRate, SegmentIndex};
use flume::{Receiver, Sender};
use rtrb::{Consumer, RingBuffer};

use super::atomics::{PlayerAtomics, TrackAtomics};
use super::callback::fill;
use super::command::FeedCommand;
use super::feed::Feed;
use super::stream::StreamParts;
use super::{PlayerEvent, PlayerState};
use crate::test_support::{pcm_ramp, write_segment};
use crate::{AudioError, Track, TrackId, TrackPosition};

pub(crate) const RATE: SampleRate = SampleRate::HZ_24000;
const RING_FRAMES: usize = 1 << 16;
const DRAIN_FRAMES: usize = 256;
const MONO: NonZeroUsize = NonZeroUsize::MIN;
const MAX_DRAINS: usize = 2_000_000;
const EVENT_TIMEOUT: Duration = Duration::from_secs(10);
const ANSWER_LIMIT: Duration = Duration::from_secs(30);

pub(crate) struct Harness {
    pub(crate) commands: Sender<FeedCommand>,
    pub(crate) events: Receiver<PlayerEvent>,
    pub(crate) event_sender: Sender<PlayerEvent>,
    pub(crate) atomics: Arc<PlayerAtomics>,
    pub(crate) opens: Arc<AtomicUsize>,
    streams: Receiver<Consumer<f32>>,
    thread: Option<JoinHandle<()>>,
}

pub(crate) struct TestTrack {
    pub(crate) id: TrackId,
    pub(crate) atomics: Arc<TrackAtomics>,
}

type Open = Box<dyn FnMut() -> Result<StreamParts<()>, AudioError> + Send>;

impl Harness {
    pub(crate) fn start(device: SampleRate) -> Self {
        Self::start_with_flush_wait(device, ANSWER_LIMIT)
    }

    pub(crate) fn start_with_flush_wait(device: SampleRate, flush_wait: Duration) -> Self {
        let (stream_sender, streams) = flume::unbounded();
        let opens = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&opens);
        let open: Open = Box::new(move || {
            counter.fetch_add(1, Ordering::Relaxed);
            let (producer, consumer) = RingBuffer::new(RING_FRAMES);
            stream_sender.send(consumer).unwrap();
            Ok(StreamParts {
                stream: (),
                rate: device,
                producer,
            })
        });
        Self::spawn(open, opens, streams, flush_wait)
    }

    pub(crate) fn start_without_device() -> Self {
        let (_, streams) = flume::unbounded();
        let open: Open = Box::new(|| Err(AudioError::NoDevice));
        Self::spawn(open, Arc::new(AtomicUsize::new(0)), streams, ANSWER_LIMIT)
    }

    fn spawn(
        open: Open,
        opens: Arc<AtomicUsize>,
        streams: Receiver<Consumer<f32>>,
        flush_wait: Duration,
    ) -> Self {
        let (commands, receiver) = flume::unbounded();
        let (event_sender, events) = flume::unbounded();
        let atomics = Arc::new(PlayerAtomics::new());
        let feed_events = event_sender.clone();
        let feed_atomics = Arc::clone(&atomics);
        let thread = thread::Builder::new()
            .name("antenna-test-feed".to_owned())
            .spawn(move || Feed::new(receiver, feed_events, feed_atomics, flush_wait, open).run())
            .unwrap();
        Self {
            commands,
            events,
            event_sender,
            atomics,
            opens,
            streams,
            thread: Some(thread),
        }
    }

    pub(crate) fn send(&self, command: FeedCommand) {
        self.commands.send(command).unwrap();
    }

    pub(crate) fn load(&self, id: u64, track: Track) -> TestTrack {
        let id = TrackId::new(id);
        let atomics = Arc::new(TrackAtomics::new(track.segment_count()));
        self.send(FeedCommand::Load {
            id,
            track,
            atomics: Arc::clone(&atomics),
        });
        TestTrack { id, atomics }
    }

    pub(crate) fn play(&self, id: TrackId) {
        self.send(FeedCommand::Play { id });
    }

    pub(crate) fn pause(&self, id: TrackId) {
        self.send(FeedCommand::Pause { id });
    }

    pub(crate) fn seek(&self, id: TrackId, segment: u32, offset: Duration) {
        let position = TrackPosition {
            segment: SegmentIndex::new(segment),
            offset,
        };
        self.send(FeedCommand::Seek { id, position });
    }

    pub(crate) fn live(&self, id: TrackId, segment: u32, samples: &[f32]) {
        let segment = SegmentIndex::new(segment);
        self.send(FeedCommand::LiveBegin { id, segment });
        self.send(FeedCommand::LiveChunk {
            id,
            samples: samples.to_vec(),
        });
    }

    pub(crate) fn finish_live(&self, id: TrackId) {
        self.send(FeedCommand::LiveEnd { id });
    }

    pub(crate) fn store(&self, id: TrackId, segment: u32, file: (PathBuf, Duration)) {
        let (path, duration) = file;
        let segment = SegmentIndex::new(segment);
        self.send(FeedCommand::Stored {
            id,
            segment,
            path,
            duration,
        });
    }

    pub(crate) fn next_stream(&self) -> Consumer<f32> {
        self.streams.recv_timeout(EVENT_TIMEOUT).unwrap()
    }

    pub(crate) fn wait_for(&self, track: TrackId, state: PlayerState) -> Vec<PlayerEvent> {
        let wanted = PlayerEvent::StateChanged { track, state };
        let mut seen = Vec::new();
        while seen.last() != Some(&wanted) {
            seen.push(self.events.recv_timeout(EVENT_TIMEOUT).unwrap());
        }
        seen
    }

    pub(crate) fn drain_until(
        &self,
        consumer: &mut Consumer<f32>,
        track: TrackId,
        state: PlayerState,
    ) -> Vec<f32> {
        let wanted = PlayerEvent::StateChanged { track, state };
        let mut samples = Vec::new();
        for _ in 0..MAX_DRAINS {
            samples.extend(self.drain(consumer));
            if self.events.try_iter().any(|event| event == wanted) {
                return samples;
            }
            thread::yield_now();
        }
        panic!("the track did not reach {state:?}");
    }

    pub(crate) fn drain_samples(&self, consumer: &mut Consumer<f32>, count: usize) -> Vec<f32> {
        let mut samples = Vec::new();
        for _ in 0..MAX_DRAINS {
            samples.extend(self.drain(consumer));
            if samples.len() >= count {
                return samples;
            }
            thread::yield_now();
        }
        panic!("the ring buffer did not deliver {count} samples");
    }

    pub(crate) fn next_event(&self) -> PlayerEvent {
        self.events.recv_timeout(EVENT_TIMEOUT).unwrap()
    }

    /// Plays a track of two stored segments, and returns it with the consumer of its stream.
    pub(crate) fn play_two_segments(&self, directory: &Path) -> (TestTrack, Consumer<f32>) {
        let (first, second) = (pcm_ramp(0, 6_000), pcm_ramp(6_000, 6_000));
        let loaded = self.load(1, track(directory, RATE, &[Some(&first), Some(&second)]));
        self.play(loaded.id);
        (loaded, self.next_stream())
    }

    pub(crate) fn wait_for_flush_request(&self, epoch: u64) {
        for _ in 0..MAX_DRAINS {
            if self.atomics.flush_epoch.load(Ordering::Acquire) >= epoch {
                return;
            }
            thread::yield_now();
        }
        panic!("the feed thread did not request the flush {epoch}");
    }

    pub(crate) fn drain_until_flushed(&self, consumer: &mut Consumer<f32>, epoch: u64) {
        for _ in 0..MAX_DRAINS {
            if self.atomics.flushed_epoch.load(Ordering::Acquire) >= epoch {
                return;
            }
            self.drain(consumer);
            thread::yield_now();
        }
        panic!("the callback did not answer the flush {epoch}");
    }

    /// Returns after the feed thread applied all commands that the test sent before.
    pub(crate) fn barrier(&self, consumer: &mut Consumer<f32>, id: TrackId) {
        let epoch = self.atomics.flush_epoch.load(Ordering::Acquire) + 1;
        self.seek(id, 0, Duration::ZERO);
        self.drain_until_flushed(consumer, epoch);
    }

    pub(crate) fn drain(&self, consumer: &mut Consumer<f32>) -> Vec<f32> {
        let before = self.atomics.played_frames.load(Ordering::Acquire);
        let mut buffer = vec![0.0; DRAIN_FRAMES];
        fill(&mut buffer, MONO, consumer, &self.atomics);
        let popped = self.atomics.played_frames.load(Ordering::Acquire) - before;
        buffer.truncate(usize::try_from(popped).unwrap());
        buffer
    }
}

impl Drop for Harness {
    fn drop(&mut self) {
        self.commands.send(FeedCommand::Shutdown).unwrap();
        if let Some(thread) = self.thread.take() {
            thread.join().unwrap();
        }
    }
}

pub(crate) fn track(directory: &Path, rate: SampleRate, segments: &[Option<&[f32]>]) -> Track {
    let entries = segments.iter().enumerate().map(|(index, samples)| {
        let samples = (*samples)?;
        let path = directory.join(format!("segment-{index}.wav"));
        write_segment(&path, rate, samples).unwrap();
        Some((path, rate.duration_of(samples.len() as u64)))
    });
    Track::new(rate, entries)
}

pub(crate) fn stored(
    directory: &Path,
    name: &str,
    rate: SampleRate,
    samples: &[f32],
) -> (PathBuf, Duration) {
    let path = directory.join(name);
    write_segment(&path, rate, samples).unwrap();
    (path, rate.duration_of(samples.len() as u64))
}
