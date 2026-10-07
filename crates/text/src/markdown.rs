use std::borrow::Cow;
use std::ops::Range;

use pulldown_cmark::{Event, LinkType, Options, Parser, Tag, TagEnd};

use crate::prose::Prose;
use crate::source_map::SourceMap;

const BLOCK_SEPARATOR: char = '\n';
const SOFT_BREAK_TEXT: &str = " ";

/// Converts Markdown to prose. Each paragraph, heading, list item, table cell and thematic break
/// ends a block. Emphasis, strong text and strikethrough keep their inner text. A link keeps its
/// text and loses its URL. Autolinks, images, code blocks and HTML give no text. Inline code
/// keeps its text, and each line break gives one space.
pub(crate) fn convert(source: &str) -> Prose<'static> {
    let options = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH;
    let mut converter = Converter::new(source);
    for (event, range) in Parser::new_ext(source, options).into_offset_iter() {
        converter.handle(event, range);
    }
    converter.finish()
}

struct Converter<'a> {
    source: &'a str,
    text: String,
    block_start: usize,
    blocks: Vec<Range<usize>>,
    map: SourceMap,
    skipped_depth: usize,
    is_in_autolink: bool,
}

impl<'a> Converter<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source,
            text: String::new(),
            block_start: 0,
            blocks: Vec::new(),
            map: SourceMap::default(),
            skipped_depth: 0,
            is_in_autolink: false,
        }
    }

    fn handle(&mut self, event: Event<'_>, range: Range<usize>) {
        match event {
            Event::Start(tag) => self.start(&tag),
            Event::End(tag) => self.end(tag),
            Event::Text(text) => self.push(&text, range),
            Event::Code(text) => self.push(&text, self.inside_backticks(range)),
            Event::SoftBreak | Event::HardBreak => self.push(SOFT_BREAK_TEXT, range),
            Event::Rule => self.end_block(),
            Event::Html(_)
            | Event::InlineHtml(_)
            | Event::InlineMath(_)
            | Event::DisplayMath(_)
            | Event::FootnoteReference(_)
            | Event::TaskListMarker(_) => {}
        }
    }

    fn start(&mut self, tag: &Tag<'_>) {
        if matches!(tag, Tag::Image { .. } | Tag::CodeBlock(_) | Tag::HtmlBlock) {
            self.skipped_depth += 1;
        }
        if matches!(
            tag,
            Tag::Link {
                link_type: LinkType::Autolink | LinkType::Email,
                ..
            }
        ) {
            self.is_in_autolink = true;
        }
        let is_inline = matches!(
            tag,
            Tag::Emphasis | Tag::Strong | Tag::Strikethrough | Tag::Link { .. } | Tag::Image { .. }
        );
        if !is_inline {
            self.end_block();
        }
    }

    fn end(&mut self, tag: TagEnd) {
        if matches!(tag, TagEnd::Image | TagEnd::CodeBlock | TagEnd::HtmlBlock) {
            self.skipped_depth -= 1;
        }
        if matches!(tag, TagEnd::Link) {
            self.is_in_autolink = false;
        }
        let is_inline = matches!(
            tag,
            TagEnd::Emphasis
                | TagEnd::Strong
                | TagEnd::Strikethrough
                | TagEnd::Link
                | TagEnd::Image
        );
        if !is_inline {
            self.end_block();
        }
    }

    fn inside_backticks(&self, span: Range<usize>) -> Range<usize> {
        let code = self.source.get(span.clone()).unwrap_or_default();
        let backticks = code.len() - code.trim_start_matches('`').len();
        span.start + backticks..span.end - backticks
    }

    fn push(&mut self, text: &str, source: Range<usize>) {
        if self.skipped_depth > 0 || self.is_in_autolink {
            return;
        }
        let start = self.text.len();
        self.text.push_str(text);
        if self.source.get(source.clone()) == Some(text) {
            self.map.push_exact(start, source);
        } else {
            self.map.push_edges(start..self.text.len(), source);
        }
    }

    fn end_block(&mut self) {
        if self.text.len() > self.block_start {
            self.blocks.push(self.block_start..self.text.len());
            self.text.push(BLOCK_SEPARATOR);
            self.block_start = self.text.len();
        }
    }

    fn finish(mut self) -> Prose<'static> {
        self.end_block();
        Prose::new(Cow::Owned(self.text), self.blocks, self.map)
    }
}

#[cfg(test)]
mod tests {
    use antenna_core::{Document, TextFormat};

    use super::*;

