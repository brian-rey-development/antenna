# Stage 06. Pipeline

## Goal

`antenna-pipeline` runs a first-in, first-out job queue on one worker. It reuses stored segments, stores each new segment, streams live chunks, isolates panics and stops a cancelled job after one more chunk or less. The `fake` engine proves each behavior.

## Context

The pipeline is the only path to synthesis. The CLI (stage 07), `antenna-eval` (stage 09) and the desktop app (stages 16 to 18) start all jobs through it.

The pipeline follows `docs/architecture.md` sections 6.1 and 6.2. Generation and playback are separate. The pipeline writes each segment to the segment store of `antenna-library` (stage 05). The player of `antenna-audio` reads the store and gets live chunks through a `LiveOutput`. The pipeline does not depend on `antenna-audio`. It knows only the traits of `antenna-core`.

A segment that is already in the store is never synthesized again. Thus a second play, an export after a play and an edit of a document synthesize only the new segments.

## Read first

1. `CLAUDE.md`
2. `docs/architecture.md` sections 2, 5, 6.1, 6.2, 6.7, 7.1, 7.3, 8 and 9
3. `docs/standards.md` sections 5, 6, 7, 11, 14, 18 and 19
4. `docs/plans/02-text.md` (`prepare` and `SegmentLimits`)
5. `docs/plans/04-models.md` (the `ModelStore` API)
6. `docs/plans/05-library.md` (the `SegmentStore` API)

## Scope

The stage can create or change these files only.

- `crates/pipeline/`
- `Cargo.toml` (the `members` list and `[workspace.dependencies]` only)

## Deliverables

### File layout

```
crates/pipeline/
├── Cargo.toml
├── src/
│   ├── lib.rs          # module declarations and re-exports
│   ├── pipeline.rs     # Pipeline: new, timeline, start, preload
│   ├── job.rs          # JobSpec, JobHandle, JobEvent
│   ├── queue.rs        # JobQueue: submission order and queue positions
│   ├── worker/
│   │   ├── mod.rs      # the worker loop and the job sequence
│   │   ├── plan.rs     # segments, keys and the timeline of a job
│   │   ├── engine.rs   # ensure, checkout and checkin of the engine of a job
│   │   └── fan_out.rs  # the emit callback: cancellation check, store write, sink write
│   ├── pool.rs         # EnginePool
│   ├── timeline.rs     # Timeline, SegmentSpan
│   └── error.rs        # JobError, ErrorKind
└── tests/
    └── pipeline/
        ├── main.rs     # integration tests
        ├── outputs.rs  # GateOutput and the other test outputs
        ├── counting.rs # CountingFactory: counts load and synthesize calls of the fake engine
        └── artifacts.rs # ArtifactFactory: a fake engine with one small artifact
```

### Dependencies

Normal dependencies are `antenna-core`, `antenna-text`, `antenna-models`, `antenna-library`, `flume`, `thiserror` and `tracing`. Dev-dependencies are `antenna-engine-fake`, `antenna-engine-testkit`, `proptest`, `tempfile` and `hound`.

### Constants

```rust
pub const ENGINE_POOL_CAPACITY: usize = 2;
const WORKER_THREAD_NAME: &str = "antenna-job";
```

### Public API

