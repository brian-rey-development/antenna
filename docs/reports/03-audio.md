# Stage 03 Report. Audio

| | |
|---|---|
| Stage file | `docs/plans/03-audio.md` |
| Date | 2026-10-06 |
| Result | Complete, with three manual criteria that wait for the project owner and for a quiet reference machine. No Blocked section |
| CI | Run 37555039311 on commit `9c6bcc1`, green on `macos-latest`, `ubuntu-latest` and `windows-latest` |

## 1. Acceptance criteria

Each test name is a test of `cargo nextest run --workspace --all-features`. The crate `antenna-audio` has 183 tests. One of them is ignored (`player_plays_track_when_device_available`).

| ID | Result | Evidence |
|---|---|---|
| AC-03-01 | Pass | `cargo xtask check` exits with code 0 on each commit of the stage. The CI run 37555039311 of the push of `9c6bcc1` has the conclusion `success` for the three jobs `check (macos-latest)`, `check (ubuntu-latest)` and `check (windows-latest)` |
| AC-03-02 | Pass | Tests `fill_does_not_allocate_when_normal`, `_paused`, `_flush`, `_underrun` and `_buffering` in `callback.rs`. `lib.rs` installs `assert_no_alloc::AllocDisabler` for the unit tests |
| AC-03-03 | Pass | Test `fill_writes_silence_when_paused` |
| AC-03-04 | Pass | Tests `fill_discards_samples_when_flush_requested` and `fill_discards_samples_when_flush_requested_while_paused` |
| AC-03-05 | Pass | Test `fill_counts_underrun_when_playing_and_short`. Test `fill_counts_no_underrun_when_buffer_fills_output` checks the boundary |
| AC-03-06 | Pass | Test `fill_skips_underrun_when_buffering` |
| AC-03-07 | Pass | Tests `fill_copies_sample_to_each_channel` and `fill_applies_volume`. Test `fill_converts_samples_when_output_is_i16` checks decision rule 4 |
| AC-03-08 | Pass | Test `feed_plays_stored_segments_in_order` |
| AC-03-09 | Pass | Test `feed_plays_live_chunks_then_swaps_to_stored`. The stored file replaces the live buffer in the middle of the segment |
| AC-03-10 | Pass | Test `feed_buffers_when_segment_pending` |
| AC-03-11 | Pass | Test `feed_ends_after_last_segment`. The track has 3000 samples, more than one drained buffer, so an early `Ended` fails the test |
| AC-03-12 | Pass | Tests `position_tracks_played_frames` and `position_restarts_at_seek_target` in `atomics.rs`. Test `seek_positions_target_when_offset_exceeds_played_frames` checks the position through the feed thread |
| AC-03-13 | Pass | Test `seek_delivers_target_sample_in_first_drained_buffer`. The test seeks while the track is paused, so the prefill is complete when the `Playing` event arrives. The first call of `drain` after that event returns the target sample. The test uses no clock |
| AC-03-14 | Pass | Test `feed_ignores_command_when_track_replaced`. A stale `pause` would pause the current track, and a barrier seek shows that the track still plays |
| AC-03-15 | Pass | Tests `live_sink_drops_samples_when_track_replaced` and `live_sink_returns_ok_when_player_stopped` |
| AC-03-16 | Pass | Tests `wait_mode_blocks_when_not_playing`, `wait_mode_polls_without_delay_when_playing_and_work_pending` and `wait_mode_polls_for_refill_when_playing_and_idle` |
| AC-03-17 | Pass | Tests `resample_keeps_length_when_24000_to_48000` and `_22050_to_48000` (exact length), `resample_keeps_level_when_sine` and `_from_22050`, and the short input tests |
| AC-03-18 | Pass | Test `peaks_match_bucket_maximums` |
| AC-03-19 | Pass | Tests `wav_export_round_trips_when_24000`, `_44100` and `_48000` |
| AC-03-20 | Pass | Tests `mp3_export_duration_matches_when_24000`, `_44100` and `_48000` decode every packet with `symphonia` and compare the exact sample count. Test `mp3_export_keeps_level_when_decoded` compares the loudness. Test `mp3_export_has_title_tag` checks the title |
| AC-03-21 | Pass | Test `ogg_export_has_valid_pages_and_granule`. Tests `ogg_export_has_opus_head_and_tags_when_exported` and `ogg_export_ends_stream_on_last_packet_only` check the pages. More tests check the final granule for the three export rates, a partial last frame, one sample and no sample |
| AC-03-22 | Pass | Tests `export_normalizes_loudness_when_minus_30_lufs` and `_minus_6_lufs` |
| AC-03-23 | Pass | Test `export_limits_true_peak_when_crest_high`. The true peak is -1 dBTP with a difference of 0.01 dB or less, and the fixture needs the limit |
| AC-03-24 | Pass | Test `export_keeps_silence_when_loudness_infinite` |
| AC-03-25 | Pass | Tests `export_leaves_no_file_when_cancelled`, `export_leaves_no_file_when_cancelled_while_measuring`, `export_leaves_no_file_when_encoder_fails` (a segment fails after the encoder wrote the first one) and `export_leaves_no_file_when_commit_fails`. Test `part_file_deletes_file_when_rename_fails` checks the cleanup |
| AC-03-26 | Pending: measured alone after wave 2 | `cargo bench -p antenna-audio --bench export`. The bench compiles and runs once with `cargo bench -p antenna-audio --bench export -- --test` (exit code 0). It reported no time |
| AC-03-27 | Pass | `cargo nextest run -p antenna-audio --run-ignored only -E 'test(player_plays_track_when_device_available)'` passed on the reference machine in 3.1 s. Section 5 has the output |
| AC-03-28 | Pass | `cargo xtask lint-repo` passes. `cargo tree -p antenna-audio -e normal` lists `antenna-core` as the only workspace dependency |
| AC-03-29 | Pass | Tests `device_lost_pauses_track_when_device_not_available` and `device_lost_sets_paused_flag_when_device_not_available` |
| AC-03-30 | Pending: measured alone after wave 2 | `cargo run --release -p antenna-audio --example seek_latency`. The example ran once as a smoke test and exited with code 0. Other stages built at the same time, so the numbers are not valid and are not in this report |
| AC-03-31 | Pass | Test `track_marks_segment_pending_when_entry_none` |
| AC-03-32 | Pass | Test `live_output_fails_when_rate_differs` |
| AC-03-33 | Pass | Test `position_is_correct_when_seek_offset_exceeds_played_frames`. Test `seek_positions_target_when_offset_exceeds_played_frames` checks the same case through the feed thread |
| AC-03-34 | Pass | Test `read_segment_scales_when_sample_is_max` |
| AC-03-35 | Pass | Test `device_changed_keeps_state_when_stream_continues`. A barrier makes sure that the feed thread applied all commands |
| AC-03-36 | Pass | Test `feed_reopens_stream_when_play_after_device_lost` |
| AC-03-37 | Pass | Test `seek_skips_flush_when_device_lost` |
| AC-03-38 | Pass | Tests `seek_drops_stream_when_flush_unanswered` and `seek_applies_queued_commands_when_flush_completes`. The second test queues a second `Seek` during the wait, so only the correct order gives the second target |
| AC-03-39 | Pass | Tests `ogg_export_uses_encoder_rate_when_24000`, `_44100` and `_48000` |
| AC-03-40 | Pass | Test `mp3_export_has_artist_and_comment_tags` |
| AC-03-41 | Pass | Tests `export_rate_round_trips_when_text_parsed` and `loudness_round_trips_when_text_parsed` |
| AC-03-42 | Pass | Test `ogg_export_duration_matches_when_hz44100` reads the file with `symphonia`. Test `ogg_export_duration_matches_when_granule_gives_it` computes the duration from the last granule position minus the pre-skip |

