use std::fmt::{self, Display, Formatter};
use std::ops::Range;
use std::sync::Arc;

use strum::{Display, EnumString, IntoStaticStr};

use crate::CoreError;

const MARKDOWN_EXTENSIONS: [&str; 2] = ["md", "markdown"];

/// The format of the text of a document. The text forms are "plain" and "markdown".
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Display, EnumString, IntoStaticStr)]
#[strum(serialize_all = "lowercase")]
pub enum TextFormat {
    /// Text with no markup.
    Plain,
    /// Markdown text.
    Markdown,
}

impl TextFormat {
    /// Returns the format of a file from its extension, with no dot. The comparison ignores the
    /// ASCII case. The extensions "md" and "markdown" give `Markdown`, all others give `Plain`.
    pub fn from_extension(extension: &str) -> Self {
        let is_markdown = MARKDOWN_EXTENSIONS
            .iter()
            .any(|markdown| markdown.eq_ignore_ascii_case(extension));
        if is_markdown {
            Self::Markdown
        } else {
            Self::Plain
        }
    }
}

/// The complete text that the user gives to Antenna. The text is never blank.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Document {
    text: Arc<str>,
    format: TextFormat,
}

impl Document {
    /// Makes a document from its text and format.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::EmptyDocument`] if the text is empty or contains only whitespace.
    pub fn new(text: impl Into<Arc<str>>, format: TextFormat) -> Result<Self, CoreError> {
        let text = text.into();
        if text.trim().is_empty() {
            return Err(CoreError::EmptyDocument);
        }
        Ok(Self { text, format })
    }

    /// Returns the text.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Returns the format of the text.
    pub fn format(&self) -> TextFormat {
        self.format
    }
}

/// The position of a segment in its document. The first segment has the index 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SegmentIndex(u32);

impl SegmentIndex {
    /// Makes a segment index.
    pub const fn new(index: u32) -> Self {
        Self(index)
    }

    /// Returns the index as a number.
    pub const fn get(self) -> u32 {
        self.0
    }
}

impl From<SegmentIndex> for usize {
    fn from(index: SegmentIndex) -> Self {
        index.0 as Self
    }
}

impl Display for SegmentIndex {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, formatter)
    }
}

/// A part of a document that an engine synthesizes in one call.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Segment {
    index: SegmentIndex,
    text: Box<str>,
    source: Range<usize>,
}

impl Segment {
    /// Makes a segment from its index, the text to speak and its source range in the document.
    pub fn new(index: SegmentIndex, text: impl Into<Box<str>>, source: Range<usize>) -> Self {
        Self {
            index,
            text: text.into(),
            source,
        }
    }

    /// Returns the position of the segment in its document.
    pub fn index(&self) -> SegmentIndex {
        self.index
    }

    /// Returns the text that the engine speaks.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Returns the byte range of the segment in the text of its document.
    pub fn source(&self) -> Range<usize> {
        self.source.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn document_rejects_text_when_blank() {
        for text in ["", " ", "\n\t \r\n"] {
            let result = Document::new(text, TextFormat::Plain);

            assert_eq!(result, Err(CoreError::EmptyDocument), "text {text:?}");
        }
    }

    #[test]
    fn document_keeps_text_and_format_when_not_blank() {
        let document = Document::new(" # Title\n", TextFormat::Markdown).unwrap();

        assert_eq!(document.text(), " # Title\n");
        assert_eq!(document.format(), TextFormat::Markdown);
    }

    #[test]
    fn text_format_is_markdown_when_extension_is_md() {
        assert_eq!(TextFormat::from_extension("md"), TextFormat::Markdown);
        assert_eq!(TextFormat::from_extension("markdown"), TextFormat::Markdown);
        assert_eq!(TextFormat::from_extension("MD"), TextFormat::Markdown);
    }

    #[test]
    fn text_format_is_plain_when_extension_is_not_markdown() {
        for extension in ["txt", "", "mdx", ".md"] {
            assert_eq!(TextFormat::from_extension(extension), TextFormat::Plain);
        }
    }

    #[test]
    fn text_format_round_trips_when_text_parsed() {
        let formats = [TextFormat::Plain, TextFormat::Markdown];
        let texts = formats.map(<&str>::from);

        let parsed = texts.map(|text| text.parse::<TextFormat>().unwrap());

        assert_eq!(texts, ["plain", "markdown"]);
        assert_eq!(parsed, formats);
        assert_eq!(TextFormat::Markdown.to_string(), "markdown");
    }

    #[test]
    fn segment_index_converts_to_usize_and_text() {
        let index = SegmentIndex::new(7);

        assert_eq!(usize::from(index), 7);
        assert_eq!(index.to_string(), "7");
        assert_eq!(index.get(), 7);
    }

    #[test]
    fn segment_keeps_index_text_and_source() {
        let segment = Segment::new(SegmentIndex::new(2), "Hola.", 10..15);

        assert_eq!(segment.index(), SegmentIndex::new(2));
        assert_eq!(segment.text(), "Hola.");
        assert_eq!(segment.source(), 10..15);
    }
}
