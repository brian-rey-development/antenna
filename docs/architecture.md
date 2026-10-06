# Antenna Architecture

| | |
|---|---|
| Status | Approved for implementation |
| Date | 2026-10-06 |
| Scope | MVP architecture, contracts, budgets, quality measurement |
| Writing standard | STE-80, see `docs/writing.md` |
| Visual design | `docs/design/`, see section 12 |

## 1. Product

Antenna is a desktop app that converts documents into spoken audio with local TTS models. The app keeps a library of documents. It plays a document while it generates the audio, and it exports the result as an episode file. After the first download of a model, all operations are offline.

### 1.1 MVP scope

The app has five screens. The visual design of each screen is in `docs/design/screens/`.

1. **Studio.** The user writes, pastes or drops a `.txt` or `.md` file. Antenna finds the language. The inspector shows the voice, the language, the quality, the export format and the loudness normalization. The player plays the document while synthesis continues. The current sentence has a highlight, and text that is not generated yet has the `READING_PENDING` color. A click on a sentence starts playback from that sentence. The waveform in the player shows the generated audio and lets the user seek.
2. **Library.** Antenna saves each document with its text, its voice and its generated audio. The library lists the documents by period, with the status, the duration and the language. The user can search, filter by status and play a document from the list. The sidebar shows the four most recent documents.
3. **Voices.** A gallery of the voices for one language. Each voice has a generated avatar, a name, a description, its engine and a preview button.
4. **Models.** The installed and available engines, with the languages, the size on disk and the license. The user can download, cancel, remove and select the default engine. The screen shows the acceleration device, the memory in use and the disk use.
5. **Settings.** The screen has these groups.
   1. General has the interface language (English or Spanish), the appearance (light, dark or system) and the document to open at start.
   2. Generation has the default voice, the quality and pronunciation review.
   3. Audio has the default export format, loudness normalization, the export sample rate and the export folder.
   4. Shortcuts has the list of keyboard shortcuts.
   5. Updates has automatic updates with a stable or beta channel.
   6. About has the version, the license and the links.

The sidebar has a "What's new" item with the release notes of the installed version and of an available update.

Antenna supports English, Spanish, Portuguese, French, Italian and German. The release target is macOS on Apple Silicon. CI compiles and tests Linux and Windows on each push to `main`.

### 1.2 Out of scope for the MVP

These items are in the visual design or in the product idea, but they are not in the MVP. Section 15 gives the extension point for each item. The UI does not show a control for an item that is out of scope.

1. Pause, emphasis and pronunciation marks in the editor
2. Interpretation controls (stability, expressivity, speed)
3. Synthesis while the user types
4. Playback speed other than 1.0×
5. Word-level highlight during playback
6. Version history of a document
7. Voice cloning and more than one speaker in a document
8. PDF, EPUB and URL import
9. Linux and Windows packages

### 1.3 License

The code is GPL-3.0-or-later. The About section says so. Model weights keep their licenses. Antenna downloads the weights at runtime and shows each license and attribution in the Models screen and in the About section.

## 2. Principles

1. **Headless core.** All behavior is in library crates. The apps create objects, connect them and show the state.
2. **Dependencies point inward.** No library crate depends on an app crate. Tensor types stay in the engine crates, `antenna-ml` and `antenna-review`.
3. **Generation and playback are separate.** Synthesis writes each segment to the segment store. Playback, seek and export read from the store. A live job also sends its chunks to the player, so that playback starts before the first segment is complete.
4. **One audio format.** Inside Antenna, audio is mono `f32` PCM at the sample rate of the engine. The segment store keeps 16-bit PCM. Three places convert audio. The player resamples to the device rate, review resamples to 16 kHz and export encodes the export format.
5. **Capabilities are data.** Each engine declares its languages, voices, quality variants, limits and files in a static descriptor.
6. **One runtime.** All models run on `candle`. Shared inference code is in `antenna-ml`.
7. **Threads for compute.** Synthesis, playback feed, review and downloads run on dedicated threads. The only async code is the GPUI executor in the desktop app.
8. **Measured quality.** Each engine has numeric quality and speed targets. `antenna-eval` measures them. The nightly CI job runs the parity tests, the conformance tests of the `Fast` variants and the WER check. The project owner runs the budget check on the reference machine.

## 3. Workspace

```
antenna/
├── Cargo.toml                  # workspace manifest, members listed one by one
├── Cargo.lock
├── rust-toolchain.toml
├── README.md  LICENSE  CONTRIBUTING.md  CHANGELOG.md  CLAUDE.md  ROADMAP.md
├── .rustfmt.toml  .clippy.toml  .gitignore
├── .cargo/config.toml          # the `xtask` alias
├── .config/                    # nextest.toml, deny.toml, about.toml, about.hbs
├── .github/workflows/          # ci.yml, nightly.yml, release.yml, feed.yml
├── apps/                       # binaries that Antenna ships
│   ├── desktop/                # Antenna.app: GPUI shell, with bundle/ for the macOS bundle files and benches/
│   └── cli/                    # antenna: speak, export, voices, models
├── crates/
│   ├── core/                   # antenna-core: domain types and the extension traits
│   ├── text/                   # antenna-text: Markdown to prose, segments, language identification
│   ├── audio/                  # antenna-audio: player, export encoders, loudness, resampler
│   ├── pipeline/               # antenna-pipeline: jobs, job queue, job events, engine pool
│   ├── ui/                     # antenna-ui: design tokens, fonts, icons, Flow components, gallery
│   ├── storage/
│   │   ├── models/             # antenna-models: model store, download, hash check, removal
│   │   └── library/            # antenna-library: documents, segment store, search
│   ├── inference/
│   │   ├── ml/                 # antenna-ml: candle device, weight loading, sampler, seed
│   │   └── review/             # antenna-review: Whisper transcription and word alignment
│   └── engines/
│       ├── registry/           # antenna-engine-registry: engine list and default voices
│       ├── testkit/            # antenna-engine-testkit: conformance suite and test outputs
│       ├── fake/               # antenna-engine-fake: deterministic tones
│       ├── qwen3/              # antenna-engine-qwen3: Qwen3-TTS 12Hz
│       └── magpie/             # antenna-engine-magpie: NVIDIA Magpie TTS Multilingual
├── tools/                      # developer tools that Antenna does not ship
│   ├── xtask/                  # cargo xtask check, lint-repo
│   ├── eval/                   # antenna-eval: bench and wer
│   ├── reference/<engine>/     # Python projects (uv) that write parity fixtures
│   ├── voices/qwen3/           # Python project (uv) that writes the Qwen3 voice prompts
│   ├── fonts/                  # Python project (uv) that makes the font instances
│   └── release/                # Python script that writes the update feed files
└── docs/
    ├── architecture.md  standards.md  writing.md  benchmarks.md
    ├── adding-an-engine.md  release.md
    ├── design/                 # the visual design: pages, screens, tokens.md, tokens-dark.md
    ├── release-notes/es/       # the Spanish release notes, one file for each version
    ├── adr/
    ├── plans/
    └── reports/                # one stage report for each completed stage
```

