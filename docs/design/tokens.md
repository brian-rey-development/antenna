# Flow Design Tokens

This file is the canonical list of the design values of Antenna. The values come from the pages in `docs/design/pages/`. Stage 15 converts each row into one constant, one field or one function in `crates/ui/src/theme/`, by the name rule of stage 15. The dark values are in `docs/design/tokens-dark.md`. A value that is not in this file does not occur in the app.

A row with the mark "derived" has no source in the design. The project owner approves each derived row in the gallery of stage 15.

The design names are Spanish because the design is in Spanish. The token names in Rust are English. Each table gives both names.

## 1. Color

### 1.1 Palette of the design system

These ten colors are the named palette of the Flow page.

| Design name | Rust token | Light value | Role |
|---|---|---|---|
| Señal | `SIGNAL` | `#2D4BE0` | The only accent. It marks what is alive, which is the playhead, the played audio, the synthesis front, the current location and the count of new release notes |
| Tinta | `INK` | `#111111` | Primary text, primary button fill, toggle fill when on, avatar ink when idle |
| Texto 2 | `TEXT_SECONDARY` | `#55554F` | Secondary text, nav labels that are not selected, descriptions |
| Texto 3 | `TEXT_TERTIARY` | `#6A6A65` | Labels of fields, icons that are not selected, hints |
| Pendiente | `PENDING` | `#ADADA8` | Timestamps that are not current |
| Onda | `WAVE_IDLE` | `#CDCDC8` | Waveform bars that are generated but not played |
| Línea | `LINE` | `#E8E8E4` | The border between the sidebar and the workspace |
| Superficie | `SURFACE` | `#F4F4F1` | Sidebar fill, search field fill, segmented track in the top bar, model monogram fill |
| Lateral | `SIDE` | `#FAFAF8` | Inspector fill, fill of the library row that plays |
| Lienzo | `CANVAS` | `#FFFFFF` | Workspace fill, selected nav item, button fill |

### 1.2 More values that the screens use

The screens use these values in addition to the palette. Each value has one role.

| Rust token | Light value | Role |
|---|---|---|
| `TEXT_STRONG` | `#3F3F3B` | Toolbar button text, table cells, transport icons, chip text |
| `TEXT_LABEL` | `#7A7A74` | Section labels ("Recientes", "Voz", "Interpretación"), library group labels |
| `TEXT_DATA` | `#8A8A84` | Counts, durations, file sizes, chevrons, relative dates |
| `READING_DONE` | `#1C1C1A` | Reading text that is generated |
| `READING_PENDING` | `#B3B3AE` | Reading text that is not generated yet |
| `INK_MUTED` | `#9A9A94` | Avatar ink of a recent voice that is not selected |
| `INK_DISABLED` | `#C2C2BD` | Disabled icons, which are the breadcrumb separator and the play icon of a document with no audio |
| `WAVE_DOT` | `#D3D3CE` | The dots of the waveform after the synthesis front |
| `TOGGLE_OFF` | `#D8D8D3` | Toggle track when off |
| `OUTLINE_STRONG` | `#DEDED9` | Ring of secondary buttons and of round play buttons in the gallery |
| `OUTLINE` | `#E2E2DD` | Ring of selects, of avatar buttons that are not selected, of keyboard keys, track of the progress ring |
| `SEPARATOR` | `#E6E6E2` | Vertical separators in bars, slider track, ring of the shortcut hint in the search field |
| `DIVIDER` | `#EDEDE9` | Borders of the top bar, the player bar and the inspector, section dividers, progress track |
| `DIVIDER_SUBTLE` | `#F2F2EE` | Dividers between table rows, settings rows and the editor toolbar |
| `TRACK` | `#EFEFEB` | Segmented track inside the inspector and the settings |
| `FILL` | `#F1F1EE` | Active icon button, pause tag in the text, speed control |
| `CHIP` | `#F2F2EF` | Engine chip, role badge, cancel button |
| `WARNING_TEXT` | `#9A3412` | Text that needs attention, which is a license that does not permit commercial use and the job error in the Studio status line |
| `REVIEW_UNDERLINE` | `#9A3412` | The wavy underline of a word that pronunciation review flags (derived) |
| `ON_SIGNAL` | `#FFFFFF` | Text and icons on a `SIGNAL` fill |
| `ON_INK` | `#FFFFFF` | Text and icons on an `INK` fill, for example the primary button and the play button |

