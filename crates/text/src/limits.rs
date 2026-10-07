use std::num::NonZeroUsize;

/// The default limit of the first segment of a document, in characters.
pub const FIRST_SEGMENT_CHARS: usize = 60;

const FIRST_SEGMENT_LIMIT: NonZeroUsize = match NonZeroUsize::new(FIRST_SEGMENT_CHARS) {
    Some(chars) => chars,
    None => panic!("the first segment limit is zero"),
};

/// The character limits of the segments of a document.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SegmentLimits {
    max_chars: NonZeroUsize,
    first_segment_chars: NonZeroUsize,
}

impl SegmentLimits {
    /// Makes the limits for an engine that accepts `max_chars` characters in one segment. The
    /// limit of the first segment is [`FIRST_SEGMENT_CHARS`], or `max_chars` if that is smaller.
    pub fn new(max_chars: NonZeroUsize) -> Self {
        Self {
            max_chars,
            first_segment_chars: max_chars.min(FIRST_SEGMENT_LIMIT),
        }
    }

    /// Returns the maximum number of characters of a segment.
    pub fn max_chars(self) -> NonZeroUsize {
        self.max_chars
    }

    /// Returns the limit of the first segment, in characters. The text crate splits a longer first
    /// sentence at a clause boundary, so the first audio starts sooner.
    pub fn first_segment_chars(self) -> NonZeroUsize {
        self.first_segment_chars
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limits(max_chars: usize) -> SegmentLimits {
        SegmentLimits::new(NonZeroUsize::new(max_chars).unwrap())
    }

    #[test]
    fn segment_limits_clamp_first_segment_when_max_chars_small() {
        let small = limits(10);

        assert_eq!(small.max_chars().get(), 10);
        assert_eq!(small.first_segment_chars().get(), 10);
    }

    #[test]
    fn segment_limits_use_first_segment_chars_when_max_chars_large() {
        for max_chars in [FIRST_SEGMENT_CHARS, 400] {
            assert_eq!(
                limits(max_chars).first_segment_chars().get(),
                FIRST_SEGMENT_CHARS
            );
        }
    }
}
