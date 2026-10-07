# Stage 05 Report. Library

| | |
|---|---|
| Stage file | `docs/plans/05-library.md` |
| Date | 2026-10-06 |
| Result | Complete, with three manual criteria pending. No Blocked section |
| CI | Run 37552421068 on commit `47ddf34` passed on `ubuntu-latest` and `macos-latest`. It failed on `windows-latest` in the text snapshot tests of stage 02, and the stage 02 commit `0b17331` fixed them. Run 37552665239 on `0b17331`, which contains all commits of this stage, passed on the three platforms |

## 1. Acceptance criteria

The last local run of `cargo nextest run -p antenna-library` reported 116 tests run and 116 passed. Tests of `src/` are unit tests. Tests of `tests/` are integration tests. The target `tests/documents/` has the files `lifecycle.rs`, `status.rs`, `listing.rs`, `recovery.rs` and `garbage.rs`.

| ID | Result | Evidence |
|---|---|---|
| AC-05-01 | Pass | `cargo xtask check` exits with code 0. Each commit of the stage passed it |
| AC-05-02 | Pass | Test `document_round_trips_when_created_and_loaded`. Test `document_persists_when_library_reopened` covers the reopen |
| AC-05-03 | Pass | Test `document_ids_sort_by_creation_time` in `src/document.rs` |
| AC-05-04 | Pass | Test `create_fails_when_title_is_blank` for an empty title, spaces, and a tab with a line feed |
| AC-05-05 | Pass | Tests `status_is_generating_when_progress_set`, `status_is_draft_when_text_changed`, `status_is_draft_when_segments_incomplete`, `status_is_exported_when_export_after_change` and `status_is_ready_when_complete_and_not_exported` in `src/status.rs` |
| AC-05-06 | Pass | Test `status_leaves_exported_when_text_saved` |
| AC-05-07 | Pass | Test `record_complete_ignored_when_text_hash_differs` |
| AC-05-08 | Pass | Test `progress_absent_after_reopen` |
| AC-05-09 | Pass | Test `status_is_draft_when_meta_is_older_than_text`. The status is `Draft` after `load`, see difference 1 |
| AC-05-10 | Pass | Tests `gc_deletes_temp_files_when_older_than_one_hour` and `open_keeps_temp_files_when_stale`. They set the file times with `File::set_modified` and pass a fixed `now` |
| AC-05-11 | Pass | Test `open_skips_document_when_meta_is_damaged` |
| AC-05-12 | Pass | Tests `search_matches_when_accents_differ`, `search_matches_text_when_index_built` and `search_matches_text_when_query_has_accents` |
| AC-05-13 | Pass | Test `search_matches_titles_only_when_index_not_built` |
| AC-05-14 | Pass | Test `filter_keeps_matching_statuses` |
| AC-05-15 | Pass | Test `groups_follow_period_rules` with a fixed `Zoned` value |
| AC-05-16 | Pass | Test `recent_returns_four_newest_opened` |
| AC-05-17 | Pass | Proptests `fold_is_idempotent` (4096 cases) and `more_terms_never_match_more`. A one-time scan of all Unicode scalar values and of pairs with `a`, `b` and a space also gave an idempotent `fold` |
| AC-05-18 | Pass | Proptests `each_document_in_one_group` and `grouping_is_deterministic`. The runs use the fixed seed `0x0a17e22a` |
| AC-05-19 | Pass | Test `segment_key_changes_when_any_field_changes`. Test `segment_key_matches_reference_digest_when_fields_given` compares one key with a SHA-256 that Python computed |
| AC-05-20 | Pass | Test `digest_differs_when_text_moves_between_fields` in `src/segments.rs` on the digest function. Test `segment_key_differs_when_text_moves_between_fields` checks the public API |
| AC-05-21 | Pass | Test `segment_round_trips_when_written_and_read` |
| AC-05-22 | Pass | Tests `segment_absent_until_commit` and `dropped_writer_deletes_temp_file` |
| AC-05-23 | Pass | Test `commit_keeps_existing_file_when_key_exists` |
| AC-05-24 | Pass | Test `gc_deletes_only_unused_old_segments` |
| AC-05-25 | Pending: measured alone after wave 2 | `cargo bench -p antenna-library`, median of `open_1000`, limit 100 ms. Add the result here |
| AC-05-26 | Pending: measured alone after wave 2 | `cargo bench -p antenna-library`, median of `search_1000`, limit 16 ms. Add the result here |
| AC-05-27 | Pending: measured alone after wave 2 | `cargo bench -p antenna-library`, median of `index_1000`, limit 1 s. Add the result here |
| AC-05-28 | Pass | Tests `default_root_uses_env_when_set` and `default_root_uses_data_dir_when_env_absent` |
| AC-05-29 | Pass | `cargo xtask lint-repo` passes |
| AC-05-30 | Pass | `wc -l docs/adr/0010-segment-store.md` prints 32 |
| AC-05-31 | Pass | Test `segment_store_matches_library_store_when_root_same` |
| AC-05-32 | Pass | Test `delete_removes_document_when_id_exists` |
| AC-05-33 | Pass | Test `set_voice_clears_segments_when_voice_changes` |
| AC-05-34 | Pass | Test `status_is_draft_when_segments_voice_differs` |
| AC-05-35 | Pass | Tests `record_segments_ignored_when_voice_differs` and `record_complete_ignored_when_voice_differs` |
| AC-05-36 | Pass | Test `rename_fails_when_title_is_blank` |
| AC-05-37 | Pass | Test `set_language_persists_when_library_reopened` |
| AC-05-38 | Pass | Test `record_export_persists_when_library_reopened` |
| AC-05-39 | Pass | Test `document_id_round_trips_when_text_parsed` |
| AC-05-40 | Pass | Test `default_root_fails_when_env_and_data_dir_absent` |
| AC-05-41 | Pass | Tests `segment_key_round_trips_when_text_parsed` and `segment_key_fails_when_text_not_64_hex_characters` |
| AC-05-42 | Pass | `rg -l RwLock crates/storage/library/src` prints `crates/storage/library/src/index.rs` only |

