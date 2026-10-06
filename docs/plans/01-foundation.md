# Stage 01. Foundation

## Goal

The Antenna workspace exists with its tooling, CI, core contracts, `fake` engine, engine test kit and registry, and `cargo xtask check` passes on macOS, Linux and Windows.

## Context

All other stages build on this stage. The contracts in `antenna-core` are the extension points of the system, so their quality sets the quality of the project. The `fake` engine lets stages 06, 07, 16, 17 and 18 run without model downloads. The conformance suite is the gate for each future engine.

Stages 02 to 07 cannot change `antenna-core`. Thus this stage gives each core type its complete API, its derives and its error type.

## Read first

1. `CLAUDE.md`
2. `docs/architecture.md` sections 2, 3, 5, 8.1, 9, 10 and 16
3. `docs/standards.md`, all sections
4. `docs/writing.md`, all sections

## Scope

The stage can create or change these files only.

- Repository root files `.gitignore`, `Cargo.toml`, `rust-toolchain.toml`, `.rustfmt.toml`, `.clippy.toml`, `LICENSE`, `README.md`
- `.cargo/config.toml`, `.config/nextest.toml`, `.config/deny.toml`, `.github/workflows/ci.yml`
- `tools/xtask/`
- `crates/core/`, `crates/engines/testkit/`, `crates/engines/fake/`, `crates/engines/registry/`
- `docs/adr/`

## Deliverables

### Workspace

1. `Cargo.toml` with `resolver = "3"`, an explicit `members` list and `[workspace.dependencies]`.
   1. `[workspace.dependencies]` has an entry with `path` for each workspace crate. The entry of `antenna-engine-registry` sets `default-features = false`, because Cargo rejects `default-features = false` on an inherited dependency when the workspace entry does not set it. Each member writes `<crate>.workspace = true` and has no `path` or version of its own.
   2. `[workspace.package]` has edition 2024, `license = "GPL-3.0-or-later"`, `repository` and `rust-version`.
   3. The lints are the lints of `docs/standards.md` section 2.1.
   4. The profiles are the profiles of `docs/architecture.md` section 8.1.
2. `rust-toolchain.toml` with `channel = "1.98.1"` and the components `rustfmt` and `clippy`.
3. `.rustfmt.toml`, `.clippy.toml` and `.config/nextest.toml` with the content of `docs/standards.md` section 2.
4. `.config/deny.toml` with `[advisories]` enabled, `multiple-versions = "warn"` and a license allowlist. The allowlist contains exactly these licenses.
   1. MIT, Apache-2.0, Apache-2.0 WITH LLVM-exception, BSD-2-Clause, BSD-3-Clause, ISC, Zlib, 0BSD
   2. Unicode-3.0, MPL-2.0, BSL-1.0, CC0-1.0, CDLA-Permissive-2.0
   3. LGPL-2.1-or-later, LGPL-3.0, GPL-3.0-or-later
5. `.cargo/config.toml` with the alias `xtask = "run --package xtask --"`. The `xtask` package is in `tools/xtask/`. `cargo xtask check` passes `--config .config/deny.toml` to `cargo deny`.
6. The repository root contains only the entries of the tree in `docs/architecture.md` section 3. In this stage these are `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `README.md`, `LICENSE`, `CLAUDE.md`, `ROADMAP.md`, `.gitignore`, the hidden configuration files and the directories `crates/`, `tools/` and `docs/`. Stage 19 adds `CONTRIBUTING.md` and `CHANGELOG.md`. Other tool configuration goes in `.config/`.

### `xtask`

```text
cargo xtask check       # runs the sequence in docs/standards.md section 1
cargo xtask lint-repo   # runs the checks in docs/standards.md section 13
```

`tools/xtask/src/output.rs` is the only module of `xtask` that prints to the terminal, as rule 1 of `docs/standards.md` section 11 says.

`lint-repo` reads the permitted dependency graph from one constant in `tools/xtask/src/graph.rs`. The constant is a copy of the table in `docs/architecture.md` section 3.1 as data, with the normal and the dev-dependencies of each crate. It contains all rows, also the rows of crates that later stages make.

`lint-repo` implements each row of `docs/standards.md` section 13 in this stage. Each check scans only the files of the "Files" column of its row. Some rows apply to paths that later stages make, for example `crates/ui` and `apps/desktop`. These checks find no file until those paths exist, so each one gets its own fixture. The row that rejects `#[non_exhaustive]` applies to all crates from this stage.

