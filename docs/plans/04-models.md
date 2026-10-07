# Stage 04. Models

## Goal

`antenna-models` installs the artifacts of a voice as local files with a correct SHA-256. It has progress, resume, byte ranges, retries and cancellation, and its tests run offline.

## Context

The first use of a voice downloads gigabytes. A corrupted file, a full disk or a half-finished download must never reach an engine. Stage 06 calls `ModelStore::ensure` before it loads an engine and converts the progress into `JobEvent::Downloading`. Stage 07 adds `antenna models list` and `antenna models pull`. Stage 08 installs the Whisper files of `antenna-review` with the same `ensure`. The nightly CI job of stage 09 keeps the model store in a CI cache with `ANTENNA_MODEL_DIR`. Stage 14 needs byte-range artifacts, because the Magpie G2P dictionaries exist only inside an uncompressed `.nemo` archive of 1.47 GB. The Models screen of stage 17 and the `models` command of stage 07 use `is_installed`, `is_engine_installed`, `engine_download_bytes`, `keep_artifacts`, `remove_engine`, `engine_disk_usage` and `disk_usage`. Both apps call these functions, so they agree on the installed state and the size of an engine.

## Read first

1. `CLAUDE.md`
2. `docs/architecture.md` sections 5.2, 7.1, 9, 10 and 13
3. `docs/standards.md` sections 5, 6, 7, 14, 18 and 19
4. The `ureq` 3 documentation of `Agent`, `config::Config`, `http::Request` and `Body::as_reader`
5. The `fs4` documentation of `available_space` and `FileExt::lock_exclusive`

## Scope

The stage can create or change these files only.

- `crates/storage/models/`
- `Cargo.toml` (root), to add the member and the workspace dependencies. The dependencies are `ureq` (default features off, feature `rustls`), `sha2`, `directories` and `fs4`. The dev-dependencies are `tempfile` and `tiny_http`
- `docs/adr/0008-byte-range-artifacts.md`

## Deliverables

### Crate layout

```
crates/storage/models/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── error.rs          # ModelError
│   ├── store.rs          # ModelStore: the public entry point
│   ├── usage.rs          # installed state, disk usage and removal
│   ├── layout.rs         # local paths of artifacts, partial files, `.verified` records and locks
│   ├── source/
│   │   ├── mod.rs        # Source and the internal Fetch trait
│   │   ├── hub.rs        # HTTP download with ureq, ranges and resume
│   │   └── directory.rs  # copy from a local directory with the store layout
│   ├── verify.rs         # streaming SHA-256 and the `.verified` record
│   ├── retry.rs          # retry delays and the cancellable wait
│   └── progress.rs       # DownloadProgress and the throttled reporter
└── tests/
    ├── store.rs          # behavior tests with the directory source
    └── hub/
        ├── main.rs       # behavior tests with the test HTTP server
        └── server.rs     # test HTTP server on 127.0.0.1 with tiny_http
```

The test server is a module of the `hub` test target, so `tests/hub/main.rs` declares `mod server;`. A test target with more than one file is a directory with `main.rs`, as `docs/standards.md` section 3.2 defines.

### Public API

