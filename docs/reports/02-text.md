# Stage 02 Report. Text

| | |
|---|---|
| Stage file | `docs/plans/02-text.md` |
| Date | 2026-10-06 |
| Result | Complete, with one criterion pending (AC-02-18). No Blocked section |
| CI | Run 37552665239 on commit `0b17331`, green on `macos-latest`, `ubuntu-latest` and `windows-latest`. The run before it failed on Windows, because a checkout changed the line ends of the fixtures. Commit `0b17331` fixes the tests |

## 1. Acceptance criteria

The last local run of `cargo nextest run -p antenna-text` reported 131 tests run and 131 passed.

| ID | Result | Evidence |
|---|---|---|
| AC-02-01 | Pass | `cargo xtask check` exits with code 0 |
| AC-02-02 | Pass | One test for each row in `src/markdown.rs`. `prose_ends_block_when_paragraph_ends`, `_heading_ends`, `_list_item_ends`, `_block_quote_paragraph_ends`, `_table_cell_ends`, `prose_keeps_inner_text_when_emphasis_strong_or_strikethrough`, `prose_drops_url_when_link`, `prose_drops_text_when_autolink`, `prose_drops_alt_text_when_image`, `prose_keeps_code_text_when_inline_code`, `prose_drops_code_when_fenced_code_block`, `prose_drops_code_when_indented_code_block`, `prose_drops_html_when_html_block`, `prose_drops_tag_when_inline_html`, `prose_gives_space_when_soft_or_hard_break`, `prose_ends_block_when_thematic_break` |
| AC-02-03 | Pass | `prepare_isolates_heading_when_paragraph_follows` |
| AC-02-04 | Pass | `segmenter_keeps_abbreviation_when_name_follows` reads the six files in `tests/fixtures/abbreviations/`. The files have 12, 12, 12, 11, 10 and 12 lines |
| AC-02-05 | Pass | `segmenter_keeps_decimal_when_digits_follow` for "3.14" and "3,14" in English, Spanish and German |
| AC-02-06 | Pass | Proptest `segments_respect_max_chars` |
| AC-02-07 | Pass | Proptest `source_ranges_are_ordered_and_valid` |
| AC-02-08 | Pass | Proptest `segments_keep_all_alphanumeric_chars_when_plain_text` |
| AC-02-09 | Pass | Proptest `prepare_is_deterministic` |
| AC-02-10 | Pass | Proptest `segment_indexes_are_sequential` |
| AC-02-11 | Pass | `split_long_uses_clause_when_available`, `split_long_uses_whitespace_when_no_clause` and `split_long_uses_char_boundary_when_no_whitespace`. Edge tests check a clause at half of the limit and before it |
| AC-02-12 | Pass | `prepare_splits_first_sentence_when_clause_in_range`, and `split_first_uses_clause_when_clause_is_at_limit` and `_at_minimum` for the edges |
| AC-02-13 | Pass | `prepare_fails_when_only_code_block` |
| AC-02-14 | Pass | `slice_equals_segment_when_plain_text` and `slice_contains_segment_letters_when_markdown` over the six fixtures and `markdown.md`. Proptest `slice_equals_segment_when_plain_text` checks the plain text rule for generated documents. See difference 3 for Markdown |
| AC-02-15 | Pass | `srx_rules_parse_for_all_languages` and `srx_ignored_rule_count_is_known`. English has 2 ignored rules. Spanish, Portuguese, French, Italian and German have 0. `GeneralImportant` has 0 |
| AC-02-16 | Pass | `identify_language_finds_language_when_fixture` |
| AC-02-17 | Pass | `identify_language_returns_none_when_text_is_short` and `identify_language_returns_none_when_unreliable` |
| AC-02-18 | Pending: measured alone after stages 02, 03, 04, 05 and 15 are on main | Command `cargo bench -p antenna-text`. The median of `prepare_one_megabyte_of_paragraphs` must be below 50 ms. `cargo bench -p antenna-text -- --test` runs both benchmarks one time. The benchmark `prepare_one_megabyte_of_short_blocks` is for information. The second review measured about 250 ms for it, because `srx` runs each rule as a separate search over each block. Finding 1 of section 2.4 fixes this. Sign-off. Project owner, approved on ____ |
| AC-02-19 | Pass | `cargo xtask lint-repo` passes. `cargo tree -p antenna-text -e normal --depth 1` shows `antenna-core`, `pulldown-cmark`, `srx`, `thiserror` and `whatlang` |
| AC-02-20 | Pass | `prose_keeps_markup_characters_when_plain_text` |
| AC-02-21 | Pass | `split_first_uses_whitespace_when_no_clause_and_sentence_long` |
| AC-02-22 | Pass | `split_first_keeps_sentence_when_no_clause_and_sentence_short` |
| AC-02-23 | Pass | `segment_limits_clamp_first_segment_when_max_chars_small` |
| AC-02-24 | Pass | `prepare_drops_sentence_when_no_letter_or_digit` and `prepare_drops_block_when_no_letter_or_digit` |

