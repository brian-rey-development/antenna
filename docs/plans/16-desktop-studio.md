# Stage 16. Desktop Studio

## Goal

The GPUI app `antenna-desktop` opens with its sidebar and the Studio screen. The Studio plays and exports a library document with the `fake` engine. The app meets the start, idle and editor budgets.

## Context

This stage builds the shell of the app and its primary screen. The Studio is where the user writes a document, listens to it and exports it. Stages 17 and 18 add the other four screens to the same shell. The stage uses the tokens and the Flow components of `antenna-ui` (stage 15), so it contains no raw design values. The stage uses the `fake` engine, so it can run before the engine stages are complete. Stage 19 repeats the manual QA with the real engines.

## Read first

1. `CLAUDE.md`
2. `docs/architecture.md` sections 1, 2, 5, 6, 7.2, 8, 9 and 12
3. `docs/standards.md` sections 3, 5, 7, 10, 13, 18 and 19
4. `docs/writing.md` section 4 (glossary)
5. `docs/design/screens/estudio.png` and `docs/design/pages/estudio.html`
6. `docs/design/tokens.md` and the component table of stage 15
7. The public API of `antenna-pipeline`, `antenna-audio`, `antenna-library`, `antenna-text` and `antenna-engine-registry` as stages 02, 03, 05 and 06 committed it to `main`

## Scope

The stage can create or change these files only.

- `apps/desktop/`
- `Cargo.toml` (the `members` list and `[workspace.dependencies]`)

## Deliverables

### Crate

1. Package `antenna-desktop` in `apps/desktop`, with one binary target `antenna-desktop` and one example target `seed_library`.
2. Feature `engine-fake`. It enables the feature with the same name in `antenna-engine-registry`. The crate gets the registry from `[workspace.dependencies]`, which sets `default-features = false`. The crate has no default features in this stage. Stage 19 adds the features `engine-qwen3` and `engine-magpie` and makes them the default features.
3. Dependencies `antenna-ui` and `gpui`, with the workspace pin of stage 15. The app does not use `gpui-component`. It uses the components of `antenna-ui`.
4. The dev-dependency `gpui` with the feature `test-support`, for the tests that use `gpui::TestAppContext`.

### Files

```
apps/desktop/src/
├── main.rs              # composition root: registry, pipeline, library, player, window
├── shell/
│   ├── mod.rs           # window root view: sidebar and the routing to the active screen
│   └── sidebar.rs       # logo, navigation, recent documents, "What's new" entry, settings entry
├── state/
│   ├── mod.rs           # AppState entity, Action, dispatch and the command runner
│   ├── screen.rs        # Screen enum
│   ├── session/
│   │   ├── mod.rs       # Session, Input, Command, the terms and transition, which calls the module of the input group
│   │   ├── document.rs  # the rows of DocumentChanged and Timeline
│   │   ├── play.rs      # the rows of Play, Pause, Stop, the seeks, TrackLoaded, PlayerState and DeviceLost
│   │   ├── job.rs       # the rows of the job events
│   │   └── export.rs    # the rows of StartExport, Export and Dismiss
│   ├── studio.rs        # StudioDocument, the draft rule and the title rule
│   ├── view.rs          # the view data of the screens, the time estimates, the skip target and the status line
│   └── bridge.rs        # GPUI tasks that move JobEvent, PlayerEvent and ExportEvent into AppState
├── settings/
│   ├── mod.rs           # Settings: typed values, defaults, voice_for, load, save
│   └── file.rs          # SettingsFile: the TOML form and its conversion to Settings
├── strings/
│   ├── mod.rs           # Strings struct and the locale selection
│   ├── en.rs            # English text
│   └── es.rs            # Spanish text
└── screens/
    ├── mod.rs           # module declarations and re-exports
    └── studio/
        ├── mod.rs       # Studio layout: top bar, toolbar row, document area, inspector, player bar
        ├── top_bar.rs   # breadcrumb, editable title, inspector toggle, export button
        ├── editor.rs    # editing view: TextArea
        ├── reading.rs   # reading view: paragraphs, gutter times, highlight, pending color, click to seek
        ├── inspector.rs # voice card, other voices, language, quality, export format, normalization
        └── player_bar.rs
apps/desktop/examples/
└── seed_library.rs      # writes N documents into a data directory for the start budget
```

### State and screens

`AppState` is the only owner of the `Player`, the `Library`, the `Pipeline`, the `ModelStore` and the job handles. Screens and the shell do not call these objects and do not name their crates. They read view data from `state/` and send an `Action` with `AppState::dispatch`.

```rust
pub(crate) enum Action {
    Play,
    Pause,
    Stop,
    SeekTo(Duration),
    SeekToSegment(SegmentIndex),
    Skip(SkipDirection),                 // AppState converts it into Input::SeekTo
    StartExport(PathBuf),
    Dismiss(Notice),
    SetVolume(Volume),                   // AppState calls Player::set_volume
    EditText(String),
    Rename(String),
    SetVoice(VoiceId),
    SetQuality(Quality),
    SetLanguage(Language),
    SetExportFormat(ExportFormat),
    SetLoudness(Loudness),
    NewDocument,
    OpenDocument(DocumentId),
    OpenFile(PathBuf),
    Navigate(Screen),
    ToggleInspector,
    ToggleSidebar,
}
pub(crate) enum SkipDirection { Back, Forward }
```

1. `AppState::studio_view`, `AppState::player_view` and `AppState::sidebar_view` return the view data of `state/view.rs`. The view types contain `antenna-core` types, `std` types, desktop types and the value types that `state/view.rs` re-exports (`DocumentId`, `Loudness`, `ExportRate`, `Volume`). Screens name these value types through `crate::state`.
2. `AppState::dispatch` converts `Play`, `Pause`, `Stop`, `SeekTo`, `SeekToSegment`, `StartExport` and `Dismiss` into the `Input` with the same name. Only `AppState` and `state/bridge.rs` make the other `Input` values. Screens and the shell never name `Input`.
3. `Skip` becomes `Input::SeekTo` of `skip_target` (see "Play position and view data"). When `skip_target` returns `None`, `AppState` sends nothing.
4. `AppState` keeps `strings: &'static Strings`. Each view reads its text from it.
5. The volume `Slider` gives a value from 0 to 1. The player bar converts it with `Volume::new`.

