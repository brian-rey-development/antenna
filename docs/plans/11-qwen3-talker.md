# Stage 11. Qwen3 talker, code predictor and voice prompts

## Goal

`antenna-engine-qwen3` converts text and a voice prompt into code frames, and its first 50 greedy frames equal the reference for both model sizes.

## Context

The talker is a Qwen3 language model with a "dual track" input. At each position, the input is the sum of a text embedding and a codec embedding. The talker predicts codebook 0 of the next frame. Then a small code predictor predicts codebooks 1 to 15 of the same frame, one after the other. Stage 10 decodes the frames into audio. Stage 12 connects the two parts.

This stage starts after stage 10. Stage 10 made the crate, the `Frame` type and the artifact constants that the parity tests of this stage use.

Each voice comes from a voice prompt. There are four voices for each language, two female and two male, so 24 voices in total. A voice prompt contains the codes of a reference clip, the token ids of its transcript and the speaker embedding of the clip. A Python tool makes the prompts one time. Thus the Rust code does not need the speech tokenizer encoder or the speaker encoder.

### Facts from the reference

The sources are `config.json` and `generation_config.json` of `Qwen/Qwen3-TTS-12Hz-1.7B-Base` at `fd4b254389122332181a7c3db7f27e918eec64e3` and of `Qwen/Qwen3-TTS-12Hz-0.6B-Base` at `5d83992436eae1d760afd27aff78a71d676296fc`, and `qwen_tts/core/models/modeling_qwen3_tts.py` and `qwen_tts/inference/qwen3_tts_model.py` at commit `022e286b98fbec7e1e916cb940cdf532cd9f488e` of `QwenLM/Qwen3-TTS`.

| Item | 0.6B | 1.7B |
|---|---|---|
| Talker layers | 28 | 28 |
| Talker hidden size | 1024 | 2048 |
| Talker MLP size | 3072 | 6144 |
| Talker attention | 16 heads of 128, 8 KV heads | 16 heads of 128, 8 KV heads |
| Text embedding | `[151936, 2048]` | `[151936, 2048]` |
| Text projection | MLP 2048 to 2048 to 1024, SiLU, with bias | MLP 2048 to 2048 to 2048, SiLU, with bias |
| Codec embedding and codec head | `[3072, 1024]` | `[3072, 2048]` |
| Code predictor | 5 layers, hidden 1024, MLP 3072, 16 heads of 128, 8 KV heads | Same |
| Code predictor input projection | None | Linear 2048 to 1024 with bias |
| Code predictor embeddings and heads | 15 of each, embeddings `[2048, 1024]`, heads `[2048, 1024]` | 15 of each, embeddings `[2048, 2048]`, heads `[2048, 1024]` |
| Speaker embedding size | 1024 | 2048 |
| Weights dtype | BF16 | BF16 |

Both transformers use the Qwen3 block. The block has these parts.

1. RMSNorm (eps 1e-6) before the attention and before the MLP
2. RMSNorm on each query and key head (QK norm)
3. RoPE with theta 1000000 and the half-rotation layout
4. A SwiGLU MLP with SiLU

The attention and the MLP have no bias. The talker config declares a multimodal RoPE with sections `[24, 20, 20]`. For text-only input, the three position ids are equal, so the result is the standard 1D RoPE.

| Token | Id |
|---|---|
| `<\|im_start\|>`, `<\|im_end\|>`, `assistant` | 151644, 151645, 77091 |
| `tts_pad`, `tts_bos`, `tts_eos` (text track) | 151671, 151672, 151673 |
| `codec_pad`, `codec_bos`, `codec_eos` | 2148, 2149, 2150 |
| `codec_think`, `codec_think_bos`, `codec_think_eos` | 2154, 2156, 2157 |
| Language ids for en, de, es, fr, it, pt | 2050, 2053, 2054, 2061, 2070, 2071 |

