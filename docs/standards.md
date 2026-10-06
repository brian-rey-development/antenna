# Antenna Code Standards

These standards apply to all Rust code in this repository. Each rule ends with the check that enforces it.

| Tag | Check |
|---|---|
| `[clippy]` | A compiler or clippy lint from section 2 |
| `[lint-repo]` | A check of `cargo xtask lint-repo` from section 13 |
| `[check]` | Another step of `cargo xtask check` from section 1, for example `cargo fmt`, `cargo deny` or `cargo machete` |
| `[test]` | A test that the stage writes |
| `[review]` | The independent review of section 20. No tool checks the rule, so the reviewer must check it |

A change to this file needs an ADR in `docs/adr/`.

## 1. The single check command

Run this command before each commit.

```sh
cargo xtask check
```

The command runs these checks in this sequence. It stops at the first failure.

1. `cargo fmt --all --check`
2. `cargo clippy --workspace --all-targets --all-features -- -D warnings`
3. `cargo nextest run --workspace --all-features`
4. `cargo test --workspace --all-features --doc`
5. `cargo doc --workspace --all-features --no-deps` with `RUSTDOCFLAGS="-D warnings"`
6. `cargo deny check --config .config/deny.toml`
7. `cargo machete`
8. `cargo xtask lint-repo` (the repository checks in section 13)

CI runs the same command. There is no other check command. The CI workflows set `NEXTEST_PROFILE=ci`, so step 3 uses the nextest profile `ci` in CI. `xtask` does not select a profile.

No crate has a feature that builds on one platform only. Platform-specific dependencies use `[target.'cfg(...)'.dependencies]`, for example the candle Metal backend in `antenna-ml`. Thus `--all-features` builds on macOS, Linux and Windows. `[review]`

## 2. Toolchain and workspace configuration

| File | Content |
|---|---|
| `rust-toolchain.toml` | One pinned stable version, with the `rustfmt` and `clippy` components |
| `Cargo.toml` | `resolver = "3"`, edition 2024, `[workspace.package]`, `[workspace.dependencies]`, `[workspace.lints]`, profiles |
| `.rustfmt.toml` | `edition = "2024"`, `use_field_init_shorthand = true`, `use_try_shorthand = true` |
| `.clippy.toml` | The thresholds and the disallowed items in section 2.2 |
| `.config/deny.toml` | The license allowlist, the advisory database and the duplicate-version policy |
| `.config/nextest.toml` | A `default` profile and a `ci` profile with `fail-fast = false` and retries set to 0 |
| `.cargo/config.toml` | The `xtask` alias only. The alias runs the `xtask` package in `tools/xtask/` |

Each member crate has `[lints] workspace = true`. A crate must not override the workspace lints. `[lint-repo]`

### 2.1 Workspace lints

```toml
[workspace.lints.rust]
unsafe_code = "deny"
missing_docs = "warn"
unreachable_pub = "warn"
unused_qualifications = "warn"
let_underscore_drop = "warn"
non_ascii_idents = "forbid"
rust_2018_idioms = { level = "warn", priority = -1 }
missing_debug_implementations = "warn"
trivial_numeric_casts = "warn"

[workspace.lints.clippy]
pedantic = { level = "warn", priority = -1 }
module_name_repetitions = "allow"
must_use_candidate = "allow"

allow_attributes = "warn"
allow_attributes_without_reason = "warn"
absolute_paths = "warn"
clone_on_ref_ptr = "warn"
cognitive_complexity = "warn"
dbg_macro = "warn"
derive_partial_eq_without_eq = "warn"
expect_used = "warn"
float_cmp_const = "warn"
get_unwrap = "warn"
if_then_some_else_none = "warn"
indexing_slicing = "warn"
iter_over_hash_type = "warn"
large_include_file = "warn"
let_underscore_must_use = "warn"
let_underscore_untyped = "warn"
lossy_float_literal = "warn"
map_err_ignore = "warn"
mem_forget = "warn"
missing_asserts_for_indexing = "warn"
panic = "warn"
partial_pub_fields = "warn"
print_stderr = "warn"
print_stdout = "warn"
redundant_clone = "warn"
redundant_type_annotations = "warn"
self_named_module_files = "warn"
shadow_unrelated = "warn"
str_to_string = "warn"
string_slice = "warn"
tests_outside_test_module = "warn"
todo = "warn"
undocumented_unsafe_blocks = "warn"
unimplemented = "warn"
unused_result_ok = "warn"
unwrap_in_result = "warn"
unwrap_used = "warn"
use_self = "warn"
wildcard_enum_match_arm = "warn"
```