```rust
pub struct Pipeline { /* command sender of the worker, Arc<Inner>: factories by EngineId, ModelStore, SegmentStore */ }

impl Pipeline {
    pub fn new(
        factories: impl IntoIterator<Item = Arc<dyn EngineFactory>>,
        models: ModelStore,
        segments: SegmentStore,
    ) -> Result<Self, JobError>;
    pub fn timeline(&self, document: &Document, voice: VoiceId, quality: Quality) -> Result<Arc<Timeline>, JobError>;
    pub fn start(&self, spec: JobSpec) -> Result<JobHandle, JobError>;
    pub fn preload(&self, voice: VoiceId, quality: Quality) -> Result<JobHandle, JobError>;
}

pub struct JobSpec {
    pub document: Document,
    pub voice: VoiceId,                     // the voice also sets the language
    pub quality: Quality,
    pub start_segment: SegmentIndex,
    pub outputs: Vec<Box<dyn Output>>,
}

pub struct JobHandle { /* id, flume::Receiver<JobEvent>, Arc<AtomicBool>, command sender */ }

impl JobHandle {
    pub fn events(&self) -> &flume::Receiver<JobEvent>;
    pub fn cancel(&self);
}

impl Drop for JobHandle { /* calls cancel */ }

pub enum JobEvent {
    Queued { position: usize },
    Started { timeline: Arc<Timeline> },
    Downloading { done_bytes: u64, total_bytes: u64 },
    Loading,
    Synthesized { segment: SegmentIndex, duration: Duration, is_cached: bool },
    Finished,
    Failed(JobError),
    Cancelled,
}

pub struct Timeline { /* rate, spans, keys, durations: Box<[OnceLock<Duration>]> */ }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SegmentSpan { pub index: SegmentIndex, pub source: Range<usize> }

impl Timeline {
    pub fn rate(&self) -> SampleRate;
    pub fn spans(&self) -> &[SegmentSpan];
    pub fn key(&self, segment: SegmentIndex) -> Option<&SegmentKey>;
    pub fn duration(&self, segment: SegmentIndex) -> Option<Duration>;
    pub fn start_of(&self, segment: SegmentIndex) -> Option<Duration>;
    pub fn segment_at(&self, position: Duration) -> Option<SegmentIndex>;
    pub fn stored(&self, store: &SegmentStore) -> Vec<Option<(PathBuf, Duration)>>;
    pub fn paths(&self, store: &SegmentStore) -> Vec<PathBuf>;
    pub fn is_complete(&self) -> bool;
}
```

`Pipeline::timeline` changes no state. It prepares the segments, calculates the keys and reads the durations of the stored segments from the WAV headers. It does not start a job. The apps use it to build the `Track` of the player before they start a job. The worker uses the same function, so both timelines are equal.

`Timeline` is shared between the worker and the UI. The worker sets each `OnceLock` one time. The UI reads without a lock. `segment_at` measures time from the start of the document. It sums the known durations with `partition_point` over a prefix sum. If a segment before `position` has no duration, it returns `None`. `start_of` returns the sum of the durations before the segment, or `None` when one of them is unknown. The apps convert between a document time and a `TrackPosition` with `segment_at` and `start_of`.

`stored` returns one entry for each segment. The entry is `Some` with the store path of the key and the duration when the duration is known, and `None` in all other cases. `Track::new` of stage 03 takes this list. Thus the CLI and the desktop app build a track with one call and no copy of this logic.

`paths` returns the store path of the key of each segment, in segment sequence. An app calls it only when `is_complete` is `true`, and gives the list to `ExportSpec::segments` of stage 03. Thus the CLI and the desktop app build the export list with one call.

### Errors

```rust
pub enum JobError {
    UnknownVoice { voice: VoiceId },
    StartSegmentOutOfRange { start: SegmentIndex, count: u32 },
    Text(TextError),
    Model(ModelError),
    Store(LibraryError),
    Engine(EngineError),
    EnginePanic { segment: Option<SegmentIndex>, message: String },
    Sink(SinkError),
    WorkerSpawn(std::io::Error),
    WorkerStopped,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ErrorKind { InvalidRequest, Download, DiskSpace, ModelCorrupt, ModelLoad, Synthesis, Storage, Output, Internal }

impl JobError { pub fn kind(&self) -> ErrorKind; }
```

| `JobError` variant | `ErrorKind` |
|---|---|
| `UnknownVoice`, `StartSegmentOutOfRange`, `Text` | `InvalidRequest` |
| `Model` with the disk space variant of `ModelError` | `DiskSpace` |
| `Model` with the hash or size variant of `ModelError` | `ModelCorrupt` |
| `Model` with any other variant | `Download` |
| `Engine` with `MissingFile` or `Load` | `ModelLoad` |
| `Engine` with `Inference` or `Runaway`, and `EnginePanic` | `Synthesis` |
| `Store` | `Storage` |
| `Sink` | `Output` |
| `WorkerSpawn`, `WorkerStopped` | `Internal` |

