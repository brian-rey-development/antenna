# Stage 13. Magpie model

## Goal

`antenna-engine-magpie` converts text in the six languages into codec frames, and each part matches the NeMo reference in the parity tolerances.

## Context

Magpie TTS Multilingual 357M (release v2607) is the second engine of Antenna. It has five baked speakers and makes speech in all six languages. This stage builds the part that converts text into codec codes, because that part has the most risk. Stage 14 adds the NanoCodec decoder, the `Engine` implementation and the descriptor value. If this part matches the reference, stage 14 only connects parts that work.

This stage is the first stage of the `magpie` engine. It makes the crate, the frame types and all artifact constants of the engine, as stage 10 does for `qwen3`.

The model has these parts.

| Part | Facts from the v2607 GGUF metadata |
|---|---|
| Text vocabulary | 3359 tokens. 15 tokenizers in one table, in the v2607 sequence |
| Encoder | 6 causal layers, d_model 768, 12 heads, FFN 3072 with a convolution of kernel 3, learnable positions (2048) |
| Decoder | 12 causal layers, d_model 768, 12 self-attention heads, 1 cross-attention head of dimension 128, FFN 3072 with kernel 1, learnable positions (2048) |
| Speaker context | Baked context embedding, 5 speakers, 217 frames of dimension 768 each, put before the audio frames in the decoder input |
| Audio tokens | 8 codebooks of 2016 codes. Frame stacking factor 2, so 16 audio embeddings and one decoder step makes 2 codec frames |
| Local transformer | 2 layers, 12 heads, hidden 768, context 18. It makes the 16 codes of a step in sequence |
| Output projection | `final_proj` 768 to 32384 (16 × 2024) and 16 local transformer projections of 768 to 2024 |
| Special audio tokens | BOS 2016, EOS 2017, context BOS 2018, context EOS 2019, mask 2020 |
| Inference defaults | Temperature 0.6, top-k 80, CFG scale 2.5, max 500 decoder steps, min 4 generated frames, attention prior on, prior epsilon 0.1, lookahead window 6, alignment from decoder layers 4, 5, 8, 9, prior on layers 2 to 10, EOS method `argmax_or_multinomial_any`, argmax temperature 0.01 |

The language tokenizers of v2607 are these.

| Antenna language | NeMo tokenizer | Type | Data |
|---|---|---|---|
| `En` | `english_phoneme` | IPA G2P | `ipa_cmudict-0.7b_nv23.01.txt`, `heteronyms-052722` |
| `Es` | `spanish_phoneme` | IPA G2P, locale `es-ES` | `es_ES_nv230301.dict` |
| `De` | `german_phoneme` | IPA G2P, locale `de-DE`, grapheme prefix `#`, mixed case | `de_nv230119.dict`, `de_nv230119.heteronym` |
| `Pt` | `portuguese_Brazilian_phoneme` | IPA G2P, locale `pt-BR`, grapheme prefix `#`, upper case | `pt_br_prondict-v1.0.dict` |
| `Fr` | `french_chartokenizer` | ByT5 bytes | None |
| `It` | `italian_chartokenizer` | ByT5 bytes | None |

Magpie Portuguese is Brazilian Portuguese. The dictionaries are members of the `.nemo` file. The `.nemo` file is an uncompressed tar of 1.47 GB, so Antenna downloads each dictionary as a byte range of that file. Stage 04 implements byte-range artifacts (ADR 0008).

### Artifacts

Variant artifacts. Each has the extent `Extent::Whole` with the bytes of the table.

| Constant | Key | Repository and revision | Path | Bytes | SHA-256 |
|---|---|---|---|---|---|
| `MODEL` | `model` | `nvidia/magpie_tts_multilingual_357m` at `19806879b16d3f2ccf28fb112b1bcd16a3c7923e` | `magpie_tts_multilingual_357m.v2607.f16.gguf` | 568663328 | `30d27551fc4a050e8095139f0185796024493373e83d49443c8ddba33c514e68` |
| `CODEC` | `codec` | `nvidia/nemo-nano-codec-22khz-1.89kbps-21.5fps` at `fc00890b604aa2de298d2641ffc6c5f6caf8c4d7` | `nemo_nano_codec_22khz_1.89kbps_21.5fps.decoder.f16.gguf` | 78823104 | `cc86d36d821a27cdc1d4ef600a3e2b0dabe76e88fcc2a8652d9543134c07ef2d` |

