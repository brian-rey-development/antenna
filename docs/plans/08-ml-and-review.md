# Stage 08. Shared inference and review

## Goal

`antenna-ml` gives all models one way to select a device, load weights, sample, seed, stream causal convolutions and pace a generation. `antenna-review` transcribes a segment with Whisper and returns the source ranges of the words that the voice said incorrectly.

## Context

Stages 10 to 14 implement two engines on `candle`. Both engines need the same device selection, weight loading, sampler, streaming convolutions and runaway guard. This stage puts these parts in one place before the first engine exists, so no engine writes them a second time. Each module in `antenna-ml` has two or more users by design (see `docs/architecture.md` section 7.4).

`antenna-review` is the second user of the device, weight and sampler modules. It runs Whisper for two purposes. The desktop app uses Whisper Small for pronunciation review (`docs/architecture.md` section 6.6). The eval tool of stage 09 uses Whisper large-v3-turbo to measure the WER of each voice (`docs/architecture.md` section 11.2). Both purposes use the same transcriber, the same normalization and the same word alignment.

## Read first

1. `CLAUDE.md`
2. `docs/architecture.md` sections 3, 4.4, 6.6, 6.7, 7.1, 7.4, 9 and 10
3. `docs/standards.md` sections 3, 6, 7, 8, 9, 12 and 14
4. `docs/plans/01-foundation.md` (the core types), `docs/plans/03-audio.md` (the `resample` function) and `docs/plans/04-models.md` (`ModelStore::ensure`)

## Scope

The stage can create or change these files only.

- `crates/inference/ml/`
- `crates/inference/review/`
- `docs/adr/0009-unsafe-weight-mapping.md`
- `Cargo.toml` (the `members` list and `[workspace.dependencies]` only)

## Deliverables

### `antenna-ml` file layout

```
crates/inference/ml/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── device.rs              # device selection
    ├── weights.rs             # safetensors and GGUF memory maps, the only unsafe code of the workspace
    ├── sampler.rs             # sampling configuration and sampler
    ├── seed.rs                # stable segment seed
    ├── stream/
    │   ├── mod.rs             # the Streaming trait and codes_tensor
    │   ├── conv.rs            # CausalConv1d
    │   └── conv_transpose.rs  # CausalConvTranspose1d
    ├── schedule.rs            # ChunkSchedule, Pacing and the runaway guard
    └── error.rs               # MlError
```

Normal dependencies are `candle-core`, `candle-nn`, `candle-transformers`, `memmap2`, `sha2`, `thiserror` and `tracing`. The dev-dependency is `tempfile`. The Metal backend of candle is a macOS target dependency. `Cargo.toml` of `antenna-ml` has a `[target.'cfg(target_os = "macos")'.dependencies]` table that enables the `metal` feature of the three candle crates. `antenna-ml` has no cargo feature, so `--all-features` builds on Linux and Windows. No other crate enables Metal.

`antenna-ml` has no dependency on another Antenna crate. Thus its API uses `candle` types and standard types only.

### Device

```rust
pub const DEVICE_ENV: &str = "ANTENNA_DEVICE";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DevicePreference { Auto, Cpu, Metal }

impl FromStr for DevicePreference { type Err = MlError; }   // "auto", "cpu", "metal"

pub fn select_device() -> Result<Device, MlError>;          // reads DEVICE_ENV, default Auto
pub fn device_for(preference: DevicePreference) -> Result<Device, MlError>;
pub fn device_label(device: &Device) -> &'static str;      // "Metal" or "CPU", for logs, reports and the UI

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Backend { Cpu, Metal }

pub fn backend(device: &Device) -> Backend;
```

1. `Auto` gives Metal on macOS when a Metal device exists. In all other cases it gives the CPU.
2. `Metal` gives `MlError::DeviceUnavailable` when no Metal device exists.
3. An unknown value of `ANTENNA_DEVICE` gives `MlError::UnknownDevice`.
4. The engines select a dtype from a `Backend` value, not from a `Device`. Thus a test of the dtype rule runs on each CI machine, also on a machine with no Metal device. `device_label` and `backend` read the same `Device` variant.
5. `select_device` calls the private function `log_device_once`. That function logs the selected device at `info` level and returns `true` at its first call in the process. It returns `false` and logs nothing at each later call. A `std::sync::OnceLock` holds the state.

### Weights

```rust
pub fn load_safetensors(paths: &[&Path], dtype: DType, device: &Device) -> Result<VarBuilder<'static>, MlError>;

pub struct GgufWeights { /* memmap2::Mmap and gguf_file::Content */ }

impl GgufWeights {
    pub fn open(path: &Path) -> Result<Self, MlError>;
    pub fn metadata(&self, key: &str) -> Option<&gguf_file::Value>;
    pub fn tensor(&self, name: &str, dtype: DType, device: &Device) -> Result<Tensor, MlError>;
}
```