`JobError::kind` matches all variants of `JobError`, `ModelError` and `EngineError` without `_ =>`. None of these enums has `#[non_exhaustive]`, so the match compiles across crates. Thus a new variant in one of these enums causes a compile error until it has a kind.

Each descriptor has a variant for each quality, because `Variants` has one field for each `Quality`. Thus a job cannot ask for a missing variant, and `JobError` has no variant for it.

### Queue

The worker thread owns the queue, the engine pool and the current job. `start`, `preload` and `JobHandle::cancel` send commands to the worker on a `flume` channel. Thus no queue state is shared, and the pipeline has no lock.

```rust
pub(crate) enum WorkerCommand {
    Submit { job: QueuedJob },
    Cancel { id: u64 },
}
```

1. The queue is first in, first out. A `preload` job and a document job wait in the same queue.
2. When a job waits, it receives `Queued { position }`. Position 1 is the next job. Each time the position of a job changes, it receives a new `Queued`. A job that starts at once receives no `Queued`.
3. The worker reads the command channel before each segment.
4. A `Cancel` command for a job in the queue removes the job from the queue. The job receives `Cancelled`. Each later job in the queue receives `Queued` with its new position.

### Job event contract

Each job sends its events in this sequence.

```
Queued ×N ──> Started ──> Downloading ×N ──> Loading? ──> Synthesized ×N ──> Finished
(a preload job has no Started and no Synthesized)
(cached Synthesized events can come before Downloading and Loading)

From any state before the terminal event:  ──> Failed(JobError)  or  ──> Cancelled
```

1. `Started` occurs one time for a document job, before any download or load. The UI shows the pending text from this event.
2. `Downloading` occurs zero or more times, only when a segment needs synthesis and artifacts are missing.
3. `Loading` occurs one time when a segment needs synthesis and the pool does not contain the engine.
4. `Synthesized` occurs one time for each segment from `start_segment` to the last segment, in segment sequence. `is_cached` is `true` when the segment was in the store.
5. Each job sends exactly one terminal event, which is `Finished`, `Failed` or `Cancelled`. The terminal event is the last event. After it, the worker drops the sender, so the channel disconnects.
6. If the cancel flag is set before the worker calls `Sink::finish`, the terminal event is `Cancelled`. The worker drops the sinks without `finish`.

### Job sequence

The worker does these steps for a document job. Each step has a `tracing` span with the same name.

1. `plan`. Find the factory and the voice. Call `antenna_text::prepare` with the document, the language of the voice and `SegmentLimits::new(descriptor.max_segment_chars)`. Calculate `SegmentKey::new(descriptor, voice, quality, segment.text())` for each segment. Read the duration of each stored segment from the `SegmentStore`. If `start_segment` is not less than the segment count, fail with `StartSegmentOutOfRange`. Send `Started`.
2. `open`. Open each output with the sample rate of the descriptor.
3. `synthesize`. For each segment from `start_segment`, do these steps.
   1. If the segment is in the store, set its duration and send `Synthesized` with `is_cached: true`. Do not send it to the outputs. The player reads it from the store.
   2. If the job has no engine yet, do the `engine` step.
   3. Call `begin_segment` on each sink. Create a `SegmentWriter` with the store.
   4. Call `Engine::synthesize` in `catch_unwind(AssertUnwindSafe(..))`. The emit callback is the fan-out of `worker/fan_out.rs`.
   5. Check the result of `synthesize`. The engine returns `Ok` also when the fan-out returned `Break`, so `Ok` alone does not prove a complete segment.
      1. If the cancel flag is set, drop the `SegmentWriter` without a commit. Drop the sinks without `finish`. Send `Cancelled`.
      2. If `FanOutState` holds a write error, drop the `SegmentWriter` without a commit. Send `Failed` with `JobError::Store` for an error of the writer, or `JobError::Sink` for an error of a sink.
      3. If `synthesize` returned an error, drop the `SegmentWriter` without a commit and send `Failed(JobError::Engine)`.
   6. Commit the `SegmentWriter`. Set the duration in the timeline. Send `Synthesized` with `is_cached: false`.