CI uses `-D warnings`, so each warning stops the build. Local builds show warnings and continue, so you can iterate.

### 2.2 Clippy thresholds and disallowed items

```toml
too-many-lines-threshold = 25
cognitive-complexity-threshold = 10
too-many-arguments-threshold = 5
type-complexity-threshold = 200
absolute-paths-max-segments = 2
max-fn-params-bools = 0
max-include-file-size = 65536
allow-unwrap-in-tests = true
allow-expect-in-tests = true
allow-panic-in-tests = true
allow-indexing-slicing-in-tests = true
allow-print-in-tests = true
disallowed-names = ["foo", "bar", "baz", "tmp", "temp", "data", "info", "item", "obj", "val", "thing", "stuff"]
disallowed-methods = [
  { path = "std::thread::sleep", reason = "Use flume recv_timeout, or park_timeout for a wait that reads a cancel flag" },
  { path = "std::thread::spawn", reason = "Use std::thread::Builder with a name antenna-<role>" },
  { path = "std::process::exit", reason = "Return an error from main" },
  { path = "std::sync::mpsc::channel", reason = "Use flume" },
  { path = "std::sync::mpsc::sync_channel", reason = "Use flume" },
]
disallowed-types = [
  { path = "std::sync::Mutex", reason = "Give the state to one thread and send commands through flume" },
  { path = "std::rc::Rc", reason = "Use ownership, a borrow or Arc" },
  { path = "std::cell::RefCell", reason = "Use ownership or a GPUI entity" },
  { path = "std::sync::RwLock", reason = "The library index is the only lock. index.rs has the inner #![expect]" },
]
```

The function limit is 25 lines, because `rustfmt` puts each element of a long chain on a new line. `max-include-file-size` is 64 KB, the size limit of a voice prompt file. The font files of `antenna-ui` are larger. The module that embeds them has `#[expect(clippy::large_include_file, reason = "...")]` with the font name.

If a GPUI API needs a disallowed type, put `#[expect(clippy::disallowed_types, reason = "...")]` on the smallest item that uses it. The reason names the GPUI API.

### 2.3 Lint exceptions

1. Use `#[expect(lint_name, reason = "...")]`. Do not use `#[allow]`. `[clippy]`
2. Put the exception on the smallest item that needs it. `[review]`
3. Write a reason that a reviewer can check, for example `reason = "PCM sample counts fit in u32 for segments below 24 h"`. `[review]`

`#[expect]` causes a warning when the lint stops firing, so stale exceptions cannot stay in the code.

## 3. Structure

### 3.1 Crates

1. Each crate has one responsibility. `[review]`
2. The group directories under `crates/` are exactly `storage/`, `inference/` and `engines/`. Each other directory under `crates/` is one crate with the name `antenna-<directory>`, for example `crates/text` is `antenna-text`. `[lint-repo]`
3. A crate in `storage/` or `inference/` has the name `antenna-<directory>`, for example `crates/storage/library` is `antenna-library`. A crate in `engines/` has the name `antenna-engine-<directory>`, for example `crates/engines/qwen3` is `antenna-engine-qwen3`. `[lint-repo]`
4. A new crate or a new group needs a human decision. Write a Blocked section in the stage file. A human changes `docs/architecture.md` section 3 and writes an ADR. The dependency graph check fails for a workspace member that has no row in the graph. `[lint-repo]`
5. `apps/` contains only binaries that Antenna ships. `tools/` contains only developer tools that Antenna does not ship. `[review]`
6. The table in `docs/architecture.md` section 3.1 is the only permitted dependency graph. `[lint-repo]`
7. The packages outside `crates/` have fixed names. `apps/cli` is `antenna-cli` with the binary `antenna`. `apps/desktop` is `antenna-desktop`. `tools/eval` is `antenna-eval`. `tools/xtask` is `xtask`. `[lint-repo]`

### 3.2 Modules and files