### `antenna-core`

```rust
// Text forms come from strum with #[strum(serialize_all = "lowercase")].
// Display and FromStr give "en", "fast", "mp3". The FromStr error is strum::ParseError.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Display, EnumString, IntoStaticStr)]
pub enum Language { En, Es, Pt, Fr, It, De }
impl Language { pub const ALL: [Language; 6]; }

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Display, EnumString, IntoStaticStr)]
pub enum Quality { Fast, #[default] Balanced, Max }
impl Quality { pub const ALL: [Quality; 3]; }

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Display, EnumString, IntoStaticStr)]
pub enum ExportFormat { #[default] Mp3, Wav, Ogg }        // the text form is the file extension
impl ExportFormat { pub const ALL: [ExportFormat; 3]; }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Localized { pub en: &'static str, pub es: &'static str }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EngineId(&'static str);
impl EngineId { pub const fn new(id: &'static str) -> Self; pub const fn as_str(self) -> &'static str; }
impl Display for EngineId;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct VoiceId { engine: EngineId, key: &'static str }
impl VoiceId { pub const fn new(engine: EngineId, key: &'static str) -> Self; pub const fn engine(self) -> EngineId; pub const fn key(self) -> &'static str; }
impl Display for VoiceId;                                     // "engine/key"

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SampleRate(NonZeroU32);
impl SampleRate {
    pub const HZ_16000: Self;
    pub const HZ_22050: Self;
    pub const HZ_24000: Self;
    pub const HZ_44100: Self;
    pub const HZ_48000: Self;
    pub const fn new(hz: NonZeroU32) -> Self;
    pub const fn hz(self) -> u32;
    pub fn duration_of(self, samples: u64) -> Duration;
}
impl TryFrom<u32> for SampleRate { type Error = CoreError; }  // CoreError::ZeroSampleRate

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SegmentIndex(u32);
impl SegmentIndex { pub const fn new(index: u32) -> Self; pub const fn get(self) -> u32; }
impl From<SegmentIndex> for usize;
impl Display for SegmentIndex;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TextHash([u8; 32]);
impl TextHash { pub const fn new(bytes: [u8; 32]) -> Self; pub const fn as_bytes(&self) -> &[u8; 32]; }
impl Display for TextHash;                                    // 64 lowercase hex characters
impl FromStr for TextHash { type Err = CoreError; }           // CoreError::InvalidTextHash

pub const PCM_SCALE: f32 = 32_767.0;                          // the scale of a 16-bit stored sample

pub mod app_dirs {
    pub const QUALIFIER: &str = "app";
    pub const ORGANIZATION: &str = "Antenna";
    pub const APPLICATION: &str = "Antenna";
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Display, EnumString, IntoStaticStr)]
pub enum TextFormat { Plain, Markdown }                     // the text forms are "plain" and "markdown"
impl TextFormat { pub fn from_extension(extension: &str) -> Self; }    // "md" and "markdown" give Markdown

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Document { text: Arc<str>, format: TextFormat }
impl Document { pub fn new(text: impl Into<Arc<str>>, format: TextFormat) -> Result<Self, CoreError>; pub fn text(&self) -> &str; pub fn format(&self) -> TextFormat; }

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Segment { index: SegmentIndex, text: Box<str>, source: Range<usize> }
impl Segment { pub fn new(index: SegmentIndex, text: impl Into<Box<str>>, source: Range<usize>) -> Self; pub fn index(&self) -> SegmentIndex; pub fn text(&self) -> &str; pub fn source(&self) -> Range<usize>; }

#[derive(Clone, Debug, PartialEq)]
pub struct PcmChunk(Vec<f32>);
impl PcmChunk { pub fn new(samples: Vec<f32>) -> Self; pub fn samples(&self) -> &[f32]; pub fn into_samples(self) -> Vec<f32>; }

pub type Emit<'a> = &'a mut dyn FnMut(PcmChunk) -> ControlFlow<()>;

// The static descriptor types derive Clone, Copy, Debug, PartialEq and Eq.
pub struct ModelLicense { pub name: &'static str, pub url: &'static str, pub attribution: &'static str }
pub struct Artifact { /* docs/architecture.md section 5.2 */ }
pub enum Extent { /* section 5.2 */ }
impl Extent { pub const fn bytes(self) -> u64; }
pub struct Variant { pub parameters: &'static str, pub artifacts: &'static [Artifact] }
pub struct Variants { pub fast: Variant, pub balanced: Variant, pub max: Variant }
impl Variants { pub const fn get(&self, quality: Quality) -> &Variant; }
pub struct VoiceDescriptor { /* section 5.2: id, language, name, description, artifacts */ }
pub struct EngineDescriptor { /* section 5.2: id, name, version, summary, license, sample_rate, max_segment_chars, variants: Variants, voices */ }
impl EngineDescriptor { pub fn voices_for(&self, language: Language) -> impl Iterator<Item = &'static VoiceDescriptor>; }
pub fn check_descriptor(descriptor: &EngineDescriptor) -> Result<(), CoreError>;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ModelFiles { /* BTreeMap<&'static str, PathBuf>, artifact key to local path */ }
impl ModelFiles { pub fn new(files: BTreeMap<&'static str, PathBuf>) -> Self; pub fn path(&self, key: &'static str) -> Result<&Path, EngineError>; }

pub trait EngineFactory { /* section 5.3 */ }
pub trait Engine { /* section 5.3 */ }
pub trait Output { /* section 5.4 */ }
pub trait Sink { /* section 5.4, begin_segment takes a SegmentIndex */ }

pub enum CoreError {
    EmptyDocument,
    ZeroSampleRate,
    InvalidTextHash,
    InvalidDescriptor { engine: EngineId, defect: Defect },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Defect {
    NoVoices,
    DuplicateVoice(VoiceId),
    ForeignVoice(VoiceId),
    EmptyName(VoiceId),
    EmptyParameters(Quality),
    Revision { artifact: &'static str },
    Sha256 { artifact: &'static str },
    EmptyExtent { artifact: &'static str },
    DuplicateKey { artifact: &'static str },
}

pub enum EngineError {
    MissingFile { key: &'static str },
    Load(#[source] Box<dyn Error + Send + Sync>),
    Inference(#[source] Box<dyn Error + Send + Sync>),
    Runaway { segment: SegmentIndex },
}

pub enum SinkError {
    RateMismatch { expected: SampleRate, actual: SampleRate },
    Closed,
}
```