The groups under `crates/` are exactly `storage/`, `inference/` and `engines/`. Each group holds crates of one kind. Each other directory under `crates/` is one crate. `docs/standards.md` section 3.1 gives the naming rule.

Python code exists only in `tools/`. Antenna does not run Python at runtime. A developer runs the tools one time to make fixtures and voice prompts, and commits their output.

### 3.1 Dependency graph

The table lists each permitted dependency between workspace crates. A crate can depend on a crate of its row and on no other workspace crate. The names are the paths under `crates/`, `apps/` and `tools/`.

| Crate | Normal dependencies | Dev-dependencies |
|---|---|---|
| `core` | None | None |
| `ui` | None | None |
| `inference/ml` | None | None |
| `text`, `audio`, `storage/models`, `storage/library`, `engines/testkit` | `core` | None |
| `inference/review` | `inference/ml`, `audio`, `core` | `storage/models` |
| `engines/fake` | `core` | `engines/testkit` |
| `engines/qwen3`, `engines/magpie` | `inference/ml`, `core` | `engines/testkit`, `storage/models` |
| `engines/registry` | `engines/fake`, `engines/qwen3`, `engines/magpie`, `core` | None |
| `pipeline` | `text`, `storage/models`, `storage/library`, `core` | `engines/testkit`, `engines/fake` |
| `apps/cli` | `engines/registry`, `pipeline`, `text`, `storage/models`, `storage/library`, `audio`, `core` | None |
| `apps/desktop` | `ui`, `engines/registry`, `pipeline`, `text`, `storage/models`, `storage/library`, `audio`, `inference/review`, `core` | None |
| `tools/eval` | `engines/registry`, `pipeline`, `storage/models`, `storage/library`, `inference/review`, `core` | None |
| `tools/xtask` | None | None |

`inference/review` depends on `audio` for the resampler, because Whisper needs 16 kHz input. The tests of the in-house engines and of `inference/review` use the model store to find downloaded files. The `fake` engine depends only on `core`, so stage 01 can build it before `antenna-ml` exists. `ui` depends only on `gpui` and `gpui-component`, so a component cannot read app state.

This table is the only permitted graph. `cargo xtask lint-repo` compares `cargo metadata` with the same table as data in `tools/xtask/src/graph.rs`.

The two apps, `antenna-eval` and `antenna-engine-registry` are the composition root. Only the composition root names concrete engines. Section 7.5 gives the steps to add an engine.

## 4. Engines

### 4.1 Selected engines

The selection criterion is the highest measured quality in the six languages. The weight license must permit commercial use. Streaming must be faster than real time on an M5 Pro.

| | Qwen3-TTS 12Hz | Magpie TTS Multilingual |
|---|---|---|
| Engine id | `qwen3` | `magpie` |
| Role | Default engine for all six languages | Second engine |
| Weights | `Qwen/Qwen3-TTS-12Hz-1.7B-Base`, `Qwen/Qwen3-TTS-12Hz-0.6B-Base` | `nvidia/magpie_tts_multilingual_357m` (v2607 GGUF), `nvidia/nemo-nano-codec-22khz-1.89kbps-21.5fps` |
| Weight license | Apache-2.0 | NVIDIA Open Model License (commercial use permitted, attribution necessary) |
| Architecture | Qwen3 LM ("talker") with a multi-token code predictor, then the 12Hz speech tokenizer decoder | Transformer encoder and decoder with a local transformer and CFG, then the NanoCodec decoder |
| Sample rate | 24 kHz | 22.05 kHz |
| Voices | Four designed voices for each language, from stored voice prompts | Five speakers, each in all six languages |
| Published quality | WER 0.93 to 2.86% in the six languages, highest speaker similarity in the Qwen3-TTS technical report | Elo 1065 on the Artificial Analysis open-weights board |
| Rust implementation | In-house, in `crates/engines/qwen3` | In-house, in `crates/engines/magpie` |

No maintained Rust crate implements these models with local file loading, streaming and our quality standards. Thus Antenna implements both engines in-house on `candle`. The official Python implementations are the reference implementations. Community ports are reading material only. Do not copy code from a repository without a license file.

### 4.2 Quality variants

Each engine declares one variant for each `Quality` value. The user selects the quality in Settings. `Balanced` is the default.

| Quality | `qwen3` | `magpie` |
|---|---|---|
| `Fast` | 0.6B, bf16 | CFG off, f16 |
| `Balanced` | 1.7B, bf16 | CFG 2.5, f16 |
| `Max` | 1.7B, f32 | CFG 2.5, f32 |

### 4.3 Qwen3-TTS voices

The Base models make speech in the voice of a voice prompt. A voice prompt contains the reference codes, the speaker embedding and the reference transcript. The Python tool in `tools/voices/qwen3` makes the prompts with these steps.

1. The tool makes a reference clip with `Qwen/Qwen3-TTS-12Hz-1.7B-VoiceDesign` (Apache-2.0) from a fixed voice description and a fixed seed. There are four descriptions for each language, two female and two male.
2. The tool encodes the clip with the Base model encoder and the speaker encoder of each size.
3. The tool writes one file for each voice in `crates/engines/qwen3/voices/`. Each file is smaller than 64 KB.

The engine embeds these files with `include_bytes!`. Thus the Rust code does not need the speech tokenizer encoder or the speaker encoder. Only the decoder path is in Rust.