```rust
pub struct ModelStore { root: PathBuf, source: Source, retry_delays: [Duration; 3] }

impl ModelStore {
    pub fn open_default() -> Result<Self, ModelError>;
    pub fn open(root: impl Into<PathBuf>, source: Source) -> Result<Self, ModelError>;
    pub fn root(&self) -> &Path;
    pub fn has_artifact(&self, artifact: &Artifact) -> bool;
    pub fn is_installed(&self, descriptor: &EngineDescriptor, voice: &VoiceDescriptor, quality: Quality) -> bool;
    pub fn is_engine_installed(&self, descriptor: &EngineDescriptor, quality: Quality) -> bool;
    pub fn missing_bytes(&self, artifacts: impl IntoIterator<Item = &'static Artifact>) -> u64;
    pub fn engine_disk_usage(&self, descriptor: &EngineDescriptor) -> u64;
    pub fn disk_usage(&self) -> Result<u64, ModelError>;
    pub fn remove_engine(&self, descriptor: &EngineDescriptor, keep: impl IntoIterator<Item = &'static Artifact>) -> Result<u64, ModelError>;
    pub fn with_retry_delays(self, delays: [Duration; 3]) -> Self;
    pub fn ensure(
        &self,
        artifacts: impl IntoIterator<Item = &'static Artifact>,
        progress: &(dyn Fn(DownloadProgress) + Sync),
        cancel: &AtomicBool,
    ) -> Result<ModelFiles, ModelError>;
}

pub fn voice_artifacts(
    descriptor: &EngineDescriptor,
    voice: &VoiceDescriptor,
    quality: Quality,
) -> Vec<&'static Artifact>;

pub fn engine_artifacts(descriptor: &EngineDescriptor) -> Vec<&'static Artifact>;

pub fn engine_download_bytes(descriptor: &EngineDescriptor, quality: Quality) -> u64;

pub fn keep_artifacts<'a>(
    other_engines: impl IntoIterator<Item = &'a EngineDescriptor>,
    extra: impl IntoIterator<Item = &'static Artifact>,
) -> Vec<&'static Artifact>;

pub enum Source {
    Hub { endpoint: Option<String> },
    Directory(PathBuf),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DownloadProgress { pub done_bytes: u64, pub total_bytes: u64 }

pub enum ModelError {
    DiskSpace { root: PathBuf, needed_bytes: u64, available_bytes: u64 },
    Hash { artifact: &'static str, expected: &'static str, actual: String },
    Size { artifact: &'static str, expected_bytes: u64, actual_bytes: u64 },
    Network { artifact: &'static str, attempts: u32, source: ureq::Error },
    NotFound { artifact: &'static str },
    RangeUnsupported { artifact: &'static str },
    DuplicateKey { key: &'static str },
    Busy { artifact: &'static str },
    Io { path: PathBuf, source: io::Error },
    NoDataDir,
    Cancelled,
}
```

`ModelError` has no `#[non_exhaustive]`. The `artifact` field is the `Artifact::key`. `NoDataDir` is the error of `open_default` when `ANTENNA_MODEL_DIR` is not set and the platform gives no data directory. `Network` keeps the `ureq` error of the last attempt as its source.

`voice_artifacts` returns the artifacts of `descriptor.variants.get(quality)`, then the artifacts of the voice. It is the only function that joins these two lists. Stages 06, 07, 17 and 18 call it. `engine_artifacts` returns the artifacts of the three variants and of all voices, with no duplicates. `ModelFiles` maps each `Artifact::key` to its local path. `check_descriptor` of stage 01 makes keys unique in each variant plus voice set. `ensure`, `missing_bytes` and `remove_engine` take any iterator of `&'static Artifact`, for example `voice_artifacts(..)`, `descriptor.variants.get(quality).artifacts.iter()` or a chain of both. Thus a caller never builds a temporary `Vec` to change the shape of a list. `ensure` returns `ModelError::DuplicateKey` when two artifacts of one request have the same key.

`ensure` does not know the kind of model. The Whisper files of `antenna-review` are a `&'static [Artifact]` without a descriptor. The caller passes `artifacts.iter()`, and `ensure` installs them the same way.

### Installed state, disk usage and removal

1. `has_artifact` is true when the file and its `.verified` record exist and the file size is equal to `Extent::bytes()`.
2. `is_installed` is true when `has_artifact` is true for each artifact of `voice_artifacts(descriptor, voice, quality)`.
3. `missing_bytes` is the sum of `Extent::bytes()` of the artifacts that `has_artifact` does not find, minus the length of their `.part` files. `ensure` uses it for the disk space check, and the apps use it to show the download size.
4. `engine_disk_usage` is the sum of `Extent::bytes()` of the artifacts of `engine_artifacts(descriptor)` that `has_artifact` finds. It reads no directory, so the Models screen can call it in each frame.
5. `disk_usage` walks `<root>` one time and adds the size of each file, `.part` files included.
6. `remove_engine` deletes the local file, the `.verified` record and the `.part` file of each artifact of `engine_artifacts(descriptor)`. It keeps each artifact whose local file is also the local file of an artifact in `keep`. The caller gives `keep_artifacts` of the other registered engines and of the review model.
7. Before it deletes an artifact, `remove_engine` takes the lock of that artifact with `try_lock_exclusive`. If a download holds the lock, it returns `ModelError::Busy` and deletes nothing more.
8. After the deletion, `remove_engine` deletes each directory that is empty. It returns the number of bytes that it deleted.
9. `is_engine_installed` is true when `is_installed` is true for each voice of the engine at `quality`.
10. `engine_download_bytes` is the sum of `Extent::bytes()` of the artifacts of the variant of `quality` and of all voices, with each local file counted one time. It is the full download size of the engine at that quality, also when some files are installed.
11. `keep_artifacts` returns `engine_artifacts` of each engine in `other_engines`, then the artifacts of `extra`. The caller gives the review model artifacts in `extra`.

