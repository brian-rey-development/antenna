# Stage 05. Library

## Goal

`antenna-library` saves documents, derives their status from data, searches and groups them, and keeps the segment store, in the budgets of this stage.

## Context

The library is the memory of Antenna. The Library screen, the recent documents in the sidebar and the Studio of stages 16 and 17 read it. The pipeline of stage 06 writes each synthesized segment to the segment store. The apps record the segment keys and the progress of each document from the job events, because the pipeline knows only the segment store. The player of stage 03 and the export read the stored segments. Because the segment key contains the segment text, a document that the user edits needs synthesis only for the changed segments.

## Read first

1. `CLAUDE.md`
2. `docs/architecture.md` sections 1.1, 6.2, 6.5, 6.7, 7.2, 8 and 13
3. `docs/standards.md` sections 3, 5, 6, 7, 8, 14, 18 and 19
4. `docs/writing.md` section 4 (glossary)
5. `docs/design/screens/biblioteca.png` for the statuses, the filters and the groups

## Scope

The stage can create or change these files only.

- `crates/storage/library/`
- `Cargo.toml` (root), to add the member and the workspace dependencies. The dependencies are `uuid` (features `v7` and `serde`), `jiff` (feature `serde`), `serde`, `toml`, `sha2`, `hound`, `unicode-normalization` and `directories`. The dev-dependencies are `tempfile`, `proptest` and `divan`
- `docs/adr/0010-segment-store.md`

## Deliverables

### Crate layout

```
crates/storage/library/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── error.rs          # LibraryError
│   ├── paths.rs          # data root, document paths, segment paths, ANTENNA_DATA_DIR
│   ├── atomic.rs         # write to a temporary file, then rename, and stale file cleanup
│   ├── document.rs       # DocumentId, DocumentMeta and its TOML form
│   ├── library.rs        # Library: open, create, change, delete, list
│   ├── index.rs          # the in-memory index behind the one RwLock of the workspace
│   ├── status.rs         # Status from data, a pure function
│   ├── search.rs         # text folding and query match, pure functions
│   ├── period.rs         # grouping by period, a pure function of now
│   ├── segments.rs       # SegmentKey, SegmentStore, SegmentWriter
│   └── gc.rs             # deletion of unused segment files and stale temporary files
├── benches/
│   └── library.rs        # divan benchmarks for open and search
└── tests/
    ├── documents.rs      # document round trip, status, recent documents, crash safety
    ├── segments.rs       # segment keys, writer, reader, garbage collection
    └── invariants.rs     # proptest invariants of search and grouping
```

### Constants

```rust
pub const RECENT_LIMIT: usize = 4;
const DATA_DIR_ENV: &str = "ANTENNA_DATA_DIR";
const LIBRARY_DIR_NAME: &str = "library";
const SEGMENTS_DIR_NAME: &str = "segments";
const META_FILE_NAME: &str = "document.toml";
const TEMP_MARKER: &str = ".tmp-";
const STALE_TEMP_AGE: Duration = Duration::from_secs(3_600);
const GC_MIN_AGE: Duration = Duration::from_secs(3_600);
const SEGMENT_KEY_PREFIX_CHARS: usize = 2;
const META_VERSION: u32 = 1;
```

### Public API

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DocumentId(Uuid);                        // UUID v7
impl DocumentId { pub fn uuid(self) -> Uuid; }
impl Display for DocumentId;                        // the hyphenated lowercase UUID text
impl FromStr for DocumentId { type Err = uuid::Error; }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredVoice { pub id: String, pub quality: Quality }   // the Display text of a VoiceId

pub struct SegmentList { pub text_hash: TextHash, pub voice: StoredVoice, pub keys: Vec<SegmentKey>, pub is_complete: bool, pub duration: Duration }

pub struct ExportRecord { pub format: ExportFormat, pub at: Timestamp }

pub struct DocumentMeta {
    pub id: DocumentId,
    pub title: String,
    pub format: TextFormat,
    pub language: Option<Language>,
    pub voice: Option<StoredVoice>,
    pub created: Timestamp,
    pub modified: Timestamp,
    pub opened: Timestamp,
    pub text_hash: TextHash,                        // SHA-256 of the format and the text
    pub segments: Option<SegmentList>,
    pub last_export: Option<ExportRecord>,
}