## 2. Quality gate

### 2.1 Automatic checks

`cargo xtask check` passes on each commit of the stage, with the target directory outside a path that has a space (see section 7). The tests of the crate ran 36 times under CPU load (6 parallel loops of 6 runs) and never failed.

### 2.2 Independent review

Three new agent sessions reviewed the complete diff. Each one got the stage file, `docs/architecture.md`, `docs/standards.md` and `docs/writing.md`. One session read the player and the live output, one read the export, the resampler and the peaks, and one read all tests, the benchmark, the example, `Cargo.toml` and the CI file. They reported 22, 24 and 27 findings, 21 of them major (counting the AC weaknesses). The author fixed each finding, except the rows in the last table.

| Finding | Fix |
|---|---|
| A flush that the callback did not answer left the stale epoch, so a reopened stream discarded its prefill | `lose_stream` sets `flushed_epoch` to `flush_epoch`. Test `seek_keeps_prefill_when_stream_reopened_after_unanswered_flush` |
| A stored segment that cannot be read counted as complete and the player skipped it without a message | The segment stays incomplete and the player waits in `Buffering`. The error is logged, and a later read retries. Tests `source_waits_when_stored_file_missing` and `source_loads_file_when_it_appears_after_failed_read` |
| `is_buffering` stayed true after a seek during the drain at the end of a track, and a seek counted one false underrun | `flush_ring_buffer` sets the flag before the flush and `reposition` publishes the flags from the state |
| `locate` could overflow with a very negative start | `checked_sub` |
| The `channels` of `fill` was a bare `usize` | `NonZeroUsize`, parsed once in `parse_config`. New variant `InvalidDeviceConfig` carries the values |
| `load` left the old track in the state `Playing` | `load` pauses the old track and sends the event. Test `feed_pauses_old_track_when_track_loaded` |
| `position` doc and `play` after `Ended` were not documented | The doc comments say it |
| `PlayerState::from_repr`, `PlayerState` default and `Volume` default were public | The derives are gone. `Volume::FULL` is `pub(crate)` |
| A zero device rate and an unsupported sample format became an error without data | New variants `InvalidDeviceConfig` and `UnsupportedFormat` |
| Names | `Clock::new`, `SegmentSource::from_file`, `flush_ring_buffer`, `request_flush`, `advance` and `push_block`, files `advance.rs` and `apply.rs`, `PartFile::open`, `is_page_end`, `AudioError::LoudnessMeter`, `linear_gain` |
| `Feed::emit` only forwarded, and `Read` and `Block` were the same idea | `send` is called directly, and `read` returns `Option<Block>` |
| Visibility and one long function | `UNSCHEDULED`, `WaitMode`, `wait_mode`, `gain_db` and `frames_in` are private. `open_default` has `parse_config` and `ring_buffer_frames` |
| The export did not check the sample rate of a segment | `read_segment_at_rate` and `AudioError::RateMismatch`. Tests in `failure.rs` for both passes |
| `Conversion::convert` and the Ogg encoder moved the pending samples for each chunk | Both process the chunks of the input directly and keep only the remainder |
| `dyn Encode` outside an extension point | `enum Encoder` |
| Constants, why-comments, doc wording, a wrong lint reason in `linear_gain`, the passes of `Loudness` | New constants for the Opus header and the artist. Why-comments for the output rate of LAME, the flush bytes, the serial number, the pages and the drop order of `PartFile`. `Loudness::passes` |
| Tests that could not fail, or raced | See the list below |
| Missing edge tests | Ogg with a partial frame, one sample and no sample. Resampling of 1, 1023, 1024 and 1025 samples and of one-sample pieces. MP3 decode with level, exact length and no segments. Loudness of joined segments and of a silent segment. Commit failure. Seek from `Ended`, to a pending segment and beyond the end. Flush while paused. Reuse of the cache after a failed read. Float, stereo and 24-bit segments |
| Tests of the player and the export | A harness barrier (`barrier`) replaces assertions on state without a barrier. Bounded spin loops (`drain_until_flushed`, `wait_for_flush_request`) replace unbounded loops. The device tests send the event before the command, so the order is fixed. Tests have one behavior each, and the MP3 test decodes the audio |
| The example accepted failed seeks | It counts timeouts, compares each seek with `SEEK_BUDGET` and exits with an error if one seek fails |