## 2. Quality gate

### 2.1 Automatic checks

`cargo xtask check` passes locally.

### 2.2 Independent review

A new agent session reviewed the diff against the stage file, `docs/architecture.md`, `docs/standards.md` and `docs/writing.md`. It read every changed file and reported 21 findings, 2 of them major. The author fixed each finding, except the parts in the last two rows.

| Finding | Fix |
|---|---|
| 1. Major. `segment.srx` had 12 literal U+2013 and U+2014 characters, so `lint-repo` failed | The copy writes them as `&#x2013;` and `&#x2014;`. `rules/README.md` tells this |
| 2. Major. Sentences did not split in plain text with Windows line ends or with a space before the newline | `sentences::in_block` gives the rules a copy with the newline first in each whitespace run. The copy has the same byte offsets. Tests for both cases, and `\r` and `\t` in the proptest alphabet |
| 3. The empty source range was not documented and not tested | The doc of `prepare` tells the case. Tests pin the ranges of a code span with two and with three sentences. See finding 3 of section 2.4 |
| 4. `exceeds` was quadratic for one long word | `Fragment::collapsed_indices` reads one character at a time. A test cuts a word of 100000 characters |
| 5. The benchmark had only long paragraphs | Added `prepare_one_megabyte_of_short_blocks` and the note in AC-02-18 |
| 6. Module cycle between `prose` and `markdown` | `markdown::convert` returns `Converted`, and `prose` builds `Prose` from it. Finding 8 of section 2.4 replaces `Converted` |
| 7. `SourceMap::source` returned an `Option` and `prepare` used `expect` | `source` returns a `Range` and an empty range if no piece fits. The `expect` is gone |
| 8. `Prose::of` is a constructor | Renamed to `Prose::for_document` |
| 9. The whitespace collapse rule occurred two times | `boundaries` uses `Fragment::collapsed_indices`, so one function knows the rule. Rejected, the language tables. `whatlang_of` is the only table and the reverse map derives from it. The six-field rules struct gives a compile error for a new language |
| 10. STE-80 and missing source in doc comments | Rewritten. The Markdown table is in the doc of `markdown::convert`. `rules/README.md` tells both changes |
| 11. The proptest alphabet had no `\r`, `\t`, U+00A0 and wide characters | Added, with plain slice and block properties. The Markdown slice property is not a proptest, see difference 3 |
| 12. Missing edge tests | Added for the clause at 19 and 20 characters, at half of the limit, at the first segment limit and for the fallback at 120 and 121 characters. Rejected, the identify edge at 19 and 20 letters. `whatlang` is not reliable for texts that short, so no text gives `Some` there. Added a test for the 2000 character sample |
| 13. Tests with two behaviors | Split |
| 14. Nit. Closure parameter `c` | Renamed |
| 15. Nit. Magic fixture indexes and unnamed numbers | `fixture(language)`, named constants and computed expectations |
| 16. Nit. Names in `split.rs` | `long_sentence`, `first_segment`, `in_block`, `chars_before`. `Boundary` and `boundaries` moved to `boundary.rs` with tests |
| 17. Nit. `(0..=u32::MAX)` stops at 4 billion segments | Kept. A document with that many segments needs more than 100 GB of memory |
| 18. Nit. Names and unused derives | `FIRST_SEGMENT_LIMIT`. Removed `PartialEq`, `Eq` and `Clone` where nothing used them |
| 19. Nit. Nested character classes | Documented in the doc of `without_capture_groups` |
| 20. Nit. "1. One." gives the segment "2." alone | Recorded as a known limit in section 4 |
| 21. The report must record the differences | Section 4 |

The `lint-repo` limit of 300 lines made two more splits. `tests/prepare.rs` is now `tests/prepare/main.rs` with four test files and `support.rs`, and the snapshots are in `tests/prepare/snapshots/`.

### 2.3 Simplification pass

The author read the complete diff again. The pass removed the `expect` and its exception, and the `Option` of the source map. It also removed three unused derives, a duplicate collapse rule and a duplicate `limits` helper in `split.rs`.

### 2.4 Second independent review

A second review of commit `c44e425` reported 9 findings, 1 of them major. The author fixed all of them.