### Session and transitions

The job, the player and the export are three separate machines, because generation and playback are separate (`docs/architecture.md` section 2).

```rust
pub(crate) struct Session {
    timeline: Option<Arc<Timeline>>,
    job: JobState,
    play: PlayState,
    export: ExportState,
    timeline_ticket: TimelineTicket,
    job_ticket: JobTicket,
    track: Option<TrackId>,
}

pub(crate) enum JobOutput { Live, Silent }

pub(crate) enum JobState {
    Idle,
    Queued { output: JobOutput, start: SegmentIndex, position: Option<usize> },   // position is None until the first Queued event
    Generating { output: JobOutput, start: SegmentIndex, preparing: Option<Preparation> },
    Failed { kind: ErrorKind },
}
pub(crate) enum Preparation { Downloading { done_bytes: u64, total_bytes: u64 }, Loading }

pub(crate) enum PlayState { Stopped, Playing, Buffering, Paused }

pub(crate) enum ExportState { Idle, Waiting { path: PathBuf }, Encoding { path: PathBuf, fraction: f32 }, Done { path: PathBuf }, Failed }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TimelineTicket(u64);                // transition increments it for each LoadTimeline
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct JobTicket(u64);                     // transition increments it for each StartJob

pub(crate) enum ExportEvent { Progress(f32), Done(ExportSummary), Failed(AudioError) }

pub(crate) enum Notice { JobError, Export }

pub(crate) enum Input {
    Timeline(TimelineTicket, Arc<Timeline>),
    Job(JobTicket, JobEvent),
    TrackLoaded(TrackId),
    PlayerState(TrackId, PlayerState),
    DeviceLost,
    Export(ExportEvent),
    Play, Pause, Stop,
    SeekTo(Duration), SeekToSegment(SegmentIndex),
    StartExport(PathBuf),
    DocumentChanged,
    Dismiss(Notice),
}

pub(crate) enum Command {
    LoadTimeline { ticket: TimelineTicket },
    StartJob { ticket: JobTicket, start_segment: SegmentIndex, output: JobOutput },
    CancelJob,
    PlayFrom(TrackPosition),
    Pause, Resume, StopPlayer,
    Seek(TrackPosition),
    Encode { path: PathBuf },
}

pub(crate) fn transition(session: Session, input: Input) -> (Session, Vec<Command>);
```

`transition` is a pure function. It does no I/O and does not touch GPUI. `AppState::dispatch` calls `transition` and then runs the commands in sequence against the pipeline, the player and the export thread. Thus each behavior rule is testable without a window.

These terms apply in the table.

1. The job "runs" when it is `Queued` or `Generating`. Its "start" is the `start` field. A segment is "generated" when it has a duration in the timeline.
2. The synthesis front `G` is the `Timeline::start_of` of the first segment without a duration, or the sum of all durations when the timeline is complete.
3. "Clamp `d`" means the time `min(d, G)`. "The position of `d`" is the `TrackPosition` with the segment `Timeline::segment_at(d)` and the offset `d − Timeline::start_of(segment)`.
4. "Live job from `n`" gives no change in two cases. The timeline is complete, or the job runs with the output `Live` and a start of `n` or less. In all other cases, it gives `CancelJob` when the job runs, then `StartJob { next ticket, n, Live }`. The job becomes `Queued` with the output `Live`, the start `n` and no position.
5. "Next ticket" is the current job ticket plus one. `transition` stores it in the session. "Next timeline ticket" is the same rule for the timeline ticket.
6. "Paused or buffering" means play `Paused` when play is `Paused`, and play `Buffering` in all other cases. A seek does not start a paused track.
7. The session does not keep a device state. A `Play` after `DeviceLost` gives `Resume` by row 7, and `TrackHandle::play` opens the stream again (stage 03).

The transition table is the specification. The first row in table order that matches the state and the input applies. A pair of a state and an input that no row matches keeps the state and gives no commands.

