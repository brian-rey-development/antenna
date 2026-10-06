# Stage 18. Desktop Settings and Updates

## Goal

The Settings screen of `antenna-desktop` changes and saves each setting of the design at once. Pronunciation review marks the words that Whisper hears differently. The app finds, shows and installs updates from the stable or beta channel.

## Context

Stages 16 and 17 built four screens with a partial settings file. This stage completes the settings and connects the interface language and the appearance to the whole app. It connects pronunciation review to the reading view. It adds the update client with the "What's new" panel. The project owner makes the signing keys. Stage 19 publishes the update feed.

## Read first

1. `CLAUDE.md`
2. `docs/architecture.md` sections 1.1, 4.4, 6.5, 6.6, 7.1 and 12
3. `docs/standards.md` sections 5, 6, 7, 13, 14, 18 and 19
4. `docs/writing.md` section 4 (glossary)
5. `docs/design/screens/ajustes.png`, `docs/design/pages/ajustes.html` and `docs/design/tokens.md`
6. `docs/plans/16-desktop-studio.md`, `docs/plans/17-desktop-library.md` and the merged code of `apps/desktop`
7. The public API of `antenna-review` as merged by stage 08
8. The `cargo-packager-updater` documentation for the version of `docs/architecture.md` section 10

## Scope

The stage can create or change these files only.

- `apps/desktop/src/main.rs`
- `apps/desktop/src/state/`
- `apps/desktop/src/settings/`
- `apps/desktop/src/updates/`
- `apps/desktop/src/strings/`
- `apps/desktop/src/shell/`
- `apps/desktop/src/screens/mod.rs` and `apps/desktop/src/screens/settings/`
- `apps/desktop/src/screens/studio/reading.rs` (the review marks only)
- `apps/desktop/Cargo.toml`
- `Cargo.toml` (`cargo-packager-updater` and `serde_json` in `[workspace.dependencies]`)

## Deliverables

### Files

```
apps/desktop/src/
├── settings/
│   └── autosave.rs      # debounced save after each change
├── updates/
│   ├── mod.rs           # UpdateClient: check, download, install, restart, and the feed constants
│   ├── version.rs       # Version: parse and order
│   ├── feed.rs          # Release and ReleaseNotes
│   └── news.rs          # the unread rule of "What's new"
├── state/
│   └── review.rs        # ReviewMarks and the review gate
└── screens/
    └── settings/
        ├── mod.rs       # Settings screen: section list and content
        ├── general.rs
        ├── generation.rs
        ├── audio.rs
        ├── shortcuts.rs
        ├── updates.rs
        └── about.rs
```

### Settings

Stage 18 adds these fields. Each field has a default, so older files stay valid. `Settings::load` converts each field of the file as stage 16 defines.

```rust
pub(crate) struct Settings {
    // fields of stages 16 and 17
    pub(crate) interface_language: InterfaceLanguage,       // System, En, Es. Default System
    pub(crate) appearance: AppearancePreference,            // System, Light, Dark. Default System
    pub(crate) on_start: OnStart,                           // LastDocument, NewDocument, Library. Default LastDocument
    pub(crate) review: ReviewMode,                          // Off, On. Default Off
    pub(crate) updates: UpdateMode,                         // Automatic, Manual. Default Automatic
    pub(crate) channel: Channel,                            // Stable, Beta. Default Stable
    pub(crate) last_seen_version: Version,                  // default: the running version
}
```

1. Each change in the Settings screen sends an `Action` that updates `Settings` in `AppState` at once. `settings/autosave.rs` saves the file 300 ms after the last change, with the temporary file and rename of stage 16.
2. The header of the screen shows the text "Changes save automatically" in `DATA_SMALL`.
3. `AppearancePreference` is a desktop enum. The pure function `resolve_appearance(preference: AppearancePreference, window: WindowAppearance) -> antenna_ui::Appearance` converts it. `antenna-ui` has no `serde`, so the file stores only `AppearancePreference`.
4. `InterfaceLanguage` resolves to a `Locale` (`En` or `Es`) with the rule of stage 16. `AppState.strings` is the `Strings` value of that locale.