Language artifacts. Each artifact is a byte range of `magpie_tts_multilingual_357m.nemo` in `nvidia/magpie_tts_multilingual_357m` at `19806879b16d3f2ccf28fb112b1bcd16a3c7923e`. Its extent is `Extent::Range { offset, bytes }` with the values of the table. The tar file contains two copies of the English dictionary. The table uses the second copy, because tar readers use the last copy.

| Constant | Language | Key | Offset | Bytes | SHA-256 |
|---|---|---|---|---|---|
| `DICT_EN` | `En` | `dict-en` | 1459781120 | 3093097 | `7ea5c4c2c59cc780748b24d4ab60bd689779c4ac4ea7e27c0a8c3e20f300c0fa` |
| `HETERONYMS_EN` | `En` | `heteronyms-en` | 10909184 | 1606 | `b701909aedf753172eff223950f8859cd4b9b4c80199cf0a6e9ac4a307c8f8ec` |
| `DICT_ES` | `Es` | `dict-es` | 10919936 | 2230910 | `94c9a25bb359f733cd863c887d0112e7ae19a744b0333ef64b6a88e576428669` |
| `DICT_DE` | `De` | `dict-de` | 13199360 | 4313907 | `5c7bbf3346ebd6dc57769b5cc805124215cb1bcc3b8174c4254e9e928c5d6094` |
| `HETERONYMS_DE` | `De` | `heteronyms-de` | 13152768 | 44566 | `771cc585a574fd35bd14f4ce6108edf1e0e512a8dbc810db7de4c1165ba9d0ef` |
| `DICT_PT` | `Pt` | `dict-pt` | 3072 | 3580284 | `6492abc404db16bbad83dd9d7f9a60eb617699f0a3ff54b3be6184e14507b745` |

1. `src/descriptor/artifacts.rs` declares each artifact one time as a `pub(crate) const` with the name of the table.
2. The same file declares the slice `VARIANT_ARTIFACTS` with `MODEL` and `CODEC`, and the function `language_artifacts(language: Language) -> &'static [Artifact]`. French and Italian have an empty slice.
3. Stage 14 builds the variants and the voices of the descriptor from these values. No other file contains a revision, a size or a hash.

## Read first

1. `CLAUDE.md`
2. `docs/architecture.md` sections 4, 5.3, 7.4, 7.5, 11.1 and 14
3. `docs/standards.md` sections 3.4, 5, 8, 14 and 18
4. `docs/writing.md`
5. `docs/plans/08-ml-and-review.md`, for the `antenna-ml` API
6. The NeMo source of `nemo/collections/tts/models/magpietts.py`, `nemo/collections/tts/modules/transformer_2501.py`, `nemo/collections/tts/g2p/models/i18n_ipa.py` and `nemo/collections/common/tokenizers/text_to_speech/tts_tokenizers.py` at the commit in `tools/reference/magpie/uv.lock`
7. The Apache-2.0 C++ runtime NVIDIA/NeMo-Speech.cpp, files `src/tts/magpietts/*` and `src/tts/tokenizer/tokenizer_impl.cpp`. It reads the same GGUF file, so it is the reference for the tensor layout

## Scope

The stage can create or change these files only.

- `tools/reference/magpie/`
- `crates/engines/magpie/` except `src/codec/`, `src/engine.rs`, `src/factory.rs` and `src/descriptor/voices.rs`. In `src/descriptor/mod.rs`, this stage writes only the declaration of `artifacts`
- The root `Cargo.toml` (the new member, and the missing `[workspace.dependencies]` entries of task 4, for example `regex`)

## Deliverables

### Reference tool

`tools/reference/magpie/` is a uv project with `pyproject.toml`, `uv.lock`, `dump_tokens.py` and `dump_fixtures.py`. It pins `nemo_toolkit[tts]` to one git commit of NVIDIA/NeMo and pins `gguf` from PyPI.

1. `dump_tokens.py` writes `crates/engines/magpie/assets/tokens_v2607.json`. The file contains the symbol list, the offset, the BOS id and the EOS id of each of the 15 tokenizers, in the v2607 sequence. It also contains the `speakers` map from `speakers.json` of the `.nemo` file, from speaker name to NeMo index.
2. `dump_fixtures.py` writes the safetensors files of the "Fixtures" table to `crates/engines/magpie/tests/fixtures/`.
3. Before the tool runs the model, it rounds each weight that is F16 in the GGUF to f16 and back to f32. Thus the reference and the Rust code use the same weight values.
4. The tool sets the G2P phoneme probability to 1.0, so each word that is in a dictionary becomes phonemes.
5. The tool runs on the CPU in f32 with a fixed seed.

