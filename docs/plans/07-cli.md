# Stage 07. CLI

## Goal

The `antenna` command plays and exports documents through the segment store, lists voices and manages models. Integration tests with the `fake` engine prove each command and exit code.

## Context

The CLI is the first complete product. It proves that the headless crates work together before the desktop app exists. It is also a tool for power users and scripts. The CLI contains no domain logic. It parses arguments, connects the crates and prints results.

The CLI uses the same data directories as the desktop app. Thus a sentence that the desktop app generated is a cache hit in the CLI, and the reverse. The environment variables `ANTENNA_DATA_DIR` and `ANTENNA_MODEL_DIR` replace the directories for tests and scripts.

## Read first

1. `CLAUDE.md`
2. `docs/architecture.md` sections 1.1, 3, 6 and 7
3. `docs/standards.md` sections 3, 6, 11 and 14
4. `docs/plans/03-audio.md`, `docs/plans/04-models.md`, `docs/plans/05-library.md` and `docs/plans/06-pipeline.md` (the APIs that the CLI uses)

## Scope

The stage can create or change these files only.

- `apps/cli/`
- `Cargo.toml` (the `members` list and `[workspace.dependencies]` only)

## Deliverables

### File layout

```
apps/cli/
├── Cargo.toml          # package antenna-cli, [[bin]] name = "antenna"
├── src/
│   ├── main.rs         # composition root: parse, build registry, stores, pipeline and player, run, map exit code
│   ├── args.rs         # clap derive types
│   ├── input.rs        # read a file or stdin, find the text format
│   ├── selection.rs    # language and voice selection
│   ├── commands/
│   │   ├── mod.rs      # dispatch to the command modules
│   │   ├── speak.rs
│   │   ├── export.rs
│   │   ├── voices.rs
│   │   └── models.rs
│   ├── progress.rs     # turn job and export events into progress lines
│   ├── output.rs       # the only module that writes to stdout and stderr
│   └── exit.rs         # ExitCode mapping
└── tests/
    └── cli.rs          # integration tests, #![cfg(feature = "engine-fake")]
```

### Dependencies

Normal dependencies are `antenna-core`, `antenna-engine-registry`, `antenna-pipeline`, `antenna-models`, `antenna-library`, `antenna-audio`, `antenna-text`, `clap` (with `derive`), `flume`, `anyhow`, `ctrlc`, `tracing` and `tracing-subscriber`. Dev-dependencies are `hound`, `ebur128` and `tempfile`. The registry dependency has no default features, because `[workspace.dependencies]` sets `default-features = false` for it. In this stage the CLI declares one feature, `engine-fake`, which enables `antenna-engine-registry/engine-fake`, and it has no default features. Stage 12 adds the feature `engine-qwen3` and stage 14 adds `engine-magpie`. Each of these stages makes its feature a default feature of the CLI.

### Commands

```text
antenna speak  <FILE|->                   [--voice <ID>] [--language <CODE>] [--quality <Q>]
antenna export <FILE|-> --output <PATH>   [--voice <ID>] [--language <CODE>] [--quality <Q>]
                                          [--format mp3|wav|ogg] [--sample-rate 24000|44100|48000]
                                          [--normalize | --no-normalize] [--force]
antenna voices                            [--language <CODE>] [--quality <Q>]
antenna models list
antenna models pull   <VOICE_ID>          [--quality <Q>]
antenna models remove <ENGINE_ID>
```

`<Q>` is `fast`, `balanced` or `max`. The default is `Quality::default()`. `<CODE>` is a language code, for example `es`. The default of `--sample-rate` is `ExportRate::default()`, which is 44100. Global flags are `--quiet` (no progress lines) and `--verbose` (log level `debug`).

### Arguments

