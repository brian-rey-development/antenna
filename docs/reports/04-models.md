# Stage 04 Report. Models

| | |
|---|---|
| Stage file | `docs/plans/04-models.md` |
| Date | 2026-10-06 |
| Result | Complete, except the manual criterion AC-04-23. No Blocked section |
| CI | Run 37554349982 on commit `b5e82de`, green on `macos-latest`, `ubuntu-latest` and `windows-latest` |

## 1. Acceptance criteria

The last local run of `cargo nextest run -p antenna-models` reported 101 tests run and 101 passed. Tests of the directory source are in `crates/storage/models/tests/store/`. Tests with the HTTP server are in `crates/storage/models/tests/hub/`. Unit tests are in `src/`.

| ID | Result | Evidence |
|---|---|---|
| AC-04-01 | Pass | `cargo xtask check` exits with code 0 on each commit of the stage |
| AC-04-02 | Pass | Test `ensure_installs_artifact_when_source_has_it` |
| AC-04-03 | Pass | Test `ensure_skips_fetch_when_artifact_installed`. The test removes the source file before the second call |
| AC-04-04 | Pass | Test `ensure_fails_and_deletes_when_hash_differs` |
| AC-04-05 | Pass | Test `ensure_fails_and_deletes_when_size_differs`. Tests `ensure_fails_on_hash_when_server_sends_more_bytes_than_declared` and `ensure_installs_declared_bytes_when_server_sends_more_bytes` cover the server. The download reads only the declared bytes, so an oversize body gives a hash error |
| AC-04-06 | Pass | Test `ensure_fetches_again_when_installed_file_truncated` |
| AC-04-07 | Pass | Test `ensure_fails_when_disk_space_insufficient` |
| AC-04-08 | Pass | Tests `ensure_succeeds_when_server_fails_three_times` and `ensure_fails_when_server_fails_four_times`. Test `ensure_completes_when_connection_breaks_more_often_than_attempt_limit` covers the retry budget of section 6, finding 12 |
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
| AC-04-20 | Pass | Test `final_path_stays_absent_when_hash_differs`. The test cannot observe the instant of the hash check, so its name tells what it proves. The plan uses the same name. During the download and after a failed hash check, the final path does not exist. Unit tests of `commit` in `src/verify.rs` show that the file moves to the final path after the hash check passes |
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
| 2. No read timeout for the body | The first fix did not work, because `timeout_recv_body` is a total budget. Fixed in section 6, finding 1 |
| 3. All errors were transient | Fixed. Only I/O, timeout, connection, host and protocol errors and the statuses 408, 429, 500, 502, 503 and 504 are transient. Another error returns `Network` with `attempts: 1`. Tests `ensure_retries_when_status_is_transient`, `ensure_does_not_retry_when_status_is_not_transient` and unit tests of `is_transient` |
| 4. A crash between the rename and the record caused a new download | Fixed. `reclaim_unverified` renames the file to the partial file, and the hash check installs it. Test `ensure_installs_without_fetch_when_verified_record_is_missing` |
| 5. The lock file was deleted after the lock | Partly fixed. `remove_artifact` deletes the lock file while it holds the lock. The race needs three parties. Section 6, finding 5 describes it and the effect of the hash check |
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
| 19. Naming | `ModelStore::install_missing` renamed. The module `verify.rs` stays, because it has one purpose. Section 6, finding 6 renamed `partial_len` |
| 20. Test structure | Fixed. Blank lines between the parts of three tests, and the test files split into topics. The attribute `#[cfg(test)]` stays on the test modules, because the lint `tests_outside_test_module` needs it |

### 2.3 Simplification pass

The author read the complete diff again. The pass removed the duplicate voice chain, the `drop` of a plain value in a test and the out parameter of `list_directory`.

## 3. Differences from the plan

