# Stage 19. Release

## Goal

A signed and notarized `Antenna-0.1.0-arm64.dmg` is GitHub release v0.1.0 and works on a clean Mac. The app finds updates in signed stable and beta feeds. All budgets and quality targets have recorded evidence.

## Context

This is the last stage of the MVP. It packages the app, signs the app and its updates, publishes the update feeds and shows the licenses of the code and of the models. It also writes the contributor documents and checks the complete product with the real engines on the reference machine. After this stage, a user who has no Rust toolchain can install Antenna, use it and receive updates.

## Inputs from the project owner

These items must exist before task 7.

| Item | Use |
|---|---|
| Bundle identifier, for example `app.antenna.Antenna` | `Info.plist` and signing |
| GitHub repository URL, for example `https://github.com/<owner>/antenna` | The `repository` field of `[workspace.package]`. The app builds the repository link and the feed URLs from it |
| Apple Developer ID Application certificate as a `.p12` file with its password | Code signing |
| App Store Connect API key (`.p8` file, key id, issuer id) | Notarization |
| Update signing key pair from `cargo packager signer generate`, with its password | Update signatures |
| GitHub environment `release` with the secrets of the "Secrets" table and a required reviewer | The release workflow |

The agent does not create, request or handle the certificate or the keys. The project owner makes them and puts them in the GitHub environment. The agent receives only the public update key.

### Secrets

| Secret | Value |
|---|---|
| `APPLE_CERTIFICATE` | The `.p12` file in base64 |
| `APPLE_CERTIFICATE_PASSWORD` | The password of the `.p12` file |
| `APPLE_API_KEY` | The key id of the App Store Connect API key |
| `APPLE_API_ISSUER` | The issuer id |
| `APPLE_API_KEY_CONTENT` | The `.p8` file. The workflow writes it to a temporary file and sets `APPLE_API_KEY_PATH` |
| `CARGO_PACKAGER_SIGN_PRIVATE_KEY` | The private update key |
| `CARGO_PACKAGER_SIGN_PRIVATE_KEY_PASSWORD` | The password of the private update key |

## Read first

1. `CLAUDE.md`
2. `docs/architecture.md` sections 1.3, 7.5, 8, 10.1, 11 and 12
3. `docs/standards.md` sections 12, 15 and 16
4. `docs/writing.md`, all sections, because this stage writes user documents
5. `docs/plans/18-desktop-settings.md`, the updates and About deliverables
6. The `cargo-packager` 0.11 documentation for macOS `.app` and `.dmg`, signing, notarization and the updater artifacts
7. The `cargo-packager-updater` 0.2 README, the endpoint and the response format
8. The `cargo-about` 0.9 documentation

## Scope

The stage can create or change these files only.

- `Cargo.toml` (the workspace version and the `repository` field of `[workspace.package]`)
- `apps/desktop/Cargo.toml` (`[package.metadata.packager]` and the features `engine-qwen3` and `engine-magpie`)
- `apps/desktop/src/updates/mod.rs` (only the value of the public key constant)
- `apps/desktop/bundle/` (app icon files, entitlements)
- `.config/about.toml`, `.config/about.hbs`
- `.github/workflows/release.yml`, `.github/workflows/feed.yml`
- `tools/release/` (the script that writes the feed files)
- `README.md`, `CONTRIBUTING.md`, `CHANGELOG.md`, `docs/release-notes/es/`
- `docs/adding-an-engine.md`, `docs/benchmarks.md`, `docs/release.md`

## Deliverables

### Tools

| Tool | Version | Use |
|---|---|---|
| `cargo-packager` | 0.11.8 | Builds `Antenna.app`, the `.dmg` and the signed update archive, signs and notarizes |
| `cargo-about` | 0.9.2 | Writes `THIRD_PARTY_LICENSES.html` from `Cargo.lock` |

Both are in the tool table of `docs/architecture.md` section 10.1.

### App bundle