The error enums derive `Debug` and `thiserror::Error`. No type in this crate has `#[non_exhaustive]`, so other crates match the enums without `_ =>`. Each public type of the four crates of this stage implements `Debug`, because the workspace denies `missing_debug_implementations`. `Registry` and `Harness` hold trait objects, so they implement `Debug` by hand and print the engine ids.

The traits have the signatures of `docs/architecture.md` sections 5.3 and 5.4. `EngineFactory::load` takes the voice, the `Quality` and the `ModelFiles`. `Sink` has `begin_segment`, `write` and `finish`. Each segment index in a signature is a `SegmentIndex`.

`antenna-core` has three error enums. The traits of section 5 return `EngineError` and `SinkError`, and the functions of the core return `CoreError`. `Defect` is data inside `CoreError`. It does not implement `Error`.

`EngineError::Load` and `EngineError::Inference` keep the error of the engine as a boxed source. The core does not know the error types of `candle` or `tokenizers`. `SinkError::Closed` is the error of a sink whose receiver is gone, for example a test output or a timing output.

`ExportFormat` is in `antenna-core` because three crates use it. `antenna-audio` encodes it, `antenna-library` records the last export and the apps show it.

`TextHash` is in `antenna-core` because the library stores it and the apps give it back to the library. `antenna-library` calculates it with SHA-256. The core has no hash dependency.

