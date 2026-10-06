# Stage 12. Qwen3 engine

## Goal

The `qwen3` engine streams 24 voices in three quality variants, meets all budgets and WER limits, and is the default engine of the registry.

## Context

Stages 10 and 11 made the decoder, the talker, the artifact constants and the 24 voice prompts. This stage connects them behind `EngineFactory` and `Engine`, declares the descriptor and measures the result. After this stage, the default voice of each language is a `qwen3` voice.

Stage 14 connects the Magpie engine with the same structure. The two stage files have the same sections and the same synthesis sequence. Where the engines do the same job, they have the same acceptance criteria.

### Pacing

One step of the talker makes one frame, which is 80 ms of audio. The runaway guard of `docs/architecture.md` section 9 item 6 stops a segment that does not end.

`antenna_ml::pace` (stage 08) owns the buffer of steps, the chunk schedule, the step limit, the retry and the stop. The engine declares only its values and gives two closures.

```rust
pub(crate) const PACING: Pacing = Pacing {
    chunks: ChunkSchedule { first: 2, rest: 4 },
    steps_per_char: 0.9,
    min_expected_steps: 12,
};
```

| Field | Value | Reason |
|---|---|---|
| `chunks.first` | 2 | The first chunk (160 ms of audio) comes after two talker steps. This keeps the time to first audio short |
| `chunks.rest` | 4 | After the first chunk, each decode has 4 steps (320 ms of audio). This decreases the decode overhead. The 2 s ring buffer of the player hides the larger step |
| `steps_per_char` | 0.9 before task 12, then the calibrated value | The expected number of steps for each character of segment text |
| `min_expected_steps` | 12 | The steps of 1 s of audio, rounded down, for a short segment |

### Artifacts

Stage 10 declared all artifact constants and the slices `SMALL_ARTIFACTS` and `LARGE_ARTIFACTS` in `src/descriptor/artifacts.rs`. The voices have no artifacts, because the voice prompts are embedded in the crate (stage 11). Thus the artifacts of a voice and a variant are the artifacts of the variant only.

| Variant | Artifacts | Model dtype on Metal | Model dtype on the CPU | First download |
|---|---|---|---|---|
| `Fast` | `SMALL_ARTIFACTS` | bf16 | f32 | 2.52 GB |
| `Balanced` | `LARGE_ARTIFACTS` | bf16 | f32 | 4.54 GB |
| `Max` | `LARGE_ARTIFACTS` | f32, converted from the bf16 file at load | f32 | None after `Balanced` |

The `Fast` and `Balanced` variants share 0.69 GB of files. The codec uses f32 in all variants, unless stage 10 decision rule 3 selected f16 on Metal.

## Read first

1. `CLAUDE.md`
2. `docs/architecture.md` sections 4, 5, 6, 7, 8, 9, 11 and 14
3. `docs/standards.md` sections 3.4, 6, 8, 11, 14, 18, 19 and 20
4. `docs/plans/08-ml-and-review.md`, `docs/plans/09-eval.md`, `docs/plans/10-qwen3-decoder.md` and `docs/plans/11-qwen3-talker.md`

## Scope

The stage can create or change these files only.

- `crates/engines/qwen3/Cargo.toml`, `crates/engines/qwen3/src/lib.rs`
- `crates/engines/qwen3/src/descriptor/mod.rs`, `crates/engines/qwen3/src/descriptor/voices.rs`
- `crates/engines/qwen3/src/factory.rs`, `crates/engines/qwen3/src/engine.rs`, `crates/engines/qwen3/src/model/steps.rs` (only if `GenerationSettings` needs a change)
- `crates/engines/qwen3/tests/conformance.rs`, `crates/engines/qwen3/tests/voices.rs`
- `crates/engines/registry/Cargo.toml` and `crates/engines/registry/src/` (the feature, the registration line, the `DEFAULT_VOICES` entries and their tests)
- The root `Cargo.toml` (the `[workspace.dependencies]` entry `antenna-engine-qwen3` only)
- `apps/cli/Cargo.toml` and `tools/eval/Cargo.toml` (the `engine-qwen3` feature line and the default features only)
- `.github/workflows/nightly.yml` (step 3 of stage 09 only)
- `docs/benchmarks.md`

## Deliverables

### Descriptor

