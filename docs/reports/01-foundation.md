# Stage 01 Report. Foundation

| | |
|---|---|
| Stage file | `docs/plans/01-foundation.md` |
| Date | 2026-10-06 |
| Result | Complete. No Blocked section |
| CI | Run 37528020622 on commit `4023f0b`, green on `macos-latest`, `ubuntu-latest` and `windows-latest` |

## 1. Acceptance criteria

Each test name below is a test of `cargo nextest run --workspace --all-features`. The last local run reported 115 tests run, 115 passed and 6 skipped. The 6 skipped tests are the ignored tests of AC-01-28.

| ID | Result | Evidence |
|---|---|---|
| AC-01-01 | Pass | `cargo xtask check` exits with code 0. Each commit of the stage passed it in a new worktree with no untracked files |
| AC-01-02 | Pass | CI run 37528020622 of the push of `4023f0b`. The jobs `check (macos-latest)`, `check (ubuntu-latest)` and `check (windows-latest)` have the conclusion `success` |
| AC-01-03 | Pass | One xtask test for each row of `docs/standards.md` section 13: `lint_repo_finds_file_length`, `lint_repo_finds_banned_markers`, `lint_repo_finds_banner_comments`, `lint_repo_finds_commented_out_code`, `lint_repo_finds_comment_density`, `lint_repo_finds_dashes`, `lint_repo_finds_names`, `lint_repo_finds_crate_names`, `lint_repo_finds_lint_override`, `lint_repo_finds_versions`, `lint_repo_finds_dependency_graph`, `lint_repo_finds_non_exhaustive`, `lint_repo_finds_unsafe_code`, `lint_repo_finds_raw_design_values`, `lint_repo_finds_component_library`, `lint_repo_finds_desktop_layers`. Each test reads its fixture from `tools/xtask/tests/fixtures/` |
| AC-01-04 | Pass | Test `lint_repo_finds_forbidden_dependency` |
| AC-01-05 | Pass | `cargo tree -p antenna-core -e normal --depth 1` shows `strum v0.28.0` and `thiserror v2.0.21` only |
| AC-01-06 | Pass | Four `compile_fail,E0080` doctests on `EngineId::new` and `VoiceId::new` (empty text and text with `/`), and test `sample_rate_try_from_fails_when_zero` |
| AC-01-07 | Pass | Test `document_rejects_text_when_blank` |
| AC-01-08 | Pass | Tests `check_descriptor_fails_when_no_voices`, `_duplicate_voice`, `_foreign_voice`, `_empty_name`, `_empty_parameters`, `_revision`, `_sha256`, `_empty_extent` and `_duplicate_key`. The revision, SHA-256 and extent tests check a voice artifact and a variant artifact |
| AC-01-09 | Pass | Self-tests `harness_check_descriptor_fails_when_engine_has_no_voices`, `harness_check_audio_fails_when_samples_not_finite`, `harness_check_break_fails_when_engine_ignores_break`, `harness_check_determinism_fails_when_samples_change_between_calls`, `harness_check_reset_fails_when_state_survives_break`, `harness_check_edge_segments_fails_when_pictograph_fails` |
| AC-01-10 | Pass | `cargo nextest run -p antenna-engine-fake` runs 15 tests and all pass, the six conformance tests included |
| AC-01-11 | Pass | Test `registry_has_default_voice_when_fake_enabled` |
| AC-01-12 | Pass | Test `registry_fails_when_language_has_no_voice` calls `from_parts` with a factory that has no Spanish voice |
| AC-01-13 | Pass | Test `registry_finds_voice_when_given_display_text` |
| AC-01-14 | Pass | `cargo test --doc -p antenna-core` runs 4 trait doctests (`EngineFactory`, `Engine`, `Output`, `Sink`) and all pass |
| AC-01-15 | Pass | `ls docs/adr` shows the nine files 0001 to 0007, 0011 and 0012. `wc -l docs/adr/*.md` shows a maximum of 29 lines |
| AC-01-16 | Pass | `[workspace.package]` sets `license = "GPL-3.0-or-later"`. `cargo deny --config .config/deny.toml check licenses` prints `licenses ok` and exits with code 0. Difference 1 in section 3 tells why the command has this form |
| AC-01-17 | Pass | Test `text_format_is_markdown_when_extension_is_md` |
| AC-01-18 | Pass | Test `fake_has_four_named_voices_for_each_language` |
| AC-01-19 | Pass | Self-test `harness_check_audio_fails_when_one_quality_is_silent` |
| AC-01-20 | Pass | Test `variants_get_returns_field_when_quality_given` |
| AC-01-21 | Pass | Tests `language_round_trips_when_text_parsed`, `quality_round_trips_when_text_parsed` and `export_format_round_trips_when_text_parsed`. Tests `quality_is_balanced_by_default` and `export_format_is_mp3_by_default` check the defaults |
| AC-01-22 | Pass | Test `recording_marks_segment_starts_when_segments_begin` |
| AC-01-23 | Pass | `git ls-tree --name-only HEAD` shows `.cargo`, `.clippy.toml`, `.config`, `.github`, `.gitignore`, `.rustfmt.toml`, `CLAUDE.md`, `Cargo.lock`, `Cargo.toml`, `LICENSE`, `README.md`, `ROADMAP.md`, `crates`, `docs`, `rust-toolchain.toml` and `tools`. `cargo metadata` shows the manifest paths `crates/core`, `crates/engines/testkit`, `crates/engines/fake`, `crates/engines/registry` and `tools/xtask` |
| AC-01-24 | Pass | Test `registry_fails_when_voice_unknown` |
| AC-01-25 | Pass | Test `registry_fails_when_descriptor_invalid` calls `from_parts` with a factory whose descriptor has no voices |
| AC-01-26 | Pass | Tests `text_hash_round_trips_when_hex_parsed` and `text_hash_fails_when_text_not_64_hex_characters` |
| AC-01-27 | Pass | Test `core_types_have_required_derives` |
| AC-01-28 | Pass | `crates/engines/testkit/tests/ignored.rs` uses `conformance!` with `ignore = "needs models"`. `cargo nextest list -p antenna-engine-testkit --run-ignored only` lists its six tests. `cargo nextest run` skips them. With `--run-ignored only`, the six tests run and pass |
| AC-01-29 | Pass | `cargo xtask lint-repo` passes, and test `lint_repo_finds_non_exhaustive` passes |
| AC-01-30 | Pass | Tests `registry_finds_engine_when_given_id_text` and `registry_fails_when_engine_unknown` |
| AC-01-31 | Pass | Test `registry_uses_first_voice_when_default_entry_has_no_factory` |
| AC-01-32 | Pass | Tests `fake_returns_error_when_fault_fail_at_segment` and `fake_panics_when_fault_panic_at_segment` |
| AC-01-33 | Pass | Tests `fake_tone_lasts_40_ms_for_each_character` and `fake_tone_frequency_follows_segment_index` |
| AC-01-34 | Pass | Test `text_format_round_trips_when_text_parsed` |
| AC-01-35 | Pass | `grep -c 'NEXTEST_PROFILE: ci' .github/workflows/ci.yml` prints `1` |
| AC-01-36 | Pass | `cargo xtask lint-repo` passes the "Versions" row, and `grep -c '^default' crates/engines/registry/Cargo.toml` prints `0` |