1. A module with submodules is a directory with `mod.rs`. Do not use `foo.rs` next to a `foo/` directory. `[clippy]`
2. `lib.rs` contains only module declarations, `pub use` re-exports and the crate documentation. `[review]`
3. A file has a maximum of 300 lines, tests included. `[lint-repo]`
4. The names `util`, `utils`, `helper`, `helpers`, `common`, `misc`, `shared`, `manager`, `base`, `foundation`, `platform`, `infra`, `libs`, `services` and `core-utils` are prohibited names. The rule applies to modules, files, types and directories. A file stem or a directory name matches when it is a prohibited name. A type name matches when one of its words is a prohibited name, for example `PlayerShared`. The rule checks only the types that a `struct`, `enum`, `trait` or `type` item of the workspace declares. A type of another crate, for example `SharedString` of GPUI, does not match. Each of these names is a container and hides a missing concept. `[lint-repo]`
5. Unit tests go in a `#[cfg(test)] mod tests` block at the end of the file. Integration tests go in `tests/`. The parity modules of section 3.4 are the only exception. `[clippy]`
6. A test or example target with more than one file is a directory with `main.rs`, for example `tests/pipeline/main.rs`. `[review]`

### 3.3 Visibility

1. Items are private by default. Use `pub(crate)` for items that other modules of the crate use. `[clippy]`
2. Use `pub` only for the public API of the crate. Re-export each public item at the crate root in `lib.rs`. Users of the crate write `antenna_<crate>::<Item>`, for example `antenna_ml::Streaming` and `antenna_ml::pace`, with no module segment. `[review]`

The `unreachable_pub` lint finds `pub` items that are not part of the public API.

### 3.4 Engine crate layout

All engine crates use the same layout. A reader who knows one engine can find each part of all the other engines. `[review]`

```
crates/engines/<name>/
├── Cargo.toml
├── src/
│   ├── lib.rs            # pub use factory::Factory;
│   ├── descriptor/       # mod.rs: the static EngineDescriptor, artifacts.rs: the Artifact constants, voices.rs: the VoiceDescriptor list
│   ├── factory.rs        # impl EngineFactory: parse config, load weights, build the Engine
│   ├── engine.rs         # impl Engine: the synthesis loop for one segment
│   ├── error.rs          # the pub(crate) error enum of the engine, boxed into EngineError
│   ├── config.rs         # the typed model config, parsed from the downloaded config file (config/ with mod.rs for more than one config)
│   ├── frame.rs          # the codebook count and the Frame type that the model and the codec share
│   ├── frontend/         # mod.rs: text to model input (tokenizer, G2P, prompt format), voice.rs: the voice input
│   ├── model/            # mod.rs, one file for each part of the network, parity.rs
│   └── codec/            # mod.rs: model output to PCM with the Streaming types of antenna-ml, parity.rs
├── voices/               # voice prompt files that the crate embeds with include_bytes!
├── assets/               # other small files that the crate embeds, for example a token table
├── benches/              # divan benchmarks of the codec and the model
└── tests/
    ├── conformance.rs    # antenna_engine_testkit::conformance!(Factory, files)
    └── fixtures/         # parity fixtures from tools/reference/<name> (in-house models only)
```

1. Each `load` function of the crate takes `&ModelFiles`, and the device when it needs one.
2. `error.rs` has a `pub(crate)` error enum with data variants and `From<MlError>`. `antenna_ml::pace` uses it. `Factory::load` and `Engine::synthesize` box it one time into `EngineError::Load` or `EngineError::Inference`.
3. The artifact constants are in `descriptor/artifacts.rs`. The stage that first needs an artifact writes its constant.
4. Parity tests need the `pub(crate)` modules of the engine. Thus they are `#[cfg(test)]` modules with the name `parity.rs`, in the directory of the module that they test. For example, `src/codec/parity.rs` tests `src/codec/mod.rs`.
5. A parity test gets its files through `ModelStore::open_default()?.ensure(...)` and has the attribute `#[ignore = "needs models"]`.
6. `config.rs`, `frame.rs`, `frontend/`, `model/`, `codec/`, `voices/`, `assets/` and `benches/` exist only when the engine needs them. When an engine wraps an external crate, `model/mod.rs` contains the adapter and nothing else.
7. The `version` of the descriptor is an integer string that starts at `"1"`. Increase it with each change that changes the audio.