### 4.4 Review model

Pronunciation review uses Whisper Small (MIT) through `antenna-review`. The Models screen shows it as a model of the kind "review". The eval tool uses Whisper large-v3-turbo through the same crate.

### 4.5 Future engines

Kokoro-82M, Kyutai Pocket TTS, Chatterbox Multilingual and VoxCPM2 are candidates for later releases. Section 7.5 gives the procedure.

## 5. Core contracts

`antenna-core` defines the vocabulary of the system. The traits in this section are the extension points. A change to this section needs an ADR.

### 5.1 Domain types

```rust
pub enum Language { En, Es, Pt, Fr, It, De }
pub enum Quality { Fast, Balanced, Max }
pub enum ExportFormat { Mp3, Wav, Ogg }

pub struct EngineId(&'static str);
pub struct VoiceId { engine: EngineId, key: &'static str }   // displayed as "qwen3/es-lucia"
pub struct SampleRate(NonZeroU32);
pub struct SegmentIndex(u32);
pub struct TextHash([u8; 32]);                                // SHA-256 of the format and the text of a document

pub enum TextFormat { Plain, Markdown }
pub struct Document { text: Arc<str>, format: TextFormat }   // text is never blank
pub struct Segment { index: SegmentIndex, text: Box<str>, source: Range<usize> }
pub struct PcmChunk(Vec<f32>);                                // mono, at the engine sample rate

pub type Emit<'a> = &'a mut dyn FnMut(PcmChunk) -> ControlFlow<()>;

pub struct Localized { pub en: &'static str, pub es: &'static str }

pub const PCM_SCALE: f32 = 32767.0;                           // f32 sample to 16-bit PCM and back

pub mod app_dirs {                                            // the arguments of ProjectDirs::from
    pub const QUALIFIER: &str = "app";
    pub const ORGANIZATION: &str = "Antenna";
    pub const APPLICATION: &str = "Antenna";
}
```

1. `ExportFormat` is in the core because `antenna-audio` encodes it, `antenna-library` records it and the apps show it.
2. Stage 01 lists the exact derives and constants of each type. The enums with a text form use the `strum` traits `Display`, `EnumString` and `IntoStaticStr`, and their `FromStr` error is `strum::ParseError`.
3. `SampleRate::new(NonZeroU32)` is a `const fn` for constants. `SampleRate::try_from(u32)` parses a value from outside the program, for example a device rate, and returns `CoreError::ZeroSampleRate` for zero.
4. `EngineId::new` and `VoiceId::new` are `const fn`. An empty key causes a compile error in a `const` context.
5. `SegmentIndex` is the index of a segment in its document. Each API that names a segment uses it.
6. `TextHash` shows as lowercase hex. The library compares it to find a document that changed after a job.
7. The segment store writer and reader convert samples with `PCM_SCALE` and with no other value.
8. `antenna-models` and `antenna-library` get the platform data directory from `app_dirs`.

### 5.2 Engine descriptor

```rust
pub struct EngineDescriptor {
    pub id: EngineId,
    pub name: &'static str,
    pub version: &'static str,                     // part of each segment key
    pub summary: Localized,                        // one line for the Models screen
    pub license: ModelLicense,                     // name, URL, attribution text
    pub sample_rate: SampleRate,
    pub max_segment_chars: NonZeroUsize,           // the text crate makes no segment longer
    pub variants: Variants,
    pub voices: &'static [VoiceDescriptor],
}

pub struct Variants { pub fast: Variant, pub balanced: Variant, pub max: Variant }

impl Variants {
    pub const fn get(&self, quality: Quality) -> &Variant;
}

pub struct Variant {
    pub parameters: &'static str,                  // model size for the Models screen, for example "1.7B"
    pub artifacts: &'static [Artifact],            // files for all voices in this variant
}

pub struct VoiceDescriptor {
    pub id: VoiceId,
    pub language: Language,
    pub name: &'static str,                        // a person name, for example "Lucía"
    pub description: Localized,                    // for example "Warm, mid register, calm"
    pub artifacts: &'static [Artifact],            // files for this voice only
}

pub struct Artifact {
    pub key: &'static str,                         // unique in the files of one voice and variant, the key in ModelFiles
    pub repo: &'static str,                        // Hugging Face repository
    pub revision: &'static str,                    // commit SHA, 40 hex characters
    pub path: &'static str,                        // file path in the repository
    pub extent: Extent,
    pub sha256: &'static str,                      // hash of the downloaded bytes
}

pub enum Extent {
    Whole { bytes: u64 },                          // the complete file
    Range { offset: u64, bytes: u64 },             // a part of the file, for a member of an uncompressed archive
}

pub struct ModelFiles { /* artifact key -> local path */ }
```

`Variants` has one field for each `Quality`. Thus each engine declares each variant, and `Variants::get` cannot fail.

### 5.3 Engine

```rust
pub trait EngineFactory: Send + Sync + 'static {
    fn descriptor(&self) -> &'static EngineDescriptor;
    fn load(&self, voice: &VoiceDescriptor, quality: Quality, files: &ModelFiles) -> Result<Box<dyn Engine>, EngineError>;
}

pub trait Engine: Send {
    fn synthesize(&mut self, segment: &Segment, emit: Emit<'_>) -> Result<(), EngineError>;
}
```

`Engine::synthesize` obeys this contract.

1. The engine calls `emit` for each chunk, as soon as the chunk is available.
2. If `emit` returns `ControlFlow::Break`, the engine stops and returns `Ok(())` before it calls `emit` again.
3. `emit` can block. A blocked `emit` is backpressure from the outputs.
4. The engine is deterministic. The same segment, voice, quality and engine version give the same samples on the same device. The engine gets its random seed from `Seed::for_segment(voice, text, attempt)` in `antenna-ml`. The first attempt is 0.
5. The engine does not do I/O. It gets all files from `ModelFiles` in `load`.
6. The engine resets its generation state at the start of each segment.

`load` reads the weights, builds the model and runs one warm-up synthesis. It is slow, so the pipeline keeps loaded engines in the engine pool.

### 5.4 Live outputs

