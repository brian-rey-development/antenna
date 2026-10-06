# Stage 03. Audio

## Goal

`antenna-audio` plays tracks of stored, live and pending segments without gaps or callback allocations. It exports stored segments to MP3, WAV and Ogg Opus with EBU R128 normalization.

## Context

This crate contains all audio input and output of Antenna. The player is the only consumer of the audio device. The export is the only code that converts the internal format into a file format.

The player follows `docs/architecture.md` sections 6.3 and 6.4. Generation and playback are separate. Thus the player reads stored segments from files and receives the chunks of a running job through `LiveOutput`. The CLI (stage 07), `antenna-review` (stage 08) and the desktop app (stage 16) use this crate. The pipeline does not depend on it. It receives a `LiveOutput` as a `Box<dyn Output>`.

This crate does not depend on `antenna-library`. It receives file paths and durations. The apps connect the library to the player.

A track has two kinds of segments, `Stored` and `Pending`. A live segment is a state of the feed thread. It exists while a `LiveOutput` sends the chunks of a pending segment, until `set_stored` replaces it.

## Read first

1. `CLAUDE.md`
2. `docs/architecture.md` sections 2, 5.4, 6.3, 6.4, 6.5, 6.7, 8 and 13
3. `docs/standards.md` sections 5, 6, 7, 8, 14, 18 and 19
4. The `cpal` 0.18 documentation of `Device`, `StreamConfig`, `build_output_stream` and `ErrorKind`
5. The `rtrb` 0.4 documentation of `RingBuffer`, `Producer`, `Consumer` and `Consumer::read_chunk`
6. The `ebur128` documentation of `EbuR128`, `Mode::I` and `Mode::TRUE_PEAK`
7. RFC 7845 sections 3 and 4 (Ogg Opus pre-skip and granule position)

## Scope

The stage can create or change these files only.

- `crates/audio/`
- `Cargo.toml` (root), to add the member and the workspace dependencies `cpal`, `rtrb`, `rubato`, `ebur128`, `mp3lame-encoder`, `hound`, `opus`, `ogg`, `tracing` and `divan`
- `Cargo.toml` (root), to add the dev-dependencies `symphonia`, `assert_no_alloc` and `tempfile`
- `.github/workflows/ci.yml`, to install `libasound2-dev` on the Linux job, and CMake on a job that does not have CMake 3.16 or later

## Deliverables

### Crate layout

```
crates/audio/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── error.rs            # AudioError
│   ├── track.rs            # Track, TrackPosition
│   ├── player/
│   │   ├── mod.rs          # Player, TrackHandle, PlayerState, PlayerEvent
│   │   ├── atomics.rs      # PlayerAtomics and TrackAtomics: the atomics of the threads
│   │   ├── callback.rs     # fill: the realtime function
│   │   ├── feed.rs         # the feed thread loop and wait_mode
│   │   ├── source.rs       # SegmentSource: read samples from a stored, live or pending segment
│   │   ├── command.rs      # FeedCommand
│   │   └── device.rs       # open the default device, build the stream, handle stream errors
│   ├── live.rs             # LiveOutput, LiveSink
│   ├── resample.rs         # Conversion: direct copy or rubato FFT resampler
│   ├── segment_file.rs     # read a 16-bit stored segment into f32 samples with PCM_SCALE
│   ├── peaks.rs            # waveform peaks of a stored segment, PeakCache
│   └── export/
│       ├── mod.rs          # ExportSpec, export
│       ├── loudness.rs     # measure and gain
│       ├── mp3.rs
│       ├── wav.rs
│       ├── ogg.rs          # Opus encoder and Ogg pages
│       └── part_file.rs    # PartFile: write to "<path>.part", rename on commit
├── benches/
│   └── export.rs           # divan benchmark of a 10-minute export
├── examples/
│   └── seek_latency.rs     # measures the seek time on the real device
└── tests/
    ├── export.rs           # file round trips and loudness
    └── device.rs           # tests on a real device through the public API, ignored by default
```

`fill`, `PlayerAtomics`, the feed loop with its `open_stream` closure and `device::on_stream_error` are crate-private. Thus their tests are unit tests in the `#[cfg(test)] mod tests` block of `callback.rs`, `feed.rs` and `device.rs`. `lib.rs` installs the allocator of `assert_no_alloc` for the unit tests of the crate with `#[cfg(test)] #[global_allocator] static ALLOCATOR: assert_no_alloc::AllocDisabler = assert_no_alloc::AllocDisabler;`. The allocator checks only the code inside `assert_no_alloc`, so the other unit tests can allocate. The integration tests in `tests/` use only the public API.