`PCM_SCALE` is in `antenna-core` because `antenna-library` writes the stored segments and `antenna-audio` reads them. Both crates use this one constant.

`app_dirs` contains the three names that `directories::ProjectDirs::from` takes. `antenna-models` and `antenna-library` use them. The core does not depend on `directories`.

`SampleRate::new` takes a `NonZeroU32`, so a zero rate has no representation. The core defines the rate constants with a private `const fn` that panics on zero. A panic in `const` evaluation is a compile error, and the helper has `#[expect(clippy::panic, reason = "...")]`. Engines use the constants. Code that gets a rate at runtime, for example from the audio device, uses `SampleRate::try_from`.

In a `const` context, `EngineId::new` and `VoiceId::new` panic when a value is not valid. Invalid values are an empty id or key, and a `/` in an id or key. Thus a descriptor with an invalid value does not compile.

`Variants` has one field for each `Quality` value. Thus each descriptor has exactly one variant for each quality, and `Variants::get` cannot fail.

`check_descriptor` returns `CoreError::InvalidDescriptor` with the first `Defect` that it finds. It checks these conditions.

1. The engine has no voices (`NoVoices`).
2. Two voices have the same id (`DuplicateVoice`).
3. A voice id has an engine id that is not the id of the descriptor (`ForeignVoice`).
4. A voice has an empty name (`EmptyName`).
5. A variant has an empty `parameters` text (`EmptyParameters`).
6. A revision is not 40 lowercase hex characters (`Revision`).
7. A SHA-256 value is not 64 lowercase hex characters (`Sha256`).
8. An extent has 0 bytes (`EmptyExtent`).
9. Two artifacts have the same key in the artifacts of one variant plus the artifacts of one voice (`DuplicateKey`). This set is the request of one voice at one quality, and `ModelFiles` uses the keys of this set.

### `antenna-engine-fake`

`antenna-engine-fake` depends only on `antenna-core`. Its dev-dependency is `antenna-engine-testkit`.

1. `FakeFactory` implements `Default`. It has four voices for each language, 24 voices in total. The voice names are "Alba", "Bruno", "Clara" and "Dario" in each language. The voice key is the language code and the lowercase name, for example `fake/es-alba`.
2. Each voice has a `Localized` description in English and Spanish, for example "Test tone, low pitch" and "Tono de prueba, grave".
3. The descriptor has three variants with no artifacts. The `parameters` text of each variant is "0". The voices have no artifacts.
4. Sample rate `SampleRate::HZ_24000`. `max_segment_chars` is 400.
5. For each segment, the engine makes a sine tone. The base frequency of the four voices is 220, 247, 262 and 294 Hz. The frequency is the base frequency multiplied by `1 + (index mod 4)`, where `index` is the segment index. The duration is 40 ms for each character of the segment text. The amplitude is 0.25.
6. The quality changes nothing in the audio.
7. The engine emits chunks of 480 samples.
8. `FakeFactory::with_fault(fault: Fault) -> Self` makes an engine that fails on purpose. `Fault` has the variants `PanicAt { segment: SegmentIndex }` and `FailAt { segment: SegmentIndex }`. Stage 06 uses these faults.

### `antenna-engine-testkit`

