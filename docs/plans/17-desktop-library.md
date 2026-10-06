# Stage 17. Desktop Library, Voices and Models

## Goal

The Library, Voices and Models screens of `antenna-desktop` show real data with search, preview, download and removal. The library search meets its 16 ms budget with 1000 documents.

## Context

Stage 16 built the shell and the Studio. This stage adds the three screens that manage documents, voices and models. After this stage, the user can find each saved document, compare voices before a long synthesis, and control the disk space that the models use. Stage 18 adds the Settings screen. Stage 19 repeats the QA steps that need a real engine.

## Read first

1. `CLAUDE.md`
2. `docs/architecture.md` sections 1.1, 4, 6.1, 7.1, 7.2, 8 and 12
3. `docs/standards.md` sections 5, 7, 13, 14, 18 and 19
4. `docs/writing.md` section 4 (glossary)
5. `docs/design/screens/biblioteca.png`, `docs/design/screens/voces.png` and `docs/design/screens/modelos.png`, and the pages with the same names in `docs/design/pages/`
6. `docs/design/tokens.md`, `docs/plans/16-desktop-studio.md`, the merged code of `apps/desktop` and the component table of stage 15
7. The public API of `antenna-audio`, `antenna-library`, `antenna-models`, `antenna-pipeline`, `antenna-engine-registry` and `antenna-review` as merged by stages 03, 04, 05, 06 and 08

## Scope

The stage can create or change these files only.

- `apps/desktop/src/main.rs` (the `cmd-k` binding)
- `apps/desktop/src/state/`
- `apps/desktop/src/settings/`
- `apps/desktop/src/strings/`
- `apps/desktop/src/shell/`
- `apps/desktop/src/screens/`, except `screens/studio/`
- `apps/desktop/benches/`
- `apps/desktop/Cargo.toml`
- `Cargo.toml` (`sysinfo` and `memory-stats` in `[workspace.dependencies]`)

## Deliverables

### Files

```
apps/desktop/src/
├── state/
│   ├── library_view.rs   # LibraryView: query, filter, the rows of the last Library::list call
│   ├── voices_view.rs    # VoicesView: language, query, the matched voices
│   ├── preview.rs        # Preview: the voice preview job and track
│   ├── downloads.rs      # Downloads: the progress and the cancel flag of each download, keyed by ModelRef
│   ├── models_view.rs    # ModelsView: rows and the removal confirmation
│   └── system.rs         # SystemInfo: device, chip name, total memory, resident memory, disk use
├── strings/
│   └── relative_time.rs  # relative time text in English and Spanish
└── screens/
    ├── library/
    │   ├── mod.rs        # Library screen
    │   ├── header.rs     # title, count, search field, filter, New button
    │   └── table.rs      # column header, period groups, rows
    ├── voices/
    │   ├── mod.rs        # Voices screen
    │   ├── header.rs     # title, count, search field, language tabs
    │   └── grid.rs       # voice cells
    └── models/
        ├── mod.rs        # Models screen
        ├── summary.rs    # acceleration, memory, disk
        └── list.rs       # installed and available rows
apps/desktop/benches/
└── library_search.rs     # divan benchmark of Library::list, the call of each LibraryView refresh
```

### Consumed API

If a name or a signature is different in the merged code, use the merged code and record the difference in the PR.

| Crate | Items |
|---|---|
| `antenna-library` | `Library::list`, `recent`, `count`, `create`, `load`, `set_voice`, `set_progress`, `build_search_index`, `fold`, `Filter`, `Group`, `Period`, `DocumentSummary`, `Status` |
| `antenna-models` | The `ModelStore` methods `ensure` with progress and cancel, `is_installed`, `is_engine_installed`, `has_artifact`, `missing_bytes`, `remove_engine`, `engine_disk_usage`, `disk_usage` and `root`. The functions `voice_artifacts`, `engine_artifacts`, `engine_download_bytes` and `keep_artifacts` |
| `antenna-engine-registry` | `Registry::factories`, `engine`, `voices`, `voice`, `default_voice` |
| `antenna-review` | `WHISPER_SMALL` (name, summary, license, artifacts), `device_label`, `select_device` |
| `antenna-pipeline` | `Pipeline::timeline`, `Pipeline::start`, `Timeline::stored`, `JobEvent` |
| `antenna-audio` | `Player::load`, `TrackHandle::play`, `pause`, `live_output`, `Track::new` |

### New actions