| Generation default | Value |
|---|---|
| Codebook 0 sampling | temperature 0.9, top-k 50, top-p 1.0, repetition penalty 1.05 over all codebook 0 ids of the segment |
| Codebooks 1 to 15 sampling | temperature 0.9, top-k 50, top-p 1.0, no repetition penalty |
| Suppressed codebook 0 ids | 2048 to 3071, except 2150 |
| Minimum frames before `codec_eos` | 2 |

### Prompt layout

The notation is `P(ids)` for `text_projection(text_embedding(ids))`, `C(id)` for the talker codec embedding and `F(frame)` for `C(code_0) + E_0(code_1) + ... + E_14(code_15)`, where `E_i` are the code predictor embeddings. `S` is the speaker embedding of the voice prompt.

1. The frontend tokenizes the target text as `<|im_start|>assistant\n{text}<|im_end|>\n<|im_start|>assistant\n`. The role ids are the first 3 ids. The text ids are the ids from index 3 to the length minus 5.
2. The voice tool tokenizes the reference transcript as `<|im_start|>assistant\n{transcript}<|im_end|>\n`. The reference ids are the ids from index 3 to the length minus 2.
3. Prefix part A is `P(role ids)`, 3 positions.
4. Prefix part B is 6 positions. The codec track is `C(2154), C(2156), C(language), C(2157), S, C(2148)`. The text track is `P(tts_pad)` 5 times, then `P(tts_bos)`. Each position is the sum of the two tracks.
5. Prefix part C is the in-context part. The text track `T` is `P(reference ids + text ids)` followed by `P(tts_eos)`. The codec track `K` is `C(2149)` followed by `F(frame)` for each reference frame. If `T` is longer than `K`, part C is `T[..len(K)] + K` and the trailing text is `T[len(K)..]`. In the other case, the prompt builder pads `T` with `P(tts_pad)` to the length of `K`. Then part C is the sum, and the trailing text is empty.
6. At generation step `n`, the talker input is `F(previous frame)` plus trailing text position `n`. After the last trailing position, it is `F(previous frame) + P(tts_pad)`.
7. The code predictor starts a new cache for each frame. Its prefill is `[talker last hidden state, C(code_0)]` through the input projection, and head 0 gives `code_1`. For step `s` from 1 to 14, the input is `E_(s-1)(code_s)` through the input projection, and head `s` gives `code_(s+1)`.
8. Generation stops when codebook 0 is `codec_eos`. Antenna does not decode the `codec_eos` frame.

## Read first

1. `CLAUDE.md`
2. `docs/architecture.md` sections 4.2, 4.3, 5.2, 5.3, 7.4, 9 and 11.1
3. `docs/standards.md` sections 3.4, 5, 7, 8, 14, 18 and 19
4. `docs/plans/08-ml-and-review.md` for `antenna-ml`, and `docs/plans/10-qwen3-decoder.md`
5. The reference functions `Qwen3TTSForConditionalGeneration.generate`, `generate_icl_prompt` and `Qwen3TTSTalkerForConditionalGeneration.forward`

## Scope

The stage can create or change these files only.

- `crates/engines/qwen3/Cargo.toml`, `crates/engines/qwen3/src/lib.rs`, `crates/engines/qwen3/src/error.rs` (new variants only)
- `crates/engines/qwen3/src/config/talker.rs`, `crates/engines/qwen3/src/config/mod.rs`
- `crates/engines/qwen3/src/frontend/`
- `crates/engines/qwen3/src/model/`
- `crates/engines/qwen3/voices/`
- New files in `crates/engines/qwen3/tests/fixtures/`, which are `tokens.json`, `talker_*`, `code_predictor_*`, `prompt_*` and `generation_*`
- `crates/engines/qwen3/benches/talker.rs`
- `crates/engines/qwen3/Cargo.toml` (the `tokenizers` dependency)
- `tools/reference/qwen3/dump_talker.py`
- `tools/voices/qwen3/`
- `docs/benchmarks.md` (rows for the talker)