## 4. Naming

1. Follow the Rust API Guidelines naming rules (RFC 430). `[clippy]`
2. Use complete words. Do not use abbreviations, except the abbreviations in this table.

   | Kind | Permitted abbreviations |
   |---|---|
   | Audio and units | `pcm`, `ms`, `hz`, `kbps`, `rms`, `fft`, `lufs`, `dbtp` |
   | Speech and models | `tts`, `g2p`, `kv`, `lm`, `cfg`, `fsq`, `ipa`, `bos`, `eos`, `dtype`, `vocab`, `conv` |
   | Measurement | `rtf`, `ttfa`, `wer` |
   | General | `id`, `config`, `repo`, `dirs`, `env`, `args` |

   File format and protocol names, for example `mp3`, `gguf`, `http` and `uuid`, are names, so you can use them. `[review]`
3. Use the glossary terms in `docs/writing.md` for types, functions and messages. One concept has one name in the code and in the documents. `[review]`
4. Constructors are `new`, `from_<source>` or `with_<option>`. A constructor that opens a resource on disk is `open` or `open_<location>`. A constructor that derives a value from one input is `for_<input>`, for example `Seed::for_segment`. A constructor that starts a thread is `start`, for example `Player::start`. `Pipeline::new` is the one exception, because `Pipeline::start` starts a job. Conversions use `From`, `TryFrom` and `AsRef`. `[review]`
5. Boolean values, fields and functions that return `bool` start with `is_`, `has_` or `can_`, for example `JobEvent::Synthesized { is_cached }`. The fields of a `clap` argument struct are the exception, because they mirror the flag names. `[review]`
6. Test names describe a behavior with the pattern `<subject>_<result>_when_<condition>`, for example `segmenter_keeps_abbreviation_when_name_follows`. A test of an invariant with no condition uses `<subject>_<result>`, for example `segments_cover_document_in_order`. `[review]`

## 5. Types and API design

1. Use a newtype for each domain value. A bare `String`, `u32` or `f32` does not cross a crate boundary when it has a domain meaning. A value has a domain meaning when the domain gives it a unit or a valid range. Examples are a sample rate, a volume, a segment index and a track id. A count of items and a progress fraction for display are plain values. `[review]`
2. Make invalid states impossible to represent. Use enums for states and `NonZero*` for values that cannot be zero. `[review]`
3. Do not use `#[non_exhaustive]`. No crate of this workspace is published, so each match stays exhaustive and the compiler finds each match that a new variant affects. `[lint-repo]`
4. Match all variants of the enums of this workspace. Do not use `_ =>` on these enums. `[clippy]`
5. Use a trait only when there are two or more implementations, or when the trait is a documented extension point. The extension points are `EngineFactory`, `Engine`, `Output` and `Sink`. `[review]`
6. Use `impl Trait` in argument position for static dispatch. Use a named generic parameter only when the same type occurs two or more times in the signature. Use `dyn Trait` at the extension points, where the composition root selects the implementation at runtime. `[review]`
7. Take parameters as borrowed types (`&str`, `&[T]`, `&Path`) when the function does not keep them. `[clippy]`
8. Use a builder only when a type has more than three optional fields. `[review]`
9. Do not use `Rc`, `RefCell`, global mutable state or `lazy_static`. Use `std::sync::LazyLock` for immutable static data that needs computation. `[clippy]`
10. Use a named constant for each literal with a meaning. Put the unit in the name, for example `RING_BUFFER_DURATION_MS` and `MP3_BITRATE_KBPS`. `[review]`

## 6. Errors

1. Library crates use `thiserror`. Each library crate has one public error enum, for example `ModelError`, `TextError` and `JobError`. `antenna-core` has three, `CoreError`, `EngineError` and `SinkError`, because the traits of `docs/architecture.md` section 5 return them. `[review]`
2. App crates use `anyhow` in `main` only. `[review]`
3. An error variant contains the data that a user or developer needs to act. Examples are the path, the file name, the expected value and the actual value. `[review]`
4. An error variant keeps the underlying error as a `#[source]` field with its concrete type, for example `std::io::Error`, `hound::Error` or `toml::de::Error`. Use a `reason: &'static str` field only when no underlying error exists. The one exception is the boxed engine error in `EngineError::Load` and `EngineError::Inference` (section 3.4). `[review]`
5. Do not log an error and also return it. The code that handles the error logs it. `[review]`
6. Do not convert an error to a string before the UI layer. `[review]`
7. Do not panic in library code. `unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!` and indexing without a bounds proof are lint errors outside tests. `[clippy]`
8. Do not discard a `Result`. `let _ =` on a `Result` is a lint error. `[clippy]`
9. Use `#[from]` only on a variant that is the only variant with that source type. In all other cases, convert the error with a closure that adds the context, for example the path. `[review]`

