# Stage 09. Evaluation tool

## Goal

`antenna-eval` measures the speed, the memory and the intelligibility of each voice with fixed methods. It writes the results to `docs/benchmarks.md`, and a nightly CI job runs it for each default voice.

## Context

`antenna-eval` turns the budgets of `docs/architecture.md` sections 8 and 11 into commands. Stages 12 and 14 use these commands as their acceptance checks. Thus "the engine is good" becomes a number with a limit.

The tool uses the same pipeline, segment store and Whisper transcriber as the desktop app. Thus each measurement includes the full path from text to audio.

## Read first

1. `CLAUDE.md`
2. `docs/architecture.md` sections 3, 6.1, 6.2, 8, 10, 11 and 14
3. `docs/standards.md` sections 7, 8, 11, 12 and 14
4. `docs/plans/05-library.md`, `docs/plans/06-pipeline.md`, `docs/plans/07-cli.md` and `docs/plans/08-ml-and-review.md`

## Scope

The stage can create or change these files only.

- `tools/eval/`
- `.github/workflows/nightly.yml`
- `docs/benchmarks.md`
- `Cargo.toml` (the `members` list and `[workspace.dependencies]` only)

## Deliverables

### File layout

```
tools/eval/
├── Cargo.toml              # package antenna-eval, [[bin]] name = "antenna-eval"
├── corpus/
│   ├── en.txt  es.txt  pt.txt  fr.txt  it.txt  de.txt                      # 40 sentences each
│   └── en-numbers.txt  es-numbers.txt  pt-numbers.txt  ...                  # 10 sentences each
└── src/
    ├── main.rs             # composition root
    ├── args.rs             # clap: bench, wer
    ├── corpus.rs           # load and validate the corpus
    ├── workspace.rs        # temporary data directory, library, model store, pipeline
    ├── bench.rs            # the bench command
    ├── budgets.rs          # the budgets of docs/architecture.md section 8 as a const table
    ├── timing.rs           # TimingOutput and the paced playback simulation
    ├── memory.rs           # peak resident memory sampler
    ├── wer.rs              # the wer command
    └── report.rs           # append rows to docs/benchmarks.md
```

Normal dependencies are `antenna-core`, `antenna-engine-registry`, `antenna-pipeline`, `antenna-library`, `antenna-models`, `antenna-review`, `clap`, `anyhow`, `flume`, `tempfile`, `memory-stats`, `sysinfo`, `tracing` and `tracing-subscriber`. `antenna-eval` depends on `antenna-engine-registry` through the `[workspace.dependencies]` entry, which has `default-features = false`. This stage declares the feature `engine-fake = ["antenna-engine-registry/engine-fake"]` and puts it in the default features. Stage 12 adds `engine-qwen3` and stage 14 adds `engine-magpie` to this manifest and to its default features.

The normalization, the word alignment and the Whisper transcriber come from `antenna-review`. `antenna-eval` has no copy of them. `TimingOutput` exists only in `antenna-eval`, because only a measurement needs the time of each write.

### Commands

```text
antenna-eval bench (--voice <ID> | --language <CODE>) [--quality fast|balanced|max] [--machine <LABEL>] [--report <PATH>] [--check-budgets]
antenna-eval wer   (--voice <ID> | --language <CODE>) [--quality fast|balanced|max] [--report <PATH>] [--sentences <N>]
```

1. `--language` selects the default voice of the language from the registry. `--voice` and `--language` are mutually exclusive, and one of them is necessary.
2. `--quality` defaults to `balanced`.
3. `--report` defaults to `docs/benchmarks.md`.
4. `--machine` defaults to the CPU brand string from `sysinfo`.
5. `--sentences` limits the corpus to its first N sentences for each file.

### Workspace

`workspace.rs` makes the objects that both commands use.

1. A temporary data directory from `tempfile`. The library and its segment store are in this directory, so no command reads a segment that an earlier run synthesized.
2. The model store from `ModelStore::open_default()`. The environment variable `ANTENNA_MODEL_DIR` can replace it, as `docs/architecture.md` section 7.1 defines. Downloaded models stay between runs.
3. The registry with the enabled engine features. A pipeline with the registry factories, the model store and a clone of `Library::segments` of the temporary library.

A fresh workspace for each measurement makes the segment cache empty. A cache hit in a measurement is a defect of the measurement.

### bench

The command measures these values for one voice and one quality. It uses the corpus of the voice language.