### New actions

```rust
pub(crate) enum Action {
    // the actions of stages 16 and 17, and:
    SetInterfaceLanguage(InterfaceLanguage),
    SetAppearance(AppearancePreference),
    SetOnStart(OnStart),
    SetDefaultVoice(Language, VoiceId),
    SetDefaultQuality(Quality),
    SetReview(ReviewMode),
    SetExportRate(ExportRate),
    SetExportDir(PathBuf),
    SetUpdateMode(UpdateMode),
    SetChannel(Channel),
    DownloadUpdate,
    RestartAndUpdate,
    MarkNewsSeen,
}
```

1. `SetDefaultQuality` changes only `Settings.quality`. `Action::SetQuality` of stage 16 changes the `StoredVoice` of the Studio document. The two actions do not share a target.
2. The default format and the loudness rows send `Action::SetExportFormat` and `Action::SetLoudness` of stage 16, because they change the same settings as the inspector.
3. The "Change" button of the export folder opens `cx.prompt_for_paths`. The result sends `Action::SetExportDir`.
4. "Repository", "Report a problem" and "Third-party licenses" open a URL or a file with GPUI. They change no state, so the About view calls GPUI directly and sends no action.

### Settings screen

The layout follows `docs/design/screens/ajustes.png`. The left column lists the sections with one `SubNavItem` each. A click on a section scrolls the content to it. The design row "Generate while you type" is out of scope and is not in the app. Each row is a `SettingsRow`.

1. **General.**
   1. Interface language, a `Select` with "System", "English" and "Español". A change replaces `AppState.strings`, and all screens show the new text in the next frame.
   2. Appearance, a `Segmented` with "Light", "Dark" and "System". "System" follows `window.appearance()` and observes its changes with `cx.observe_window_appearance`. Each change calls `antenna_ui::set_appearance` with the resolved appearance.
   3. On start, a `Select` with "Last document", "New document" and "Library".
2. **Generation.**
   1. Default voice. The control shows the default voice of the interface language with its `AVATAR_ROW` avatar. A click opens a `Popover` with one voice `Select` for each language. Each select sends `Action::SetDefaultVoice`, which saves into `Settings.voices`. The setting applies to new documents.
   2. Quality, a `Segmented` with "Fast", "Balanced" and "Max", and the line "Max takes longer and uses more memory". It sends `Action::SetDefaultQuality`. The setting applies to new documents. The quality of an open document stays the quality of its `StoredVoice`.
   3. Pronunciation review, a `Toggle` with the line "Whisper listens to each sentence and marks misread words". See "Pronunciation review" below.
3. **Audio.**
   1. Default format, a `Segmented` with MP3, WAV and OGG. It changes `Settings.export_format`, the same value as the inspector control.
   2. Loudness normalization, a `Toggle` with the line "-16 LUFS, the standard for podcasts". It changes `Settings.loudness`.
   3. Sample rate, a `Select` with 24 kHz, 44.1 kHz and 48 kHz. It changes `Settings.export_rate`.
   4. Export folder, the path in `DATA` and a "Change" `Button` that opens `cx.prompt_for_paths` for one directory.
4. **Shortcuts.** A read-only list of the shortcuts of stages 16 and 17, with the `Key` component. The rows are "Play or pause", "Stop", "New document", "Export", "Search" and "Show or hide inspector".
5. **Updates.** See "Updates" below.
6. **About.** The `Logo` app icon tile, the word mark "antenna" in `BRAND_ABOUT` with the version in `DATA`, and the line "Open source, GPL-3.0-or-later license. Everything runs on this device." The Spanish line is "Código abierto, licencia GPL-3.0-or-later. Todo se procesa en este equipo." The design page says "MIT" and "antena". The product decisions replace both. Then the buttons "Repository", "Report a problem" and "Third-party licenses". Below them, each engine of the registry and the review model has one row. The row shows the license name, the URL and the attribution text of the descriptor.
   1. "Repository" opens `REPOSITORY_URL` with `cx.open_url`. `updates/mod.rs` defines `REPOSITORY_URL` as `env!("CARGO_PKG_REPOSITORY")`, which is the `repository` field of `[workspace.package]`. Stage 19 sets that field to the repository of the project owner.
   2. "Report a problem" opens the new-issue URL of the repository with the app version and the macOS version in the query.
   3. "Third-party licenses" opens `THIRD_PARTY_LICENSES.html` of the app bundle with `cx.open_with_system`. The button is disabled when the file does not exist, for example in a development build.

