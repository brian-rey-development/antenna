# Stage 02. Text

## Goal

`antenna-text` converts a document into speakable segments with exact source ranges and identifies its language, in the budgets of this stage.

## Context

Each engine receives segments from this crate. A wrong segment boundary causes a wrong pause or a sentence that an engine cannot read. A wrong source range causes a wrong highlight in the editor. Stage 06 calls `prepare` on the worker thread. The segment store of stage 05 keys each segment by its text, so `prepare` must give the same segments for the same input. Stages 07 and 16 call `identify_language` when the user opens or changes a document.

## Read first

1. `CLAUDE.md`
2. `docs/architecture.md` sections 5.1, 5.2, 8.1 and 13
3. `docs/standards.md` sections 3, 5, 6, 14, 18 and 19
4. `docs/writing.md` section 4 (glossary)

## Scope

The stage can create or change these files only.

- `crates/text/`
- `Cargo.toml` (root), to add the member and the workspace dependencies `pulldown-cmark`, `srx`, `whatlang`, `proptest`, `insta` and `divan`

## Deliverables

### Crate layout

```
crates/text/
├── Cargo.toml
├── rules/
│   ├── segment.srx         # vendored LanguageTool rules
│   └── README.md           # source URL, commit SHA, license LGPL-2.1-or-later, attribution
├── src/
│   ├── lib.rs
│   ├── error.rs            # TextError
│   ├── prose.rs            # Markdown to prose, with block boundaries
│   ├── source_map.rs       # prose byte offset to document byte offset
│   ├── sentences.rs        # SRX rules for each language
│   ├── split.rs            # long-sentence split and first-segment split
│   ├── language.rs         # identify_language
│   └── prepare.rs          # prepare: the public entry point
├── benches/
│   └── prepare.rs          # divan benchmark
└── tests/
    ├── fixtures/           # en.md, es.md, pt.md, fr.md, it.md, de.md, markdown.md, abbreviations/<lang>.txt
    ├── snapshots/          # insta snapshots
    ├── prepare.rs          # snapshot and behavior tests
    └── invariants.rs       # proptest invariants
```

### Public API

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SegmentLimits { max_chars: NonZeroUsize, first_segment_chars: NonZeroUsize }

impl SegmentLimits {
    pub fn new(max_chars: NonZeroUsize) -> Self;
    pub fn max_chars(self) -> NonZeroUsize;
    pub fn first_segment_chars(self) -> NonZeroUsize;
}

pub fn prepare(
    document: &Document,
    language: Language,
    limits: SegmentLimits,
) -> Result<Vec<Segment>, TextError>;

pub fn identify_language(document: &Document) -> Option<Language>;

