use std::ops::Range;

pub(crate) const SPACE: char = ' ';
const BYTE_ORDER_MARK: char = '\u{feff}';

/// A part of the prose with no whitespace at its ends, and its byte offset in the prose. The
/// byte order mark U+FEFF is not part of a fragment.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Fragment<'a> {
    text: &'a str,
    start: usize,
}

impl<'a> Fragment<'a> {
    /// Makes a fragment of the text that starts at the byte offset `start` of the prose. The
    /// fragment drops the whitespace and the byte order marks at both ends of the text.
    pub(crate) fn new(text: &'a str, start: usize) -> Self {
        let content = text.trim_start_matches(is_blank);
        Self {
            text: content.trim_end_matches(is_blank),
            start: start + text.len() - content.len(),
        }
    }

    pub(crate) fn text(self) -> &'a str {
        self.text
    }

    pub(crate) fn range(self) -> Range<usize> {
        self.start..self.start + self.text.len()
    }

    /// Splits the fragment at a byte offset of its text. Both parts drop their outer whitespace.
    pub(crate) fn split_at(self, offset: usize) -> (Self, Self) {
        let (head, tail) = self.text.split_at(offset);
        (
            Self::new(head, self.start),
            Self::new(tail, self.start + offset),
        )
    }

    /// Returns each character of the text with its byte offset, where each whitespace run is one
    /// space at the offset of its first whitespace character. The byte order marks are not in the
    /// result.
    pub(crate) fn collapsed_indices(self) -> impl Iterator<Item = (usize, char)> + 'a {
        let mut is_after_space = false;
        self.text
            .char_indices()
            .filter(|(_, character)| *character != BYTE_ORDER_MARK)
            .filter_map(move |(offset, character)| {
                let is_space = character.is_whitespace();
                let is_kept = !(is_space && is_after_space);
                is_after_space = is_space;
                is_kept.then_some((offset, if is_space { SPACE } else { character }))
            })
    }

    /// Returns the characters of the text with each whitespace run replaced by one space.
    pub(crate) fn collapsed(self) -> impl Iterator<Item = char> + 'a {
        self.collapsed_indices().map(|(_, character)| character)
    }

    /// Tells if the collapsed text has more than `limit` characters. The check reads at most
    /// `limit` plus one characters.
    pub(crate) fn exceeds(self, limit: usize) -> bool {
        self.collapsed().nth(limit).is_some()
    }
}

fn is_blank(character: char) -> bool {
    character.is_whitespace() || character == BYTE_ORDER_MARK
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fragment_drops_outer_whitespace_when_made() {
        let fragment = Fragment::new("  \tHello world. \n", 10);

        assert_eq!(fragment.text(), "Hello world.");
        assert_eq!(fragment.range(), 13..25);
    }

    #[test]
    fn fragment_is_empty_when_text_is_whitespace() {
        let fragment = Fragment::new(" \n ", 4);

        assert_eq!(fragment.text(), "");
        assert!(fragment.range().is_empty());
    }

    #[test]
    fn fragment_trims_both_parts_when_split() {
        let fragment = Fragment::new("one  two", 5);

        let (head, tail) = fragment.split_at(3);

        assert_eq!((head.text(), head.range()), ("one", 5..8));
        assert_eq!((tail.text(), tail.range()), ("two", 10..13));
    }

    #[test]
    fn fragment_collapses_whitespace_runs_to_one_space() {
        let fragment = Fragment::new("a \t b\n\nc", 0);

        let text: String = fragment.collapsed().collect();

        assert_eq!(text, "a b c");
    }

    #[test]
    fn fragment_gives_offsets_when_collapsed_indices() {
        let fragment = Fragment::new("a \t b", 0);

        let indices: Vec<_> = fragment.collapsed_indices().collect();

        assert_eq!(indices, [(0, 'a'), (1, ' '), (4, 'b')]);
    }

    #[test]
    fn fragment_exceeds_when_collapsed_text_is_longer_than_limit() {
        let fragment = Fragment::new("ab   cd", 0);

        assert!(fragment.exceeds(4));
        assert!(!fragment.exceeds(5));
    }

    #[test]
    fn fragment_drops_byte_order_mark_when_at_ends() {
        let fragment = Fragment::new("\u{feff} \u{feff}Hello\u{feff}", 0);

        assert_eq!(fragment.text(), "Hello");
        assert_eq!(fragment.range(), 7..12);
    }

    #[test]
    fn fragment_drops_byte_order_mark_when_collapsed_inside_text() {
        let fragment = Fragment::new("a\u{feff}b \u{feff} c", 0);

        let text: String = fragment.collapsed().collect();

        assert_eq!(text, "ab c");
    }
}