| Row | State | Input | Next state | Commands |
|---|---|---|---|---|
| 1 | any | `DocumentChanged` | timeline `None`, job `Idle`, play `Stopped`, export `Waiting` becomes `Idle`, the next timeline ticket | `CancelJob` when the job runs, `StopPlayer` when play is not `Stopped`, `LoadTimeline { next timeline ticket }` |
| 2 | any | `Timeline(k, _)`, `k` is not the current timeline ticket | same | none |
| 3 | job does not run | `Timeline(k, t)` | timeline `Some(t)` | none |
| 4 | timeline `None` | `Play`, `SeekTo` or `SeekToSegment` | same | none |
| 5 | play `Stopped`, timeline complete | `Play` | play `Playing` | `PlayFrom(segment 0, offset 0)` |
| 6 | play `Stopped`, timeline not complete | `Play` | play `Buffering`, the job of "live job from 0" | `PlayFrom(segment 0, offset 0)`, then the commands of "live job from 0" |
| 7 | play `Paused` | `Play` | play `Playing` | `Resume` |
| 8 | play `Playing` or `Buffering` | `Pause` | play `Paused` | `Pause` |
| 9 | any | `Stop` | play `Stopped`, job `Idle` when it runs, export `Waiting` becomes `Idle` | `CancelJob` when the job runs, `StopPlayer` when play is not `Stopped` |
| 10 | play `Stopped` | `SeekTo(d)` | as rows 5 and 6, with the position of the clamped `d` in place of segment 0 | as rows 5 and 6, with that position and "live job from" its segment |
| 11 | play not `Stopped` | `SeekTo(d)` | same when the segment of the position of the clamped `d` is generated. In all other cases, play "paused or buffering" and the job of "live job from" that segment | `Seek(position of the clamped d)`, then the commands of "live job from" that segment when it is not generated |
| 12 | any | `SeekToSegment(n)`, `start_of(n)` is known | as `SeekTo(start_of(n))` | as `SeekTo(start_of(n))` |
| 13 | play `Stopped` | `SeekToSegment(n)`, `start_of(n)` is unknown | play `Buffering`, the job of "live job from n" | `PlayFrom(segment n, offset 0)`, then the commands of "live job from n" |
| 14 | play not `Stopped` | `SeekToSegment(n)`, `start_of(n)` is unknown | play "paused or buffering", the job of "live job from n" | `Seek(segment n, offset 0)`, then the commands of "live job from n" |
| 15 | any | `Job(t, _)`, `t` is not the current ticket | same | none |
| 16 | job `Queued` | `Job(t, Queued { p })` | job `Queued` with `position` `Some(p)` | none |
| 17 | job `Queued` | `Job(t, Started { timeline })` | timeline `Some(timeline)`, job `Generating` with the same output and start and no preparation | none |
| 18 | job runs | `Job(t, Downloading { .. })` or `Job(t, Loading)` | job `Generating` with `preparing` set to the new data | none |
| 19 | job runs | `Job(t, Synthesized { .. })` | job `Generating` with `preparing` `None` | none |
| 20 | job runs, export not `Waiting` | `Job(t, Finished)` | job `Idle` | none |
| 21 | job runs, export `Waiting { path }`, timeline complete | `Job(t, Finished)` | job `Idle`, export `Encoding { path, 0.0 }` | `Encode { path }` |
| 22 | job runs, export `Waiting { path }`, timeline not complete | `Job(t, Finished)` | export stays `Waiting { path }`, job `Queued` with the output `Silent`, the start 0 and no position | `StartJob { next ticket, 0, Silent }` |
| 23 | job runs | `Job(t, Failed(e))` | job `Failed { e.kind() }`, play `Stopped` when the output is `Live`, export `Waiting` becomes `Failed` | `StopPlayer` when the output is `Live` and play is not `Stopped` |
| 24 | job runs | `Job(t, Cancelled)` | job `Idle`, export `Waiting` becomes `Idle` | none |
| 25 | any | `TrackLoaded(k)` | track `Some(k)` | none |
| 26 | any | `PlayerState(k, _)`, `k` is not the current track | same | none |
| 27 | play `Buffering` | `PlayerState(k, Playing)` | play `Playing` | none |
| 28 | play `Playing` | `PlayerState(k, Buffering)` | play `Buffering` | none |
| 29 | play not `Stopped` | `PlayerState(k, Ended)` | play `Stopped` | `StopPlayer` |
| 30 | play `Playing` or `Buffering` | `DeviceLost` | play `Paused` | none |
| 31 | play `Stopped` or `Paused` | `DeviceLost` | same | none |
| 32 | export `Idle`, `Done` or `Failed`, timeline complete, job does not run | `StartExport(path)` | export `Encoding { path, 0.0 }` | `Encode { path }` |
| 33 | export `Idle`, `Done` or `Failed`, job runs | `StartExport(path)` | export `Waiting { path }` | none |
| 34 | export `Idle`, `Done` or `Failed`, timeline `None` or not complete, job does not run | `StartExport(path)` | export `Waiting { path }`, job `Queued` with the output `Silent`, the start 0 and no position | `StartJob { next ticket, 0, Silent }` |
| 35 | export `Encoding` | `Export(Progress(f))` | export `Encoding` with `fraction` `f` | none |
| 36 | export `Encoding { path, .. }` | `Export(Done(_))` | export `Done { path }` | none |
| 37 | export `Encoding` | `Export(Failed(_))` | export `Failed` | none |
| 38 | job `Failed` | `Dismiss(Notice::JobError)` | job `Idle` | none |
| 39 | export `Done` or `Failed` | `Dismiss(Notice::Export)` | export `Idle` | none |

The progress of a running job is the number of segments with a duration in the timeline, divided by the number of segments.

### Commands and job events

`AppState` runs each command with the APIs of stages 03, 05 and 06.

| Command | Calls |
|---|---|
| `LoadTimeline { ticket }` | On a background task, `Pipeline::timeline` with the text of the Studio document, its voice and its quality. Then `Input::Timeline(ticket, timeline)`. If the document has no text, or the call returns an error, log the error with `warn!` and send nothing |
| `StartJob { ticket, start_segment, output }` | First, if a `save_text` of the document waits for its debounce, run it at once and cancel the debounce task. Thus the library has the text hash of the job before `Started`. Then `Pipeline::start`. For `Live`, the outputs contain `TrackHandle::live_output` of the current track. For `Silent`, the outputs are empty. The bridge tags each event of the handle with `ticket` |
| `CancelJob` | `JobHandle::cancel`, then drop the handle |
| `PlayFrom(position)` | Build the track with `Track::new(timeline.rate(), timeline.stored(&store))` (stages 03 and 06). Drop the voice preview of stage 17 if one plays. Call `Player::load`, `TrackHandle::seek(position)` and `TrackHandle::play`. Then send `Input::TrackLoaded(handle.id())` |
| `Pause`, `Resume` | `TrackHandle::pause`, `TrackHandle::play` |
| `Seek(position)` | `TrackHandle::seek` |
| `StopPlayer` | `TrackHandle::pause`, then drop the `TrackHandle` |
| `Encode { path }` | Start the thread `antenna-export`. It calls `antenna_audio::export` and sends `ExportEvent` values |

`AppState` keeps the document id, the text hash and the `StoredVoice` of each job when it starts the job. It also does these steps for the events of the current ticket, after `transition`. The library functions are those of stage 05.

1. On `Started`, call `Library::record_segments` with the text hash and the `StoredVoice` of the job and the keys of the timeline.
2. On each `Synthesized`, call `TrackHandle::set_stored` with `SegmentStore::path` of the key and the duration when a track is loaded. Call `Library::set_progress` with the count of segments with a duration and the segment count. Rebuild the waveform peaks.
3. On `Finished`, if the timeline is complete, call `Library::record_complete`. Its arguments are the text hash and the `StoredVoice` of the job, and the sum of the durations. Call `Library::set_progress` with `None`.
4. On `Failed` and `Cancelled`, call `Library::set_progress` with `None`.