### Fixtures

| File | Content |
|---|---|
| `tokens_<language>.safetensors` | For each of the six languages, the token ids of 12 test sentences. The sentences include an out-of-dictionary word, a heteronym (EN, DE), an apostrophe, punctuation and a sentence of 3 tokens or less |
| `encoder.safetensors` | Text embeddings and the output of encoder layers 0 and 5 for one Spanish sentence |
| `context.safetensors` | The first 8 values of frame 0 and frame 216 of the baked context of each speaker index 0 to 4 |
| `decoder.safetensors` | The output of decoder layers 0 and 11 and the cross-attention scores of layers 4, 5, 8 and 9 for the first 3 decoder steps |
| `local_transformer.safetensors` | The logits of the 16 local transformer steps of decoder step 0 |
| `greedy_<language>.safetensors` | The codes of the first 25 decoder steps (50 codec frames) for one sentence and speaker 0 (Aria), with top-k 1, CFG on and the attention prior on |
| `greedy_no_cfg_es.safetensors` | The codes of the first 25 decoder steps for the Spanish sentence of `greedy_es`, with top-k 1, CFG off and the attention prior on. The `Fast` variant of stage 14 uses this mode |

Each fixture file is smaller than 2 MB. Each greedy sentence has 20 to 30 words, so its generation has more than 25 decoder steps.

### Crate modules

```
crates/engines/magpie/
├── assets/tokens_v2607.json
├── src/
│   ├── lib.rs
│   ├── frame.rs             # CODEBOOKS, Frame and StackedFrame
│   ├── error.rs             # MagpieError, the same pattern as Qwen3Error of stage 10
│   ├── config.rs            # MagpieConfig, read from the GGUF metadata (one source, so one file, see stage 10)
│   ├── descriptor/
│   │   ├── mod.rs           # the declaration of artifacts (stage 14 adds the descriptor value)
│   │   └── artifacts.rs     # the artifact constants
│   ├── frontend/
│   │   ├── mod.rs           # Frontend: text and language to token ids
│   │   ├── vocab.rs         # the token table from tokens_v2607.json
│   │   ├── ipa.rs           # dictionary G2P with heteronyms and grapheme fallback
│   │   ├── byt5.rs          # UTF-8 bytes to ByT5 ids
│   │   ├── voice.rs         # Speaker and the map from voice key to NeMo index
│   │   └── parity.rs        # #[cfg(test)] token and speaker parity tests
│   └── model/
│       ├── mod.rs           # MagpieModel: loads the weights, owns the modules
│       ├── layer.rs         # TransformerLayer: norm, self-attention, cross-attention, conv FFN
│       ├── attention.rs     # attention with a KV cache and an optional prior
│       ├── encoder.rs       # Encoder
│       ├── decoder.rs       # Decoder with the baked speaker context
│       ├── local.rs         # LocalTransformer
│       ├── prior.rs         # AlignmentTracker: the inference attention prior
│       ├── steps.rs         # StepStream: the step loop with CFG and the EOS decision
│       └── parity.rs        # #[cfg(test)] module and generation parity tests
└── tests/
    └── fixtures/
```

