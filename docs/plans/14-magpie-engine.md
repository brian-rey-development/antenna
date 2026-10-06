# Stage 14. Magpie engine

## Goal

The `magpie` engine streams 30 voices through NanoCodec in three quality variants and meets the budgets and WER limits of a non-default voice.

## Context

Stage 13 made the text frontend, the model, the frame types and the artifact constants. This stage converts the frames into audio, connects the parts behind `EngineFactory` and `Engine`, declares the descriptor and registers the engine. After this stage, a user can select a Magpie voice in the CLI.

Stage 12 connects the Qwen3 engine with the same structure. The two stage files have the same sections and the same synthesis sequence. Where the engines do the same job, they have the same acceptance criteria.

NanoCodec (`nvidia/nemo-nano-codec-22khz-1.89kbps-21.5fps`) has these facts in its decoder GGUF.

| Property | Value |
|---|---|
| Sample rate | 22050 Hz |
| Samples per codec frame | 1024 (21.533 frames each second) |
| Quantizer | Group FSQ, 8 groups of 4 dimensions, levels `[8, 7, 6, 6]`, base index `[1, 8, 56, 336]`, scale `[4, 3, 3, 3]`, offset `[4, 3, 3, 3]`, latent dimension 32 |
| Decoder | Causal HiFi-GAN. Input convolution kernel 7 from 32 to 864 channels |
| Upsample rates | `[8, 8, 4, 2, 2]`, channels 864, 432, 216, 108, 54, 27 |
| Residual blocks | Kernels `[3, 7, 11]`, dilations `[1, 3, 5]` |
| Activation | `half_snake` with `alpha` and `alpha_inv` parameters for half of the channels |
| Output | Convolution kernel 3 to 1 channel, then clamp to `[-1, 1]` |

The causal convolutions, their states and the `Streaming` trait come from `antenna_ml::stream` (stage 08). The Qwen3 codec of stage 10 uses the same parts.

### Pacing

One decoder step of stage 13 makes 2 codec frames, which is 2048 samples or 92.88 ms of audio. The runaway guard of `docs/architecture.md` section 9 item 6 stops a segment that does not end.

`antenna_ml::pace` (stage 08) owns the buffer of steps, the chunk schedule, the step limit, the retry and the stop. The engine declares only its values and gives two closures.

```rust
pub(crate) const PACING: Pacing = Pacing {
    chunks: ChunkSchedule { first: 1, rest: 4 },
    steps_per_char: 0.8,
    min_expected_steps: 10,
};
```

| Field | Value | Reason |
|---|---|---|
| `chunks.first` | 1 | The first chunk (92.88 ms of audio) comes after one decoder step. This keeps the time to first audio short |
| `chunks.rest` | 4 | After the first chunk, each decode has 4 steps (371.5 ms of audio). This decreases the decode overhead. The 2 s ring buffer of the player hides the larger step |
| `steps_per_char` | 0.8 before task 16, then the calibrated value | The expected number of steps for each character of segment text |
| `min_expected_steps` | 10 | The steps of 1 s of audio, rounded down, for a short segment |

The step limit of a segment is the smaller value of `self.pacing.step_limit(chars)` and `self.model.config().max_decoder_steps()`. The model has a limit of its own, so this is the only difference from the `qwen3` sequence.

### Variants

| Variant | Guidance | Model dtype on Metal | Model dtype on the CPU |
|---|---|---|---|
| `Fast` | `Guidance::Off`, a batch of 1 | f16 | f32 |
| `Balanced` | `Guidance::Cfg { scale: 2.5 }`, a batch of 2 | f16 | f32 |
| `Max` | `Guidance::Cfg { scale: 2.5 }`, a batch of 2 | f32 | f32 |

The codec uses f32 in all variants. All variants have the artifacts `VARIANT_ARTIFACTS` of stage 13.

## Read first

1. `CLAUDE.md`
2. `docs/architecture.md` sections 4, 5, 6, 7, 8, 9, 11 and 14
3. `docs/standards.md` sections 3.4, 6, 8, 11, 14, 18, 19 and 20
4. `docs/plans/08-ml-and-review.md`, `docs/plans/09-eval.md`, `docs/plans/12-qwen3-engine.md` and `docs/plans/13-magpie-model.md`
5. The NeMo source of `CausalHiFiGANDecoder` and `GroupFiniteScalarQuantizer` in `nemo/collections/tts/modules/audio_codec_modules.py`
6. NVIDIA/NeMo-Speech.cpp, files `src/tts/nanocodec/*` and `src/s2s/codec/decoder.cpp`