4. `finish`. Call `Sink::finish` on each sink. Check in the engine. Send `Finished`.

The `engine` step occurs only at the first segment that is not in the store.

1. Call `ModelStore::ensure` with `antenna_models::voice_artifacts(descriptor, voice, quality)`. Send `Downloading` for each progress report.
2. Take the engine for the voice and the quality from the pool. If the pool does not contain it, send `Loading` and call `EngineFactory::load` in `catch_unwind`.

Thus a job for a document that is completely in the store never downloads and never loads an engine.

After a panic, the worker drops the engine. It never returns a panicked engine to the pool. The worker commits a `SegmentWriter` only in step 3.6. Each other exit drops it, so no partial segment reaches the store. The store keeps each committed file under its content key. No later job repairs a partial segment in the store.

A `preload` job does only the `engine` step, then checks in the engine and sends `Finished`.

### Fan-out

```rust
pub(crate) fn fan_out<'a>(
    writer: &'a mut SegmentWriter,
    sinks: &'a mut [Box<dyn Sink>],
    cancel: &'a AtomicBool,
    state: &'a mut FanOutState,
) -> impl FnMut(PcmChunk) -> ControlFlow<()> + 'a;
```

1. If `cancel` is set, return `Break` and do not write the chunk.
2. Write the chunk to the `SegmentWriter`, then to each sink in sequence. If a write fails, keep the first error in `FanOutState` with its source (`LibraryError` or `SinkError`) and return `Break`.
3. Add the sample count of the chunk to the running total in `FanOutState`.

The fan-out does not allocate memory. A blocked `Sink::write` blocks the engine. This is the backpressure of `docs/architecture.md` section 5.3.

### Engine pool

```rust
pub(crate) struct EnginePool { slots: VecDeque<((VoiceId, Quality), Box<dyn Engine>)> }

impl EnginePool {
    pub(crate) fn checkout(&mut self, voice: VoiceId, quality: Quality) -> Option<Box<dyn Engine>>;
    pub(crate) fn checkin(&mut self, voice: VoiceId, quality: Quality, engine: Box<dyn Engine>);
}
```

1. `checkout` removes the engine of the voice and the quality from the pool and returns it.
2. `checkin` puts the engine at the front. If the pool has more than `ENGINE_POOL_CAPACITY` engines, it drops the engine at the back.
3. Only the worker thread owns the pool. Thus the pool has no `Mutex`.

### Events channel

The events channel of each job is `flume::unbounded`. The number of events of a job is limited by the number of segments, the download reports and the queue moves. A bounded channel blocks synthesis when the UI is slow, so the channel is unbounded.

## Tasks

1. Add `crates/pipeline` to the workspace.
2. Write `error.rs` with `JobError`, `ErrorKind` and the `kind` mapping.
3. Write `timeline.rs`. Add the unit tests and the `proptest` invariant of AC-06-19.
4. Write `pool.rs` with its unit tests.
5. Write `queue.rs` with its unit tests for submission order and positions.
6. Write `worker/fan_out.rs` with its unit tests.
7. Write `worker/plan.rs` and `worker/engine.rs`.
8. Write `worker/mod.rs` with the worker loop, the command handling and the job sequence.
9. Write `job.rs` and `pipeline.rs`. `start`, `preload` and `timeline` check that a factory has the voice before they send a command. They return these errors directly and send no events.
10. Write `tests/pipeline/outputs.rs`. `GateOutput` blocks each `write` until the test releases it through a `flume` channel. It reports each `begin_segment`, each write and each `finish` through a second channel. `FailingOutput` returns `SinkError::Closed` from its first `write`.
11. Write `tests/pipeline/counting.rs`. `CountingFactory` wraps `antenna_engine_fake::Factory` and counts the calls of `load` and `synthesize` with atomics.
12. Write `tests/pipeline/artifacts.rs`. `ArtifactFactory` has a static descriptor with one voice and one artifact of 4 KB. The test writes the artifact bytes into a temporary source directory, and the descriptor has the SHA-256 of these bytes as a constant. The engine of `ArtifactFactory` delegates to `antenna_engine_fake::Factory`.
13. Write the integration tests of the acceptance criteria in `tests/pipeline/main.rs`. Each test opens a `SegmentStore` and a `ModelStore` with `Source::Directory` in a new `tempfile::TempDir`.
14. Run `cargo xtask check`. Fix each failure.
15. Do the quality gate in `docs/standards.md` section 20.