### 1.3 Alpha values

| Rust token | Value | Role |
|---|---|---|
| `SIGNAL_TINT` | `SIGNAL` at alpha `0x12` (7%) | Fill and 4 px spread of the current sentence |
| `SIGNAL_TINT_STRONG` | `SIGNAL` at alpha `0x14` (8%) | Fill of the current timestamp, avatar fill while the voice plays |
| `INK_TINT` | `INK` at alpha `0x08` (3%) | Avatar fill when idle |

## 2. Typography

### 2.1 Families

| Family | Use | License |
|---|---|---|
| Fraunces | Brand, voice names, reading text | SIL Open Font License 1.1 |
| Figtree | All interface text | SIL Open Font License 1.1 |
| DM Mono | Data, for example times, counts, sizes, formats and keys | SIL Open Font License 1.1 |

Fraunces is a variable font with the axes `opsz`, `wght`, `SOFT` and `WONK`. The design uses `WONK 0` in all places.

### 2.2 Type styles

The base interface text is Figtree 13 px with line height 1.45. The Flow page shows 13.5 px as the "Interfaz" sample, but all five screens use 13 px. The screens are correct.

| Rust token | Family | Size (px) | Weight | Line height | Letter spacing | Axes | Use |
|---|---|---|---|---|---|---|---|
| `BRAND_DISPLAY` | Fraunces | 88 | 600 | 1 | -0.045em | SOFT 100 | Wordmark on the Flow page only |
| `BRAND_SIDEBAR` | Fraunces | 21 | 600 | 1 | -0.03em | SOFT 100 | Wordmark in the sidebar |
| `BRAND_ABOUT` | Fraunces | 24 | 600 | normal | -0.03em | SOFT 100 | Wordmark in the About section |
| `VOICE_NAME` | Fraunces | 22 | 600 | 1.1 | -0.025em | SOFT 100 | Voice name in the inspector and the gallery |
| `READING` | Fraunces | 20 | 380 | 1.7 | -0.006em | SOFT 50 | Document text in the Studio |
| `STAT_VALUE` | Figtree | 20 | 600 | normal | -0.02em | | Values in the Models screen header |
| `SECTION_TITLE` | Figtree | 15 | 700 | normal | -0.01em | | Settings sections, popover title |
| `MODEL_NAME` | Figtree | 14.5 | 600 | normal | | | Engine name in the Models screen |
| `NAV` | Figtree | 14 | 500 | normal | | | Nav items and settings sub-nav items that are not selected, settings row labels |
| `NAV_SELECTED` | Figtree | 14 | 600 | normal | | | The selected nav item, the selected settings sub-nav item, the top bar title |
| `ROW_TITLE` | Figtree | 14 | 500 | normal | | | Document name in the library |
| `BODY` | Figtree | 13 | 400 | 1.45 | | | Default text |
| `BODY_STRONG` | Figtree | 13 | 600 | 1.45 | | | Button labels, current recent document, voice name in the player |
| `SMALL` | Figtree | 12.5 | 400 | normal | | | Voice traits in the inspector |
| `SMALL_ACTION` | Figtree | 12.5 | 600 | normal | | | Popover buttons, links, the toast action |
| `LABEL` | Figtree | 12 | 600 | normal | | | Section labels, table headers, stat labels |
| `BADGE` | Figtree | 11.5 | 600 | normal | | | Role badge in the Models screen |
| `COUNTER` | Figtree | 11 | 700 | normal | | | Unread count in the badge |
| `DATA` | DM Mono | 12 | 400 | normal | | | Times in the player, durations, languages, sizes, keys |
| `DATA_MONOGRAM` | DM Mono | 12 | 500 | normal | | | Model monograms |
| `DATA_SMALL` | DM Mono | 11.5 | 400 | normal | | | Counts in titles, character count, timestamps in the gutter, format options |
| `DATA_TINY` | DM Mono | 11 | 400 | normal | | | Nav counts, chips, download progress, units |

## 3. Spacing

Each gap, padding and margin in the app is one value of this scale. The values are in logical pixels. The design places the timestamp pill 7 px from the top of its paragraph. The app uses `SPACE_S3` (8 px) there, because 7 px is not on the scale.

