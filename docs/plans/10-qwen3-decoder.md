# Stage 10. Qwen3 speech decoder

## Goal

A streaming decoder in `antenna-engine-qwen3` converts Qwen3-TTS 12Hz codes into 24 kHz PCM in the parity tolerances of `docs/architecture.md` section 11.1.

## Context

Qwen3-TTS has two parts. The talker makes codes (stage 11). The speech tokenizer decoder converts the codes into audio (this stage). The decoder is the last step before the sink, so its latency adds directly to the time to first audio.

This stage is the first stage of the `qwen3` engine. It makes the crate, the frame type and all artifact constants of the engine. Stage 11 starts after this stage, because it adds modules to the same crate and uses these constants.

The decoder in the reference implementation is fully causal. Each convolution pads only on the left, each transposed convolution trims only on the right, and the attention is causal with a sliding window. Thus a stateful decoder that receives frames one by one gives the same samples as a decoder that receives all frames at one time. This stage implements one forward path, the streaming path. A one-shot decode is a stream that receives all frames in one call.

The causal convolutions, their states and the `Streaming` trait come from `antenna_ml::stream` (stage 08). The Magpie codec of stage 14 uses the same parts.

### Facts from the reference

The source is `speech_tokenizer/config.json` at revision `fd4b254389122332181a7c3db7f27e918eec64e3` of `Qwen/Qwen3-TTS-12Hz-1.7B-Base`, and `qwen_tts/core/tokenizer_12hz/modeling_qwen3_tts_tokenizer_v2.py` at commit `022e286b98fbec7e1e916cb940cdf532cd9f488e` of `QwenLM/Qwen3-TTS` (Apache-2.0). The 0.6B repository has the same file, with the same SHA-256.

| Item | Value |
|---|---|
| Frame rate | 12.5 Hz (24000 / 1920) |
| Output sample rate | 24000 Hz |
| Samples for each frame | 1920 (`decode_upsample_rate`) |
| Codebooks for each frame | 16 (1 semantic, 15 acoustic), 2048 entries each |
| Quantizer | Split RVQ. Each codebook entry is `embedding_sum / max(cluster_usage, 1e-5)`, dimension 256. A 1x1 output projection maps 256 to 512 for each of the two groups. The two group outputs are added |
| `pre_conv` | Causal conv 512 to 1024, kernel 3 |
| `pre_transformer` | Input projection 1024 to 512, 8 layers, hidden 512, 16 heads of 64 (attention width 1024), MLP 1024 (SiLU gate), RMSNorm eps 1e-5, RoPE theta 10000, causal sliding window of 72 frames, layer scale on the attention and MLP outputs, final RMSNorm, output projection 512 to 1024. No QK norm |
| Upsampler | 2 stages. Each stage is a transposed conv 1024 to 1024 (kernel 2, stride 2) and a ConvNeXt block (causal depthwise conv kernel 7, LayerNorm eps 1e-6, MLP 4096 with exact GELU, `gamma` scale) |
| Decoder | Causal conv 1024 to 1536 (kernel 7), then 4 blocks with strides 8, 5, 4 and 3 and widths 1536, 768, 384, 192, 96. Each block is SnakeBeta, a transposed conv (kernel 2 × stride), and 3 residual units with dilations 1, 3 and 9. Then SnakeBeta, a causal conv 96 to 1 (kernel 7) and a clamp to [-1, 1] |
| Residual unit | SnakeBeta, causal conv kernel 7 with the dilation, SnakeBeta, causal conv kernel 1, plus the input |
| SnakeBeta | `x + sin²(x × exp(alpha)) / (exp(beta) + 1e-9)`, with one `alpha` and one `beta` value for each channel |
| Weights | `speech_tokenizer/model.safetensors`, all F32. Decoder tensors have the prefix `decoder.` |

### Artifacts

All artifacts come from Hugging Face. The tokenizer and codec artifacts come from `Qwen/Qwen3-TTS-12Hz-1.7B-Base` at revision `fd4b254389122332181a7c3db7f27e918eec64e3`. The 0.6B repository has the same five files with the same SHA-256 values, so all variants use the 1.7B copies.

