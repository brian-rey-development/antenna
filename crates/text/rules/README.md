# Sentence rules

The file `segment.srx` contains the sentence segmentation rules of LanguageTool.
The `srx` crate reads the file.

| Item | Value |
|---|---|
| Attribution | Segmentation rules from LanguageTool |
| Source | https://github.com/languagetool-org/languagetool/blob/be43b493a6155e612a30b1aaadb84ebfd42c92cf/languagetool-core/src/main/resources/org/languagetool/resource/segment.srx |
| Commit | `be43b493a6155e612a30b1aaadb84ebfd42c92cf` (2026-09-10) |
| License | LGPL-2.1-or-later |

The file is a copy of the file at the commit with two changes.

1. The copy has only the rules that Antenna uses. The full file has 34 rule sets and 1,705 rules. Parsing all of them makes the first call of `prepare` slow. The copy keeps the rule sets `GeneralImportant`, `English`, `Spanish`, `Portuguese`, `French`, `Italian`, `German` and `Default`, and the `languagemap` elements that point to them. The header, the order of the rules and the text of each kept rule are the same as in the source. The source file has no license header, so the copy has none.
2. The repository forbids the characters U+2013 and U+2014 in text files, so the copy writes them as the XML references `&#x2013;` and `&#x2014;`. An XML parser reads the same characters.

The code in `src/srx_rules.rs` also changes the rules in memory before the parse. The `srx` crate reads the break position from the first capture group of a rule. The code turns each capture group of a `beforebreak` pattern into a non-capturing group. The test `rules_have_no_capture_group_in_beforebreak_when_rewritten` checks all patterns of the file.

The code in `src/break_guard.rs` skips the rules for a block that cannot break. The set `BREAK_CHARS` lists the characters that each `break="yes"` rule needs before the break. The test `break_guard_covers_each_break_rule_when_vendored_rules` checks the set against the file.

To update the file, do these steps.

1. Copy the file from a newer commit.
2. Remove the rule sets and the `languagemap` elements that Antenna does not use.
3. Write the U+2013 and U+2014 characters as references.
4. Write the new commit in the table above.
5. Run the tests of `antenna-text`. If `break_guard_covers_each_break_rule_when_vendored_rules` fails, derive `BREAK_CHARS` again from the new rules.