The author rejected these findings.

| Finding | Reason |
|---|---|
| No `catch_unwind` around the feed thread | `docs/standards.md` section 6.7 forbids a panic in library code. The `expect` calls in the resampler have a lint reason that names the proven precondition, and a panic stops the thread. A guard would hide a defect |
| A seqlock for `TrackAtomics::restart` | A reader can see a mix of old and new starts for one frame. The UI reads the position in each animation frame, so the effect is one frame of a wrong highlight. The plan names atomics for the starts |
| The field `paused` of `PlayerAtomics` without `is_` | The stage file names the field in the table of atomics |
| An event for a segment that cannot be read | `PlayerEvent` belongs to the plan, and a new variant needs an ADR. The player waits in `Buffering` and logs the error. Stage 06 or 16 can decide the event |
| Opus packets allocate once for each frame | `PacketWriter` takes owned packets, and one allocation of 120 bytes for each 20 ms is the cost of the crate. All other buffers are reused |
| `unwrap_or` in the Ogg encoder for values that cannot overflow | The calls are conversions of values with a known range. A checked constructor for each would add types for one use |
| Progress reaches 1.0 before the commit, and no `sync_all` before the rename | The architecture asks for a temporary path and a rename. The commit error comes after the last report |
| A `Decibels` newtype for `gain_db` | The plan names `f64` in `ExportSummary` |
| The harness drop can wait for the flush limit after a failed test | It happens only after a failure, and the limit is 30 s |