```rust
// src/frame.rs
pub(crate) const CODEBOOKS: usize = 8;
pub(crate) type Frame = [u32; CODEBOOKS];
pub(crate) type StackedFrame = [Frame; 2];                    // the codes of one decoder step

// src/error.rs
#[derive(Debug, thiserror::Error)]
pub(crate) enum MagpieError {
    #[error(transparent)] Ml(#[from] MlError),
    #[error(transparent)] Candle(#[from] candle_core::Error),
    #[error(transparent)] Files(#[from] EngineError),          // EngineError::MissingFile from ModelFiles::path
    #[error("cannot parse the token table")] Tokens(#[source] serde_json::Error),
    #[error("the GGUF has no metadata key {key}")] MissingMetadata { key: String },
    #[error("unknown Magpie speaker {name}")] UnknownSpeaker { name: String },
    #[error("cannot read {key}")] Dictionary { key: &'static str, #[source] source: std::io::Error },
}
impl MagpieError {
    pub(crate) fn into_load(self) -> EngineError;
    pub(crate) fn into_inference(self) -> EngineError;
}

// src/config.rs
pub(crate) struct MagpieConfig { /* every value of the "Context" table */ }
impl MagpieConfig {
    pub(crate) fn from_gguf(weights: &GgufWeights) -> Result<Self, MagpieError>;
    pub(crate) fn sampling(&self) -> SamplingConfig;          // temperature 0.6, top-k 80
    pub(crate) fn max_decoder_steps(&self) -> usize;          // 500
}

// src/frontend/mod.rs
pub(crate) struct Frontend { /* vocab, dictionaries */ }
impl Frontend {
    pub(crate) fn load(language: Language, files: &ModelFiles) -> Result<Self, MagpieError>;
    pub(crate) fn encode(&self, text: &str) -> Vec<u32>;      // BOS, tokens, short-text padding, EOS
}

// src/frontend/voice.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Speaker(u8);                                // NeMo index: Aria 0, Jason 1, John 2, Leo 3, Sofia 4
impl Speaker { pub(crate) fn from_name(name: &str) -> Result<Self, MagpieError>; }

// src/model/mod.rs
pub(crate) struct MagpieModel { /* config, encoder, decoder, local transformer, embeddings, projections, KV caches */ }
impl MagpieModel {
    pub(crate) fn load(files: &ModelFiles, device: &Device, dtype: DType) -> Result<Self, MagpieError>;
    pub(crate) fn config(&self) -> &MagpieConfig;
    pub(crate) fn generate(&mut self, settings: &GenerationSettings, tokens: &[u32], seed: Seed) -> Result<StepStream<'_>, MagpieError>;
}

// src/model/steps.rs
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Guidance { Off, Cfg { scale: f32 } }          // Off runs a batch of 1 with the conditional input only

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct GenerationSettings { pub(crate) speaker: Speaker, pub(crate) guidance: Guidance, pub(crate) sampling: SamplingConfig }

pub(crate) struct StepStream<'a> { /* borrows the model and owns the sampler and the alignment tracker of one segment */ }
impl Iterator for StepStream<'_> { type Item = Result<StackedFrame, MagpieError>; }   // one step makes two frames
```

1. Each function of the crate returns `MagpieError`. `into_load` and `into_inference` give `Files` unchanged and put each other variant in a box, as stage 10 defines for `Qwen3Error`. `SamplingConfig`, `Seed` and `GgufWeights` come from `antenna-ml`. The greedy parity tests use `SamplingConfig::GREEDY`. The crate defines no other sampling type.
2. `MagpieModel::load` opens the `model` file with `antenna_ml::GgufWeights`, reads `MagpieConfig` from it and converts the F16 tensors to `dtype`. Stage 14 selects f16 or f32 from the quality variant. The parity tests use f32 on the CPU.
3. `StepStream` ends after the EOS step, without that step. It does not stop at a maximum length. Stage 14 runs it through `antenna_ml::pace`, which adds the runaway guard, as stage 12 does for `qwen3`.
4. `MagpieModel::generate` has the same shape as `Talker::generate` of stage 11. The arguments are the settings, the tokens and the seed, in this sequence. Each model owns its preallocated KV caches, and `generate` resets them. The model state is never cloned, so a cache that writes in place is correct here.
5. `Speaker::from_name` takes the speaker part of the voice key, for example `sofia` of `es-sofia`. It compares the name without case with the names of the `speakers` map. An unknown name gives `MagpieError::UnknownSpeaker` with the name.

## Tasks