### Constants

```rust
const HUB_ENDPOINT: &str = "https://huggingface.co";
const MODELS_DIR_NAME: &str = "models";
const PARTIAL_SUFFIX: &str = ".part";
const VERIFIED_SUFFIX: &str = ".verified";
const LOCK_SUFFIX: &str = ".lock";
const MODEL_DIR_ENV: &str = "ANTENNA_MODEL_DIR";
const DISK_MARGIN_BYTES: u64 = 1_073_741_824;
const RETRY_DELAYS: [Duration; 3] = [Duration::from_secs(1), Duration::from_secs(4), Duration::from_secs(16)];
const CANCEL_POLL: Duration = Duration::from_millis(100);
const PROGRESS_INTERVAL: Duration = Duration::from_millis(100);
const READ_BLOCK_BYTES: usize = 65_536;
const HASH_BUFFER_BYTES: usize = 1_048_576;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const READ_TIMEOUT: Duration = Duration::from_secs(30);
```

### Store layout

```
<root>/
├── <owner>/<name>/<revision>/<local name>            # the installed artifact
├── <owner>/<name>/<revision>/<local name>.verified   # the SHA-256 hex of the installed file
├── <owner>/<name>/<revision>/<local name>.part       # a download in progress
└── <owner>/<name>/<revision>/<local name>.lock       # the lock file of the artifact
```

`<owner>/<name>` is the `Artifact::repo`. For `Extent::Whole`, `<local name>` is `Artifact::path`. For `Extent::Range`, `<local name>` is `<path>@<offset>+<bytes>`. Thus two ranges of one archive have different local files, and the same range in two engines has one local file.

`open_default` uses the `ANTENNA_MODEL_DIR` environment variable if it is set. Otherwise it uses `ProjectDirs::from` of `directories` with the three names of `antenna_core::app_dirs`, then its `data_dir()`, then `models`. A pure function `default_root(env: Option<OsString>, dirs: Option<ProjectDirs>) -> Result<PathBuf, ModelError>` contains this logic. Thus a test can check it without a change to the environment.

### `ensure` sequence

1. If two artifacts have the same key, return `ModelError::DuplicateKey`.
2. Find the artifacts that are not installed. An artifact is installed when its file and its `.verified` record exist and the file size is equal to `Extent::bytes()`.
3. If all artifacts are installed, return the `ModelFiles` immediately.
4. Calculate `missing_bytes` of the artifacts. Get the available space of `<root>` with `fs4::available_space`. If the available space is less than the sum plus `DISK_MARGIN_BYTES`, return `ModelError::DiskSpace`.
5. Do these steps for each missing artifact, in sequence.
   1. Take an exclusive lock on its `.lock` file with `fs4::FileExt::try_lock_exclusive`. If another process or thread holds the lock, wait `CANCEL_POLL` with the cancellable wait of `retry.rs` and try again. If `cancel` is `true` during this wait, return `ModelError::Cancelled`. After the lock, check again if the artifact is installed. If it is installed, release the lock and continue with the next artifact.
   2. Fetch the bytes into the `.part` file. The fetch continues from the current length of the `.part` file.
   3. If the fetch fails with a transient error, wait for the next delay of the retry delays, then try again. If the `.part` file grew during the attempt, the count of attempts starts again. After four consecutive attempts without progress (the first attempt and three retries), return `ModelError::Network` with `attempts: 4`.
   4. Make sure that the size is equal to `Extent::bytes()`. If not, delete the `.part` file and return `ModelError::Size`.
   5. Calculate the SHA-256 with a buffer of `HASH_BUFFER_BYTES`. If it is different from `Artifact::sha256`, delete the `.part` file and return `ModelError::Hash`.
   6. Rename the `.part` file to its final path. Write the `.verified` record. Release the lock.
