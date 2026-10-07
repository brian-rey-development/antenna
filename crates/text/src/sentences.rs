use std::sync::{Arc, LazyLock};

use antenna_core::Language;
use srx::{Rules, SRX};

use crate::TextError;
use crate::fragment::Fragment;

#[expect(
    clippy::large_include_file,
    reason = "segment.srx is the vendored LanguageTool rule file, 216 KB"
)]
const SEGMENT_SRX: &str = include_str!("../rules/segment.srx");

const BEFORE_BREAK_OPEN: &str = "<beforebreak>";
const BEFORE_BREAK_CLOSE: &str = "</beforebreak>";

static RULES: LazyLock<Result<RulesByLanguage, Arc<srx::Error>>> = LazyLock::new(|| {
    parse()
        .map(|srx| RulesByLanguage::new(&srx))
        .map_err(Arc::new)
});

fn parse() -> Result<SRX, srx::Error> {
    without_capture_groups(SEGMENT_SRX).parse()
}

/// Turns each capture group of a `beforebreak` pattern into a non-capturing group.
///
/// The `srx` crate takes the break position from the first capture group of a rule (`Rule` in
/// `srx` 0.1.4). A group in a `beforebreak` pattern moves the position to the start of an
/// abbreviation, so the abbreviation rules do not work. The function reads a character class as
/// one pair of brackets.
fn without_capture_groups(rules: &str) -> String {
    let mut output = String::with_capacity(rules.len());
    let mut rest = rules;
    while let Some((head, after_open)) = rest.split_once(BEFORE_BREAK_OPEN) {
        let Some((pattern, tail)) = after_open.split_once(BEFORE_BREAK_CLOSE) else {
            break;
        };
        output.push_str(head);
        output.push_str(BEFORE_BREAK_OPEN);
        push_non_capturing(pattern, &mut output);
        output.push_str(BEFORE_BREAK_CLOSE);
        rest = tail;
    }
    output.push_str(rest);
    output
}

fn push_non_capturing(pattern: &str, output: &mut String) {
    let mut characters = pattern.chars().peekable();
    let mut is_in_class = false;
    while let Some(character) = characters.next() {
        output.push(character);
        match character {
            '\\' => output.extend(characters.next()),
            '[' => is_in_class = true,
            ']' => is_in_class = false,
            '(' if !is_in_class && characters.peek() != Some(&'?') => output.push_str("?:"),
            _ => {}
        }
    }
}

struct RulesByLanguage {
    en: Rules,
    es: Rules,
    pt: Rules,
    fr: Rules,
    it: Rules,
    de: Rules,
}

impl RulesByLanguage {
    fn new(srx: &SRX) -> Self {
        let rules = |language: Language| srx.language_rules(<&str>::from(language));
        Self {
            en: rules(Language::En),
            es: rules(Language::Es),
            pt: rules(Language::Pt),
            fr: rules(Language::Fr),
            it: rules(Language::It),
            de: rules(Language::De),
        }
    }

    fn get(&self, language: Language) -> &Rules {
        match language {
            Language::En => &self.en,
            Language::Es => &self.es,
            Language::Pt => &self.pt,
            Language::Fr => &self.fr,
            Language::It => &self.it,
            Language::De => &self.de,
        }
    }
}

/// Returns the sentence rules of a language. The rules are built one time.
pub(crate) fn rules(language: Language) -> Result<&'static Rules, TextError> {
    match &*RULES {
        Ok(rules) => Ok(rules.get(language)),
        Err(error) => Err(TextError::Rules(Arc::clone(error))),
    }
}

/// Splits a block into sentences. The function drops each sentence with no letter and no digit.
pub(crate) fn in_block<'a>(
    rules: &Rules,
    block: Fragment<'a>,
) -> impl Iterator<Item = Fragment<'a>> {
    let lengths = rules
        .split_ranges(&newlines_first(block.text()))
        .into_iter()
        .map(|range| range.len());
    let mut rest = block.text();
    let mut offset = block.range().start;
    lengths
        .map(move |length| {
            let (sentence, tail) = rest.split_at(length);
            rest = tail;
            let fragment = Fragment::new(sentence, offset);
            offset += length;
            fragment
        })
        .filter(|fragment| fragment.text().chars().any(char::is_alphanumeric))
}

/// Moves the newline of each ASCII whitespace run to the start of the run. The rules do not break
/// a sentence when whitespace comes between the sentence end and the newline, for example in a
/// line that ends with a space or in a Windows line end. The result has the same byte offsets.
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

    use super::*;

    fn sentences_of(text: &str, language: Language) -> Vec<(String, Range<usize>)> {
        let rules = rules(language).unwrap();

        in_block(rules, Fragment::new(text, 0))
            .map(|sentence| (sentence.text().to_owned(), sentence.range()))
            .collect()
    }

    #[test]
    fn srx_rules_parse_for_all_languages() {
        for language in Language::ALL {
            let rules = rules(language);

            assert!(
                rules.is_ok_and(|rules| !rules.is_empty()),
                "language {language}"
            );
        }
    }

    #[test]
    fn srx_ignored_rule_count_is_known() {
        let srx = parse().unwrap();
        let ignored = |name: &str| srx.errors()[&srx::Language(name.to_owned())].len();

        let counts = [
            ("English", 2),
            ("Spanish", 0),
            ("Portuguese", 0),
            ("French", 0),
            ("Italian", 0),
            ("German", 0),
        ];

        assert_eq!(ignored("GeneralImportant"), 0);
        for (name, count) in counts {
            assert_eq!(ignored(name), count, "rules {name}");
        }
    }

    #[test]
    fn capture_groups_become_non_capturing_when_in_beforebreak_only() {
        let rules = "<beforebreak>\\b(Mr|[(]x)\\.(?i)(a)</beforebreak><afterbreak>(b)</afterbreak>";

        let result = without_capture_groups(rules);

        let expected =
            "<beforebreak>\\b(?:Mr|[(]x)\\.(?i)(?:a)</beforebreak><afterbreak>(b)</afterbreak>";
        assert_eq!(result, expected);
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
        let sentences = sentences_of("One two.\r\nThree four.\r\nFive.", Language::En);

        let texts: Vec<&str> = sentences.iter().map(|(text, _)| text.as_str()).collect();
        assert_eq!(texts, ["One two.", "Three four.", "Five."]);
    }

    #[test]
    fn sentences_split_when_line_ends_have_spaces() {
        let sentences = sentences_of("One two. \nThree four.\t\nFive.", Language::En);

        let texts: Vec<&str> = sentences.iter().map(|(text, _)| text.as_str()).collect();
        assert_eq!(texts, ["One two.", "Three four.", "Five."]);
    }

    #[test]
    fn newlines_first_keeps_byte_length_when_runs_have_newlines() {
        let text = "a \r\n b\n\t c \u{a0}\n";

        assert_eq!(newlines_first(text).len(), text.len());
        assert_eq!(newlines_first("a \r\n b"), "a\n \r b");
    }

    #[test]
    fn sentences_drop_fragment_when_no_letter_or_digit() {
        let sentences = sentences_of("One. ... ?! 2.", Language::En);

        let texts: Vec<&str> = sentences.iter().map(|(text, _)| text.as_str()).collect();
        assert_eq!(texts, ["One.", "2."]);
    }
}