## Scope

The stage can create or change these files only.

- `tools/reference/magpie/dump_fixtures.py` and the new fixtures `codec.safetensors` and `sentence_es.safetensors` in `crates/engines/magpie/tests/fixtures/`
- `crates/engines/magpie/Cargo.toml`, `crates/engines/magpie/src/lib.rs`
- `crates/engines/magpie/src/codec/`
- `crates/engines/magpie/src/descriptor/mod.rs`, `crates/engines/magpie/src/descriptor/voices.rs`
- `crates/engines/magpie/src/factory.rs`, `crates/engines/magpie/src/engine.rs`
- `crates/engines/magpie/tests/conformance.rs`, `crates/engines/magpie/tests/voices.rs`
- `crates/engines/magpie/benches/codec.rs`, `crates/engines/magpie/benches/step.rs`
- `crates/engines/registry/Cargo.toml` and `crates/engines/registry/src/` (the feature, the registration line and its tests)
- The root `Cargo.toml` (the `[workspace.dependencies]` entry `antenna-engine-magpie` only)
- `apps/cli/Cargo.toml` and `tools/eval/Cargo.toml` (the `engine-magpie` feature line and the default features only)
- `docs/benchmarks.md` (new rows only)

## Deliverables

### Codec

```
src/codec/
├── mod.rs              # Codec and CodecState
├── fsq.rs              # group FSQ: code indices to the 32-dimension latent
├── hifigan.rs          # the causal HiFi-GAN decoder and half_snake
└── parity.rs           # #[cfg(test)] parity and split tests against the fixtures
```

```rust
// src/codec/mod.rs
pub(crate) struct Codec { /* FSQ, decoder modules */ }
impl Codec {
    pub(crate) fn load(files: &ModelFiles, device: &Device) -> Result<Self, MagpieError>;
    pub(crate) fn start(&self) -> CodecState;
    pub(crate) fn decode(&self, state: &mut CodecState, frames: &[Frame]) -> Result<PcmChunk, MagpieError>;
}

#[derive(Clone)]
pub(crate) struct CodecState { /* one state value for each stateful module */ }
```

1. `Codec::load` opens the `codec` file with `antenna_ml::GgufWeights`. The Qwen3 codec of stage 10 has the same three methods with the same argument types.
2. `decode` converts the frames to a tensor with `antenna_ml::stream::codes_tensor` and returns exactly `frames.len() × 1024` samples. The engine passes the frames of its stacked frames with `as_flattened`.
3. A state update makes new tensors and never writes into an existing tensor, as stage 08 defines for `Streaming`. Thus a clone of `CodecState` copies no tensor data, and the decode of a clone does not change the original.
4. The causal convolutions and the causal transposed convolutions are `antenna_ml::stream::CausalConv1d` and `antenna_ml::stream::CausalConvTranspose1d`. The residual blocks and the upsample stages implement `antenna_ml::stream::Streaming`. This crate does not contain a second convolution with state.

### Descriptor

| Field | Value |
|---|---|
| `id` | `magpie` |
| `name` | `Magpie TTS Multilingual` |
| `version` | `1` |
| `summary` | en "30 voices in six languages. A 0.65 GB download." es "30 voces en seis idiomas. Una descarga de 0,65 GB." |
| `license` | NVIDIA Open Model License, `https://www.nvidia.com/en-us/agreements/enterprise-software/nvidia-open-model-license`, attribution "Licensed by NVIDIA Corporation under the NVIDIA Open Model License" |
| `sample_rate` | 22050 |
| `max_segment_chars` | 300 |
| `variants` | `Variants { fast, balanced, max }`. Each variant has `VARIANT_ARTIFACTS`. The `parameters` text of each variant is "357M" |
| `voices` | 5 speakers × 6 languages, in the sequence of `Language::ALL` and then the NeMo speaker index |

The `version` is a plain integer as a string. Increment it by one in each change that alters the audio of a voice.

### Voices

The voice id is `magpie/<language>-<speaker>`, for example `magpie/es-sofia`. The name is the speaker name. The artifacts of a voice are `language_artifacts(language)` of stage 13, so the five voices of one language share one static slice. The Portuguese voices speak Brazilian Portuguese.