The export spec has the paths of `Timeline::paths(&store)` (stage 06) and `Timeline::rate`. It has the export format, the export rate and the loudness of the settings. It also has the destination and the tags, which are the document title, the voice and the language. The thread passes a cancel flag and a progress callback to `antenna_audio::export`. After `Done`, `AppState` calls `Library::record_export`.

### Consumed API

This stage uses these items of earlier stages. The code on `main` can use a different name or signature. In that case, use the code on `main` and record the difference in the stage report.

| Crate | Items |
|---|---|
| `antenna-core` | `Document`, `TextFormat::from_extension`, `Language`, `Quality`, `ExportFormat`, `SegmentIndex`, `VoiceId`, `VoiceDescriptor` |
| `antenna-pipeline` | `Pipeline::new`, `timeline`, `start`, `preload`, `JobSpec`, `JobHandle`, `JobEvent`, `Timeline::spans`, `key`, `duration`, `start_of`, `segment_at`, `rate`, `is_complete`, `stored`, `paths`, `JobError::kind`, `ErrorKind` |
| `antenna-audio` | `Player::start`, `load`, `set_volume`, `events`, `TrackId`, `TrackHandle::id`, `play`, `pause`, `seek`, `position`, `set_stored`, `live_output`, `Track`, `TrackPosition`, `PlayerState`, `PlayerEvent`, `Volume::new`, `PeakCache`, `PEAKS_PER_SECOND`, `export`, `ExportSpec`, `ExportRate`, `Loudness`, `EpisodeTags`, `ExportSummary`, `AudioError` |
| `antenna-library` | `Library::open_default`, `create`, `load`, `save_text`, `rename`, `set_voice`, `set_language`, `mark_opened`, `record_segments`, `record_complete`, `record_export`, `set_progress`, `recent`, `count`, `segments`, `collect_garbage`, `text_hash`, `SegmentStore::path`, `DocumentId`, `DocumentSummary`, `StoredVoice` |
| `antenna-models` | `ModelStore::open_default`, `is_installed`, `is_engine_installed` |
| `antenna-text` | `identify_language` |
| `antenna-engine-registry` | `Registry::new`, `factories`, `voices`, `voice`, `default_voice` |
| `antenna-ui` | `install`, `colors`, `duration`, `Assets`, the tokens and the components of the stage 15 component table |

### Studio screen

The layout follows `docs/design/screens/estudio.png` with the changes of `docs/architecture.md` section 1.2. These controls of the design are not in the app.

- The "Pause", "Emphasis" and "Pronunciation" toolbar items
- The undo and redo buttons and the history button
- The interpretation sliders and the playback speed control
- The word-level highlight

1. **Top bar.** The `Breadcrumb` component shows "Library" and the document title. "Library" opens the Library screen. A click on the title opens the `TextField` component. Enter or a click outside sends `Action::Rename`, and `AppState` calls `Library::rename`. Escape, or a title that `Library::rename` rejects, restores the old title. On the right are the inspector toggle and the Export button.
2. **Toolbar row.** The job status line is at the left edge, and the character count is at the right edge. Both use the `DATA_SMALL` type in `TEXT_DATA`. The status line is the only place that shows the job state before the audio plays (derived). It shows one text for each job state.
   1. `Queued` with a position shows "Waiting, 2 jobs before this one". `Queued` without a position shows "Waiting".
   2. `Generating` with `Downloading` shows "Downloading the voice, 37%", with the percent of `done_bytes` and `total_bytes`.
   3. `Generating` with `Loading` shows "Loading the voice".
   4. `Generating` without a preparation shows the job progress as "Generating, 40%".
   5. `Failed` shows the message of the `ErrorKind` in `WARNING_TEXT` and a `Link` "Dismiss" that sends `Action::Dismiss(Notice::JobError)`.
   6. `Idle` shows no text.
3. **Editing view.** The view is the `TextArea` component. A `.txt` or `.md` file dropped on the window sends `Action::OpenFile` (`on_drop::<ExternalPaths>`). The editing view is active when the job does not run and play is `Stopped`.
4. **Reading view.** The view is active in all other states. It is a GPUI `list` with one item for each paragraph. Each item is an `InteractiveText` that wraps a `StyledText::with_highlights`.
   1. The current segment is the `current_segment` of the player view. It has the `SIGNAL_TINT` background with the `RADIUS_HINT` radius. Text of segments without a duration in the timeline has the `READING_PENDING` color. Text of the other segments has the `READING_DONE` color.
   2. When a segment gets a duration, its color changes from `READING_PENDING` to `READING_DONE` in `DURATION_REVEAL` with a linear curve.
   3. The synthesis front is a dot in `SIGNAL` after the last generated segment while the job runs. It blinks with `CARET_PERIOD`.
   4. The gutter shows a `TimestampPill` only for a generated paragraph. The time is `Timeline::start_of` of the first segment of the paragraph. The pill of the current paragraph is in the current state.
   5. A click on a sentence sends `Action::SeekToSegment(index)`.
   6. The view keeps `SharedString` slices for each paragraph and rebuilds them only when the document changes.
5. **Inspector.** The panel is on the right edge and the inspector toggle shows or hides it. It contains these items from top to bottom.
   1. The `SectionLabel` "Voice" with an "Explore" link to the Voices screen
   2. The voice card with the `AVATAR_INSPECTOR` avatar, the name in `VOICE_NAME` and the description in `SMALL`
   3. A row of `AvatarButton` components for the other voices of the language. A click sends `Action::SetVoice`
   4. The language `Select` and the quality `Select` (Fast, Balanced, Max)
   5. The export format `Segmented` (MP3, WAV, OGG)
   6. The loudness normalization `Toggle` (compact) with the text "-16 LUFS"
