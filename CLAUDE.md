# Antenna

Antenna is a desktop app that reads text aloud with local TTS models. It plays the audio while it generates it, and it exports the result as an episode file. The code is Rust. The license is GPL-3.0-or-later.

## Read before you start

1. `docs/architecture.md` for the design and the contracts.
2. `docs/standards.md` for the code rules. Each rule ends with its check. CI runs the `[clippy]`, `[lint-repo]`, `[check]` and `[test]` checks. The independent review of section 20 checks the `[review]` rules. Sections 17 to 20 are review rules. Sections 18 and 19 define the design quality and show the style to copy.
3. `docs/writing.md` for the writing rules (STE-80) and the glossary.
4. `docs/plans/README.md` and the stage file of your task.

## Hard rules

1. Do only the work of your stage. The stage file lists the scope.
2. Run `cargo xtask check` before each commit. Commit only when it passes.
3. Do not change a test, a lint, a threshold, a fixture or an acceptance criterion to make a check pass. Fix the code. If a test is wrong, stop and write a Blocked section.
4. Do not add a dependency that is not in the crate table of `docs/architecture.md`. If you need one, write a Blocked section. A human changes the table.
5. Put each crate and each file in the location of `docs/architecture.md` section 3 and `docs/standards.md` section 3. Do not add a file to the repository root that is not in the tree of `docs/architecture.md` section 3.
6. Do not change a contract in `docs/architecture.md` section 5 without approval and an ADR.
7. Use `#[expect(lint, reason = "...")]` for each lint exception. Do not use `#[allow]`.
8. Write zero comments, except comments that tell why. Do not write `TODO`.
9. Write documentation, commit messages and stage reports in English, in the STE-80 profile. Do not use em dashes.
10. A stage is complete only when the definition of done in `docs/standards.md` section 16 is true. It includes the quality gate of section 20.

## When you are blocked

1. Read the "Decision rules" section of your stage file. It has the answers to the expected problems.
2. If no rule applies, stop. Add a "Blocked" section at the end of the stage file. Write what you tried, the exact error and the decision that you need.
3. Do not invent a workaround that changes the architecture.

## Commands

```sh
cargo xtask check                 # all checks, the same as CI
cargo nextest run -p antenna-text # tests of one crate
cargo run -p antenna-cli -- speak notes.md
cargo run -p antenna-eval -- bench --voice <voice-id>
cargo run -p antenna-ui --example gallery
```