| Field | Value |
|---|---|
| `id` | `qwen3` |
| `name` | `Qwen3-TTS` |
| `version` | `1` |
| `summary` | en "24 voices in six languages. The default engine." es "24 voces en seis idiomas. El motor por defecto." |
| `license` | Apache-2.0, `https://www.apache.org/licenses/LICENSE-2.0`, attribution "Qwen3-TTS by the Qwen team, Alibaba Cloud. Licensed under the Apache License 2.0." |
| `sample_rate` | 24000 |
| `max_segment_chars` | 300 |
| `variants` | `Variants { fast, balanced, max }` with the artifacts of the "Artifacts" section. The `parameters` text is "0.6B" for `fast` and "1.7B" for `balanced` and `max` |
| `voices` | 24 voices, four for each language, in the sequence of `Language::ALL` and then the archetype sequence of the table below |

The `version` is a plain integer as a string. Increment it by one in each change that alters the audio of a voice.

### Voices

The voice id is `qwen3/<language>-<key>`. The key comes from stage 11. Each voice has the description of its archetype.

| Archetype | Description (en) | Description (es) |
|---|---|---|
| `warm_female` | Warm, mid register, calm | Cálida, registro medio, pausada |
| `bright_female` | Bright, high register, close | Luminosa, aguda, cercana |
| `clear_male` | Clear, mid register, agile | Clara, registro medio, ágil |
| `deep_male` | Firm, low register, narrative | Firme, grave, narrativa |

| Language | `warm_female` | `bright_female` | `clear_male` | `deep_male` |
|---|---|---|---|---|
| `en` | Ava (`en-ava`) | Grace (`en-grace`) | Owen (`en-owen`) | Henry (`en-henry`) |
| `es` | Lucía (`es-lucia`) | Inés (`es-ines`) | Mateo (`es-mateo`) | Bruno (`es-bruno`) |
| `pt` | Beatriz (`pt-beatriz`) | Helena (`pt-helena`) | Rafael (`pt-rafael`) | Tiago (`pt-tiago`) |
| `fr` | Camille (`fr-camille`) | Juliette (`fr-juliette`) | Louis (`fr-louis`) | Hugo (`fr-hugo`) |
| `it` | Giulia (`it-giulia`) | Chiara (`it-chiara`) | Marco (`it-marco`) | Luca (`it-luca`) |
| `de` | Lena (`de-lena`) | Clara (`de-clara`) | Jonas (`de-jonas`) | Felix (`de-felix`) |

The default voice of each language is its `warm_female` voice. `DEFAULT_VOICES` in `antenna-engine-registry` contains `qwen3/en-ava`, `qwen3/es-lucia`, `qwen3/pt-beatriz`, `qwen3/fr-camille`, `qwen3/it-giulia` and `qwen3/de-lena`.

### Rust API

```rust
// src/lib.rs
pub use factory::Factory;

// src/factory.rs
#[derive(Clone, Copy, Debug, Default)]
pub struct Factory;
impl EngineFactory for Factory { /* descriptor() and load(voice, quality, files) */ }

// src/engine.rs
pub(crate) struct QwenEngine {
    frontend: Frontend,
    talker: Talker,
    codec: Codec,
    settings: GenerationSettings,  // the voice prompt and talker.defaults().sampling()
    primed: CodecState,            // the codec state after the reference codes of the voice prompt
    voice_id: VoiceId,
    pacing: Pacing,                // a copy of PACING
}
impl Engine for QwenEngine { /* synthesize */ }

// src/descriptor/mod.rs
pub(crate) const DESCRIPTOR: EngineDescriptor;
pub(crate) fn model_size(quality: Quality) -> ModelSize;        // Fast is Small, Balanced and Max are Large
pub(crate) fn model_dtype(quality: Quality, backend: Backend) -> DType;
```

`model_dtype` takes `antenna_ml::Backend`, so its tests run on each CI machine. The magpie engine of stage 14 has the same function.

`src/descriptor/voices.rs` contains the 24 voice descriptors, so that each file has fewer than 300 lines.

### `load` sequence