## 2. Quality gate

### 2.1 Automatic checks

`cargo xtask check` passes after each commit and after the rebase onto `main`.

### 2.2 Independent review

A new agent session reviewed the complete diff against the stage file, `docs/architecture.md`, `docs/standards.md` and `docs/writing.md`. It read every changed file and reported 24 findings, 3 of them major. The author fixed or answered each one.

| Finding | Fix |
|---|---|
| 1 (major). A replayed `Started` made a ready document `Draft` | `record_segments` keeps the list and its flag when it records the same keys. Test `record_segments_keeps_complete_list_when_keys_same` |
| 2 (major). AC-05-20 was not proven | The key hash uses the function `digest`. A unit test checks the field boundary on it |
| 3 (major). AC-05-09 holds only after `load` | The repair is difference 1. ADR 0010 records it. The test reads the status after `load`, and a second test checks that the hash is stored |
| 4. Writers can lose an update | ADR 0010 and the doc of `Library` state the single writer rule. A guard in `Index::replace` was not added, because the apps own one writer thread |
| 5. `build_search_index` stopped at the first failing file | It indexes the readable files and returns the first error |
| 6. The search ran under the read lock | `Index::matching` copies `Arc` handles under the lock and matches outside it |
| 7. `commit` returned the duration of a discarded write | It returns the duration of the stored file |
| 8. `save_text` failed when the old text file was absent | It ignores `NotFound`. An orphan file of the other format stays after a crash, and it is harmless |
| 9. `create` left a directory after a failed write | `create` removes the directory, best effort |
| 10. Garbage collection scope and races | The walk covers `library/` and `segments/` only, so `models/` is safe. The key set is read after the file listing. Not changed: the first error still stops the pass, and the segments of a skipped document are unprotected after one hour |
| 11. `open_reader` did not check the WAV format | It checks mono, 16 bits and integer samples. The channel count is a constant |
| 12. A blank title on read, and the trim | Not changed on read, because a hand edit of `document.toml` is not supported. The docs of `create` and `rename` state the trim |
| 13. `skipped` drops the cause | Not changed. The API of the stage is `&[PathBuf]`, and `tracing` is not in the scope |
| 14. Test structure | Tests that checked two behaviors are split. Tests got `when` names, named locals instead of magic values, and `RECENT_LIMIT` left the behavior test. Not changed: tests with the call inside `assert!` where the call is the only step. Unit tests of `src/` keep their own small builders, because a shared builder needs a `cfg(test)` item outside a test module |
| 15. Benchmark exceptions | Each `#[expect]` is on its function. The generator constants have names |
| 16. `Period` had no `Hash` | It derives `Hash`, and the proptest uses the value |
| 17. Naming | `TempFile::for_target`, `Filter::is_selected`, `is_after_space`, the field `temp_file`, `edits.rs`, and `text_form.rs` that removed the module cycle. `Entry::matches` became `Candidate::is_match`. Not changed: `meta` and `DocumentMeta`, which the stage file names |
| 18 to 20. Why comments | Comments tell the drop order of the writer, the process-wide counter of temporary names and the poisoned lock |
| 21. Forwarding methods | Kept. The public API of the stage file needs them |
| 22. `rename` changes `modified` | Kept. The stage file passes `now` to `rename`. A rename of an exported document gives `Ready` |
| 23. Defensive code and costs | Kept. The `unwrap_or` calls on paths without a file name make no panic. A change clones the metadata, which is small. No `fsync` of the parent directory, as the stage file defines |
| 24. STE-80 | Fixed in the ADR, `status.rs`, `edits.rs` and the `NoDataDir` message |