| Rust token | Value | Typical use |
|---|---|---|
| `SPACE_2XS` | 2 | Gap between nav items, segmented options |
| `SPACE_XS` | 3 | Segmented track padding |
| `SPACE_S1` | 4 | Gutter gap, small icon gaps |
| `SPACE_S2` | 6 | Icon gaps in buttons, vertical separator margin |
| `SPACE_S3` | 8 | Gaps in the top bar, chip padding |
| `SPACE_S4` | 10 | Gap in recent rows |
| `SPACE_M1` | 12 | Nav gap, nav padding, row padding |
| `SPACE_M2` | 14 | Inspector voice gap, model monogram gap |
| `SPACE_M3` | 16 | Top bar right padding, table column gap |
| `SPACE_M4` | 18 | Popover padding, inspector section gap |
| `SPACE_L1` | 20 | Inspector padding |
| `SPACE_L2` | 24 | Top bar left padding, sidebar group gap, table padding, player padding |
| `SPACE_L3` | 26 | Paragraph gap in the reading view |
| `SPACE_L4` | 32 | Reading view side padding, settings content padding, gallery column gap |
| `SPACE_XL1` | 40 | Gallery padding, models section gap |
| `SPACE_XL2` | 44 | Settings section gap |
| `SPACE_XL3` | 48 | Reading view top padding, gallery row gap |
| `SPACE_XL4` | 56 | Settings column gap |
| `SPACE_XL5` | 64 | Settings bottom padding |

## 4. Shape

### 4.1 Radii

| Rust token | Value (px) | Use |
|---|---|---|
| `RADIUS_FULL` | 999 | All pills, which are buttons, selects, search fields, segmented controls, chips, badges, toggles and timestamps |
| `RADIUS_CIRCLE` | 50% | Round icon buttons, play buttons, dots, avatars |
| `RADIUS_TRACK` | 2 | Progress bars and slider tracks of 4 px |
| `RADIUS_HINT` | 6 | Shortcut hint in the search field, current sentence highlight |
| `RADIUS_KEY` | 7 | Keyboard keys in the settings |
| `RADIUS_TOOL` | 8 | Editor toolbar buttons, breadcrumb title button |
| `RADIUS_ROW` | 10 | Recent rows, settings sub-nav items |
| `RADIUS_ITEM` | 12 | Nav items, model monograms |
| `RADIUS_POPOVER` | 18 | Popovers |
| `RADIUS_PANEL` | 28 | Brand panels on the Flow page only |

### 4.2 Borders and rings

| Rust token | Value (px) | Use |
|---|---|---|
| `BORDER_WIDTH` | 1 | All borders and dividers |
| `RING_WIDTH` | 1 | Rings, which are inset box shadows |
| `RING_WIDTH_SELECTED` | 1.5 | The ring of a selected avatar button |
| `REVIEW_UNDERLINE_WIDTH` | 1.5 | Thickness of the review underline (derived) |

### 4.3 Shadows

A shadow layer is written as `x y blur spread color`. A color is a color token, or a color token with an alpha, for example `INK/0.06`. No layer has a hex value, so the dark theme of stage 15 can derive each layer from its token.

| Rust token | Value | Use |
|---|---|---|
| `SHADOW_RAISED` | `0 1 2 0 INK/0.06`, `0 0 0 1 INK/0.05` | Selected nav item, open notification button |
| `SHADOW_SEGMENT` | `0 1 2 0 INK/0.10` | Selected option of a segmented control |
| `SHADOW_PLAY` | `0 6 16 -8 INK/0.55` | Primary play button in the player |
| `SHADOW_PLAY_LIVE` | `0 6 14 -8 SIGNAL` | Round play button of an item that plays |
| `SHADOW_POPOVER` | `0 0 0 1 INK/0.06`, `0 28 56 -20 INK/0.30` | Popovers and the toast |
| `SHADOW_THUMB` | `0 1 2 0 INK/0.20` | Toggle thumb |
| `SHADOW_SLIDER_THUMB` | `0 0 0 1 INK/0.14`, `0 1 4 0 INK/0.18` | Slider thumb (`estudio.html`, interpretation sliders) |
| `SHADOW_KEY` | inset `0 0 0 1 OUTLINE`, `0 1 0 0 OUTLINE` | Keyboard keys |
| `SHADOW_BADGE_RING` | `0 0 0 2 SURFACE` | Unread dot on the collapsed sidebar |