```rust
pub trait Output: Send {
    fn open(self: Box<Self>, rate: SampleRate) -> Result<Box<dyn Sink>, SinkError>;
}

pub trait Sink: Send {
    fn begin_segment(&mut self, index: SegmentIndex) -> Result<(), SinkError>;
    fn write(&mut self, samples: &[f32]) -> Result<(), SinkError>;
    fn finish(self: Box<Self>) -> Result<(), SinkError>;
}
```

An `Output` receives the chunks of a job while the job runs. The pipeline opens each output with the sample rate of the engine. Thus a sink cannot receive samples at an incorrect rate, and no code can write to a sink before it is open.

The pipeline itself writes each segment to the segment store. Outputs exist for live playback (`LiveOutput` in `antenna-audio`), for tests (`RecordingOutput` in `antenna-engine-testkit`) and for measurements (`TimingOutput` in `antenna-eval`).

## 6. Data flow and concurrency

```
                      ┌──────────────── antenna-library ────────────────┐
                      │ documents (text, meta)    segment store (16-bit) │
                      └───────▲──────────────────────────▲──────┬───────┘
                              │ save                     │ write│ read
JobSpec ─> queue ─> worker "antenna-job"                  │      │
                    1. text::prepare(document, language, limits) ─> Started { timeline }
                    2. for each segment:                  │      │
                         key = segment key                │      │
                         if key in store ─> Synthesized { is_cached }
                         else, at the first such segment: │      │
                           models.ensure(artifacts) ─> Downloading
                           pool.checkout(voice, quality) ─> Loading
                         engine.synthesize ─> store ──────┘      │
                              └─> live outputs (chunks) ──┐      │
                    3. ─> Finished                        ▼      ▼
                                          player feed thread "antenna-player"
                                          (track of stored, live and pending segments,
                                           resample, push into rtrb ring buffer)
                                                     │
                                          cpal callback: pop samples, count played samples
```

### 6.1 Jobs and the job queue

```rust
pub struct JobSpec {
    pub document: Document,
    pub voice: VoiceId,                            // the voice also sets the language
    pub quality: Quality,
    pub start_segment: SegmentIndex,
    pub outputs: Vec<Box<dyn Output>>,
}

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

impl Pipeline {
    pub fn timeline(&self, document: &Document, voice: VoiceId, quality: Quality) -> Result<Arc<Timeline>, JobError>;
    pub fn start(&self, spec: JobSpec) -> Result<JobHandle, JobError>;
    pub fn preload(&self, voice: VoiceId, quality: Quality) -> Result<JobHandle, JobError>;
}
```

1. The pipeline has one job queue and one worker. A job waits in the queue while another job runs, because two models on one GPU are slower than one model. The queue is first in, first out.
2. `Started` comes first, before any download or load. Thus the UI shows the pending text at once. `Downloading` and `Loading` occur only when a segment needs synthesis.
3. `JobEvent::Finished` comes after each segment is in the store and each sink returns from `finish`.
4. `Pipeline::timeline` calculates the segments, the keys and the stored durations of a document without a job. The apps use it to build the player track before they start a job.
5. `Pipeline::preload(voice, quality)` downloads the files of a voice and puts a loaded engine in the pool. It does not start a document job.
6. The `Timeline` has one `OnceLock<Duration>` for the duration of each segment. The worker sets each value one time. The UI reads the values without a lock.

### 6.2 Segment store

The segment store keeps the audio of each segment in a file with the name `<key>.wav`. The key is the SHA-256 of the engine id, the engine version, the voice id, the quality and the segment text. The file is 16-bit mono PCM at the engine sample rate.

1. The worker writes a segment to a temporary file and renames it when the segment is complete. Thus a reader never sees a partial file.
2. If the job is cancelled or a sink fails while the engine synthesizes a segment, the worker deletes the temporary file of that segment. Then it sends `Cancelled` or fails the job. A partial segment never gets a key.
3. Two documents with the same sentence and voice share one file.
4. When the user edits a document, only the changed segments need synthesis.
5. At start, a background thread deletes each segment file that no document uses and that is older than 1 h. The age limit protects the segments of a job that runs now, also in another process. The garbage collection takes the current time as a parameter.

### 6.3 Player

The player in `antenna-audio` plays a `Track`. A track is the list of the segments of a document. In a `Track`, each segment is `Stored` (a file in the segment store) or `Pending` (not generated yet). The feed thread keeps one more state. A `Pending` segment that gets chunks from a running job through `LiveOutput` is `Live` until the job stores the segment.

1. The feed thread owns the track. It reads samples at the play position, resamples them to the device rate and pushes them into the `rtrb` ring buffer.
2. The cpal callback pops samples from the ring buffer and adds the number of played samples to an `AtomicU64`. It does not lock, allocate or do I/O.
3. When the play position reaches a `Pending` segment, the player waits for that segment. The UI shows a buffer state. When the segment becomes `Live` or `Stored`, playback continues.
4. A seek clears the ring buffer and moves the play position. A seek to a stored position starts to play in 100 ms or less.
5. Pause stops the callback reads. Playback continues from the same sample.
6. If the default output device changes, cpal moves the stream to the new device. If the device is lost, the player pauses and sends a device event. The next `TrackHandle::play` opens the stream again. The player has no separate reopen command.

### 6.4 Highlight of the current sentence

`AppState` reads `TrackHandle::position` one time in each animation frame, and the screens read the position from `AppState`. The current segment is `TrackPosition::segment`, so the highlight needs no search. `AppState` converts a time of the scrubber into a segment with `Timeline::segment_at`, a binary search. When nothing plays, the UI does no work.

### 6.5 Export

Export reads the segments of a document from the store. If a segment is not in the store, the export first runs a job for the document. Then export does these steps in sequence.

1. Read all segments and join them.
2. If loudness normalization is on, measure the integrated loudness with EBU R128. Apply one gain to get -16 LUFS. If the true peak is then more than -1 dBTP, decrease the gain to a true peak of -1 dBTP.
3. Resample to the encoder rate. For MP3 and WAV, the encoder rate is the export sample rate (24, 44.1 or 48 kHz). Opus has no 44.1 kHz mode, so an Ogg export at 44.1 kHz uses 48 kHz.
4. Encode as MP3 (64 kbps CBR, mono), WAV (16-bit PCM, mono) or Ogg Opus (48 kbps, mono).
5. Write the file to a temporary path and rename it to the destination.