## Deliverables

### Voice prompt tool

```text
tools/voices/qwen3/
├── pyproject.toml   # same pins as tools/reference/qwen3
├── uv.lock
├── voices.toml      # one entry for each voice: language, key, archetype, seed
└── make_voices.py   # writes crates/engines/qwen3/voices/<language>-<key>.safetensors
```

1. `make_voices.py` uses `Qwen/Qwen3-TTS-12Hz-1.7B-VoiceDesign` at revision `5ecdb67327fd37bb2e042aab12ff7391903235d3` to make one reference clip for each voice with `generate_voice_design`.
2. It encodes the clip with `create_voice_clone_prompt(x_vector_only_mode=False)` of the 1.7B Base model, and gets the 0.6B speaker embedding with `extract_speaker_embedding` of the 0.6B Base model.
3. It exits with an error if the clip is shorter than 6 s or longer than 14 s. It also exits with an error if the reference codes of the two Base models are different.
4. It writes the clip to `tools/voices/qwen3/out/<language>-<key>.wav`. This directory is in `.gitignore`.

| Tensor in `<language>-<key>.safetensors` | Shape and dtype |
|---|---|
| `ref_codes` | `[frames, 16]`, U32 |
| `ref_text_ids` | `[ids]`, U32 (the reference ids of the prompt layout, step 2) |
| `speaker_embedding_0b6` | `[1024]`, F32 |
| `speaker_embedding_1b7` | `[2048]`, F32 |

The safetensors metadata contains `language`, `key`, `archetype`, `transcript`, `instruct`, `seed` and the three model revisions. One file serves both model sizes, because the two Base models share the speech tokenizer. Only the speaker embedding is different for each size. One file for each voice is smaller in total than one file for each voice and size, because the reference codes occur one time. A file with 14 s of reference codes and both embeddings is smaller than 32 KB.

Each voice has one of four archetypes. The instruct of an archetype is the same in all languages, with the accent at the end.

| Archetype | Instruct |
|---|---|
| `warm_female` | A warm adult female narrator with a mid register and a calm, unhurried pace, recorded close to the microphone in a quiet studio, with a {accent} accent. |
| `bright_female` | A bright young adult female voice with a higher register and a close, friendly delivery, recorded in a quiet studio, with a {accent} accent. |
| `clear_male` | A clear adult male voice with a mid register and an agile, lively pace, recorded in a quiet studio, with a {accent} accent. |
| `deep_male` | A firm adult male narrator with a low register and a steady storytelling pace, recorded close to the microphone in a quiet studio, with a {accent} accent. |

The voices have these keys. The key is the ASCII lowercase form of the display name. Stage 12 gives the display names and the localized descriptions.

| Language | `warm_female` | `bright_female` | `clear_male` | `deep_male` |
|---|---|---|---|---|
| en | `ava` | `grace` | `owen` | `henry` |
| es | `lucia` | `ines` | `mateo` | `bruno` |
| pt | `beatriz` | `helena` | `rafael` | `tiago` |
| fr | `camille` | `juliette` | `louis` | `hugo` |
| it | `giulia` | `chiara` | `marco` | `luca` |
| de | `lena` | `clara` | `jonas` | `felix` |

The accents are General American (en), neutral Latin American (es), Brazilian (pt), Parisian (fr), standard Italian (it) and standard German (de). All four voices of a language use the same reference text.