## 7. Concurrency

1. Synthesis, playback feed, review and downloads run on dedicated `std::thread` workers. Library crates do not use `async`. `[review]`
2. Use `flume` for events and commands between threads. `[clippy]`
3. Use `rtrb` for audio samples to the audio device. The audio callback does not allocate memory, does not lock and does not do I/O. `[test]`
4. Use atomics only for single values that a hot path reads, for example the play position. `[review]`
5. The only lock is the `RwLock` of the library index in `docs/architecture.md` section 6.7. `index.rs` of `antenna-library` starts with the inner attribute `#![expect(clippy::disallowed_types, reason = "...")]`, because the lint fires on the `use` line, the field type and the constructor. A thread that owns its state and receives commands through `flume` needs no lock. `[clippy]`
6. Start each thread with `std::thread::Builder`. `[clippy]`
7. Give each thread a name with the pattern `antenna-<role>`. `[review]`
8. Hold the library lock for the shortest possible scope. Do not hold it during inference or I/O. `[review]`

## 8. Performance

1. Do not allocate memory inside a per-sample or per-frame loop. Allocate before the loop with `Vec::with_capacity` and reuse the buffer. `[review]`
2. Do not use `clone()` to satisfy the borrow checker. Change the ownership or the lifetimes. The `redundant_clone` lint finds a part of these clones. `[clippy]`
3. Do not add `#[inline]` without a benchmark that shows the improvement. `[review]`
4. Each performance claim in a PR has a benchmark or a `tracing` span measurement. `[review]`
5. Load model weights with memory mapping when the format permits it (safetensors, GGUF). `[review]`
6. The performance budgets in `docs/architecture.md` section 8 are acceptance criteria. A change that makes a budget fail is a defect. `[review]`

## 9. Unsafe code and FFI

1. Only `crates/inference/ml/src/weights.rs` can contain `unsafe` code. Memory-mapped weights in candle need it. ADR 0009 gives the safety argument. `[lint-repo]`
2. Each `unsafe` block in that file has `#[expect(unsafe_code, reason = "...")]` and a `// SAFETY:` comment that tells why the block is sound. `[clippy]`
3. Use a maintained crate for each FFI binding, for example `mp3lame-encoder` and `cpal`. `[review]`
4. If more `unsafe` code is necessary, write a Blocked section. A human writes the ADR before the code. `[lint-repo]`

## 10. Comments and documentation

1. The default is zero comments. Write a comment only to tell why. Valid reasons are a hidden constraint, a numeric invariant, a workaround for a specific defect, or a reference to a paper or a specification. The comment density check of section 13 limits the count. `[review]`
2. Each comment that refers to an external source includes the link or the section number. `[review]`
3. Do not put commented-out code, banner lines, `TODO`, `FIXME`, `XXX`, `HACK`, the word "Note" or emoji in comments. `[lint-repo]`
4. Each `pub` item in a library crate has a `///` doc comment. The first sentence tells what the item is or does, in one line. `[clippy]`
5. Each public function that returns `Result` has an `# Errors` section. `[clippy]`
6. Write all documentation in the STE-80 profile from `docs/writing.md`. `[review]`
7. The `EngineFactory`, `Engine`, `Output` and `Sink` traits have a doctest that compiles. `[test]`
8. Do not write a comment that tells what the next line does. `[review]`

The comment density check of rule 1 counts only `//` lines. It does not count `///`, `//!` or `// SAFETY:` lines.

## 11. Logging