```rust
pub(crate) enum Action {
    // the actions of stage 16, and:
    PlayDocument(DocumentId),
    Preview(VoiceId),
    DownloadModel(ModelRef),
    CancelDownload(ModelRef),
    RemoveEngine(EngineId),
    SetDefaultEngine(EngineId),
    Search(String),
    SetFilter(Filter),
    SetVoicesLanguage(Language),
    SearchVoices(String),
}
```

`AppState` stays the only owner of the player, the library, the pipeline and the model store. The screens send these actions and read `LibraryView`, `VoicesView`, `ModelsView` and `Downloads`. `state/view.rs` also re-exports `Filter`, `Group`, `Period`, `Status` and `DocumentSummary`, so the screens name them through `crate::state`.

```rust
pub(crate) enum ModelRef {
    Engine(EngineId, Quality),           // all artifacts of the engine at the quality, from engine_artifacts
    Voice(VoiceId, Quality),             // the artifacts of one voice at the quality, from voice_artifacts
    Review,                              // the artifacts of WHISPER_SMALL
}
```

`Downloads` is the only download state of the app. Each download of the Voices screen, the Models screen and the review toggle of stage 18 sends `Action::DownloadModel` and reads its progress from `Downloads`. `AppState` runs each download with `ModelStore::ensure` on a background thread and removes the entry when the download ends.

### Library screen

The layout follows `docs/design/screens/biblioteca.png`.

1. **Header.** The title "Library" and the document count. The `SearchField` of `LAYOUT_SEARCH_LIBRARY` with the `⌘K` hint. The `Segmented` filter with "All", "Ready" and "Drafts". The "New" primary `Button`, which sends `Action::NewDocument` and `Action::Navigate(Screen::Studio)`.
2. **Columns.** `LAYOUT_LIBRARY_COLUMNS` in this sequence. Play button, Name, Voice (`AVATAR_TABLE` avatar and name, or "No voice"), Language (upper-case code), Duration (`DATA`, `--:--` when unknown), Status, Modified.
3. **Groups.** "Today", "This week", then one group for each month, from the `Group` values of `Library::list` with the current date. Each group label is a `SectionLabel`.
4. **Status.** The `StatusIndicator` component. `Generating` shows the progress ring and the percent of synthesized segments. `Ready` shows the check. `Exported` shows the arrow and the format. `Draft` shows the dashed ring.
5. **Modified.** The relative time of `strings/relative_time.rs`.
6. **Rows.** Each row is a `TableRow`. A click on a row sends `Action::OpenDocument` and `Action::Navigate(Screen::Studio)`. The play button sends `Action::PlayDocument`. It is enabled only for the statuses `Ready` and `Exported`. The row of the Studio document shows the live state and the pause glyph while the session play state is `Playing` or `Buffering`. A click on it then sends `Action::Pause`.
7. **Filters.** "Ready" shows the `Ready` and `Exported` documents. "Drafts" shows the `Draft` documents. "All" shows all documents.
8. **Search.** The field sends `Action::Search` on each key press. `LibraryView` passes the query and the filter to `Library::list`, which matches the title and the text. `AppState` calls `Library::build_search_index` on a background thread after the first frame. `cmd-k` from any screen opens this screen and focuses the field. Escape clears the field.
9. **Empty states.** An empty library shows the `EmptyState` with the text "No documents yet" and the "New" button. A search without results shows the `EmptyState` with the text "No results" and no button.

### Play from the Library

`Action::PlayDocument(id)` makes the document the Studio document and plays it. The screen stays the Library.

1. `AppState` opens the document as `Action::OpenDocument` does. This sends `Input::DocumentChanged`, which stops the playback and the job of the previous Studio document.
2. When the `Input::Timeline` of the document arrives, `AppState` sends `Input::Play`.

### Relative time

`relative_time(modified: Timestamp, now: &Zoned, locale: Locale) -> String` is a pure function. It uses the first rule that matches.

| Rule | English | Spanish |
|---|---|---|
| Less than 1 minute | "now" | "ahora" |
| Less than 60 minutes | "12 min ago" | "hace 12 min" |
| Same civil date | "3 h ago" | "hace 3 h" |
| Previous civil date | "yesterday" | "ayer" |
| Same ISO 8601 week | "Monday" | "lunes" |
| Same year | "28 Sep" | "28 sep" |
| Other years | "28 Sep 2025" | "28 sep 2025" |

### Live status

The percent of a `Generating` document comes from `Status::Generating { done, total }` of `Library::list`. `AppState` sets the progress with `Library::set_progress` from the job events, as stage 16 defines. Thus the library is the only place that knows the progress, and the app keeps no second map.