1. The default features of `antenna-desktop` are `engine-qwen3` and `engine-magpie`. This stage adds both features to `apps/desktop/Cargo.toml`, each one forwarding to the feature with the same name in `antenna-engine-registry`. `engine-fake` is not a default feature, so the release build has no `fake` engine. The gallery is an example of `antenna-ui`, so the app binary does not contain it.
2. `[package.metadata.packager]` sets these values.
   1. The product name `Antenna` and the bundle identifier from the inputs
   2. The version from the workspace and the category `public.app-category.productivity`
   3. The minimum macOS version 14.0, the icon and the formats `app` and `dmg`
3. The app icon is the `Logo` app icon tile of stage 15, the `INK` tile with radius 9 in a 32 × 32 box. `apps/desktop/bundle/icon.svg` has the tile and the path data of `docs/design/tokens.md` section 6. `rsvg-convert -w <size> -h <size>` (Homebrew `librsvg`) renders each size of an `.iconset`, and `iconutil -c icns` makes `apps/desktop/bundle/icon.icns`.
4. The bundle contains `THIRD_PARTY_LICENSES.html` and the font license files in `Contents/Resources`.
5. The binary is built for `aarch64-apple-darwin` with the release profile.
6. The app uses the hardened runtime. The entitlements file contains no entitlement in this stage. See decision rule 2.
7. `candle` compiles its Metal kernels at runtime from source strings in the binary. The bundle needs no kernel files. AC-19-09 checks this on a clean Mac.

### About section

Stage 18 builds the About section of the Settings screen. This stage checks its content against the release.

1. The text says that Antenna is open source under GPL-3.0-or-later and that all processing occurs on the device. The English and the Spanish text are the texts of the About deliverable of stage 18.
2. The version comes from the workspace version.
3. The button "Third-party licenses" opens `THIRD_PARTY_LICENSES.html` from the bundle.
4. The Models screen and the About section show the license and attribution of each engine from its descriptor.

### Update feeds

Antenna has two update channels. Each channel is one JSON file in the GitHub release with the tag `update-feed`. That release exists only to hold the two files, and each new version replaces them.

| Channel | URL |
|---|---|
| Stable | `<repository>/releases/download/update-feed/stable.json` |
| Beta | `<repository>/releases/download/update-feed/beta.json` |

`<repository>` is the `repository` field of `[workspace.package]`. Stage 18 builds both URLs from it with `env!("CARGO_PKG_REPOSITORY")`.

Each file has the "one platform" format of `cargo-packager-updater`.

```json
{
  "version": "0.1.0",
  "pub_date": "2026-11-20T12:00:00Z",
  "url": "<repository>/releases/download/v0.1.0/Antenna_0.1.0_aarch64.app.tar.gz",
  "signature": "<content of the .sig file>",
  "format": "app",
  "notes": "<the CHANGELOG.md section of the version, in Markdown>",
  "notes_es": "<the content of docs/release-notes/es/<version>.md>"
}
```

1. `tools/release/feed.py` writes the file from the version, the archive URL, the `.sig` file, the `CHANGELOG.md` section and the Spanish notes. It uses only the Python standard library.
2. `notes` has the format of `cargo-packager-updater` and contains the English notes. `notes_es` is an extra field that only the app reads. `CHANGELOG.md` is in English. The Spanish notes of a version are in `docs/release-notes/es/<version>.md`. If that file is missing, `feed.py` writes no `notes_es`, and the app shows the English notes.
3. A tag without a pre-release part (for example `v0.1.0`) updates `stable.json` and `beta.json`.
4. A tag with a pre-release part (for example `v0.2.0-beta.1`) updates only `beta.json`.
5. The `notes` and `notes_es` fields are the source of the "What's new" panel of stage 18.
6. `apps/desktop/src/updates/mod.rs` contains the public update key and the two URLs as constants. The app checks the signature of each archive with the public key before it installs the archive.

### Release workflow

`.github/workflows/release.yml` runs when a tag `v*` is pushed.