1. Use `tracing`. Do not use `println!` or `eprintln!`, except in one output module of each binary that writes to a terminal. The output module is `src/output.rs` in `antenna-cli`, `antenna-eval` and `xtask`. Each output module starts with `#![expect(clippy::print_stdout, clippy::print_stderr, reason = "...")]`. `[clippy]`
2. Use structured fields. Write `info!(segments = count, "job started")`. Do not interpolate values into the message. `[review]`
3. Use the levels with the meanings in this table. `[review]`

| Level | Meaning | Example |
|---|---|---|
| `error` | A job or an operation failed and the user sees the failure | Model download failed after four attempts |
| `warn` | The system recovered from a problem | The output device changed |
| `info` | A lifecycle event | Job started, model loaded |
| `debug` | One event for each segment | Segment synthesized in 84 ms |
| `trace` | One event for each chunk | Chunk of 1920 samples emitted |

4. Use a `tracing` span for each operation that has a performance budget, which are `prepare`, `load`, `synthesize` and `encode`. `[review]`

## 12. Dependencies

1. Declare each version once, in `[workspace.dependencies]`. Member crates use `name.workspace = true`. `[lint-repo]`
2. Use only the crates in the table in `docs/architecture.md` section 10. If you need another crate, write a Blocked section. A human changes the table. `[review]`
3. Set `default-features = false` in `[workspace.dependencies]` and enable only the features that the code uses. A member crate adds features with `features = [...]` next to `workspace = true`. Cargo does not accept `default-features = false` on an inherited dependency when the workspace entry does not set it. `[review]`
4. `cargo deny` permits only licenses compatible with GPL-3.0-or-later. `[check]`
5. `cargo machete` finds unused dependencies. `[check]`

## 13. Repository checks

`cargo xtask lint-repo` does these checks. Each check prints the file and the line. The checks read only the files that git tracks. "Rust files" means the `*.rs` files under `crates/`, `apps/` and `tools/`.

| Check | Files | Failure condition |
|---|---|---|
| File length | Rust files | A file has more than 300 lines |
| Banned markers | Rust files | A comment contains `TODO`, `FIXME`, `XXX`, `HACK`, the word "Note" or an emoji |
| Banner comments | Rust files | A `//` line (not `///` or `//!`) contains three or more `=`, `-`, `*` or `#` in sequence |
| Commented-out code | Rust files | A `//` line (not `///` or `//!`) ends with `;`, `{` or `}` |
| Comment density | Rust files | More than 15% of the non-blank lines of a file are `//` lines. The check does not count `///`, `//!` or `// SAFETY:` lines |
| Dashes | All text files, except `docs/design/pages/`, `docs/design/source-bundle.html`, `tests/fixtures/` and `tools/eval/corpus/` | A file contains U+2014 (em dash) or U+2013 (en dash). Rust code writes these characters as `'\u{2014}'` and `'\u{2013}'` |
| Names | Rust files and the directories under `crates/`, `apps/` and `tools/` | A module, file, type or directory has a prohibited name from section 3.2, with the match rule of section 3.2 |
| Crate names | `Cargo.toml` files | A package name does not follow the rules of section 3.1 |
| Lint override | `Cargo.toml` files | A member crate has no `[lints] workspace = true`, or it overrides a workspace lint |
| Versions | `Cargo.toml` files | A member crate declares a dependency `version` or `path` instead of `workspace = true` |
| Dependency graph | `cargo metadata` | A crate depends on a crate that the graph does not permit, or a workspace member has no row in the graph |
| Exhaustive enums | Rust files | A file contains `#[non_exhaustive]` |
| Unsafe code | Rust files | A file other than `crates/inference/ml/src/weights.rs` contains the `unsafe` keyword |
| Raw design values | Rust files in `apps/desktop/src` and `crates/ui/src`, except `crates/ui/src/theme/` | A line contains `rgb(`, `rgba(`, `hsla(`, `px(`, a match of `0x[0-9A-Fa-f]{6}\b\|"#[0-9A-Fa-f]{6}"`, or a GPUI style method with a size in its name, which is a match of `\.(p\|px\|py\|pt\|pr\|pb\|pl\|m\|mx\|my\|mt\|mr\|mb\|ml\|gap\|gap_x\|gap_y\|w\|h\|size\|min_w\|min_h\|max_w\|max_h\|top\|right\|bottom\|left\|inset)_\d\|\.rounded_(sm\|md\|lg\|xl\|2xl\|3xl\|full)\b\|\.text_(xs\|sm\|base\|lg\|xl\|2xl\|3xl)\b` |
| Component library | Rust files outside `crates/ui` | A file names `gpui_component` |
| Desktop layers | Rust files in `apps/desktop/src/screens/` and `apps/desktop/src/shell/` | A file names `antenna_audio`, `antenna_library`, `antenna_models`, `antenna_pipeline` or `Input::` |