6. **Player bar.** It contains these items from left to right.
   1. The `AVATAR_PLAYER` avatar and the voice name, with the language name and the engine name below
   2. The skip back button, the play and pause button and the skip forward button
   3. The elapsed time, the waveform scrubber and the total time, in the `DATA` type
   4. The volume `IconButton`. It opens a `Popover` of `LAYOUT_VOLUME_POPOVER` with a `Slider` that sends `Action::SetVolume`
7. **Waveform scrubber.** It is the `Waveform` component of `antenna-ui` with the `WaveformData` of the player view. A click or a drag sends `Action::SeekTo`. The play button is disabled while the session timeline is `None`.

### Owners of the document values

1. The library is the owner of the title, the text, the language and the voice of a document. `StudioDocument` is a copy for the views. `AppState` calls the library function first and changes `StudioDocument` only when the function returns `Ok`. When the function returns an error, `AppState` logs it with `warn!` and keeps the old values.
2. The voice and the quality of a document are its `StoredVoice` in the library. The inspector changes them with `Library::set_voice`.
3. The language of a document is its `language` in the library.
4. `Settings.voices` and `Settings.quality` are only the defaults for a new document.
5. The export format, the export rate and the loudness are settings of the app. The inspector changes `Settings.export_format` and `Settings.loudness`.

### Play position and view data

1. `state/view.rs` has the pure functions `estimated_start(timeline: &Timeline, segment: SegmentIndex) -> Duration` and `estimated_total(timeline: &Timeline) -> Duration`. `estimated_start` adds the durations of the segments before `segment`. For each of these segments without a duration, it adds the byte length of its source range divided by the speech rate. `estimated_total` uses the same rule for all segments. The speech rate is the mean bytes for each second of the known segments. When no segment has a duration, it uses `FALLBACK_BYTES_PER_SECOND`, which is 15.
2. The player view has the play state, the current segment, the elapsed time, `G`, the total time and the `WaveformData`. The elapsed time is the start of the current segment plus the offset of `TrackHandle::position`. The start is `Timeline::start_of` of the segment, or `estimated_start` when `start_of` returns `None`. The total time is the sum of the durations when the timeline is complete, and `estimated_total` in all other cases.
3. `state/view.rs` has the pure function `skip_target(elapsed: Duration, front: Duration, direction: SkipDirection) -> Option<Duration>`. `Back` gives the elapsed time minus 10 s, and the subtraction saturates at zero. `Forward` gives the elapsed time plus 10 s. It gives `None` when the elapsed time is `G` or more. In that case the clamp of the session moves the play position back, and a forward skip must not do that.
4. `state/view.rs` has the pure function `status_line(job: &JobState, progress: Option<(u32, u32)>, strings: &Strings) -> StatusLine`. `StatusLine` has the text and the tone `Data` or `Warning`. The toolbar row renders it.
5. `AppState` builds the peaks of the `WaveformData` from `PeakCache::peaks` of each segment of the generated part, in sequence. It rebuilds them on each `Synthesized` event and on each `Input::Timeline`.

### Animation frames

1. When play is `Playing`, the reading view and the player bar call `window.request_animation_frame()` in their render functions. The reading view calls `cx.notify()` only when the current segment changes, and scrolls the list to keep the current segment visible.
2. A component animation of `antenna-ui` requests frames only while it runs.
3. While the job runs and the reading view is visible, a GPUI timer of `CARET_PERIOD / 2` toggles the synthesis front dot and calls `cx.notify()`. The timer stops when the job stops. It does not run when "Reduce motion" is on.
4. In all other states, no view requests animation frames. Thus the app uses no CPU when nothing plays, no job runs and no animation runs.

### Sidebar

1. The `Logo` and the word mark "antenna" in `BRAND_SIDEBAR` are at the top, with the collapse `IconButton`. When collapsed, the sidebar shows only the icons and the count badges are hidden.
2. The `NavItem` components are Studio, Library, Voices and Models. Library shows `Library::count`. Voices shows the number of voices for the language of the Studio document. Models shows the number of registered engines for which `ModelStore::is_engine_installed` is true at `Settings.quality`.
3. The "Recent" `SectionLabel` and one `RecentRow` for each document of `Library::recent`, with its duration. A click sends `Action::OpenDocument`. `AppState` opens the document in the Studio and calls `Library::mark_opened`.
4. At the bottom are the "What's new" entry and the Settings entry. In this stage, "What's new" has no badge and opens nothing. Stage 18 connects it.

### Documents

```rust
pub(crate) struct StudioDocument {
    id: Option<DocumentId>,              // None for a draft that is not saved
    title: String,
    text: String,
    format: TextFormat,
    language: Option<Language>,
    language_source: LanguageSource,     // Identified or User
    voice: StoredVoice,
}
pub(crate) fn title_from_text(text: &str, untitled: &'static str) -> String;
```

1. At start, the Studio opens the first document of `Library::recent`. If the library is empty, the Studio opens a new draft. Stage 18 adds the setting that changes this.
2. A new draft has no id, an empty text, the format `Markdown` and the voice and quality of the settings for the interface language.
3. A draft without an id is saved when its text gets the first character that is not whitespace. Then `AppState` calls `Library::create` with the title of `title_from_text`, and `Library::set_voice`. The draft gets the id.
4. `title_from_text` takes the first line that has a letter or a digit after the removal of the leading `#` characters and the whitespace. It cuts the line at the last word boundary before 60 characters. If no line has a letter or a digit, it returns `untitled`, which is the localized text "Untitled".
5. Each change of the text of a document with an id starts a 500 ms debounce task. After the delay, `AppState` calls `Library::save_text`.
6. `cmd-n` sends `Action::NewDocument`, which opens a new draft.
7. `Action::OpenFile` reads the file, calls `Library::create` with the file name without its extension as the title, and opens the document. The format comes from `TextFormat::from_extension`.
8. When the text changes and `language_source` is `Identified`, `AppState` starts a 300 ms debounce task. It calls `antenna_text::identify_language`. A selection in the language `Select` sets `language_source` to `User`.
9. A new language can come from identification or from the language `Select`. It also sets the voice of the document to `Settings::voice_for` of that language, because the voice sets the language of a job. `AppState` calls `Library::set_language` and `Library::set_voice`. A result of identification that is the current language changes nothing. `Action::SetVoice` with a voice of another language is the exception. It sets the language of that voice and keeps that voice, as stage 17 defines.
10. A preload needs three conditions. The document has text, the language is known, and `ModelStore::is_installed` is true for the voice and the quality of the document. Then `AppState` calls `Pipeline::preload` with that voice and quality. A preload never downloads, because a download in the single job worker delays the Studio job with no progress on the status line. `AppState` keeps the returned `JobHandle`, because a drop of the handle cancels the preload. It replaces the handle only when the voice or the quality changes. The bridge reads the events of the preload handle. The preload runs until its `Finished`, `Failed` or `Cancelled` event.
11. A change of the text, the voice, the quality or the language, and the open of another document, send `Input::DocumentChanged`.