```rust
#[derive(Parser)]
#[command(name = "antenna", version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
    #[arg(long, global = true)]
    pub quiet: bool,
    #[arg(long, global = true)]
    pub verbose: bool,
}

#[derive(Subcommand)]
pub enum Command {
    Speak(SpeakArgs),
    Export(ExportArgs),
    Voices(VoicesArgs),
    #[command(subcommand)]
    Models(ModelsCommand),
}

#[derive(Args)]
pub struct Selection {
    #[arg(long)]
    pub voice: Option<String>,
    #[arg(long)]
    pub language: Option<Language>,
    #[arg(long, default_value_t)]
    pub quality: Quality,
}

#[derive(Args)]
#[group(multiple = false)]
pub struct LoudnessArgs {
    #[arg(long)]
    pub normalize: bool,
    #[arg(long)]
    pub no_normalize: bool,
}

#[derive(Args)]
pub struct ExportArgs {
    pub input: String,
    #[arg(long)]
    pub output: PathBuf,
    #[command(flatten)]
    pub selection: Selection,
    #[arg(long)]
    pub format: Option<ExportFormat>,
    #[arg(long, default_value_t)]
    pub sample_rate: ExportRate,
    #[command(flatten)]
    pub loudness: LoudnessArgs,
    #[arg(long)]
    pub force: bool,
}
```

`clap` parses `Language`, `Quality` and `ExportFormat` with their `FromStr` of `antenna-core`, and `ExportRate` with its `FromStr` of `antenna-audio`. The CLI defines no copy of these enums. `LoudnessArgs` converts to `Loudness`. With no flag, it gives `Loudness::default()`. The `bool` fields exist only because `clap` needs them, and they keep the names of their flags. `docs/standards.md` exempts `clap` argument fields from the `is_` prefix rule. No function of the CLI takes a boolean parameter.

### Input

1. The argument `-` reads stdin until the end. Any other argument is a file path.
2. `TextFormat::from_extension` of `antenna-core` gives the format of a file. Stdin gives `TextFormat::Plain`.
3. The input must be valid UTF-8. If it is not, the CLI exits with code 3.
4. The episode title is the file stem of the input. For stdin, the title is "stdin".

### Language and voice selection

`selection.rs` contains one pure function with this signature.

```rust
pub fn select(registry: &Registry, selection: &Selection, document: &Document) -> Result<&'static VoiceDescriptor, SelectionError>;

pub enum SelectionError {
    Conflict { voice: VoiceId, language: Language },
    UnknownVoice(#[source] RegistryError),
    UnknownLanguage,
}
```

The function obeys these rules in this sequence. The language of the job is the language of the returned voice.

1. If `--voice` is present, find it with `Registry::voice`. If the registry does not have it, return `SelectionError::UnknownVoice`.
2. If `--language` is also present and the language of the voice is different, return `SelectionError::Conflict`. Otherwise, use the voice.
3. If `--language` is present, use the default voice of the language.
4. Find the language with `antenna_text::identify_language`. Use the default voice of the language.
5. If the language cannot be found, return `SelectionError::UnknownLanguage`. The message tells the user to use `--language`.

### Data directories

1. The CLI opens the model store with `ModelStore::open_default`, which reads `ANTENNA_MODEL_DIR`.
2. The CLI opens the segment store with `SegmentStore::open_default`, which reads `ANTENNA_DATA_DIR`.
3. The CLI opens only the `SegmentStore`. It does not read or write library documents.
4. The tests and the nightly CI job set both variables.

### Commands in detail

1. **speak.** Do these steps in sequence.
   1. Select the voice. Call `Pipeline::timeline`. Build the track with `Track::new(timeline.rate(), timeline.stored(&store))`. Load it in the player.
   2. Start a job with one output, `TrackHandle::live_output`. Call `TrackHandle::play`.
   3. For each `Synthesized` event with `is_cached: false`, call `TrackHandle::set_stored` with the path of the segment key.
   4. Wait on the job events and the player events with `flume::Selector`. Exit with code 0 when the job sent `Finished` and the player state is `Ended`.
2. **export.** Do these steps in sequence.
   1. Find the format from `--format`. If `--format` is not present, find it from the extension of `--output` (`.mp3`, `.wav`, `.ogg`, `.opus`). If both are present and they are different, exit with code 2. If neither gives a format, use MP3.
   2. If the output file exists and `--force` is not present, exit with code 2.
   3. Call `Pipeline::timeline`. If that timeline is complete, use it in step 4. Otherwise, start a job with no outputs and keep the timeline of its `Started` event. The worker sets the durations of that timeline, so it is complete when the job sends `Finished`. The timeline of the first call stays incomplete, so the CLI does not use it after a job. Show the progress, and wait for `Finished`.
   4. Call `antenna_audio::export` with an `ExportSpec`. Its `segments` are `timeline.paths(&store)` (stage 06) of the complete timeline of step 3, `source_rate` is the rate of that timeline, and `export_rate` comes from `--sample-rate`.
   5. Print the summary line to stderr. The line has the segment count, the count of synthesized segments, the count of cached segments, the duration and the gain in dB.