| Language | Reference text |
|---|---|
| en | The morning light came slowly over the hills, and the small town woke up to the sound of rain on the old stone roofs. |
| es | La luz de la mañana llegó despacio sobre las colinas, y el pequeño pueblo despertó con el sonido de la lluvia sobre los viejos techos de piedra. |
| pt | A luz da manhã chegou devagar sobre as colinas, e a pequena cidade acordou com o som da chuva nos velhos telhados de pedra. |
| fr | La lumière du matin arrivait lentement sur les collines, et la petite ville se réveillait au son de la pluie sur les vieux toits de pierre. |
| it | La luce del mattino arrivava lentamente sulle colline, e la piccola città si svegliava al suono della pioggia sui vecchi tetti di pietra. |
| de | Das Morgenlicht kam langsam über die Hügel, und die kleine Stadt erwachte beim Klang des Regens auf den alten Steindächern. |

### Rust crate

```rust
// src/config/talker.rs
pub(crate) struct TalkerConfig { /* talker_config and the top-level token ids of config.json */ }
pub(crate) struct CodePredictorConfig { /* code_predictor_config */ }
pub(crate) struct GenerationDefaults { /* generation_config.json */ }
impl GenerationDefaults {
    pub(crate) fn sampling(&self) -> FrameSampling;           // the random sampling of the reference
    pub(crate) fn greedy(&self) -> FrameSampling;             // temperature 0, the same penalty and window
}
impl TalkerConfig { pub(crate) fn from_json(bytes: &[u8]) -> Result<Self, Qwen3Error>; pub(crate) fn language_id(&self, language: Language) -> u32; }

// src/error.rs (new variants of the Qwen3Error of stage 10)
#[error("cannot load the tokenizer")] Tokenizer(#[source] tokenizers::Error),
#[error("talker hidden size {size} is not 1024 or 2048")] HiddenSize { size: usize },
#[error("voice prompt {voice} has no tensor {tensor}")] VoicePrompt { voice: String, tensor: &'static str },

// src/frontend/mod.rs
pub(crate) struct Frontend { /* tokenizers::Tokenizer */ }
impl Frontend {
    pub(crate) fn load(files: &ModelFiles) -> Result<Self, Qwen3Error>;     // vocab, merges, tokenizer-config
    pub(crate) fn encode(&self, text: &str) -> Result<PromptTokens, Qwen3Error>;
}
pub(crate) struct PromptTokens { pub(crate) role: [u32; 3], pub(crate) text: Vec<u32> }

// src/frontend/voice.rs
pub(crate) struct VoicePrompt { /* language, ref_codes [frames, 16], ref_text_ids, speaker embedding of one size */ }
impl VoicePrompt {
    pub(crate) fn embedded(voice: &VoiceId, size: ModelSize, device: &Device) -> Result<Self, Qwen3Error>;
    pub(crate) fn language(&self) -> Language;
}

// src/model/mod.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ModelSize { Small, Large }        // 0.6B and 1.7B

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct FrameSampling { pub(crate) codebook_zero: SamplingConfig, pub(crate) other_codebooks: SamplingConfig }

pub(crate) struct Talker { /* config, talker, code predictor, prompt builder, KV caches */ }
impl Talker {
    pub(crate) fn load(files: &ModelFiles, device: &Device, dtype: DType) -> Result<Self, Qwen3Error>;   // talker-config, generation-config, talker-weights
    pub(crate) fn size(&self) -> ModelSize;
    pub(crate) fn defaults(&self) -> &GenerationDefaults;
    pub(crate) fn generate(&mut self, settings: &GenerationSettings, tokens: &PromptTokens, seed: Seed) -> Result<StepStream<'_>, Qwen3Error>;
}

// src/model/steps.rs
#[derive(Clone)]
pub(crate) struct GenerationSettings { pub(crate) voice: VoicePrompt, pub(crate) sampling: FrameSampling }

pub(crate) struct StepStream<'a> { /* borrows the talker and owns the sampler state of one segment */ }
impl Iterator for StepStream<'_> { type Item = Result<Frame, Qwen3Error>; }   // one step makes one frame
```