| Tokenizer and codec artifact | Constant | Key | Bytes | SHA-256 |
|---|---|---|---|---|
| `vocab.json` | `VOCAB` | `vocab` | 2776833 | `ca10d7e9fb3ed18575dd1e277a2579c16d108e32f27439684afa0e10b1440910` |
| `merges.txt` | `MERGES` | `merges` | 1671839 | `599bab54075088774b1733fde865d5bd747cbcc7a547c5bc12610e874e26f5e3` |
| `tokenizer_config.json` | `TOKENIZER_CONFIG` | `tokenizer-config` | 7344 | `dc3c31c3bdaedd5016382bb3cbe07323026775ad51f5a4fb564505992ae4a670` |
| `speech_tokenizer/config.json` | `CODEC_CONFIG` | `codec-config` | 2336 | `ee65bb901c876664ab8707c487157aa1a6ee57c65969b28fb5ec9dc211e68167` |
| `speech_tokenizer/model.safetensors` | `CODEC_WEIGHTS` | `codec-weights` | 682293092 | `836b7b357f5ea43e889936a3709af68dfe3751881acefe4ecf0dbd30ba571258` |

| 1.7B talker artifact (`Qwen/Qwen3-TTS-12Hz-1.7B-Base` at `fd4b254389122332181a7c3db7f27e918eec64e3`) | Constant | Key | Bytes | SHA-256 |
|---|---|---|---|---|
| `config.json` | `TALKER_CONFIG_1B7` | `talker-config` | 4494 | `b4f01752d15a488abde3e1ab44723ae4f4b9e68a4037257b098b3737893cc1f9` |
| `generation_config.json` | `GENERATION_CONFIG_1B7` | `generation-config` | 245 | `f1b90b4513f3b34c62851049e2492d7b4c5940daf1276f89c82b8ef04127f3aa` |
| `model.safetensors` | `TALKER_WEIGHTS_1B7` | `talker-weights` | 3857413744 | `38fc7fc51c5e776e840414b6fd443962e9411b9654888fd7913e4da643cb857c` |

| 0.6B talker artifact (`Qwen/Qwen3-TTS-12Hz-0.6B-Base` at `5d83992436eae1d760afd27aff78a71d676296fc`) | Constant | Key | Bytes | SHA-256 |
|---|---|---|---|---|
| `config.json` | `TALKER_CONFIG_0B6` | `talker-config` | 4494 | `2e714c787c8edb98b05432685cddb634add2de4d4e645f653d68251ef72ba011` |
| `generation_config.json` | `GENERATION_CONFIG_0B6` | `generation-config` | 245 | `f1b90b4513f3b34c62851049e2492d7b4c5940daf1276f89c82b8ef04127f3aa` |
| `model.safetensors` | `TALKER_WEIGHTS_0B6` | `talker-weights` | 1829344272 | `180b3b10eb1c9f1b4db7806d5475bae3071c0243c299d49926bab1da3b6946f6` |

1. Each artifact has the extent `Extent::Whole` with the bytes of the table.
2. `src/descriptor/artifacts.rs` declares each artifact one time as a `pub(crate) const` with the name of the table.
3. The same file declares two slices. `SMALL_ARTIFACTS` has the five tokenizer and codec artifacts and the three 0.6B talker artifacts. `LARGE_ARTIFACTS` has the five tokenizer and codec artifacts and the three 1.7B talker artifacts.
4. Stage 12 builds the variants of the descriptor from these slices. No other file contains a revision, a size or a hash.

## Read first

1. `CLAUDE.md`
2. `docs/architecture.md` sections 4, 7.4, 8 and 11.1
3. `docs/standards.md` sections 3.4, 5, 8, 14, 18 and 19
4. `docs/plans/08-ml-and-review.md`, for the `antenna-ml` API
5. The reference file `modeling_qwen3_tts_tokenizer_v2.py`, classes `Qwen3TTSTokenizerV2Decoder` and the classes that it uses

## Scope

The stage can create or change these files only.

- `crates/engines/qwen3/Cargo.toml`, `crates/engines/qwen3/src/lib.rs`, `crates/engines/qwen3/src/frame.rs`, `crates/engines/qwen3/src/error.rs`
- `crates/engines/qwen3/src/descriptor/mod.rs`, `crates/engines/qwen3/src/descriptor/artifacts.rs`
- `crates/engines/qwen3/src/config/mod.rs`, `crates/engines/qwen3/src/config/decoder.rs`
- `crates/engines/qwen3/src/codec/`
- `crates/engines/qwen3/tests/fixtures/codec_*.safetensors`
- `crates/engines/qwen3/benches/codec.rs`
- `tools/reference/qwen3/`
- The root `Cargo.toml` (the new member, and the missing `[workspace.dependencies]` entries of task 1)
- The root `.gitignore` (the `.venv` directory of `tools/`)
- `docs/benchmarks.md` (rows of the module benchmarks table for the decoder)

## Deliverables

### Python reference project