1. Select the device with `antenna_ml::select_device`. Select the dtype with `model_dtype(quality, antenna_ml::backend(&device))`.
2. Load the frontend, the talker and the codec from the `ModelFiles`, each with its `load` function. Each `load` drops its `VarBuilder` before it returns.
3. Load the embedded voice prompt of the voice, with the speaker embedding of `talker.size()`. Make sure that `talker.size()` equals `model_size(quality)`. Put the voice prompt and `talker.defaults().sampling()` in `GenerationSettings`.
4. Decode the reference codes of the voice prompt into a new codec state and keep that state as `primed`. Discard the audio.
5. Synthesize "Hello." one time and discard the audio, as a warm-up. Thus the first real segment does not pay for the Metal kernel compilation.
6. Each step returns `Qwen3Error`. `Factory::load` converts an error with `Qwen3Error::into_load` at the end of the sequence, one time.

### `synthesize` sequence

The `magpie` engine of stage 14 uses the same sequence.

1. Encode `segment.text()` with the frontend. Count its `char` values. The step limit is `self.pacing.step_limit(chars)`.
2. Clone `primed` into a new codec state.
3. Call `antenna_ml::pace` with `self.pacing.chunks`, the step limit and two closures.
   1. `generate(attempt, on_step)` calculates the seed with `Seed::for_segment(&self.voice_id, segment.text(), attempt)`. It starts `self.talker.generate` with `&self.settings`, the tokens and the seed, and gives each frame to `on_step`. It returns when the stream ends or when `on_step` returns `Break`.
   2. `flush(steps)` decodes the steps with `self.codec.decode` on the codec state and returns the result of `emit`.
4. Map the result of `pace`. `Paced::Ended` and `Paced::Stopped` give `Ok(())`. `Paced::Runaway` gives `EngineError::Runaway` with `segment.index()`. An error of `pace` is a `Qwen3Error`, and `synthesize` converts it with `Qwen3Error::into_inference`.

A retry occurs only before the first flush, so the codec state of a retry is still the clone of `primed`. The engine has no buffer, no step counter and no retry code of its own. The two closures borrow different fields of the engine (`talker` and `settings` in `generate`, `codec` in `flush`), so the borrows do not conflict.

## Tasks

1. Write `descriptor/mod.rs` and `descriptor/voices.rs` with the tables of the "Descriptor" and "Voices" sections. Remove the `#[expect(dead_code)]` attributes of stages 10 and 11 from `lib.rs`.
2. Write `QwenEngine::synthesize` with `antenna_ml::pace`.
3. Write `Factory` with the `load` sequence.
4. Add `tests/conformance.rs` with `antenna_engine_testkit::conformance!(Factory, files, ignore = "needs models")`. The `files` closure gets the `ModelFiles` with `ModelStore::open_default()` and `ModelStore::ensure` on `antenna_models::voice_artifacts`.
5. Add `tests/voices.rs` with the tests `every_voice_synthesizes_when_balanced` and `fast_variant_synthesizes_when_language_is_any`. The first test synthesizes one sentence with each of the 24 voices. The second test synthesizes one sentence with the default voice of each language in the `Fast` variant.
6. Add the feature `engine-qwen3` to `antenna-engine-registry`. The registry has no default features. Register `Factory`. Add the six voices of the "Voices" section to `DEFAULT_VOICES`, each entry with `#[cfg(feature = "engine-qwen3")]`, as stage 01 defines for each entry.
7. Add `antenna-engine-qwen3` to `[workspace.dependencies]` of the root `Cargo.toml`.
8. In `apps/cli/Cargo.toml` and `tools/eval/Cargo.toml`, add the feature `engine-qwen3 = ["antenna-engine-registry/engine-qwen3"]` and add it to the default features. Stage 19 adds the feature to the desktop app.
9. Turn on step 3 of `.github/workflows/nightly.yml`, the `wer` step of stage 09.
10. Run `antenna-eval bench --voice <id> --quality <q> --check-budgets` for each default voice in each of the three variants. Also run it for the `clear_male` voice of each language in the `Balanced` variant, for the budget of `VoiceRole::Other`. Write the 24 rows in `docs/benchmarks.md`.
11. If a default voice misses a budget, apply decision rule 1.
12. Calibrate `PACING.steps_per_char`. Take the largest `Audio per char (ms)` value of the six default-voice `Balanced` rows of task 10. Divide it by 80 ms and round up to the next 0.05. The calibration does not change the audio, so the rows of task 10 stay valid.
13. Run `antenna-eval wer` for each of the 24 voices in the `Balanced` variant. Also run it for each default voice in the `Fast` and `Max` variants. Write the 36 rows in `docs/benchmarks.md`.
14. Run `cargo nextest run -p antenna-engine-qwen3 --run-ignored all` on the reference machine. The nightly job skips the tests that load the 1.7B model, so this run is the only run of them.
15. Ask the project owner to do the listening check of `docs/architecture.md` section 11.4.
16. Run `cargo xtask check` and do the quality gate.

