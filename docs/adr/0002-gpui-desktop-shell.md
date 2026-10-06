# ADR 0002. GPUI for the desktop shell

## Status

Accepted, 2026-10-06.

## Context

The desktop app shows a text editor, a player with a waveform and lists of documents and voices. The design in `docs/design/` needs full radii, soft motion and a highlight that follows the playback. The release target is macOS on Apple Silicon. The budgets of `docs/architecture.md` section 8 ask for an editor at 120 fps with a 1 MB document and a start in 300 ms.

The candidates were GPUI, a web view shell and an immediate mode UI. A web view shell adds a JavaScript runtime and a second language. An immediate mode UI draws each frame again, also when nothing changes, so it fails the budget of 0% CPU use when nothing plays.

## Decision

1. The desktop app uses GPUI (package `gpui-pre`, pinned `=0.3.8`) and `gpui-component` (pinned `=0.7.1`).
2. Only `crates/ui` names `gpui_component`. `cargo xtask lint-repo` enforces this rule.
3. Components read only the design tokens of `antenna-ui`. Raw colors and sizes outside `crates/ui/src/theme/` fail `cargo xtask lint-repo`.
4. The GPUI executor is the only async code in Antenna. Library crates use threads and `flume`.

## Consequences

1. All UI code is Rust, so the apps share the types of `antenna-core` with no conversion layer.
2. GPUI changes its API between versions. The exact pins stop a silent upgrade. A human updates the pins in a dedicated change.
3. Screens cannot use a component library directly, so the look of the app stays consistent with Flow.
4. Accessibility depends on the GPUI accessibility API. `docs/architecture.md` section 15 keeps it as a future extension.