1. `SamplingConfig` and `Seed` come from `antenna-ml`, and `Frame` comes from `src/frame.rs` of stage 10. `FrameSampling` holds the two sampling configurations of the reference. It does not repeat a field of `SamplingConfig`.
2. `StepStream` makes one `antenna_ml::Sampler` for codebook 0 and one for codebooks 1 to 15, both with `seed`.
3. `GenerationDefaults::greedy` returns the configurations of `sampling` with `temperature` 0.0. The repetition penalty and its window stay, so the greedy parity test also proves the penalty rule.
4. The `repetition_window` of codebook 0 is 4096, the capacity of the KV cache. Thus the penalty covers all codebook 0 ids of the segment. Codebooks 1 to 15 have a penalty of 1.0.
5. The frontend builds the tokenizer with the `tokenizers` crate from `vocab.json` and `merges.txt`, with the Qwen2 pre-tokenizer. The pre-tokenizer does NFC normalization, a split with the Qwen2 regex and byte-level BPE. The frontend reads the added tokens from `added_tokens_decoder` in `tokenizer_config.json`.
6. `Talker::size` comes from the hidden size of the talker configuration. A hidden size other than 1024 or 2048 gives `Qwen3Error::HiddenSize` at load.
7. `StepStream` ends after the `codec_eos` frame, without that frame. It does not stop at a maximum length. Stage 12 runs it through `antenna_ml::pace`, which adds the runaway guard.
8. The KV cache of the talker is a preallocated `candle_nn::kv_cache::KvCache` with room for 4096 positions. `generate` resets it. The talker state is never cloned, so a cache that writes in place is correct here.
9. `Talker::generate` and `MagpieModel::generate` of stage 13 have the same shape. The arguments are the settings of the voice, the tokens and the seed, in this sequence. `GenerationSettings` of each engine holds the voice input and the sampling of the variant. A `VoicePrompt` holds tensors, so the qwen3 settings are `Clone` and not `Copy`.

### Module files

| File | Content |
|---|---|
| `src/frontend/mod.rs` | `Frontend`, `PromptTokens`, the template |
| `src/frontend/tokenizer.rs` | Tokenizer construction |
| `src/frontend/voice.rs` | `VoicePrompt` and the 24 `include_bytes!` files |
| `src/model/mod.rs` | `Talker`, `ModelSize`, `FrameSampling`, module declarations |
| `src/model/transformer.rs` | Qwen3 block and layer stack with KV cache, used by the talker and the code predictor |
| `src/model/talker.rs` | Embeddings, text projection, layers, codec head |
| `src/model/code_predictor.rs` | Input projection, layers, 15 embeddings, 15 heads, the 15-step frame loop |
| `src/model/prompt.rs` | The prompt layout, from tokens and a voice prompt to prefix and trailing embeddings |
| `src/model/steps.rs` | `GenerationSettings`, `StepStream`, logit processing for codebook 0 |
| `src/model/parity.rs` | `#[cfg(test)]` parity tests |

## Tasks

1. Make the voice prompt tool and run it. Commit the 24 prompt files. Give the 24 clips in `tools/voices/qwen3/out/` to the project owner for AC-11-02.
2. Write `dump_talker.py` in `tools/reference/qwen3`. It loads the Base models in f32 on CPU with eager attention and writes the fixtures in the table below. The greedy generation uses `do_sample=False` and `subtalker_dosample=False`, and keeps `repetition_penalty=1.05`.
3. Run the script and commit the fixtures. Each fixture is smaller than 2 MB.

The parity sentence is the text below. It has 26 words, so its greedy generation has more than 50 frames.

```text
The old lighthouse keeper climbed the narrow stairs every evening, lit the great lamp, and watched the ships pass safely through the dark and stormy water.
```