## 2. Quality gate

### 2.1 Automatic checks

`cargo xtask check` passes locally and in CI on the three platforms.

### 2.2 Independent review

A new agent session reviewed the complete diff against the stage file, `docs/architecture.md`, `docs/standards.md` and `docs/writing.md`. It read every changed file. It reported 18 findings, 2 of them major. The author fixed each finding, except the part of finding 12 in the last row.

| Finding | Fix |
|---|---|
| 1. The engine of `tests/ignored.rs` failed when the ignored tests ran | The engine needs no model file now. The six tests pass with `--run-ignored only`, so the nightly job of stage 09 can run them |
| 2. The harness dropped the error of a failed descriptor, load or synthesis | `condition.rs` of the test kit gives one condition for each `Defect` and each `EngineError` variant. `RecordingSink` writes `map_err(\|flume::SendError(_)\| SinkError::Closed)` |
| 3. Panic messages and the `Violation` text stated the expected value | The panic messages state the defect. The `Violation` text starts with "failed condition:" |
| 4. Messages used `Debug` formatting | `Defect` implements `Display`. `Kind` of xtask derives `strum::Display`. The usage error has one variant for a missing command and one for an unknown command |
| 5. The "Versions" check gave the first line with the key anywhere in the manifest | The check searches only the table that declares the dependency, also `[<table>.<name>]` and target tables. The fixture has a `[features]` key with the same name |
| 6. Hex parsing occurred two times, and each xtask step repeated its command | `crates/core/src/hex.rs` is the one hex parser, with the named constant `BITS_PER_DIGIT`. Each step builds its command text from its arguments. `Spawn` names the real program |
| 7. The derive test did not check `Eq` of `ModelFiles` and some conversions | The test checks `Eq` of `ModelFiles`, `FromStr` of `TextHash`, `TryFrom<u32>` of `SampleRate` and `Into<usize>` of `SegmentIndex` |
| 8. A bare `compile_fail` doctest also passes for other errors | The four doctests are `compile_fail,E0080` |
| 9. Constructor and boolean names | `Defaults::new`, `Violation::new`, `Violation::with_line`, `is_prohibited_declaration` |
| 10. Visibility, a forwarding function and a module cycle | `ID` of the fake descriptor is private. `Harness::descriptor` is gone. `voices_of` moved to `defaults.rs`, so `registry.rs` depends on `defaults.rs` only |
| 11. The fake engine found the tone of a voice by its display name | `Timbre` has a `key`. The engine finds the timbre from the part of the voice key after the language code |
| 12. Redundant tests, test gaps and a test with two behaviors | `check_descriptor_passes_when_variants_share_key` is gone. The descriptor tests cover variant artifacts. The voice test checks each key and both descriptions. The reset test checks one behavior |
| 13. Parameters with a `_` prefix | Unused parameters of trait implementations are `_: Type` patterns |
| 14. STE-80 defects in four ADRs and the README | Fixed in ADR 0001, 0004, 0005, 0011 and 0012 and in the Build section of the README |
| 15. STE-80 defects and an inaccurate text in doc comments | Fixed in `audio.rs`, `quality.rs`, `recording.rs`, `harness.rs` and `defaults.rs` |
| 16. CI installed the latest tools, and `cargo deny` printed 11 warnings | CI and the README pin `cargo-nextest@0.9.146`, `cargo-deny@0.20.2` and `cargo-machete@0.9.2`. `.config/deny.toml` sets `unused-allowed-license = "allow"`. The allowlist did not change |
| 17. The `cargo deny` command of the documents fails with cargo-deny 0.20 | Recorded as difference 1 in section 3 |
| 18. `Option` as a literal kind, and `cr"..."` read as a plain literal | The lexer has the enum `Literal { Plain, Raw { hashes } }` and accepts the `c` prefix. A test covers `cr"..."` |
| 12, rejected part. `dashes_finds_em_dash` duplicates `lint_repo_finds_dashes` | Kept. The tooling of this session rejects a file write with U+2014, so the fixture contains U+2013 only. The second test makes sure that the check finds U+2014, which the Rust code writes as `'\u{2014}'` |