1. `load_safetensors` memory-maps the files with `VarBuilder::from_mmaped_safetensors`. This function is `unsafe` in candle, because another process can change a mapped file.
2. `GgufWeights::open` memory-maps the file with `memmap2::Mmap::map` and reads the header from the map. `tensor` reads one tensor from the map through a `std::io::Cursor`, dequantizes it and converts it to `dtype`. `tensor` takes `&self`, so one `GgufWeights` value serves all loaders of a model.
3. Both memory maps are `unsafe`. ADR 0009 records this exception. `docs/standards.md` section 9 permits `unsafe` code in `weights.rs` only.
4. Each `unsafe` block has `#[expect(unsafe_code, reason = "...")]` and a `// SAFETY:` comment. The comment tells why the files do not change. The model store checks the hash of each file and never writes to a checked file.

### Sampler

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SamplingConfig {
    pub temperature: f64,             // 0.0 means greedy
    pub top_k: Option<usize>,
    pub top_p: Option<f64>,
    pub repetition_penalty: f32,      // 1.0 means no penalty
    pub repetition_window: usize,     // the number of recent tokens that the penalty uses
}

impl SamplingConfig { pub const GREEDY: Self; }

pub struct Sampler { /* candle_transformers::generation::LogitsProcessor, config */ }

impl Sampler {
    pub fn new(config: SamplingConfig, seed: Seed) -> Self;
    pub fn sample(&mut self, logits: &Tensor, history: &[u32]) -> Result<u32, MlError>;
}
```

1. `Sampler` uses `candle_transformers::generation::LogitsProcessor` with the `Sampling` variant that matches the configuration.
2. `sample` applies `candle_transformers::utils::apply_repeat_penalty` to the last `repetition_window` tokens of `history` before it samples.
3. Each model reads its default sampling values from its own configuration files. `antenna-ml` has no model-specific values.
4. The engines use `SamplingConfig` and `Seed` directly. An engine does not define a second sampling type.

### Seed

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Seed(u64);

impl Seed {
    pub fn for_segment(voice: impl Display, text: &str, attempt: u8) -> Self;
    pub fn value(self) -> u64;
}
```

1. The engines pass their `VoiceId` as `voice`. `antenna-ml` does not depend on `antenna-core`, so it takes the `Display` form of the voice id, for example `qwen3/es-lucia`.
2. The seed is the first 8 bytes, in little-endian order, of the SHA-256 of a byte sequence. The sequence is the UTF-8 bytes of the `Display` form of `voice`, `0x00`, the UTF-8 bytes of `text`, `0x00` and the byte `attempt`.
3. SHA-256 does not change between Rust versions, so the seed of a segment is the same on all machines and in all releases. Do not use `DefaultHasher`.
4. The `attempt` is 0 for the first attempt and 1 for the retry of item 6 in `docs/architecture.md` section 9.

### Streaming convolutions

```rust
pub trait Streaming {
    type State: Clone;
    fn start(&self) -> Self::State;
    fn forward(&self, input: &Tensor, state: &mut Self::State) -> Result<Tensor, MlError>;
}

pub struct CausalConv1d { /* weight, bias, dilation, groups */ }
impl CausalConv1d {
    pub fn new(weight: Tensor, bias: Option<Tensor>, dilation: usize, groups: usize) -> Result<Self, MlError>;
}
impl Streaming for CausalConv1d { type State = ConvState; }

pub struct CausalConvTranspose1d { /* weight, bias, stride */ }
impl CausalConvTranspose1d {
    pub fn new(weight: Tensor, bias: Option<Tensor>, stride: usize) -> Result<Self, MlError>;
}
impl Streaming for CausalConvTranspose1d { type State = ConvTransposeState; }

pub fn codes_tensor<const N: usize>(frames: &[[u32; N]], device: &Device) -> Result<Tensor, MlError>;
```

1. `Streaming` is the one forward path of every module with time state. A module without time state has a plain `forward`. There is no second forward path without state.
2. The input and output of `forward` have the shape `[channels, time]`. A one-shot pass is one `forward` call with all time steps.
3. `CausalConv1d` pads only on the left. Its state is the last `(kernel - 1) × dilation` input steps. The kernel size comes from the weight shape.
4. `CausalConvTranspose1d` trims only on the right. Its state is the last `kernel - stride` output steps that the next input changes.
5. A state update makes new tensors and never writes into an existing tensor. Do not use `slice_set`, `candle_nn::kv_cache::KvCache` or a ring buffer in a `Streaming` state. Thus a clone of `ConvState` or `ConvTransposeState` copies no tensor data, and an update of the original does not change the clone. Each engine composes these states into the state of its codec with the same rule.
6. `codes_tensor` converts a slice of frames into a `U32` tensor with the shape `[N, frames.len()]`, so the codebooks are the first dimension.