### 2.3 Simplification pass

The author read the complete diff again. It removed `Feed::emit`, the `Read` enum, `Volume::default`, the public `from_repr`, two single-use `pub(crate)` markers, and the helper `fn harness()` of the tests. It moved the shared fixture code of the unit tests, the integration tests, the benchmark and the example into `tests/fixtures/mod.rs`, which each target includes with `#[path]`. The feed loop tests moved from `feed.rs` to `advance.rs` because of the limit of 300 lines for a file.

## 3. Differences from the plan

1. **Errors of `cpal` 0.18.** `cpal` 0.18 has one error type, `cpal::Error`, with an `ErrorKind`. `DeviceConfig`, `BuildStream` and `PlayStream` keep this type, one variant for each operation (decision rule 10). `AudioError` has more variants than the plan, each with data. `SpawnThread` is the error of `Player::start`. `LoudnessMeter` keeps the `ebur128::Error`. `InvalidDeviceConfig`, `UnsupportedFormat` and `RateMismatch` carry values. `Mp3Tag` has no `#[source]`, because `Id3TagError` does not implement `Error` (decision rule 10).
2. **No `SampleRate` argument of `open_stream`.** The player opens the device at the first `play`, and the stream has the default configuration of the device. An argument would have no user. `Player::start` only starts the thread, so no function of the crate returns `NoDevice` or the other device errors. The feed thread logs them and sends `PlayerEvent::DeviceLost`.
3. **`wait_mode` has an enum.** `docs/standards.md` section 18.7 and `max-fn-params-bools = 0` forbid a boolean parameter. `wait_mode(state, work)` takes `Work::Pending` or `Work::Idle`. The table of the stage file is unchanged.
4. **Shutdown of the player.** `FeedCommand::Shutdown` and `Drop for Player` stop and join the thread, because the stream callback holds a sender and the channel never disconnects.
5. **Load flushes.** `Player::load` replaces the track with the same flush handshake as a seek, because the ring buffer holds the audio of the old track. The old track pauses and sends `StateChanged`.
6. **A flush that nobody answers.** The feed thread sends `PlayerEvent::DeviceLost` itself, because the callback that would send it does not run.
7. **`fill` is generic over the output sample type.** `T: Sample + FromSample<f32>`, so a device with `i16`, `u16` or `i32` samples needs no allocation (decision rule 4). Other formats give `AudioError::UnsupportedFormat`.
8. **Live sink.** `LiveSink` sends `LiveEnd` in `Drop`, so a job that stops early completes the partial segment. `finish` has no body.
9. **Constants.** `MP3_BITRATE` and `MP3_LAME_QUALITY` are typed constants of `mp3lame-encoder`. The durations of the feed are `*_MS` constants of the plan. New constants name the flush poll, the Opus header fields, the page length, the serial number and the packet size.
10. **MP3 flush.** The encoder flushes with `FlushGap`. `FlushNoGap` keeps the end of the audio in the encoder, and the decode was 1152 samples too short.
11. **Test files.** Integration tests of the export are in `tests/export/`, a directory, because of the limit of 300 lines. The feed loop tests are in `advance.rs`. The harness for the feed tests is `player/harness.rs`.
12. **Segment reader.** `read_segment` uses `SegmentFile`, because `PeakCache` needs the sample rate of the file.
13. **Features.** The crate enables `fft_resampler` of `rubato`, `std` of `mp3lame-encoder`, and `id3v2`, `mp3` and `ogg` of `symphonia`.
14. **CMake.** The runner images of GitHub for macOS, Ubuntu and Windows have CMake 3.16 or later, so the CI file has no CMake step (decision rule 2).