### 6.6 Pronunciation review

If review is on, the review thread "antenna-review" transcribes each new segment with Whisper Small. It aligns the transcript with the segment text and finds the words that do not match. The UI underlines those words. The review thread runs only when no job runs. A preview job also stops it. Thus review never decreases the synthesis speed.

### 6.7 Concurrency rules

| Data | Mechanism | Reason |
|---|---|---|
| Samples to the audio device | `rtrb` single-producer single-consumer ring buffer | The realtime thread must not lock or allocate |
| Job events, live chunks, player commands, review results | `flume` | Blocking send on worker threads, async receive in GPUI |
| Play position | `AtomicU64` | The UI reads it each frame and the callback writes it |
| Cancellation | `AtomicBool` in `JobHandle` | The emit callback reads it for each chunk |
| Library index | `RwLock` | Many reads from the UI, rare writes from the apps |

Each thread owns its state. Threads share data only through the mechanisms in this table. The job worker owns the job queue and the engine pool, and receives commands through `flume`. Thus the queue and the pool need no lock.

## 7. Models and library storage

### 7.1 Model store

`antenna-models` converts artifact declarations into `ModelFiles`.

1. Find each artifact in the model store. The model store is `<data>/models`, where `<data>` is the platform data directory from the `directories` crate. The environment variable `ANTENNA_MODEL_DIR` can replace it. `ANTENNA_DATA_DIR` does not move the model store, so a test or a measurement can use a new library with the downloaded models.
2. Calculate the total size of the missing files. If the free disk space is less than this size plus 1 GB, stop with `ModelError::DiskSpace`.
3. Download each missing artifact with `ureq` from `https://huggingface.co/<repo>/resolve/<revision>/<path>`. For an `Extent::Range` artifact, send an HTTP `Range` header. Continue a partial download with a `Range` header from the last byte on disk. Read in blocks of 64 KB. After each block, send a progress event and read the cancel flag.
4. Calculate the SHA-256 of each downloaded file. If the hash is different, delete the file and stop with `ModelError::Hash`.
5. Write a `verified` record next to the file. On the next start, the store checks only that the file exists with the expected size.
6. `ModelStore::remove_engine(descriptor, keep)` deletes the artifacts of an engine. It keeps each artifact that another engine or the review model also uses.
7. `ensure`, `missing_bytes` and the `keep` argument of `remove_engine` take `impl IntoIterator<Item = &'static Artifact>`.
8. `engine_download_bytes`, `is_engine_installed` and `keep_artifacts` of `antenna-models` are the only implementation of the download size, the installed state and the keep list of an engine. The CLI and the Models screen call them.

A failed download retries three times with exponential backoff (1 s, 4 s, 16 s). Thus a download has a maximum of four attempts. The store takes the delays as a parameter, so a test can use short delays.

### 7.2 Library

`antenna-library` keeps the documents and the segment store in the platform data directory. The environment variable `ANTENNA_DATA_DIR` can replace it.

```
<data>/library/<document-id>/document.toml   # title, format, voice, quality, segment keys, created, modified, opened, last export
<data>/library/<document-id>/text.md          # or text.txt
<data>/segments/<key prefix>/<key>.wav
```

1. The document id is a UUID v7, so the ids sort by creation time.
2. At start, the library reads all `document.toml` files into an in-memory index.
3. The status of a document comes from data.
   1. `Draft` means that not all segments of the current text are stored and no job runs.
   2. `Generating` means that a job runs for the document. The apps report its progress with `Library::set_progress`.
   3. `Ready` means that all segments are stored.
   4. `Exported` means that the last export is newer than the last change.
4. Search is case-insensitive and accent-insensitive on the title and the text.
5. The library groups documents by period. The groups are today, this week and then one group for each month.
6. Each write goes to a temporary file first and then a rename. A crash cannot corrupt a document.
7. The pipeline knows only the segment store. The apps connect a job to its document. On `Started`, they record the segment keys with `Library::record_segments`. On `Finished`, they mark the document complete with `Library::record_complete`.

### 7.3 Engine pool

The engine pool keeps loaded engines with an LRU policy and a capacity of 2. The key is the voice and the quality. A job takes an engine from the pool and returns it when the job ends. Only the job worker uses the pool.

### 7.4 Shared inference code

`antenna-ml` contains the inference code that more than one crate uses.

| Module | Content |
|---|---|
| `device` | Device selection, Metal and then CPU. The `ANTENNA_DEVICE` environment variable can force `cpu`. `Backend { Cpu, Metal }` and `backend(&Device)` let code select a `DType` without a Metal device, for example in a test on Linux |
| `weights` | Memory-mapped safetensors loading into a `VarBuilder` with the selected `DType`, and `GgufWeights` for memory-mapped GGUF files. `GgufWeights::tensor` takes `&self` and returns a `Tensor` |
| `sampler` | `SamplingConfig` (temperature, top-k, top-p, repetition penalty, with the `GREEDY` constant) and the sampler with a seeded RNG. An engine can group `SamplingConfig` values in one type, for example one configuration for each codebook. No engine defines its own sampling fields or its own sampler |
| `seed` | `Seed::for_segment(voice: impl Display, text: &str, attempt: u8)`, a stable hash of the `Display` form of the voice id, the segment text and the attempt. The engine gives its `VoiceId`, because `antenna-ml` does not depend on `antenna-core` |
| `stream` | The `Streaming` trait of a module with time state. `start` makes the state, and `forward(&Tensor, &mut State)` processes the next input. A state update makes new tensors and does not write in place, so a clone of a state stays valid. `CausalConv1d` and `CausalConvTranspose1d` implement it, with split-invariance tests at a tolerance of 1e-5. The codec of each engine builds on these types |
| `schedule` | `ChunkSchedule { first, rest }`, the number of generation steps in the first chunk and in each next chunk. `Pacing { chunks, steps_per_char, min_expected_steps }` with `step_limit(chars)`, the runaway limit with `RUNAWAY_FACTOR` 4. `pace`, the synthesis driver of section 9 item 6, gets each step through an `OnStep` callback and returns `Paced { Ended, Stopped, Runaway }`. It never calls `flush` with an empty buffer. The retry rule is private to the module. Unit tests with fake closures cover all of them |

