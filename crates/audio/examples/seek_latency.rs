//! Measures the time from `seek` to the first sample of the target at the output device.
//!
//! The example plays a silent track of three stored segments on the default device. It seeks 20
//! times to stored positions. For each seek, it waits until the play position passes the target,
//! which needs a callback that plays a sample of the target. It prints the median and the maximum
//! time.

#[path = "../tests/fixtures/mod.rs"]
mod fixtures;

use std::error::Error;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

use antenna_audio::{AudioError, Player, PlayerEvent, Track, TrackHandle, TrackPosition, Volume};
use antenna_core::{SampleRate, SegmentIndex};
use flume::RecvTimeoutError;
use tempfile::TempDir;

const RATE: SampleRate = SampleRate::HZ_24000;
const SEGMENTS: u32 = 3;
const SEGMENT_SECONDS: u64 = 30;
const SEGMENT_MILLIS: u64 = SEGMENT_SECONDS * 1_000;
const SEEK_BUDGET: Duration = Duration::from_millis(100);
const SEEKS: usize = 20;
const SEEK_TIMEOUT: Duration = Duration::from_secs(5);
const DEVICE_WAIT: Duration = Duration::from_secs(2);
const SINE_HZ: f32 = 220.0;
const AMPLITUDE: f32 = 0.1;
const XORSHIFT_SEED: u64 = 0x9E37_79B9_7F4A_7C15;
const MARGIN_MS: u64 = 2_000;

struct Random(u64);

impl Random {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

type Entries = Vec<Option<(PathBuf, Duration)>>;

fn write_segments() -> Result<(TempDir, Entries), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let frames = usize::try_from(SEGMENT_SECONDS * u64::from(RATE.hz()))?;
    let samples = fixtures::sine(SINE_HZ, RATE, frames, AMPLITUDE);
    let mut entries = Vec::new();
    for index in 0..SEGMENTS {
        let path = directory.path().join(format!("segment-{index}.wav"));
        fixtures::write_segment(&path, RATE, &samples)?;
        entries.push(Some((path, Duration::from_secs(SEGMENT_SECONDS))));
    }
    Ok((directory, entries))
}

fn wait_for_device(player: &Player) -> Result<(), AudioError> {
    match player.events().recv_timeout(DEVICE_WAIT) {
        Ok(PlayerEvent::DeviceLost) => Err(AudioError::NoDevice),
        Ok(_) | Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => Ok(()),
    }
}

fn time_seek(handle: &TrackHandle, target: TrackPosition) -> Option<Duration> {
    let start = Instant::now();
    handle.seek(target);
    while start.elapsed() < SEEK_TIMEOUT {
        let position = handle.position();
        if position.is_some_and(|at| at.segment == target.segment && at.offset > target.offset) {
            return Some(start.elapsed());
        }
        thread::yield_now();
    }
    None
}

#[expect(clippy::print_stdout, reason = "the example prints its measurement")]
fn report(times: &[Duration], timeouts: usize) {
    let mut sorted = times.to_vec();
    sorted.sort();
    let median = sorted.get(sorted.len() / 2).copied().unwrap_or_default();
    let maximum = sorted.last().copied().unwrap_or_default();
    println!(
        "{} seeks, {timeouts} timeouts: median {median:?}, maximum {maximum:?}, budget {SEEK_BUDGET:?}",
        sorted.len()
    );
}

struct Measurement {
    times: Vec<Duration>,
    timeouts: usize,
}

fn measure_seeks(handle: &TrackHandle) -> Result<Measurement, Box<dyn Error>> {
    let (mut random, mut segment) = (Random(XORSHIFT_SEED), 0);
    let (mut times, mut timeouts) = (Vec::new(), 0);
    for _ in 0..SEEKS {
        let step = u32::try_from(random.next() % u64::from(SEGMENTS - 1))?;
        segment = (segment + 1 + step) % SEGMENTS;
        let offset = Duration::from_millis(random.next() % (SEGMENT_MILLIS - MARGIN_MS));
        let target = TrackPosition {
            segment: SegmentIndex::new(segment),
            offset,
        };
        match time_seek(handle, target) {
            Some(time) => times.push(time),
            None => timeouts += 1,
        }
    }
    Ok(Measurement { times, timeouts })
}

fn main() -> Result<(), Box<dyn Error>> {
    let (_directory, entries) = write_segments()?;
    let player = Player::start()?;
    player.set_volume(Volume::new(0.0));
    let handle = player.load(Track::new(RATE, entries));
    handle.play();
    wait_for_device(&player)?;

    let Measurement { times, timeouts } = measure_seeks(&handle)?;
    report(&times, timeouts);
    if timeouts > 0 || times.iter().any(|time| *time > SEEK_BUDGET) {
        return Err("a seek missed the budget".into());
    }
    Ok(())
}