### Export

1. The Export button and `cmd-e` open the native save panel with `cx.prompt_for_new_path(dir, Some(name))`. The directory is `Settings.export_dir`. The suggested name is the document title with the extension of the export format. The result sends `Action::StartExport(path)`.
2. While the export is `Waiting` or `Encoding`, the Export button is in the progress state. A `Waiting` export shows the job progress. An `Encoding` export shows its `fraction`.
3. When the export is `Done`, a `Toast` shows the file name and the action "Show in Finder" (`cx.reveal_path`). When the toast hides, the Studio sends `Action::Dismiss(Notice::Export)`.
4. When the export is `Failed`, a `Toast` shows the export error message. `AppState` logs the `AudioError` with `error!`. When the toast hides, the Studio sends `Action::Dismiss(Notice::Export)`.

### Settings in this stage

```rust
pub(crate) struct Settings {
    pub(crate) voices: BTreeMap<Language, &'static VoiceDescriptor>,   // the default voice of each language for a new document
    pub(crate) quality: Quality,                                        // the default quality for a new document
    pub(crate) export_format: ExportFormat,
    pub(crate) export_rate: ExportRate,
    pub(crate) loudness: Loudness,
    pub(crate) export_dir: PathBuf,
    pub(crate) inspector: Panel,                                        // Shown or Hidden
    pub(crate) sidebar: Panel,
}
impl Settings {
    pub(crate) fn load(path: &Path, registry: &Registry) -> Settings;  // the defaults when the file is missing or not valid
    pub(crate) fn save(&self, path: &Path) -> Result<(), SettingsError>;
    pub(crate) fn voice_for(&self, language: Language, registry: &Registry) -> &'static VoiceDescriptor;
}
```

1. `settings/file.rs` has `SettingsFile`, the `serde` form of the TOML file. Its fields are text. `Settings::load` converts each field with the `FromStr` of its type. The types of `antenna-core` and the strum text forms of `ExportRate` (`24000`, `44100`, `48000`) and `Loudness` (`normalize`, `keep`) of stage 03 give these functions. `file.rs` has no `match` on these texts. Voices are the `Display` text of a `VoiceId`, and `Registry::voice` converts them.
2. A field that the file does not contain, or a value that does not convert, gets its default. `load` logs one `warn!` event for each value that does not convert.
3. The file has a `version` field with the value of `SETTINGS_VERSION`, which is 1.
4. The file is `settings.toml` in the config directory of `directories::ProjectDirs` with the constants of `antenna_core::app_dirs`.
5. The default `export_dir` is the folder `Antenna` in the audio directory of `directories::UserDirs`. The default quality is `Balanced`, the format `Mp3`, the rate `Hz44100`, the loudness `Normalize` and both panels `Shown`.
6. `save` writes to a temporary file in the same directory and then renames it.
7. `voice_for` returns the voice of `voices` for the language, or `Registry::default_voice`.
8. Stages 17 and 18 add fields with the same method.

### Strings

`strings/mod.rs` defines a `Strings` struct with one field for each text of the UI. `strings/en.rs` and `strings/es.rs` each define one `const` value of `Strings`. Thus a missing translation is a compile error. In this stage the locale is Spanish when `sys_locale::get_locale()` starts with `es`, and English in all other cases. Stage 18 adds the setting.

### Keyboard shortcuts

| Keys | Action | Condition |
|---|---|---|
| `space` | `Action::Pause` when play is `Playing` or `Buffering`, `Action::Play` in all other cases | The Studio is visible and no text input has focus |
| `escape` | `Action::Stop` | Play is not `Stopped` or the job runs, and the title field has no focus |
| `cmd-n` | `Action::NewDocument` | Always |
| `cmd-e` | Open the save panel of the export | The Studio document has text |
| `cmd-alt-i` | `Action::ToggleInspector` | The Studio is visible |

Stage 17 adds `cmd-k`.

### Startup measurement

When the environment variable `ANTENNA_STARTUP_PROBE=1` is set, the app logs `info!(startup_ms, "editor ready")` after the first frame in which the editor can take focus, and then quits. The value is the time from the first line of `main` to that frame. The environment variable `ANTENNA_DATA_DIR` replaces the data directory of the library, so the measurement can use a seeded library. The model store stays in its directory.

## Tasks