6. Read `cancel` after each block of `READ_BLOCK_BYTES` and during each retry wait. If `cancel` is `true`, keep the `.part` file for the next resume, release the lock and return `ModelError::Cancelled`.
7. Return the `ModelFiles`.

The retry delays are `RETRY_DELAYS` by default. `with_retry_delays` replaces them, and the tests use `[Duration::ZERO; 3]`. The retry wait calls `std::thread::park_timeout` with `CANCEL_POLL` until the delay ends or `cancel` is `true`.

A transient error is a connection error, a timeout or an HTTP status 408, 429, 500, 502, 503 or 504. A 401, 403 or 404 status returns `ModelError::NotFound` without a retry.

### Sources

`source/mod.rs` defines a `pub(crate) trait Fetch` with one method.

```rust
pub(crate) trait Fetch {
    fn fetch(&self, artifact: &'static Artifact, partial: &Path, progress: &dyn Fn(u64), cancel: &AtomicBool) -> Result<(), FetchError>;
}
```

The `u64` of `progress` is the number of bytes that this call of `fetch` wrote to the `.part` file so far. It does not count the bytes that the `.part` file held before the call.

`FetchError` has three variants, `Transient(ureq::Error)`, `Permanent(ModelError)` and `Cancelled`. The two implementations are `HubFetch` and `DirectoryFetch`.

`HubFetch` uses one `ureq::Agent` with `CONNECT_TIMEOUT`, `READ_TIMEOUT` and the user agent `antenna/<crate version>`. It does these steps.

1. Build the URL `<endpoint>/<repo>/resolve/<revision>/<path>`. The endpoint is `HUB_ENDPOINT` unless `Source::Hub` gives a different one.
2. Let `have` be the length of the `.part` file. Let `start` be `0` for `Extent::Whole` and `offset` for `Extent::Range`. Let `end` be `start + bytes - 1`.
3. If `have` is greater than 0 or the extent is a range, send the header `Range: bytes=<start + have>-<end>`.
4. If the response status is 206, append the body to the `.part` file.
5. If the response status is 200 and the request had a `Range` header, the server ignored the range. For `Extent::Whole`, truncate the `.part` file and write the full body. For `Extent::Range`, return `Permanent(ModelError::RangeUnsupported)`.
6. Read the body in blocks of `READ_BLOCK_BYTES`. After each block, call `progress` with the byte count of this call and read `cancel`.

`DirectoryFetch` copies the bytes of the extent from `<directory>/<owner>/<name>/<revision>/<path>` into the `.part` file in blocks of `READ_BLOCK_BYTES`. It reads `cancel` after each block. The tests of this stage and of stage 06 use it, so they run without a network.

### Progress

`progress.rs` converts the byte counts of each file into one `DownloadProgress` for the full request. `total_bytes` is the sum from step 4 of `ensure`. `done_bytes` is the bytes of the completed artifacts plus the bytes of the current file. The reporter calls the `progress` callback at most once in each `PROGRESS_INTERVAL`, and always once at the end with `done_bytes == total_bytes`. `done_bytes` never decreases, because the reporter keeps the maximum value.

The reporter takes the time of each report as an `Instant` parameter. `ensure` gives `Instant::now()`. The unit tests give `Instant` values that they calculate from one start value, so the tests do not depend on the clock.

### Test HTTP server

`tests/hub/server.rs` starts a `tiny_http` server on `127.0.0.1:0` in a thread. It serves files from a temporary directory with the path layout of the hub. It supports these behaviors, which a test selects for each path.

1. Normal responses with 200, and 206 for a valid `Range` header.
2. A fixed number of 503 responses before a normal response.
3. A 404 response.
4. A 200 response that ignores the `Range` header.
5. A 302 redirect to a second path on the same server.
6. A response that closes the connection after a fixed number of bytes.

The server counts the requests for each path, so a test can assert the number of attempts.

### ADR 0008

Write `docs/adr/0008-byte-range-artifacts.md`. It records why `Artifact` has `Extent::Range`, why the store uses HTTP `Range` requests and resume, and why `hf-hub` is not used. Use the format of the ADRs of stage 01.

## Tasks