### Schedule

```rust
pub const RUNAWAY_FACTOR: usize = 4;          // item 6 of docs/architecture.md section 9

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChunkSchedule { pub first: usize, pub rest: usize }
impl ChunkSchedule { pub const fn steps(&self, chunks_emitted: usize) -> usize; }

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pacing { pub chunks: ChunkSchedule, pub steps_per_char: f32, pub min_expected_steps: usize }
impl Pacing { pub fn step_limit(&self, chars: usize) -> usize; }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Paced { Ended, Stopped, Runaway }

pub type OnStep<'a, S, E> = &'a mut dyn FnMut(S) -> Result<ControlFlow<()>, E>;

pub fn pace<S, E>(
    chunks: ChunkSchedule,
    step_limit: usize,
    generate: impl FnMut(u8, OnStep<'_, S, E>) -> Result<(), E>,
    flush: impl FnMut(&[S]) -> Result<ControlFlow<()>, E>,
) -> Result<Paced, E>;
```

1. A step is one iteration of the generation loop of a model. Each engine defines how many codec frames one step makes.
2. `ChunkSchedule::steps` returns `first` when `chunks_emitted` is 0 and `rest` in all other cases.
3. `Pacing::step_limit` returns `RUNAWAY_FACTOR × max(ceil(chars × steps_per_char), min_expected_steps)`.
4. `pace` is the synthesis driver of every engine. The engine gives two closures. `generate` runs one generation with the attempt number and gives each step to `on_step`. `flush` decodes a buffer of steps and emits the audio. `E` is the error enum of the engine crate, so the closures return the errors of the engine with their data.
5. `pace` runs attempt 0 first. It puts each step in a buffer. When the buffer has `chunks.steps(chunks_emitted)` steps, it calls `flush` and empties the buffer. If `flush` returns `Break`, `on_step` returns `Break` and `pace` returns `Paced::Stopped`.
6. If `generate` returns before the step limit, `pace` flushes the steps that remain in the buffer and returns `Paced::Ended`. `pace` never calls `flush` with an empty buffer. When the last chunk takes all the steps, `generate` returns with an empty buffer and `pace` returns `Paced::Ended` with no further call.
7. A generation that gives more steps than `step_limit` is a runaway. `on_step` returns `Break` at the first step over the limit. Then the private function `runaway_action(chunks_emitted, attempt)` decides. `Retry` runs `generate` again with attempt 1 and an empty buffer. `Fail` returns `Paced::Runaway`.
8. `runaway_action` returns `Retry` when `chunks_emitted` is 0 and `attempt` is 0. It returns `Fail` in all other cases. A retry after the first chunk is not possible, because the user already heard that audio.
9. The engine maps `Paced::Runaway` to `EngineError::Runaway` with the index of the segment. `Ended` and `Stopped` give `Ok(())`.
10. Each engine has one `const PACING: Pacing` and keeps a copy in its engine value. The tests of an engine change that copy to force the runaway guard. No engine has a constructor or a trait for tests.
11. `pace` has no model code, so its unit tests use closures that give a fixed number of steps. Thus all branches of the guard run in CI without models.

### `MlError`

```rust
#[derive(Debug, thiserror::Error)]
pub enum MlError {
    Candle(#[from] candle_core::Error),
    UnknownDevice { value: String },
    DeviceUnavailable { device: &'static str },
    Io { path: PathBuf, #[source] source: std::io::Error },
    MissingTensor { name: String },
    Shape { expected: String, actual: String },
}
```

### `antenna-review` file layout

```
crates/inference/review/
├── Cargo.toml
├── assets/
│   ├── README.md              # source, commit and license of each asset
│   ├── melfilters.bytes       # 80-bin mel filters (Whisper Small)
│   └── melfilters128.bytes    # 128-bin mel filters (Whisper large-v3-turbo)
├── src/
│   ├── lib.rs
│   ├── model.rs               # ReviewModel and the two static descriptors
│   ├── transcriber/
│   │   ├── mod.rs             # Transcriber: load and transcribe
│   │   ├── mel.rs             # PCM to log-mel spectrogram
│   │   └── decode.rs          # greedy decoding with forced language and task
│   ├── words.rs               # word split with source ranges and normalization
│   ├── align.rs               # minimum edit distance alignment, WordErrors
│   ├── queue.rs               # ReviewQueue and the "antenna-review" thread
│   └── error.rs               # ReviewError
└── tests/
    ├── fixtures/
    │   └── jfk.flac           # public domain reference speech, 11 s, 16 kHz
    └── transcribe.rs          # nightly tests that need the Whisper models
```

