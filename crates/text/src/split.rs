use crate::SegmentLimits;
use crate::boundary::{Boundary, boundaries};
use crate::fragment::Fragment;

const MIN_FIRST_SEGMENT_CHARS: usize = 20;
const FIRST_SPLIT_FALLBACK_CHARS: usize = 120;
const MIN_CLAUSE_FRACTION_DENOMINATOR: usize = 2;

/// Splits a sentence into parts of `max_chars` characters or fewer. Each cut is at the last clause
/// boundary in the second half of the limit, else at the last whitespace, else at the limit.
pub(crate) fn long_sentence(sentence: Fragment<'_>, limits: SegmentLimits) -> Vec<Fragment<'_>> {
    let max = limits.max_chars().get();
    let mut parts = Vec::new();
    let mut rest = sentence;
    while rest.exceeds(max) {
        let (head, tail) = rest.split_at(long_cut(rest, max));
        parts.push(head);
        rest = tail;
    }
    parts.push(rest);
    parts
}

fn long_cut(sentence: Fragment<'_>, max: usize) -> usize {
    let min_clause = max / MIN_CLAUSE_FRACTION_DENOMINATOR;
    let (clause, whitespace) =
        boundaries(sentence, max).fold((None, None), |(clause, _), boundary| {
            let is_clause = boundary.is_clause && boundary.chars_before >= min_clause;
            (
                if is_clause { Some(boundary) } else { clause },
                Some(boundary),
            )
        });
    clause.or(whitespace).map_or_else(
        || {
            let end = sentence.text().len();
            sentence
                .collapsed_indices()
                .nth(max)
                .map_or(end, |(offset, _)| offset)
        },
        |boundary| boundary.offset,
    )
}

/// Splits the first segment of a document when it is longer than the first segment limit. The
/// cut is at the first clause boundary in range. A sentence of more than
/// `FIRST_SPLIT_FALLBACK_CHARS` characters with no such boundary is cut at the first whitespace
/// after the limit.
pub(crate) fn first_segment(
    segment: Fragment<'_>,
    limits: SegmentLimits,
) -> (Fragment<'_>, Option<Fragment<'_>>) {
    let first_chars = limits.first_segment_chars().get();
    if !segment.exceeds(first_chars) {
        return (segment, None);
    }
    let max = limits.max_chars().get();
    let clause = boundaries(segment, max).find(|boundary| {
        let range = MIN_FIRST_SEGMENT_CHARS..=first_chars;
        boundary.is_clause && range.contains(&boundary.chars_before)
    });
    let cut = clause.or_else(|| first_whitespace_after(segment, limits));
    cut.map_or((segment, None), |boundary| {
        let (head, tail) = segment.split_at(boundary.offset);
        (head, Some(tail))
    })
}

fn first_whitespace_after(segment: Fragment<'_>, limits: SegmentLimits) -> Option<Boundary> {
    if !segment.exceeds(FIRST_SPLIT_FALLBACK_CHARS) {
        return None;
    }
    let first_chars = limits.first_segment_chars().get();
    boundaries(segment, limits.max_chars().get())
        .find(|boundary| boundary.chars_before >= first_chars)
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use super::*;
    use crate::FIRST_SEGMENT_CHARS;
    use crate::boundary::CLAUSE_END_CHARS;

    const MAX_CHARS: usize = 400;
    const LONG_WORD_CHARS: usize = 100_000;

    fn limits(max_chars: usize) -> SegmentLimits {
        SegmentLimits::new(NonZeroUsize::new(max_chars).unwrap())
    }

    fn long_parts(text: &str, max_chars: usize) -> Vec<String> {
        long_sentence(Fragment::new(text, 0), limits(max_chars))
            .iter()
            .map(|part| part.text().to_owned())
            .collect()
    }

    fn first_parts(text: &str) -> (String, Option<String>) {
        let (head, tail) = first_segment(Fragment::new(text, 0), limits(MAX_CHARS));

        (
            head.text().to_owned(),
            tail.map(|tail| tail.text().to_owned()),
        )
    }

    fn words_of_chars(count: usize) -> String {
        const WORD: &str = "word ";
        let (words, rest) = match count % WORD.len() {
            0 => (count / WORD.len() - 1, WORD.len()),
            rest => (count / WORD.len(), rest),
        };
        format!("{}{}", WORD.repeat(words), "x".repeat(rest))
    }

    #[test]
    fn split_long_keeps_sentence_when_it_fits() {
        assert_eq!(long_parts("aaaa  bbbb cccc", 14), ["aaaa  bbbb cccc"]);
    }

    #[test]
    fn split_long_uses_clause_when_available() {
        let parts = long_parts("aaaa bbbb cccc, dddd eeee ffff", 20);

        assert_eq!(parts, ["aaaa bbbb cccc,", "dddd eeee ffff"]);
    }

    #[test]
    fn split_long_uses_clause_when_clause_char_is_any_clause_end() {
        for clause_end in CLAUSE_END_CHARS {
            let text = format!("aaaa bbbb cccc{clause_end} dddd eeee ffff");

            let parts = long_parts(&text, 20);

            let head = format!("aaaa bbbb cccc{clause_end}");
            assert_eq!(parts, [head.as_str(), "dddd eeee ffff"]);
        }
    }

    #[test]
    fn split_long_uses_clause_when_clause_is_at_half_of_limit() {
        let parts = long_parts("aaaa, bbbb cccc dddd eeee", 10);

        assert_eq!(parts, ["aaaa,", "bbbb cccc", "dddd eeee"]);
    }

    #[test]
    fn split_long_uses_whitespace_when_clause_is_before_half_of_limit() {
        let parts = long_parts("aaa, bbbb cccc dddd eeee", 10);

        assert_eq!(parts, ["aaa, bbbb", "cccc dddd", "eeee"]);
    }

    #[test]
    fn split_long_uses_whitespace_when_no_clause() {
        let parts = long_parts("aaaa bbbb cccc dddd eeee ffff", 20);

        assert_eq!(parts, ["aaaa bbbb cccc dddd", "eeee ffff"]);
    }

    #[test]
    fn split_long_counts_one_space_when_whitespace_run() {
        let parts = long_parts("aaaa    bbbb    cccc", 9);

        assert_eq!(parts, ["aaaa    bbbb", "cccc"]);
    }

    #[test]
    fn split_long_uses_char_boundary_when_no_whitespace() {
        let parts = long_parts("abcdefghijklmnopqrstuvwxyz", 10);

        assert_eq!(parts, ["abcdefghij", "klmnopqrst", "uvwxyz"]);
    }

    #[test]
    fn split_long_keeps_chars_whole_when_chars_are_multibyte() {
        let parts = long_parts("\u{e9}\u{e9}\u{e9}\u{e9}\u{e9}\u{e9}\u{e9}", 3);

        assert_eq!(
            parts,
            ["\u{e9}\u{e9}\u{e9}", "\u{e9}\u{e9}\u{e9}", "\u{e9}"]
        );
    }

    #[test]
    fn split_long_cuts_all_parts_when_text_is_one_long_word() {
        let word = "a".repeat(LONG_WORD_CHARS);

        let parts = long_parts(&word, 100);

        assert_eq!(parts.len(), LONG_WORD_CHARS / 100);
    }

    #[test]
    fn split_long_keeps_source_ranges_when_split() {
        let parts = long_sentence(Fragment::new("aaaa bbbb cccc", 100), limits(9));

        let ranges: Vec<_> = parts.iter().map(|part| part.range()).collect();

        assert_eq!(ranges, [100..109, 110..114]);
    }

    #[test]
    fn split_first_uses_clause_when_in_range() {
        let text =
            "The quick brown fox jumps over the lazy dog, and then it runs away from the farm.";

        let (head, tail) = first_parts(text);

        assert_eq!(head, "The quick brown fox jumps over the lazy dog,");
        assert_eq!(
            tail.as_deref(),
            Some("and then it runs away from the farm.")
        );
    }

    #[test]
    fn split_first_uses_clause_when_clause_is_at_limit() {
        let text = format!(
            "{}, {}",
            "a".repeat(FIRST_SEGMENT_CHARS - 1),
            "b".repeat(10)
        );

        let (head, tail) = first_parts(&text);

        assert_eq!(head.chars().count(), FIRST_SEGMENT_CHARS);
        assert_eq!(tail.as_deref(), Some("bbbbbbbbbb"));
    }

    #[test]
    fn split_first_keeps_sentence_when_clause_is_after_limit() {
        let text = format!("{}, {}", "a".repeat(FIRST_SEGMENT_CHARS), "b".repeat(10));

        assert_eq!(first_parts(&text).1, None);
    }

    #[test]
    fn split_first_uses_clause_when_clause_is_at_minimum() {
        let text = format!(
            "{}, {}",
            "a".repeat(MIN_FIRST_SEGMENT_CHARS - 1),
            "b".repeat(FIRST_SEGMENT_CHARS)
        );

        let (head, _) = first_parts(&text);

        assert_eq!(head.chars().count(), MIN_FIRST_SEGMENT_CHARS);
    }

    #[test]
    fn split_first_keeps_sentence_when_clause_is_before_minimum() {
        let text = format!(
            "{}, {}",
            "a".repeat(MIN_FIRST_SEGMENT_CHARS - 2),
            "b".repeat(FIRST_SEGMENT_CHARS)
        );

        assert_eq!(first_parts(&text).1, None);
    }

    #[test]
    fn split_first_keeps_sentence_when_it_fits() {
        assert_eq!(
            first_parts("Short, and fine."),
            ("Short, and fine.".to_owned(), None)
        );
    }

    #[test]
    fn split_first_uses_whitespace_when_no_clause_and_sentence_long() {
        let text = words_of_chars(FIRST_SPLIT_FALLBACK_CHARS + 1);

        let (head, tail) = first_parts(&text);

        assert!(head.chars().count() >= FIRST_SEGMENT_CHARS);
        assert!(tail.is_some());
    }

    #[test]
    fn split_first_keeps_sentence_when_no_clause_and_sentence_short() {
        let text = words_of_chars(FIRST_SPLIT_FALLBACK_CHARS);

        assert_eq!(first_parts(&text).1, None);
    }
}