    fn blocks_of(text: &str) -> Vec<String> {
        let document = Document::new(text, TextFormat::Markdown).unwrap();

        Prose::for_document(&document)
            .blocks()
            .map(|block| block.text().to_owned())
            .collect()
    }

    #[test]
    fn prose_ends_block_when_paragraph_ends() {
        assert_eq!(blocks_of("One two.\n\nThree."), ["One two.", "Three."]);
    }

    #[test]
    fn prose_ends_block_when_heading_ends() {
        assert_eq!(blocks_of("# Title\nBody text."), ["Title", "Body text."]);
    }

    #[test]
    fn prose_ends_block_when_list_item_ends() {
        assert_eq!(
            blocks_of("- one\n- two\n  - nested"),
            ["one", "two", "nested"]
        );
    }

    #[test]
    fn prose_ends_block_when_block_quote_paragraph_ends() {
        assert_eq!(blocks_of("> one\n>\n> two"), ["one", "two"]);
    }

    #[test]
    fn prose_ends_block_when_table_cell_ends() {
        let table = "| a | b |\n|---|---|\n| c | d |";

        assert_eq!(blocks_of(table), ["a", "b", "c", "d"]);
    }

    #[test]
    fn prose_keeps_inner_text_when_emphasis_strong_or_strikethrough() {
        assert_eq!(blocks_of("a *b* **c** ~~d~~ e"), ["a b c d e"]);
    }

    #[test]
    fn prose_drops_url_when_link() {
        let blocks = blocks_of("see [the page](http://example.com/x) now");

        assert_eq!(blocks, ["see the page now"]);
    }

    #[test]
    fn prose_drops_text_when_autolink() {
        let blocks = blocks_of("go to <http://example.com> or <me@example.com> now");

        assert_eq!(blocks, ["go to  or  now"]);
    }

    #[test]
    fn prose_drops_alt_text_when_image() {
        assert_eq!(blocks_of("a ![alt *text*](pic.png) b"), ["a  b"]);
    }

    #[test]
    fn prose_keeps_code_text_when_inline_code() {
        assert_eq!(blocks_of("run `ls -l` now"), ["run ls -l now"]);
    }

    #[test]
    fn prose_drops_code_when_fenced_code_block() {
        let blocks = blocks_of("one\n\n```rust\nlet x = 1;\n```\n\ntwo");

        assert_eq!(blocks, ["one", "two"]);
    }

    #[test]
    fn prose_drops_code_when_indented_code_block() {
        assert_eq!(blocks_of("one\n\n    let x = 1;\n\ntwo"), ["one", "two"]);
    }

    #[test]
    fn prose_drops_html_when_html_block() {
        assert_eq!(
            blocks_of("one\n\n<div>\nhidden\n</div>\n\ntwo"),
            ["one", "two"]
        );
    }

    #[test]
    fn prose_drops_tag_when_inline_html() {
        assert_eq!(blocks_of("a <b>bold</b> c"), ["a bold c"]);
    }

    #[test]
    fn prose_gives_space_when_soft_or_hard_break() {
        assert_eq!(
            blocks_of("one\ntwo  \nthree\\\nfour"),
            ["one two three four"]
        );
    }

    #[test]
    fn prose_ends_block_when_thematic_break() {
        assert_eq!(blocks_of("one\n***\ntwo"), ["one", "two"]);
    }

    #[test]
    fn prose_maps_to_exact_source_when_text_equals_source() {
        let prose = convert("a &amp; b \\* c");

        assert_eq!(prose.map().source(4..5), 8..9);
        assert_eq!(prose.map().source(6..7), 11..12);
    }

    #[test]
    fn prose_maps_to_piece_edges_when_entity() {
        let prose = convert("a &amp; b \\* c");

        assert_eq!(prose.map().source(2..3), 2..7);
    }

    #[test]
    fn prose_maps_to_exact_source_when_inline_code() {
        let prose = convert("run `ls -l` now");

        assert_eq!(prose.map().source(4..9), 5..10);
    }

    #[test]
    fn prose_maps_to_code_edges_when_inline_code_has_edge_spaces() {
        let prose = convert("run `` ls -l `` now");

        assert_eq!(prose.map().source(4..9), 6..13);
    }

    #[test]
    fn prose_has_block_separators_when_markdown_has_blocks() {
        let document = Document::new("one\n\ntwo", TextFormat::Markdown).unwrap();

        assert_eq!(Prose::for_document(&document).text(), "one\ntwo\n");
    }
}