1. It runs on a macOS Apple Silicon runner.
2. It uses the `release` environment, so a human approves each run.
3. It fails when the tag without the `v` is not the workspace version of `Cargo.toml`.
4. It runs `cargo xtask check`.
5. It runs `cargo about generate -c .config/about.toml .config/about.hbs -o THIRD_PARTY_LICENSES.html`.
6. It imports the certificate into a temporary keychain and runs `cargo packager --release`. The packager signs the app, notarizes and staples the `.dmg`, and writes the update archive and its `.sig` file.
7. It renames the `.dmg` to `Antenna-<version>-arm64.dmg` and writes `Antenna-<version>-arm64.dmg.sha256`.
8. It makes a draft GitHub release with the `.dmg`, the checksum, the update archive, the `.sig` file and the `CHANGELOG.md` section of the version.
9. A second workflow, `.github/workflows/feed.yml`, runs `on: release: types: [published]`. It writes the feed files with `tools/release/feed.py` and uploads them to the `update-feed` release with `gh release upload --clobber`.

### Documents

| File | Content |
|---|---|
| `README.md` | Update the existing file. Remove the development notice. Keep its sections and add what Antenna does in 3 sentences, 2 screenshots of the Studio and the Library, the supported languages, the system requirements (macOS 14, Apple Silicon, 16 GB of memory, 8 GB of free disk space for all voices), the install procedure, the build procedure, and the license |
| `CONTRIBUTING.md` | The read order of `CLAUDE.md`, the `cargo xtask check` command, the commit rules of `docs/standards.md` section 15, the STE-80 rule, and the link to `docs/adding-an-engine.md` |
| `docs/adding-an-engine.md` | The procedure of `docs/architecture.md` section 7.5 as a numbered walk-through, with the `fake` engine as the example, the engine crate layout, and the commands that prove each step |
| `docs/release.md` | The procedure to make a stable release and a beta release, the secrets, the update feeds, and the checks after a release |
| `CHANGELOG.md` | The "Keep a Changelog" format, with one section for 0.1.0 |
| `docs/benchmarks.md` | One row for each voice and quality with the date, the commit, the machine, TTFA, RTF, peak memory and WER, plus the listening sign-off of `docs/architecture.md` section 11.4 |

## Tasks

1. Set the workspace version to `0.1.0-beta.1`.
2. Run `cargo tree -p antenna-desktop -e features`. The default features are `engine-qwen3` and `engine-magpie`.
3. Write `.config/about.toml` with the same license allowlist as `.config/deny.toml`. Write `.config/about.hbs`. Run `cargo about generate` and examine the output.
4. Check the About section against the "About section" deliverable. If the text is wrong, write a "Blocked" section for stage 18, because stage 18 owns it.
5. Write `[package.metadata.packager]`. Write `apps/desktop/bundle/icon.svg` and make `icon.icns` with the commands of the "App bundle" deliverable.
6. Build an unsigned `.app` with `cargo packager --release` on the reference machine. Start it from Finder and do the "QA with real engines" script below.
7. Set the `repository` field of `[workspace.package]` to the repository URL of the inputs. Put the public update key in `apps/desktop/src/updates/mod.rs`.
8. Write `tools/release/feed.py`, `.github/workflows/release.yml` and `.github/workflows/feed.yml`.
9. Run `antenna-eval bench --voice <id> --quality <q> --check-budgets` and `antenna-eval wer` for each voice and each quality on the reference machine. Update `docs/benchmarks.md`.
10. Ask the project owner to do the listening check of `docs/architecture.md` section 11.4 and to sign off in `docs/benchmarks.md`.
11. Write `README.md`, `CONTRIBUTING.md`, `docs/adding-an-engine.md`, `docs/release.md`, `CHANGELOG.md` and `docs/release-notes/es/0.1.0.md`.
12. Run `cargo xtask check`. Fix each failure.
13. Do the quality gate in `docs/standards.md` section 20.
14. After the commit is on `main`, the project owner pushes the tag `v0.1.0-beta.1`, approves the workflow run and publishes the beta release. The feed workflow updates `beta.json`.
15. Make a second commit on `main` that sets the workspace version to `0.1.0` and adds the date of 0.1.0 to `CHANGELOG.md`.
16. After that commit is on `main`, the project owner pushes the tag `v0.1.0` and approves the workflow run.
17. Do the clean Mac test of AC-19-09 with the draft release.
18. The project owner publishes the release. The feed workflow updates both feed files.
19. Do the update test of AC-19-11.

