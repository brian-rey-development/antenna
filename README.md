# Antenna

Antenna converts text into podcast audio with text-to-speech models that run on your computer. You paste a text or drop a `.txt` or `.md` file, and Antenna starts to play it while it generates the rest. You can then export the result as an episode file. No text and no audio leave the machine.

> Antenna is in development. This repository contains the architecture, the standards and the implementation plan. The code arrives in 19 stages. The first release target is macOS on Apple Silicon.

## Features of the first release

| Screen | What the user can do |
|---|---|
| Studio | Write or paste a document. Play it while synthesis continues, with a highlight on the current sentence. Click a sentence to play from it. Export to MP3, WAV or Ogg Opus with loudness normalization to -16 LUFS |
| Library | Find each saved document by period, status, language or text. Play a document from the list |
| Voices | Listen to a preview of each voice of a language and select one |
| Models | Download, cancel, remove and select engines. See the disk use, the memory use and the acceleration device |
| Settings | Set the interface language (English or Spanish), the appearance, the default voice and quality, the export defaults, pronunciation review and updates |

Antenna finds the language of each document. It supports English, Spanish, Portuguese, French, Italian and German.

## Engines

Antenna has two speech engines. Both are in-house Rust implementations on [candle](https://github.com/huggingface/candle) and run on Metal.

| | Qwen3-TTS 12Hz | NVIDIA Magpie TTS Multilingual |
|---|---|---|
| Role | Default engine | Second engine |
| Model size | 0.6B (`Fast`), 1.7B (`Balanced`, `Max`) | 357M |
| Voices | Four designed voices for each language | Five speakers, each in all six languages |
| Sample rate | 24 kHz | 22.05 kHz |
| Weight license | Apache-2.0 | NVIDIA Open Model License |

Antenna downloads the weights of a voice the first time you use it. To add an engine, implement the `EngineFactory` and `Engine` traits and pass the conformance suite. The procedure is in [docs/architecture.md](docs/architecture.md) section 7.5.

## Performance budgets

Each budget is an acceptance criterion on an Apple M5 Pro with 24 GB.

| Metric | Budget |
|---|---|
| Time to first audio, engine loaded | 500 ms (`qwen3`), 300 ms (`magpie`) |
| Real-time factor of the default voices | 0.5 or less |
| Seek to a stored position | 100 ms or less |
| Export of 10 minutes to MP3 with normalization | 3 s or less |
| App start with 1000 documents | 300 ms or less |
| CPU use when nothing plays | 0% |

The full list is in [docs/architecture.md](docs/architecture.md) section 8.

## Repository

```
apps/      the desktop app and the CLI
crates/    the libraries: core contracts, text, audio, pipeline, ui, storage, inference, engines
tools/     developer tools that Antenna does not ship: xtask, eval, reference fixtures
docs/      architecture, standards, design, plans and stage reports
```

| Document | Content |
|---|---|
| [docs/architecture.md](docs/architecture.md) | The crates, the contracts, the data flow and the budgets |
| [docs/standards.md](docs/standards.md) | The code rules, each with the check that enforces it |
| [docs/writing.md](docs/writing.md) | The writing standard for all English text (ASD-STE100 profile STE-80) |
| [docs/design/tokens.md](docs/design/tokens.md) | The design tokens of the Flow design system |
| [docs/plans/README.md](docs/plans/README.md) | The 19 stages, their dependencies and how to run a stage |
| `docs/reports/` | One report for each completed stage, with the evidence of each acceptance criterion |
| [ROADMAP.md](ROADMAP.md) | The work after the first release |

## Quality rules

1. `cargo xtask check` is the only gate. It runs format, clippy with pedantic and restriction lints, tests, doc tests, `cargo deny`, `cargo machete` and the repository checks. CI runs the same command on macOS, Linux and Windows.
2. The dependency graph between crates is fixed in a table. `cargo xtask lint-repo` fails on an edge that is not in the table.
3. Each stage has acceptance criteria that a test, a command or a measurement proves. A new agent session reviews each stage against the standards before the stage is complete.
4. All English text uses STE-80, a profile of ASD-STE100 Simplified Technical English.

## Build

Install [rustup](https://rustup.rs). The file `rust-toolchain.toml` selects the Rust version.

`cargo xtask check` needs `cargo-nextest`, `cargo-deny` and `cargo-machete`, at the versions that CI uses.

```sh
cargo install --locked cargo-nextest@0.9.146 cargo-deny@0.20.2 cargo-machete@0.9.2
```

Build the workspace, then run all checks.

```sh
cargo build --workspace
cargo xtask check
```

## License

Antenna is free software under the [GNU General Public License v3.0 or later](LICENSE). The model weights have their own licenses, which the Models screen shows.