```rust
pub struct RecordingOutput { /* shares a buffer with its Recording */ }
impl RecordingOutput { pub fn new() -> (Self, Recording); }
pub struct Recording { /* flume::Receiver of the sink events */ }
impl Recording { pub fn collect(self) -> RecordedAudio; }
pub struct RecordedAudio {
    pub rate: SampleRate,
    pub samples: Vec<f32>,
    pub segment_starts: Vec<(SegmentIndex, usize)>,   // segment index and its first sample index
    pub is_finished: bool,
}

pub type FilesFn<'a> = &'a dyn Fn(&VoiceDescriptor, Quality) -> ModelFiles;

pub struct Harness<'a> { factory: &'a dyn EngineFactory, files: FilesFn<'a> }
impl<'a> Harness<'a> {
    pub fn new(factory: &'a dyn EngineFactory, files: FilesFn<'a>) -> Self;
    pub fn check_descriptor(&self) -> Result<(), Violation>;
    pub fn check_audio(&self) -> Result<(), Violation>;
    pub fn check_break(&self) -> Result<(), Violation>;
    pub fn check_determinism(&self) -> Result<(), Violation>;
    pub fn check_reset(&self) -> Result<(), Violation>;
    pub fn check_edge_segments(&self) -> Result<(), Violation>;
}

#[derive(Debug, thiserror::Error)]
pub struct Violation {
    pub voice: Option<VoiceId>,
    pub quality: Option<Quality>,
    pub condition: Condition,
    #[source] pub source: Option<Cause>,                // None when the engine gives wrong audio
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, strum::Display)]   // the text form states the condition
pub enum Condition { Descriptor, Load, Synthesis, NonEmptyAudio, FiniteSamples, Peak, Break, Determinism, Reset }

#[derive(Debug, thiserror::Error)]
pub enum Cause {
    Descriptor(CoreError),                             // the error of check_descriptor, with the defect
    Engine(EngineError),                               // the error of load or synthesize
}

#[macro_export]
macro_rules! conformance {
    ($factory:expr, $files:expr) => { /* one #[test] for each check */ };
    ($factory:expr, $files:expr, ignore = $reason:literal) => { /* the same tests, each with #[ignore = $reason] */ };
}
```

An engine crate writes `antenna_engine_testkit::conformance!(factory, files)` in `tests/conformance.rs`. An engine that needs downloaded models writes `conformance!(factory, files, ignore = "needs models")`. `files` is a closure that returns the `ModelFiles` of a voice and a quality.

Each check except `check_descriptor` runs on the first voice of each language, one time for each `Quality`. A `Violation` names the voice, the quality and the failed condition. If an error of the core or of the engine broke the condition, the `Violation` keeps that error as its source. `check_descriptor` gives a `Violation` with no voice and no quality.

The checks have these definitions.

| Check | Pass condition |
|---|---|
| `check_descriptor` | `antenna_core::check_descriptor` returns `Ok` |
| `check_audio` | A short segment ("Hello.") and a segment of `max_segment_chars` characters give more than 0 samples. All samples are finite. The peak is 1.0 or less |
| `check_break` | When `emit` returns `Break` on the first chunk, `synthesize` returns `Ok` and does not call `emit` again |
| `check_determinism` | Two calls with the same segment give the same samples, bit for bit |
| `check_reset` | A call after a `Break` gives the same samples as a call on a new engine |
| `check_edge_segments` | The segments "Hi", "...", "1 2 3" and `"Ok \u{1F399}"` (a word and a pictograph) each return `Ok` |

### `antenna-engine-registry`

