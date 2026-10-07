/// The characters that a sentence break needs in the text before the break.
///
/// Each `break="yes"` rule of `rules/segment.srx` matches only text that has one of these
/// characters. The sentence rules have `. ! ? … :` and `»`. The line end rules have a newline and
/// U+2029. The marker rule `{0>` has `>`. The marker rule `<0}` has no text before the break, so
/// the set has the first character of the text after the break, `<`. The test
/// `break_guard_covers_each_break_rule_when_vendored_rules` checks the set against the rules file.
const BREAK_CHARS: [char; 10] = [
    '.', '!', '?', '\u{2026}', ':', '\u{bb}', '\n', '\u{2029}', '<', '>',
];

/// Tells if the rules can break the text. A break needs a character from `BREAK_CHARS` before it,
/// and the rules never break at the end of the text. Thus the function reads all characters but
/// the last. If it returns `false`, the text is one sentence and the rules need not run.
pub(crate) fn may_break(text: &str) -> bool {
    text.chars()
        .rev()
        .skip(1)
        .any(|character| BREAK_CHARS.contains(&character))
}

#[cfg(test)]
mod tests {
    use std::iter::Peekable;
    use std::str::Chars;

    use crate::srx_rules::SEGMENT_SRX;

    use super::*;

    type Characters<'a> = Peekable<Chars<'a>>;

    const RULE_OPEN: &str = "<rule break=\"yes\">";
    const RULE_CLOSE: &str = "</rule>";

    /// The characters that the part of a pattern at the top level needs, one set for each element
    /// that the match must contain. A set is empty if the element is not a literal.
    fn required_sets(pattern: &str) -> Option<Vec<Vec<char>>> {
        let mut characters = pattern.chars().peekable();
        let mut sets = Vec::new();
        while let Some(character) = characters.next() {
            let members = match character {
                '|' => return None,
                '^' | '$' => continue,
                '\\' => escaped(&mut characters),
                '[' => class(&mut characters),
                '(' => skip_group(&mut characters),
                literal => vec![literal],
            };
            if is_required(&mut characters) {
                sets.push(members);
            }
        }
        Some(sets)
    }

    fn escaped(characters: &mut Characters<'_>) -> Vec<char> {
        match characters.next() {
            Some('n') => vec!['\n'],
            Some('r') => vec!['\r'],
            Some('u') => {
                let code: String = characters.take(4).collect();
                let parsed = u32::from_str_radix(&code, 16).ok().and_then(char::from_u32);
                parsed.into_iter().collect()
            }
            Some('p' | 'P') if characters.peek() == Some(&'{') => {
                characters.by_ref().find(|character| *character == '}');
                Vec::new()
            }
            Some(literal) if !literal.is_ascii_alphanumeric() => vec![literal],
            _ => Vec::new(),
        }
    }

    fn class(characters: &mut Characters<'_>) -> Vec<char> {
        let is_negated = characters.next_if_eq(&'^').is_some();
        let mut members = Vec::new();
        while let Some(character) = characters.next() {
            match character {
                ']' => break,
                '\\' => members.extend(escaped_or_unknown(characters)),
                literal => members.push(literal),
            }
        }
        if is_negated { Vec::new() } else { members }
    }

    fn escaped_or_unknown(characters: &mut Characters<'_>) -> Vec<char> {
        let members = escaped(characters);
        if members.is_empty() {
            vec!['\0']
        } else {
            members
        }
    }

    fn skip_group(characters: &mut Characters<'_>) -> Vec<char> {
        let mut depth = 1;
        while depth > 0 {
            match characters.next() {
                Some('\\') => drop(characters.next()),
                Some('(') => depth += 1,
                Some(')') | None => depth -= 1,
                Some(_) => {}
            }
        }
        Vec::new()
    }

    fn is_required(characters: &mut Characters<'_>) -> bool {
        match characters.peek() {
            Some('?' | '*') => {
                characters.next();
                false
            }
            Some('+') => {
                characters.next();
                true
            }
            Some('{') => {
                let count: String = characters
                    .by_ref()
                    .skip(1)
                    .take_while(|c| *c != '}')
                    .collect();
                !count.starts_with('0')
            }
            _ => true,
        }
    }

    fn is_covered(sets: &[Vec<char>]) -> bool {
        sets.iter()
            .any(|set| !set.is_empty() && set.iter().all(|member| BREAK_CHARS.contains(member)))
    }

    fn field<'a>(rule: &'a str, name: &str) -> &'a str {
        let open = format!("<{name}>");
        let close = format!("</{name}>");
        let after_open = rule.split_once(open.as_str()).unwrap().1;
        after_open.split_once(close.as_str()).unwrap().0
    }

    fn unescaped(xml: &str) -> String {
        xml.replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&amp;", "&")
    }

    fn break_rules() -> Vec<(String, String)> {
        SEGMENT_SRX
            .split(RULE_OPEN)
            .skip(1)
            .map(|part| part.split_once(RULE_CLOSE).unwrap().0)
            .map(|rule| {
                (
                    unescaped(field(rule, "beforebreak")),
                    unescaped(field(rule, "afterbreak")),
                )
            })
            .collect()
    }

    #[test]
    fn may_break_is_false_when_text_has_no_break_character() {
        assert!(!may_break("A short item"));
    }

    #[test]
    fn may_break_is_false_when_break_character_is_last() {
        assert!(!may_break("A short item."));
    }

    #[test]
    fn may_break_is_true_when_break_character_is_before_last() {
        assert!(may_break("One. Two"));
    }

    #[test]
    fn may_break_is_false_when_text_is_empty() {
        assert!(!may_break(""));
    }

    #[test]
    fn break_guard_covers_each_break_rule_when_vendored_rules() {
        let rules = break_rules();

        assert!(rules.len() > 20);
        for (before, after) in rules {
            let is_covered = if before.is_empty() {
                let mut sets = required_sets(&after).unwrap();
                sets.pop();
                is_covered(&sets)
            } else {
                is_covered(&required_sets(&before).unwrap())
            };
            assert!(is_covered, "rule {before:?} {after:?}");
        }
    }
}