Normal dependencies are `antenna-core`, `antenna-ml`, `antenna-audio`, `candle-core`, `candle-nn`, `candle-transformers`, `tokenizers`, `serde_json`, `unicode-normalization`, `uuid`, `flume`, `thiserror` and `tracing`. Dev-dependencies are `antenna-models`, `symphonia` (with the `flac` feature) and `insta`. `antenna-review` uses `antenna_audio::resample` to convert the engine sample rate to 16 kHz. `docs/architecture.md` section 3.1 permits these edges.

`antenna-review` re-exports `antenna_ml::device_label` and `antenna_ml::select_device`. The apps do not depend on `antenna-ml`, and they show the device with these functions.

### Review models

```rust
pub struct ReviewModel {
    pub id: &'static str,                  // "whisper-small", "whisper-large-v3-turbo"
    pub name: &'static str,                // "Whisper Small"
    pub summary: Localized,                // one line for the Models screen
    pub license: ModelLicense,
    pub languages: &'static [Language],    // all of Language::ALL for both models
    pub mel_bins: usize,                   // 80 or 128
    pub artifacts: &'static [Artifact],
}

pub static WHISPER_SMALL: ReviewModel;             // the desktop app
pub static WHISPER_LARGE_V3_TURBO: ReviewModel;    // antenna-eval
```

The artifacts of each model have the keys `whisper-config`, `whisper-tokenizer` and `whisper-weights`. Each extent is `Extent::Whole`. These are the real values from the Hugging Face API on 2026-10-06.

| Model | Repository | Revision |
|---|---|---|
| `WHISPER_SMALL` | `openai/whisper-small` | `973afd24965f72e36ca33b3055d56a652f456b4d` |
| `WHISPER_LARGE_V3_TURBO` | `openai/whisper-large-v3-turbo` | `41f01f3fe87f28c78e2fbf8b568835947dd65ed9` |

| Model | Path | Bytes | SHA-256 |
|---|---|---|---|
| Small | `config.json` | 1967 | `e6a2b489da1b5aed65a8eb8d1e7466fa867ad5643a8bc138ba708bd56b2875c4` |
| Small | `tokenizer.json` | 2480466 | `27fc476bfe7f17299480be2273fc0608e4d5a99aba2ab5dec5374b4482d1a566` |
| Small | `model.safetensors` | 966995080 | `1d7734884874f1a1513ed9aa760a4f8e97aaa02fd6d93a3a85d27b2ae9ca596b` |
| Turbo | `config.json` | 1256 | `c5b526b3e3cd64cd8940dabb45e8ba726629e22d8ed389c29b552f9140daf04a` |
| Turbo | `tokenizer.json` | 2710337 | `297b13372ac43916285644fb9687add3cc62ee2a1adb60da3dc25cc94c1871fd` |
| Turbo | `model.safetensors` | 1617824864 | `542566a422ae4f3fd23f1ba11add198fca01bbf82e66e6a2857b3f608b1eb9d1` |

Both models have the license `MIT` with the URL `https://huggingface.co/openai/whisper-small` or `https://huggingface.co/openai/whisper-large-v3-turbo`. The attribution text is "Whisper by OpenAI, MIT License". Whisper Small has 80 mel bins, d_model 768 and 12 encoder and 12 decoder layers in f32. Whisper large-v3-turbo has 128 mel bins, d_model 1280, 32 encoder layers and 4 decoder layers in f16.

`antenna-models` downloads these artifacts with `ModelStore::ensure`, like the files of an engine. `ensure` takes `impl IntoIterator<Item = &'static Artifact>`, so the caller passes `model.artifacts` directly.

### Transcriber

```rust
pub const WHISPER_SAMPLE_RATE: SampleRate = SampleRate::HZ_16000;
pub const WHISPER_WINDOW: Duration = Duration::from_secs(30);

pub struct Transcriber { /* model, tokenizer, mel filters, device, language token ids */ }

impl Transcriber {
    pub fn load(model: &'static ReviewModel, files: &ModelFiles) -> Result<Self, ReviewError>;
    pub fn transcribe(&mut self, samples: &[f32], rate: SampleRate, language: Language) -> Result<String, ReviewError>;
}
```