| Speaker | NeMo index | Description (en) | Description (es) |
|---|---|---|---|
| Aria | 0 | Bright, mid register, friendly | Luminosa, registro medio, amable |
| Jason | 1 | Clear, mid register, direct | Clara, registro medio, directa |
| John | 2 | Deep, low register, narrative | Profunda, grave, narrativa |
| Leo | 3 | Warm, low register, relaxed | Cálida, grave, relajada |
| Sofia | 4 | Soft, mid register, calm | Suave, registro medio, serena |

The project owner checks these descriptions against the audio in AC-14-21. The default voices of the registry stay the `qwen3` voices.

### Rust API

```rust
// src/lib.rs
pub use factory::Factory;

// src/factory.rs
#[derive(Clone, Copy, Debug, Default)]
pub struct Factory;
impl EngineFactory for Factory { /* descriptor() and load(voice, quality, files) */ }

// src/engine.rs
pub(crate) struct MagpieEngine {
    frontend: Frontend,
    model: MagpieModel,
    codec: Codec,
    settings: GenerationSettings,  // speaker, guidance of the variant, model.config().sampling()
    voice_id: VoiceId,
    pacing: Pacing,                // a copy of PACING
}
impl Engine for MagpieEngine { /* synthesize */ }

// src/descriptor/mod.rs
pub(crate) const DESCRIPTOR: EngineDescriptor;
pub(crate) fn guidance(quality: Quality) -> Guidance;          // Fast is Off, Balanced and Max are Cfg { scale: 2.5 }
pub(crate) fn model_dtype(quality: Quality, backend: Backend) -> DType;
```

`model_dtype` takes `antenna_ml::Backend`, as in stage 12.

`src/descriptor/voices.rs` contains the 30 voice descriptors, so that each file has fewer than 300 lines.

### `load` sequence

1. Select the device with `antenna_ml::select_device`. Select the dtype with `model_dtype(quality, antenna_ml::backend(&device))`.
2. Load the frontend, the model and the codec from the `ModelFiles`, each with its `load` function.
3. Get the speaker with `Speaker::from_name` and the speaker part of the voice key. Get the guidance with `guidance`.
4. Synthesize "Hello." one time and discard the audio, as a warm-up. Thus the first real segment does not pay for the Metal kernel compilation.
5. Each step returns `MagpieError`. `Factory::load` converts an error with `MagpieError::into_load` at the end of the sequence, one time.

### `synthesize` sequence

The `qwen3` engine of stage 12 uses the same sequence.

1. Encode `segment.text()` with the frontend. Count its `char` values. The step limit is the value of the "Pacing" section.
2. Start a new codec state with `self.codec.start()`.
3. Call `antenna_ml::pace` with `self.pacing.chunks`, the step limit and two closures.
   1. `generate(attempt, on_step)` calculates the seed with `Seed::for_segment(&self.voice_id, segment.text(), attempt)`. It starts `self.model.generate` with `&self.settings`, the tokens and the seed, and gives each stacked frame to `on_step`. It returns when the stream ends or when `on_step` returns `Break`.
   2. `flush(steps)` decodes `steps.as_flattened()` with `self.codec.decode` on the codec state and returns the result of `emit`.
4. Map the result of `pace`. `Paced::Ended` and `Paced::Stopped` give `Ok(())`. `Paced::Runaway` gives `EngineError::Runaway` with `segment.index()`. An error of `pace` is a `MagpieError`, and `synthesize` converts it with `MagpieError::into_inference`.

A retry occurs only before the first flush, so the codec state of a retry is still the new state. The engine has no buffer, no step counter and no retry code of its own. The two closures borrow different fields of the engine (`model` and `settings` in `generate`, `codec` in `flush`), so the borrows do not conflict.

## Tasks