## 14. Testing

1. Each behavior in an acceptance criterion has a test. The test name contains the behavior. `[review]`
2. A test checks one behavior. Separate the arrange, act and assert parts with one blank line. Do not add comments to mark the parts. `[review]`
3. Do not use a mocking library. Use the `fake` engine and `RecordingOutput` from `antenna-engine-testkit`. `[review]`
4. Tests are deterministic. They do not use the internet, wall-clock time or a random seed that changes. A test can use a local HTTP server from `tiny_http`. `[review]`
5. Code that waits or compares ages takes the current time or the delay as a parameter. A test gives a fixed value. `[review]`
6. The stage that owns a budget from `docs/architecture.md` section 8 measures it with `divan`, `hyperfine` or `antenna-eval` on the reference machine. A unit test or an integration test does not measure time. `[review]`
7. Use `insta` for snapshot tests of text output. Review each snapshot change in the PR. `[review]`
8. Use `proptest` for invariants of the text crate, for example "the segment ranges cover the document in order". `[review]`
9. Tests that need downloaded models have the `#[ignore = "needs models"]` attribute. The nightly CI job runs them with `--run-ignored all`. `[review]`
10. Each defect fix includes a test that fails before the fix. `[review]`
11. Do not change a test, a threshold or a fixture to make a failing check pass. Fix the code. If the test is wrong, stop and write a Blocked section. `[review]`

## 15. Git

1. Use Conventional Commits. The scope is the crate role, for example `feat(text): segment sentences with SRX rules`. `[review]`
2. Each commit compiles and passes `cargo xtask check`. `[review]`
3. Use one branch for each stage, with the name `stage/<NN>-<name>`. `[review]`
4. Each PR description lists each acceptance criterion of the stage with its evidence. The evidence is a test name, or a command and its output. `[review]`

## 16. Definition of done

A stage or a change is done when all of these conditions are true. This is the only definition of done.

1. `cargo xtask check` exits with code 0 on a clean clone.
2. Each acceptance criterion of the stage has evidence in the PR description.
3. No lint threshold, test or fixture changed to make a check pass.
4. Each new `#[expect]` has a reason that a reviewer can check.
5. Each new public item has documentation in STE-80.
6. If a contract in `docs/architecture.md` changed, the document and an ADR describe the change.
7. The PR changes only the files in the scope of the stage and the documents that the stage names.
8. The quality gate of section 20 is done.

## 17. Code review checklist

The reviewer checks these items in addition to the automatic checks. All items are `[review]`.

1. Each function does one thing, and its name tells what.
2. No wrapper only forwards a call to another function.
3. No code handles a state that the types make impossible.
4. No `Option` holds a value that is always present.
5. No string holds a value that has a closed set of possible values.
6. No parameter has a `_` prefix because the function does not use it.
7. Each abstraction has two users, or it is a documented extension point.
8. The code uses a crate from the approved table for each problem that is not specific to Antenna.

## 18. Design quality

Lints find mechanical defects. These principles define the quality level that the lints cannot measure. The reviewer rejects code that breaks them. All items are `[review]`.

1. **Deep modules.** A module has a small interface and does a large amount of work behind it. If the interface is as large as the implementation, merge the module into its user. The reference is John Ousterhout, *A Philosophy of Software Design*, chapter 4.
2. **Parse, do not validate.** Convert input into a type that cannot hold an invalid value, at the boundary, one time. Internal code does not check the value again. The reference is Alexis King, "Parse, don't validate" (2019).
3. **Functional core, imperative shell.** Text preparation, voice selection and timing calculations are pure functions. Threads, devices, files and the network stay at the edges.
4. **One place for each piece of knowledge.** When the same rule, constant or sequence of steps occurs a second time, extract it. Two blocks of code that look the same but change for different reasons stay separate.
5. **The signature tells the story.** A reader understands what a function does from its name, its parameter types and its return type, without the body.
6. **The common path is short.** Use `?`, `let ... else` and early returns. The successful path is the least indented code in the function.
7. **No boolean parameters.** Use an enum with named variants. The `max-fn-params-bools = 0` threshold finds boolean parameters.
8. **Immutable by default.** Keep each `mut` binding in the smallest possible scope.
9. **Delete before you add.** The best change removes code. A PR that adds a feature also removes the code that the feature makes unnecessary.