## 5. Motion

| Rust token | Value | Use |
|---|---|---|
| `EASE_FLOW` | `cubic-bezier(.34, 1.56, .64, 1)` | Each change of state or layout. The curve overshoots and settles |
| `DURATION_STATE` | 300 ms | Fill, shadow and color of controls, segmented selection, avatar ring |
| `DURATION_LAYOUT` | 350 ms | Sidebar width, toggle thumb position |
| `DURATION_REVEAL` | 500 ms | Color of reading text when its segment becomes generated. The color goes from `READING_PENDING` to `READING_DONE` with a linear curve |
| `DURATION_INSTANT` | 120 ms | Color of small text marks |
| `CARET_PERIOD` | 1000 ms | Blink of the synthesis front dot. The dot is visible in the first 500 ms of each period and hidden in the second 500 ms |
| `AVATAR_PHASE_RATE` | 1.0 phase unit for each second | Animation of an avatar while its voice plays |
| `TOAST_DURATION` | 6 s | Time until the toast hides (derived) |

Color changes of reading text use a linear curve. All other changes use `EASE_FLOW`. When the system setting "Reduce motion" is on, each change occurs immediately, the caret does not blink and avatars do not animate.

## 6. Icons

The icons are drawn for this design. They are not from an icon library, so they have the license of the project. Each icon is a 24 × 24 SVG with round caps and round joins. The default stroke width is 1.9. Stage 15 writes each icon to `crates/ui/assets/icons/<name>.svg` with the exact path data of the design page.

| Name | Design use | Stroke | Path data source |
|---|---|---|---|
| `studio` | Nav, Studio | 1.9 | `estudio.html`, nav item "Estudio" |
| `library` | Nav, Library | 1.9 | nav item "Biblioteca" |
| `voices` | Nav, Voices | 1.9 | nav item "Voces" |
| `models` | Nav, Models, engine chip | 1.9, 2.2 in chips | nav item "Modelos" |
| `settings` | Nav, Settings | 1.9 | nav item "Ajustes" |
| `bell` | Nav, Novedades | 1.9 | nav item "Novedades" |
| `sidebar` | Collapse and expand the sidebar | 1.8 | sidebar toggle |
| `inspector` | Show and hide the inspector | 1.8 | top bar |
| `export` | Export, exported status, download | 2.0, 2.2 in status | top bar "Exportar" |
| `plus` | New document | 2.2 | library "Nuevo" |
| `search` | Search fields | 1.9 | library search |
| `chevron-down` | Selects, breadcrumb | 2.8, 2.6 in breadcrumb | inspector selects |
| `play` | Play (filled) | fill | player |
| `pause` | Pause (filled) | fill | player |
| `back-10` | Back 10 seconds | 1.9 | player |
| `forward-10` | Forward 10 seconds | 1.9 | player |
| `volume` | Volume | 1.9 | player |
| `check` | In use, default | 2.6 | gallery, models |
| `close` | Cancel download | 2.6 | models |
| `more` | More options (filled dots) | fill | models |
| `folder` | Open folder | 1.9 | models top bar |
| `undo` | Undo | 1.9 | editor toolbar |
| `redo` | Redo | 1.9 | editor toolbar |
| `history` | Version history (out of scope) | 1.8 | top bar |

The logo is a separate 32 × 32 SVG. It has two dots (r 1.8 at x 5.2 and 26.8, y 16) and one stroke path of width 2.8. The dots are `SIGNAL` and the path is `INK` on light fills. On an `INK` or `SIGNAL` tile, the path is white. The app icon is the `INK` tile with radius 9 in the 32 × 32 box.

The word mark is "antenna" in lowercase with `BRAND_SIDEBAR` or `BRAND_ABOUT`. The design pages show "antena" and an About text with the MIT license. The product name is Antenna and the license is GPL-3.0-or-later, so the app does not use these two texts of the design.

## 7. Voice avatar

Each voice has a generated avatar. The avatar is a ring of radial lines around a dot, in a 100 × 100 box with the center at (50, 50).

### 7.1 Seed

The seed is a number from the voice id. Calculate the 64-bit FNV-1a hash of the UTF-8 bytes of the voice id. The seed is `(hash mod 4096) / 64`, as `f32`. Thus the same voice always has the same avatar.