1. `load` selects the device with `antenna_ml::select_device`. It loads the weights with `antenna_ml::load_safetensors` in the dtype of the model configuration. It reads the configuration with `serde_json` and the tokenizer with `tokenizers`.
2. `load` selects the mel filters from the embedded asset with the `mel_bins` of the model. A model with an unknown bin count gives `ReviewError::MelBins`.
3. `transcribe` resamples the samples to 16 kHz with `antenna_audio::resample` when `rate` is not 16 kHz.
4. `transcribe` splits audio that is longer than 30 s into windows of 30 s and joins the window transcripts with one space. The private function `windows(samples: usize) -> Vec<Range<usize>>` gives the sample ranges of the windows at 16 kHz. The last window holds the remaining samples.
5. The decoder forces the start sequence `<|startoftranscript|>`, the language token of `language`, `<|transcribe|>` and `<|notimestamps|>`. It decodes greedily and stops at `<|endoftext|>` or at 224 tokens.
6. The decoder suppresses the tokens in `suppress_tokens` of the model configuration.
7. `transcribe` returns the decoded text without special tokens and with trimmed whitespace.

### Words and normalization

```rust
pub struct Word { pub source: Range<usize>, pub normal: Box<str> }

pub fn words(text: &str) -> Vec<Word>;         // the words of a text with their byte ranges
pub fn normalize(text: &str) -> String;        // the normal form of a text, for WER
```

Normalization obeys these rules in this sequence. `words` applies the same rules to each word and drops a word whose normal form is empty. The apostrophe stays, because it is part of a word in French, Italian and English, for example `l'eau`.

| Rule | Example |
|---|---|
| Convert to NFC with `unicode-normalization` | `e` followed by U+0301 becomes `é` |
| Lowercase with `str::to_lowercase` | `Hola` becomes `hola` |
| Replace U+2019 with U+0027 | `l’eau` becomes `l'eau` |
| Replace each character that is not alphanumeric, not U+0027 and not whitespace with a space | `¿Qué?` becomes ` qué ` |
| Collapse whitespace to one space and trim | `a  b ` becomes `a b` |
| Keep diacritics | `qué` stays `qué` |

A word is a maximal sequence of characters that are not whitespace after the fourth rule. Its source range is the byte range of the original characters in `text`.

### Alignment

```rust
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WordErrors { pub substitutions: usize, pub deletions: usize, pub insertions: usize, pub reference_words: usize }

impl WordErrors {
    pub fn total(&self) -> usize;
    pub fn add(self, other: Self) -> Self;
}

pub struct Alignment { pub errors: WordErrors, pub mismatched: Vec<Range<usize>> }

pub fn align(reference: &str, transcript: &str) -> Alignment;
```

1. `align` splits both texts with `words` and computes the minimum edit distance on the sequences of normal forms. A substitution, a deletion and an insertion each cost 1.
2. When two paths have the same cost, the backtrace prefers a match, then a substitution, then a deletion, then an insertion. Thus the result is deterministic.
3. `mismatched` contains the source range of each reference word that `align` counts as a substitution or a deletion, in source order. An inserted word has no source range, so it is only in `errors.insertions`.
4. A reference word that contains a digit is never in `mismatched`. Whisper writes numbers in words or in digits without a rule, so a digit word gives no reliable result. It still counts in `errors`.
5. The WER of a corpus is the sum of `total()` divided by the sum of `reference_words`. Stage 09 uses `WordErrors::add` for the sum.

### Review queue

```rust
pub struct ReviewRequest {
    pub document: Uuid,
    pub segment: SegmentIndex,
    pub text: Box<str>,
    pub source: Range<usize>,          // the range of the segment in the document
    pub language: Language,
    pub samples: Arc<[f32]>,
    pub rate: SampleRate,
}

#[derive(Debug)]
pub enum ReviewEvent {
    Reviewed { document: Uuid, segment: SegmentIndex, mismatched: Vec<Range<usize>>, transcript: String },   // document byte ranges
    Failed { document: Uuid, segment: SegmentIndex, error: ReviewError },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gate { Open, Closed }

pub struct ReviewQueue { /* flume::Sender<Command>, JoinHandle */ }

impl ReviewQueue {
    pub fn start(transcriber: Transcriber) -> (Self, flume::Receiver<ReviewEvent>);
    pub fn submit(&self, request: ReviewRequest);
    pub fn set_gate(&self, gate: Gate);             // the app opens the gate when the job worker is idle
    pub fn clear(&self, document: Uuid);            // drops the pending requests of a document
}

impl Drop for ReviewQueue { /* sends Shutdown and joins the thread */ }
```