## 19. Exemplars

Copy the style of the "correct" column. Each pair shows one principle. The domain types and the error variants come from `docs/architecture.md` and the stage files. The other names show the style only.

### 19.1 Enum instead of a boolean parameter

```rust
// Incorrect
pub fn encode(samples: &[f32], path: &Path, lossless: bool) -> Result<(), AudioError>;

// Correct
/// The file format of an export.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Display, EnumString, IntoStaticStr)]
pub enum ExportFormat { Mp3, Wav, Ogg }

/// Encodes the samples into a file of the given format.
pub fn encode(samples: &[f32], path: &Path, format: ExportFormat) -> Result<(), AudioError>;
```

### 19.2 Parse at the boundary

```rust
// Incorrect: each user of the value must check for zero.
pub struct Voice { pub sample_rate: u32 }

// Correct: a zero value cannot exist.
/// The number of samples in one second of audio.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SampleRate(NonZeroU32);

impl SampleRate {
    /// Makes a sample rate for a constant.
    pub const fn new(hz: NonZeroU32) -> Self {
        Self(hz)
    }
}

impl TryFrom<u32> for SampleRate {
    type Error = CoreError;

    /// Parses a sample rate from outside the program, for example a device rate.
    fn try_from(hz: u32) -> Result<Self, Self::Error> {
        NonZeroU32::new(hz).map(Self).ok_or(CoreError::ZeroSampleRate)
    }
}
```

### 19.3 Errors with context

```rust
// Incorrect: the error has no path, so the user cannot know which file failed.
#[error(transparent)]
Io(#[from] io::Error),

let bytes = fs::read(path)?;

// Correct
#[error("cannot read {path}")]
Io { path: PathBuf, #[source] source: io::Error },

let bytes = fs::read(path).map_err(|source| ModelError::Io { path: path.to_owned(), source })?;
```

### 19.4 Short successful path

```rust
// Incorrect
pub fn voice(&self, id: &str) -> Result<&'static VoiceDescriptor, RegistryError> {
    for factory in self.factories() {
        for voice in factory.descriptor().voices {
            if voice.id.to_string() == id {
                return Ok(voice);
            }
        }
    }
    Err(RegistryError::UnknownVoice(id.to_owned()))
}

// Correct
pub fn voice(&self, id: &str) -> Result<&'static VoiceDescriptor, RegistryError> {
    self.factories()
        .flat_map(|factory| factory.descriptor().voices)
        .find(|voice| voice.id.to_string() == id)
        .ok_or_else(|| RegistryError::UnknownVoice(id.to_owned()))
}
```

### 19.5 Streaming with cancellation

```rust
fn synthesize(&mut self, segment: &Segment, emit: Emit<'_>) -> Result<(), EngineError> {
    let tone = Tone::for_segment(self.voice, segment);
    for chunk in tone.chunks(CHUNK_SAMPLES) {
        if emit(chunk).is_break() {
            return Ok(());
        }
    }
    Ok(())
}
```

This is the shape of the `fake` engine. The function has no flags, no state machine and no comments. The types and the names tell the sequence. The in-house engines get the same shape from the driver `antenna_ml::pace`.

## 20. Quality gate for each stage

A stage is complete after these three steps in sequence.

1. **Automatic checks.** `cargo xtask check` passes.
2. **Independent review.** A new agent session gets the diff, the stage file, `docs/architecture.md`, all of this document and `docs/writing.md`. It checks each `[review]` rule. It reads every changed file completely and lists each finding. The author fixes each finding or writes the reason to reject it in the PR.
3. **Simplification pass.** The author reads the complete diff one more time and removes each line that the stage does not need. Examples are unused generality, defensive checks for impossible states, forwarding functions and comments that tell what the code does.

The PR description records the result of steps 2 and 3.