1. Make the uv project in `tools/reference/magpie/`. Pin NeMo with `uv add "nemo_toolkit[tts] @ git+https://github.com/NVIDIA/NeMo@<commit>"`. Use the newest commit that loads the v2607 `.nemo` file.
2. Write `dump_tokens.py`. Run it and commit `assets/tokens_v2607.json`.
3. Write `dump_fixtures.py`. Run it and commit the fixtures. Write the NeMo commit and the run command in the docstring of the script.
4. Make the crate `crates/engines/magpie` with the modules of the "Crate modules" deliverable. Add it to the workspace `members`. The normal dependencies are `antenna-core`, `antenna-ml`, `candle-core`, `candle-nn`, `serde`, `serde_json`, `regex`, `thiserror` and `tracing`. `vocab.rs` embeds `tokens_v2607.json` with `include_str!` and reads it with `serde_json`. If the file is larger than 64 KB, put `#[expect(clippy::large_include_file, reason = "the token table of v2607")]` on the include. The dev-dependencies are `antenna-models` and `divan`.
5. In `lib.rs`, declare `frame`, `error`, `config`, `descriptor`, `frontend` and `model` with `#[expect(dead_code, reason = "stage 14 connects these modules to the engine")]`.
6. Write `descriptor/artifacts.rs` with the constants of the "Artifacts" section.
7. Write `config.rs`. Read each value from the `magpietts.*` metadata keys of the GGUF. Return `MagpieError::MissingMetadata` with the key if a key is missing.
8. Write `frontend/vocab.rs` and `frontend/byt5.rs`. Make the French and Italian token tests pass.
9. Write `frontend/ipa.rs`. Copy the word split, the case rules, the heteronym rule and the grapheme fallback of `IpaG2p`. Use the first pronunciation of an entry that has more than one. Make the English, Spanish, German and Portuguese token tests pass.
10. Write `frontend/mod.rs`. Add the BOS and EOS ids and the short-text padding of `pad_short_tokens_before_eos`.
11. Write `frontend/voice.rs`. Make the speaker parity test pass.
12. Write `model/mod.rs`. Load the tensors from the GGUF with `antenna_ml::GgufWeights`. Use the tensor names and the split FFN convolution taps (`weight.k0`, `weight.k1`, `weight.k2`) of the GGUF.
13. Write `model/attention.rs`, `model/layer.rs` and `model/encoder.rs`. Make the encoder parity test pass.
14. Write `model/decoder.rs`. Put the 217 frames of the baked context before the audio embeddings. Make the context parity test and the decoder parity test pass.
15. Write `model/local.rs`. Make the local transformer parity test pass.
16. Write `model/prior.rs`. Copy the logic of `construct_inference_prior` and of the alignment estimate from the cross-attention scores.
17. Write `model/steps.rs` and `MagpieModel::generate`. For `Guidance::Cfg`, run the conditional and the unconditional input as one batch of 2. Combine the logits with `(1 - cfg_scale) × uncond + cfg_scale × cond`.
18. In the same file, run the conditional input only, as a batch of 1, for `Guidance::Off`. Use `antenna_ml::Sampler` for top-k and temperature. Make the EOS decision with the `argmax_or_multinomial_any` method.
19. Make the greedy parity tests pass for the six languages with CFG on, and for Spanish with CFG off.
20. Run `cargo xtask check` and do the quality gate.

## Acceptance criteria

All parity tests run on the CPU in f32. Tests that need the GGUF file or a dictionary have `#[ignore = "needs models"]`. Each of these tests gets its files with one call, `ModelStore::open_default()?.ensure(VARIANT_ARTIFACTS.iter().chain(language_artifacts(language)), &|_| {}, &AtomicBool::default())?`.

| ID | Criterion | Check |
|---|---|---|
| AC-13-01 | The reference tool is reproducible | Commands 1 and 2 below the table |
| AC-13-02 | Each fixture is smaller than 2 MB | `find crates/engines/magpie/tests/fixtures -size +2M` prints nothing |
| AC-13-03 | The configuration comes from the GGUF | Test `config_reads_all_values_when_gguf_is_v2607` |
| AC-13-04 | The frontend gives the reference token ids in all six languages | Tests `frontend_matches_reference_when_language_is_<language>`, 12 sentences each, exact match |
| AC-13-05 | An out-of-dictionary word becomes graphemes with the correct prefix | Test `ipa_uses_graphemes_when_word_is_unknown` |
| AC-13-06 | A heteronym becomes graphemes | Test `ipa_uses_graphemes_when_word_is_heteronym` |
| AC-13-07 | The encoder matches the reference | Test `encoder_matches_reference`, max absolute difference 1e-4 |
| AC-13-08 | Speaker indices select the correct baked context | Test `context_matches_reference_when_speaker_is_<index>`, exact match |
| AC-13-09 | The decoder and its cross-attention scores match the reference | Test `decoder_matches_reference`, max absolute difference 1e-4 |
| AC-13-10 | The local transformer matches the reference | Test `local_transformer_matches_reference`, max absolute difference 1e-4 |
| AC-13-11 | Greedy generation gives the reference codes | Tests `greedy_codes_match_reference_when_language_is_<language>`, exact match of 50 frames |
| AC-13-12 | Greedy generation without CFG gives the reference codes with a batch of 1 | Test `greedy_codes_match_reference_when_cfg_is_off`, exact match of 50 frames |
| AC-13-13 | Generation stops at EOS | Test `generation_stops_when_eos_is_predicted` with a 5-word sentence, fewer than 100 steps |
| AC-13-14 | The same seed gives the same codes | Test `generation_is_deterministic_when_seed_is_fixed` |
| AC-13-15 | No file of the crate has more than 300 lines | `cargo xtask lint-repo` |
| AC-13-16 | The map from speaker name to NeMo index equals the `speakers` map of `tokens_v2607.json` | Test `speaker_indices_match_reference` |
| AC-13-17 | The artifact constants have valid values | Test `artifacts_have_valid_values` checks the 40-character revisions, the 64-character hashes, the non-zero sizes and the unique keys of each language set |
| AC-13-18 | The crate defines no sampling type of its own | `rg -n "enum Sampling" crates/engines/magpie/src` prints nothing |
| AC-13-19 | `Speaker::from_name` rejects an unknown name | Test `speaker_fails_when_name_is_unknown` |
| AC-13-20 | A different seed gives different codes | Test `generation_changes_when_seed_changes` compares the first 10 steps of two seeds with the sampling of `MagpieConfig::sampling` |
| AC-13-21 | A missing metadata key gives `MagpieError::MissingMetadata` with the key | Test `config_fails_when_metadata_key_is_missing` |
| AC-13-22 | An error keeps its variant until the engine boundary | Tests `into_load_keeps_missing_file_when_file_is_absent` and `into_inference_boxes_error_when_candle_fails` |

