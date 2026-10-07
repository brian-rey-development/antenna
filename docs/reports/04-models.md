# Stage 04 Report. Models

| | |
|---|---|
| Stage file | `docs/plans/04-models.md` |
| Date | 2026-10-06 |
| Result | Complete, except the manual criterion AC-04-23. No Blocked section |
| CI | See section 5 |

## 1. Acceptance criteria

The last local run of `cargo nextest run -p antenna-models` reported 87 tests run and 87 passed. Tests of the directory source are in `crates/storage/models/tests/store/`. Tests with the HTTP server are in `crates/storage/models/tests/hub/`. Unit tests are in `src/`.

| ID | Result | Evidence |
|---|---|---|
| AC-04-01 | Pass | `cargo xtask check` exits with code 0 on each commit of the stage |
| AC-04-02 | Pass | Test `ensure_installs_artifact_when_source_has_it` |
| AC-04-03 | Pass | Test `ensure_skips_fetch_when_artifact_installed`. The test removes the source file before the second call |
| AC-04-04 | Pass | Test `ensure_fails_and_deletes_when_hash_differs` |
| AC-04-05 | Pass | Test `ensure_fails_and_deletes_when_size_differs`. Test `ensure_fails_when_server_sends_more_bytes_than_declared` covers the server |
| AC-04-06 | Pass | Test `ensure_fetches_again_when_installed_file_truncated` |
| AC-04-07 | Pass | Test `ensure_fails_when_disk_space_insufficient` |
| AC-04-08 | Pass | Tests `ensure_succeeds_when_server_fails_three_times` and `ensure_fails_when_server_fails_four_times` |
| AC-04-09 | Pass | Test `ensure_does_not_retry_when_not_found`. The server counts 1 request. Test `ensure_does_not_retry_when_access_denied` covers 401 and 403 |
| AC-04-10 | Pass | Test `ensure_stops_after_one_block_when_cancelled` (exactly one block with the directory source). Test `ensure_stops_after_one_block_when_cancelled_during_download` (HTTP) |
| AC-04-11 | Pass | Test `ensure_resumes_when_partial_file_exists` checks the header `bytes=100000-299999` |
| AC-04-12 | Pass | Test `ensure_resumes_when_connection_breaks`. The server closes the connection after 100000 bytes. The retry sends the `Range` header and the result is installed with the correct SHA-256 |
| AC-04-13 | Pass | Test `ensure_downloads_only_range_when_extent_is_range` |
| AC-04-14 | Pass | Test `ensure_fails_when_server_ignores_range_for_extent_range` |
| AC-04-15 | Pass | Test `ensure_restarts_when_server_ignores_range_on_resume` |
| AC-04-16 | Pass | Test `ensure_keeps_range_when_redirected`. `ureq` keeps the header, so decision rule 1 is not necessary |
| AC-04-17 | Pass | Unit test `layout_separates_ranges_of_one_file` |
| AC-04-18 | Pass | Unit test `progress_is_monotonic_and_complete`. Test `ensure_reports_increasing_progress_when_connection_breaks` checks it through `ensure` |
| AC-04-19 | Pass | Test `ensure_fails_when_keys_collide` |
| AC-04-20 | Pass | Test `final_path_absent_until_hash_checked` |
| AC-04-21 | Pass | Unit tests `default_root_uses_env_when_set` and `default_root_uses_data_dir_when_env_absent` |
| AC-04-22 | Pass | Test `voice_artifacts_lists_variant_then_voice` |
| AC-04-23 | Pending | Manual. See section 4 |
| AC-04-24 | Pass | `cargo xtask lint-repo` passes. `cargo tree -p antenna-models -e normal --depth 1` lists `antenna-core` as the only workspace crate |
| AC-04-25 | Pass | `wc -l docs/adr/0008-byte-range-artifacts.md` prints 27 |
| AC-04-26 | Pass | Tests `is_installed_true_when_variant_and_voice_installed` and `is_installed_false_when_variant_missing` |
| AC-04-27 | Pass | Test `remove_engine_keeps_artifact_when_listed_in_keep` |
| AC-04-28 | Pass | Test `remove_engine_fails_when_artifact_is_locked` |
| AC-04-29 | Pass | Test `remove_engine_returns_deleted_bytes_and_prunes_directories` |
| AC-04-30 | Pass | Test `disk_usage_counts_partial_files` |
| AC-04-31 | Pass | Test `engine_disk_usage_ignores_missing_artifacts` |
| AC-04-32 | Pass | Test `ensure_installs_artifacts_without_descriptor` |
| AC-04-33 | Pass | Test `missing_bytes_subtracts_partial_files` |
| AC-04-34 | Pass | Test `ensure_waits_on_lock_when_other_call_fetches`. The server counts 1 request |
| AC-04-35 | Pass | Unit test `progress_throttles_reports_when_interval_not_elapsed` |
| AC-04-36 | Pass | Test `ensure_returns_cancelled_when_waiting_on_lock` |
| AC-04-37 | Pass | Tests `engine_is_installed_when_all_voices_installed` and `engine_is_not_installed_when_one_voice_missing` |
| AC-04-38 | Pass | Test `engine_download_bytes_counts_shared_file_once` |
| AC-04-39 | Pass | Test `keep_artifacts_includes_other_engines_and_extra` |
| AC-04-40 | Pass | Unit test `default_root_fails_when_env_and_data_dir_absent` |