| Value | Method |
|---|---|
| Time to first audio, warm | Run one job on corpus sentence 1 to load the engine. Then run one job on each of the corpus sentences 2 to 6. Report the median time from `Pipeline::start` to the first `Sink::write` of a `TimingOutput` |
| Time to first audio, cold | Make a new workspace, so the pool and the segment store are empty. The weights are in the page cache from the warm runs. Run one job on corpus sentence 7 and report the same time |
| Real-time factor | Make a new workspace. Call `Pipeline::preload` and wait for its `Finished`, so the load is outside the interval. Run one job on the full corpus as one document. Divide the time from `JobEvent::Started` to `JobEvent::Finished` by the total audio duration of the `Timeline` |
| Audio for each character | The total audio duration of the full corpus job in milliseconds, divided by the `char` count of the corpus text without line breaks. Stages 12 and 14 calibrate their runaway guard with this value |
| Peak resident memory | A thread reads `memory_stats::memory_stats()` during the full corpus job and keeps the maximum |
| Underruns | The paced playback simulation of `timing.rs` on the records of the full corpus job |

The warm runs use different sentences, so each run synthesizes its segment and no run is a cache hit.

The memory thread has the name `antenna-eval-memory` and starts with `std::thread::Builder`. It reads the memory, then waits with `recv_timeout` of 50 ms on a `flume` stop channel. It stops when the channel gets a message or disconnects. The thread does not call `std::thread::sleep`.

`TimingOutput` is an `Output` that records the `Instant` and the sample count of each write. The paced playback simulation reads these records after the job. It simulates a player that starts at the first sample, consumes audio in real time and has a 2 s buffer. The player can need a sample that the job did not write yet. Each time, the simulation counts one underrun and waits for the next write. The simulation is a pure function of the records, so it has unit tests.

In the report and the budgets, 1 MB is 10^6 bytes and 1 GB is 10^9 bytes.