The commands of AC-13-01 are these.

1. In `tools/reference/magpie`, run `uv sync --locked`, `uv run dump_tokens.py` and `uv run dump_fixtures.py`.
2. Run `git diff --exit-code crates/engines/magpie/tests/fixtures crates/engines/magpie/assets`. The command exits with code 0.

## Decision rules

1. If a frontend test fails, compare the word split before the G2P lookup. Most differences come from the punctuation and apostrophe rules of `IPATokenizer`.
2. If a GGUF tensor has a shape that is the reverse of the PyTorch shape, transpose it in `model/mod.rs` when you load it. The GGUF stores the dimensions in GGML order.
3. If the greedy parity test fails after the module tests pass, compare the attention prior of each step. Then compare the EOS decision.
4. If the reference generation of a greedy sentence has a tie in its first 25 steps, change that sentence in `dump_fixtures.py`. Then run the script again. A tie is a difference smaller than 1e-4 between the two best logits of a code. Write the old sentence and the tie step in the stage report. Do not change the exact match of AC-13-11 and AC-13-12.
5. The NeMo reference can use random grapheme replacement even with probability 1.0. In that case, set the `_rng` of the G2P to a fixed seed in the tool. Write the reason in the script.
6. Antenna does not apply the NeMo WFST text normalization in the MVP. The number subset of the eval corpus measures the effect in stage 14. Do not write a text normalizer in this stage.
7. If the alignment estimate needs the iteration order of a Python `dict`, use a `Vec` of key and value pairs in insertion order in Rust.

### Facts to check

| Fact | Check |
|---|---|
| The layer norms have no bias | Read `transformer_2501.py`. The GGUF has only `norm_*.weight` tensors |
| The FFN activation is GELU, and its approximation | Read `PositionwiseConvFF` in `transformer_2501.py` |
| How the 16 audio embeddings combine into one decoder input | Read `embed_audio_tokens` in `magpietts.py` (sum or mean) |
| How the prior changes the cross-attention scores (multiplication after softmax, or a log value before softmax) | Read the cross-attention forward of `transformer_2501.py` |
| The local transformer also uses CFG | Read the local transformer sample function in `magpietts.py` |
| The GGUF speaker list `[John, Sofia, Aria, Jason, Leo]` is a display order and the embedding rows use the NeMo indices of `speakers.json` (Aria 0, Jason 1, John 2, Leo 3, Sofia 4) | AC-13-08 and AC-13-16 |
| What NeMo keeps when the EOS code is in the second frame of a step. `StepStream` drops the whole EOS step, which is correct only if NeMo also drops the first frame of that step | Read the EOS handling and the frame unstacking in `magpietts.py`. If NeMo keeps the first frame, write a "Blocked" section with the code path, because the step type of `antenna_ml::pace` then changes |

## Out of scope

1. The NanoCodec decoder, the `Engine` implementation and the descriptor value. Stage 14 does them.
2. Byte-range downloads. Stage 04 does them.
3. Text normalization of numbers and abbreviations.
4. Languages that are not in `Language`.
5. Metal performance work. Stage 14 measures and fixes the speed.