### Constants

```rust
pub const PEAKS_PER_SECOND: u32 = 50;
const RING_BUFFER_DURATION_MS: u64 = 2_000;
const FEED_TARGET_MS: u64 = 300;
const FEED_POLL: Duration = Duration::from_millis(10);
const PREFILL_DURATION_MS: u64 = 50;
const RESAMPLER_CHUNK_FRAMES: usize = 1_024;
const MP3_BITRATE_KBPS: u32 = 64;
const MP3_LAME_QUALITY: u8 = 2;
const WAV_BITS_PER_SAMPLE: u16 = 16;
const OPUS_BITRATE_BPS: i32 = 48_000;
const OPUS_FRAME_MS: u32 = 20;
const OGG_GRANULE_RATE_HZ: u64 = 48_000;
const TARGET_LOUDNESS_LUFS: f64 = -16.0;
const TRUE_PEAK_LIMIT_DBTP: f64 = -1.0;
const EXPORT_BUFFER_BYTES: usize = 64 * 1024;
const UNSCHEDULED: i64 = i64::MAX;
const FLUSH_WAIT_LIMIT: Duration = Duration::from_millis(100);
```

`FEED_TARGET_MS` is the amount of audio that the feed thread keeps in the ring buffer. A small target keeps seek and pause fast. The ring buffer is larger, so a slow feed thread cannot cause an underrun.

### Public API

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Track { rate: SampleRate, segments: Vec<TrackSegment> }
impl Track {
    pub fn new(rate: SampleRate, segments: impl IntoIterator<Item = Option<(PathBuf, Duration)>>) -> Self;
}
pub(crate) enum TrackSegment { Stored { path: PathBuf, duration: Duration }, Pending }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrackPosition { pub segment: SegmentIndex, pub offset: Duration }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TrackId(u64);                                       // the player gives each loaded track a new id

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Volume(f32);
impl Volume { pub fn new(gain: f32) -> Self; }                 // clamps to 0.0 ..= 1.0

pub struct Player { /* command sender, event receiver, Arc<PlayerAtomics>, feed thread handle */ }
impl Player {
    pub fn start() -> Result<Self, AudioError>;              // starts the feed thread
    pub fn load(&self, track: Track) -> TrackHandle;
    pub fn set_volume(&self, volume: Volume);
    pub fn underruns(&self) -> u64;
    pub fn events(&self) -> &flume::Receiver<PlayerEvent>;
}

pub struct TrackHandle { /* command sender, Arc<TrackAtomics>, Arc<PlayerAtomics>, track id */ }
impl TrackHandle {
    pub fn id(&self) -> TrackId;
    pub fn play(&self);
    pub fn pause(&self);
    pub fn seek(&self, position: TrackPosition);
    pub fn position(&self) -> Option<TrackPosition>;
    pub fn state(&self) -> PlayerState;
    pub fn set_stored(&self, segment: SegmentIndex, path: PathBuf, duration: Duration);
    pub fn live_output(&self) -> LiveOutput;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayerState { Paused, Playing, Buffering, Ended }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayerEvent {
    StateChanged { track: TrackId, state: PlayerState },
    DeviceChanged,
    DeviceLost,
}

pub struct LiveOutput { /* command sender, track id, track rate */ }
impl Output for LiveOutput;

pub fn resample(samples: &[f32], from: SampleRate, to: SampleRate) -> Vec<f32>;

#[derive(Debug, Default)]
pub struct PeakCache { /* HashMap<PathBuf, Arc<[f32]>> */ }
impl PeakCache {
    pub fn peaks(&mut self, path: &Path) -> Result<Arc<[f32]>, AudioError>;
}

pub struct ExportSpec {
    pub segments: Vec<PathBuf>,
    pub source_rate: SampleRate,
    pub format: ExportFormat,
    pub export_rate: ExportRate,
    pub loudness: Loudness,
    pub tags: EpisodeTags,
    pub destination: PathBuf,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Display, EnumString, IntoStaticStr)]