pub enum Status { Draft, Generating { done: u32, total: u32 }, Ready, Exported(ExportFormat) }
pub enum Filter { All, Ready, Drafts }
pub enum Period { Today, ThisWeek, Month { year: i16, month: i8 } }

pub struct DocumentSummary { pub meta: Arc<DocumentMeta>, pub status: Status }
pub struct Group { pub period: Period, pub documents: Vec<DocumentSummary> }

pub struct Library { /* root, RwLock<Index>, SegmentStore */ }

impl Library {
    pub fn open_default() -> Result<Self, LibraryError>;
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, LibraryError>;
    pub fn skipped(&self) -> &[PathBuf];
    pub fn segments(&self) -> &SegmentStore;

    pub fn create(&self, title: &str, document: &Document, now: Timestamp) -> Result<DocumentId, LibraryError>;
    pub fn load(&self, id: DocumentId) -> Result<(Arc<DocumentMeta>, Document), LibraryError>;
    pub fn save_text(&self, id: DocumentId, document: &Document, now: Timestamp) -> Result<(), LibraryError>;
    pub fn rename(&self, id: DocumentId, title: &str, now: Timestamp) -> Result<(), LibraryError>;
    pub fn set_voice(&self, id: DocumentId, voice: StoredVoice, now: Timestamp) -> Result<(), LibraryError>;
    pub fn set_language(&self, id: DocumentId, language: Option<Language>) -> Result<(), LibraryError>;
    pub fn record_segments(&self, id: DocumentId, text_hash: TextHash, voice: &StoredVoice, keys: Vec<SegmentKey>) -> Result<(), LibraryError>;
    pub fn record_complete(&self, id: DocumentId, text_hash: TextHash, voice: &StoredVoice, duration: Duration) -> Result<(), LibraryError>;
    pub fn record_export(&self, id: DocumentId, format: ExportFormat, now: Timestamp) -> Result<(), LibraryError>;
    pub fn mark_opened(&self, id: DocumentId, now: Timestamp) -> Result<(), LibraryError>;
    pub fn set_progress(&self, id: DocumentId, progress: Option<(u32, u32)>);
    pub fn delete(&self, id: DocumentId) -> Result<(), LibraryError>;

    pub fn build_search_index(&self) -> Result<(), LibraryError>;
    pub fn list(&self, filter: Filter, query: &str, now: &Zoned) -> Vec<Group>;
    pub fn recent(&self) -> Vec<DocumentSummary>;
    pub fn count(&self) -> usize;
    pub fn collect_garbage(&self, now: SystemTime) -> Result<GcReport, LibraryError>;
}

pub fn text_hash(document: &Document) -> TextHash;
pub fn fold(text: &str) -> String;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SegmentKey([u8; 32]);
impl SegmentKey { pub fn new(descriptor: &EngineDescriptor, voice: VoiceId, quality: Quality, text: &str) -> Self; }
impl Display for SegmentKey;                        // 64 lowercase hex characters
impl FromStr for SegmentKey { type Err = LibraryError; }     // LibraryError::InvalidSegmentKey

#[derive(Clone)]
pub struct SegmentStore { /* root */ }
impl SegmentStore {
    pub fn open_default() -> Result<Self, LibraryError>;
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, LibraryError>;
    pub fn contains(&self, key: &SegmentKey) -> bool;
    pub fn path(&self, key: &SegmentKey) -> PathBuf;
    pub fn writer(&self, key: SegmentKey, rate: SampleRate) -> Result<SegmentWriter, LibraryError>;
    pub fn read(&self, key: &SegmentKey) -> Result<StoredSegment, LibraryError>;
    pub fn duration(&self, key: &SegmentKey) -> Result<Duration, LibraryError>;
}

pub struct SegmentWriter { /* key, temporary path, hound::WavWriter */ }
impl SegmentWriter {
    pub fn write(&mut self, samples: &[f32]) -> Result<(), LibraryError>;
    pub fn commit(self) -> Result<Duration, LibraryError>;
}
impl Drop for SegmentWriter;                        // deletes the temporary file if commit did not run

pub struct StoredSegment { pub rate: SampleRate, pub samples: Vec<f32> }
pub struct GcReport { pub deleted_files: u32, pub deleted_bytes: u64 }