### 7.2 Shape

For each line `i` from 0 to `n - 1`, these values apply.

1. The angle is `a = 2π · i / n − π / 2`.
2. The shape value is `m = 0.5 + 0.22 · sin(3a + seed) + 0.18 · sin(7a + 1.7 · seed) + 0.1 · sin(13a + 0.3 · seed)`.
3. Clamp `m` to the range 0.08 to 1.
4. The line starts at radius `r0` and ends at radius `r0 + span · m`, at angle `a`.
5. In the gallery and inspector sizes, the line opacity is `0.3 + 0.7 · m`. In the other sizes, the line opacity is 1.

In the gallery and inspector sizes, a polygon connects the outer ends of all lines. Its fill is `INK_TINT` (or `SIGNAL_TINT_STRONG` while the voice plays). Its stroke is the ink color at opacity 0.35. A dot at the center has the core radius.

### 7.3 Live animation

While the voice plays, the phase `p` increases by `AVATAR_PHASE_RATE`. The envelope is `env = 0.5 + 0.5 · (0.62 · |sin(4.3p)| + 0.38 · |sin(10.1p + 1.3)|)`. These steps replace steps 2 and 3 of section 7.2.

1. Calculate `m` with step 2 of section 7.2.
2. The live value is `m · env + 0.16 · sin(5a − 6.5p + seed) · env`.
3. Each size except the table cell adds `0.08 · sin(11a + 9.3p)` to the live value.
4. Clamp the live value to the range 0.08 to 1.

The ink becomes `SIGNAL`, and the core radius is the live value of the table in section 7.4.

### 7.4 Sizes

| Rust token | Use | Size (px) | Lines `n` | `r0` | `span` | Stroke width (box units) | Polygon | Core radius idle | Core radius live |
|---|---|---|---|---|---|---|---|---|---|
| `AVATAR_GALLERY` | Voice gallery | 120 | 64 | 12 | 34 | 1.5 | yes, stroke 0.6 | 4.5 | 3 + 3.5 · env |
| `AVATAR_INSPECTOR` | Inspector voice card | 64 | 56 | 12 | 34 | 2.2 | yes, stroke 0.8 | 6 | 4 + 4 · env |
| `AVATAR_PLAYER` | Player bar | 40 | 32 | 12 | 34 | 4 | no | 7 | 5 + 5 · env |
| `AVATAR_ROW` | Other voices in the inspector, voice select, default voice in the settings | 24 | 24 | 14 | 32 | 6 | no | none | none |
| `AVATAR_TABLE` | Voice cell of the library table | 22 | 24 | 14 | 32 | 6 | no | none | none |

The ink of an `AVATAR_ROW` avatar is `INK` when selected and `INK_MUTED` when not selected. The ink of an `AVATAR_TABLE` avatar is `TEXT_STRONG`.

## 8. Waveform

The waveform in the player shows the audio of the document. The `Waveform` component of stage 15 takes the peaks of the generated audio in linear amplitude, at `PEAKS_PER_SECOND` of stage 03. It does each step of this section.

| Rust token | Value | Use |
|---|---|---|
| `WAVE_HEIGHT` | 48 px | Height of the waveform |
| `WAVE_BAR_PITCH` | 7.27 px | Distance between the starts of two bars (800 / 110 in the design) |
| `WAVE_BAR_WIDTH` | 4.6 px | Bar width. The radius is half the width |
| `WAVE_BAR_MIN` | 6 px | Height of the lowest bar |
| `WAVE_BAR_MAX` | 40 px | Height of a bar at 0 dBFS |
| `WAVE_FLOOR_DB` | -48 dBFS | Level that gives a bar of `WAVE_BAR_MIN` |
| `WAVE_DOT_RADIUS` | 1.8 px | Radius of a pending dot |
| `PLAYHEAD_RADIUS` | 6.5 px | Radius of the playhead |
| `PLAYHEAD_STROKE` | 2.6 px | Width of the `CANVAS` stroke of the playhead |
| `WAVE_BLUR_SIGMA` | 2 px | Sigma of the gaussian blur, in logical pixels |
| `WAVE_ALPHA_GAIN` | 20 | Gain of the alpha threshold |
| `WAVE_ALPHA_OFFSET` | 8 | Offset of the alpha threshold |