3. **voices.** Print one line for each voice, with the columns id, language, name, engine, default and installed, separated by tab characters. "Installed" is `ModelStore::is_installed` of the voice at the selected quality. With `--language`, print only the voices of that language.
4. **models list.** Print one line for each engine and quality, with the columns engine id, quality, size in MB and installed. The size is `antenna_models::engine_download_bytes`. "Installed" is `ModelStore::is_engine_installed`. The Models screen of stage 17 calls the same two functions.
5. **models pull.** Call `ModelStore::ensure` with `voice_artifacts` of the voice at the selected quality. Show the progress. Do not load the engine.
6. **models remove.** Find the engine with `Registry::engine`. Call `ModelStore::remove_engine` with its descriptor and `antenna_models::keep_artifacts` of the other registered engines, with no `extra`. Print the freed size in MB.

### Progress and output

1. `output.rs` is the only module with `println!` and `eprintln!`. The module has one inner attribute `#![expect(clippy::print_stdout, clippy::print_stderr, reason = "the CLI writes its output here")]`, as `docs/standards.md` section 11 defines.
2. Data (voice lists, model lists) goes to stdout. Progress lines, summaries and errors go to stderr.
3. If stderr is a terminal (`std::io::IsTerminal`), a progress line updates in place with a carriage return. If stderr is not a terminal, the CLI prints one line for each 10% of progress.
4. The download line shows the engine id, the done and total sizes in MB and the percentage.
5. The synthesis line shows the segment number, the segment count and "cached" for a cache hit.
6. The export line shows the percentage of the export.
7. An error message has one line with the text of the error and one line with the action that the user can do. The CLI does not print a backtrace unless `--verbose` is set.

### Ctrl-C

The CLI registers a `ctrlc` handler. The handler cancels the current job and sets the cancel flag of `export`. When the job sends `Cancelled` or `export` returns `AudioError::Cancelled`, the CLI exits with code 130. The export writes to a `.part` file, so no partial export stays on the disk.

### Exit codes

```rust
pub enum Exit { Success = 0, JobFailed = 1, Usage = 2, Input = 3, Model = 4, Device = 5, Storage = 6, Interrupted = 130 }
```

| Condition | Code |
|---|---|
| Success | 0 |
| `ErrorKind::Synthesis` or `ErrorKind::Internal` | 1 |
| Usage error from `clap`, `SelectionError::Conflict`, `SelectionError::UnknownVoice`, each `RegistryError` (for example an unknown `<VOICE_ID>` or `<ENGINE_ID>`), a format conflict, an output file that exists | 2 |
| File not found, input that is not UTF-8, empty document, `SelectionError::UnknownLanguage`, `ErrorKind::InvalidRequest` | 3 |
| `ErrorKind::Download`, `ErrorKind::DiskSpace`, `ErrorKind::ModelCorrupt`, `ErrorKind::ModelLoad`, each `ModelError` except `Cancelled` | 4 |
| `ErrorKind::Output`, `AudioError::NoDevice`, `AudioError::DeviceConfig`, `AudioError::BuildStream`, `AudioError::PlayStream` | 5 |
| `ErrorKind::Storage`, each `LibraryError`, and the other `AudioError` variants of an export except `Cancelled` | 6 |
| Cancelled with Ctrl-C, `JobEvent::Cancelled`, `AudioError::Cancelled`, `ModelError::Cancelled` | 130 |

`exit.rs` maps all `ErrorKind`, `AudioError`, `ModelError`, `LibraryError`, `RegistryError` and `SelectionError` variants without `_ =>`. `ModelError` and `LibraryError` come from `models pull`, `models remove` and the `open_default` calls. `RegistryError` comes from `Registry::new`, `Registry::voice` and `Registry::engine`.

## Tasks