pub enum TextError {
    Rules(#[source] Arc<srx::Error>),
    NoSpeech,
}
```

### Constants

```rust
pub const FIRST_SEGMENT_CHARS: usize = 60;
const MIN_FIRST_SEGMENT_CHARS: usize = 20;
const FIRST_SPLIT_FALLBACK_CHARS: usize = 120;
const MIN_CLAUSE_FRACTION_DENOMINATOR: usize = 2;
const IDENTIFY_SAMPLE_CHARS: usize = 2_000;
const IDENTIFY_MIN_LETTERS: usize = 20;
```

`SegmentLimits::new` sets `first_segment_chars` to `FIRST_SEGMENT_CHARS`, or to `max_chars` if `max_chars` is smaller. Thus `first_segment_chars` is never larger than `max_chars`. The fields are private, so no caller can break this rule. A caller gives `descriptor.max_segment_chars` of the engine.

`TextError::Rules` keeps the parse error of `srx` as its source. The error is in an `Arc`, because one `LazyLock` keeps one error for all calls. If the `srx` error type has a different name, use the real name.

### Markdown conversion rules

`prose.rs` uses the rules of `document.format()`. `TextFormat` comes from `antenna-core`.

1. For `TextFormat::Plain`, each run of lines without a blank line is one block. Each block is one piece with a source range equal to its prose range. Plain text keeps every character. `#`, `*` and the indentation also stay.
2. For `TextFormat::Markdown`, `prose.rs` parses the document with `pulldown-cmark` and its `into_offset_iter()`. It enables only the `ENABLE_TABLES` and `ENABLE_STRIKETHROUGH` options. Each Markdown document is valid input. The table in this section gives the conversion.

| Markdown element | Prose output |
|---|---|
| Paragraph, heading, list item, block quote paragraph, table cell | The text of the element. The end of the element is a block boundary |
| Emphasis, strong, strikethrough | The inner text |
| Link | The link text. Antenna drops the URL |
| Autolink | Nothing |
| Image | Nothing |
| Inline code | The code text |
| Code block (fenced or indented) | Nothing |
| HTML block, inline HTML | Nothing |
| Soft break, hard break | One space |
| Thematic break | A block boundary |

A segment never crosses a block boundary. Thus a heading is always its own segment.

### Source map

`source_map.rs` stores a sorted list of pieces. Each piece maps a prose byte range to the document byte range of one `pulldown-cmark` text event. To get the source range of a segment, take the document start of its first piece and the document end of its last piece. A segment that starts or ends inside a piece maps to the exact byte offset when the piece text and the source text are equal. Otherwise it maps to the edge of the piece. Entities and backslash escapes are the cases where the texts differ.

### Segmentation rules

`prepare` does these steps in sequence.

1. Convert the document to prose blocks with the rules of its format.
2. Split each block into sentences with the SRX rules of the language.
3. Collapse each run of whitespace in a sentence to one space. Trim the sentence.
4. Drop each sentence that contains no letter and no digit. Example, "..." or "* * *".
5. If a sentence has more than `max_chars` characters, split it with the long-sentence rule.
6. If the first segment of the document has more than `first_segment_chars` characters, split it with the first-segment rule.
7. Give the segments the indexes `SegmentIndex::new(0)`, `SegmentIndex::new(1)` and so on, in sequence.
8. If no segment remains, return `TextError::NoSpeech`.

A character count is a count of `char` values.

**Clause boundary.** A clause boundary is the position after one of the characters `,`, `;`, `:`, `)`, U+2014 or U+2013, when whitespace follows it.

Write U+2014 and U+2013 as the escapes `'\u{2014}'` and `'\u{2013}'` in Rust code. The fixture files contain neither character. Tests that need them build the text with these escapes. Thus the Dashes check of `docs/standards.md` section 13 finds no violation in this crate.

**Long-sentence rule.** Find the last clause boundary at or before `max_chars` characters and at or after `max_chars / 2` characters. If none exists, find the last whitespace at or before `max_chars` characters. If none exists, split at the last `char` boundary at or before `max_chars` characters. Apply the rule again to the rest of the sentence.

**First-segment rule.** Find the first clause boundary at or after `MIN_FIRST_SEGMENT_CHARS` characters and at or before `first_segment_chars` characters. If none exists and the sentence has more than `FIRST_SPLIT_FALLBACK_CHARS` characters, split at the first whitespace at or after `first_segment_chars` characters. Otherwise, do not split.

### SRX rules

1. Vendor `segment.srx` from the LanguageTool repository at a pinned commit. The path in that repository is `languagetool-core/src/main/resources/org/languagetool/resource/segment.srx`.
2. Write `rules/README.md` with the source URL, the commit SHA, the license (LGPL-2.1-or-later) and the attribution "Segmentation rules from LanguageTool".
3. Load the file with `include_str!` and parse it one time in a `LazyLock`. The `srx` crate needs its `from_xml` feature for this.
4. Build the `Rules` of each `Language` one time from its ISO 639-1 code.
5. The `srx` crate ignores rules that use look-ahead or look-behind, because the `regex` crate does not support them. `srx::SRX::errors()` lists these rules. A test records the number of ignored rules for each of the six languages, so a change of the rules file is visible.

The `LazyLock` holds a `Result`. If the rules do not parse, `prepare` returns `TextError::Rules`. The test `srx_rules_parse_for_all_languages` makes sure that this does not occur with the vendored file.

### Language identification

1. Convert the document to prose with `prose.rs`. Take the first `IDENTIFY_SAMPLE_CHARS` characters of the prose.
2. If the sample has fewer than `IDENTIFY_MIN_LETTERS` letters, return `None`.
3. Use a `whatlang::Detector` with an allowlist of the six languages of `Language::ALL`. Build the detector one time in a `LazyLock`.
4. If `Info::is_reliable()` is `false`, return `None`.
5. Map the `whatlang::Lang` to `Language`.

The caller decides the fallback language. `identify_language` does not use the locale of the system.

## Tasks

1. Add the crate to the workspace members and add the workspace dependencies.
2. Vendor `segment.srx` and write `rules/README.md`.
3. Write `error.rs` and `source_map.rs`. Write the unit tests of the source map first.
4. Write `prose.rs`. Write one unit test for each row of the Markdown table and one unit test for the plain text blocks.
5. Write `sentences.rs` with the `LazyLock` rules.
6. Write `split.rs`. Write unit tests for each branch of the two split rules.
7. Write `prepare.rs` and connect the steps.
8. Write `language.rs`.
9. Write the fixtures. Each language fixture has 30 or more sentences of narrative prose. It contains abbreviations, decimal numbers, quotes, a list and a heading. It contains no HTML entity, no U+2013 and no U+2014.
10. Write the abbreviation fixtures. Each file has 10 or more lines. Each line is one input with the expected number of segments.
11. Write the snapshot tests with `insta`. Each snapshot shows each segment with its index, its text and the document slice of its source range. Write the slice tests of AC-02-14.
12. Write the `proptest` invariants.
13. Write the `divan` benchmark. Measure `prepare` on a 1 MB document that repeats `tests/fixtures/en.md`.
14. Run `cargo xtask check`. Fix each failure.
15. Do the quality gate in `docs/standards.md` section 20.

## Acceptance criteria

| ID | Criterion | Check |
|---|---|---|
| AC-02-01 | All checks pass | `cargo xtask check` exits with code 0 |
| AC-02-02 | Each Markdown element converts as the table in this stage defines | One test for each row, named `prose_<result>_when_<element>`, for example `prose_drops_url_when_link` |
| AC-02-03 | A heading is always a separate segment | Test `prepare_isolates_heading_when_paragraph_follows` |
| AC-02-04 | Abbreviations do not end a sentence | Test `segmenter_keeps_abbreviation_when_name_follows` passes for each file in `tests/fixtures/abbreviations/` |
| AC-02-05 | Decimal numbers do not end a sentence | Test `segmenter_keeps_decimal_when_digits_follow` for "3.14" and "3,14" |
| AC-02-06 | No segment is longer than `max_chars` characters | Proptest `segments_respect_max_chars` |
| AC-02-07 | Source ranges are in sequence, do not overlap, are in the document and are on `char` boundaries | Proptest `source_ranges_are_ordered_and_valid` |
| AC-02-08 | Plain text input loses no letter or digit | Proptest `segments_keep_all_alphanumeric_chars_when_plain_text` |
| AC-02-09 | `prepare` is deterministic | Proptest `prepare_is_deterministic` |
| AC-02-10 | Segment indexes start at 0 and increase by 1 | Proptest `segment_indexes_are_sequential` |
| AC-02-11 | The long-sentence rule prefers a clause boundary, then whitespace, then a `char` boundary | Tests `split_long_uses_clause_when_available`, `split_long_uses_whitespace_when_no_clause` and `split_long_uses_char_boundary_when_no_whitespace` |
| AC-02-12 | The first segment has `first_segment_chars` characters or fewer when it has a clause boundary in range | Test `prepare_splits_first_sentence_when_clause_in_range` |
| AC-02-13 | A document with no speakable text returns `TextError::NoSpeech` | Test `prepare_fails_when_only_code_block` |
| AC-02-14 | The source slice of each segment matches the segment text. For plain text, the slice with each whitespace run collapsed to one space equals the segment text. For Markdown, the letters and digits of the segment text occur in the slice in the same sequence | Tests `slice_equals_segment_when_plain_text` and `slice_contains_segment_letters_when_markdown`, over the six language fixtures and `markdown.md` |
| AC-02-15 | The SRX rules parse for each language, and the number of ignored rules is recorded | Tests `srx_rules_parse_for_all_languages` and `srx_ignored_rule_count_is_known` |
| AC-02-16 | Language identification is correct for each language fixture | Test `identify_language_finds_language_when_fixture` for the six fixtures |
| AC-02-17 | Language identification returns `None` for a short or mixed text | Tests `identify_language_returns_none_when_text_is_short` and `identify_language_returns_none_when_unreliable` |
| AC-02-18 | `prepare` on a 1 MB document takes less than 50 ms in a release build on the reference machine (Apple M5 Pro, 24 GB) **(manual)** | `cargo bench -p antenna-text` median below 50 ms. Add the result to the PR |
| AC-02-19 | `antenna-text` depends only on `antenna-core` in the workspace | `cargo xtask lint-repo` passes |
| AC-02-20 | Plain text keeps Markdown characters and indented lines | Test `prose_keeps_markup_characters_when_plain_text` |
| AC-02-21 | The first-segment rule splits at the first whitespace after `first_segment_chars` when no clause boundary is in range and the sentence has more than `FIRST_SPLIT_FALLBACK_CHARS` characters | Test `split_first_uses_whitespace_when_no_clause_and_sentence_long` |
| AC-02-22 | The first-segment rule keeps a sentence of `FIRST_SPLIT_FALLBACK_CHARS` characters or fewer when no clause boundary is in range | Test `split_first_keeps_sentence_when_no_clause_and_sentence_short` |
| AC-02-23 | `SegmentLimits::new` never gives a first segment limit larger than `max_chars` | Test `segment_limits_clamp_first_segment_when_max_chars_small` |
| AC-02-24 | Step 4 drops each sentence with no letter and no digit, and keeps the indexes in sequence | Test `prepare_drops_sentence_when_no_letter_or_digit` with the input "First. ... * * * Last." gives two segments with indexes 0 and 1 |

## Decision rules

1. If the vendored `segment.srx` from the latest LanguageTool commit does not parse with `srx`, use the newest commit whose file parses. Write the commit SHA and the reason in `rules/README.md`.
2. If an abbreviation fixture fails because `srx` ignored a look-around rule, write an equivalent rule without look-around. Put it in a second file `rules/antenna.srx`, and cascade it before `segment.srx`. Each added rule has a comment in the XML with the ignored rule it replaces.
3. If the benchmark fails AC-02-18, profile with `cargo flamegraph` or `samply`. Make sure that `prepare` calls the SRX rules once for each block and not once for the full document. Do not replace `srx`. If the budget still fails, stop and write a "Blocked" section with the profile.
4. If `whatlang` returns a reliable but wrong language for a fixture, examine the sentence count. If the fixture has fewer than 30 sentences, add more prose. Otherwise, stop and write a "Blocked" section.
5. A `pulldown-cmark` text event range can contain text that is different from the event text, for example an entity. In that case, map the full piece as defined in "Source map". Do not try to map inside the piece.

## Out of scope

1. Number, date and abbreviation expansion. Engines read the text as it is.
2. Per-paragraph language identification.
3. Import of PDF, EPUB or HTML.
4. Any change to `antenna-core`. Stage 01 defines `Document` and `TextFormat`.