1. Divide the width into bars with the pitch `WAVE_BAR_PITCH`.
2. For each bar, take the largest peak in the matching time range of the document audio. Convert it to dBFS with `20 · log10(peak)`. Map `WAVE_FLOOR_DB` to 0 and 0 dBFS to 1, and clamp. The bar height is `max(WAVE_BAR_MIN, round(WAVE_BAR_MAX · level))`, centered on the middle line.
3. A bar before the play position has the color `SIGNAL`. A bar after the play position that is generated has the color `WAVE_IDLE`. The player page uses `#CFCFCA` for these bars. The app uses the palette value of `WAVE_IDLE`.
4. After the synthesis front, the waveform has no bars. It has one dot (radius `WAVE_DOT_RADIUS`, color `WAVE_DOT`) for each two bar positions.
5. The playhead is a circle of radius `PLAYHEAD_RADIUS` with fill `SIGNAL` and a `CANVAS` stroke of `PLAYHEAD_STROKE`.
6. The bars merge into one shape. The design does this with a gaussian blur (σ 2 in the player, σ 2.2 on the Flow page) and an alpha threshold (`alpha · 20 − 8`). The app uses the player values. Stage 15 gives the method for GPUI.

## 9. Layout

The values are in logical pixels. Each padding and gap of this section is a token of section 3. Stage 15 puts the size tokens in `crates/ui/src/theme/size.rs`.

### 9.1 Control sizes

All controls use one height scale.

| Rust token | Value | Use |
|---|---|---|
| `SIZE_CONTROL_XS` | 30 | Small round icon buttons |
| `SIZE_CONTROL_S` | 32 | Editor toolbar buttons, sidebar toggle, row play buttons, recent rows, small buttons |
| `SIZE_CONTROL_M` | 34 | Top bar buttons, selects, search fields, the volume button, regular buttons |
| `SIZE_CONTROL_L` | 36 | Skip buttons, the gallery play button, large buttons |
| `SIZE_CONTROL_XL` | 44 | The primary play button of the player |
| `SIZE_SEGMENT` | 28 | Option of a segmented control |
| `SIZE_PILL` | 24 | Timestamp pill |
| `SIZE_BADGE` | 20 | Height and minimum width of the unread badge |
| `SIZE_KEY` | 26 | Keyboard key |
| `SIZE_ICON_NAV` | 18 | Nav icons |
| `SIZE_LOGO` | 30 | Logo in the sidebar |
| `SIZE_LOGO_ABOUT` | 56 | App icon tile in the About section |
| `SIZE_MONOGRAM` | 40 | Model monogram tile |
| `SIZE_AVATAR_BUTTON` | 38 | Avatar button of the other voices in the inspector (`estudio.html`) |
| `SIZE_TOGGLE` | 36 × 22, thumb 16 | Toggle in the settings |
| `SIZE_TOGGLE_COMPACT` | 34 × 20, thumb 16 | Toggle in the inspector |
| `SIZE_SLIDER_THUMB` | 14 | Slider thumb |
| `SIZE_TRACK` | 4 | Height of progress bars and slider tracks |
| `SIZE_PROGRESS_RING` | 14 | Progress ring of a generating status and of the Export button |

### 9.2 Rows

| Rust token | Value | Use |
|---|---|---|
| `SIZE_NAV_ITEM` | 38 | Nav item |
| `SIZE_RECENT_LABEL` | 28 | "Recent" label in the sidebar |
| `SIZE_FIELD_ROW` | 38 | Minimum height of an inspector field row |
| `SIZE_TABLE_HEADER` | 40 | Header row of the library table |
| `SIZE_GROUP_LABEL` | 44 | Period group label of the library table |
| `SIZE_TABLE_ROW` | 56 | Row of the library table |
| `SIZE_MODEL_ROW` | 72 | Minimum height of a row of the Models screen |
| `SIZE_SETTINGS_ROW` | 60 | Minimum height of a settings row |

### 9.3 Panels and columns