1. Add `apps/cli` to the workspace with the binary name `antenna` and the feature `engine-fake`. Do not add default features.
2. Write `args.rs` and `exit.rs`.
3. Write `input.rs` with its unit tests.
4. Write `selection.rs` with one unit test for each rule.
5. Write `output.rs` and `progress.rs`. Test `progress.rs` with a sequence of events and a string buffer.
6. Write the four command modules and `commands/mod.rs`.
7. Write `main.rs`. It initializes `tracing-subscriber`, builds the registry, the model store, the segment store, the pipeline and, for `speak` only, the player with `Player::start`. It runs the command and returns the exit code.
8. Write the integration tests in `tests/cli.rs`. Run the binary with `std::process::Command` and `env!("CARGO_BIN_EXE_antenna")`. Set `ANTENNA_DATA_DIR` and `ANTENNA_MODEL_DIR` to new `tempfile::TempDir` directories for each test. Write two helpers. `run_with_voice(language, args)` adds `--voice fake/<language>-alba`, and each test of a command that synthesizes uses it. `run(args)` runs the other commands, for example `voices` or a test with its own `--voice`. Stage 12 adds `engine-qwen3` to the CLI and makes it a default feature, and `cargo xtask check` builds with all features. Then an integration test with no `--voice` selects a `qwen3` voice and downloads gigabytes. The `select_<rule>` unit tests compare the result with `registry.default_voice(language)`, not with a literal voice id.
9. Run `cargo xtask check`. Fix each failure.
10. Do the quality gate in `docs/standards.md` section 20.

## Acceptance criteria

| ID | Criterion | Check |
|---|---|---|
| AC-07-01 | `export` writes a WAV file with the expected duration | Test `export_writes_wav_with_expected_duration`. Read the file with `hound` and compare with the duration of the fake engine (40 ms for each character of each segment) |
| AC-07-02 | `export` writes a valid MP3 file | Test `export_writes_mp3_when_extension_is_mp3`. The file starts with an ID3 tag or an MPEG frame sync |
| AC-07-03 | `export` writes a valid Ogg file | Test `export_writes_ogg_when_format_is_ogg`. The file starts with `OggS` |
| AC-07-04 | `--sample-rate` sets the rate of the file | Test `export_uses_requested_sample_rate` exports with `--sample-rate 48000` and reads a WAV at 48000 Hz |
| AC-07-05 | Normalization is on by default and gives -16 LUFS with a difference of 0.5 LU or less | Test `export_normalizes_when_no_loudness_flag`, measured with `ebur128` on the WAV |
| AC-07-06 | `--no-normalize` keeps the level of the fake engine | Test `export_keeps_level_when_no_normalize` exports with `--sample-rate 24000`, the rate of the fake engine, so no resampler changes the peak. The WAV peak is 0.25 with a difference of one 16-bit step or less |
| AC-07-07 | A second export of the same document synthesizes no segment | Test `export_uses_segment_cache_when_repeated`. The summary line of the second run shows 0 synthesized segments |
| AC-07-08 | `export` reads stdin when the argument is `-` | Test `export_reads_stdin_when_argument_is_dash` |
| AC-07-09 | A Markdown file is read as Markdown | Test `export_strips_markdown_when_extension_is_md`. The WAV of `# Title` plus a paragraph has the duration of the prose only |
| AC-07-10 | `export` refuses to overwrite a file without `--force` | Test `export_exits_with_usage_when_output_exists` (code 2) |
| AC-07-11 | A format conflict gives code 2 | Test `export_exits_with_usage_when_format_conflicts` |
| AC-07-12 | `--normalize` and `--no-normalize` together give code 2 | Test `export_exits_with_usage_when_loudness_flags_conflict` |
| AC-07-13 | Each selection rule returns the voice or the error of its rule | Unit tests `select_<rule>` in `selection.rs`, one for each rule |
| AC-07-14 | `--quiet` prints no progress line, and errors still go to stderr | Test `export_prints_no_progress_when_quiet` |
| AC-07-15 | An empty document, a missing file and input that is not UTF-8 each give code 3 | Tests `export_exits_with_input_error_when_<case>` |
| AC-07-16 | `voices` lists one line for each voice with six columns | Test `voices_prints_one_line_per_voice` |
| AC-07-17 | `voices --language es` prints only the Spanish voices | Test `voices_prints_only_language_when_language_given` |
| AC-07-18 | `models list`, `models pull` and `models remove` exit with code 0 for an engine that has no artifacts | Tests `models_list_prints_engines`, `models_pull_succeeds_when_no_artifacts` and `models_remove_frees_zero_when_nothing_installed` |
| AC-07-19 | `--quality fast` and `--quality max` run with the fake engine | Test `export_accepts_each_quality` |
| AC-07-20 | Each `ErrorKind`, `AudioError`, `ModelError`, `LibraryError`, `RegistryError` and `SelectionError` variant maps to the exit code of the table | Unit test `exit_code_matches_table` |
| AC-07-21 | Progress output goes to stderr and data goes to stdout | Test `voices_writes_data_to_stdout_only` |
| AC-07-22 | Only `output.rs` writes to stdout or stderr | `rg -l -e 'println!' -e 'eprintln!' -e 'print!' -e 'eprint!' apps/cli/src` prints only `apps/cli/src/output.rs` |
| AC-07-23 | `speak` plays a document from start to end on a machine with an audio device **(manual)** | On the reference machine (Apple M5 Pro, 24 GB), run `cargo run --release -p antenna-cli --features engine-fake -- speak README.md --voice fake/en-alba`. The tones play without a gap, and `echo $?` prints `0` |
| AC-07-24 | A second `speak` of the same document starts to play from the store without synthesis **(manual)** | Run the command of AC-07-23 two times. Each progress line of the second run shows "cached" |
| AC-07-25 | Ctrl-C during `export` exits with code 130 and leaves no file **(manual)** | The steps of AC-07-25 below this table |
| AC-07-26 | An unknown `--voice` gives code 2 | Test `export_exits_with_usage_when_voice_unknown` |
| AC-07-27 | The CLI uses the engine functions of `antenna-models` and has no copy of them | `rg -n -e 'fn keep_artifacts' -e 'fn engine_download_bytes' -e 'fn is_engine_installed' apps/cli/src` prints nothing |
| AC-07-28 | An export without `--sample-rate` gives a 44100 Hz file | Test `export_uses_default_rate_when_no_sample_rate` |
| AC-07-29 | `models remove` with an unknown engine id gives code 2 | Test `models_remove_exits_with_usage_when_engine_unknown` |
| AC-07-30 | The `.opus` extension gives an Ogg Opus file | Test `export_writes_ogg_when_extension_is_opus`. The file starts with `OggS` |
| AC-07-31 | The episode title of stdin input is "stdin" | Test `export_tags_title_stdin_when_input_is_dash` with `--format ogg`. The `OpusTags` page contains `TITLE=stdin` |
| AC-07-32 | The progress lines have the formats of "Progress and output" | Unit tests `progress_formats_download_line`, `progress_marks_cached_when_segment_cached` and `progress_prints_line_per_ten_percent_when_not_terminal` in `progress.rs` |
| AC-07-33 | Each integration test that synthesizes passes a `fake` voice | Each test runs the binary through one helper `run_with_voice(language, args)` in `tests/cli.rs` that adds `--voice fake/<language>-alba`. `grep -c "Command::new" apps/cli/tests/cli.rs` prints `2` (the helper and the helper for commands that do not synthesize) |