## 2. Quality gate

### 2.1 Automatic checks

`cargo xtask check` passes locally on each commit.

### 2.2 Independent review

A new agent session reviewed the diff against the stage file, `docs/architecture.md`, `docs/standards.md` and `docs/writing.md`. It read every changed file. It reported 20 findings. The author fixed or answered each one.

| Finding | Result |
|---|---|
| 1. An artifact of zero bytes failed | Fixed. `fetch_once` creates the partial file. Test `ensure_installs_empty_artifact_when_extent_is_zero` |
| 2. No read timeout for the body | Not fixed, see difference 5. The constant is `RESPONSE_TIMEOUT` |
| 3. All errors were transient | Fixed. Only I/O, timeout, connection, host and protocol errors and the statuses 408, 429, 500, 502, 503 and 504 are transient. Another error returns `Network` with `attempts: 1`. Tests `ensure_retries_when_status_is_transient`, `ensure_does_not_retry_when_status_is_not_transient` and unit tests of `is_transient` |
| 4. A crash between the rename and the record caused a new download | Fixed. `reclaim_unverified` renames the file to the partial file, and the hash check installs it. Test `ensure_installs_without_fetch_when_verified_record_is_missing` |
| 5. The lock file was deleted after the lock | Partly fixed. `remove_artifact` deletes it while it holds the lock. A caller that waited on the old file can still lock it. The case needs a removal during a download of the same engine, which the Models screen does not permit |
| 6. Progress overshoot when a server ignores `Range` | Fixed. The bytes of one file stay below its share. Test renamed to `progress_stays_at_total_when_download_restarts` |
| 7. `disk_usage` failed when a file vanished | Fixed. The walk skips an entry that is gone |
| 8. A shared file counted two times | Fixed. `ensure` removes duplicate local files from the missing list. Test `ensure_installs_shared_file_once_when_two_keys_have_one_file` |
| 9. Test gaps | Fixed. Tests for statuses, `range_header`, a long partial file over HTTP, more bytes than declared, cancel in an HTTP body, the verified record, progress after a retry, the total with a partial file and the total of two files. `open_default` stays untested, because a test cannot set the environment without `unsafe` code |
| 10. Loose cancel bound | Fixed. The directory test checks exactly one block |
| 11. Time in the lock test | Answered. A comment gives the margin. The assertions hold for any timing |
| 12. `requests` drained the log | Fixed. The server keeps the requests that it already read |
| 13. STE-80 | Fixed in ADR 0008 and in the documentation of `ensure` and `keep_artifacts`. The error message keeps the literal `ANTENNA_MODEL_DIR` |
| 14, 15, 17, 18 | Fixed. Names `kept_bytes` and `earlier_bytes`, one `voice_files` helper, `RETRY_COUNT`, hex text without a table of nibbles |
| 16. Two identical match arms | Answered. A generic `Installer` gives static dispatch, as `docs/standards.md` section 5.6 asks. The two arms are one line each |
| 19. Naming | `ModelStore::install_missing` renamed. The module `verify.rs` and `partial_len` stay, because each has one purpose |
| 20. Test structure | Fixed. Blank lines between the parts of three tests, and the test files split into topics. The attribute `#[cfg(test)]` stays on the test modules, because the lint `tests_outside_test_module` needs it |

### 2.3 Simplification pass

The author read the complete diff again. The pass removed the duplicate voice chain, the `drop` of a plain value in a test and the out parameter of `list_directory`.

## 3. Differences from the plan

1. `std::fs::File::try_lock` replaces `fs4::FileExt::try_lock_exclusive`. `fs4` 1.1 has no `try_lock_exclusive`, and the method of the standard library has the same behavior. `fs4` stays for `available_space`.
2. The crate has the file `src/install.rs` with the type `Installer`. It keeps `store.rs` small and keeps each function at 5 parameters or less.
3. `tests/store/` is a directory with `main.rs`, because a test file has a maximum of 300 lines. `tests/support/mod.rs` is shared by both test targets with `#[path]`.
4. The test server keeps the bodies in memory. A one-shot `TcpListener` implements "close the connection after a fixed number of bytes", because `tiny_http` cannot close a connection early.
5. `ureq` 3 has no timeout for each read of a body. `READ_TIMEOUT` is `RESPONSE_TIMEOUT` and limits the wait for the response head. A server that stops in the middle of a body blocks the download. A custom `ureq` connector with a socket read timeout can fix this in a later stage.
6. `flume` is a dev-dependency, because the clippy configuration forbids `Mutex` and `mpsc` for the records of the tests.
7. A permanent error that is not `NotFound` returns `Network` with `attempts: 1`.
8. `open` creates the root directory, so `fs4::available_space` works on a new store (decision rule 2).

## 4. Manual criteria

### AC-04-23. No request to a host other than 127.0.0.1

Pending. The project owner runs these steps on the reference machine (Apple M5 Pro, 24 GB).

1. Run `networksetup -setairportpower en0 off` and remove any network cable.
2. Run `curl -sI https://huggingface.co`. The command must fail.
3. Run `cargo nextest run -p antenna-models`. All tests must pass.
4. Run `networksetup -setairportpower en0 on`.

Sign-off. The project owner writes "approved" and the date here.

Sign-off line: ______________________

## 5. CI

Recorded after the push. See the section "CI" at the end of this file.