1. `start` starts the thread `antenna-review` with `std::thread::Builder`. The thread owns the transcriber and a `VecDeque` of pending requests.
2. The thread blocks on one `flume` channel of commands. It does not poll and it does not sleep.
3. The gate is `Closed` at start. The thread transcribes a request only when the gate is `Open`. It reads the next command after each request, so a closed gate stops the queue after the current request.
4. A `Reviewed` event contains the mismatched ranges in document coordinates and the transcript of the segment. The thread adds `source.start` to each range from `align`. The desktop app shows the transcript in the tooltip of a flagged word.
5. If the same document and segment are in the queue two times, the thread keeps only the newest request.
6. `Uuid` is the value inside the `DocumentId` of `antenna-library`, from `DocumentId::uuid`. `antenna-review` uses the `uuid` crate directly and does not depend on `antenna-library`.

### `ReviewError`

```rust
#[derive(Debug, thiserror::Error)]
pub enum ReviewError {
    Ml(#[from] MlError),
    Engine(#[from] EngineError),               // a missing file in ModelFiles
    Config(#[source] serde_json::Error),
    Tokenizer(#[source] tokenizers::Error),
    MelBins { bins: usize },
}
```

`MlError` already wraps `candle_core::Error`, so `ReviewError` has no second `Candle` variant. The transcriber converts a candle error with `MlError::from`.

## Tasks

1. Write ADR 0009. It records that `crates/inference/ml/src/weights.rs` is the only `unsafe` code in the workspace, with the two memory maps, and gives the safety argument.
2. Add `crates/inference/ml` and `crates/inference/review` to the workspace.
3. Write `device.rs`, `seed.rs`, `sampler.rs` and `weights.rs` with their unit tests.
4. Write `stream/` with its unit tests. The tests make the weights and the inputs from a fixed formula, for example `sin(0.37 × i)` over `Tensor::arange`, so they run in CI without models. Do not use a random generator, because candle cannot seed its CPU generator.
5. Write `schedule.rs` with its unit tests. The tests of `pace` use closures that give a fixed number of steps.
6. Copy `melfilters.bytes` and `melfilters128.bytes` from the candle repository at a pinned commit. Write the commit and the license in `crates/inference/review/assets/README.md`.
7. Copy `tests/jfk.flac` from the `openai/whisper` repository at a pinned commit to `crates/inference/review/tests/fixtures/jfk.flac`. The recording is a speech by a US president, which is in the public domain. Write the source and the commit in `crates/inference/review/assets/README.md`.
8. Write `model.rs` with the two static descriptors and the values of the tables in "Review models".
9. Write `words.rs` and `align.rs` with their unit tests and `insta` snapshots.
10. Write `mel.rs`, `decode.rs` and `transcriber/mod.rs`. Use the Whisper model of `candle-transformers`. Use the decoding of the candle Whisper example as reading material, and write the decoder in the style of this repository.
11. Write `queue.rs` with its tests. Decision rule 6 defines the test transcriber.
12. Write the nightly tests in `tests/transcribe.rs`. Each test gets its files with one call, `ModelStore::open_default()?.ensure(WHISPER_SMALL.artifacts, &|_| {}, &AtomicBool::default())?` (or the turbo model), and decodes `jfk.flac` with `symphonia`.
13. Run the nightly tests one time on the reference machine and record the transcription time of `jfk.flac` for each model in the stage report.
14. Run `cargo xtask check`. Fix each failure.
15. Do the quality gate in `docs/standards.md` section 20.

## Acceptance criteria

The reference transcript of `jfk.flac` is the text below.

```text
And so my fellow Americans, ask not what your country can do for you, ask what you can do for your country.
```