### Recent documents

The sidebar section "Recent" uses `Library::recent`. The list updates after each save, rename, open and job end.

### Voices screen

The layout follows `docs/design/screens/voces.png`.

1. **Header.** The title "Voices" and the voice count of the selected language. The `SearchField` of `LAYOUT_SEARCH_VOICES` with the hint "Name or tone". The `Segmented` language tabs, one for each language of the registry, in the order of `Language::ALL`. The first selected tab is the language of the Studio document, or the interface language when the Studio document has no language.
2. **Search.** `VoicesView` folds the query, the voice name and the voice description in the interface language with `antenna_library::fold`. A voice matches when each term of the query is a substring of the folded name or the folded description.
3. **Grid.** The number of columns is the largest number of `LAYOUT_VOICE_COLUMN` columns with the gap `SPACE_L4` that fits in the width. At `LAYOUT_WINDOW` with the open sidebar, the grid has four columns. Each cell contains these items.
   1. The `VoiceAvatar` with `AVATAR_GALLERY`, idle or live while its preview plays
   2. The name in `VOICE_NAME`, and the description in `SMALL`
   3. The engine `Chip` with the engine name
   4. The preview `PlayButton` (Gallery) and the "Use voice" `Button`
4. **Use.** "Use voice" sends `Action::SetVoice` for the Studio document. If the language of the voice is not the language of the document, `AppState` also sets the document language to the language of the voice. In both cases `AppState` sets `language_source` to `User`, so that language identification does not change the language again. The voice of the Studio document shows a check icon and the text "In use".
5. **Preview.** See "Voice preview".
6. **Not installed.** The files of the voice for `Quality::Fast` can be missing in the model store. Then the preview button is in the download state, and a `DATA_TINY` label next to it shows the download size of `ModelStore::missing_bytes(voice_artifacts(descriptor, voice, Quality::Fast))`. A click sends `Action::DownloadModel(ModelRef::Voice(voice, Quality::Fast))`. The button shows the progress state from `Downloads`. The preview plays when the download ends.

### Voice preview

`state/preview.rs` owns the preview job and the preview track. `AppState` owns the `Preview`.

1. For `Action::Preview(voice)`, `AppState` sends `Input::Stop` to the session first. Then it builds a `Document` of the preview sentence of the language of the voice. The sentence is in `strings/` and contains the voice name, for example "Hola, soy Lucía. Así suena esta voz.".
2. It calls `Pipeline::timeline` with `Quality::Fast`, builds the `Track` with `Timeline::stored` and `Track::new`, and calls `Player::load` and `TrackHandle::play`.
3. If the timeline is not complete, it starts a job with the `LiveOutput` of the preview track. The segment store keeps the result, so the second preview of the voice starts without synthesis.
4. `Action::Preview` of the voice that plays drops the preview. `Action::Preview` of another voice drops the current preview and starts the new one.
5. The preview ends when the track sends `Ended`. The `PlayFrom` command of the Studio session drops the preview.

### Models screen

The layout follows `docs/design/screens/modelos.png`.

1. **Header.** The title "Models", the installed count in `DATA` and the "Open folder" `Button`, which opens the model store folder with `cx.reveal_path`.
2. **Summary.** Three columns, separated by vertical `Divider` components.
   1. Acceleration shows the device of `antenna_review::select_device`, and below it the chip name and the total memory. `antenna_review::device_label` returns "Metal" or "CPU". These are product names, so the view shows them in both locales with no string of `strings/`.
   2. Memory in use shows the resident memory of the process and the total memory, with a `ProgressBar`.
   3. On disk shows the size of the model store and its path.
3. **Lists.** "Installed" and "Available". An engine is installed when `ModelStore::is_engine_installed` is true for the engine at `Settings.quality`. The review model is installed when `ModelStore::has_artifact` is true for each artifact of `WHISPER_SMALL`. The review model is in the same lists with the kind "Review". The CLI of stage 07 uses the same functions, so the two apps agree.
4. **Rows.** Each row has the columns `LAYOUT_MODELS_COLUMNS` and contains these items.
   1. The `MonogramTile` with two letters, the name and the kind `Chip` ("Voice" or "Review")
   2. The summary and the number of languages of the engine voices
   3. The `parameters` text of the variant for `Settings.quality`, the download size and the license name. The download size is `antenna_models::engine_download_bytes` for the engine at `Settings.quality`, or the sum of the bytes of the artifacts of `WHISPER_SMALL`
   4. The actions