### Budgets

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Budget {
    pub ttfa_warm: Option<Duration>,
    pub ttfa_cold: Option<Duration>,
    pub rtf: f64,
    pub underruns: u32,
    pub peak_memory_bytes: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VoiceRole { Default, Other }

pub fn budget(engine: &EngineId, quality: Quality, role: VoiceRole) -> Option<Budget>;
```

`budgets.rs` is the only place in the workspace that holds the budget values. The values come from `docs/architecture.md` section 8.

The time to first audio budgets apply to `Balanced` only, as `docs/architecture.md` section 8 defines. `budget` gives `None` in `ttfa_warm` and `ttfa_cold` for `Fast` and `Max`.

| Engine | Warm time to first audio (`Balanced`) | Cold time to first audio (`Balanced`) | Underruns | Peak memory |
|---|---|---|---|---|
| `qwen3` | 500 ms | 4 s | 0 | 6 GB in `Balanced`, 10 GB in `Max`, no limit in `Fast` |
| `magpie` | 300 ms | 4 s | 0 | No limit |

| Quality and role | Real-time factor |
|---|---|
| `Balanced`, `VoiceRole::Default` | 0.5 |
| `Balanced`, `VoiceRole::Other` | 0.8 |
| `Fast`, both roles | 0.8 |
| `Max`, both roles | 0.9 |

1. `bench` gets the role from the registry. The voice is `VoiceRole::Default` when the registry gives it as the default voice of its language. The voices of one engine share the weights, so the engine stages check the `VoiceRole::Other` budget on one voice of each language.
2. `budget` returns `None` for an engine without a row, for example `fake`.
3. With `--check-budgets`, `bench` compares each measured value with the budget. A field with the value `None` has no limit, and `bench` prints "no limit" for it. `bench` exits with code 1 when a value is outside its budget, and with code 2 when `budget` returns `None`. It prints each value, its limit and the result.
4. Only the reference machine uses `--check-budgets`. CI does not use it, because CI machines are not the reference machine.

### wer

1. Make a workspace and load a `Transcriber` for `WHISPER_LARGE_V3_TURBO`. Get its files with `ModelStore::ensure`.
2. Synthesize each sentence of the corpus as one job. Each sentence is one document with `TextFormat::Plain`.
3. Read the samples of each segment of the job with `SegmentStore::read` and the keys of the `Timeline`. Join the segments of the sentence.
4. Transcribe the samples with `Transcriber::transcribe` and the language of the voice. The transcriber resamples to 16 kHz.
5. Call `antenna_review::align` with the sentence as the reference and the transcript. Sum the `WordErrors` of all sentences with `WordErrors::add`.
6. Calculate the corpus WER as `total()` divided by `reference_words` of the sum.
7. Do steps 2 to 6 for the numbers subset and report its WER without a limit.
8. Append one row to the intelligibility table of the report.
9. Exit with code 1 if the corpus WER is more than the limit of `docs/architecture.md` section 11.2 for the voice language.

### Corpus

The agent of this stage writes the corpus. Each language has `<code>.txt` with 40 sentences and `<code>-numbers.txt` with 10 sentences. The files are UTF-8 with one sentence on each line.

1. The agent writes each sentence for this project. Do not copy text from books, websites or datasets.
2. Each sentence has 8 to 25 words.
3. The sentences of `<code>.txt` contain no digits and no abbreviations. An abbreviation is one of two things. It is a word of 1 to 4 letters with a full stop that is not the last character of the line. It is also a word of two or more uppercase letters.
4. Each sentence of `<code>-numbers.txt` contains one or more digits. Each numbers file contains one or more percentages with `%`, one or more years from 1900 to 2099, and one or more decimal numbers. The decimal separator is a full stop for `en` and a comma for the other five languages.
5. Each `<code>.txt` file has 4 or more questions (lines that end with `?`) and 2 or more exclamations (lines that end with `!`). The numbers files have no such rule.
6. No sentence occurs two times in a file.

**WARNING.** Do not change the corpus after this stage to make a WER limit pass. `docs/architecture.md` section 14 rule 3 applies.

### Report format

`docs/benchmarks.md` has four tables. `report.rs` appends one row at the end of the correct table. If the file does not exist, `report.rs` makes it with the four empty tables. The engine stages write the rows of the module benchmarks table by hand from the `divan` output, because `antenna-eval` does not run the benches.

```markdown
## Speed

| Date | Commit | Voice | Quality | Machine | Device | TTFA warm (ms) | TTFA cold (ms) | RTF | Audio per char (ms) | Peak RSS (MB) | Underruns |
|---|---|---|---|---|---|---|---|---|---|---|---|

## Intelligibility

| Date | Commit | Voice | Quality | Device | Sentences | WER (%) | Limit (%) | Numbers WER (%) |
|---|---|---|---|---|---|---|---|---|

## Module benchmarks

| Date | Commit | Crate | Bench | Case | Machine | Device | Median (ms) |
|---|---|---|---|---|---|---|---|

## Listening sign-off

| Date | Voice | Listener | Result |
|---|---|---|---|
```

1. The date has the format `YYYY-MM-DD`.
2. The commit is the output of `git rev-parse --short HEAD`. If the command fails, the commit is `unknown`.
3. The device is the output of `antenna_review::device_label`, the re-export of stage 08.
4. RTF has two decimals. WER and audio per char have one decimal.

### Nightly CI

`.github/workflows/nightly.yml` has these properties.

1. Triggers are a schedule at 03:00 UTC each day and `workflow_dispatch`.
2. The runner is `macos-15` (Apple Silicon).
3. The environment sets `ANTENNA_MODEL_DIR` to `~/antenna-models`, `ANTENNA_DEVICE` to `cpu` and `NEXTEST_PROFILE` to `ci`.
4. `actions/cache` keeps `~/antenna-models`. The cache key is the hash of `crates/engines/*/src/descriptor/artifacts.rs` and `crates/inference/review/src/model.rs`.
5. Step 1 runs the tests that fit the runner, with the command below this list.
6. Step 2 builds `antenna-eval` in release with its default features. Stages 12 and 14 add the real engines to these defaults.
7. Step 3 runs `antenna-eval wer --language <code> --quality fast` for each of the six language codes. In this stage no real engine exists, so step 3 does not run. Stage 12 turns on step 3.
8. The tests of the `qwen3` crate that load the 1.7B model do not run in the nightly job. The CPU runner loads the weights in f32, and the 1.7B talker alone needs about 7 GB. These tests are the conformance suite of `qwen3`, which loads all three variants, and the tests whose names end in `_when_large` or `_when_balanced`. The reference machine runs them in stage 12. The `magpie` model has 357M parameters, so all its tests run in the nightly job.
9. The job timeout is 360 minutes.

Step 1 runs this command.

```sh
cargo nextest run --workspace --all-features --release --run-ignored all -E "$NIGHTLY_FILTER"
```

The workflow sets `NIGHTLY_FILTER` to this filterset.

```text
not (package(antenna-audio) & binary(device))
& not (package(antenna-engine-qwen3) & (binary(conformance) | test(/_when_large$/) | test(/_when_balanced$/)))
```

The first part skips the tests of `crates/audio/tests/device.rs`, because the runner has no audio device. The second part skips the `qwen3` tests of item 8. Use one `-E` option, because nextest runs the union of two `-E` options.

## Tasks

1. Add `tools/eval` to the workspace.
2. Write the corpus files. Then write `corpus.rs` with the validation tests of AC-09-01.
3. Write `workspace.rs`.
4. Write `timing.rs` with the simulation tests, then `memory.rs`.
5. Write `budgets.rs` with its tests.
6. Write `bench.rs`, `wer.rs` and `report.rs`.
7. Write `main.rs` and `args.rs`.
8. Write `docs/benchmarks.md` with the four empty tables.
9. Write `.github/workflows/nightly.yml`. Start it with `workflow_dispatch` on the stage branch.
10. Run `cargo xtask check`. Fix each failure.
11. Do the quality gate in `docs/standards.md` section 20.

## Acceptance criteria

| ID | Criterion | Check |
|---|---|---|
| AC-09-01 | The corpus obeys its rules | Tests `corpus_has_forty_sentences_per_language`, `corpus_has_ten_number_sentences_per_language`, `corpus_sentences_have_no_digits`, `corpus_sentences_have_no_abbreviations`, `corpus_sentences_have_eight_to_twenty_five_words`, `number_sentences_have_digits`, `number_files_have_percent_year_and_decimal`, `corpus_has_questions_and_exclamations` and `corpus_has_no_duplicates` |
| AC-09-02 | `--voice` and `--language` are mutually exclusive and one of them is necessary | Tests `args_fail_when_voice_and_language_given` and `args_fail_when_no_voice_or_language` |
| AC-09-03 | `--language` selects the registry default voice | Test `args_select_default_voice_when_language_given` with the fake engine |
| AC-09-04 | Each measurement starts with an empty segment store | Test `workspace_starts_with_empty_segment_store` |
| AC-09-05 | The warm runs have no cache hit | Test `bench_has_no_cached_segment_when_fake_voice` reads the `is_cached` flag of each `Synthesized` event |
| AC-09-06 | `bench` runs on the fake engine and appends one row to the speed table | Test `bench_appends_row_when_fake_voice` with `--report` set to a temporary file |
| AC-09-07 | The paced playback simulation counts underruns correctly | Tests `simulation_counts_no_underrun_when_faster_than_real_time` and `simulation_counts_underruns_when_slower_than_real_time` |
| AC-09-08 | `report.rs` makes the file with four tables when it does not exist | Test `report_creates_four_tables_when_file_missing` |
| AC-09-09 | `report.rs` appends a row to the correct table when the file exists | Test `report_appends_row_to_named_table_when_file_exists` |
| AC-09-10 | `wer` reads the audio from the segment store and calls `antenna_review::align` | Test `wer_aligns_store_audio_when_fake_voice`, marked `#[ignore = "needs models"]` |
| AC-09-11 | `wer` exits with code 1 when the WER is above the limit | Test `wer_fails_when_above_limit`, marked `#[ignore = "needs models"]`, with the fake voice (tones give a WER of 100%) |
| AC-09-12 | `bench --check-budgets` exits with code 1 when a value is outside its budget | Test `bench_fails_budget_check_when_rtf_above_limit` with a synthetic measurement |
| AC-09-13 | The crate has no copy of normalization, alignment or Whisper code | `rg -n -e "fn normalize" -e "fn align" -e "whisper::model" tools/eval/src` prints nothing |
| AC-09-14 | The nightly workflow passes | The `workflow_dispatch` run on the stage branch is green. Link it in the PR |
| AC-09-15 | `budget` returns the values of the budget tables | Tests `budget_uses_default_rtf_when_voice_is_default`, `budget_uses_other_rtf_when_voice_is_other`, `budget_limits_memory_when_qwen3_max`, `budget_has_no_ttfa_when_quality_is_max`, `budget_has_no_ttfa_when_quality_is_fast` and `budget_is_none_when_engine_is_fake` |
| AC-09-16 | `bench --check-budgets` exits with code 2 for an engine without a budget | Test `bench_fails_budget_check_when_engine_has_no_budget` with the fake voice |
| AC-09-17 | No other file of the crate holds a budget value | `rg -l "ttfa_warm:" tools/eval/src` prints only `tools/eval/src/budgets.rs` |
| AC-09-18 | The memory thread stops when the job ends | Test `memory_sampler_stops_when_stop_channel_disconnects` |
| AC-09-19 | The nightly filter skips the audio device tests | `cargo nextest list --workspace --all-features --run-ignored all -E "$NIGHTLY_FILTER"` lists no test of the `device` binary of `antenna-audio` |

## Decision rules

1. Read the memory with `memory-stats` only. Do not use `unsafe` code or `libc`.
2. If the GitHub macOS runner has no Metal device, keep `ANTENNA_DEVICE=cpu` in the nightly job.
3. If the nightly job takes more than 360 minutes after stages 12 and 14, run `wer` with `--sentences 10` in the nightly job. The full corpus runs in stage 19 and on the reference machine.
4. If the pipeline of stage 06 binds a library at construction, make one pipeline for each workspace. The engine load is outside the measured interval of each value except the cold time to first audio, so this does not change a result.

## Out of scope

1. Any engine. Stages 10 to 14 implement the engines.
2. Speed budgets in CI. Only the reference machine checks the speed budgets.
3. A listening test. The project owner does it in stages 12 and 14.
4. Whisper, normalization and alignment code. Stage 08 delivers them.