| ID | Criterion | Check |
|---|---|---|
| AC-08-01 | `DevicePreference` parses the three values and rejects other values | Tests `device_preference_parses_when_value_known` and `device_preference_fails_when_value_unknown` |
| AC-08-02 | `Cpu` always gives the CPU device | Test `device_for_returns_cpu_when_cpu_requested` |
| AC-08-03 | `load_safetensors` loads a file that candle wrote | Test `weights_load_safetensors_when_written_by_candle` (write a small file to a temporary directory, load it, compare the tensors) |
| AC-08-04 | `GgufWeights` memory-maps a file that candle wrote and reads two tensors through `&self` | Test `weights_read_gguf_tensors_when_written_by_candle` |
| AC-08-05 | The only `unsafe` code is in `crates/inference/ml/src/weights.rs` | `cargo xtask lint-repo` passes with its "Unsafe code" check |
| AC-08-06 | Greedy sampling returns the index of the largest logit | Test `sampler_returns_argmax_when_greedy` |
| AC-08-07 | The same seed gives the same token sequence, and a different seed gives a different sequence | Tests `sampler_repeats_sequence_when_seed_same` and `sampler_changes_sequence_when_seed_differs` |
| AC-08-08 | The repetition penalty decreases the probability of a recent token | Test `sampler_penalizes_token_when_in_window` |
| AC-08-09 | The seed algorithm does not change | Test `seed_matches_golden_value_when_input_fixed` with three fixed inputs and their expected `u64` values. One input has `attempt` 1 |
| AC-08-10 | The review model descriptors are valid | Test `review_models_have_valid_artifacts` checks the 40-character revisions, the 64-character hashes, the non-zero sizes and the unique keys |
| AC-08-11 | Normalization obeys each rule of its table | One test `normalize_<rule>` for each row |
| AC-08-12 | `words` returns correct source ranges for text with accents, apostrophes and punctuation | Test `words_keep_source_ranges_when_text_has_accents` with an `insta` snapshot |
| AC-08-13 | `align` counts each edit type | Tests `align_counts_substitution`, `align_counts_deletion`, `align_counts_insertion` and `align_counts_nothing_when_texts_match` |
| AC-08-14 | `align` returns the source range of each mismatched reference word | Test `align_returns_ranges_when_words_differ` with the reference "El otro atraviesa los huesos" and the transcript "El otro atraviesa sus huesos" |
| AC-08-15 | `align` never flags a word with a digit | Test `align_skips_word_when_it_has_digit` |
| AC-08-16 | `align` is deterministic when two paths have the same cost | Test `align_prefers_substitution_when_costs_equal` |
| AC-08-17 | The queue does nothing while the gate is `Closed` and starts when it is `Open` | Test `queue_waits_when_gate_closed` with the test transcriber of decision rule 6 |
| AC-08-18 | The queue keeps only the newest request for a segment | Test `queue_replaces_request_when_segment_repeats` |
| AC-08-19 | `clear` drops the pending requests of a document | Test `queue_drops_requests_when_document_cleared` |
| AC-08-20 | `Reviewed` ranges are in document coordinates | Test `queue_offsets_ranges_when_segment_starts_after_zero` |
| AC-08-21 | Whisper Small and Whisper large-v3-turbo transcribe `jfk.flac` with a WER of 5% or less against the reference transcript | Tests `small_transcribes_reference_speech` and `turbo_transcribes_reference_speech`, marked `#[ignore = "needs models"]`, pass with `cargo nextest run -p antenna-review --run-ignored all` |
| AC-08-22 | The transcriber resamples audio that is not at 16 kHz | Test `small_transcribes_reference_speech_when_rate_is_24k`, marked `#[ignore = "needs models"]`, resamples `jfk.flac` to 24 kHz first and gets the same WER limit |
| AC-08-23 | ADR 0009 exists and has 60 lines or less | `wc -l docs/adr/0009-unsafe-weight-mapping.md` |
| AC-08-24 | `CausalConv1d` gives the same output as a candle `conv1d` with left padding on the full input | Test `causal_conv_matches_padded_conv_when_input_is_whole`, max absolute difference 1e-5 |
| AC-08-25 | `CausalConvTranspose1d` gives the same output as a candle `conv_transpose1d` with a right trim on the full input | Test `causal_conv_transpose_matches_trimmed_conv_when_input_is_whole`, max absolute difference 1e-5 |
| AC-08-26 | Both streaming convolutions give the one-call result in any split | Tests `causal_conv_is_split_invariant_when_chunk_is_<n>` and `causal_conv_transpose_is_split_invariant_when_chunk_is_<n>` for 1, 2, 3, 4 and 7 on 40 time steps, max absolute difference 1e-5 |
| AC-08-27 | An update of the original state does not change a clone | Test `conv_state_clone_is_independent_when_original_advances`. Advance a state with input P and clone it. Advance the original with input A and the clone with input B. The clone output equals the output of a new state advanced with P and then B, max absolute difference 1e-5 |
| AC-08-28 | `codes_tensor` puts the codebooks in the first dimension | Test `codes_tensor_puts_codebooks_first_when_frames_given` |
| AC-08-29 | `ChunkSchedule` gives `first` and then `rest` | Test `chunk_schedule_uses_rest_when_first_chunk_emitted` |
| AC-08-30 | `Pacing::step_limit` obeys its formula | Tests `step_limit_uses_minimum_when_text_is_short` and `step_limit_scales_when_text_is_long` |
| AC-08-31 | `runaway_action` retries only before the first chunk of the first attempt | Tests `runaway_retries_when_no_chunk_and_first_attempt`, `runaway_fails_when_chunk_emitted` and `runaway_fails_when_attempt_is_retry` |
| AC-08-32 | `pace` flushes the first chunk after `chunks.first` steps and each later chunk after `chunks.rest` steps | Test `pace_flushes_chunks_when_steps_follow_schedule` with `ChunkSchedule { first: 2, rest: 4 }` and 11 steps, flush sizes 2, 4, 4 and 1 |
| AC-08-33 | `pace` flushes the remaining steps and returns `Ended` when the generation ends | Test `pace_returns_ended_when_generation_ends_before_limit` |
| AC-08-34 | `pace` stops when `flush` returns `Break` | Test `pace_returns_stopped_when_flush_breaks`, no `generate` call after the break |
| AC-08-35 | `pace` retries with attempt 1 when the limit comes before the first chunk | Test `pace_retries_with_attempt_one_when_runaway_precedes_first_chunk`, attempts seen by `generate` are 0 and 1 |
| AC-08-36 | `pace` returns `Runaway` without a retry when the limit comes after the first chunk | Test `pace_returns_runaway_when_limit_follows_first_chunk`, one `generate` call |
| AC-08-37 | `pace` returns `Runaway` when the retry also runs away | Test `pace_returns_runaway_when_retry_runs_away`, no flush call |
| AC-08-38 | The device log occurs one time for each process | Test `log_device_once_returns_true_only_when_called_first` |
| AC-08-39 | `antenna-review` re-exports the device functions | Test `device_label_is_cpu_when_device_is_cpu` in `crates/inference/review/tests/` calls `antenna_review::device_label` and `antenna_review::select_device` |
| AC-08-40 | `windows` splits audio longer than 30 s | Tests `windows_returns_one_range_when_audio_is_short` and `windows_splits_audio_when_longer_than_thirty_seconds` with 70 s of samples, ranges of 30 s, 30 s and 10 s |
| AC-08-41 | A failed transcription gives `ReviewEvent::Failed` and the queue continues | Test `queue_sends_failed_when_transcriber_fails` with a test transcriber that fails on the first request and succeeds on the second |
| AC-08-42 | `pace` never calls `flush` with an empty buffer | Test `pace_skips_empty_flush_when_last_chunk_is_full` with `ChunkSchedule { first: 2, rest: 4 }` and 10 steps, flush sizes 2, 4 and 4 |
| AC-08-43 | `backend` gives the backend of the device | Test `backend_is_cpu_when_device_is_cpu` |