pub enum LibraryError {
    NotFound(DocumentId),
    EmptyTitle,
    MetaRead { path: PathBuf, source: toml::de::Error },
    MetaWrite { path: PathBuf, source: toml::ser::Error },
    Wav { path: PathBuf, source: hound::Error },
    Io { path: PathBuf, source: io::Error },
    EmptyText { path: PathBuf, source: CoreError },
    InvalidSegmentKey { text: String },
    NoDataDir,
}
```

`LibraryError` has no `#[non_exhaustive]`. Each variant with a source keeps the concrete error of its library. `NoDataDir` is the error of `open_default` when `ANTENNA_DATA_DIR` is not set and the platform gives no data directory. `InvalidSegmentKey` is the error of `SegmentKey::from_str` for a text that is not 64 lowercase hex characters. It keeps the text, because no library error exists for it. `EmptyText` is the error of `load` for a text file that has no text. It keeps the `CoreError` of the document constructor.

`index.rs` holds the index in a `std::sync::RwLock`. It is the only lock of the workspace that `docs/architecture.md` section 6.7 permits. `clippy::disallowed_types` fires on the `use` line, on the field type and on `RwLock::new`, so `index.rs` starts with the inner attribute `#![expect(clippy::disallowed_types, reason = "the library index is the one permitted lock")]`. No other file names `RwLock`.

`SegmentStore::open` takes the same data root as `Library::open` and uses `<root>/segments`. The CLI and the eval tool open only the segment store. The pipeline gets a clone of the store of the library, so both use one directory.

The apps call `record_segments` when a job sends `Started`, with the text hash and the voice of the job and the keys of the timeline. They call `set_progress` for each `Synthesized` event and `record_complete` with the total duration when the job sends `Finished`. When a job ends, they call `set_progress` with `None`.

`fold` is public because the Voices screen of stage 17 uses the same accent-insensitive match on voice names. No other crate folds text.

`Timestamp` and `Zoned` are the `jiff` types. The library takes `now` as a parameter in each function that needs the time. Thus each test controls the time.

### Data layout

```
<root>/library/<document id>/document.toml
<root>/library/<document id>/text.md       # TextFormat::Markdown
<root>/library/<document id>/text.txt      # TextFormat::Plain
<root>/segments/<first 2 hex characters of the key>/<key>.wav
```

`open_default` uses the `ANTENNA_DATA_DIR` environment variable if it is set. Otherwise it uses `ProjectDirs::from` of `directories` with the three names of `antenna_core::app_dirs`, and its `data_dir()`. A pure function `default_root(env: Option<OsString>, dirs: Option<ProjectDirs>) -> Result<PathBuf, LibraryError>` contains this logic. This is the same data directory that `antenna-models` uses for `models/`.

`document.toml` has a `version` field with the value of `META_VERSION`. Timestamps are RFC 3339 text. The format, the language, the quality, the export format, the text hash and the segment keys are their `Display` text. One private serde helper module in `document.rs` writes each of these types with `Display` and reads it with `FromStr`.

### Atomic writes

`atomic.rs` writes each file in this sequence.

1. Write the bytes to `<final name><TEMP_MARKER><process id>-<counter>` in the same directory.
2. Call `sync_all` on the file.
3. Rename the temporary file to the final name. The rename replaces an old file.

`save_text` writes the text file first and `document.toml` second. If a crash occurs between the two renames, the stored `text_hash` is different from the hash of the text file. The status rules then give `Draft`, so no wrong audio plays.

`collect_garbage(now)` deletes each file in the data root that contains `TEMP_MARKER` and is older than `STALE_TEMP_AGE` at `now`. A younger temporary file can belong to a CLI process that runs now, so the library keeps it. `open` reads no clock and deletes no file.

### Status rules

`status.rs` has one pure function, `status(meta: &DocumentMeta, progress: Option<(u32, u32)>) -> Status`. It applies these rules in sequence and returns the first match.

1. If `progress` is `Some((done, total))`, return `Generating { done, total }`.
2. `Draft` means that not all segments of the current text and the current voice are stored and no job runs. Thus return `Draft` if one of these conditions is true.
   1. `segments` is `None`.
   2. `segments.text_hash` is not equal to `text_hash`.
   3. `voice` is `None`, or `segments.voice` is not equal to `voice`.
   4. `segments.is_complete` is `false`.