| Fixture | Content |
|---|---|
| `tokens.json` | For 12 strings (2 for each language, with accents, digits and punctuation), the ids of the target template |
| `talker_projection_<size>.safetensors` | 8 text ids and the text projection output |
| `talker_layer_<size>.safetensors` | A random input `[8, hidden]` at positions 0 to 7 and the output of talker layer 0 |
| `code_predictor_<size>.safetensors` | A random talker hidden state, `code_0`, the 15 greedy codes and the logits of head 0 |
| `prompt_<size>.safetensors` | The prefix and trailing embeddings for the parity sentence with the `en-ava` voice prompt |
| `generation_<size>.safetensors` | The first 50 greedy frames `[50, 16]` of the parity sentence, and the codebook 0 logits of the first step |

4. Add `tokenizers` to the normal dependencies. Write `TalkerConfig`, `CodePredictorConfig` and `GenerationDefaults` with tests that parse the real files of both sizes.
5. Write the frontend. Make `tokens.json` pass.
6. Write `VoicePrompt`.
7. Write the Qwen3 block in `model/transformer.rs`. Make the talker layer parity test pass.
8. Write the talker and the text projection. Make the projection parity test pass.
9. Write the code predictor. Make its parity test pass.
10. Write the prompt builder. Make the prompt parity test pass.
11. Write `GenerationSettings` and `StepStream`. Make the greedy generation parity test pass for both sizes with `GenerationDefaults::greedy`.
12. Write `benches/talker.rs`. Measure the prefill of the parity prompt on Metal in bf16, for both sizes.
13. In the same bench, measure the time of one frame, which is one talker step and 15 code predictor steps. Add the medians to the module benchmarks table of `docs/benchmarks.md`.
14. Run `cargo xtask check` and do the quality gate.

## Acceptance criteria

All parity tests run on CPU in f32 and have the attribute `#[ignore = "needs models"]`. Each parity module gets its files with one call, `ModelStore::open_default()?.ensure(LARGE_ARTIFACTS, &|_| {}, &AtomicBool::default())?`, or the same call with `SMALL_ARTIFACTS` of stage 10. In a test name, `<size>` is `small` for 0.6B and `large` for 1.7B. The nightly job of stage 09 skips the tests whose names end in `_when_large`.

| ID | Criterion | Check |
|---|---|---|
| AC-11-01 | 24 voice prompt files exist, four for each language, each smaller than 32 KB | Commands 1 and 2 below the table |
| AC-11-02 | Each reference clip sounds like its archetype, and the four voices of a language are easy to tell apart **(manual)** | The project owner listens and writes "approved" in the stage report |
| AC-11-03 | The two Base models give equal reference codes | `make_voices.py` exits with code 0 |
| AC-11-04 | Both config files of both sizes parse | Tests `talker_config_parses_when_<size>` |
| AC-11-05 | The tokenizer gives the reference ids for all 12 strings | Test `tokenizer_matches_reference_ids` |
| AC-11-06 | The text projection matches the reference | Tests `text_projection_matches_reference_when_<size>`, 1e-4 |
| AC-11-07 | Talker layer 0 matches the reference | Tests `talker_layer_matches_reference_when_<size>`, 1e-4 |
| AC-11-08 | The code predictor gives the reference codes and logits | Tests `code_predictor_matches_reference_when_<size>`, exact codes, logits 1e-4 |
| AC-11-09 | The prompt embeddings match the reference | Tests `prompt_matches_reference_when_<size>`, 1e-4 |
| AC-11-10 | The first 50 greedy frames are equal to the reference, with the repetition penalty on | Tests `greedy_frames_match_reference_when_<size>` with `GenerationDefaults::greedy`, exact match of all 50 frames |
| AC-11-11 | The same seed gives the same frames, and a different seed gives different frames | Tests `random_sampling_is_deterministic_when_seed_is_equal` and `random_sampling_changes_when_seed_changes` |
| AC-11-12 | Codebook 0 never takes a suppressed id | Test `codebook_zero_skips_suppressed_ids` |
| AC-11-13 | The median frame time on Metal in bf16 is 32 ms or less for 1.7B and 56 ms or less for 0.6B, on the Apple M5 Pro 24 GB **(manual)** | `cargo bench -p antenna-engine-qwen3 --bench talker`, rows in the module benchmarks table of `docs/benchmarks.md` |
| AC-11-14 | No source file has more than 300 lines | `cargo xtask lint-repo` |
| AC-11-15 | The crate defines no sampling type other than `FrameSampling` | `rg -n "enum Sampling" crates/engines/qwen3/src` prints nothing |
| AC-11-16 | The new fixtures exist and each is smaller than 2 MB | `find crates/engines/qwen3/tests/fixtures -size +2M` prints nothing |
| AC-11-17 | `uv run dump_talker.py` makes the same fixtures again, bit for bit | Commands 3 and 4 below the table |
| AC-11-18 | A bad hidden size gives `Qwen3Error::HiddenSize` | Test `talker_config_fails_when_hidden_size_is_unknown` |