```rust
pub struct Registry { /* factories and defaults */ }
impl Registry {
    pub fn new() -> Result<Self, RegistryError>;               // the engines of the enabled features
    pub(crate) fn from_parts(factories: Vec<Arc<dyn EngineFactory>>, defaults: &[VoiceId]) -> Result<Self, RegistryError>;
    pub fn factories(&self) -> impl Iterator<Item = &Arc<dyn EngineFactory>>;
    pub fn engine(&self, id: &str) -> Result<&Arc<dyn EngineFactory>, RegistryError>;   // the text of an engine id, for example "qwen3"
    pub fn voices(&self, language: Language) -> impl Iterator<Item = &'static VoiceDescriptor>;
    pub fn voice(&self, id: &str) -> Result<&'static VoiceDescriptor, RegistryError>;
    pub fn default_voice(&self, language: Language) -> &'static VoiceDescriptor;
}

pub enum RegistryError {
    NoVoice(Language),
    UnknownVoice(String),
    UnknownEngine(String),
    Descriptor(#[source] CoreError),
}
```

1. The feature `engine-fake` enables the `fake` engine. The registry has no default features. Each app declares the engine features that it builds and forwards them to the registry. Stage 12 adds `engine-qwen3` and stage 14 adds `engine-magpie`, each with its `[workspace.dependencies]` entry.
2. The constant `DEFAULT_VOICES` contains the default voice id for each language. In this stage it is empty. Each entry has the `#[cfg(feature = "engine-<name>")]` attribute of its engine, so a build without that engine has no entry for it.
3. If `DEFAULT_VOICES` has no entry for a language, the default voice is the first voice for that language in registration sequence. The same rule applies when the entry names a voice that no registered factory has.
4. `Registry::new` collects the factories of the enabled features and `DEFAULT_VOICES`, then calls `from_parts`. All rules of this list are in `from_parts`, and the unit tests of the registry call it with test factories and test defaults. `from_parts` calls `check_descriptor` for each factory. It returns `RegistryError::Descriptor` for the first error.
5. `from_parts` returns `RegistryError::NoVoice(language)` if a language has no voice. Thus `default_voice` cannot fail.
6. `Registry::voice` takes the `Display` text of a voice id. It returns `RegistryError::UnknownVoice` with that text if no voice has it.
7. `Registry::engine` takes the text of an engine id. It returns `RegistryError::UnknownEngine` with that text if no factory has it. The CLI and the desktop app use it to find an engine from text.

### ADRs

Write the ADRs 0001 to 0007, 0011 and 0012 of `docs/architecture.md` section 16. Stage 04 writes ADR 0008, stage 08 writes ADR 0009 and stage 05 writes ADR 0010. Each ADR has the sections "Status", "Context", "Decision" and "Consequences" and has a maximum of 60 lines. Use STE-80.

## Tasks

1. Run `git init` in `antenna/`. Add a `.gitignore` for `target/` and the macOS `.DS_Store` files.
2. Write the workspace files of the "Workspace" deliverable.
3. Write `LICENSE` with the full GPL-3.0 text from gnu.org. Write a `README.md` with one paragraph about the product, the build command and the license.
4. Make the `xtask` crate in `tools/xtask/`. Implement `check` first, then each check of `lint-repo`.
5. For each `lint-repo` check, add a test with a fixture file that contains the violation. The test makes sure that the check finds it.
6. Make `antenna-core`. Write the types, then the traits, then `check_descriptor`.
7. Add a `compile_fail` doctest for each `const fn` constructor that can get an invalid value.
8. Add a doctest that compiles to each trait in `antenna-core`.
9. Write the test `core_types_have_required_derives`. It calls a generic function with trait bounds for each type of the derive list.
10. Make `antenna-engine-testkit` with `RecordingOutput`, `Harness` and the `conformance!` macro.
11. Write the self-tests of the test kit. For each check, make a defective engine in the test file and make sure that the check returns `Err`.
12. Make `antenna-engine-fake`. Add `antenna_engine_testkit::conformance!(FakeFactory::default(), &|_, _| ModelFiles::default())` to `tests/conformance.rs`.
13. Make `antenna-engine-registry` with the `engine-fake` feature.
14. Write the CI workflow. Use a matrix of `macos-latest`, `ubuntu-latest` and `windows-latest`. Install the tools with `taiki-e/install-action`. Cache with `Swatinem/rust-cache`. Set the environment variable `NEXTEST_PROFILE=ci` for the job, so nextest uses the `ci` profile of `.config/nextest.toml`. Run `cargo xtask check`.
15. Write the ADRs 0001 to 0007, 0011 and 0012.
16. Run `cargo xtask check`. Fix each failure.
17. Do the quality gate in `docs/standards.md` section 20.