3. If `last_export` is `Some` and its time is later than or equal to `modified`, return `Exported(format)`.
4. Return `Ready`.

The progress is in memory only. `set_progress` changes it, and the library does not write it to disk. Thus a crash during a job leaves the status `Draft`.

`record_segments` and `record_complete` compare their text hash with `text_hash` of the document and their voice with `voice` of the document. If one of them is different, the call does nothing and returns `Ok`. `record_complete` also does nothing when `segments.voice` is not its voice. Thus a job that finishes after the user changed the text or the voice does not mark the document complete.

`set_voice` sets `segments` to `None` when the new voice is different from the old voice. Thus the audio of the old voice never gives `Ready`.

`delete` removes the document directory and the index entry. The segment files of the document stay until garbage collection deletes them.

Status comes from `document.toml` only. The library does not check segment files at `open`, so the start time does not depend on the number of segments.

### Search

`search.rs` has two pure functions.

1. `fold` decomposes the text with NFD and removes each character of the Unicode category `Mn`. Then it converts the text to lowercase and collapses each run of whitespace to one space.
2. `matches(query_terms: &[String], title: &str, text: &str) -> bool` is true when each term is a substring of the folded title or of the folded text. The caller folds the terms and the texts one time.

`list` splits the folded query at whitespace. An empty query matches each document.

`open` reads only the `document.toml` files. `build_search_index` reads the text files and stores the folded texts in the index. The app calls it on a background thread after the first frame. Until it completes, `list` matches the query against the titles only. `save_text` updates the folded text of the document in the index.

### Filters, groups and recent documents

1. `Filter::All` keeps each document. `Filter::Ready` keeps `Ready` and `Exported`. `Filter::Drafts` keeps `Draft`.
2. `period.rs` has one pure function, `period_of(modified: Timestamp, now: &Zoned) -> Period`. It converts `modified` to the time zone of `now`. The period is `Today` for the same civil date, `ThisWeek` for the same ISO 8601 week, and `Month` for all other dates.
3. `list` returns the groups in sequence from the newest period. Inside a group, the documents are in sequence from the newest `modified`.
4. `recent` returns the `RECENT_LIMIT` documents with the newest `opened` time.

### Segment store

1. `SegmentKey::new` calculates SHA-256 over five fields in sequence. Each field is its length as a `u64` in little-endian byte order, then its UTF-8 bytes. The fields are `descriptor.id`, `descriptor.version`, the `Display` text of the voice id, the `Display` text of the quality and the segment text. A `debug_assert_eq!` makes sure that `voice.engine()` is `descriptor.id`. The length prefix makes sure that two different field lists cannot give the same bytes.
2. `writer` makes the prefix directory and opens a `hound::WavWriter` on a temporary file. The format is mono, 16-bit integer PCM at `rate`.
3. `SegmentWriter::write` converts each sample with `(sample.clamp(-1.0, 1.0) * PCM_SCALE).round()` to `i16`. `PCM_SCALE` is the constant of `antenna-core`. `antenna-audio` reads the files with the same constant.
4. `commit` finalizes the WAV header, calls `sync_all` and renames the file. If the final file exists already, `commit` deletes the temporary file and keeps the existing file. Engines are deterministic, so both files contain the same audio.
5. `read` converts each `i16` sample back with a division by `PCM_SCALE`.
6. `duration` reads only the WAV header.

### Garbage collection

`collect_garbage` deletes each segment file that no `SegmentList` of a document refers to, and that is older than `GC_MIN_AGE` at `now`. It also deletes the stale temporary files of "Atomic writes". The age rule protects the segments of a job that started before its `record_segments` call, and the segments of a CLI process. The app calls `collect_garbage` on a background thread at start.

### ADR 0010

Write `docs/adr/0010-segment-store.md`. It records why generation and playback are separate and why the segment key contains the text. It also records why the store keeps 16-bit WAV files and why status comes from metadata only. Use the format of the ADRs of stage 01.

## Tasks

