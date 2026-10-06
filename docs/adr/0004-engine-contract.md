# ADR 0004. Engine contract

## Status

Accepted, 2026-10-06.

## Context

Antenna has two engines at the first release and more engines later. Each engine has its own model files, voices, quality variants and sample rate. The pipeline, the apps and the tests must use all engines in the same way. Stages 02 to 07 cannot change `antenna-core`, so stage 01 must define the complete contract.

## Decision

1. An engine type implements `EngineFactory`. The factory gives a static `EngineDescriptor` and loads an `Engine` for one voice at one `Quality` from `ModelFiles`.
2. The descriptor is data. It declares the id, the license, the sample rate, the maximum segment length, one `Variant` for each quality and the voices. `Variants` has one field for each quality, so `Variants::get` cannot fail.
3. `antenna_core::check_descriptor` finds nine defects. The registry calls it for each factory.
4. `Engine::synthesize` sends each chunk to an `Emit` callback. If the callback returns `ControlFlow::Break`, the engine returns `Ok(())` before it calls the callback again.
5. An engine is deterministic, does no I/O and resets its generation state at the start of each segment.
6. The pipeline opens each `Output` with the sample rate of the engine to get a `Sink`. A sink receives `begin_segment`, `write` and `finish`.
7. The traits return `EngineError` and `SinkError`. An engine boxes its own error into `EngineError::Load` or `EngineError::Inference`.
8. `antenna-engine-testkit` is the conformance suite. Each engine crate runs it with `conformance!` in `tests/conformance.rs`.

The trait signatures of `docs/architecture.md` sections 5.3 and 5.4 compiled as written. Stage 01 made no change to them.

## Consequences

1. The pipeline and the apps contain no code that is specific to one engine. Only the composition root names concrete engines.
2. A new engine follows the steps of `docs/architecture.md` section 7.5. The pipeline and the core do not change.
3. The `fake` engine lets the pipeline and the apps run with no model download.
4. A change to a trait or to the descriptor needs a new ADR, because all engines and all outputs depend on it.
