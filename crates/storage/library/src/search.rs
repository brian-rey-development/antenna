use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;

/// Folds a text for a search that ignores case and accents.
///
/// The function decomposes the text with NFD and removes the combining marks. Then it converts
/// the text to lowercase and replaces each run of whitespace with one space.
pub fn fold(text: &str) -> String {
    let mut folded = String::with_capacity(text.len());
    let mut after_space = false;
    let characters = text
        .nfd()
        .filter(|&character| !is_combining_mark(character))
        .flat_map(char::to_lowercase);
    for character in characters {
        let is_space = character.is_whitespace();
        if !(is_space && after_space) {
            folded.push(if is_space { ' ' } else { character });
        }
        after_space = is_space;
    }
    folded
}

pub(crate) fn matches(terms: &[String], title: &str, text: &str) -> bool {
    terms
        .iter()
        .all(|term| title.contains(term.as_str()) || text.contains(term.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn terms(query: &str) -> Vec<String> {
        fold(query).split_whitespace().map(str::to_owned).collect()
    }

    #[test]
    fn fold_removes_accents_and_case() {
        assert_eq!(fold("Canci\u{f3}n DE Cu\u{f1}a"), "cancion de cuna");
        assert_eq!(fold("Cre\u{300}me Bru\u{302}le\u{301}e"), "creme brulee");
        assert_eq!(fold("\u{130}stanbul"), "istanbul");
    }

    #[test]
    fn fold_collapses_each_run_of_whitespace_to_one_space() {
        assert_eq!(fold("a \t\n b"), "a b");
        assert_eq!(fold("  a  "), " a ");
        assert_eq!(fold(""), "");
    }

    #[test]
    fn fold_keeps_letters_without_accents() {
        assert_eq!(
            fold("\u{65e5}\u{672c}\u{8a9e} stra\u{df}e"),
            "\u{65e5}\u{672c}\u{8a9e} stra\u{df}e"
        );
    }

    #[test]
    fn matches_requires_every_term() {
        let title = fold("Weekly plan");
        let text = fold("Buy milk and eggs");

        assert!(matches(&terms("weekly eggs"), &title, &text));
        assert!(!matches(&terms("weekly bread"), &title, &text));
    }

    #[test]
    fn matches_finds_term_inside_a_word() {
        assert!(matches(&terms("ekl"), "weekly", ""));
    }

    #[test]
    fn matches_everything_when_no_terms() {
        assert!(matches(&[], "", ""));
    }
}