5. **Actions of an installed engine.** "Default" with a check icon for the default engine, or "Use as default" for the others. The "More" menu has "Show license" and "Remove". "Remove" opens a confirmation `Popover` with the size of `ModelStore::engine_disk_usage`. After a click on the confirmation button, `ModelStore::remove_engine` deletes the files. The `keep` argument is `antenna_models::keep_artifacts` of the descriptors of all other engines and the artifacts of `WHISPER_SMALL`.
6. **Actions of the installed review model.** "Show license".
7. **Actions of an available model.** The "Download" `Button` sends `Action::DownloadModel` with `ModelRef::Engine(id, Settings.quality)` or `ModelRef::Review`. During the download, the row shows the downloaded and total size, the percent, a `ProgressBar` and a cancel `IconButton` that sends `Action::CancelDownload`. The values come from `Downloads`.
8. **Default engine.** "Use as default" sends `Action::SetDefaultEngine`.

### System data

1. `SystemInfo::read` gets the chip name and the total memory with `sysinfo`, and the resident memory with `memory-stats`.
2. The Models screen reads `SystemInfo` when it opens and after each download or removal. It does not poll. Thus the screen uses no CPU when nothing changes.
3. The size of the model store comes from `ModelStore::disk_usage`, on a background thread. The size of each engine comes from `ModelStore::engine_disk_usage`.

### Settings in this stage

Add `default_engine: Option<EngineId>` to `Settings`. The default is `None`, which means the registry default. The file has the engine id as text. `Settings::load` finds the factory with `Registry::engine`. An id that the registry does not have gives `None` and one `warn!` event.

`Settings::voice_for` uses the first rule that matches.

1. The voice of `voices` for the language.
2. The first voice of `default_engine` for the language.
3. `Registry::default_voice` of the language.

## Tasks

1. Write `state/library_view.rs`. `LibraryView` keeps the query, the filter and the rows of the last `Library::list` call. It calls `Library::list` again when the query, the filter or the library changes. Test it with a temporary library. Do not copy the search, filter or grouping logic of stage 05.
2. Write `strings/relative_time.rs`. Test it with fixed dates.
3. Write the Library screen in the sequence `header.rs`, `table.rs` and `library/mod.rs`.
4. Bind `cmd-k` in `main.rs`. Connect the "Recent" section of the sidebar to `Library::recent`.
5. Write `state/voices_view.rs`, `state/preview.rs` and the Voices screen. Add the preview sentence of each language to `strings/`.
6. Write `state/downloads.rs`, `state/system.rs` and `state/models_view.rs`. Test the installed and available split with a temporary model store.
7. Write the Models screen in the sequence `summary.rs`, `list.rs` and `models/mod.rs`.
8. Add `default_engine` to `Settings` and use it in `Settings::voice_for`.
9. Write `benches/library_search.rs`. It seeds a temporary library with 1000 documents of 2 KB and builds the search index. Then it measures `Library::list` with the filter `All` for each of 10 queries. `LibraryView` calls `Library::list` one time for each refresh and adds no other work, so the bench measures the refresh. The bench uses `antenna-library` directly, because `antenna-desktop` has only a binary target.
10. Run the app with `cargo run -p antenna-desktop --features engine-fake`. Do the manual QA script below.
11. Measure the budget of AC-17-12. Write the result in the PR.
12. Run `cargo xtask check`. Fix each failure.
13. Do the quality gate in `docs/standards.md` section 20.

### Manual QA script

Do these steps with `--features engine-fake` in a release build. Write the result of each step in the PR.

1. Seed a data directory with `seed_library` and 40 documents. Start the app with that directory. Open the Library. The documents are in period groups.
2. Type a word that occurs in three documents. Only those three rows stay.
3. Select "Drafts". Only drafts stay. Select "All".
4. Click the play button of a ready document. Its audio plays and the button shows pause. The screen stays the Library.
5. Click a row. The Studio opens the document.
6. Press `cmd-k` in the Studio. The Library opens and the search field has focus.
7. Open Voices. The tab of the Studio language is selected. Click the preview of a voice. The sentence plays and the avatar is live. Click it again in less than 1 s after the end. It plays at once with no buffer state.
8. Click "Use voice" on another voice. The Studio inspector shows that voice.
9. Open Models. The summary shows the device, the memory and the disk use.
10. Click "Download" on Whisper Small. The row shows the progress. Click cancel. The row shows "Download" again.
11. Download Whisper Small. It moves to "Installed", and the disk use increases by the download size.

