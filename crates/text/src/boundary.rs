use crate::fragment::{Fragment, SPACE};

pub(crate) const CLAUSE_END_CHARS: [char; 6] = [',', ';', ':', ')', '\u{2014}', '\u{2013}'];

#[derive(Clone, Copy)]
pub(crate) struct Boundary {
    pub(crate) offset: usize,
    pub(crate) chars_before: usize,
    pub(crate) is_clause: bool,
}

/// Returns each whitespace run of the sentence, in sequence. A boundary has the byte offset of
/// the run and the number of characters of the collapsed text before it. The iterator ends after
/// the last boundary with `limit` characters or fewer before it.
pub(crate) fn boundaries(
    sentence: Fragment<'_>,
    limit: usize,
) -> impl Iterator<Item = Boundary> + '_ {
    let mut previous = SPACE;
    sentence
        .collapsed_indices()
        .enumerate()
        .take_while(move |(chars_before, _)| *chars_before <= limit)
        .filter_map(move |(chars_before, (offset, character))| {
            let is_boundary = character == SPACE;
            let is_clause = CLAUSE_END_CHARS.contains(&previous);
            previous = character;
            is_boundary.then_some(Boundary {
                offset,
                chars_before,
                is_clause,
            })
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn boundaries_of(text: &str, limit: usize) -> Vec<(usize, usize, bool)> {
        boundaries(Fragment::new(text, 0), limit)
            .map(|boundary| (boundary.offset, boundary.chars_before, boundary.is_clause))
            .collect()
    }

    #[test]
    fn boundaries_give_offset_and_length_when_whitespace_runs() {
        let found = boundaries_of("ab,  cd ef", 100);

        assert_eq!(found, [(3, 3, true), (7, 6, false)]);
    }

    #[test]
    fn boundaries_stop_when_length_passes_limit() {
        let found = boundaries_of("ab cd ef gh", 5);

        assert_eq!(found, [(2, 2, false), (5, 5, false)]);
    }
}