pub enum ExportRate {                                          // the text forms are "24000", "44100" and "48000"
    #[strum(serialize = "24000")] Hz24000,
    #[default] #[strum(serialize = "44100")] Hz44100,
    #[strum(serialize = "48000")] Hz48000,
}
impl ExportRate { pub const ALL: [ExportRate; 3]; pub const fn sample_rate(self) -> SampleRate; }
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Display, EnumString, IntoStaticStr)]
#[strum(serialize_all = "lowercase")]
pub enum Loudness { #[default] Normalize, Keep }                // the text forms are "normalize" and "keep"
pub struct EpisodeTags { pub title: String, pub voice: VoiceId, pub language: Language }
pub struct ExportSummary { pub duration: Duration, pub gain_db: f64, pub bytes: u64 }

pub fn export(
    spec: &ExportSpec,
    cancel: &AtomicBool,
    progress: &dyn Fn(f32),
) -> Result<ExportSummary, AudioError>;

pub enum AudioError {
    NoDevice,
    DeviceConfig(#[source] cpal::DefaultStreamConfigError),
    BuildStream(#[source] cpal::BuildStreamError),
    PlayStream(#[source] cpal::PlayStreamError),
    ReadSegment { path: PathBuf, source: hound::Error },
    UnsupportedSegment { path: PathBuf, spec: hound::WavSpec },
    WriteWav { path: PathBuf, source: hound::Error },
    Write { path: PathBuf, source: io::Error },
    Mp3Build(#[source] mp3lame_encoder::BuildError),
    Mp3Encode(#[source] mp3lame_encoder::EncodeError),
    Mp3Tag(#[source] mp3lame_encoder::Id3TagError),
    Opus(#[source] opus::Error),
    Cancelled,
}
```

No type of this crate has `#[non_exhaustive]`. Each `AudioError` variant keeps the concrete error of its library as its source. The apps parse and print `ExportRate` and `Loudness` with their strum text forms. No app defines a second type for them. `PeakCache::default()` is the only constructor of the cache.

`Track::new` makes a `Stored` segment for each `Some((path, duration))` and a `Pending` segment for each `None`. `Timeline::stored` of stage 06 gives this list, so each app builds its track with one call.

`TrackHandle::position` returns `None` before the first sample of the track plays. The UI uses `TrackPosition::segment` for the sentence highlight. `TrackHandle::id` returns the `TrackId` of `PlayerEvent::StateChanged`, so the app can ignore the events of an old track. The app stores the `TrackId` and defines no second id type.

`ExportFormat` is the type of `antenna-core`. This crate does not define a second export format type. `ExportRate::default()` is `Hz44100`. `Loudness::default()` is `Normalize`. The apps use these defaults.

The player has no stop command. To stop, the app calls `pause` and drops the `TrackHandle`. The next `Player::load` replaces the track.

`segment_file.rs` has the function `pub(crate) fn read_segment(path: &Path) -> Result<Vec<f32>, AudioError>`. It divides each `i16` sample by `antenna_core::PCM_SCALE`. The library of stage 05 writes the segments with the same constant.

### Track and feed thread

`Player::start` starts the thread `antenna-player`. The name follows rule 4 of `docs/standards.md` section 4, because the constructor starts a thread. The feed thread owns the current track, the `Producer` of the ring buffer, the `Conversion` and the device stream. All `TrackHandle` methods except `position` and `state` send a `FeedCommand` on a `flume` channel. Each `TrackHandle` has a track id. The feed thread ignores a command with the id of a track that is not current.

```rust
pub(crate) enum FeedCommand {
    Load { id: TrackId, track: Track, atomics: Arc<TrackAtomics> },
    Play { id: TrackId },
    Pause { id: TrackId },
    Seek { id: TrackId, position: TrackPosition },
    Stored { id: TrackId, segment: SegmentIndex, path: PathBuf, duration: Duration },
    LiveBegin { id: TrackId, segment: SegmentIndex },
    LiveChunk { id: TrackId, samples: Vec<f32> },
    LiveEnd { id: TrackId },
    DeviceLost,
}
```

The feed thread state of each segment is `Stored`, `Live` (a `Vec<f32>` that grows with each `LiveChunk`), `LiveComplete` or `Pending`. A `Stored` command replaces a `Live` or `LiveComplete` buffer and drops it. The samples are the same, so the swap does not change the audio.

The feed loop does these steps in sequence.

1. Receive all commands in the channel with `try_recv`. Apply each command in sequence.
2. If the state is `Playing` and the ring buffer holds less than `FEED_TARGET_MS` of audio, read samples at the feed position. Convert them to the device rate and push them.
3. When the feed position starts a segment, store the device frame of that start in `TrackAtomics::starts`.
4. If the feed position reaches a `Pending` segment, or the end of a `Live` buffer that is not complete, set the state to `Buffering`. Continue when the data arrives.
5. If the feed position passes the last segment and the ring buffer is empty, set the state to `Ended`.
6. Wait for the next command with the result of `wait_mode`.

The feed loop gets the device stream from a closure `open_stream: impl FnMut(SampleRate) -> Result<StreamParts, AudioError>`. `StreamParts` holds the stream value, the device rate and the `Producer` of a new ring buffer. `Player::start` passes the closure of `device.rs`. A test passes a closure that returns a test stream, which the test drains from its own thread. The closure is the only seam for tests, so the feed loop needs no trait. The feed loop also takes the flush wait limit as a parameter. `Player::start` passes `FLUSH_WAIT_LIMIT`, and a test passes `Duration::ZERO`.

`wait_mode(state: PlayerState, has_work: bool) -> WaitMode` is a pure function in `feed.rs`. `WaitMode` has the variants `Poll(Duration)` and `Block`.

| State | `has_work` | Result |
|---|---|---|
| `Playing` | `true` | `Poll(Duration::ZERO)`, so the loop continues at once |
| `Playing` | `false` | `Poll(FEED_POLL)`, the time to refill the ring buffer |
| `Paused`, `Buffering`, `Ended` | any | `Block`, a `recv` with no timeout, so the thread uses no CPU |

The feed thread reads a stored segment with `read_segment` when the feed position enters it. The feed thread keeps at most the current and the next stored segment in memory.

Each change of the state sets `TrackAtomics::state` and `PlayerAtomics::is_buffering`, and sends `PlayerEvent::StateChanged`.

### Atomics

`PlayerAtomics` contains only atomics. The callback reads and writes them without locks.

| Field | Type | Writer | Reader |
|---|---|---|---|
| `played_frames` | `AtomicU64` | callback | `TrackHandle::position` |
| `flush_epoch` | `AtomicU64` | feed thread | callback |
| `flushed_epoch` | `AtomicU64` | callback | feed thread |
| `flushed_at_frame` | `AtomicU64` | callback | feed thread |
| `paused` | `AtomicBool` | feed thread, stream error callback | callback |
| `is_buffering` | `AtomicBool` | feed thread | callback |
| `device_rate_hz` | `AtomicU32` | feed thread | `TrackHandle::position` |
| `underruns` | `AtomicU64` | callback | `Player::underruns` |
| `volume_bits` | `AtomicU32` | `Player::set_volume` | callback |

`TrackAtomics` belongs to one track. It has `starts: Box<[AtomicI64]>` with one device frame for each segment, `first_scheduled: AtomicU32` and `state: AtomicU8`. A start of `UNSCHEDULED` means that the segment is not scheduled. A start can be negative after a seek, because the start of the target segment is before the first played frame of the seek.

`TrackHandle::position` reads the `PlayerAtomics` and the `TrackAtomics` that the handle holds. It does these steps.

1. Read `played_frames`.
2. Search the scheduled starts from `first_scheduled` with `partition_point`. The scheduled starts increase, so the search is valid.
3. Return the segment and the offset from its start, converted with `device_rate_hz`.

### Callback

`fill(output: &mut [f32], channels: usize, consumer: &mut Consumer<f32>, atomics: &PlayerAtomics)` is the callback logic. The stream closure only calls `fill`. Thus tests can call `fill` without a device.

1. If `flush_epoch` is greater than `flushed_epoch`, discard all readable samples. Store `played_frames` in `flushed_at_frame`. Store the epoch in `flushed_epoch`.
2. If `paused` is `true`, write zeros. Do not read the ring buffer. Do not change `played_frames`.
3. Pop one mono sample for each frame. Multiply it by the volume from `f32::from_bits(volume_bits)`. Write it to each channel of the frame. Add the number of popped frames to `played_frames`.
4. If the ring buffer has fewer samples than the frames in `output`, write zeros for the missing frames. If `is_buffering` is `false`, add 1 to `underruns`. A short buffer in the `Buffering` state is not an underrun.

### Seek

`TrackHandle::seek` sends `Seek`. The feed thread does these steps.

1. If the feed thread has no stream (after `DeviceLost`), skip steps 1 and 2 of the handshake. Set `flushed_at_frame` to `played_frames` and continue at step 3.
2. Increase `flush_epoch`. Wait with `recv_timeout` of 1 ms until `flushed_epoch` is equal. The callback runs each 5 to 10 ms, so the wait is short. Put each command that arrives during the wait in a local queue. Apply the queue in sequence after the seek. If the wait passes the flush wait limit, the callback does not run. Then do the `DeviceLost` action of "Device events" and continue at step 3.
3. Set each start to `UNSCHEDULED`. Set `first_scheduled` to the target segment.
4. Calculate the start of the target segment as `flushed_at_frame as i64 - offset_frames as i64`, where `offset_frames` is the offset of the seek at the device rate. Store it.
5. Move the feed position. If a stream exists, push `PREFILL_DURATION_MS` of audio at once.

The budget is 100 ms from `seek` to the first sample of a stored position at the device.

### Live output

`LiveOutput::open(rate)` returns a `LiveSink` that sends commands to the feed thread.

1. `begin_segment(index)` sends `LiveBegin`.
2. `write(samples)` copies the samples into a `Vec<f32>` and sends `LiveChunk`. The copy occurs on the job worker thread. The callback never makes this copy.
3. `finish` sends `LiveEnd` and returns at once.
4. If the track is not current or the player has stopped, `write` drops the samples and returns `Ok`. The job continues, because the segment store keeps the audio.

`LiveSink::write` never blocks. When the user pauses, synthesis continues and fills the segment store. This is the intended behavior of `docs/architecture.md` section 6.3.

`LiveOutput::open` returns `SinkError::RateMismatch` if `rate` is not the rate of the track. The rate of the track is the rate of the engine, so this error shows a defect in the caller.

### Device events

The stream error callback runs on a thread of `cpal`. It calls `device::on_stream_error(kind: cpal::ErrorKind, atomics: &PlayerAtomics, commands: &flume::Sender<FeedCommand>, events: &flume::Sender<PlayerEvent>)`. Tests call this function without a device.

| `cpal::ErrorKind` | Action |
|---|---|
| `DeviceChanged` | Send `PlayerEvent::DeviceChanged`. The stream continues on the new default device |
| `DeviceNotAvailable` | Set `paused` to `true`. Send `FeedCommand::DeviceLost` and `PlayerEvent::DeviceLost`. The feed thread drops the stream and its ring buffer, sets the state to `Paused` and sends `StateChanged` |
| Other kinds | Log with `tracing::warn!`. Continue |

When the feed thread has no stream and receives `Play`, it calls `open_stream` for the default device. It makes a new `Conversion` and continues from the current position. The track and its live buffers stay. If `open_stream` fails, the feed thread keeps the state `Paused` and sends `PlayerEvent::DeviceLost` again. Thus the app reopens the device with a normal `play`, and the player has no separate reopen command.

### Resampling

`Conversion` is an enum with two variants.

1. `Direct` copies the samples when the engine rate and the device rate are equal.
2. `Resample` uses the synchronous FFT resampler of `rubato` with an input chunk of `RESAMPLER_CHUNK_FRAMES` frames. It keeps pre-allocated input and output buffers. At the end of the track, it pads the last chunk with zeros and trims the output to the expected length.

The resampler runs on the feed thread and in `export`. It never runs in the callback. A seek resets the resampler state. `antenna-review` uses the public `resample` function.

### Waveform peaks

`PeakCache::peaks` reads a stored segment once and returns `PEAKS_PER_SECOND` peaks for each second of audio. A peak is the maximum absolute sample in its bucket. Stored segments never change, because the file name is the hash of its content. Thus the cache never invalidates an entry. The desktop app owns one `PeakCache` on its UI thread, so the cache needs no lock.

### Export

`export` follows `docs/architecture.md` section 6.5 with two streaming passes, so memory does not grow with the length of the episode.

1. **Measure.** If `loudness` is `Normalize`, read each segment in sequence and add its samples to an `EbuR128` with `Mode::I | Mode::TRUE_PEAK` at `source_rate`.
2. **Gain.** Calculate `gain_db = TARGET_LOUDNESS_LUFS - integrated`. If `true_peak_db + gain_db` is more than `TRUE_PEAK_LIMIT_DBTP`, set `gain_db = TRUE_PEAK_LIMIT_DBTP - true_peak_db`. If the integrated loudness is not finite (silence), set `gain_db` to 0.
3. **Encode.** Read each segment again. Apply the linear gain. Resample from `source_rate` to the encoder rate, and send the samples to the encoder. The encoder rate is `export_rate.sample_rate()` for MP3 and WAV, and the Opus encoder rate of the format table for Ogg. Use `Conversion::Direct` when `source_rate` and the encoder rate are equal.
4. **Commit.** Finish the encoder, flush the file and rename `<destination>.part` to `<destination>`.

After each segment, `export` calls `progress` with a value from 0.0 to 1.0, and reads `cancel`. If `cancel` is set, `export` drops the `PartFile` and returns `AudioError::Cancelled`.

| Format | Encoder | Settings |
|---|---|---|
| MP3 | `mp3lame-encoder` | Mono, the export rate, `MP3_BITRATE_KBPS` CBR, `MP3_LAME_QUALITY`. LAME ID3 tag with `title`, `artist` "Antenna" and `comment` "<language code> <voice id>" |
| WAV | `hound` | Mono, the export rate, `WAV_BITS_PER_SAMPLE` signed integer samples. Each sample is `(sample.clamp(-1.0, 1.0) * PCM_SCALE).round()` |
| Ogg Opus | `audiopus` and `ogg` | Mono, `OPUS_BITRATE_BPS`, frames of `OPUS_FRAME_MS`. The encoder rate is 24 kHz for `Hz24000` and 48 kHz for `Hz44100` and `Hz48000`. Pages contain `OpusHead` (with the encoder lookahead as pre-skip) and `OpusTags` (title, artist, language), then audio |

The Ogg granule position counts samples at `OGG_GRANULE_RATE_HZ`, with the pre-skip included, as RFC 7845 section 4 defines. The last page sets the end-of-stream flag. Its granule position marks the true end, so a player trims the padding of the last frame.

`PartFile` writes to `<destination>.part` through a `BufWriter` of `EXPORT_BUFFER_BYTES`. `PartFile::commit` renames the file. If a `PartFile` drops before `commit`, it deletes the `.part` file. If the delete fails, it logs the error with `tracing::warn!`.

## Tasks

1. Add the crate to the workspace members and add the workspace dependencies.
2. Add `libasound2-dev` to the Linux job of `.github/workflows/ci.yml` before `cargo xtask check`.
3. Write `track.rs`, `segment_file.rs` and `resample.rs` with their unit tests.
4. Write `player/atomics.rs` and `player/callback.rs`. Write the unit tests of `fill` for each rule in the tests module of `callback.rs`.
5. Install `assert_no_alloc::AllocDisabler` as the `#[cfg(test)]` global allocator in `lib.rs`. In the tests module of `callback.rs`, write the allocation tests. Each test wraps its call of `fill` in `assert_no_alloc::assert_no_alloc`.
6. Write `player/source.rs`, `player/command.rs` and `player/feed.rs`. Make the feed loop testable without a device through the `open_stream` closure. A test stream takes the place of the device stream and drains the ring buffer from the test thread.
7. Write `player/device.rs` and `player/mod.rs`. Write the tests of `on_stream_error` in the tests module of `device.rs`.
8. Write `live.rs`.
9. Write `peaks.rs`.
10. Write `export/part_file.rs`, `export/wav.rs`, `export/mp3.rs`, `export/ogg.rs`, `export/loudness.rs` and `export/mod.rs`.
11. Write the feed loop tests in the tests module of `feed.rs`, and write `tests/export.rs`. Make the stored segment fixtures in the tests with `hound` and a generated sine. Do not commit audio files.
12. Write `tests/device.rs`. Mark each test `#[ignore = "needs audio device"]`.
13. Write `benches/export.rs` with `divan` and `examples/seek_latency.rs`. The example loads a track of three stored segments and seeks 20 times to random stored positions. It prints the median and the maximum time from `seek` to the first target sample at the callback.
14. Run the device tests, the benchmark and the example on the reference machine. Record the output in the stage report.
15. Run `cargo xtask check`. Fix each failure.
16. Do the quality gate in `docs/standards.md` section 20.

## Acceptance criteria

| ID | Criterion | Check |
|---|---|---|
| AC-03-01 | All checks pass on macOS, Linux and Windows | `cargo xtask check` exits with code 0, and the CI run is green |
| AC-03-02 | The callback does not allocate in the normal, paused, flush, underrun and buffering states | Test `fill_does_not_allocate_when_<state>` for each state, in the tests module of `callback.rs` |
| AC-03-03 | Pause writes zeros and keeps the played frames | Test `fill_writes_silence_when_paused` |
| AC-03-04 | A flush discards the readable samples and records the frame | Test `fill_discards_samples_when_flush_requested` |
| AC-03-05 | A short buffer in the `Playing` state counts one underrun for each callback | Test `fill_counts_underrun_when_playing_and_short` |
| AC-03-06 | A short buffer in the `Buffering` state is not an underrun | Test `fill_skips_underrun_when_buffering` |
| AC-03-07 | Mono samples go to all channels of a frame, multiplied by the volume | Tests `fill_copies_sample_to_each_channel` and `fill_applies_volume` |
| AC-03-08 | A track of stored segments plays all samples in segment sequence | Test `feed_plays_stored_segments_in_order` |
| AC-03-09 | Live chunks play before the segment is stored, and a later `set_stored` does not change the output | Test `feed_plays_live_chunks_then_swaps_to_stored` |
| AC-03-10 | The state becomes `Buffering` at a pending segment and `Playing` when its data arrives | Test `feed_buffers_when_segment_pending` |
| AC-03-11 | The state becomes `Ended` after the last sample | Test `feed_ends_after_last_segment` |
| AC-03-12 | `position` returns the segment and offset of the played frames, before and after a seek | Tests `position_tracks_played_frames` and `position_restarts_at_seek_target` |
| AC-03-13 | After a seek to a stored position, the first buffer that the test sink drains after the feed thread applies `Seek` starts with the first sample of the target | Test `seek_delivers_target_sample_in_first_drained_buffer`. The test counts drained buffers and uses no clock |
| AC-03-14 | A command for an old track has no effect | Test `feed_ignores_command_when_track_replaced` |
| AC-03-15 | `LiveSink::write` returns `Ok` and drops samples when the track is not current | Test `live_sink_drops_samples_when_track_replaced` |
| AC-03-16 | The feed thread blocks with no timeout in the `Paused`, `Buffering` and `Ended` states | Test `wait_mode_blocks_when_not_playing` checks each row of the `wait_mode` table |
| AC-03-17 | Resampling from 24 kHz and from 22.05 kHz to 48 kHz keeps the length to one chunk or less of difference and the level of a 1 kHz sine to 0.5 dB or less of difference | Tests `resample_keeps_length_when_<from>_to_48000` and `resample_keeps_level_when_sine` |
| AC-03-18 | Peaks have `PEAKS_PER_SECOND` values for each second and equal the bucket maximums | Test `peaks_match_bucket_maximums` |
| AC-03-19 | A WAV export has 1 channel, the export rate, 16-bit samples and the expected sample count for each export rate | Test `wav_export_round_trips_when_<rate>` with `hound` as the reader |
| AC-03-20 | An MP3 export decodes to the expected duration with a difference of 50 ms or less for each export rate, and has the title tag | Tests `mp3_export_duration_matches_when_<rate>` with `symphonia`, and `mp3_export_has_title_tag` |
| AC-03-21 | An Ogg Opus export has `OpusHead` and `OpusTags`, an end-of-stream page, and a final granule position equal to pre-skip plus the duration at 48 kHz | Test `ogg_export_has_valid_pages_and_granule` with the `ogg` reader |
| AC-03-22 | Normalization gives an integrated loudness of -16 LUFS with a difference of 0.5 LU or less, for a sine at -30 LUFS and at -6 LUFS | Test `export_normalizes_loudness_when_<level>`, measured with `ebur128` on the decoded WAV |
| AC-03-23 | Normalization obeys the true peak limit of -1 dBTP | Test `export_limits_true_peak_when_crest_high` with a fixture of short clicks at -20 LUFS |
| AC-03-24 | Silence exports without gain and without an error | Test `export_keeps_silence_when_loudness_infinite` |
| AC-03-25 | A cancelled or failed export leaves no file and no `.part` file | Tests `export_leaves_no_file_when_cancelled` and `export_leaves_no_file_when_encoder_fails` |
| AC-03-26 | Export of 10 minutes of stored 24 kHz audio to MP3 with normalization takes 3 s or less on the reference machine (Apple M5 Pro, 24 GB) **(manual)** | `cargo bench -p antenna-audio --bench export` reports a median of 3 s or less. Add the output to the stage report |
| AC-03-27 | Playback of a 3-segment track on the default device advances `position` through all segments with 0 underruns **(manual)** | On the reference machine (Apple M5 Pro, 24 GB), `cargo nextest run -p antenna-audio --run-ignored only -E 'test(player_plays_track_when_device_available)'` passes. Add the output to the stage report |
| AC-03-28 | `antenna-audio` depends only on `antenna-core` in the workspace | `cargo xtask lint-repo` passes |
| AC-03-29 | `DeviceNotAvailable` sets `paused`, sends `DeviceLost` and moves the state to `Paused` | Test `device_lost_pauses_track_when_device_not_available` calls `on_stream_error` and runs the feed loop with a test sink |
| AC-03-30 | A seek to a stored position plays its first sample at the device in 100 ms or less **(manual)** | On the reference machine (Apple M5 Pro, 24 GB), `cargo run --release -p antenna-audio --example seek_latency` prints a maximum of 100 ms or less. Add the output to the stage report |
| AC-03-31 | `Track::new` makes a stored segment for each `Some` and a pending segment for each `None` | Test `track_marks_segment_pending_when_entry_none` |
| AC-03-32 | `LiveOutput::open` with a rate that is not the track rate returns `RateMismatch` | Test `live_output_fails_when_rate_differs` |
| AC-03-33 | A seek to an offset larger than the played frames gives a correct position | Test `position_is_correct_when_seek_offset_exceeds_played_frames` |
| AC-03-34 | `read_segment` divides by `PCM_SCALE`, so `i16::MAX` reads as 1.0 | Test `read_segment_scales_when_sample_is_max` |
| AC-03-35 | `DeviceChanged` sends `PlayerEvent::DeviceChanged` and keeps the state | Test `device_changed_keeps_state_when_stream_continues` calls `on_stream_error` |
| AC-03-36 | `play` after `DeviceLost` opens the stream again and continues from the current position | Test `feed_reopens_stream_when_play_after_device_lost` counts the calls of the `open_stream` closure and checks the first drained sample |
| AC-03-37 | A seek while the device is lost moves the position without a handshake | Test `seek_skips_flush_when_device_lost` |
| AC-03-38 | A flush that the callback does not answer in `FLUSH_WAIT_LIMIT` makes the device lost, and commands that arrive during the wait apply after the seek | Tests `seek_drops_stream_when_flush_unanswered` and `seek_applies_queued_commands_when_flush_completes`. The first test runs the feed loop with a flush wait limit of `Duration::ZERO` and a test stream that never drains |
| AC-03-39 | The Ogg Opus encoder runs at 24 kHz for `Hz24000` and at 48 kHz for the other rates | Test `ogg_export_uses_encoder_rate_when_<rate>` reads the input sample rate field of `OpusHead` |
| AC-03-40 | The MP3 tag has the artist "Antenna" and the comment "<language code> <voice id>" | Test `mp3_export_has_artist_and_comment_tags` |
| AC-03-41 | The text forms of `ExportRate` and `Loudness` round trip | Tests `export_rate_round_trips_when_text_parsed` and `loudness_round_trips_when_text_parsed` |
| AC-03-42 | An Ogg export at `Hz44100` gives the Opus encoder 48 kHz samples, so the duration of the file matches the source | Test `ogg_export_duration_matches_when_hz44100` reads the file with the Ogg reader of `symphonia`. It calculates the duration from the granule position of the last page minus the pre-skip, at 48 kHz, and compares it with the source duration with a difference of 50 ms or less. `symphonia` has no Opus decoder, so the test does not decode the audio |

## Decision rules

1. `mp3lame-encoder` has the license LGPL-3.0, and stage 01 put it in the allowlist of `.config/deny.toml`. If `cargo deny` rejects a different license, stop and write a "Blocked" section.
2. The `opus` crate builds libopus from source through `opusic-sys`, and the build needs CMake 3.16 or later. If a CI target does not have it, install CMake in that job of `.github/workflows/ci.yml`. If the build of `opus` fails for a different cause, stop and write a "Blocked" section with the build error. A human decides the replacement crate and changes the table in `docs/architecture.md` section 10.
3. If `cpal::Stream` is not `Send` on a CI target, create the stream on the feed thread and keep it there. The feed thread already owns the stream, so no other change is necessary.
4. If the device does not support `f32` samples, build the stream in the native sample format and convert with `cpal::Sample::from_sample`. Do not allocate in the conversion.
5. If `assert_no_alloc` reports an allocation in `fill`, remove the allocation. Do not move the check out of the test.
6. If the `rubato` FFT resampler cannot work with a fixed input chunk, use its synchronous sinc resampler with the same chunk size. Write the reason in the stage report.
7. The `symphonia` decode of an MP3 file can be longer than the input by the LAME encoder delay. Remove the delay and the padding that the Xing/LAME header records, then compare the duration.
8. If a stored segment is not 16-bit mono PCM, return `AudioError::UnsupportedSegment`. Do not convert other formats. Stage 05 writes only this format.
9. If the seek budget of AC-03-30 fails, decrease `PREFILL_DURATION_MS` before any other change. Do not change the budget.
10. If a library error type has a different name than the `AudioError` block, use the real type and keep one variant for each source. If the type does not implement `std::error::Error`, keep it as a field without `#[source]` and put its `Debug` text in the `#[error]` message.

## Out of scope

1. Selection of an output device other than the default device.
2. Playback speed. See `docs/architecture.md` section 1.2.
3. Read or write of the segment store and the library. Stage 05 owns the store. The apps give paths to this crate.
4. Any change to `antenna-core`.