The steps of AC-07-25 run on the reference machine (Apple M5 Pro, 24 GB).

1. Write the input file with this command.
   ```sh
   yes "This is a long test sentence." | head -n 2000 > /tmp/long.txt
   ```
2. Run `cargo run --release -p antenna-cli --features engine-fake -- export /tmp/long.txt --output /tmp/long.wav --voice fake/en-alba`.
3. Press Ctrl-C when the progress line shows 10% or more.
4. Run `echo $?`. It prints `130`.
5. Run `ls /tmp/long.wav /tmp/long.wav.part`. It reports that neither file exists.

## Decision rules

1. If the player state does not become `Ended` after the last sample, stop and write a "Blocked" section about the defect of stage 03. Do not wait with `sleep`.
2. If a test needs a long document for the fake engine, generate it in the test with a repeated sentence. Do not add large fixture files.
3. If CI on Linux has no audio device, the integration tests do not use `speak`. AC-07-23 and AC-07-24 cover `speak`.
4. `antenna-review` is not a dependency of the CLI. Thus the `keep` list of `models remove` does not contain the Whisper files. They are in other repositories, so no engine artifact has the same local file.
5. If the signatures of the APIs of stages 03 to 06 differ from this file, use the real signatures. Keep the behavior of this file.

## Out of scope

1. Any logic that belongs in a library crate. If the CLI needs a domain function that does not exist, stop and write a "Blocked" section.
2. Library documents. The CLI uses only the segment store.
3. Shell completion and man pages.
4. Configuration files. The CLI has no settings file in the MVP.