1. Add the crate to the workspace members and add the workspace dependencies.
2. Write `error.rs`, `paths.rs` and `atomic.rs`, with unit tests for `default_root` and for the atomic write.
3. Write `document.rs` with the TOML form and a round-trip unit test.
4. Write `status.rs`. Write one unit test for each status rule.
5. Write `search.rs` and `period.rs` with unit tests.
6. Write `segments.rs`. Write unit tests for the key encoding and for the sample conversion.
7. Write `index.rs` and `library.rs`.
8. Write `gc.rs` with the segment rule and the temporary file rule.
9. Write the integration tests in `tests/documents.rs` and `tests/segments.rs`. Each test uses a `tempfile` root.
10. Write the `proptest` invariants in `tests/invariants.rs`.
11. Write the `divan` benchmarks. Generate 1000 documents of 20 KB each in a temporary root, then measure `open` and `list` with a two-term query.
12. Write ADR 0010.
13. Run `cargo xtask check`. Fix each failure.
14. Do the quality gate in `docs/standards.md` section 20.

## Acceptance criteria

| ID | Criterion | Check |
|---|---|---|
| AC-05-01 | All checks pass | `cargo xtask check` exits with code 0 |
| AC-05-02 | A created document loads with the same title, text, format and times | Test `document_round_trips_when_created_and_loaded` |
| AC-05-03 | Document ids sort by creation time | Test `document_ids_sort_by_creation_time` |
| AC-05-04 | An empty title is rejected | Test `create_fails_when_title_is_blank` |
| AC-05-05 | Each status rule gives its status | Tests `status_is_generating_when_progress_set`, `status_is_draft_when_text_changed`, `status_is_draft_when_segments_incomplete`, `status_is_exported_when_export_after_change` and `status_is_ready_when_complete_and_not_exported` |
| AC-05-06 | A change of the text after an export gives `Draft` | Test `status_leaves_exported_when_text_saved` |
| AC-05-07 | A late `record_complete` with an old text hash does not mark the document complete | Test `record_complete_ignored_when_text_hash_differs` |
| AC-05-08 | Progress is in memory only | Test `progress_absent_after_reopen` |
| AC-05-09 | A crash between the text rename and the meta rename gives `Draft` | Test `status_is_draft_when_meta_is_older_than_text` |
| AC-05-10 | `collect_garbage` deletes stale temporary files and keeps young ones, and `open` deletes no file | Tests `gc_deletes_temp_files_when_older_than_one_hour` and `open_keeps_temp_files_when_stale`. The tests set the file times with `File::set_modified` and pass a fixed `now` |
| AC-05-11 | `open` skips a damaged `document.toml` and lists it in `skipped` | Test `open_skips_document_when_meta_is_damaged` |
| AC-05-12 | Search ignores case and accents in the title and the text | Tests `search_matches_when_accents_differ` and `search_matches_text_when_index_built` |
| AC-05-13 | Before `build_search_index`, search matches titles only | Test `search_matches_titles_only_when_index_not_built` |
| AC-05-14 | Each filter keeps the documents of its statuses | Test `filter_keeps_matching_statuses` |
| AC-05-15 | Grouping gives today, this week and months in sequence from the newest | Test `groups_follow_period_rules` with a fixed `Zoned` value |
| AC-05-16 | `recent` returns the four documents with the newest `opened` time | Test `recent_returns_four_newest_opened` |
| AC-05-17 | Fold is idempotent, and a query with more terms never matches more documents | Proptests `fold_is_idempotent` and `more_terms_never_match_more` |
| AC-05-18 | Each document is in exactly one group, and grouping is deterministic | Proptests `each_document_in_one_group` and `grouping_is_deterministic` |
| AC-05-19 | Segment keys change when any of the five fields changes | Test `segment_key_changes_when_any_field_changes` |
| AC-05-20 | Field boundaries cannot collide in a segment key | Test `segment_key_differs_when_text_moves_between_fields` |
| AC-05-21 | A written segment reads back with an error of at most `1 / PCM_SCALE` for each sample | Test `segment_round_trips_when_written_and_read` |
| AC-05-22 | A segment file is not visible before `commit`, and a dropped writer leaves no file | Tests `segment_absent_until_commit` and `dropped_writer_deletes_temp_file` |
| AC-05-23 | `commit` keeps an existing file with the same key | Test `commit_keeps_existing_file_when_key_exists` |
| AC-05-24 | Garbage collection deletes unused old segments and keeps used or young segments | Test `gc_deletes_only_unused_old_segments` |
| AC-05-25 | `open` with 1000 documents takes 100 ms or less in a release build on the reference machine (Apple M5 Pro, 24 GB) **(manual)** | `cargo bench -p antenna-library` median of `open_1000`. Add the result to the stage report |
| AC-05-26 | `list` with a two-term query over 1000 documents of 20 KB takes 16 ms or less on the reference machine (Apple M5 Pro, 24 GB) **(manual)** | `cargo bench -p antenna-library` median of `search_1000`. Add the result to the stage report |
| AC-05-27 | `build_search_index` for 1000 documents of 20 KB takes 1 s or less on the reference machine (Apple M5 Pro, 24 GB) **(manual)** | `cargo bench -p antenna-library` median of `index_1000`. Add the result to the stage report |
| AC-05-28 | The default root uses `ANTENNA_DATA_DIR` when it is set | Tests `default_root_uses_env_when_set` and `default_root_uses_data_dir_when_env_absent` |
| AC-05-29 | `antenna-library` depends only on `antenna-core` in the workspace | `cargo xtask lint-repo` passes |
| AC-05-30 | ADR 0010 exists and has 60 lines or less | `wc -l docs/adr/0010-segment-store.md` |
| AC-05-31 | `SegmentStore::open` and `Library::segments` with the same root use the same files | Test `segment_store_matches_library_store_when_root_same` |
| AC-05-32 | `delete` removes the document directory and the index entry, and keeps the segment files | Test `delete_removes_document_when_id_exists` |
| AC-05-33 | `set_voice` with a different voice clears the segment list | Test `set_voice_clears_segments_when_voice_changes` |
| AC-05-34 | A document whose segments have a different voice has the status `Draft` | Test `status_is_draft_when_segments_voice_differs` |
| AC-05-35 | `record_segments` and `record_complete` with a different text hash or voice change nothing | Tests `record_segments_ignored_when_voice_differs` and `record_complete_ignored_when_voice_differs` |
| AC-05-36 | `rename` rejects a blank title and keeps the old title | Test `rename_fails_when_title_is_blank` |
| AC-05-37 | `set_language` stores the language, and it survives a reopen | Test `set_language_persists_when_library_reopened` |
| AC-05-38 | `record_export` stores the format and the time, and they survive a reopen | Test `record_export_persists_when_library_reopened` |
| AC-05-39 | `DocumentId` round trips through its text | Test `document_id_round_trips_when_text_parsed` |
| AC-05-40 | `default_root` returns `NoDataDir` when the environment variable and the platform directory are absent | Test `default_root_fails_when_env_and_data_dir_absent` |
| AC-05-41 | `SegmentKey` round trips through its text and rejects an invalid text | Tests `segment_key_round_trips_when_text_parsed` and `segment_key_fails_when_text_not_64_hex_characters` |
| AC-05-42 | Only `index.rs` names `RwLock` | `rg -l RwLock crates/storage/library/src` prints only `crates/storage/library/src/index.rs` |

## Decision rules

1. If `open` misses AC-05-25, profile it. Make sure that it reads each `document.toml` one time and reads no text file. Do not add a database. If the budget still fails, stop and write a "Blocked" section with the profile.
2. If `list` misses AC-05-26, join the folded title and the folded text of each document in one `String` with a newline. Search that string once for each term. Do not add a search crate.
3. If `std::fs::rename` cannot replace an existing file on a platform, delete the old file first only for `document.toml` and text files. For segment files, keep the existing file as `commit` defines.
4. If a document directory has no `document.toml`, skip it and list it in `skipped`. Do not delete it.
5. If `toml` cannot read a field of an older `META_VERSION`, stop and write a "Blocked" section. This stage writes version 1 only, so the case cannot occur in this stage.
6. The CLI and the eval tool use the segment store without documents. Their segments become unused, and garbage collection deletes them after `GC_MIN_AGE`. This is the expected behavior.

## Out of scope

1. The Library screen and the sidebar. Stages 16 and 17 build them.
2. The language of a document. The app identifies it with `antenna-text` and stores it with `set_language`. This stage stores the value only.
3. Version history, sync between devices and import of PDF, EPUB or URL.
4. Settings. The desktop app of stage 18 stores them.
5. Any change to `antenna-core`.
