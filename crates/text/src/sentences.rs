use srx::Rules;

use crate::break_guard;
use crate::fragment::Fragment;

/// Splits a block into sentences. The function drops each sentence with no letter and no digit.
pub(crate) fn in_block<'a>(
    rules: &Rules,
    block: Fragment<'a>,
) -> impl Iterator<Item = Fragment<'a>> {
    let lengths = sentence_lengths(rules, block.text());
    let mut rest = block.text();
    let mut offset = block.range().start;
    lengths
        .into_iter()
        .map(move |length| {
            let (sentence, tail) = rest.split_at(length);
            rest = tail;
            let fragment = Fragment::new(sentence, offset);
            offset += length;
            fragment
        })
        .filter(|fragment| fragment.text().chars().any(char::is_alphanumeric))
}

/// Returns the byte length of each sentence of the text. The rules run only if the text has a
/// character that a break needs. Without it, the text is one sentence.
fn sentence_lengths(rules: &Rules, text: &str) -> Vec<usize> {
    let text = newlines_first(text);
    if !break_guard::may_break(&text) {
        return vec![text.len()];
    }
    rules
        .split_ranges(&text)
        .into_iter()
        .map(|range| range.len())
        .collect()
}

/// Moves the newline of each ASCII whitespace run to the start of the run. The result has the same
/// byte offsets as the text. The rules do not break a sentence when whitespace comes between the
/// sentence end and the newline. This occurs in a line that ends with a space and in a Windows
/// line end.
fn newlines_first(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut other_space = String::new();
    for character in text.chars() {
        match character {
            '\n' => output.push(character),
            ' ' | '\t' | '\r' => other_space.push(character),
            _ => {
                output.push_str(&other_space);
                other_space.clear();
                output.push(character);
            }
        }
    }
    output.push_str(&other_space);
    output
}

#[cfg(test)]
mod tests {
    use std::ops::Range;

    use antenna_core::Language;
    use proptest::prelude::*;
    use proptest::test_runner::{Config, RngAlgorithm, RngSeed};

    use crate::srx_rules::for_language;

    use super::*;

    const CASES: u32 = 1024;
    const SEED: u64 = 0x5E27_E2CE;
    const BLOCK_PATTERN: &str = concat!(
        "[A-Ca-c0-2 .!?:;,\\n\\r\\t<>{}\\[\\]()'\"*-",
        "\\x{bb}\\x{2026}\\x{2029}\\x{a0}\\x{e9}]{1,60}"
    );

    fn sentences_of(text: &str, language: Language) -> Vec<(String, Range<usize>)> {
        let rules = for_language(language).unwrap();

        in_block(rules, Fragment::new(text, 0))
            .map(|sentence| (sentence.text().to_owned(), sentence.range()))
            .collect()
    }

    fn texts_of(text: &str) -> Vec<String> {
        sentences_of(text, Language::En)
            .into_iter()
            .map(|(sentence, _)| sentence)
            .collect()
    }

    fn unguarded_lengths(rules: &Rules, text: &str) -> Vec<usize> {
        let ranges = rules.split_ranges(&newlines_first(text));

        ranges.into_iter().map(|range| range.len()).collect()
    }

    #[test]
    fn sentences_split_when_period_ends_sentence() {
        let sentences = sentences_of("  One two.  Three four. ", Language::En);

        assert_eq!(
            sentences,
            [
                ("One two.".to_owned(), 2..10),
                ("Three four.".to_owned(), 12..23)
            ]
        );
    }

    #[test]
    fn sentences_split_when_line_ends_have_carriage_returns() {
        let texts = texts_of("One two.\r\nThree four.\r\nFive.");

        assert_eq!(texts, ["One two.", "Three four.", "Five."]);
    }

    #[test]
    fn sentences_split_when_line_ends_have_spaces() {
        let texts = texts_of("One two. \nThree four.\t\nFive.");

        assert_eq!(texts, ["One two.", "Three four.", "Five."]);
    }

    #[test]
    fn sentences_split_when_marker_rules_apply() {
        assert_eq!(texts_of("One<0}"), ["One", "<0}"]);
        assert_eq!(texts_of("{0>One"), ["{0>", "One"]);
    }

    #[test]
    fn sentence_lengths_split_when_closing_quote_alone_precedes_break() {
        let rules = for_language(Language::Es).unwrap();

        let lengths = sentence_lengths(rules, "a\u{bb} \u{bb}b");

        assert_eq!(lengths, [4, 3]);
    }

    #[test]
    fn sentences_stay_one_when_block_has_no_break_character() {
        assert_eq!(texts_of("A short item."), ["A short item."]);
    }

    #[test]
    fn newlines_first_keeps_byte_length_when_runs_have_newlines() {
        let text = "a \r\n b\n\t c \u{a0}\n";

        assert_eq!(newlines_first(text).len(), text.len());
    }

    #[test]
    fn newlines_first_moves_newline_to_run_start_when_run_has_newline() {
        assert_eq!(newlines_first("a \r\n b"), "a\n \r b");
    }

    #[test]
    fn sentences_drop_fragment_when_no_letter_or_digit() {
        let texts = texts_of("One. ... ?! 2.");

        assert_eq!(texts, ["One.", "2."]);
    }

    proptest! {
        #![proptest_config(Config {
            cases: CASES,
            rng_seed: RngSeed::Fixed(SEED),
            rng_algorithm: RngAlgorithm::ChaCha,
            failure_persistence: None,
            ..Config::default()
        })]

        #[test]
        fn sentence_lengths_match_rules_when_block_is_random(
            text in BLOCK_PATTERN,
            language in prop::sample::select(Language::ALL.to_vec()),
        ) {
            let rules = for_language(language).unwrap();

            let guarded = sentence_lengths(rules, &text);

            prop_assert_eq!(guarded, unguarded_lengths(rules, &text));
        }
    }
}