```text
tools/reference/qwen3/
├── pyproject.toml     # uv project, Python 3.12, exact pins
├── uv.lock
├── common.py          # model loading at the pinned revisions, CPU, float32, eager attention
└── dump_decoder.py    # writes crates/engines/qwen3/tests/fixtures/codec_*.safetensors
```

1. Pin `qwen-tts==0.1.1`, `transformers==4.57.3`, `torch`, `safetensors` and `numpy` with exact versions. Commit `uv.lock`.
2. `common.py` loads the models from the Hugging Face cache at the pinned revisions only.
3. `uv run dump_decoder.py` makes the fixtures in the table below. Each fixture is smaller than 2 MB. The script uses `torch.manual_seed(0)` for all random inputs.

| Fixture | Content |
|---|---|
| `codec_quantizer.safetensors` | Random codes `[16, 8]`, the quantizer output `[512, 8]` |
| `codec_pre_conv.safetensors` | Random input `[512, 8]`, output `[1024, 8]` |
| `codec_transformer.safetensors` | Random input `[96, 1024]`, output `[96, 1024]`. The length 96 is longer than the window of 72 |
| `codec_upsample.safetensors` | Random input `[1024, 8]`, output of the two upsample stages `[1024, 32]` |
| `codec_block_<i>.safetensors` | For each decoder block `i` in 0 to 3, a random input with 8 time steps and its output |
| `codec_waveform.safetensors` | Random codes `[16, 24]`, the full decoder output `[46080]` |
| `codec_primed.safetensors` | Codes `ref` `[16, 20]` and `gen` `[16, 12]`, and the output of the decoder on `ref` followed by `gen`, without the first `20 × 1920` samples |

The full decoder output is the output of `Qwen3TTSTokenizerV2Decoder.forward` on all frames at one time. Do not use `chunked_decode` for fixtures. `chunked_decode` recomputes 25 frames of context after each 300 frames, so it is an approximation of the full forward pass.

### Rust crate

```rust
// src/frame.rs
pub(crate) const CODEBOOKS: usize = 16;
pub(crate) type Frame = [u32; CODEBOOKS];

// src/error.rs
#[derive(Debug, thiserror::Error)]
pub(crate) enum Qwen3Error {
    #[error(transparent)] Ml(#[from] MlError),
    #[error(transparent)] Candle(#[from] candle_core::Error),
    #[error(transparent)] Files(#[from] EngineError),          // EngineError::MissingFile from ModelFiles::path
    #[error("cannot parse {file}")] Config { file: &'static str, #[source] source: serde_json::Error },
}
impl Qwen3Error {
    pub(crate) fn into_load(self) -> EngineError;
    pub(crate) fn into_inference(self) -> EngineError;
}

// src/config/decoder.rs
#[derive(Deserialize)]
pub(crate) struct DecoderConfig { /* the fields of decoder_config that the decoder uses */ }
impl DecoderConfig {
    pub(crate) fn from_json(bytes: &[u8]) -> Result<Self, Qwen3Error>;
    pub(crate) fn samples_per_frame(&self) -> usize;   // product of upsampling_ratios and upsample_rates
}

// src/codec/mod.rs
pub(crate) struct Codec { /* all decoder modules */ }
impl Codec {
    pub(crate) fn load(files: &ModelFiles, device: &Device) -> Result<Self, Qwen3Error>;
    pub(crate) fn start(&self) -> CodecState;
    pub(crate) fn decode(&self, state: &mut CodecState, frames: &[Frame]) -> Result<PcmChunk, Qwen3Error>;
}

#[derive(Clone)]
pub(crate) struct CodecState { /* one state value for each stateful module, and the frame position */ }
```

1. Each function of the crate returns `Qwen3Error`, so an error keeps its data. Stage 11 adds the variants of the talker and the frontend. `antenna_ml::pace` of stage 12 uses `Qwen3Error` as its error type. `into_load` and `into_inference` give `Files` unchanged and put each other variant in a box in `EngineError::Load` or `EngineError::Inference`. Stage 12 calls `into_load` one time in `Factory::load` and `into_inference` one time in `synthesize`. The Magpie crate has the same file with `MagpieError`.
2. `Codec::load` reads `DecoderConfig` from the `codec-config` file and loads the weights from the `codec-weights` file with `antenna_ml::load_safetensors`. The Magpie codec of stage 14 has the same three methods with the same argument types.
3. `decode` converts the frames to a tensor with `antenna_ml::stream::codes_tensor` and returns exactly `frames.len() × 1920` samples.
4. A state update makes new tensors and never writes into an existing tensor, as stage 08 defines for `Streaming`. Thus a clone of `CodecState` copies no tensor data, and the decode of a clone does not change the original. Stage 12 uses a clone to start each segment from a state that already contains the voice prompt.
5. The causal conv and the causal transposed conv are `antenna_ml::stream::CausalConv1d` and `antenna_ml::stream::CausalConvTranspose1d`. This crate does not contain a second convolution with state.
6. The ConvNeXt block, the residual unit, the decoder block and the sliding-window transformer implement `antenna_ml::stream::Streaming`. There is no second forward path without state.