Each module in this table has two or more users by design. Any other module goes into `antenna-ml` only when two crates use it.

### 7.5 Add an engine

1. Write a Blocked section that asks for the new crate. A human adds its row to the table in section 3.1 and to `tools/xtask/src/graph.rs`.
2. Make the crate `crates/engines/<name>` with the layout in `docs/standards.md` section 3.4.
3. Implement `EngineFactory` and `Engine`. Use the synthesis driver `antenna_ml::pace`.
4. Declare a static `EngineDescriptor` with a variant for each quality and pinned artifacts.
5. If the model is in-house, add a Python project in `tools/reference/<name>` and commit its parity fixtures.
6. Add `antenna_engine_testkit::conformance!(Factory, files)` to `tests/conformance.rs`. For an engine that needs downloaded models, write `conformance!(Factory, files, ignore = "needs models")`.
7. Add the crate and each new third-party crate of the engine to the `members` list and to `[workspace.dependencies]` of the root `Cargo.toml`.
8. Add the feature `engine-<name>` and one registration line to `antenna-engine-registry`. Gate the entries of the engine in `DEFAULT_VOICES` with the same feature. The registry has no default features.
9. Add the feature `engine-<name>` to the manifests of `apps/cli`, `apps/desktop` and `tools/eval`, and add it to their default features. Each app gets the registry with `default-features = false` from `[workspace.dependencies]`, so the app features are the only switch.
10. Add the budgets of the engine to `tools/eval/src/budgets.rs`.
11. Run `antenna-eval bench` and `antenna-eval wer` for the voices of section 8. Add the results to `docs/benchmarks.md`.

These steps list each file outside the engine crate that changes. `docs/adding-an-engine.md` gives the same steps with examples.

## 8. Performance budgets

These budgets are acceptance criteria. The reference machine is an Apple M5 Pro with 24 GB, in a release build with Metal. Synthesis budgets apply to the `Balanced` quality unless the row names another quality. The time to first audio rows apply to `Balanced` only.

`tools/eval/src/budgets.rs` contains the synthesis rows as a constant table with one entry for each `EngineId`. Its time to first audio fields are `Option<Duration>`, with `None` for `Fast` and `Max`. `antenna-eval bench --voice <id> --quality <q> --check-budgets` exits with code 0 only when the measurement meets each budget of that engine and quality. The stage that owns each other row measures it with `divan`, `hyperfine` or the steps of that stage. Unit tests do not measure time.

The voices of one engine share the weights. Thus one voice of each language represents the speed of all voices of that language.

| Metric | Budget |
|---|---|
| Time to first audio, engine in the pool | 500 ms or less (`qwen3`), 300 ms or less (`magpie`) |
| Time to first audio, weights in the page cache, engine not loaded | 4 s or less |
| Real-time factor of the default voice of each language | 0.5 or less |
| Real-time factor of one other voice of each language, and of the default voices in `Fast` | 0.8 or less |
| Real-time factor of `Max` | 0.9 or less |
| Buffer underruns during playback of the eval corpus | 0 |
| Seek to a stored position | 100 ms or less to the first sample |
| Export of 10 minutes of stored audio to MP3 with normalization | 3 s or less |
| App start to an editor that accepts input, with 1000 documents in the library | 300 ms or less |
| Library search and filter with 1000 documents | 16 ms or less |
| CPU use when no job runs and nothing plays | 0% (no polling, and no wake-up more often than one time each hour) |
| Resident memory with one `qwen3` `Balanced` engine loaded | 6 GB or less |
| Resident memory with one `qwen3` `Max` engine loaded | 10 GB or less |
| Allocations in the cpal callback | 0 |
| Editor frame rate with a 1 MB document | 120 fps during scroll and typing |

### 8.1 Techniques

1. **Short first segment.** If the first sentence has more than 60 characters, the text crate splits it at the first clause boundary. Thus the first chunk comes after a clause of 60 characters or less.
2. **Live chunks.** The player plays chunks of a running segment before the segment is in the store.
3. **Segment cache.** A segment with the same text, voice and quality is never synthesized two times.
4. **Lazy loads.** The app loads no model at start. When the Studio contains text and the language is known, the app preloads the voice in the background.
5. **Memory-mapped weights.** The second load of a model reads from the page cache.
6. **Build profiles.**

```toml
[profile.release]
lto = "fat"
codegen-units = 1

[profile.dev.package."*"]
opt-level = 3
```

## 9. Errors

1. Each library crate has one public error enum. `antenna-core` has three, `CoreError`, `EngineError` and `SinkError`, because the traits of section 5 return them.
2. An error variant keeps the underlying error as a `#[source]` field with its concrete type, for example `std::io::Error`, `hound::Error` or `toml::de::Error`. Use a `reason: &'static str` field only when no underlying error exists. The one exception is the boxed source of `EngineError::Load` and `EngineError::Inference`, because each engine has its own error type.
3. The worker runs `synthesize` in `catch_unwind`. A panic in an engine becomes `JobEvent::Failed`. The app does not stop.
4. If one segment fails, the job fails. Antenna never skips a segment without a message.
5. `JobError` has a `kind()` method that returns a small `ErrorKind` enum. The UI shows a message for each `ErrorKind`. The UI does not show the text of an internal error.
6. Each engine declares `const PACING: antenna_ml::Pacing` and keeps a copy in its engine value, so a test can change it. Each segment goes through the driver `antenna_ml::pace`. It owns the step buffer, the chunk schedule, the step limit and the `ControlFlow::Break` handling. The step limit is `self.pacing.step_limit(chars)`. Magpie also caps it with `max_decoder_steps` of its config. A generation can pass the step limit before its end token. If no chunk went out, the driver runs the segment again with attempt 1 of `Seed::for_segment`. In all other cases it returns `Paced::Runaway`, and the engine returns `EngineError::Runaway`. A retry after the first chunk is not possible, because the user already heard that audio.
7. Each engine crate has a `pub(crate)` error enum in `error.rs`, with data variants and `From<MlError>`. `pace` uses this enum. `Factory::load` and `Engine::synthesize` box it one time into `EngineError::Load` or `EngineError::Inference`.