## 4. Measurements

The stage has two budgets and both need the reference machine alone.

| Budget | Command | Result |
|---|---|---|
| Export of 10 minutes of 24 kHz audio to MP3 with normalization, 3 s or less | `cargo bench -p antenna-audio --bench export` | Pending: measured alone after wave 2 |
| Seek to a stored position, 100 ms or less to the first sample | `cargo run --release -p antenna-audio --example seek_latency` | Pending: measured alone after wave 2 |

## 5. Manual QA

AC-03-27 passed on the reference machine (Apple M5 Pro, 24 GB, default output device).

```
cargo nextest run -p antenna-audio --run-ignored only -E 'test(player_plays_track_when_device_available)'
PASS [   3.135s] antenna-audio::device tests::player_plays_track_when_device_available
Summary 1 test run: 1 passed, 112 skipped
```

Other stages built in parallel during the run. The test passed with 0 underruns and the positions 0, 1 and 2.

## 6. Sign-off

The project owner writes "approved" and the date on each line.

| Criterion | Sign-off |
|---|---|
| AC-03-26 export time, from the command in section 4 | |
| AC-03-27 playback on the default device, from the output in section 5 | |
| AC-03-30 seek time, from the command in section 4 | |

## 7. Findings for later stages

1. **Build path.** The crate `mp3lame-sys` builds LAME with autotools. The build fails when the target directory path has a space, as in `/Users/brian/Develop/Side Projects/Apps/antenna`. The error is `install: target directory ... does not exist`. Set `CARGO_TARGET_DIR` to a path without a space. CI paths have no space. A human must decide a permanent fix, for example a checkout path without a space.
2. **Idle callback.** The output stream keeps running while the track is paused or ended, because the flush handshake needs the callback. The callback writes silence. Stage 16 must check the idle CPU budget of `docs/architecture.md` section 8.
3. **Unreadable segment.** The player waits in `Buffering` when a stored file cannot be read, and logs the error. The apps cannot see the cause. Stage 06 or stage 16 can add an event with an ADR.
4. **Tail delay.** The resampler holds back up to 1023 input samples until the next segment arrives or the track ends. The tail of a segment plays when the next audio arrives.
