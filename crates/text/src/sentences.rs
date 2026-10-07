use srx::Rules;

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

/// Returns the byte length of each sentence of the text.
fn sentence_lengths(rules: &Rules, text: &str) -> Vec<usize> {
    let text = newlines_first(text);
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

    use crate::srx_rules::for_language;

    use super::*;

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
}