## 10. Crates

Use only these crates. If a stage needs another crate, the agent writes a Blocked section with the reason. A human changes this table.

| Problem | Crate |
|---|---|
| Errors | `thiserror` (libraries), `anyhow` (`main` only) |
| Diagnostics | `tracing`, `tracing-subscriber` |
| Enum string conversion | `strum` |
| Channels | `flume` |
| Realtime ring buffer | `rtrb` |
| Audio device | `cpal` |
| Resampling | `rubato` |
| Loudness | `ebur128` |
| MP3 encoding | `mp3lame-encoder` (LAME, LGPL) |
| WAV encoding and segment files | `hound` |
| Opus encoding | `audiopus` (libopus, static build) |
| Ogg container | `ogg` |
| Markdown | `pulldown-cmark` |
| Sentence segmentation | `srx`, with the LanguageTool `segment.srx` rules |
| Language identification | `whatlang` |
| Accent-insensitive search | `unicode-normalization` |
| HTTP download | `ureq` (blocking, rustls) |
| Disk space and file locks | `fs4` |
| Hashes | `sha2` |
| Document ids | `uuid` (v7) |
| Dates in the library | `jiff` |
| Platform directories | `directories` |
| Documents, settings and model configs | `serde`, `toml` |
| Tensors and inference | `candle-core`, `candle-nn`, `candle-transformers` |
| Memory-mapped GGUF | `memmap2` (`antenna-ml` only) |
| Text tokenizers | `tokenizers` |
| Magpie G2P word split | `regex` |
| Device name and total memory | `sysinfo` (desktop app and `antenna-eval`) |
| Resident memory | `memory-stats` |
| System locale for the interface language | `sys-locale` |
| CLI arguments | `clap` |
| Ctrl+C in the CLI | `ctrlc` |
| Desktop UI | `gpui` (package `gpui-pre`, pinned `=0.3.8`), `gpui-component` (pinned `=0.7.1`) |
| App updates | `cargo-packager-updater` |
| Tests | `cargo-nextest`, `insta`, `proptest`, `tempfile` |
| Temporary data directory of a measurement | `tempfile` (normal dependency of `antenna-eval` only) |
| Audio tests (dev-dependencies) | `symphonia` (decode), `assert_no_alloc` (callback allocations) |
| JSON | `serde_json` (the model configuration files in the engine crates and in `antenna-review`, the Magpie token table, the update feed in the desktop app, and tests) |
| Local HTTP server in tests | `tiny_http` |
| Benchmarks | `divan` |
| Workspace metadata in `xtask` | `cargo_metadata` |
| File walk in `xtask` | `ignore` |

### 10.1 Tools

| Problem | Tool |
|---|---|
| Test runner | `cargo-nextest` |
| Licenses and advisories | `cargo-deny` |
| Unused dependencies | `cargo-machete` |
| macOS app bundle, DMG and update signatures | `cargo-packager` |
| Third-party license file | `cargo-about` |
| Start time measurement | `hyperfine` |
| Binary size | `cargo-bloat` |

## 11. Quality measurement

### 11.1 Parity tests

Each in-house engine has parity tests against its reference implementation. The fixtures are small safetensors files from `tools/reference/<engine>`. Each fixture is smaller than 2 MB.

| Level | Comparison | Tolerance |
|---|---|---|
| Module | Output tensor of each module (embeddings, one transformer layer, the codec decoder) for a fixed input | Max absolute difference 1e-4 in f32 on CPU |
| Greedy generation | Codes of the first 50 frames with greedy sampling | Exact match on CPU in f32 |
| Waveform | Codec decoder output for fixed codes | Max absolute difference 1e-3 in f32 on CPU |

### 11.2 Intelligibility

`antenna-eval wer --voice <id>` synthesizes the corpus in `tools/eval/corpus/<language>.txt` (40 sentences for each language, no digits). It transcribes the audio with Whisper large-v3-turbo and calculates the WER after normalization. The normalization makes the text lowercase and removes all punctuation except an apostrophe (U+0027) between two letters.

| Language | Maximum WER |
|---|---|
| English | 4% |
| Spanish | 4% |
| Portuguese | 5% |
| French | 6% |
| Italian | 4% |
| German | 4% |

The corpus also has a `numbers` subset. The WER of this subset goes into `docs/benchmarks.md`, but it does not cause a failure.

### 11.3 Speed

`antenna-eval bench --voice <id> --quality <q>` measures the time to first audio, the real-time factor, the buffer underruns and the peak resident memory on the corpus. It writes one row in `docs/benchmarks.md`. With `--check-budgets`, it also compares the results with section 8.

### 11.4 Listening check

The project owner listens to 5 sentences of each default voice and signs off in `docs/benchmarks.md`. This is the only manual quality check of the audio.

## 12. Desktop app

The visual design is in `docs/design/`. The design system ("Flow") is in `docs/design/pages/sistema-flow.html` and `docs/design/screens/sistema-flow.png`. The design shows the Spanish UI. The design pages contain example data (voices, engines, documents). The app shows real data.

### 12.1 Design rules from Flow

1. **Everything flows.** Controls have full radii, and shapes merge into each other. Nothing has a sharp corner.
2. **One accent.** The signal blue `#2D4BE0` marks only what is alive or new. That is the audio that plays, the text that generates, the current location, unread items and the count of new release notes.
3. **The dots mark.** The two dots of the logo are the playhead, the synthesis front and the status.
4. **Soft bounce.** Each change settles with a small overshoot, `cubic-bezier(.34, 1.56, .64, 1)`, 300 ms for state and 350 ms for layout.
5. **Typography.** Fraunces for the brand, voice names and reading text. Figtree for the interface. DM Mono for data. All three fonts are under the SIL Open Font License.
6. **No nested containers.** Sections separate with lines and space. A card never contains a card.

The design has no dark palette. Stage 15 makes the dark tokens from the light tokens with the rules in that stage, and the project owner approves them.

### 12.2 Structure