The commands of AC-11-01 and AC-11-17 are these.

1. `find crates/engines/qwen3/voices -name '*.safetensors'` prints 24 lines.
2. `find crates/engines/qwen3/voices -size +32k` prints nothing.
3. In `tools/reference/qwen3`, run `uv sync --locked` and `uv run dump_talker.py`.
4. Run `git diff --exit-code crates/engines/qwen3/tests/fixtures`. The command exits with code 0.

The limits of AC-11-13 come from the real-time factor budgets of `docs/architecture.md` section 8. One frame is 80 ms of audio. A real-time factor of 0.5 gives 40 ms for each frame, and 0.8 gives 64 ms. The decoder uses 7.5 ms of each frame (AC-10-13), so the talker gets the rest, rounded down.

## Decision rules

1. If a parity test fails, follow `docs/architecture.md` section 14, rule 1. Compare the prompt embeddings first, then layer 0, then the last hidden state, then the logits.
2. If the reference generation of the parity sentence has a tie in its first 50 frames, change the sentence in `dump_talker.py`. Then run the script again. A tie is a difference smaller than 1e-4 between the two best codebook 0 logits. Write the old sentence and the tie frame in the stage report. Do not change the exact match of AC-11-10.
3. Use `antenna_ml::Sampler` for random sampling. Mask the suppressed ids in the logits before the sampler call. If the sampler cannot express a rule of the reference, write a "Blocked" section, because the fix changes stage 08.
4. Put the repetition penalty only on codebook 0, over the codebook 0 ids of the frames that this segment generated. The reference applies it through Hugging Face `generate`. AC-11-10 runs with the penalty on, so it proves the exact rule.
5. Use `candle_nn::rotary_emb::rope` and `candle_nn::ops::rms_norm`. Do not write a custom kernel.
6. If the code predictor loop is the largest cost in the frame benchmark, keep its 15 steps on the device. Copy only the 15 selected ids to the host after each frame. Greedy selection with `argmax` on the device makes this possible. Random sampling needs one host copy for each step.
7. If AC-11-13 fails after decision rule 6, write a "Blocked" section with the time of each part of a frame. Stage 12 decision rule 1 has the next steps, and a human decides if this stage can merge first.
8. If the voice design model makes a clip outside 6 s to 14 s, change the seed in `voices.toml` and run the tool again. Do not change the text.

### Facts to check

1. The id of `assistant` and the slice indices of the templates. Check with `tokens.json` from the reference tokenizer.
2. The set of ids that the reference repetition penalty uses. Check with AC-11-10, because the greedy fixture keeps the penalty and the parity sentence has more than 50 frames.
3. That `extract_speaker_embedding` of the 0.6B Base model returns 1024 values. Check in `make_voices.py`.

## Out of scope

1. `Engine`, `EngineFactory`, the descriptor value and the runaway guard. Stage 12 adds them.
2. The x-vector-only voice mode. Antenna uses the in-context mode only.
3. Voice cloning from user audio.