## Acceptance criteria

| ID | Criterion | Check |
|---|---|---|
| AC-06-01 | A job on an empty store sends `Started`, `Loading`, one `Synthesized` for each segment with `is_cached: false`, then `Finished` | Test `job_emits_events_in_contract_order` |
| AC-06-02 | After `Finished`, each segment is in the store as 16-bit mono PCM at the engine rate, with the sample count of the fake engine | Test `job_stores_each_segment` reads the files with `hound` |
| AC-06-03 | A second job for the same document sends only cached `Synthesized` events, and loads and synthesizes nothing | Test `job_uses_store_when_all_segments_cached` with `CountingFactory` (0 loads, 0 synthesize calls, no `Loading`, no `Downloading`) |
| AC-06-04 | After an edit of one sentence, a job synthesizes only that segment | Test `job_synthesizes_only_changed_segment` with `CountingFactory` (1 synthesize call) |
| AC-06-05 | Live outputs receive `begin_segment` and the samples of each synthesized segment in order, and nothing for cached segments | Test `job_streams_only_synthesized_segments_to_outputs` |
| AC-06-06 | The outputs open at the sample rate of the descriptor | Test `job_opens_outputs_at_engine_rate` |
| AC-06-07 | A job with `start_segment` set to k synthesizes only the segments from k | Test `job_starts_at_requested_segment` |
| AC-06-08 | A `start_segment` that is too large fails with `StartSegmentOutOfRange` | Test `job_fails_when_start_segment_out_of_range` |
| AC-06-09 | After `cancel`, the sinks receive one chunk or less | Test `job_stops_after_one_chunk_when_cancelled` with `GateOutput` |
| AC-06-10 | Drop of the `JobHandle` cancels the job | Test `job_cancels_when_handle_dropped` |
| AC-06-11 | A cancelled waiting job sends `Cancelled` and no `Started` | Test `queued_job_cancels_without_start` |
| AC-06-12 | A blocked sink stops the engine | Test `job_stops_synthesis_when_sink_blocks`. While `GateOutput` blocks, the engine emits no more than one more chunk |
| AC-06-13 | An engine panic gives `Failed(EnginePanic)`, leaves no partial segment in the store, and the next job of the same voice sends `Loading` and finishes | Test `pipeline_runs_next_job_when_engine_panics` with `Fault::PanicAt` |
| AC-06-14 | An engine error gives `Failed(Engine)`, and the sinks do not receive `finish` | Test `job_fails_without_finish_when_segment_fails` with `Fault::FailAt` |
| AC-06-15 | An unknown voice fails in `start`, `preload` and `timeline` and sends no command | Tests `start_fails_when_voice_unknown`, `preload_fails_when_voice_unknown` and `timeline_fails_when_voice_unknown` |
| AC-06-16 | Jobs run in submission order, and each waiting job receives its position | Test `jobs_run_in_submission_order` |
| AC-06-17 | When a waiting job is cancelled, each later waiting job receives `Queued` with its new position | Test `queued_positions_update_when_waiting_job_cancelled` |
| AC-06-18 | The pool evicts the least recently used engine, and a different quality of the same voice is a different engine | Tests `pool_evicts_least_recent_engine_when_full` and `pool_loads_again_when_quality_differs` |
| AC-06-19 | `segment_at` and `start_of` agree with a linear search over the known durations, and return `None` after an unknown duration | `proptest` test `timeline_agrees_with_linear_search` and test `segment_at_returns_none_after_unknown_duration` |
| AC-06-20 | `Pipeline::timeline` gives the same spans and keys as the `Started` timeline of a job | Test `timeline_query_matches_job_timeline` |
| AC-06-21 | `preload` loads the engine and sends no `Started`. The next job of that voice and quality sends no `Loading` | Test `preload_loads_engine_without_document` |
| AC-06-22 | Each job sends exactly one terminal event, for a cancel at each possible chunk | Test `job_ends_with_one_terminal_event_when_cancelled_at_any_chunk` |
| AC-06-23 | `Finished` comes after every sink returns from `finish` | Test `job_sends_finished_after_sinks_finish` |
| AC-06-24 | The segments use the language of the voice | Test `job_segments_with_voice_language` with a Spanish abbreviation fixture and the `fake/es-alba` voice |
| AC-06-25 | `kind` gives the kind of the mapping table for each variant | Test `job_error_kind_matches_table` |
| AC-06-26 | The pipeline has no lock | `rg -n -e Mutex -e RwLock -e '\.lock\(' crates/pipeline/src` prints nothing |
| AC-06-27 | No test uses `sleep` or wall-clock timing | `rg -n -e sleep -e Instant crates/pipeline/tests` prints nothing |
| AC-06-28 | A job whose voice has missing artifacts sends `Downloading` events that end at the total, before `Loading`. A second job of that voice sends no `Downloading` | Test `job_sends_downloading_when_artifacts_missing` with `ArtifactFactory` |
| AC-06-29 | `Timeline::stored` gives `Some` with the store path for each known duration and `None` for each other segment | Test `timeline_stored_marks_segment_none_when_duration_unknown` |
| AC-06-30 | `Timeline::paths` gives the store path of each segment key in segment sequence | Test `timeline_paths_follow_segment_order` |
| AC-06-31 | A cancel in the middle of a segment leaves no file of that segment in the store, and the job ends with `Cancelled` | Test `job_leaves_no_segment_when_cancelled_mid_segment` with `GateOutput`. The test cancels after the first chunk of a segment and checks `SegmentStore::contains` for its key |
| AC-06-32 | A sink write error fails the job with `JobError::Sink`, leaves no file of that segment in the store, and sends no `Finished` | Test `job_fails_when_sink_write_fails` with a test output whose `write` returns `SinkError::Closed` |