### Pronunciation review

```rust
pub(crate) struct ReviewMark { pub(crate) ranges: Vec<Range<usize>>, pub(crate) transcript: String }   // ranges relative to the segment start
pub(crate) struct ReviewMarks { marks: HashMap<SegmentKey, ReviewMark>, pending: HashMap<SegmentIndex, SegmentKey>, failed: HashSet<SegmentKey> }
impl ReviewMarks { pub(crate) fn to_submit(&self, timeline: &Timeline) -> Vec<SegmentIndex>; }   // stored, no mark, not pending, not failed
pub(crate) enum JobActivity { Idle, Running }
pub(crate) fn review_gate(activity: JobActivity) -> Gate;            // Closed when Running, Open when Idle
```

1. When the toggle turns on, `AppState` checks the artifacts of `WHISPER_SMALL` with `ModelStore::has_artifact`. If one is missing, `AppState` sends `Action::DownloadModel(ModelRef::Review)` of stage 17, and the toggle row shows a `ProgressBar` with the progress of `Downloads`.
2. When the files are there, `AppState` loads a `Transcriber` and starts the `ReviewQueue`. The gate of a new queue is `Closed` (stage 08). Thus, right after `ReviewQueue::start`, `AppState` calls `ReviewQueue::set_gate` with `review_gate` of `job_activity`, and submits the segments of `to_submit`. A document that is complete and idle is then reviewed with no job event. When the toggle turns off, `AppState` drops the queue.
3. After each `Input::Timeline` and each `Synthesized` event of the Studio document, `AppState` submits each segment of `ReviewMarks::to_submit`. A segment is in that list when it has a duration in the timeline. Its key also has no mark, is not pending and is not in `failed`. The `document` field of the request is `DocumentId::uuid`. The samples come from `SegmentStore::read`. `ReviewMarks` records the key of each submitted segment in `pending`.
4. `AppState::job_activity` returns `Running` while a job of the app runs. These jobs are the session job, a voice preview job of stage 17 and the preload of stage 16. In all other cases it returns `Idle`. After each change of either job, `AppState` calls `ReviewQueue::set_gate` with `review_gate` of the new activity. Thus review never runs while the job worker synthesizes.
5. On `ReviewEvent::Reviewed`, `ReviewMarks` takes the key of the segment from `pending`. It stores the mismatched ranges minus the start of the source range of the request, and the transcript.
6. On `ReviewEvent::Failed`, `ReviewMarks` moves the key from `pending` to `failed`. Thus the app does not submit the segment again while the document stays open. `AppState` logs the `ReviewError` with `warn!`.
7. The marks are keyed by the segment key, and the key contains the segment text. Thus a change of the text keeps the marks of each segment with the same text and drops the marks of each changed segment. On `Input::DocumentChanged`, `AppState` calls `ReviewQueue::clear` with the document and empties `pending` and `failed`.
8. The reading view finds the mark of each segment of the timeline by its key. It draws each range plus the start of the segment source with a wavy underline in `REVIEW_UNDERLINE` of `REVIEW_UNDERLINE_WIDTH`. A hover on a flagged range shows the `Tooltip` component with the transcript of the mark.