```
apps/desktop/
├── src/
│   ├── main.rs          # composition root: registry, pipeline, library, player, review, window
│   ├── shell/           # window, sidebar and the routing between screens
│   ├── screens/         # studio/, library/, voices/, models/, settings/
│   ├── state/           # AppState entity and the GPUI tasks that move job, player, export and review events into it
│   ├── settings/        # settings file with typed values
│   ├── updates/         # update check and install, the Version type, the feed URLs and the public key
│   └── strings/         # all UI text in English and Spanish
├── bundle/              # the app icon and the macOS bundle files
└── examples/
    └── seed_library.rs  # writes N documents into a data directory, for the start budget

crates/ui/
├── src/
│   ├── lib.rs
│   ├── theme/           # design tokens (colors, type, sizes, motion), the light and dark themes, fonts, icons
│   └── components/      # Flow components
├── assets/              # fonts and icons
└── examples/
    └── gallery/         # the component gallery: cargo run -p antenna-ui --example gallery
```

1. `AppState` is the only owner of the player, the library handle, the model store and the pipeline. Screens and the shell send `Action` values to `AppState`, for example `Action::PlayDocument(DocumentId)`, `Action::Preview(VoiceId)` and `Action::Play`. Screens and the shell read state, for example the play position, through `AppState`. `Input` is the internal message type of the Studio session, and only `AppState` sends it.
2. `cargo xtask lint-repo` fails when a file in `src/screens/` or `src/shell/` names `antenna_audio`, `antenna_library`, `antenna_models`, `antenna_pipeline` or `Input::`. Screens send explicit `Action` values. Only `AppState` and the bridge make `Input` values.
3. Components come from `antenna-ui` and read only design tokens. The dependency graph enforces this rule, because `antenna-ui` has no workspace dependency. `antenna-ui` has no `serde` dependency. The desktop settings keep an `AppearancePreference { System, Light, Dark }` and convert it to the `Appearance { Light, Dark }` of `antenna-ui`.
4. `cargo xtask lint-repo` fails when a file outside `crates/ui` names `gpui_component`, or when a file outside `crates/ui/src/theme/` contains a raw design value. `docs/standards.md` section 13 defines the check.
5. The settings file keeps typed values, for example `Language`, `Quality`, `ExportFormat`, `ExportRate` and `Loudness`. The quality of a document has one source, the voice record of that document. The quality in the settings is only the default for a new document.
6. The `Version` type of `updates/` parses `major.minor.patch` with an optional `-beta.N` suffix. It needs no crate.

## 13. Testing strategy

| Crate | Tests |
|---|---|
| `core` | Constructors, descriptor validation |
| `text` | Markdown conversion and segmentation with `insta` snapshots. Range and order invariants with `proptest` |
| `audio` | Track states, seek, resampler, loudness, encoders decode to the expected duration, zero callback allocations |
| `storage/models` | Store logic against a local HTTP server. Hash failure, size check, disk space, range download, resume, removal |
| `storage/library` | Document round trip, status from data, search, grouping, segment keys, garbage collection, crash safety |
| `pipeline` | Event sequence, queue order, cache hits, cancellation in one chunk, backpressure, panic isolation, start segment. All with the `fake` engine |
| `inference/review` | Word alignment with fixed transcripts. Transcription with Whisper (nightly) |
| `engines/registry` | Each language has a default voice. Each default voice exists |
| engines | Conformance suite. Parity tests. WER and speed with `antenna-eval` (nightly) |
| `ui` | Token values against `docs/design/tokens.md`. Visual check of the gallery against `docs/design/screens` |
| `apps/desktop` | `AppState` transitions without a window. Visual check against `docs/design/screens` |

## 14. Global decision rules

These rules apply to all stages. A stage file can add more rules.

1. If a parity test fails, compare the tensors module by module from the input. Fix the first module that differs. Do not increase a tolerance.
2. If the default voice of a language misses the RTF budget, profile with `tracing` spans and remove the largest cost. Repeat until the voice meets the budget. If no cost that the stage can remove is left, write a Blocked section.
3. If a WER target fails, examine the transcripts. If the cause is the text, fix the text crate. If the cause is the model, fix the engine. Do not change the corpus.
4. If a crate in section 10 does not do what the stage needs, write a Blocked section. Do not replace the crate.
5. If the design and this document do not agree, this document is correct for behavior and the design is correct for appearance.

## 15. Future extensions

| Extension | Extension point |
|---|---|
| Pause, emphasis and pronunciation marks | A markup layer in `antenna-text` that produces segment attributes, and a list of the supported attributes in the `EngineDescriptor` |
| Interpretation controls | A list of interpretation settings with their ranges in each `Variant`, with the values passed to `load` |
| Synthesis while the user types | A job for each changed segment. The segment store already keeps the segments that did not change |
| Playback speed | A time-stretch step in the player feed thread |
| Word-level highlight | Word timestamps from `antenna-review` |
| LLM script rewrite | A new step in `text::prepare` before segmentation |
| More than one speaker | An optional speaker in `Segment`, and a speaker-to-voice map in `JobSpec` |
| PDF, EPUB, URL | An `Importer` trait in `antenna-text`, one module for each format |
| More engines | Section 7.5 |
| Linux and Windows packages | CI already compiles both. Only packaging is necessary |
| Accessibility | An ADR after the GPUI accessibility API is stable |

## 16. Decision records

1. `0001-gpl-license.md` (stage 01)
2. `0002-gpui-desktop-shell.md` (stage 01)
3. `0003-threads-for-synthesis.md` (stage 01)
4. `0004-engine-contract.md` (stage 01)
5. `0005-candle-single-runtime.md` (stage 01)
6. `0006-in-house-engines.md` (stage 01)
7. `0007-mp3-default-export.md` (stage 01)
8. `0008-byte-range-artifacts.md` (stage 04)
9. `0009-unsafe-weight-mapping.md` (stage 08)
10. `0010-segment-store.md` (stage 05)
11. `0011-repository-layout.md` (stage 01)
12. `0012-exhaustive-enums.md` (stage 01), no `#[non_exhaustive]` in the workspace, because no crate is published and each match stays exhaustive
13. `0013-cargo-deny-arguments.md` (stage 01), the argument order of cargo-deny 0.20 and the pinned version