### 2.3 Simplification pass

The author read the complete diff again after the fixes. These changes made the code smaller.

1. `Index::matching` has one lock scope and one filter chain. The `bool` parameter became the private enum `TextIndex`.
2. The `as_text` helper left `meta_file.rs`, so `document.rs` and `meta_file.rs` no longer import each other.
3. `read_duration` serves `SegmentStore::duration` and `SegmentWriter::commit`.
4. The gc test helper builds files of any kind, so it has the name `aged_file`.

## 3. Differences from the plan

1. **Status after a crash between the two writes of `save_text`.** The status rules read only the metadata. In this crash the stored text hash and the hash of the segment list are both old, so the rules give `Ready` for the new text. AC-05-09 needs `Draft`. `Library::load` compares the hash of the text file with the stored hash. If they differ, it stores the new hash. Then `segments.text_hash` differs, and the status is `Draft`. Before `load`, the list shows `Ready`. ADR 0010 records this.
2. **Files.** The limit of 300 lines per file needs more files than the plan lists. Added are `src/meta_file.rs` (the TOML file), `src/text_form.rs` (the serde helpers, one private module as the plan says, in its own file), `src/edits.rs` (the metadata changes of `Library`) and the directory target `tests/documents/` with five files instead of `tests/documents.rs`. The garbage collection tests are in `tests/documents/garbage.rs`. The key tests are in `tests/segments.rs`.
3. **Combining marks.** `unicode_normalization::char::is_combining_mark` covers the general category Mark (Mn, Mc and Me). No approved crate gives only Mn. The six languages of Antenna have only Mn marks after NFD.
4. **`Period` order.** The variants go from the oldest to the newest, so `derive(Ord)` gives the newest as the greatest. `list` reads the groups in reverse order.
5. **Extra API.** `StoredVoice::new(VoiceId, Quality)` makes the stored form in one place. `Hash` and `Ord` are derived on `Period`.
6. **Early returns.** `save_text`, `set_voice` and `set_language` do nothing when nothing changes, so `modified` stays and no file is written.
7. **`LibraryError` variants.** The plan variants are unchanged. `load` reports a blank or non-UTF-8 text file as `Io` with `ErrorKind::InvalidData`, and the `CoreError` is the source. `ANTENNA_DATA_DIR` with an empty value counts as not set.
8. **`SegmentWriter` drop.** The private `TempFile` guard implements `Drop`, not `SegmentWriter`. `commit` moves the WAV writer out of `self`, which a `Drop` impl on `SegmentWriter` forbids.
9. **Segment store `open`.** It makes the `segments` directory, so the `Result` of the plan has an error to report.
10. **Garbage collection scope.** It walks `library/` and `segments/`, not the complete data root. The model store uses `<data>/models`, and a `.tmp-` file of a download must stay.
11. **Search.** `build_search_index` keeps going after a failing file and returns the first error. `list` matches a term in the title or in the text.
12. **Job records.** `record_complete` also needs a segment list with the same text hash. A replay of `record_segments` with the same keys keeps the list.

## 4. Measurements

AC-05-25, AC-05-26 and AC-05-27 are manual criteria. Other stages built and tested at the same time, so a measurement on this machine is not valid. The benchmarks compile and run once with `cargo bench -p antenna-library -- --test`.

| Criterion | Command | Limit | Result |
|---|---|---|---|
| AC-05-25 | `cargo bench -p antenna-library --bench library -- open_1000` | 100 ms | pending: measured alone after wave 2 |
| AC-05-26 | `cargo bench -p antenna-library --bench library -- search_1000` | 16 ms | pending: measured alone after wave 2 |
| AC-05-27 | `cargo bench -p antenna-library --bench library -- index_1000` | 1 s | pending: measured alone after wave 2 |

## 5. Manual QA

The stage has no manual QA step except the three measurements. Sign-off lines.

1. AC-05-25. Project owner, "approved" and date.
2. AC-05-26. Project owner, "approved" and date.
3. AC-05-27. Project owner, "approved" and date.