| Module | State |
|---|---|
| Causal conv | `antenna_ml` `ConvState` |
| Causal transposed conv | `antenna_ml` `ConvTransposeState` |
| Sliding-window transformer | For each layer, the keys and values of the last 72 positions, and the next position. An update concatenates the new keys and values with `Tensor::cat` and keeps the last 72 positions with `narrow`. It does not use `candle_nn::kv_cache::KvCache`, which writes in place |

### Configuration files

An engine has `src/config.rs` when its model has one configuration source. It has `src/config/` with one file for each source when the model has more than one source. Qwen3 has two sources, the codec configuration and the talker configuration, so this crate has `config/mod.rs`, `config/decoder.rs` (this stage) and `config/talker.rs` (stage 11). Magpie reads all values from one GGUF file, so stage 13 writes `config.rs`.

### Module files

| File | Content |
|---|---|
| `src/codec/mod.rs` | `Codec`, `CodecState`, the module declarations |
| `src/codec/quantizer.rs` | Split RVQ decode |
| `src/codec/snake.rs` | SnakeBeta |
| `src/codec/convnext.rs` | ConvNeXt block |
| `src/codec/transformer.rs` | Sliding-window transformer with layer scale |
| `src/codec/decoder.rs` | Residual unit, decoder block, the full decoder stack |
| `src/codec/parity.rs` | `#[cfg(test)]` parity tests against the fixtures |

## Tasks

1. Add `crates/engines/qwen3` to the workspace with the layout of `docs/standards.md` section 3.4. The normal dependencies are `antenna-core`, `antenna-ml`, `candle-core`, `candle-nn`, `serde`, `serde_json`, `thiserror` and `tracing`. The dev-dependencies are `antenna-models` and `divan`.
2. In `lib.rs`, declare `frame`, `error`, `descriptor`, `config` and `codec` with `#[expect(dead_code, reason = "stage 12 connects these modules to the engine")]`.
3. Write `descriptor/mod.rs` with the declaration of `artifacts` only. Write `descriptor/artifacts.rs` with the constants and the slices of the "Artifacts" section.
4. Make the uv project in `tools/reference/qwen3`. Run `uv lock` and commit the lock file.
5. Write `dump_decoder.py` and run it. Commit the fixtures.
6. Write `DecoderConfig` and a unit test that parses the real `speech_tokenizer/config.json` text.
7. Write the quantizer. Make its parity test pass.
8. Write SnakeBeta and the ConvNeXt block. Make the upsample parity test pass.
9. Write the sliding-window transformer. Make its parity test pass.
10. Write the residual unit and the decoder blocks. Make the four block parity tests pass.
11. Write `Codec`. Make the waveform and primed parity tests pass.
12. Write the split tests. For a fixed sequence of 40 frames, decode it in one call, then in calls of 1, 2, 3, 4 and 7 frames. Compare each result with the one-call result.
13. Write `benches/codec.rs` with `divan`. Measure `decode` with 1 frame and with 4 frames on Metal, from a state that already holds 125 frames.
14. Add the median results to the module benchmarks table of `docs/benchmarks.md`, one row for each case.
15. Run `cargo xtask check` and do the quality gate.

## Acceptance criteria

All parity tests run on CPU in f32 and have the attribute `#[ignore = "needs models"]`. Each parity module gets its files with one call, `ModelStore::open_default()?.ensure([&CODEC_CONFIG, &CODEC_WEIGHTS], &|_| {}, &AtomicBool::default())?`, in a test that returns `Result<(), Box<dyn Error>>`.