### 2.3 Simplification pass

The author read the complete diff again after the fixes. These changes removed code or made it smaller.

1. `registry.rs` had 371 lines. The default voice selection moved to `defaults.rs`, and the test factories moved to the `#[cfg(test)]` module `test_factory.rs`.
2. The self-tests of the test kit had 302 lines. They are now the directory target `tests/harness/` with `main.rs` and `engine.rs`.
3. The harness uses early returns instead of `then_some` chains, and it has no helper with a `bool` parameter.
4. The two raw string tests of the lexer are one test, so `source.rs` has 300 lines.

## 3. Differences from the plan

1. **`cargo deny` command.** cargo-deny 0.20 accepts `--config` only before the subcommand. `cargo deny check --config .config/deny.toml` prints a usage error. `cargo xtask check` runs `cargo deny --config .config/deny.toml check`, so it still passes `--config .config/deny.toml` to `cargo deny` as deliverable 5 says. After the stage, the text of `docs/standards.md` section 1 step 6 and the check of AC-01-16 got the correct argument order. This is a correction of the command syntax, not a change of a rule, so it needs no ADR.
2. **Variables of the xtask package.** `cargo run` gives xtask the `CARGO_PKG_*` variables. cargo-machete reads `CARGO_PKG_NAME` to parse its arguments, and it failed with `IO error for operation on machete`. Each step of `cargo xtask check` runs without the `CARGO_PKG_*` variables.
3. **Integration tests in a test module.** Clippy `tests_outside_test_module` fires on a `#[test]` function at the top level of a file in `tests/`. Each integration test file puts its tests in `#[cfg(test)] mod tests`. The `conformance!` macro puts its tests in `#[cfg(test)] mod conformance`.
4. **Fixture files.** The fixtures of `lint-repo` are `.txt` and `.toml` files in `tools/xtask/tests/fixtures/`. A fixture with the `.rs` extension or the name `Cargo.toml` would fail `cargo xtask lint-repo` on the repository itself.
5. **First commit.** `cargo test --workspace --doc` fails in a workspace with no library target. Thus the first commit contains the workspace, `xtask` and `antenna-core` together, and each commit of the stage passes `cargo xtask check`.
6. **Task 1 and task 3.** The repository and `LICENSE` with the full GPL-3.0 text existed before the stage. The stage added `.gitignore`. The project owner wrote the README before the stage, so the stage kept it and added a Build section with the build command.
7. **Crates of xtask.** `xtask` uses `cargo_metadata`, `toml`, `strum` and `thiserror`. All four are in the crate table of `docs/architecture.md` section 10. The file list comes from `git ls-files`, because `docs/standards.md` section 13 checks only the files that git tracks. Thus `xtask` does not use `ignore`.
8. **Extra derives and traits.** `CoreError` and `SinkError` also derive `Clone`, `Copy`, `PartialEq` and `Eq`, because all their data is `Copy`. `Defect` implements `Display` for the text of `CoreError`, and it does not implement `Error`. `FakeFactory` and `Fault` derive `Clone`, `Copy`, `Debug`, `PartialEq` and `Eq`.
9. **Lint expectations.** Each one has a reason that a reviewer can check.
   1. `SampleRate::constant`: `clippy::panic`, because only the rate constants call it.
   2. `Display for Defect`: `clippy::too_many_lines`, because the match has one arm for each of the nine defects.
   3. `Recording::collect`: `clippy::expect_used`, because a test that collects an output that it did not open has a defect.
   4. `FakeEngine::check_fault`: `clippy::panic`, because `Fault::PanicAt` tests the panic isolation of the pipeline.
   5. `Tone::sample_at`: `clippy::cast_precision_loss`, because the phase and the rate are below 2^24.
   6. `tools/xtask/src/output.rs`: `clippy::print_stdout` and `clippy::print_stderr`, as `docs/standards.md` section 11 says.

ADR 0004 records that the trait signatures of `docs/architecture.md` sections 5.3 and 5.4 compiled as written.

## 4. Measurements

This stage owns no budget of `docs/architecture.md` section 8. It made no measurement.

## 5. Manual QA

The stage has no manual acceptance criterion and no manual QA step.
