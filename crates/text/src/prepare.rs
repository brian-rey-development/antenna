use std::iter;
use std::ops::Range;

use antenna_core::{Document, Language, Segment, SegmentIndex};

use crate::fragment::Fragment;
use crate::prose::Prose;
use crate::source_map::SourceMap;
use crate::{SegmentLimits, TextError, sentences, split, srx_rules};

/// Converts a document into the segments that an engine speaks, in document sequence.
///
/// The segments are the same for the same arguments. A segment never crosses a block of the
/// document, for example a heading is always a segment of its own.
///
/// The source range of a segment never overlaps the range of the segment before it. An inline code
/// span with a space at both ends or a line end has no exact map, because the parser changes its
/// text. If such a span holds two sentences, the second range is empty.
///
/// # Errors
///
/// Returns [`TextError::Rules`] if the sentence rules do not parse, and [`TextError::NoSpeech`] if
/// the document has no sentence with a letter or a digit.
pub fn prepare(
    document: &Document,
    language: Language,
    limits: SegmentLimits,
) -> Result<Vec<Segment>, TextError> {
    let rules = srx_rules::for_language(language)?;
    let prose = Prose::for_document(document);
    let mut fragments = prose
        .blocks()
        .flat_map(|block| sentences::in_block(rules, block))
        .flat_map(|sentence| split::long_sentence(sentence, limits));
    let (head, tail) = split::first_segment(fragments.next().ok_or(TextError::NoSpeech)?, limits);
    let mut previous_end = 0;
    let indexes = (0..=u32::MAX).map(SegmentIndex::new);
    Ok(indexes
        .zip(iter::once(head).chain(tail).chain(fragments))
        .map(|(index, fragment)| {
            let source = source_after(previous_end, fragment, prose.map());
            previous_end = source.end;
            Segment::new(index, fragment.collapsed().collect::<String>(), source)
        })
        .collect())
}

/// Returns the source range of a fragment. The range starts at `previous_end` or later, so the
/// ranges of two segments never overlap.
fn source_after(previous_end: usize, fragment: Fragment<'_>, map: &SourceMap) -> Range<usize> {
    let source = map.source(fragment.range());
    source.start.max(previous_end)..source.end.max(previous_end)
}