## Acceptance criteria

"Default voice" means the voice in `DEFAULT_VOICES` for that language at the end of the stage. Each budget check runs on the Apple M5 Pro 24 GB in a release build with Metal. `antenna-eval` gets the limits from `tools/eval/src/budgets.rs`. The tests that need models have `#[ignore = "needs models"]`.

| ID | Criterion | Check |
|---|---|---|
| AC-12-01 | The descriptor is valid | `check_descriptor` passes in the conformance suite |
| AC-12-02 | Each artifact hash in the descriptor matches the downloaded file | Commands 1 and 2 below the table |
| AC-12-03 | The engine passes the conformance suite on the reference machine **(manual)** | `cargo nextest run -p antenna-engine-qwen3 --run-ignored all` passes on the Apple M5 Pro 24 GB |
| AC-12-04 | Each of the 24 voices synthesizes in the `Balanced` variant | Test `every_voice_synthesizes_when_balanced` |
| AC-12-05 | The `Fast` variant synthesizes in all languages | Test `fast_variant_synthesizes_when_language_is_any` |
| AC-12-06 | The descriptor has four voices for each language with unique ids | Test `descriptor_has_four_voices_in_each_language` |
| AC-12-07 | Each voice has an English and a Spanish description | Test `descriptor_describes_each_voice_in_both_languages` |
| AC-12-08 | The first chunk has the audio of `PACING.chunks.first` steps | Test `engine_emits_short_first_chunk`, first chunk of `PACING.chunks.first × 1920` samples, with an engine of the `Fast` variant |
| AC-12-09 | The engine maps a runaway of `pace` to `EngineError::Runaway`, without audio when the limit comes before the first chunk and after the first chunk in the other case | Tests `runaway_fails_without_audio_when_limit_precedes_first_chunk` (`min_expected_steps` 0) and `runaway_fails_after_first_chunk_when_limit_follows_it` (`min_expected_steps` 1). Each test sets `steps_per_char` to 0 in the `pacing` copy of an engine of the `Fast` variant. AC-08-32 to AC-08-37 and AC-08-42 test the branches of `pace` in CI |
| AC-12-10 | The registry has the `warm_female` `qwen3` voice as the default voice of each language | Test `registry_defaults_to_qwen3_when_feature_enabled` |
| AC-12-11 | Time to first audio, engine in the pool, is 500 ms or less for each default voice in `Balanced` **(manual)** | `antenna-eval bench --voice <id> --quality balanced --check-budgets` exits with code 0, rows in `docs/benchmarks.md` |
| AC-12-12 | Time to first audio, weights in the page cache, engine not loaded, is 4 s or less for each default voice in `Balanced` **(manual)** | The same command and rows as AC-12-11 |
| AC-12-13 | The real-time factor of each default voice is 0.5 or less in `Balanced`, 0.8 or less in `Fast` and 0.9 or less in `Max`. The real-time factor of the `clear_male` voice of each language is 0.8 or less in `Balanced` **(manual)** | `antenna-eval bench --voice <id> --quality <q> --check-budgets` exits with code 0 for each of the 24 runs of task 10 |
| AC-12-14 | Playback of the eval corpus has 0 buffer underruns for each default voice in each variant **(manual)** | The same commands as AC-12-13 |
| AC-12-15 | Resident memory with one `Balanced` engine loaded is 6 GB or less **(manual)** | The same command as AC-12-11 |
| AC-12-16 | Resident memory with one `Max` engine loaded is 10 GB or less **(manual)** | `antenna-eval bench --voice <id> --quality max --check-budgets` exits with code 0 |
| AC-12-17 | The WER of each voice in `Balanced`, and of each default voice in `Fast` and `Max`, is at or below the limit of `docs/architecture.md` section 11.2 | `antenna-eval wer --voice <id> --quality <q>` exits with code 0 for each of the 36 runs |
| AC-12-18 | `docs/benchmarks.md` has the 24 speed rows and the 36 WER rows, and each WER row has the WER of the `numbers` subset | Command 3 below the table |
| AC-12-19 | `PACING.steps_per_char` comes from the measurement **(manual)** | The reviewer takes the largest `Audio per char (ms)` value of the six `qwen3` default voice rows in `Balanced`, divides it by 80, rounds up to the next 0.05 and compares the result with `PACING.steps_per_char` |
| AC-12-20 | The default voices sound natural in each language **(manual)** | The project owner signs off in the listening table of `docs/benchmarks.md` |
| AC-12-21 | The nightly job runs the `qwen3` tests that use the 0.6B model and the `Fast` WER of each default voice | The `workflow_dispatch` run on `main` is green. Link it in the stage report |
| AC-12-22 | The About text of the engine has the Qwen attribution | Test `descriptor_has_qwen_attribution` |
| AC-12-23 | Each quality selects the model size and dtype of the variant table | Tests `model_size_is_small_when_quality_is_fast` and `model_dtype_is_f32_when_quality_is_max_and_backend_is_metal` |
| AC-12-24 | The CLI can speak with a `qwen3` voice **(manual)** | `antenna speak --voice qwen3/es-lucia notes.md` plays audio |
| AC-12-25 | The crate has no copy of the streaming, sampling or pacing logic of `antenna-ml` | `rg -n -e "struct CausalConv" -e "enum Sampling" -e "struct ChunkSchedule" -e RUNAWAY_FACTOR -e "fn runaway_action" crates/engines/qwen3/src` prints nothing |
| AC-12-26 | The apps get the engine through their features | `cargo tree -p antenna-cli -e features -i antenna-engine-qwen3` prints the feature `engine-qwen3` |