## Decision rules

1. If the signatures of `antenna_text::prepare`, `ModelStore` or `SegmentStore` differ from this file, use the real signatures. Keep the behavior of this file.
2. Use `antenna_models::voice_artifacts` for the artifacts of a job. Do not join `Variant::artifacts` and `VoiceDescriptor::artifacts` in this crate.
3. If `Box<dyn Engine>` is not `UnwindSafe`, use `AssertUnwindSafe`. After a panic, drop the engine. Do not return it to the pool.
4. If a test needs to wait for the worker, wait on a channel. Do not use `sleep`.
5. If the panic payload is not a `&str` or a `String`, use the message "engine panicked".
6. If the default panic hook prints to stderr during the panic tests, keep it. Do not change the global panic hook in library code.
7. If a stored segment disappears between `plan` and `synthesize`, synthesize it again. A garbage collection of the library can cause this.
8. If a requirement of this file needs a change to a trait in `antenna-core`, stop and write a "Blocked" section.

## Out of scope

1. Concrete outputs and the player. Stage 03 writes `LiveOutput` and the `Player`.
2. Export. Stage 03 writes `export`, and the apps call it after a job finishes.
3. Real engines and model downloads from the network. The tests install artifacts from a `Source::Directory`.
4. The documents of the library. Stage 05 owns them. The pipeline uses only the `SegmentStore`. The apps call `Library::record_segments`, `set_progress` and `record_complete` from the job events, as stage 05 defines.
5. Any UI or CLI code.
