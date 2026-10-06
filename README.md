# Antenna

Antenna converts text into podcast audio with text-to-speech models that run on your computer. No text and no audio leave the machine. Antenna supports English, Spanish, Portuguese, French, Italian and German.

## Status

Antenna is in development. The repository contains the architecture and the implementation plan. The code arrives in 19 stages.

| Document | Content |
|---|---|
| [docs/architecture.md](docs/architecture.md) | The crates, the contracts, the data flow and the performance budgets |
| [docs/standards.md](docs/standards.md) | The code standards and the checks that enforce them |
| [docs/writing.md](docs/writing.md) | The writing standard for all English text (STE-80) |
| [docs/plans/README.md](docs/plans/README.md) | The 19 stages, their dependencies and how to run a stage |
| [ROADMAP.md](ROADMAP.md) | The work after the first release |

## Technology

Antenna is a Rust workspace. The desktop app uses GPUI. The speech engines run Qwen3-TTS and NVIDIA Magpie TTS Multilingual with candle, on Metal on Apple Silicon. The first release target is macOS on Apple Silicon.

## License

Antenna is free software under the [GNU General Public License v3.0 or later](LICENSE).