1. `std::fs::File::try_lock` replaces `fs4::FileExt::try_lock_exclusive`. `fs4` 1.1 has no `try_lock_exclusive`, and the method of the standard library has the same behavior. `fs4` stays for `available_space`.
2. The crate has the file `src/install.rs` with the type `Installer`. It keeps `store.rs` small and keeps each function at 5 parameters or less.
3. `tests/store/` is a directory with `main.rs`, because a test file has a maximum of 300 lines. `tests/support/mod.rs` is shared by both test targets with `#[path]`.
4. The test server keeps the bodies in memory. A one-shot `TcpListener` implements "close the connection after a fixed number of bytes", because `tiny_http` cannot close a connection early.
5. `ureq` 3 has no timeout for each read of a body, and `timeout_recv_body` is a total budget. The file `src/source/stall.rs` wraps `DefaultConnector` in `ReadTimeoutConnector`. The agent comes from `Agent::with_parts`. Each wait for input is limited to `READ_TIMEOUT` (30 s), also for the response head. A server that stops in the middle of a body gives a transient timeout error, and the retry continues from the `.part` file. The agent has a resolve timeout of 10 s. `HubFetch::new` takes the read timeout as a parameter. The cancel flag is read after each block, so a cancel waits up to `READ_TIMEOUT` while a read blocks.
6. `flume` is a dev-dependency, because the clippy configuration forbids `Mutex` and `mpsc` for the records of the tests.
7. A permanent error that is not `NotFound` returns `Network` with `attempts: 1`.
8. `open` does not touch the disk. `ensure` creates the root directory before `fs4::available_space` (decision rule 2). `disk_usage` returns 0 for a root that does not exist.
9. `ModelStore::open` returns `Self`. It does not touch the disk, so it cannot fail. The plan signature is changed to match.
10. A download counts only consecutive attempts without progress. The plan text of `ensure` step 5.3 and AC-04-08 says so.

## 4. Manual criteria

### AC-04-23. No request to a host other than 127.0.0.1

Pending. The project owner runs these steps on the reference machine (Apple M5 Pro, 24 GB).

1. Run `networksetup -setairportpower en0 off` and remove any network cable.
2. Run `curl -sI https://huggingface.co`. The command must fail.
3. Run `unset HTTP_PROXY http_proxy HTTPS_PROXY https_proxy ALL_PROXY all_proxy`. `ureq` 3 reads these variables by default, so a proxy can change the route of the tests.
4. In the same shell, run `cargo nextest run -p antenna-models`. All tests must pass.
5. Run `networksetup -setairportpower en0 on`.

Sign-off. The project owner writes "approved" and the date here.

Sign-off line: ______________________

## 5. CI

Run 37554349982 of the push of `b5e82de`, the last commit of the second review. The jobs `check (macos-latest)`, `check (ubuntu-latest)` and `check (windows-latest)` have the conclusion `success`.

## 6. Second independent review

A second review of commit `deeb0f9` reported 13 findings. The project owner asked to fix all of them. Each fix has a test, except finding 2, which no test can show.

