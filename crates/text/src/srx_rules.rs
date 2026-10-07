use std::sync::{Arc, LazyLock};

use antenna_core::Language;
use srx::{Rules, SRX};

use crate::TextError;

pub(crate) const SEGMENT_SRX: &str = include_str!("../rules/segment.srx");

const BEFORE_BREAK_OPEN: &str = "<beforebreak>";
const BEFORE_BREAK_CLOSE: &str = "</beforebreak>";
const NAMED_GROUP_OPENERS: [&str; 2] = ["?P<", "?<"];
const NON_CAPTURING_OPEN: &str = "(?:";

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
    let mut characters = pattern.chars();
    let mut is_in_class = false;
    while let Some(character) = characters.next() {
        if character == '(' && !is_in_class {
            let (opening, skipped) = group_opening(characters.as_str());
            output.push_str(opening);
            characters = characters
                .as_str()
                .get(skipped..)
                .unwrap_or_default()
                .chars();
            continue;
        }
        output.push(character);
        match character {
            '\\' => output.extend(characters.next()),
            '[' => is_in_class = true,
            ']' => is_in_class = false,
            _ => {}
        }
    }
}

/// Returns the text that replaces a `(`, and the length of the group name that follows it.
fn group_opening(after_paren: &str) -> (&'static str, usize) {
    if let Some(length) = named_group_length(after_paren) {
        (NON_CAPTURING_OPEN, length)
    } else if after_paren.starts_with('?') {
        ("(", 0)
    } else {
        (NON_CAPTURING_OPEN, 0)
    }
}

/// Returns the length of `?P<name>` or `?<name>` at the start of the text. A `?<` that is followed
/// by `=` or `!` is a look-behind, not a name.
fn named_group_length(after_paren: &str) -> Option<usize> {
    let opener = NAMED_GROUP_OPENERS
        .into_iter()
        .find(|opener| after_paren.starts_with(opener))?;
    let name = after_paren.strip_prefix(opener)?;
    name.chars().next().filter(|first| first.is_alphabetic())?;
    Some(opener.len() + name.find('>')? + 1)
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
pub(crate) fn for_language(language: Language) -> Result<&'static Rules, TextError> {
    match &*RULES {
        Ok(rules) => Ok(rules.get(language)),
        Err(error) => Err(TextError::Rules(Arc::clone(error))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn capture_groups(pattern: &str) -> usize {
        let mut characters = pattern.chars();
        let mut count = 0;
        let mut is_in_class = false;
        while let Some(character) = characters.next() {
            match character {
                '\\' => drop(characters.next()),
                '[' => is_in_class = true,
                ']' => is_in_class = false,
                '(' if !is_in_class => {
                    let after = characters.as_str();
                    let is_plain = !after.starts_with('?');
                    count += usize::from(is_plain || named_group_length(after).is_some());
                }
                _ => {}
            }
        }
        count
    }

    fn before_break_patterns(rules: &str) -> Vec<&str> {
        rules
            .split(BEFORE_BREAK_OPEN)
            .skip(1)
            .filter_map(|part| part.split_once(BEFORE_BREAK_CLOSE))
            .map(|(pattern, _)| pattern)
            .collect()
    }

    fn rewritten(rules: &str) -> String {
        without_capture_groups(&format!("{BEFORE_BREAK_OPEN}{rules}{BEFORE_BREAK_CLOSE}"))
    }

    #[test]
    fn srx_rules_parse_for_all_languages() {
        for language in Language::ALL {
            let rules = for_language(language);

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
    fn rewrite_keeps_escaped_parenthesis_when_pattern_has_one() {
        assert_eq!(
            rewritten("\\(a\\)(b)"),
            "<beforebreak>\\(a\\)(?:b)</beforebreak>"
        );
    }

    #[test]
    fn rewrite_keeps_group_when_already_non_capturing() {
        assert_eq!(
            rewritten("(?:a)(?=b)"),
            "<beforebreak>(?:a)(?=b)</beforebreak>"
        );
    }

    #[test]
    fn rewrite_keeps_look_behind_when_pattern_has_one() {
        assert_eq!(
            rewritten("(?<=a)(?<!b)"),
            "<beforebreak>(?<=a)(?<!b)</beforebreak>"
        );
    }

    #[test]
    fn rewrite_drops_name_when_group_is_named() {
        let result = rewritten("(?P<first>a)(?<second>b)");

        assert_eq!(result, "<beforebreak>(?:a)(?:b)</beforebreak>");
    }

    #[test]
    fn rules_have_no_capture_group_in_beforebreak_when_rewritten() {
        let rules = without_capture_groups(SEGMENT_SRX);

        let patterns = before_break_patterns(&rules);

        assert!(!patterns.is_empty());
        for pattern in patterns {
            assert_eq!(capture_groups(pattern), 0, "pattern {pattern}");
        }
    }

    #[test]
    fn rules_have_capture_groups_in_beforebreak_when_not_rewritten() {
        let patterns = before_break_patterns(SEGMENT_SRX);

        assert!(
            patterns
                .into_iter()
                .any(|pattern| capture_groups(pattern) > 0)
        );
    }
}