## Acceptance criteria

| ID | Criterion | Check |
|---|---|---|
| AC-01-01 | All checks pass on a clean clone | `cargo xtask check` exits with code 0 |
| AC-01-02 | CI passes on macOS, Linux and Windows | The CI run of the push to `main` is green on the three jobs |
| AC-01-03 | Each `lint-repo` check finds its violation | One xtask test for each row of `docs/standards.md` section 13, named `lint_repo_finds_<check>` |
| AC-01-04 | `lint-repo` finds a dependency that the graph does not permit | Test `lint_repo_finds_forbidden_dependency` |
| AC-01-05 | `antenna-core` has only `thiserror` and `strum` as normal dependencies | `cargo tree -p antenna-core -e normal --depth 1` shows only these two |
| AC-01-06 | Invalid constant ids do not compile, and a zero sample rate is rejected at runtime | `compile_fail` doctests on `EngineId::new` and `VoiceId::new`, and test `sample_rate_try_from_fails_when_zero` |
| AC-01-07 | `Document::new` rejects an empty or whitespace-only text | Test `document_rejects_text_when_blank` |
| AC-01-08 | `check_descriptor` finds each of its nine defects | One test for each `Defect` variant, named `check_descriptor_fails_when_<defect>` |
| AC-01-09 | Each test kit check finds a defective engine | Self-tests `harness_<check>_fails_when_<defect>`, one for each check |
| AC-01-10 | The `fake` engine passes the conformance suite | `cargo nextest run -p antenna-engine-fake` |
| AC-01-11 | The registry has a default voice for each language when `engine-fake` is enabled | Test `registry_has_default_voice_when_fake_enabled` |
| AC-01-12 | The registry fails when a language has no voice | Test `registry_fails_when_language_has_no_voice`, which calls `from_parts` with a factory that has no Spanish voice |
| AC-01-13 | `Registry::voice` finds each voice by its `Display` text | Test `registry_finds_voice_when_given_display_text` |
| AC-01-14 | Each trait has a doctest that compiles | `cargo test --doc -p antenna-core` runs 4 or more trait doctests |
| AC-01-15 | The ADRs 0001 to 0007, 0011 and 0012 exist and each has 60 lines or less | `ls docs/adr` shows the nine files, and `wc -l docs/adr/*.md` shows 60 or less for each |
| AC-01-16 | The workspace license is GPL-3.0-or-later and `cargo deny check` passes | `cargo deny --config .config/deny.toml check licenses` exits with code 0 |
| AC-01-17 | `TextFormat::from_extension` maps file extensions | Test `text_format_is_markdown_when_extension_is_md` |
| AC-01-18 | The `fake` engine has four voices with names for each language | Test `fake_has_four_named_voices_for_each_language` |
| AC-01-19 | The conformance suite runs each check for each quality | Self-test `harness_check_audio_fails_when_one_quality_is_silent` |
| AC-01-20 | `Variants::get` returns the variant of each quality | Test `variants_get_returns_field_when_quality_given` |
| AC-01-21 | The text forms of `Language`, `Quality` and `ExportFormat` round trip, and the defaults are `Balanced` and `Mp3` | Tests `language_round_trips_when_text_parsed`, `quality_round_trips_when_text_parsed` and `export_format_round_trips_when_text_parsed` |
| AC-01-22 | `RecordingOutput` records the first sample of each segment | Test `recording_marks_segment_starts_when_segments_begin` |
| AC-01-23 | The repository root contains only the permitted entries, and the crates are in the directories of `docs/architecture.md` section 3 | `git ls-tree --name-only HEAD` shows only the entries of deliverable 6. `cargo metadata` shows the manifest paths `crates/core`, `crates/engines/testkit`, `crates/engines/fake`, `crates/engines/registry` and `tools/xtask` |
| AC-01-24 | `Registry::voice` returns `UnknownVoice` for a text that names no voice | Test `registry_fails_when_voice_unknown` |
| AC-01-25 | The registry rejects a factory with an invalid descriptor | Test `registry_fails_when_descriptor_invalid`, which calls `from_parts` with a factory whose descriptor has no voices |
| AC-01-26 | `TextHash` round trips through its hex text and rejects invalid text | Tests `text_hash_round_trips_when_hex_parsed` and `text_hash_fails_when_text_not_64_hex_characters` |
| AC-01-27 | Each core type has the derives of the API block | Test `core_types_have_required_derives` |
| AC-01-28 | The `ignore` form of `conformance!` marks each generated test as ignored | `crates/engines/testkit/tests/ignored.rs` uses `conformance!` with a test engine of the file and `ignore = "needs models"`. `cargo nextest list -p antenna-engine-testkit --run-ignored only` lists its six tests, and `cargo nextest run` skips them |
| AC-01-29 | No crate has `#[non_exhaustive]` | `cargo xtask lint-repo` passes, and test `lint_repo_finds_non_exhaustive` passes |
| AC-01-30 | `Registry::engine` finds an engine by its text and rejects an unknown text | Tests `registry_finds_engine_when_given_id_text` and `registry_fails_when_engine_unknown` |
| AC-01-31 | A default entry with no registered factory falls back to the first voice of the language | Test `registry_uses_first_voice_when_default_entry_has_no_factory`, which calls `from_parts` with a default voice id of an engine that is not in the factory list |
| AC-01-32 | `FakeFactory::with_fault` fails or panics at the given segment | Tests `fake_returns_error_when_fault_fail_at_segment` and `fake_panics_when_fault_panic_at_segment` |
| AC-01-33 | The `fake` tone has the frequency and the duration of deliverable 5 | Tests `fake_tone_lasts_40_ms_for_each_character` and `fake_tone_frequency_follows_segment_index` |
| AC-01-34 | The text forms of `TextFormat` round trip | Test `text_format_round_trips_when_text_parsed` |
| AC-01-35 | CI uses the nextest `ci` profile | `grep -c 'NEXTEST_PROFILE: ci' .github/workflows/ci.yml` prints `1` |
| AC-01-36 | Each member inherits its workspace dependencies, and the registry has no default features | `cargo xtask lint-repo` passes the "Versions" row, and `grep -c '^default' crates/engines/registry/Cargo.toml` prints `0` |

## Decision rules

1. If a `strum` derive causes a pedantic lint, put `#[expect(lint, reason = "...")]` on the type.
2. If `cargo machete` reports a crate that only a macro uses, add the crate to `[package.metadata.cargo-machete] ignored` in that crate.
3. If a `lint-repo` check gives a different result on Windows, compare paths with `Path::components`. Do not compare path strings.
4. If `cargo deny` reports a license of a transitive dependency that is not in the allowlist, stop and write a "Blocked" section. Do not add the license to the allowlist.
5. If the trait signatures of `docs/architecture.md` section 5 do not compile as written, change the smallest possible detail. Write the change and its reason in ADR 0004.
6. If `thiserror` cannot derive `Error` for the struct `Violation`, implement `Display` and `Error` by hand in the test kit.

## Out of scope

1. Any crate that is not in the "Scope" section.
2. The nightly CI job. Stage 09 adds it.
3. Linux audio packages in CI. Stage 03 adds them.
