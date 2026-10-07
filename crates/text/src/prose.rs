use std::borrow::Cow;
use std::ops::Range;

use antenna_core::{Document, TextFormat};

use crate::fragment::Fragment;
use crate::markdown;
use crate::source_map::SourceMap;

/// The text that Antenna speaks, in blocks, with the map from each prose byte to the document.
#[derive(Debug)]
pub(crate) struct Prose<'a> {
    text: Cow<'a, str>,
    blocks: Vec<Range<usize>>,
    map: SourceMap,
}

impl<'a> Prose<'a> {
    pub(crate) fn for_document(document: &'a Document) -> Self {
        match document.format() {
            TextFormat::Plain => plain(document.text()),
            TextFormat::Markdown => {
                let converted = markdown::convert(document.text());
                Self {
                    text: Cow::Owned(converted.text),
                    blocks: converted.blocks,
                    map: converted.map,
                }
            }
        }
    }

    pub(crate) fn text(&self) -> &str {
        &self.text
    }

    pub(crate) fn map(&self) -> &SourceMap {
        &self.map
    }

    #[expect(
        clippy::string_slice,
        reason = "the block ranges start and end at char boundaries of the prose"
    )]
    pub(crate) fn blocks(&self) -> impl Iterator<Item = Fragment<'_>> {
        self.blocks
            .iter()
            .map(|block| Fragment::new(&self.text[block.clone()], block.start))
    }
}

fn plain(text: &str) -> Prose<'_> {
    let blocks = plain_blocks(text);
    let mut map = SourceMap::default();
    for block in &blocks {
        map.push_exact(block.start, block.clone());
    }
    Prose {
        text: Cow::Borrowed(text),
        blocks,
        map,
    }
}

fn plain_blocks(text: &str) -> Vec<Range<usize>> {
    let mut blocks = Vec::new();
    let mut open: Option<Range<usize>> = None;
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        if line.trim().is_empty() {
            blocks.extend(open.take());
        } else {
            let end = offset + line.trim_end().len();
            open = Some(open.take().map_or(offset..end, |block| block.start..end));
        }
        offset += line.len();
    }
    blocks.extend(open);
    blocks
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain_blocks_of(text: &str) -> Vec<String> {
        let document = Document::new(text, TextFormat::Plain).unwrap();

        Prose::for_document(&document)
            .blocks()
            .map(|block| block.text().to_owned())
            .collect()
    }

    #[test]
    fn prose_keeps_markup_characters_when_plain_text() {
        let blocks = plain_blocks_of("# Title\n* item *one*\n    indented\n");

        assert_eq!(blocks, ["# Title\n* item *one*\n    indented"]);
    }

    #[test]
    fn prose_ends_block_when_blank_line_in_plain_text() {
        let blocks = plain_blocks_of("one\ntwo\n \t\n\n\nthree\r\nfour\r\n");

        assert_eq!(blocks, ["one\ntwo", "three\r\nfour"]);
    }

    #[test]
    fn prose_maps_to_exact_source_when_plain_text() {
        let document = Document::new("one\n\ntwo three", TextFormat::Plain).unwrap();
        let prose = Prose::for_document(&document);

        assert_eq!(prose.map().source(10..13), 10..13);
    }
}
