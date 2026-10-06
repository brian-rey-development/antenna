# ADR 0005. Candle as the single inference runtime

## Status

Accepted, 2026-10-06.

## Context

Antenna runs a TTS model for each engine and Whisper for the pronunciation review. The release target is Apple Silicon, so the runtime must use Metal. Two runtimes in one app need two copies of the shared inference code, two native build paths and two sets of failure modes.

The candidates were `candle`, ONNX Runtime and a C++ runtime through FFI. ONNX Runtime needs exported models and a large native library. A C++ runtime adds `unsafe` FFI code and a second build system.

## Decision

1. All models run on `candle` (`candle-core`, `candle-nn` and `candle-transformers`).
2. Shared inference code is in `antenna-ml`. That is the device selection, the weight loading, the sampler, the seed, the streaming modules and the synthesis driver `pace`.
3. Weights load with memory mapping when the format permits it. Only `crates/inference/ml/src/weights.rs` contains `unsafe` code, and ADR 0009 gives the safety argument.
4. Device selection prefers Metal and falls back to the CPU. `ANTENNA_DEVICE=cpu` forces the CPU.

## Consequences

1. Tensor types stay in the engine crates, `antenna-ml` and `antenna-review`. Other crates do not depend on `candle`.
2. Each engine implements its network in Rust and proves parity with the reference implementation.
3. A second load of a model reads from the page cache, so the time to first audio with a cold engine stays in its budget.
4. Linux and Windows CI compile and test the CPU path. The Metal backend is a target-specific dependency, so `--all-features` builds on all platforms.
