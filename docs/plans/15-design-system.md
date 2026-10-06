# Stage 15. Design system

## Goal

Each token of `docs/design/tokens.md` is one constant, one field or one function in `antenna-ui`, by the name rule of this stage. The light theme matches the design and the dark theme obeys the derivation rules. The fonts and icons are in the bundle. Each Flow component renders in a gallery window that matches the design.

## Context

The project owner gave the visual design in `docs/design/`. This stage converts it into code before any screen exists. Stages 16, 17 and 18 build the screens only from the components and tokens of this stage. Thus no screen has a raw design value, and all screens look the same.

The stage starts after stage 01. It runs at the same time as stages 02 to 14. It creates the `crates/ui` crate (`antenna-ui`) with the theme, the components and the gallery. Stage 15 owns this crate. Stage 16 makes the `apps/desktop` crate, which uses `antenna-ui`.

`antenna-ui` has no workspace dependency. Thus a component cannot read app state, and the compiler enforces the rule of `docs/architecture.md` section 12.2.

## Read first

1. `CLAUDE.md`
2. `docs/design/tokens.md`, all sections
3. `docs/design/screens/*.png` and the HTML of `docs/design/pages/`. Read the HTML as text. Do not run its scripts in a browser, except in task 2
4. `docs/architecture.md` section 12
5. `docs/standards.md` sections 3, 13, 18 and 20
6. The `gpui-component` 0.7.1 theme API (`Theme`, `ThemeColor`, `ActiveTheme`) and its input, popover and notification primitives
7. The `gpui-pre` 0.3.8 APIs for `svg`, `img`, `canvas`, `Animation`, `AssetSource`, `Global`, `px` and `TextSystem::add_fonts`

## Scope

The stage can create or change these files only.

- `Cargo.toml` (the `crates/ui` member and the pins of `gpui` and `gpui-component` in `[workspace.dependencies]`)
- `.github/workflows/ci.yml` (the Linux packages that GPUI needs)
- `crates/ui/`
- `tools/fonts/`
- `docs/design/tokens.md` (only the rows that decision rule 1 adds)
- `docs/design/tokens-dark.md`, `docs/design/screens/2x/`, `docs/design/screens/actual/`

## Deliverables

### Crate

1. Dependencies `gpui = { package = "gpui-pre", version = "=0.3.8" }` and `gpui-component = "=0.7.1"`. `gpui-component` 0.7.1 pins `gpui-pre =0.3.8`, so the two pins must stay equal.
2. The crate has no features and no `serde` dependency. The gallery is the example `examples/gallery/`, so the app binary does not contain it.
3. `lib.rs` re-exports the public API. The public API is the token constants and types, `Flow`, `Appearance`, `Assets`, `Icon`, the functions of the "Theme API" deliverable and the components.
4. `gpui_component` is used only in this crate. The apps use the components of this crate.

### File layout

```
crates/ui/
├── Cargo.toml
├── assets/
│   ├── fonts/            # the bundled font files and OFL.txt for each family
│   └── icons/            # one SVG for each icon in docs/design/tokens.md section 6
├── examples/
│   └── gallery/
│       ├── main.rs       # opens the gallery window
│       └── <section>.rs  # one file for each gallery section
└── src/
    ├── lib.rs
    ├── theme/
    │   ├── mod.rs        # Flow, Appearance, install, set_appearance, colors, duration
    │   ├── color.rs      # Color (a const RGBA value), Colors (the semantic set), LIGHT
    │   ├── dark.rs       # DARK, written as literals
    │   ├── contrast.rs   # WCAG contrast ratio, used by tests
    │   ├── typography.rs # TypeStyle, the type tokens and the FlowStyled trait
    │   ├── space.rs      # the spacing scale
    │   ├── size.rs       # the size and layout tokens of tokens.md section 9 and the avatar specs
    │   ├── shape.rs      # radii and border widths
    │   ├── shadow.rs     # the shadow tokens
    │   ├── motion.rs     # ease_flow and the durations
    │   ├── wave.rs       # the waveform tokens of tokens.md section 8
    │   ├── fonts.rs      # embedded font bytes and their registration
    │   ├── icons.rs      # Icon enum and Assets, the AssetSource for the SVG files
    │   └── component.rs  # the gpui-component Theme built from Colors
    └── components/       # mod.rs and one file or directory for each component in the component table
```

