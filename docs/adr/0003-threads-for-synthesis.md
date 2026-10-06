# ADR 0003. Threads for synthesis

## Status

Accepted, 2026-10-06.

## Context

Synthesis, the playback feed, the review and the downloads are long operations. Synthesis and review keep a CPU core or the GPU busy for seconds. The audio callback of `cpal` runs on a realtime thread and must not wait. The desktop app has the GPUI executor, but the CLI and `antenna-eval` have no executor.

An async runtime gives no benefit for work that uses the CPU or the GPU all the time. It also adds a second model of concurrency to each library crate.

## Decision

1. Each long operation runs on a dedicated `std::thread` worker. The worker starts with `std::thread::Builder` and a name with the pattern `antenna-<role>`.
2. Threads send events and commands through `flume` channels.
3. Samples go to the audio device through an `rtrb` ring buffer. The audio callback does not allocate, lock or do I/O.
4. A thread owns its state. The only lock is the `RwLock` of the library index.
5. Library crates do not use `async`. The GPUI executor receives `flume` messages in the desktop app.

## Consequences

1. Each library crate works in the CLI, in `antenna-eval` and in tests with no runtime.
2. Clippy rejects `std::thread::spawn`, `std::sync::Mutex` and the `std::sync::mpsc` channels. The `disallowed-methods` and `disallowed-types` lists of `.clippy.toml` enforce the rules.
3. A blocked `emit` callback is the backpressure of the outputs. The engine thread waits and does no extra work.
4. Cancellation is an `AtomicBool` that the emit callback reads for each chunk. A job stops in one chunk.
