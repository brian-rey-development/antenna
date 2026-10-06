# ADR 0006. In-house engines

## Status

Accepted, 2026-10-06.

## Context

The selected engines are Qwen3-TTS 12Hz and NVIDIA Magpie TTS Multilingual. They have the highest measured quality in the six languages, and their weight licenses permit commercial use. No maintained Rust crate implements these models with local file loading, streaming and the quality standards of this project. The official implementations are in Python, and Antenna does not run Python at runtime.

## Decision

1. Antenna implements both engines in-house, in `crates/engines/qwen3` and `crates/engines/magpie`, on `candle`.
2. The official Python implementations are the reference implementations. A Python project in `tools/reference/<engine>` writes parity fixtures, and the engine crate commits them.
3. Parity tests compare each module, the greedy generation and the waveform with the tolerances of `docs/architecture.md` section 11.1.
4. Community ports are reading material only. Code from a repository with no license file does not go into Antenna.
5. The Qwen3 voices come from stored voice prompts that `tools/voices/qwen3` makes one time. Only the decoder path is in Rust.

## Consequences

1. The project controls the streaming behavior, the memory use and the speed of each engine.
2. Each engine needs more work before the first release than a wrapper crate needs. Stages 10 to 14 contain this work.
3. A defect in an engine shows as a parity failure at the first module that differs.
4. Each new engine follows the same layout and procedure, in `docs/standards.md` section 3.4 and `docs/architecture.md` section 7.5.