The commands of AC-12-02 and AC-12-18 are these.

1. `antenna models pull qwen3/en-ava --quality fast` exits with code 0.
2. `antenna models pull qwen3/en-ava --quality balanced` exits with code 0.
3. `rg -c -F "qwen3/" docs/benchmarks.md` prints 60 or more.

## Decision rules

1. If a default voice misses AC-12-11, AC-12-13 or AC-12-14 in `Balanced`, do these steps in sequence. Stop at the first step that meets the budget.
   1. If only the time to first audio fails, set `PACING.chunks.first` to 1.
   2. If the decode overhead is more than 20% of the step time, set `PACING.chunks.rest` to 8.
   3. If the code predictor loop is the largest cost, apply stage 11 decision rule 6.
   4. If the budget still fails, write a "Blocked" section with the time of each part of a step. A human decides.
2. If `Max` misses its RTF budget after the steps of rule 1, write a "Blocked" section with the measured value. Do not remove the `Max` variant.
3. If AC-12-15 or AC-12-16 fails, keep the text embedding table (`[151936, 2048]`, 0.62 GB in bf16) on the CPU. Look up the rows on the CPU and copy only the result to the device.
4. If `check_determinism` fails on Metal, find the operation that is not deterministic with the module parity tests on Metal. Do not move the conformance suite to the CPU.
5. If AC-12-17 fails for a voice, examine the Whisper transcripts. If the voice reads the reference transcript of the voice prompt in the output, the prompt layout is wrong. Compare with the prompt fixture of stage 11. If the errors are in words of the target text, follow `docs/architecture.md` section 14, rule 3. If only one voice of a language fails, make its voice prompt again with a different seed in stage 11 `voices.toml`.
6. If a segment of 300 characters makes the talker exceed 4096 positions, decrease `max_segment_chars` to 250. Write the reason in the stage report.
7. Use the model store API that stage 04 delivered in the conformance test. Use the `antenna-ml` names that stage 08 delivered. Do not add a second API.

### Facts to check

1. That the Metal kernels of candle are deterministic for the talker. AC-12-03 checks it.

## Out of scope

1. Voices from user audio.
2. The x-vector-only voice mode.
3. A cache of the talker prefix between segments. The prefix contains the target text, so it changes for each segment.
4. Pronunciation review. `antenna-review` does it separately (stage 08).