1. Add two fixtures to `dump_fixtures.py`. `codec.safetensors` has the latent of 4 fixed stacked frames and the PCM of the decoder. `sentence_es.safetensors` has the decoder PCM of the 50 frames of `greedy_es.safetensors` of stage 13, which is 51200 samples. Run the script and commit the files.
2. Write `codec/fsq.rs`. Make the FSQ parity test pass.
3. Write `codec/hifigan.rs` and `codec/mod.rs`. Make the codec parity test pass for one call on all frames.
4. Write the split tests. Decode the 50 frames of `greedy_es.safetensors` (stage 13) in one call, then in calls of 1, 2, 3, 4 and 7 frames. Compare each result with the one-call result.
5. Write `descriptor/mod.rs` and `descriptor/voices.rs` with the tables of the "Descriptor" and "Voices" sections. Remove the `#[expect(dead_code)]` attributes of stage 13 from `lib.rs`.
6. Write `MagpieEngine::synthesize` with `antenna_ml::pace`.
7. Write `Factory` with the `load` sequence.
8. Add `tests/conformance.rs` with `antenna_engine_testkit::conformance!(Factory, files, ignore = "needs models")`. The `files` closure gets the `ModelFiles` with `ModelStore::open_default()` and `ModelStore::ensure` on `antenna_models::voice_artifacts`.
9. Add `tests/voices.rs` with the tests `every_voice_synthesizes_when_balanced` and `fast_variant_synthesizes_when_language_is_any`. The first test synthesizes one sentence with each of the 30 voices. The second test synthesizes one sentence with the Sofia voice of each language in the `Fast` variant.
10. Add the feature `engine-magpie` to `antenna-engine-registry`. The registry has no default features. Register `Factory` after the `qwen3` line. Do not change `DEFAULT_VOICES`.
11. Add `antenna-engine-magpie` to `[workspace.dependencies]` of the root `Cargo.toml`. In `apps/cli/Cargo.toml` and `tools/eval/Cargo.toml`, add the feature `engine-magpie = ["antenna-engine-registry/engine-magpie"]` and add it to the default features. Stage 19 adds the feature to the desktop app. Start the nightly workflow with `workflow_dispatch` on `main`.
12. Write `benches/codec.rs` with `divan`. Measure `decode` of 2 frames and of 8 frames on Metal, from a state that already holds 100 frames.
13. Write `benches/step.rs`. Measure one decoder step with its local transformer on Metal in `Balanced`, after the prefill of a 25-word sentence. Add the medians of both benches to the module benchmarks table of `docs/benchmarks.md`.
14. Run `antenna-eval bench --voice <id> --quality <q> --check-budgets` for the Sofia voice of each language in each of the three variants. Write the 18 rows in `docs/benchmarks.md`.
15. If a voice misses a budget, apply decision rule 1.
16. Calibrate `PACING.steps_per_char`. Take the largest `Audio per char (ms)` value of the six `Balanced` rows of task 14. Divide it by 92.88 ms and round up to the next 0.05. The calibration does not change the audio, so the rows of task 14 stay valid.
17. Run `antenna-eval wer` for each of the 30 voices in the `Balanced` variant. Also run it for the Sofia voice of each language in the `Fast` and `Max` variants. Write the 42 rows in `docs/benchmarks.md`.
18. Ask the project owner to do the listening check of AC-14-21.
19. Run `cargo xtask check` and do the quality gate.

## Acceptance criteria

Each budget check runs on the Apple M5 Pro 24 GB in a release build with Metal. `antenna-eval` gets the limits from `tools/eval/src/budgets.rs`. The Magpie voices are not default voices, so the budgets of `VoiceRole::Other` apply. The tests that need models have `#[ignore = "needs models"]` and get their files as in stage 13.