| Rust token | Value | Use |
|---|---|---|
| `LAYOUT_WINDOW` | 1440 × 900 | Size of a new app window |
| `LAYOUT_WINDOW_MIN` | 960 × 600 | Minimum size of the app window |
| `LAYOUT_GALLERY_WINDOW` | 1200 × 900 | Size of the gallery window |
| `LAYOUT_SIDEBAR_OPEN` | 248 | Width of the open sidebar |
| `LAYOUT_SIDEBAR_CLOSED` | 72 | Width of the closed sidebar |
| `LAYOUT_SIDEBAR_HEAD` | 30 | Height of the sidebar head |
| `LAYOUT_TOP_BAR` | 56 | Height of the top bar |
| `LAYOUT_TOOLBAR` | 48 | Minimum height of the editor toolbar |
| `LAYOUT_READING_MAX` | 720 | Maximum width of the reading column |
| `LAYOUT_GUTTER` | 64 | Width of the gutter of the reading view |
| `LAYOUT_INSPECTOR` | 300 | Minimum width of the inspector |
| `LAYOUT_FIELD_LABEL` | 84 | Label column of an inspector field row |
| `LAYOUT_PLAYER` | 84 | Minimum height of the player bar |
| `LAYOUT_PLAYER_SIDE` | 220 | Width of each side block of the player bar |
| `LAYOUT_TIME_LABEL` | 40 | Width of a time label in the player bar |
| `LAYOUT_NEWS_POPOVER` | 360 | Width of the "What's new" popover |
| `LAYOUT_VOLUME_POPOVER` | 200 | Width of the volume popover (derived) |
| `LAYOUT_TOAST` | 360 | Maximum width of the toast (derived) |
| `LAYOUT_TOOLTIP` | 320 | Maximum width of a tooltip (derived) |
| `LAYOUT_SEARCH_LIBRARY` | 260 | Width of the library search field |
| `LAYOUT_SEARCH_VOICES` | 240 | Width of the voices search field |
| `LAYOUT_LIBRARY_COLUMNS` | 40, flexible, 150, 70, 80, 150, 100 | Columns of the library table |
| `LAYOUT_VOICE_COLUMN` | 220 | Minimum width of a column of the voice gallery |
| `LAYOUT_MODELS_COLUMNS` | 44, flexible, 80, 120, 110, 220 | Columns of the Models screen |
| `LAYOUT_SETTINGS_NAV` | 180 to 220 | Width of the settings sub-nav |
| `LAYOUT_SETTINGS_CONTENT` | 720 | Maximum width of the settings content |

### 9.4 Paddings and gaps

| Element | Padding | Gap |
|---|---|---|
| Sidebar | `SPACE_M2` vertical, `SPACE_M1` horizontal (`SPACE_S4` closed) | `SPACE_L2` between groups |
| Nav item and recent row | 0 `SPACE_M1` | `SPACE_M1` |
| Unread badge | 0 `SPACE_S2` | |
| "What's new" popover | `SPACE_M4` | `SPACE_S3` from the sidebar edge, `SPACE_M3` from the window bottom |
| Top bar | 0 `SPACE_M3` 0 `SPACE_L2` | `SPACE_S3` |
| Editor toolbar | `SPACE_S2` `SPACE_M3` | |
| Reading column | `SPACE_XL3` top, `SPACE_L4` sides, `SPACE_XL1` bottom | `SPACE_L3` between paragraphs, `SPACE_S1` after the gutter |
| Timestamp pill | 0 `SPACE_S3` | `SPACE_S3` from the top of the paragraph |
| Inspector | `SPACE_L1` | `SPACE_M4` between sections |
| Player bar | `SPACE_M1` `SPACE_L2` | `SPACE_L2` |
| Library table | 0 `SPACE_L2` | `SPACE_M3` between columns |
| Segmented control | `SPACE_XS` track, 0 `SPACE_M1` option | `SPACE_2XS` |
| Voice gallery | `SPACE_XL1` | `SPACE_XL3` between rows, `SPACE_L4` between columns |
| Models screen | `SPACE_L4` `SPACE_L2` `SPACE_XL1` | `SPACE_XL1` between sections |
| Settings screen | `SPACE_L4` `SPACE_L2` `SPACE_XL5` | `SPACE_XL2` between sections, `SPACE_XL4` between columns |
| Volume popover and toast (derived) | `SPACE_M4` | `SPACE_M1`. The toast is `SPACE_M3` above the player bar, centered on the workspace |
| Tooltip (derived) | `SPACE_S3` `SPACE_M1` | `SPACE_S1` from the hovered text |
| Settings sub-nav item | 0 `SPACE_M1` | `SPACE_2XS` between items |