1. Add `apps/desktop` to the workspace `members`. Add `sys-locale` and `directories` to `[workspace.dependencies]` if they are not there.
2. Write `state/session/`. `mod.rs` has `Session`, `Input`, `Command` and `transition`. Each input group file has the rows of its inputs and one test for each of these rows in its `tests` module.
3. Write `settings/mod.rs` and `settings/file.rs` and their tests. Use a temporary directory in the tests.
4. Write `strings/mod.rs`, `strings/en.rs` and `strings/es.rs`. Write one message for each `ErrorKind` of `antenna-pipeline` and one message for a failed export.
5. Write `state/studio.rs` with `title_from_text` and its tests, and `state/view.rs` with `estimated_start`, `estimated_total`, `skip_target`, `status_line` and their tests.
6. Write `state/screen.rs` and `state/mod.rs`. `AppState::dispatch` calls `transition` and runs the commands.
7. Write `state/bridge.rs` with one task for each event channel. Each task ends when its channel closes or when the entity no longer exists.
8. Write `main.rs`. Build the registry, the library and the player. Call `ModelStore::open_default` two times, because `ModelStore` is not `Clone`. The pipeline takes one store with a clone of `Library::segments`, and `AppState` keeps the other. Start the thread `antenna-gc` that calls `Library::collect_garbage` one time. Load the settings. Create the application with `antenna_ui::Assets` and call `antenna_ui::install` with `Appearance::Light`. Open one window of `LAYOUT_WINDOW` with the minimum size `LAYOUT_WINDOW_MIN`. Bind the keyboard shortcuts.
9. Write `shell/mod.rs` and `shell/sidebar.rs`.
10. Write the Studio views in the sequence `editor.rs`, `player_bar.rs`, `reading.rs`, `inspector.rs`, `top_bar.rs` and `studio/mod.rs`. Use only the components and tokens of `antenna-ui`.
11. Write `examples/seed_library.rs`. It takes a data directory and a count, and it writes that number of documents with 2 KB of text each.
12. Add the startup probe.
13. Run the app with `cargo run -p antenna-desktop --features engine-fake`. Do the manual QA script below.
14. Measure the budgets of the acceptance criteria. Write the results in the stage report.
15. Run `cargo xtask check`. Fix each failure.
16. Do the quality gate in `docs/standards.md` section 20.

### Manual QA script

Do these steps with `--features engine-fake` in a release build and a new data directory. Write the result of each step in the stage report.

1. Start the app. The Studio shows a new draft, and the editor has focus.
2. Paste 3 paragraphs of Spanish text. In less than 1 s, the language select shows Spanish. The sidebar "Recent" list shows the title of the first line.
3. Click outside the editor and press `space`. Audio starts. The reading view shows the first sentence with the highlight, and the text that is not generated has the pending color.
4. Watch for 10 s. The highlight moves to each sentence when its audio starts. A timestamp pill appears next to a paragraph when the paragraph is generated.
5. Click the fourth sentence. Audio continues from the fourth sentence in less than 1 s.
6. Drag the scrubber back to the start. Audio continues from the start in less than 100 ms.
7. Press `space` two times. The audio pauses and then continues.
8. Press `escape`. The audio stops and the editing view returns.
9. Rename the document in the top bar. The sidebar "Recent" list shows the new title.
10. Press `cmd-e` and save as OGG. The Export button shows progress, then a toast with the file name. Click "Show in Finder". Finder shows the file.
11. Open the file in QuickTime Player. It plays the tones of the `fake` engine.
12. Press `cmd-alt-i` two times. The inspector hides and shows.
13. Press `cmd-n` and type "Second document". The sidebar "Recent" list shows "Second document" and the renamed document.
14. Quit and start the app again. The Studio opens "Second document", and the inspector is visible.

## Acceptance criteria