## Acceptance criteria

| ID | Criterion | Check |
|---|---|---|
| AC-17-01 | The library view groups documents by period in the correct order | Test `library_view_groups_by_period_when_dates_vary` |
| AC-17-02 | Each filter gives the correct statuses | Tests `library_view_keeps_exported_when_filter_is_ready` and `library_view_keeps_only_drafts_when_filter_is_drafts` |
| AC-17-03 | Search matches the title and the text without case or accents | Test `library_view_search_matches_when_accents_differ` |
| AC-17-04 | The live percent of a generating document comes from its job events through `Library::set_progress` | Test `app_state_generating_percent_follows_synthesized_events` |
| AC-17-05 | The relative time is correct in English and Spanish | One test for each row of the relative time table and each locale, named `relative_time_gives_<text>_when_<rule>_in_<locale>` |
| AC-17-06 | The voice search matches the name and the description in the interface language | Test `voices_view_search_matches_description_when_locale_is_es` |
| AC-17-07 | "Use voice" sets the Studio document voice, and the language when the voice has another language | Tests `use_voice_sets_document_voice_when_language_is_same` and `use_voice_sets_document_language_when_language_differs` |
| AC-17-08 | A second preview of the same voice does not start a synthesis | Test `preview_uses_store_when_segment_is_cached` |
| AC-17-09 | The installed and available lists follow the model store | Test `models_view_moves_engine_when_files_are_removed` |
| AC-17-10 | The default engine setting changes the default voice of each language | Test `settings_voice_for_uses_engine_when_default_engine_is_set` |
| AC-17-11 | The Models screen does not poll | `rg -e timer -e interval apps/desktop/src/screens/models apps/desktop/src/state/system.rs` returns no lines |
| AC-17-12 | Search and filter with 1000 documents take 16 ms or less **(manual)** | On the Apple M5 Pro 24 GB, `cargo bench -p antenna-desktop --bench library_search` reports a median of 16 ms or less for each query |
| AC-17-13 | The three screens match the design **(manual)** | Take a screenshot of each screen at `LAYOUT_WINDOW`. Open each one and its file in `docs/design/screens/` in Preview at the same scale. For each control, measure the height, the padding and the gap with the rectangle selection. Each value is in 1 px of the design. Each color that Digital Color Meter reads is the token value. The PR contains the images and the list of measured controls |
| AC-17-14 | The manual QA script passes **(manual)** | The PR lists the result of each of the 11 steps |
| AC-17-15 | No raw design value is in `apps/desktop/src` | `cargo xtask lint-repo` passes |
| AC-17-16 | Play from the Library plays the document after its timeline loads | Test `play_document_plays_when_timeline_arrives` |
| AC-17-17 | A preview stops the Studio playback, and a Studio play stops the preview | Tests `preview_stops_session_when_session_plays` and `session_play_drops_preview_when_preview_plays` |
| AC-17-18 | Screens and the shell do not call the player, the library, the model store or the pipeline, and do not name `Input` | `rg -e antenna_audio:: -e antenna_library:: -e antenna_models:: -e antenna_pipeline:: -e Input:: apps/desktop/src/screens apps/desktop/src/shell` returns no lines |
| AC-17-19 | The removal of an engine keeps the artifacts of the other engines and of the review model | Test `models_view_keeps_review_artifacts_when_engine_is_removed`, with a temporary model store |
| AC-17-20 | Each download of the app has one state in `Downloads` | Tests `downloads_shows_progress_when_voice_download_runs` and `downloads_removes_entry_when_download_is_cancelled` |
| AC-17-21 | "Use voice" stops the language identification of the document | Test `use_voice_sets_language_source_user_when_voice_is_used` |

## Decision rules

1. If `Library::list` is slower than the budget with 1000 documents, write a "Blocked" section for the owner of stage 05. Do not add a second search in the app.
2. If `sysinfo` cannot read the chip name, show only the total memory.
3. If the confirmation popover cannot open from a menu item in `gpui-component`, open it from the row instead. Do not remove files without a confirmation.
4. If the design example data shows engines that the registry does not have, show only the registry engines. The design data is an example.
5. If `antenna_library::fold` is not public in the merged code, write a "Blocked" section. Do not copy the fold logic.

## Out of scope

1. The Settings screen and the updates. Stage 18 does them.
2. Deletion of documents from the library. It is not in the design.
3. Voice cloning and voice creation.
4. Engines that are not in the registry.
5. The removal of the review model files.
6. The QA steps with a real engine. Stage 19 does them.
