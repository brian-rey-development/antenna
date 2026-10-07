use std::ops::Range;

#[derive(Debug)]
struct Piece {
    prose: Range<usize>,
    source: Range<usize>,
    is_exact: bool,
}

impl Piece {
    fn start_in_source(&self, prose_position: usize) -> usize {
        if self.is_exact {
            self.source.start + self.offset_of(prose_position)
        } else {
            self.source.start
        }
    }

    fn end_in_source(&self, prose_position: usize) -> usize {
        if self.is_exact {
            self.source.start + self.offset_of(prose_position)
        } else {
            self.source.end
        }
    }

    fn offset_of(&self, prose_position: usize) -> usize {
        prose_position.clamp(self.prose.start, self.prose.end) - self.prose.start
    }
}

/// A sorted list of pieces. Each piece maps a byte range of the prose to a byte range of the
/// document.
#[derive(Debug, Default)]
pub(crate) struct SourceMap {
    pieces: Vec<Piece>,
}

impl SourceMap {
    /// Adds a piece whose prose text and source text are equal. A position inside the piece maps
    /// to the exact source position. The piece must start after the end of the last piece.
    pub(crate) fn push_exact(&mut self, prose_start: usize, source: Range<usize>) {
        let prose = prose_start..prose_start + source.len();
        self.pieces.push(Piece {
            prose,
            source,
            is_exact: true,
        });
    }

    /// Adds a piece whose prose text and source text can differ. A position inside the piece maps
    /// to the edge of the source range. The piece must start after the end of the last piece.
    pub(crate) fn push_edges(&mut self, prose: Range<usize>, source: Range<usize>) {
        self.pieces.push(Piece {
            prose,
            source,
            is_exact: false,
        });
    }

    /// Returns the source range of a prose range. The range starts in the source of the first
    /// piece that it touches and ends in the source of the last piece that it touches. The result
    /// is the empty range at 0 if the range starts after the last piece.
    pub(crate) fn source(&self, prose: Range<usize>) -> Range<usize> {
        let first = self
            .pieces
            .partition_point(|piece| piece.prose.end <= prose.start);
        let after_last = self
            .pieces
            .partition_point(|piece| piece.prose.start < prose.end);
        let Some(first_piece) = self.pieces.get(first) else {
            return 0..0;
        };
        let start = first_piece.start_in_source(prose.start);
        let end = after_last
            .checked_sub(1)
            .and_then(|last| self.pieces.get(last))
            .map_or(start, |piece| piece.end_in_source(prose.end));
        start..end.max(start)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map_of_two_exact_pieces() -> SourceMap {
        let mut map = SourceMap::default();
        map.push_exact(0, 10..15);
        map.push_exact(5, 20..30);
        map
    }

    #[test]
    fn source_is_exact_when_range_is_inside_exact_piece() {
        let map = map_of_two_exact_pieces();

        assert_eq!(map.source(7..9), 22..24);
    }

    #[test]
    fn source_spans_pieces_when_range_crosses_pieces() {
        let map = map_of_two_exact_pieces();

        assert_eq!(map.source(2..8), 12..23);
    }

    #[test]
    fn source_is_piece_edge_when_range_is_inside_edge_piece() {
        let mut map = SourceMap::default();
        map.push_edges(0..1, 4..9);

        assert_eq!(map.source(0..1), 4..9);
    }

    #[test]
    fn source_starts_at_next_piece_when_range_starts_at_piece_end() {
        let map = map_of_two_exact_pieces();

        assert_eq!(map.source(5..7), 20..22);
    }

    #[test]
    fn source_skips_gap_when_range_starts_between_pieces() {
        let mut map = SourceMap::default();
        map.push_exact(0, 0..3);
        map.push_exact(4, 10..13);

        assert_eq!(map.source(3..6), 10..12);
    }

    #[test]
    fn source_is_empty_when_map_is_empty() {
        assert_eq!(SourceMap::default().source(0..1), 0..0);
    }

    #[test]
    fn source_is_empty_when_range_is_after_all_pieces() {
        let map = map_of_two_exact_pieces();

        assert_eq!(map.source(20..30), 0..0);
    }

    #[test]
    fn source_is_empty_when_range_is_inside_gap() {
        let mut map = SourceMap::default();
        map.push_exact(0, 0..3);
        map.push_exact(5, 10..13);

        assert_eq!(map.source(3..5), 10..10);
    }
}