### QA with real engines

Do these steps with the unsigned `.app` of task 6 and a new data directory. Write the result of each step in the stage report.

1. Do steps 2 to 11 of the manual QA script of stage 16 with the default voice of Spanish. In step 11, the file plays speech.
2. Do steps 2 to 11 of the same script with the default voice of English. In step 2, paste 3 paragraphs of English text, and the language select shows English.
3. Open Models. Click "Download" on the engine that is not installed. The row shows the progress. Click cancel. The row shows "Download" again.
4. Download the engine. It moves to "Installed". Click "Remove", then click the button of the confirmation popover. It moves to "Available" and the disk use decreases by the size in the popover.
5. Open Voices. Click the preview of a voice of each engine. Each sentence plays in less than 1 s after the click, when the engine is loaded.
6. Turn on pronunciation review. Paste the text "The zorblat sat on the mat." and play it. The reading view underlines "zorblat", and the tooltip shows the transcript of Whisper.

## Acceptance criteria

| ID | Criterion | Check |
|---|---|---|
| AC-19-01 | The release build has no `fake` engine | `cargo tree -p antenna-desktop -e features` shows no `engine-fake` |
| AC-19-02 | The third-party license file lists every dependency | `cargo about generate -c .config/about.toml .config/about.hbs` exits with code 0 and reports no missing license |
| AC-19-03 | The About text states GPL-3.0-or-later in both languages | `rg -c -F "GPL-3.0-or-later" apps/desktop/src/strings/en.rs apps/desktop/src/strings/es.rs` gives 1 or more for each file |
| AC-19-04 | The feed script writes a valid feed | `python3 -m unittest discover -s tools/release` passes. The test checks the fields, the version form of stage 18 and the RFC 3339 date |
| AC-19-05 | A pre-release tag changes only the beta feed | The same command. A test uses the tag `v0.2.0-beta.1` |
| AC-19-06 | The app bundle has a valid signature **(manual)** | `codesign --verify --deep --strict --verbose=2 Antenna.app` reports "valid on disk" and "satisfies its Designated Requirement" |
| AC-19-07 | The app is notarized **(manual)** | `spctl -a -vvv Antenna.app` reports "accepted" and "source=Notarized Developer ID" |
| AC-19-08 | The ticket is stapled **(manual)** | `xcrun stapler validate Antenna-0.1.0-arm64.dmg` reports "The validate action worked" |
| AC-19-09 | Antenna installs and works on a clean Mac **(manual)** | On a Mac with no Rust toolchain and no Xcode, download `Antenna-0.1.0-arm64.dmg` from the draft release and copy the app to Applications. Then do these steps. 1. Start the app. macOS shows no security warning. 2. Paste 3 sentences of English text, click outside the editor and press `space`. The status line of the toolbar row shows the download of the default voice, then the speech plays with the sentence highlight. 3. Click the third sentence. The audio continues from it. 4. Press `space` two times. The audio pauses and continues. 5. Export the document as MP3, WAV and OGG. Each file plays in QuickTime Player. 6. Open the Library. The document is there with the status "Exported". 7. Do step 6 of the "QA with real engines" script. Write the macOS version and the Mac model in `docs/release.md` |
| AC-19-10 | The `.dmg` is 80 MB or smaller **(manual)** | `stat -f %z Antenna-0.1.0-arm64.dmg` gives 83886080 or less |
| AC-19-11 | The update flow works **(manual)** | Install `v0.1.0-beta.1` and select the beta channel. Publish `v0.1.0`. In less than 15 s after a restart of the app, the Updates section shows 0.1.0 and "What's new" shows its notes. "Restart and update" installs it, and the About section shows 0.1.0 |
| AC-19-12 | The app rejects an archive with a wrong signature **(manual)** | Change one byte of the `.sig` value in a copy of the beta feed on a test release. The app reports a failed update and keeps the installed version |
| AC-19-13 | Each voice at each quality meets the budgets of `docs/architecture.md` section 8 **(manual)** | On the Apple M5 Pro 24 GB, `antenna-eval bench --voice <id> --quality <q> --check-budgets` exits with code 0 for each voice and each quality. `docs/benchmarks.md` has one row for each run with TTFA, RTF and peak memory |
| AC-19-14 | Each voice meets the WER target of `docs/architecture.md` section 11.2 **(manual)** | `docs/benchmarks.md` has the WER of each voice from `antenna-eval wer`, and each value is in the target |
| AC-19-15 | The project owner approved each default voice **(manual)** | The sign-off in `docs/benchmarks.md` |
| AC-19-16 | The documents obey STE-80 **(manual)** | The reviewer checks `README.md`, `CONTRIBUTING.md`, `docs/adding-an-engine.md` and `docs/release.md` against each rule of `docs/writing.md` section 1 and lists each finding and its fix in the stage report. `cargo xtask lint-repo` passes |
| AC-19-17 | The walk-through in `docs/adding-an-engine.md` works **(manual)** | A new agent session that did not write it follows it with a copy of the `fake` engine named `example`. `cargo xtask check` passes. The session then deletes the copy and writes the result in the stage report |
| AC-19-18 | The release has the `.dmg`, the checksum, the update archive, its signature and the changelog section **(manual)** | The GitHub release page lists the five items |
| AC-19-19 | The checksum is correct **(manual)** | `shasum -a 256 -c Antenna-0.1.0-arm64.dmg.sha256` reports "OK" |
| AC-19-20 | Both feeds point to 0.1.0 after the stable release **(manual)** | `curl -L` of each feed URL shows `"version": "0.1.0"` |
| AC-19-21 | The "QA with real engines" script passes **(manual)** | The stage report lists the result of each of the 6 steps |
| AC-19-22 | The release workflow rejects a tag that is not the workspace version **(manual)** | Push the tag `v9.9.9` to a fork. The workflow fails at the version step |