| Finding | Fix |
|---|---|
| 1. Major. AC-02-18 fails for short blocks. `srx` runs each of the 106 rules of a language as a separate regex search over each block, and this takes 91% of the time | `break_guard::may_break` returns `false` if the block, without its last character, has none of the characters of `BREAK_CHARS`. Then `sentences::sentence_lengths` makes the block one sentence and does not call `srx`. The set is not the list of the review. The test `break_guard_covers_each_break_rule_when_vendored_rules` checks the set against each break rule of the rules file. It found two more characters, U+00BB for a Spanish rule and `<` for the marker rule `<0}`. A proptest with a fixed seed compares the guarded lengths with the lengths of `srx` in all six languages. `rules/README.md` tells to derive the set again after an update. AC-02-18 stays pending, because the machine ran parallel builds |
| 2. The first call of `prepare` took about 135 ms in the measurement of the review, because the `LazyLock` parsed 1705 rules for 34 rule sets | `rules/segment.srx` has only `GeneralImportant`, the six languages and `Default`, with their maps. It has 8 rule sets and 355 rules. `rules/README.md` records the cut and the commit. The snapshots and the abbreviation tests did not change. The source file has no license header, so the copy has none. The attribution is in `rules/README.md` |
| 3. A sentence inside an inline code span had a wrong source range | `Converter::inside_backticks` removes the backticks from the source range of `Event::Code`. The piece is exact if the code text equals the source text. Tests pin the ranges of "Use `a. B. C` end." and of spans with two and three sentences |
| 4. The tests did not cover some cases of the capture group rewrite | New tests for `\(`, `(?:`, look-behinds and named groups. The rewrite converts `(?P<name>` and `(?<name>`. The test `rules_have_no_capture_group_in_beforebreak_when_rewritten` checks each pattern of the file. The manual check in `rules/README.md` is gone |
| 5. A byte order mark U+FEFF stayed in the first segment | `Fragment::new` trims it at both ends and `Fragment::collapsed_indices` skips it. Tests in `fragment.rs` and in `tests/prepare/segmentation.rs` |
| 6. Names and STE-80 | `lang_of` is `whatlang_of` and `lang` is `detected`. The long sentences of `sentences.rs`, `rules/README.md` and this report are split |
| 7. Two tests checked two behaviors | Split in `sentences.rs` and `fragment.rs`. The new marker rule tests check one behavior each |
| 8. `Converted` repeated the fields of `Prose` | `markdown::convert` returns `Prose` through `Prose::new`. `Converted` and the copy are gone |
| 9. Known limits of the rules were not in the report | Difference 5 in section 4 |

The rules are now in `src/srx_rules.rs`, and the guard is in `src/break_guard.rs`. The limit of 300 lines for a file needs this split.

## 3. Measurements

None are recorded. The machine ran parallel builds during the work, so its times are not evidence. AC-02-18 is pending, and a run alone on the reference machine will measure it.

## 4. Differences from the plan

1. Added the files `fragment.rs`, `limits.rs`, `markdown.rs` and `boundary.rs` in `src`, and a test directory `tests/prepare/`. The reasons are the limit of 300 lines for a file and one purpose for each module. `SegmentLimits` is in `limits.rs`.
2. `srx_rules.rs` turns each capture group of a `beforebreak` pattern into a non-capturing group before the parse. `srx` 0.1.4 reads the break position from the first capture group of a rule. Thus each abbreviation rule of `segment.srx` broke the sentence after the abbreviation. No decision rule covers this. A test shows the effect, because the abbreviation fixtures fail without the change.
3. Two segments can fall in one inline code span. The map of the span is exact if the parser keeps the code text. The parser changes the text of a span in two cases. It removes one space at each end if both ends have a space. It changes a line end to a space. Then the map keeps only the edges of the span. `prepare` starts each range at or after the end of the range before it, so the ranges do not overlap (AC-02-07). The second range is empty if such a span holds two sentences. No other Markdown element has this case, because an entity, an escape and a line break give one character. The tests of AC-02-14 use the fixtures, where this does not occur.
4. `prepare` has no `tracing` span (`docs/standards.md` section 11, rule 4). The scope of the stage does not permit the dependency `tracing`. Stage 06 can put a span around its call of `prepare`.
5. The LanguageTool rules do not cover French "Mme." and Italian "sig.". The abbreviation fixtures use abbreviations that the rules cover. The rules do not need a look-around rule here, so decision rule 2 does not apply. The rules have three more limits. "5 p.m. today" splits after "p.m.". "J. R. R. Tolkien" splits after "R.". German "z. B. ein Test" splits after "z. B.".
6. `segment.srx` has the characters U+2013 and U+2014 as XML references (finding 1).
7. A plain text list such as "1. One.\n2. Two." gives the segments "1. One.", "2." and "Two.". The rules treat "2." as a sentence. Stage 06 or a later stage can decide on a fix.
8. `segment.srx` has only the rule sets that Antenna uses (finding 2 of section 2.4).
9. `prepare` skips the sentence rules for a block that cannot break (finding 1 of section 2.4). The result is the same as with the rules, and a proptest checks this.

## 5. Manual QA and sign-off

No manual QA step in this stage. Sign-off for AC-02-18. Project owner, approved on ____ .