## Decision rules

1. If `LogitsProcessor` does not support a combination of the configuration, implement that combination in `sampler.rs` with candle tensor operations. Do not add a crate.
2. If the candle Whisper implementation cannot load large-v3-turbo, write a "Blocked" section with the error. The change of model changes the eval method of `docs/architecture.md` section 11.2, so a human decides.
3. If the candle repository has no mel filter file for a bin count, calculate the filters with the Slaney mel formula of librosa. Compare them in a test fixture with the Python output of `librosa.filters.mel(sr=16000, n_fft=400, n_mels=<bins>)`.
4. If the Hugging Face revision of a Whisper repository is different from the table, keep the revision of the table. The pinned revision is the contract.
5. If Whisper large-v3-turbo in f16 gives different text on CPU and on Metal, load it in f32 on CPU for the nightly test. Keep f16 on Metal.
6. The queue tests need a transcriber that does not load a model. Define a `pub(crate)` trait `Transcribe` with one method in `queue.rs`. `Transcriber` implements it, and the test module defines a test implementation. The test implementation counts its calls and fails on the requests that the test selects. The trait has two implementations, so `docs/standards.md` section 5 permits it. It stays `pub(crate)`.
7. If `cargo xtask lint-repo` rejects the `review` edges, compare `tools/xtask/src/graph.rs` with `docs/architecture.md` section 3.1. Stage 01 copied the table, so a difference is a defect of stage 01. Report it and stop.
8. If `memmap2` is not in the crate table of `docs/architecture.md` section 10 at the start of this stage, write a "Blocked" section. Do not add the crate without the table row.

### Facts to check

1. The exact file names of the mel filter files in the candle repository. Check `candle-examples/examples/whisper/` at the pinned commit.
2. The path of `jfk.flac` in the Whisper repository. Check `tests/jfk.flac` in `openai/whisper` at the pinned commit.
3. That `suppress_tokens` exists in `config.json` of both repositories. If it exists only in `generation_config.json`, read it from that file and add a fourth artifact with the key `whisper-generation`.

## Out of scope

1. Any engine. Stages 10 to 14 implement the engines.
2. The `antenna-eval` tool. Stage 09 makes it.
3. The connection of the review queue to the job worker and the UI. Stage 18 connects them.
4. Word timestamps. They are a future extension in `docs/architecture.md` section 15.