| ID | Criterion | Check |
|---|---|---|
| AC-14-01 | FSQ dequantization matches the reference | Test `fsq_matches_reference`, max absolute difference 1e-4 |
| AC-14-02 | The codec decoder matches the reference | Test `codec_matches_reference`, max absolute difference 1e-3 on the CPU in f32 |
| AC-14-03 | Streaming in any split gives the one-call result | Tests `decode_is_split_invariant_when_chunk_is_<n>` for 1, 2, 3, 4 and 7, max absolute difference 1e-5 |
| AC-14-04 | The decoder PCM of the 50 greedy Spanish frames matches the reference | Test `sentence_matches_reference_when_greedy`, 51200 samples, max absolute difference 1e-3 |
| AC-14-05 | The engine passes the conformance suite | `cargo nextest run -p antenna-engine-magpie --run-ignored all` |
| AC-14-06 | The descriptor has five voices for each language with unique ids | Test `descriptor_has_five_voices_in_each_language` |
| AC-14-07 | Each artifact hash in the descriptor matches the downloaded file | Commands 1 and 2 below the table |
| AC-14-08 | The first chunk has the audio of `PACING.chunks.first` steps | Test `engine_emits_short_first_chunk`, first chunk of `PACING.chunks.first × 2048` samples, with an engine of the `Fast` variant |
| AC-14-09 | The engine maps a runaway of `pace` to `EngineError::Runaway`, without audio when the limit comes before the first chunk and after the first chunk in the other case | Tests `runaway_fails_without_audio_when_limit_precedes_first_chunk` (`min_expected_steps` 0) and `runaway_fails_after_first_chunk_when_limit_follows_it` (`min_expected_steps` 1). Each test sets `steps_per_char` to 0 in the `pacing` copy of an engine of the `Fast` variant. AC-08-32 to AC-08-37 and AC-08-42 test the branches of `pace` in CI |
| AC-14-10 | Time to first audio, engine in the pool, is 300 ms or less for each Sofia voice in `Balanced` **(manual)** | `antenna-eval bench --voice <id> --quality balanced --check-budgets` exits with code 0, rows in `docs/benchmarks.md` |
| AC-14-11 | The real-time factor of each Sofia voice is 0.8 or less in `Balanced` and `Fast`, and 0.9 or less in `Max` **(manual)** | `antenna-eval bench --voice <id> --quality <q> --check-budgets` exits with code 0 for the three qualities |
| AC-14-12 | The WER of each voice in `Balanced`, and of each Sofia voice in `Fast` and `Max`, is at or below the limit of `docs/architecture.md` section 11.2 | `antenna-eval wer --voice <id> --quality <q>` exits with code 0 for each of the 42 runs |
| AC-14-13 | `docs/benchmarks.md` has the 18 speed rows and the 42 WER rows, and each WER row has the WER of the `numbers` subset | Command 3 below the table |
| AC-14-14 | The CLI can speak with a `magpie` voice **(manual)** | `antenna speak --voice magpie/es-sofia notes.md` plays audio |
| AC-14-15 | The About text of the engine has the NVIDIA attribution | Test `descriptor_has_nvidia_attribution` |
| AC-14-16 | Each quality selects the guidance and dtype of the variant table | Tests `guidance_is_off_when_quality_is_fast` and `model_dtype_is_f32_when_quality_is_max_and_backend_is_metal` |
| AC-14-17 | Each voice has an English and a Spanish description | Test `descriptor_describes_each_voice_in_both_languages` |
| AC-14-18 | Time to first audio, weights in the page cache, engine not loaded, is 4 s or less for each Sofia voice in `Balanced` **(manual)** | The same command and rows as AC-14-10 |
| AC-14-19 | Playback of the eval corpus has 0 buffer underruns for each Sofia voice in each variant **(manual)** | The same commands as AC-14-11 |
| AC-14-20 | `PACING.steps_per_char` comes from the measurement **(manual)** | The reviewer takes the largest `Audio per char (ms)` value of the six Sofia rows in `Balanced`, divides it by 92.88, rounds up to the next 0.05 and compares the result with `PACING.steps_per_char` |
| AC-14-21 | Each speaker description matches the voice, in English and in Spanish **(manual)** | The project owner listens to one sentence of each speaker in both languages and signs off in the listening table of `docs/benchmarks.md` |
| AC-14-22 | The nightly job runs the `magpie` tests | The `workflow_dispatch` run on `main` is green. Link it in the stage report |
| AC-14-23 | The registry keeps the `qwen3` default voices when the `magpie` feature is enabled | Test `registry_keeps_qwen3_defaults_when_magpie_enabled` |
| AC-14-24 | The crate has no copy of the streaming, sampling or pacing logic of `antenna-ml` | `rg -n -e "struct CausalConv" -e "enum Sampling" -e "struct ChunkSchedule" -e RUNAWAY_FACTOR -e "fn runaway_action" crates/engines/magpie/src` prints nothing |
| AC-14-25 | The descriptor is valid | `check_descriptor` passes in the conformance suite |
| AC-14-26 | Each of the 30 voices synthesizes in the `Balanced` variant | Test `every_voice_synthesizes_when_balanced` |
| AC-14-27 | The `Fast` variant synthesizes in all languages | Test `fast_variant_synthesizes_when_language_is_any` |
| AC-14-28 | `decode` returns 1024 samples for each frame | Test `decode_returns_1024_samples_for_each_frame` |
| AC-14-29 | A decode with the original state does not change a clone | Test `cloned_state_is_independent_when_original_decodes`. Decode 20 frames P and clone the state. Decode 12 frames A with the original and 12 frames B with the clone. The clone output equals the output of a new state that decodes P and then B, max absolute difference 1e-5 |
| AC-14-30 | Decoding 8 frames on Metal takes 40 ms or less (median) on the Apple M5 Pro 24 GB **(manual)** | `cargo bench -p antenna-engine-magpie --bench codec`, rows in the module benchmarks table of `docs/benchmarks.md` |
| AC-14-31 | The median decoder step in `Balanced` on Metal takes 64 ms or less on the Apple M5 Pro 24 GB **(manual)** | `cargo bench -p antenna-engine-magpie --bench step`, rows in the module benchmarks table of `docs/benchmarks.md` |
| AC-14-32 | The new fixtures exist and each is smaller than 2 MB | `find crates/engines/magpie/tests/fixtures -size +2M` prints nothing |
| AC-14-33 | `dump_fixtures.py` makes the same fixtures again, bit for bit | Commands 4 and 5 below the table |
| AC-14-34 | The apps get the engine through their features | `cargo tree -p antenna-cli -e features -i antenna-engine-magpie` prints the feature `engine-magpie` |