| ID | Criterion | Check |
|---|---|---|
| AC-16-01 | Each row of the transition table gives the specified state and commands | One test for each row in the file of its input group, named `transition_gives_<result>_when_<state>_gets_<input>`. The stage report lists the test name of each row |
| AC-16-02 | Events of an old job do not change the session | Test `transition_ignores_event_when_ticket_is_old` |
| AC-16-03 | A seek to a generated position does not cancel the job | Test `transition_seek_keeps_job_when_position_is_generated` |
| AC-16-04 | A seek to a segment that is not generated restarts the job at its segment | Test `transition_seek_restarts_job_when_segment_is_pending` |
| AC-16-05 | An export of a complete document encodes without a new job | Test `transition_export_encodes_when_timeline_is_complete` |
| AC-16-06 | The app converts times only with `Timeline::segment_at` and `Timeline::start_of` | `rg -e partition_point -e binary_search apps/desktop/src` returns no lines |
| AC-16-07 | Settings survive a save and a load | Test `settings_round_trip_when_saved` |
| AC-16-08 | A corrupt or missing settings file gives the defaults | Tests `settings_load_gives_defaults_when_file_is_corrupt` and `settings_load_gives_defaults_when_file_is_missing` |
| AC-16-09 | A settings file without a field gives the default of that field | Test `settings_load_gives_field_default_when_field_is_missing` |
| AC-16-10 | A saved voice that the registry does not have gives the default voice | Test `settings_voice_for_gives_default_when_voice_is_unknown` |
| AC-16-11 | The English and Spanish strings are complete | The crate compiles, because each locale is a `const` value of `Strings` |
| AC-16-12 | No raw design value is in `apps/desktop/src` | `cargo xtask lint-repo` passes |
| AC-16-13 | The app uses only the components of `antenna-ui` | `rg "gpui_component" apps/desktop` returns no lines |
| AC-16-14 | The editor accepts input in 300 ms or less after start, with 1000 documents in the library **(manual)** | On the Apple M5 Pro 24 GB, run `cargo build --release -p antenna-desktop --features engine-fake --bins --examples`, then `target/release/examples/seed_library "$DIR" 1000`, then `hyperfine --warmup 3 --runs 20 --show-output "ANTENNA_DATA_DIR=$DIR ANTENNA_STARTUP_PROBE=1 target/release/antenna-desktop"`. The mean is 300 ms or less, and each logged `startup_ms` is 300 or less |
| AC-16-15 | The app uses no CPU when no job runs and nothing plays **(manual)** | Start the app, paste 1 KB of text, click the sidebar outside the editor so that no caret blinks, and wait 5 s. Run `ps -o cputime= -p $PID`, wait 10 s, and run it again. The two values are equal |
| AC-16-16 | The app uses no CPU after a playback ends **(manual)** | Play a document to its end, wait 5 s, then do the check of AC-16-15. The two values are equal |
| AC-16-17 | Scroll and typing with a 1 MB document run at 120 fps **(manual)** | Make the file with `for i in $(seq 30); do cat LICENSE; done > /tmp/one-megabyte.md`. Open it on a 120 Hz display. Record 10 s of scroll and 10 s of typing with the Instruments "Animation Hitches" template. The hitch count is 0 |
| AC-16-18 | The reading view with the 1 MB document runs at 120 fps **(manual)** | Start playback of the same file and record 10 s with "Animation Hitches". The hitch count is 0 |
| AC-16-19 | The Studio layout matches the design **(manual)** | Take a screenshot of the Studio at `LAYOUT_WINDOW` with the data of the QA script. Open it and `docs/design/screens/estudio.png` in Preview at the same scale. For each control in scope, measure the height, the padding and the gap with the rectangle selection. Each value is in 1 px of the design. Each color that Digital Color Meter reads is the token value. The stage report contains both images and the list of measured controls |
| AC-16-20 | The manual QA script passes **(manual)** | The stage report lists the result of each of the 14 steps |
| AC-16-21 | Screens and the shell do not call the player, the library, the model store or the pipeline, and do not name `Input` | `rg -e antenna_audio:: -e antenna_library:: -e antenna_models:: -e antenna_pipeline:: -e Input:: apps/desktop/src/screens apps/desktop/src/shell` returns no lines |
| AC-16-22 | The app compiles and its tests pass on macOS, Linux and Windows | CI is green on the three jobs |
| AC-16-23 | Each state and input pair gives a result without a panic | `proptest` test `transition_is_total_when_inputs_are_random` |
| AC-16-24 | The title of a new document comes from its first line | Tests `title_from_text_strips_heading_marks_when_line_is_heading`, `title_from_text_cuts_at_word_when_line_is_long` and `title_from_text_gives_untitled_when_text_has_no_word` |
| AC-16-25 | A draft is saved in the library only after its first character that is not whitespace | Test `draft_creates_document_when_text_gets_first_character` with a temporary library |
| AC-16-26 | The time estimates use the mean rate of the known segments | Tests `estimated_total_uses_known_rate_when_some_durations_are_known`, `estimated_total_uses_fallback_when_no_duration_is_known` and `estimated_start_estimates_gap_when_earlier_segment_has_no_duration` |
| AC-16-27 | A timeline of an old document does not change the session | Test `transition_ignores_timeline_when_ticket_is_old` |
| AC-16-28 | The first matching row applies when two rows match | Test `transition_ignores_seek_when_timeline_is_none_and_segment_is_unknown` |
| AC-16-29 | An export that waits for a job that started after segment 0 starts a job from segment 0 | Test `transition_restarts_job_from_start_when_export_waits_and_timeline_is_partial` |
| AC-16-30 | `AppState` records the job in the library | Tests `app_state_records_segments_when_job_starts`, `app_state_sets_stored_when_segment_is_synthesized`, `app_state_records_complete_when_job_finishes` and `app_state_records_export_when_export_is_done`, with `gpui::TestAppContext`, a temporary library and the `fake` engine |
| AC-16-31 | A play after a lost device resumes the track, and `TrackHandle::play` opens the stream | Test `transition_resumes_when_play_follows_device_lost` |
| AC-16-32 | A language from identification sets the voice of that language | Test `app_state_sets_voice_when_identified_language_changes`, with `gpui::TestAppContext` |
| AC-16-33 | A library error keeps the old values of the Studio document | Test `app_state_keeps_title_when_rename_fails`, with `gpui::TestAppContext` |
| AC-16-34 | The status line shows a text for each job state | Test `status_line_text_matches_when_job_state_varies`, one case for each item of the toolbar row deliverable |
| AC-16-35 | A seek to a segment that the running job does not generate starts a live job at that segment | Tests `transition_starts_job_when_seek_target_is_before_job_start` and `transition_keeps_job_when_seek_target_is_after_live_job_start` |
| AC-16-36 | The library has the text of a job before the job starts | Test `app_state_saves_text_when_job_starts_before_debounce`, with `gpui::TestAppContext` and a temporary library |
| AC-16-37 | A preload runs only for an installed voice | Test `app_state_skips_preload_when_voice_is_not_installed`, with `gpui::TestAppContext` and an empty model store |
| AC-16-38 | A forward skip never moves the play position back | Tests `skip_target_is_none_when_forward_and_elapsed_reaches_front` and `skip_target_saturates_when_back_from_start` |

## Decision rules

1. If a screen needs a component or a token that stage 15 does not have, write a "Blocked" section. Do not use `gpui_component` in the app and do not write a raw value.
2. If the GPUI build on macOS fails because `xcrun metal` is missing, install the Metal toolchain with `xcodebuild -downloadComponent MetalToolchain`. Do not change GPUI features.
3. If `InteractiveText` with one item for each paragraph does not meet AC-16-18, render only the paragraphs in the visible range of the list. Do not move the highlight logic into the `TextArea`.
4. If `sys_locale::get_locale()` returns `None`, use English.
5. If `hyperfine` is not installed, install it with `brew install hyperfine`.
6. If the save panel returns an error on Linux, show the export error message. Linux packages are out of scope, so do not add a fallback dialog.
7. A GPUI function of this file, for example `prompt_for_new_path`, can have a different name in `gpui-pre =0.3.8`. In that case, use the function with the same behavior and record it in the stage report. If no function has the behavior, write a "Blocked" section.
8. If a `Timeline` or `Track` function of the "Consumed API" table is missing in the code on `main`, write a "Blocked" section. Do not build the track or convert times in the app.

## Out of scope

1. The Library, Voices and Models screens. Stage 17 does them.
2. The Settings screen, the interface language setting, appearance, pronunciation review, updates and the "What's new" panel. Stage 18 does them.
3. The design tokens, the fonts and the Flow components. Stage 15 does them.
4. App bundle, signing and the QA with real engines. Stage 19 does them.
5. The features `engine-qwen3` and `engine-magpie`. Stage 19 adds them.
6. The controls that `docs/architecture.md` section 1.2 lists as out of scope.
7. More than one window, and menus other than the default app menu.