## Decision rules

1. If an input from the project owner is missing at task 7, do tasks 1 to 6 and 8 to 13. Then stop and write a "Blocked" section that lists the missing input.
2. If the notarized app fails to start or to compile the Metal kernels under the hardened runtime, find the entitlement that the error names. Add only that entitlement and write an ADR with the reason.
3. Each engine gets its data files as artifacts from the model store. If an engine needs a data file that is not an artifact, write a "Blocked" section.
4. If the `.dmg` is larger than 80 MB, run `cargo bloat --release --crates -p antenna-desktop` and list the largest crates in a "Blocked" section. Do not remove features.
5. If `cargo-packager` cannot sign, notarize or write the update archive, write a "Blocked" section. Do not change to a different packaging tool.
6. If a budget fails on the reference machine, apply `docs/architecture.md` section 14 rule 2. Do not publish a release with a budget failure.
7. If the updater offers the version that is already installed, the feed comparison is wrong. If the error is in `tools/release/feed.py`, fix it. If the error is in `apps/desktop/src/updates/`, write a "Blocked" section for stage 18. Do not publish a feed that offers the installed version.

### Facts to check

| Fact | Check |
|---|---|
| `cargo-packager` 0.11.8 writes `<name>_<version>_aarch64.app.tar.gz` and its `.sig` for the `app` format when `CARGO_PACKAGER_SIGN_PRIVATE_KEY` is set | Run task 6 with a test key and list the output files. Use the real names in the feed script |
| `cargo-packager-updater` returns no update when the feed version is equal to the installed version | Read `check_update` in the 0.2.3 source. The app of stage 18 also compares the version of the checked release with `Version::running` |
| `cargo-packager` 0.11.8 names the `.dmg` file | Run task 6 and list the output files. The workflow renames the file to `Antenna-<version>-arm64.dmg` |
| `cargo-packager` notarizes with `APPLE_API_KEY`, `APPLE_API_ISSUER` and `APPLE_API_KEY_PATH` | Read `crates/packager/src/codesign/macos.rs` at the 0.11.8 tag |

## Out of scope

1. Linux and Windows packages and feeds.
2. Delta updates.
3. Mac App Store distribution and sandbox entitlements.
4. Intel Macs.
5. A project website.