| Finding | Result |
|---|---|
| 1. A server that stops in the middle of a body blocks the download | Fixed. See difference 5. Test `fetch_times_out_and_keeps_bytes_when_server_stalls_mid_body` uses a raw `TcpListener` that sends a partial body and then waits for a message. The read timeout is 200 ms. The test expects `FetchError::Transient(ureq::Error::Timeout(_))` and a `.part` file with the bytes sent before the stall. The assertions do not depend on timing |
| 2. No flush before the rename | Fixed. `commit` calls `sync_all` on the partial file after the hash check and before the rename and the `.verified` write. A crash cannot leave wrong data at the final path. No test can simulate a crash, so the unit tests only show that `commit` still installs the file |
| 3. Cancel ignored before the first request and during the hash check | Fixed. `fetch_with_retry` returns `Cancelled` before the first request when the flag is set, after it takes the lock. The hash check reads the flag before each block. Tests `ensure_returns_cancelled_without_request_when_cancel_is_set`, `ensure_returns_cancelled_without_partial_file_when_cancel_is_set`, `sha256_hex_returns_cancelled_when_flag_is_set` and `commit_keeps_partial_file_when_cancelled`. The HTTP test now expects 0 requests |
| 4. The body is not limited, and `Content-Range` is not read | Fixed. The body reader stops at the bytes that are still missing, so a body cannot write past the disk space check. A 206 reply must have a `Content-Range` that starts at the requested byte, or the result is `RangeUnsupported`. An oversize body gives a hash error. Tests `ensure_fails_on_hash_when_server_sends_more_bytes_than_declared`, `ensure_installs_declared_bytes_when_server_sends_more_bytes`, `ensure_fails_when_server_answers_other_range_than_requested` and `ensure_keeps_partial_file_when_server_answers_other_range_on_resume` |
| 5. Wrong text about the lock race, and a spurious `NotFound` | Fixed. The race needs three parties. The remover unlinks the lock file while it holds the lock. A caller that opened the old file then locks the unlinked file. A third caller creates a new lock file and locks it. Then two callers hold a lock of one artifact and write one `.part` file. The hash check turns this worst case into a `Hash` error, because a mixed file cannot pass it. The design stays. `try_lock` now creates the directory of the artifact on each call, so a removal that prunes an empty directory cannot make a waiting download fail with `NotFound`. A small window remains between the creation and the open of the lock file. Test `try_lock_creates_directory_when_it_is_missing` |
| 6. Abbreviation `len` in own names | Fixed. `partial_bytes`, `resumable_bytes`, `file_bytes` and `length` in the test helpers |
| 7. Unused `fs4` feature `sync` | Fixed. The crate uses `fs4.workspace = true`. `available_space` does not need the feature |
| 8. Duplicated code | Fixed. `has_declared_size` replaces two closures. `BLOCK_BYTES`, `FILE_BYTES`, `NOT_CANCELLED` and the path helpers `file`, `partial` and `with_suffix` are in `tests/support/mod.rs`. The method `Fixture::published` replaces the two copies of `published`. It is not in `tests/support/mod.rs`, because it needs the `Fixture` type of the `store` target, and the `hub` target would not use it. Both targets share one `FILE_BYTES` of 300000 bytes |
| 9. Test quality | Fixed. The status loops have a message with the status in each assertion. The test of the final path has the name `final_path_stays_absent_when_hash_differs` (see AC-04-20). Test `ensure_resumes_when_partial_file_exists_and_extent_is_range` resumes an `Extent::Range` artifact over HTTP with a partial file |
| 10. "failed after 1 attempts" | Fixed. The message is "the download of {artifact} failed, attempt count {attempts}". Test `network_message_names_attempt_count_when_count_is_one` |
| 11. `open` creates the root | Fixed. `open` does not touch the disk. `ensure` creates the root before the disk space check. `disk_usage` returns 0 for a missing root. The text of decision rule 2 does not need a change, because it says to create the root first. Tests `open_leaves_root_absent_when_directory_missing`, `ensure_creates_root_when_directory_missing` and `disk_usage_is_zero_when_root_missing`. `open` returns `Self` (difference 9) |
| 12. Retry budget | Fixed by decision of the project owner. The count of attempts starts again when the `.part` file grew during an attempt. A download that never gains bytes still fails after four attempts. The plan text of `ensure` step 5.3 and AC-04-08 says "four consecutive attempts without progress". Test `ensure_completes_when_connection_breaks_more_often_than_attempt_limit` breaks the connection five times, and the download completes. The test fails without the change |
| 13. Empty `ANTENNA_MODEL_DIR` | Recorded. An empty `ANTENNA_MODEL_DIR` counts as not set, and `open_default` uses the platform directory. Unit test `default_root_uses_data_dir_when_env_empty` covers it. The documentation of `open_default` says so |

The manual steps of AC-04-23 in section 4 now include the unset of the proxy variables.