1. Add the crate to the workspace members and add the workspace dependencies.
2. Write `error.rs` and `layout.rs`, and the unit tests of `layout.rs`.
3. Write `verify.rs` and its unit tests with known SHA-256 values.
4. Write `progress.rs` and its unit tests.
5. Write `retry.rs` with the cancellable wait.
6. Write `source/mod.rs` and `source/directory.rs`.
7. Write `tests/hub/server.rs`.
8. Write `source/hub.rs`.
9. Write `store.rs` with `ensure`.
10. Write `usage.rs` with the logic of `has_artifact`, `is_installed`, `engine_disk_usage`, `disk_usage` and `remove_engine`. `ModelStore` calls it. Add `engine_artifacts` to `layout.rs`.
11. Write `tests/store.rs`. Each test makes a temporary root and a temporary source directory with small artifact files and their real SHA-256 values.
12. Write `tests/hub/main.rs` with the test HTTP server.
13. Write ADR 0008.
14. Run `cargo xtask check`. Fix each failure.
15. Do the quality gate in `docs/standards.md` section 20.

## Acceptance criteria

| ID | Criterion | Check |
|---|---|---|
| AC-04-01 | All checks pass | `cargo xtask check` exits with code 0 |
| AC-04-02 | `ensure` installs a missing artifact and returns its path for its key | Test `ensure_installs_artifact_when_source_has_it` |
| AC-04-03 | `ensure` does not fetch an installed artifact | Test `ensure_skips_fetch_when_artifact_installed` |
| AC-04-04 | A hash mismatch deletes the file and returns `ModelError::Hash` | Test `ensure_fails_and_deletes_when_hash_differs` |
| AC-04-05 | A size mismatch deletes the file and returns `ModelError::Size` | Test `ensure_fails_and_deletes_when_size_differs` |
| AC-04-06 | An installed file with a changed size is not installed any more, and `ensure` fetches it again | Test `ensure_fetches_again_when_installed_file_truncated` |
| AC-04-07 | Not enough disk space stops `ensure` before any fetch | Test `ensure_fails_when_disk_space_insufficient` with an artifact of `u64::MAX / 2` bytes |
| AC-04-08 | Transient failures without progress retry three times, then return `ModelError::Network` with `attempts: 4` | Tests `ensure_succeeds_when_server_fails_three_times` and `ensure_fails_when_server_fails_four_times`, with `with_retry_delays([Duration::ZERO; 3])` |
| AC-04-09 | A permanent failure does not retry | Test `ensure_does_not_retry_when_not_found`. The server counts 1 request |
| AC-04-10 | Cancellation in the middle of a file stops after one more block or less and keeps the `.part` file | Test `ensure_stops_after_one_block_when_cancelled` |
| AC-04-11 | A download continues from the `.part` file with a `Range` header | Test `ensure_resumes_when_partial_file_exists`. The server receives `Range: bytes=<have>-<end>` |
| AC-04-12 | A broken connection resumes on the retry and the result has the correct SHA-256 | Test `ensure_resumes_when_connection_breaks` |
| AC-04-13 | An `Extent::Range` artifact downloads only its bytes and has the correct SHA-256 | Test `ensure_downloads_only_range_when_extent_is_range` |
| AC-04-14 | A server that ignores `Range` gives `RangeUnsupported` for a range artifact | Test `ensure_fails_when_server_ignores_range_for_extent_range` |
| AC-04-15 | A server that ignores `Range` on a resume restarts the whole file | Test `ensure_restarts_when_server_ignores_range_on_resume` |
| AC-04-16 | A redirect keeps the `Range` header | Test `ensure_keeps_range_when_redirected` |
| AC-04-17 | Two range artifacts of one archive have different local files | Test `layout_separates_ranges_of_one_file` |
| AC-04-18 | Progress increases, never decreases, and ends at the total | Test `progress_is_monotonic_and_complete` |
| AC-04-19 | Two artifacts with the same key return `ModelError::DuplicateKey` | Test `ensure_fails_when_keys_collide` |
| AC-04-20 | A file is never visible at its final path before its hash check passes | Test `final_path_absent_until_hash_checked` |
| AC-04-21 | The default root uses `ANTENNA_MODEL_DIR` when it is set | Tests `default_root_uses_env_when_set` and `default_root_uses_data_dir_when_env_absent` |
| AC-04-22 | `voice_artifacts` returns the variant artifacts of the quality, then the voice artifacts | Test `voice_artifacts_lists_variant_then_voice` |
| AC-04-23 | The tests make no request to a host other than `127.0.0.1` **(manual)** | On the reference machine (Apple M5 Pro, 24 GB), run `networksetup -setairportpower en0 off` and remove any network cable. Make sure that `curl -sI https://huggingface.co` fails. Run `cargo nextest run -p antenna-models`. All tests pass. Run `networksetup -setairportpower en0 on` |
| AC-04-24 | `antenna-models` depends only on `antenna-core` in the workspace | `cargo xtask lint-repo` passes |
| AC-04-25 | ADR 0008 exists and has 60 lines or less | `wc -l docs/adr/0008-byte-range-artifacts.md` |
| AC-04-26 | `is_installed` is true only when each artifact of the variant and the voice is installed | Tests `is_installed_true_when_variant_and_voice_installed` and `is_installed_false_when_variant_missing` |
| AC-04-27 | `remove_engine` deletes the files of the engine and keeps each file in `keep` | Test `remove_engine_keeps_artifact_when_listed_in_keep` |
| AC-04-28 | `remove_engine` stops when a download holds a lock | Test `remove_engine_fails_when_artifact_is_locked` |
| AC-04-29 | `remove_engine` deletes empty directories and returns the deleted bytes | Test `remove_engine_returns_deleted_bytes_and_prunes_directories` |
| AC-04-30 | `disk_usage` counts installed and partial files | Test `disk_usage_counts_partial_files` |
| AC-04-31 | `engine_disk_usage` counts only installed artifacts of the engine | Test `engine_disk_usage_ignores_missing_artifacts` |
| AC-04-32 | `ensure` installs a slice of artifacts without a descriptor | Test `ensure_installs_artifacts_without_descriptor` |
| AC-04-33 | `missing_bytes` counts the bytes of the artifacts that are not installed, minus the bytes of their `.part` files | Test `missing_bytes_subtracts_partial_files` |
| AC-04-34 | Two concurrent `ensure` calls for one artifact fetch it one time. The second call waits on the lock and then finds the artifact installed | Test `ensure_waits_on_lock_when_other_call_fetches`. The server counts 1 request |
| AC-04-35 | The progress reporter calls the callback at most once in each `PROGRESS_INTERVAL`, and once at the end | Unit test `progress_throttles_reports_when_interval_not_elapsed` with calculated `Instant` values |
| AC-04-36 | `ensure` returns `Cancelled` while it waits on the lock of another call | Test `ensure_returns_cancelled_when_waiting_on_lock` holds the lock in the test, sets `cancel` and gets `ModelError::Cancelled` |
| AC-04-37 | `is_engine_installed` is true only when each voice of the engine is installed at the quality | Tests `engine_is_installed_when_all_voices_installed` and `engine_is_not_installed_when_one_voice_missing` |
| AC-04-38 | `engine_download_bytes` counts the variant and all voice artifacts, with each local file one time | Test `engine_download_bytes_counts_shared_file_once` |
| AC-04-39 | `keep_artifacts` returns the artifacts of the other engines and of `extra` | Test `keep_artifacts_includes_other_engines_and_extra` |
| AC-04-40 | `default_root` returns `NoDataDir` when the environment variable and the platform directory are absent | Test `default_root_fails_when_env_and_data_dir_absent` |

## Decision rules

1. If `ureq` drops the `Range` header on a redirect, turn off automatic redirects in the agent. Follow up to 5 redirects in `HubFetch` and send the `Range` header on each request.
2. If `fs4` cannot get the available space of a root that does not exist yet, create the root first.
3. If a lock file stays after a crash, do nothing. The operating system releases `fs4` locks when the process ends.
4. If a `.part` file is longer than `Extent::bytes()`, delete it and start again from zero.
5. If `remove_engine` finds a file that is not in `engine_artifacts`, do not delete it. Only declared artifacts are removed.
6. Do not use `hf-hub`. The store has one download path and one retry policy, as `docs/architecture.md` section 7.1 defines.

## Out of scope

1. The Models screen. Stage 17 builds it.
2. Model updates to a new revision. A new revision is a new artifact.
3. Authentication tokens. All selected models are public.
4. Any change to `antenna-core`.