### Token types

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Color(u32);                              // 0xRRGGBBAA
impl Color {
    pub const fn from_rgb(rgb: u32) -> Self;        // opaque
    pub const fn with_alpha(self, alpha: u8) -> Self;
}
impl From<Color> for gpui::Hsla;
impl From<Color> for gpui::Fill;

pub struct Colors { pub signal: Color, pub ink: Color, /* one field for each color token */ }
pub const LIGHT: Colors;
pub const DARK: Colors;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Appearance { Light, Dark }

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TypeStyle {
    pub family: FontFamily,                         // Reading, Display, Ui, Mono
    pub size: f32,                                  // logical pixels
    pub weight: u16,
    pub line_height: LineHeight,                    // Normal or a factor
    pub letter_spacing_em: f32,
}
pub trait FlowStyled: gpui::Styled {
    fn type_style(self, style: TypeStyle) -> Self;  // family, size, weight, line height and letter spacing
}

pub struct ShadowLayer { pub is_inset: bool, pub x: f32, pub y: f32, pub blur: f32, pub spread: f32, pub color: ColorToken, pub alpha: f32 }
pub struct Shadow { pub layers: &'static [ShadowLayer] }
pub fn ease_flow(t: f32) -> f32;                    // cubic-bezier(.34, 1.56, .64, 1)
pub const DURATION_STATE: Duration;                 // 300 ms, and the other durations of tokens.md section 5

pub const SPACE_M1: gpui::Pixels;                   // and each other token of tokens.md sections 3 and 9
pub enum Radius { Px(gpui::Pixels), Circle }        // Circle is half the side of the element
pub struct ToggleSize { pub track: gpui::Size<gpui::Pixels>, pub thumb: gpui::Pixels }
pub struct WidthRange { pub min: gpui::Pixels, pub max: gpui::Pixels }
pub enum Column { Fixed(gpui::Pixels), Flexible }
pub const LAYOUT_LIBRARY_COLUMNS: [Column; 7];

pub struct AvatarSpec { pub size: gpui::Pixels, pub lines: u16, pub r0: f32, pub span: f32, pub stroke: f32, pub polygon_stroke: Option<f32>, pub core_idle: Option<f32>, pub core_live: Option<(f32, f32)> }
pub const AVATAR_GALLERY: AvatarSpec;               // and AVATAR_INSPECTOR, AVATAR_PLAYER, AVATAR_ROW, AVATAR_TABLE
```

1. Each token of `docs/design/tokens.md` is one constant, one field or one function. This table gives the name rule and the Rust type of each kind of token. The tests run without a window.

| Kind | Name in code | Type |
|---|---|---|
| Color and alpha tokens | A field of `Colors` in snake case, for example `SIGNAL_TINT` is `signal_tint`, and a `ColorToken` variant in UpperCamelCase, for example `SignalTint` | `Color` |
| Type tokens | The token name | `TypeStyle` |
| `SPACE_*`, `BORDER_WIDTH`, `RING_*`, `REVIEW_UNDERLINE_WIDTH` and each `SIZE_*` and `LAYOUT_*` token with one value | The token name | `gpui::Pixels` |
| `RADIUS_*` | The token name | `Radius` |
| `SIZE_TOGGLE`, `SIZE_TOGGLE_COMPACT` | The token name | `ToggleSize` |
| `LAYOUT_WINDOW`, `LAYOUT_WINDOW_MIN`, `LAYOUT_GALLERY_WINDOW` | The token name | `gpui::Size<gpui::Pixels>` |
| `LAYOUT_SETTINGS_NAV` | The token name | `WidthRange` |
| `LAYOUT_LIBRARY_COLUMNS`, `LAYOUT_MODELS_COLUMNS` | The token name | `[Column; N]` |
| `SHADOW_*` | The token name | `Shadow` |
| `EASE_FLOW` | The function `ease_flow` | `fn(f32) -> f32` |
| `DURATION_*`, `CARET_PERIOD`, `TOAST_DURATION` | The token name | `Duration` |
| `AVATAR_PHASE_RATE` and the `WAVE_*` and `PLAYHEAD_*` tokens, except the color tokens `WAVE_IDLE` and `WAVE_DOT` | The token name | `f32` in the unit of the token, because the CPU mask and the avatar geometry use them |
| `AVATAR_*` sizes | The token name | `AvatarSpec` |

2. These names in `theme/` are not tokens, and the name test skips them. `LIGHT`, `DARK`, the font byte constants with the prefix `FONT_`, and the `Icon` variants, which AC-15-12 checks.
3. `ColorToken` is an enum with one variant for each field of `Colors`. A shadow layer names its color token, so the light and the dark theme resolve it to their own value.
4. Components read colors only through `colors(cx)`. Thus a change of appearance changes all components.

### Theme API

```rust
pub struct Flow { pub appearance: Appearance, pub colors: &'static Colors, pub motion: Motion }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Motion { Full, Reduced }
impl gpui::Global for Flow;

pub struct Assets;                                  // embeds the icons. The app passes it to Application::with_assets
impl gpui::AssetSource for Assets;

pub fn install(cx: &mut App, appearance: Appearance);        // registers the fonts, sets Flow and the gpui-component theme
pub fn set_appearance(cx: &mut App, appearance: Appearance); // changes Flow and the gpui-component theme, then refreshes all windows
pub fn colors(cx: &App) -> &'static Colors;
pub fn duration(cx: &App, base: Duration) -> Duration;       // zero when Flow.motion is Reduced
```

1. The app calls `install` one time, before it opens the first window.
2. `install` reads the system setting "Reduce motion" and stores `Motion::Reduced` or `Motion::Full` in `Flow`.
3. Each animation gets its length from `duration`. Avatars and the caret do not animate when the motion is `Reduced`.
4. The name `Flow` prevents a clash with `gpui_component::Theme`.

### Dark theme

The design has no dark palette. `theme/dark.rs` contains values that come from these rules. `docs/design/tokens-dark.md` lists each value next to its light value for the approval of the project owner.

1. For each color except `SIGNAL`, `WARNING_TEXT`, `REVIEW_UNDERLINE`, `ON_SIGNAL` and `ON_INK`, convert the light value to OKLCH. Keep the chroma and the hue. Set the lightness to `L' = 0.17 + (1 − L) × 0.95`.
2. For `SIGNAL`, keep the hue and the chroma. Increase the lightness in steps of 0.01 until the contrast ratio is 4.5 or more against the dark `CANVAS` and the dark `SIDE`. If the color is outside sRGB, decrease the chroma until it is inside.
3. For `WARNING_TEXT` and `REVIEW_UNDERLINE`, use the rule of `SIGNAL`.
4. `ON_SIGNAL` is `#FFFFFF` or the dark `CANVAS`, the one with the higher contrast on the dark `SIGNAL`. `ON_INK` is the dark `CANVAS`.
5. Alpha tokens keep their alpha and use the dark base color.
6. Round each value to the nearest 8-bit RGB value. Write the result as a literal.

The dark theme resolves each shadow layer with the first rule that matches.

1. A layer with a color token other than `INK` uses the dark value of that token and keeps its alpha.
2. A ring layer has offset 0, blur 0 and a spread more than 0. A ring layer with the color `INK` uses the dark `DIVIDER` at alpha 1.
3. Each other layer with the color `INK` uses black at alpha `min(1, 2a)`, where `a` is the light alpha.

### Fonts

GPUI cannot set variable font axes (see "Facts to check"). Thus the app uses static instances of Fraunces.

1. `tools/fonts/` is a `uv` project with `fonttools` pinned. Its script downloads the variable Fraunces font and the static Figtree and DM Mono fonts from the `google/fonts` repository at a pinned commit.
2. The script makes two static Fraunces instances with `fontTools.varLib.instancer`. The instances get new family names, because the OFL needs a new name for a modified font when the font has a Reserved Font Name.

| Instance | Family name | `opsz` | `wght` | `SOFT` | `WONK` | Token users |
|---|---|---|---|---|---|---|
| Reading | `Antenna Reading` | 20 | 380 | 50 | 0 | `READING` |
| Display | `Antenna Display` | 24 | 600 | 100 | 0 | `BRAND_SIDEBAR`, `BRAND_ABOUT`, `VOICE_NAME` |

3. The bundled Figtree weights are 400, 500, 600 and 700. The bundled DM Mono weights are 400 and 500.
4. Each family directory in `assets/fonts/` has its `OFL.txt`.
5. `theme/fonts.rs` embeds the files with `include_bytes!`. `install` registers them with `cx.text_system().add_fonts(...)`.
6. Each font file is larger than the 64 KB limit of `clippy::large_include_file`. Each `FONT_` constant has `#[expect(clippy::large_include_file, reason = "a font file is one asset")]`. No other file of the crate has this exception.

### Icons

Each icon in `docs/design/tokens.md` section 6 is one SVG file in `assets/icons/`. The path data is the exact path data of the design page. The icon color comes from the text color of the element, so each SVG uses `currentColor`. `theme/icons.rs` has an `Icon` enum with one variant for each file, and `Assets`, which embeds the files.

### Components

Each component is one file or one directory in `components/`. Each component takes its values only from the theme. Each component has the states in this table and shows each state in the gallery. A row with the mark "derived" has no design source, so the project owner approves it in AC-15-15.

| Component | Variants and states | Design source |
|---|---|---|
| `Button` | Primary (`INK` fill), Secondary (`CANVAS` fill, `OUTLINE_STRONG` ring), Ghost (no fill). Heights `SIZE_CONTROL_S`, `SIZE_CONTROL_M` and `SIZE_CONTROL_L`. Optional leading icon. Hover, pressed, disabled, focus. Progress, which shows a `ProgressRing` in place of the icon and keeps the label (derived) | Top bar "Exportar", "Usar voz", "Usar por defecto" |
| `IconButton` | Round, `SIZE_CONTROL_XS` to `SIZE_CONTROL_L`. Plain, active (`FILL`) | Top bar, transport, models |
| `PlayButton` | Large (`SIZE_CONTROL_XL`, `INK`, `SHADOW_PLAY`), Row (`SIZE_CONTROL_S`), Gallery (`SIZE_CONTROL_L`). Idle, live (`SIGNAL` fill, `SHADOW_PLAY_LIVE`), disabled. Play and pause glyphs. Download, which shows the `export` icon in place of the play glyph, and progress, which shows a `ProgressRing` (derived) | Player, library rows, voice gallery |
| `Segmented` | Track `SURFACE` or `TRACK`. Text options or mono options. Selected option with `SHADOW_SEGMENT` | Library filter, voice languages, format, settings |
| `Toggle` | `SIZE_TOGGLE` and `SIZE_TOGGLE_COMPACT`. On and off. The thumb moves with `EASE_FLOW` in `DURATION_LAYOUT` | Settings, inspector |
| `Select` | Trigger of `SIZE_CONTROL_M` with an optional leading `AVATAR_ROW` avatar and a chevron. Open state shows a menu in a `Popover` | Inspector, settings |
| `Popover` | Surface with `RADIUS_POPOVER`, `SHADOW_POPOVER` and padding `SPACE_M4` | Novedades, select menus |
| `TextArea` | Multi-line input with soft wrap and the `READING` type. Paste, IME and the undo of the input primitive. Empty with a placeholder in `READING_PENDING`, focus | Studio editor |
| `TextField` | Single-line input with the `NAV_SELECTED` type and no fill. Idle, editing with a `RING_WIDTH` ring in `OUTLINE` | Top bar title |
| `Slider` | Value from 0 to 1. Track `SEPARATOR` of `SIZE_TRACK` with `RADIUS_TRACK`, fill `INK`, thumb `CANVAS` of `SIZE_SLIDER_THUMB` with `SHADOW_SLIDER_THUMB`. Click and drag | `estudio.html` interpretation sliders, used for the volume |
| `Toast` | `Popover` surface with one line of `BODY` text and one `SMALL_ACTION` action, `LAYOUT_TOAST` wide. It hides after `TOAST_DURATION` or after a click on its action (derived) | Export done |
| `Link` | `SMALL_ACTION` text in `INK`, no fill. Hover, focus, disabled | Inspector "Explorar", popover actions |
| `Breadcrumb` | A root `Link` in `TEXT_SECONDARY`, the `chevron-down` separator in `INK_DISABLED`, and the title button with `RADIUS_TOOL` in `NAV_SELECTED`. A click on the title calls a callback | Top bar |
| `AvatarButton` | Round, `SIZE_AVATAR_BUTTON`, `CANVAS` fill with an `AVATAR_ROW` avatar. Not selected (`RING_WIDTH` ring in `OUTLINE`, `INK_MUTED` ink), selected (`RING_WIDTH_SELECTED` ring in `INK`, `INK` ink). The ring changes in `DURATION_STATE` | Inspector other voices |
| `Tooltip` | `Popover` surface with one or more lines of `SMALL` text, at most `LAYOUT_TOOLTIP` wide, padding `SPACE_S3` `SPACE_M1`. It shows on hover (derived) | Review transcript |
| `NavItem` | Icon, label, count. Selected (`CANVAS`, `SHADOW_RAISED`, icon `SIGNAL`, `NAV_SELECTED`), not selected (`NAV`), collapsed (icon only) | Sidebar |
| `RecentRow` | Name with ellipsis, duration. Current and not current | Sidebar |
| `UnreadBadge` | Count pill and dot (`SHADOW_BADGE_RING`) | Sidebar |
| `SubNavItem` | `SIZE_CONTROL_L` high, padding 0 `SPACE_M1`, `RADIUS_ROW`. Not selected (`NAV` in `TEXT_SECONDARY`, no fill), selected (`NAV_SELECTED` in `INK`, `SURFACE` fill) | Settings section list |
| `MonogramTile` | `SIZE_MONOGRAM` square with `RADIUS_ITEM`, `SURFACE` fill and two letters in `DATA_MONOGRAM` | Models rows |
| `Chip` | Engine chip (cube icon and mono text), role badge | Voice gallery, models |
| `Key` | One keyboard key of `SIZE_KEY` with `SHADOW_KEY` | Settings |
| `SearchField` | Optional shortcut hint. Empty, typed, focus | Library, voices |
| `ProgressBar` | Neutral (`INK` fill) and signal (`SIGNAL` fill) | Models |
| `ProgressRing` | A `SIZE_PROGRESS_RING` ring with an `OUTLINE` track and the fraction from 0 to 1 in `SIGNAL`. `Button`, `PlayButton` and `StatusIndicator` use it. No other component draws a progress ring | Library status, Export button |
| `StatusIndicator` | Draft (dashed ring), generating (`ProgressRing` with percent), ready (check disk), exported (arrow) with label | Library |
| `TimestampPill` | Current (`SIGNAL` text on `SIGNAL_TINT_STRONG`) and not current (`PENDING` text) | Studio gutter |
| `SectionLabel` | `LABEL` text in `TEXT_LABEL`, with an optional action link | Inspector, library groups |
| `SettingsRow` | Label, optional description, trailing control | Settings |
| `TableRow` | Grid of cells with a `Column` list, `SIZE_TABLE_ROW` high, `DIVIDER_SUBTLE` at the bottom, `SIDE` fill when live | Library |
| `Divider` | Horizontal and vertical, `DIVIDER`, `DIVIDER_SUBTLE` or `SEPARATOR` | All screens |
| `EmptyState` | One line of `BODY` text in `TEXT_TERTIARY` and one secondary button, centered (derived) | None |
| `VoiceAvatar` | The five `AvatarSpec` tokens. `AvatarState::Idle` and `AvatarState::Live { phase }` | Gallery, inspector, player, tables |
| `Waveform` | Generated, played, pending dots, playhead, empty | Player |
| `Logo` | Mark on light fill, mark on `INK` tile, app icon tile (`SIZE_LOGO_ABOUT`) | Sidebar, about |

### Avatar geometry

```rust
pub enum AvatarState { Idle, Live { phase: f32 } }
pub(crate) fn geometry(voice_id: &str, spec: &AvatarSpec, state: AvatarState) -> AvatarGeometry;  // lines, polygon, core radius
```

`components/avatar/geometry.rs` is a pure function. It returns the lines, the polygon and the core radius of `docs/design/tokens.md` section 7. The render code draws the result with a GPUI path in a `canvas`.

### Waveform

```rust
pub struct WaveformData {
    pub peaks: Arc<[f32]>,       // linear peaks of the generated audio, from the start of the document
    pub peaks_per_second: u32,
    pub generated: Duration,     // the end of the generated audio
    pub total: Duration,         // the width of the waveform in time
    pub position: Duration,      // the play position
}
pub(crate) fn bar_levels(waveform: &WaveformData, bar_count: usize) -> Vec<f32>;   // tokens.md section 8 step 2
```

The `Waveform` component takes a `WaveformData` and a seek callback `Fn(Duration)`. A click or a drag calls the callback with the time under the pointer, clamped to `generated`.

GPUI has no blur filter. The waveform uses a CPU mask that gives the same result as the design.

1. `components/waveform/mask.rs` is a pure function. It takes the bar heights, the width, the height and the scale factor, and returns an 8-bit alpha mask.
2. It draws the bars as rounded rectangles into the mask. It applies a separable gaussian blur with σ = `WAVE_BLUR_SIGMA` × scale factor. Then it maps each alpha value `a` to `clamp(WAVE_ALPHA_GAIN · a − WAVE_ALPHA_OFFSET, 0, 1)`.
3. The component makes two images from the mask, one in `SIGNAL` and one in `WAVE_IDLE`. It draws the first image clipped to the left of the playhead and the second image clipped to the right.
4. The component makes the mask again only when the peaks (`Arc::ptr_eq`), `generated`, `total` or the size change. A change of the play position changes only the clip. Thus a frame during playback does no blur work.
5. The pending dots and the playhead are drawn on top as GPUI elements.

### Motion

1. `ease_flow` solves the cubic Bézier for `x` and returns `y`. The curve overshoots to 1.098 at `x = 0.573`.
2. Each animated change uses `ease_flow`, except color changes of reading text, which use a linear curve.
3. Each animation gets its length from `duration(cx, base)`.

### Gallery

`cargo run -p antenna-ui --example gallery` opens a window of `LAYOUT_GALLERY_WINDOW` at the screen position (100, 100). The window has one section for each component with each variant and state. A segmented control at the top switches between the light and the dark theme. The gallery uses the example data of the design pages, for example the voice names of `voces.html`.

### Contrast test

`theme/contrast.rs` implements the WCAG 2.2 contrast ratio. A test lists each pair of a text color and a background color that the components use. These pairs have a ratio of 4.5 or more in the light and in the dark theme. The design uses some text colors with a lower ratio on purpose. The test lists these exceptions with the measured light ratio.

| Text token | Light ratio on `CANVAS` | Reason in the design |
|---|---|---|
| `TEXT_LABEL` | 4.32 | Small section labels next to bold content |
| `TEXT_DATA` | 3.47 | Secondary data next to the primary value |
| `PENDING` | 2.25 | A timestamp that is not current |
| `READING_PENDING` | 2.11 | Text that has no audio yet |
| `INK_DISABLED` | 1.79 | Disabled icons, which WCAG exempts |

## Tasks

1. Read `docs/design/tokens.md` and the HTML of each page. List each component and each state that the screens use.
2. Render each design page at a scale of 2 with Chrome headless (`--force-device-scale-factor=2`) into `docs/design/screens/2x/<page>.png`. Use the window size of each board from `docs/design/source-bundle.html`.
3. Write `tools/fonts/` and run it. Copy the instances and the static fonts with their `OFL.txt` into `assets/fonts/`.
4. Extract the path data of each icon from the HTML pages. Write each SVG file.
5. Add `crates/ui` to the workspace and add the pins to `[workspace.dependencies]`.
6. Add the Linux packages that GPUI needs to `.github/workflows/ci.yml`. Run the CI and add each package that the build reports as missing.
7. Write `theme/color.rs` and the other token modules from `docs/design/tokens.md`.
8. Write a throwaway script in the scratch directory that calculates the dark values with the rules of the "Dark theme" deliverable. Write the results into `theme/dark.rs` and `docs/design/tokens-dark.md`. Do not commit the script.
9. Write `theme/mod.rs`, `theme/component.rs`, `theme/fonts.rs` and `theme/icons.rs`.
10. Write `components/avatar/geometry.rs`, `components/waveform/mask.rs` and `bar_levels` with their tests first.
11. Write each component. Start with `Button`, `Popover`, `Segmented`, `Toggle` and `Select`, because the other components use them.
12. Write the gallery.
13. Take screenshots of the gallery in both themes with `screencapture -x -R100,100,1200,900 docs/design/screens/actual/gallery-<theme>.png`.
14. Compare each component in the screenshots with the same element in `docs/design/screens/2x/`. Fix each difference in size, position, spacing, color or type.
15. Run `cargo xtask check`. Fix each failure.
16. Do the quality gate in `docs/standards.md` section 20.

## Acceptance criteria

| ID | Criterion | Check |
|---|---|---|
| AC-15-01 | No raw design value is outside `crates/ui/src/theme/` | `cargo xtask lint-repo` passes |
| AC-15-02 | Each light color token has the value of `docs/design/tokens.md` | Test `light_colors_match_tokens_file`, which parses the color tables of `tokens.md` with `include_str!` |
| AC-15-03 | Each Rust token name in each table of `tokens.md` has its code name of the name table, and each code name of `theme/` that is not in the exempt list is in `tokens.md` | Test `tokens_file_and_code_have_same_names`. It reads `tokens.md` and the `theme/` source files with `include_str!`, collects each `pub const` name, each field of `Colors` and the function `ease_flow`, and compares the two sets with the name rule |
| AC-15-04 | The dark values obey the derivation rules | Test `dark_colors_follow_derivation_rules`, which calculates each dark value from the light value and compares each channel with a tolerance of 1 |
| AC-15-05 | Each text pair meets WCAG AA in both themes, except the listed exceptions | Test `text_pairs_meet_wcag_aa` |
| AC-15-06 | `ease_flow` matches the CSS curve | Test `ease_flow_matches_css_samples` with the values 0.4039 at 0.1, 0.8163 at 0.25, 1.0874 at 0.5, 1.0596 at 0.75 and 1.0126 at 0.9, with a tolerance of 0.001 |
| AC-15-07 | Each animation has zero duration when "Reduce motion" is on | Test `duration_is_zero_when_motion_is_reduced` |
| AC-15-08 | The avatar geometry follows `tokens.md` section 7 | Tests `avatar_is_same_when_voice_is_same`, `avatar_first_line_points_up_when_idle`, `avatar_live_value_is_clamped_when_phase_varies` and `avatar_shape_changes_when_phase_changes` |
| AC-15-09 | Adjacent tall bars merge in the waveform mask and a bar of minimum height stays separate | Tests `waveform_mask_merges_neighbors_when_tall` and `waveform_mask_keeps_bars_apart_when_short` |
| AC-15-10 | A frame during playback does not rebuild the waveform mask | Test `waveform_reuses_mask_when_only_position_changes` |
| AC-15-11 | The bar levels follow `tokens.md` section 8 step 2 | Tests `bar_level_is_min_when_peak_is_below_floor` and `bar_level_is_max_when_peak_is_full_scale` |
| AC-15-12 | Each icon of `tokens.md` section 6 has an SVG file and an `Icon` variant | Test `each_icon_has_an_asset` |
| AC-15-13 | The fonts load and each type token finds its family | Test `each_type_style_resolves_a_font`, which runs with a GPUI test app context |
| AC-15-14 | Each font has its license | Each family directory in `crates/ui/assets/fonts/` has `OFL.txt` |
| AC-15-15 | The light gallery matches the design, and the derived rows are approved **(manual)** | The project owner opens `docs/design/screens/actual/gallery-light.png` and the matching image of `docs/design/screens/2x/` in Preview at the same scale. For each component, the owner measures the height, the padding and the gap with the rectangle selection. Each value is in 1 px of the design at scale 1. Each color that Digital Color Meter reads is the token value. The owner approves each derived row of the component table and signs off in the stage report |
| AC-15-16 | The project owner approves the dark theme **(manual)** | The project owner reads `docs/design/tokens-dark.md`, looks at `gallery-dark.png` and signs off in the stage report |
| AC-15-17 | The project owner accepts the contrast exceptions **(manual)** | The project owner signs off on the exception table of the "Contrast test" deliverable, or gives darker values |
| AC-15-18 | No component contains a second surface with its own fill and ring **(manual)** | The reviewer checks each component against `docs/architecture.md` section 12.1 rule 6 and lists the result for each component in the stage report |
| AC-15-19 | The gallery shows each component in each state of the component table **(manual)** | The reviewer opens the gallery and checks each state of each row of the component table. The stage report lists each row with "shown" or the missing state |
| AC-15-20 | `gpui_component` occurs only in `crates/ui` | `rg -l gpui_component --glob '*.rs' --glob '!crates/ui/**' .` returns no lines |

## Decision rules

1. If a value that a component needs is not in `tokens.md`, find it in the HTML pages. If the pages have it, add a row to `tokens.md` with a Rust name, the value and the page name. List the addition in the stage report. If the pages do not have it, write a "Blocked" section.
2. If two pages use different values for one role, use the value of the five app screens. Do not use the value of the Flow page. List the case in the stage report.
3. If a `gpui-component` primitive cannot show a component with the token values, build the component with GPUI elements in `components/`. Do not fork `gpui-component`.
4. If GPUI cannot draw stroked paths with round caps in `canvas`, draw the avatar into a CPU mask with the method of the waveform. Do not add a crate.
5. If a design color pair fails the contrast test and is not in the exception table, write a "Blocked" section. Do not change the color.
6. If `gpui-component` 0.7.1 does not compile with `gpui-pre =0.3.8` on a platform, write a "Blocked" section. Do not change the pins.
7. If `gpui-pre` 0.3.8 cannot draw an RGBA image from memory, write a "Blocked" section. Do not add a crate and do not remove the joined shape of the waveform.
8. If GPUI does not report the "Reduce motion" setting, write a "Blocked" section. Do not add a crate. Do not read the setting with `unsafe` code, because `docs/standards.md` section 9 permits `unsafe` only in `antenna-ml`.
9. If `gpui-component` 0.7.1 has no select primitive that fits the token values, build `Select` from a `Button` and a `Popover` of this stage.
10. If `gpui::px` is not a `const fn` in `gpui-pre` 0.3.8, make each space, size and layout token a `pub fn` without arguments that returns `Pixels`. Keep the token names.
11. If `gpui-component` 0.7.1 has no multi-line input with IME, build `TextArea` on the GPUI `EntityInputHandler` example of `gpui-pre` 0.3.8. Write the result in the stage report.

### Facts to check

| Fact | Check |
|---|---|
| GPUI 0.3.8 cannot set the `SOFT` and `opsz` axes of a variable font | Read the `FontFeatures` and `Font` types of `gpui-pre` 0.3.8. If GPUI can set font variations, use the variable font and skip the instances. Write the result in the stage report |
| Fraunces, Figtree and DM Mono have no Reserved Font Name | Read the `OFL.txt` of each family. If a family has no Reserved Font Name, the instance can keep the original name, but this stage still renames it for clarity |
| `gpui-pre` 0.3.8 can draw an RGBA image from memory | Find `RenderImage` or the equivalent type and its `img` source. Decision rule 7 applies when it does not exist |
| `gpui-pre` 0.3.8 has a `canvas` element with a path builder that strokes | Find `PathBuilder::stroke` or the equivalent. Decision rule 4 applies when it does not exist |
| GPUI reports the system setting "Reduce motion" | Find the function. Decision rule 8 applies when it does not exist |
| `gpui::px` is a `const fn` | Read the `px` function of `gpui-pre` 0.3.8. Decision rule 10 applies when it is not |
| The icon paths of `tokens.md` section 6 are complete | Search each page for `<path d=` and compare with the icon list |

## Out of scope

1. Screens, the app state and the sidebar behavior. Stages 16, 17 and 18 own them.
2. The app icon files in the bundle. Stage 19 makes them from the `Logo` component.
3. Components for out of scope features, for example the group of interpretation sliders, the pause tag and the speed control.
4. Localization of the gallery text.