| ID | Criterion | Check |
|---|---|---|
| AC-10-01 | The fixtures exist and each is smaller than 2 MB | `find crates/engines/qwen3/tests/fixtures -name 'codec_*' -size +2M` prints nothing |
| AC-10-02 | `uv run dump_decoder.py` makes the same fixtures again, bit for bit | Commands 1 and 2 below the table |
| AC-10-03 | The quantizer output matches the reference | Test `quantizer_matches_reference`, max absolute difference 1e-4 |
| AC-10-04 | The pre-conv output matches the reference | Test `pre_conv_matches_reference`, 1e-4 |
| AC-10-05 | The transformer output matches the reference for 96 positions | Test `transformer_matches_reference_when_longer_than_window`, 1e-4 |
| AC-10-06 | The upsampler output matches the reference | Test `upsampler_matches_reference`, 1e-4 |
| AC-10-07 | Each decoder block matches the reference | Tests `decoder_block_<i>_matches_reference` for `i` in 0 to 3, 1e-4 |
| AC-10-08 | The waveform matches the reference | Test `waveform_matches_reference`, max absolute difference 1e-3 |
| AC-10-09 | A state primed with the reference frames gives the reference continuation | Test `primed_state_matches_reference_continuation`, 1e-3 |
| AC-10-10 | Streaming in any split gives the one-call result | Tests `decode_is_split_invariant_when_chunk_is_<n>` for 1, 2, 3, 4 and 7, max absolute difference 1e-5 |
| AC-10-11 | `decode` returns 1920 samples for each frame | Test `decode_returns_1920_samples_for_each_frame` |
| AC-10-12 | A decode with the original state does not change a clone | Test `cloned_state_is_independent_when_original_decodes`. Decode 20 frames P and clone the state. Decode 12 frames A with the original and 12 frames B with the clone. The clone output equals the output of a new state that decodes P and then B, max absolute difference 1e-5 |
| AC-10-13 | Decoding 4 frames on Metal takes 30 ms or less (median) on the Apple M5 Pro 24 GB **(manual)** | `cargo bench -p antenna-engine-qwen3 --bench codec`, rows in the module benchmarks table of `docs/benchmarks.md` |
| AC-10-14 | No source file has more than 300 lines | `cargo xtask lint-repo` |
| AC-10-15 | The artifact constants have valid values | Test `artifacts_have_valid_values` checks the 40-character revisions, the 64-character hashes, the non-zero sizes and the unique keys in each slice |
| AC-10-16 | The crate has no convolution with state of its own | `rg -n "struct CausalConv" crates/engines/qwen3/src` prints nothing |
| AC-10-17 | An error keeps its variant until the engine boundary | Tests `into_load_keeps_missing_file_when_file_is_absent` and `into_inference_boxes_error_when_candle_fails` |

The commands of AC-10-02 are these.

1. In `tools/reference/qwen3`, run `uv sync --locked` and `uv run dump_decoder.py`.
2. Run `git diff --exit-code crates/engines/qwen3/tests/fixtures`. The command exits with code 0.

## Decision rules

1. If a module parity test fails, compare the input, then each sub-operation, in the same sequence as the reference `forward`. Fix the first operation that differs. Do not increase a tolerance.
2. The reference uses `nn.GELU()` with the exact erf formula. Use the erf GELU of candle, not the tanh approximation.
3. If AC-10-13 fails in f32, do these steps in sequence and stop at the first step that passes.
   1. Remove host and device copies in the decode path. Keep all tensors on the device until the final PCM copy.
   2. Fuse SnakeBeta into one custom op only if `candle` has no combination of existing ops that meets the budget. Write an ADR first.
   3. Run the decoder blocks in f16 on Metal. Keep f32 on CPU. `Codec::load` selects the dtype from `antenna_ml::backend(device)`.
   4. Add a test that compares the f16 waveform with the f32 waveform. The max absolute difference is 1e-2.
4. If `antenna_ml::stream::CausalConvTranspose1d` gives a result that differs from PyTorch, write a "Blocked" section with the module test output. The fix changes stage 08, so a human decides.
5. If the fixture script gives different bytes on two runs, set `torch.use_deterministic_algorithms(True)` and run it on CPU with one thread (`torch.set_num_threads(1)`).

### Facts to check

1. The exact `torch` version that `uv lock` selects with `transformers==4.57.3`. Check with `uv tree`.
2. The weight names of the decoder blocks. Check with the safetensors header of `speech_tokenizer/model.safetensors`. The expected names are `decoder.decoder.<i>.block.<j>.*`, with the SnakeBeta at `j = 0`, the transposed conv at `j = 1` and the residual units at `j = 2` to `4`. The final SnakeBeta is `decoder.decoder.5` and the final conv is `decoder.decoder.6`.

## Out of scope

1. The talker, the code predictor and the text frontend. Stage 11 adds them.
2. `Engine`, `EngineFactory`, the voices and the descriptor value. Stage 12 adds them.
3. The speech tokenizer encoder and the speaker encoder. Antenna never runs them in Rust.