### Updates

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Version { major: u64, minor: u64, patch: u64, pre: Pre }
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Pre { Beta(u64), Release }                 // Beta orders before Release
impl FromStr for Version;                                  // "0.1.0" and "0.1.0-beta.1". Other forms are errors
impl Display for Version;
impl Version { pub(crate) fn running() -> Self; }          // from env!("CARGO_PKG_VERSION")

pub(crate) struct Release { pub(crate) version: Version, pub(crate) date: jiff::Timestamp, pub(crate) notes: ReleaseNotes }
pub(crate) struct ReleaseNotes { en: String, es: Option<String> }
impl ReleaseNotes { pub(crate) fn text(&self, locale: Locale) -> &str; }   // English when the Spanish text is missing

pub(crate) struct UpdateClient { /* feed URL for each channel, public key, running version */ }
impl UpdateClient {
    pub(crate) fn check(&self, channel: Channel) -> Result<Release, UpdateError>;   // the release of the feed, also when it is not newer
    pub(crate) fn download(&self, release: &Release, progress: impl FnMut(u64, u64)) -> Result<DownloadedUpdate, UpdateError>;
    pub(crate) fn install_and_restart(&self, update: DownloadedUpdate) -> Result<(), UpdateError>;
}
```

1. `UpdateClient` uses `cargo-packager-updater`. The feed URLs and the public key are constants in `updates/mod.rs`. Each feed URL is `concat!` of `env!("CARGO_PKG_REPOSITORY")` and the path of stage 19, for example `/releases/download/update-feed/stable.json`. Stage 19 sets only the value of the public key constant. The feed of stage 19 has the English Markdown notes in the field `notes` and the Spanish Markdown notes in the field `notes_es`. `feed.rs` parses the feed JSON with `serde_json`, because the type of the updater has no `notes_es` field. It reads both fields. The app shows the notes in the active locale. If `notes_es` is missing, the Spanish UI shows the English notes.
2. The environment variable `ANTENNA_UPDATE_FEED` replaces the feed URL, for tests with a local feed.
3. The app checks for an update 10 s after start and then one time each 24 h, on a background thread. A `cx.background_executor().timer` in `updates/mod.rs` waits between checks. The wait uses no CPU.
4. An update is available when the version of the checked release is newer than `Version::running`. `AppState` keeps the last checked release, so "What's new" can show its notes after an update installs.
5. If `updates` is `Automatic` and an update is available, the app downloads it in the background. The Updates section then shows "Antenna X is available", the line "You have Y. It installs when you restart." and the "Restart and update" `Button`.
6. If `updates` is `Manual`, the section shows the same text with a "Download" `Button` before "Restart and update". The buttons send `Action::DownloadUpdate` and `Action::RestartAndUpdate`.
7. The "Automatic updates" `Toggle` and the "Channel" `Segmented` with "Stable" and "Beta" are below. The beta line is "Beta gets new features first. A beta can have more defects."

### What's new

```rust
pub(crate) fn is_unread(running: Version, last_seen: Version, update: Option<Version>) -> bool;
```

1. `is_unread` is true when the running version is newer than `last_seen_version`, or when an update is available and its version is newer than `last_seen_version`.
2. On the first start, the default of `last_seen_version` is the running version, so a new install shows no badge.
3. The sidebar entry "What's new" shows the `UnreadBadge` with the count 1 when `is_unread` is true.
4. A click opens a `Popover` of `LAYOUT_NEWS_POPOVER` with the notes of the last checked release of the current channel. If no check returned a release, the popover shows the line "Release notes need a network connection".
5. When the popover closes, the shell sends `Action::MarkNewsSeen`. Then `last_seen_version` becomes the newer one of the running version and the version of the feed release.

## Tasks

1. Add the fields of the "Settings" deliverable. Write the tests of the defaults for an old file.
2. Write `settings/autosave.rs`.
3. Write the interface language switch, which replaces `AppState.strings`.
4. Write `resolve_appearance` and the appearance switch with `antenna_ui::set_appearance`.
5. Write the Settings screen in the sequence `general.rs`, `generation.rs`, `audio.rs`, `shortcuts.rs`, `about.rs` and `screens/settings/mod.rs`.
6. Write `state/review.rs` and connect the review queue. Add the underline and the tooltip to the reading view.
7. Write `updates/version.rs`, `updates/feed.rs` and `updates/news.rs` with their tests. Use a fixed feed file in the tests.
8. Write `updates/mod.rs` and `screens/settings/updates.rs`.
9. Connect the "What's new" entry of the sidebar.
10. Run the app with `cargo run -p antenna-desktop --features engine-fake`. Do the manual QA script below.
11. Run `cargo xtask check`. Fix each failure.
12. Do the quality gate in `docs/standards.md` section 20.

### Manual QA script

Do these steps with `--features engine-fake` in a release build. Write the result of each step in the PR.

1. Open Settings. Change the interface language to English. All screens show English text in the next frame.
2. Change the appearance to "Dark". All screens use the dark theme. Select "System" and change the macOS appearance in System Settings. The app follows in less than 1 s.
3. Change "On start" to "Library". Quit and start the app. The Library opens.
4. Change the quality to "Fast". Press `cmd-n` and type one line. The inspector of the new document shows "Fast". The inspector of the previous document shows its own quality.
5. Change the default format to WAV and the export folder to a new folder. Export from the Studio. The save panel opens in the new folder with a `.wav` name.
6. Turn on pronunciation review. The Whisper Small download shows its progress. After the download, the toggle stays on.
7. Start a local feed server with a feed of a newer version and set `ANTENNA_UPDATE_FEED`. Start the app. In less than 15 s, the Updates section shows the new version and the "What's new" badge shows 1.
8. Open "What's new". The notes show. Close it. The badge goes away.
9. Click "Repository" and "Report a problem". Each opens the correct page in the browser.

## Acceptance criteria

| ID | Criterion | Check |
|---|---|---|
| AC-18-01 | A settings file of stage 16 loads with the defaults of the new fields | Test `settings_load_gives_new_field_defaults_when_file_is_old` |
| AC-18-02 | Settings save 300 ms after the last change and only one time for a burst | Test `autosave_writes_once_when_changes_burst`, with a test clock |
| AC-18-03 | The interface language "System" follows the macOS preference | Test `interface_language_system_gives_es_when_preference_is_es` |
| AC-18-04 | The appearance "System" follows the window appearance | Test `resolve_appearance_gives_dark_when_window_is_dark` |
| AC-18-05 | Each "On start" value opens the correct screen and document | Tests `on_start_opens_last_document_when_value_is_last_document`, `on_start_opens_draft_when_value_is_new_document` and `on_start_opens_library_when_value_is_library` |
| AC-18-06 | A text change keeps the review marks of unchanged segments and drops the marks of changed segments | Tests `review_marks_keep_segment_when_text_is_same` and `review_marks_drop_segment_when_text_changes` |
| AC-18-07 | Review never runs while the job worker synthesizes | Tests `review_gate_is_closed_when_activity_is_running`, `review_gate_is_open_when_activity_is_idle` and `job_activity_is_running_when_preview_job_runs` |
| AC-18-08 | The feed parser reads the version, the date and both notes | Test `feed_parses_fields_when_json_is_valid` |
| AC-18-09 | Version order puts a beta before the release with the same numbers | Tests `version_orders_beta_first_when_numbers_are_equal` and `version_parse_fails_when_form_is_unknown` |
| AC-18-10 | The unread rule follows the "What's new" deliverable | Tests `news_is_unread_when_update_is_newer_than_last_seen`, `news_is_unread_when_running_is_newer_than_last_seen` and `news_is_read_when_last_seen_is_current` |
| AC-18-11 | The update check has one wait, and the wait is a timer | The commands of the list "AC-18-11 commands" below the table |
| AC-18-12 | The About section lists each engine license and its attribution | Test `about_rows_include_engine_when_registry_has_it` |
| AC-18-13 | The Settings screen matches the design, without the out-of-scope row **(manual)** | At `LAYOUT_WINDOW`, scroll to each section and take one screenshot. Open each one and the matching part of `docs/design/screens/ajustes.png` in Preview at the same scale. For each row, measure the height, the padding and the gap with the rectangle selection. Each value is in 1 px of the design. Each color that Digital Color Meter reads is the token value. The PR contains the images |
| AC-18-14 | The manual QA script passes **(manual)** | The PR lists the result of each of the 9 steps |
| AC-18-15 | The app uses no CPU when no job runs and nothing plays, with automatic updates on **(manual)** | Start a local feed server with a feed of the running version. Start the release build with `ANTENNA_UPDATE_FEED` set to that feed and automatic updates on. Click the sidebar outside the editor. Wait 15 s, so the first check at 10 s ends. Run `ps -o cputime= -p $PID`, wait 10 s, and run it again. The two values are equal |
| AC-18-16 | No raw design value is in `apps/desktop/src` | `cargo xtask lint-repo` passes |
| AC-18-17 | The notes follow the interface language, with English when the Spanish notes are missing | Tests `release_notes_use_spanish_when_present` and `release_notes_use_english_when_spanish_is_missing` |
| AC-18-18 | Screens and the shell do not call the player, the library, the model store or the pipeline, and do not name `Input` | `rg -e antenna_audio:: -e antenna_library:: -e antenna_models:: -e antenna_pipeline:: -e Input:: apps/desktop/src/screens apps/desktop/src/shell` returns no lines |
| AC-18-19 | A review result stores ranges relative to the segment start | Test `review_marks_store_segment_relative_ranges_when_reviewed` |
| AC-18-20 | The app submits each stored segment without a mark one time | Tests `review_marks_select_stored_segment_when_mark_is_missing` and `review_marks_skip_segment_when_review_is_pending` |
| AC-18-21 | A failed review is not submitted again for the open document | Test `review_marks_skip_segment_when_review_failed` |
| AC-18-22 | A queue that starts while no job runs reviews the stored segments at once | Test `app_state_submits_segments_when_review_starts_and_job_is_idle`, with `gpui::TestAppContext` |
| AC-18-23 | "What's new" shows the notes of the installed version after an update | Test `check_returns_release_when_feed_version_is_running_version`, with a fixed feed file |
| AC-18-24 | The preload of stage 16 closes the review gate | Test `job_activity_is_running_when_preload_runs` |

AC-18-11 commands. Run each command from the repository root.

1. `rg -c -F "timer(" apps/desktop/src/updates/mod.rs` gives 1.
2. `rg -l -F "timer(" apps/desktop/src/updates` lists only `apps/desktop/src/updates/mod.rs`.
3. `rg -F "thread::sleep" apps/desktop/src/updates` returns no lines.

## Decision rules

1. If `cargo-packager-updater` cannot use a custom feed URL for each channel, use one feed with a channel field in each release, and filter in `updates/feed.rs`.
2. If the update install needs a signed app bundle, test only `check` and `download` in this stage. Stage 19 tests the install with the signed bundle.
3. If the review model is not installed and the network is not available, the toggle goes back to off. The row shows the network error message.
4. If `cx.observe_window_appearance` does not exist in `gpui-pre =0.3.8`, read the appearance in the window activation callback. Do not poll.
5. If `sys_locale::get_locale()` returns `None`, "System" means English.
6. If `cx.open_url` does not exist in `gpui-pre =0.3.8`, use the GPUI function that opens a URL with the default app. If no such function exists, write a "Blocked" section. Do not add a crate.
7. If `InteractiveText` cannot show a tooltip for a text range, show the transcript in a `Popover` when the user clicks a flagged range. Do not add a second text view.

## Out of scope

1. The "Generate while you type" setting.
2. Editable shortcuts.
3. The signing keys, the feed hosting and the release workflow. Stage 19 does them.
4. Interface languages other than English and Spanish.
5. The QA of review marks with a real engine. Stage 19 does it.