The commands of AC-14-07, AC-14-13 and AC-14-33 are these.

1. For each `<language>` of `en`, `es`, `pt`, `fr`, `it` and `de`, run `antenna models pull magpie/<language>-sofia`.
2. Each command of step 1 exits with code 0.
3. `rg -c -F "magpie/" docs/benchmarks.md` prints 60 or more.
4. In `tools/reference/magpie`, run `uv sync --locked` and `uv run dump_fixtures.py`.
5. Run `git diff --exit-code crates/engines/magpie/tests/fixtures`. The command exits with code 0.

The limits of AC-14-30 and AC-14-31 come from the real-time factor budget of `VoiceRole::Other` in `docs/architecture.md` section 8. One step is 92.88 ms of audio. A real-time factor of 0.8 gives 74.3 ms for each step. The codec gets 10 ms of each step (40 ms for 4 steps), and the decoder step gets the rest, rounded down to 64 ms.

## Decision rules

1. If a voice misses AC-14-10, AC-14-11 or AC-14-19, do these steps in sequence. Stop at the first step that meets the budget.
   1. Profile with `tracing` spans for frontend, encoder, decoder step, local transformer and codec.
   2. Compute the cross-attention keys and values of the encoder output one time for each segment, and keep them for all steps.
   3. Run the local transformer on the CPU and the decoder on Metal. NeMo-Speech.cpp has the same option (`--tts.lt-backend`). Select the faster device for each part with a measurement.
   4. If the budget still fails, write a "Blocked" section with the time of each part of a step. A concurrent codec decode needs a second driver beside `antenna_ml::pace`, so a human decides.
2. If the WER of a voice fails, examine the transcripts first. If the cause is the frontend, fix the frontend. If the cause is the model for one speaker in one language, write a "Blocked" section with the WER and the transcripts. Do not remove the voice. A human decides.
3. If the number subset of a language has a WER above 25%, write a row in `docs/benchmarks.md` and continue. The text normalizer is a future stage.
4. If the `Fast` variant fails the WER limit of a language, write a "Blocked" section with the WER of each variant. The fix changes the variant table of `docs/architecture.md` section 4.2, so a human decides.
5. Stage 04 implements byte-range artifacts (ADR 0008). If a range download fails, report a defect of stage 04 and stop.
6. If `check_determinism` fails on Metal, find the operation that is not deterministic with the module parity tests on Metal. Do not move the conformance suite to the CPU.
7. Use the model store API that stage 04 delivered in the conformance test. Use the `antenna-ml` names that stage 08 delivered. Do not add a second API.

### Facts to check

| Fact | Check |
|---|---|
| `half_snake` applies snake to the first half of the channels and a different function to the second half | Read `HalfSnake` in `audio_codec_modules.py` |
| The transposed convolutions of the causal decoder need only a right-side trim for streaming | AC-14-03 |
| The attribution text of the NVIDIA Open Model License | Read section "Attribution" of the license at the URL in the descriptor |

## Out of scope

1. A change to the default voices. Qwen3 is the default engine.
2. Text normalization.
3. Voice cloning. The v2607 release has no voice cloning.
4. Batch synthesis of more than one segment at a time.
5. Pronunciation review. `antenna-review` does it separately (stage 08).
