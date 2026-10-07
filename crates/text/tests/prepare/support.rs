use std::num::NonZeroUsize;

use antenna_core::{Document, Language, Segment, TextFormat};
use antenna_text::{SegmentLimits, prepare};

pub(crate) const MAX_CHARS: usize = 400;
const MARKDOWN: &str = include_str!("../fixtures/markdown.md");
const FIXTURES: [(Language, &str); 6] = [
    (Language::En, include_str!("../fixtures/en.md")),
    (Language::Es, include_str!("../fixtures/es.md")),
    (Language::Pt, include_str!("../fixtures/pt.md")),
    (Language::Fr, include_str!("../fixtures/fr.md")),
    (Language::It, include_str!("../fixtures/it.md")),
    (Language::De, include_str!("../fixtures/de.md")),
];
pub(crate) const ABBREVIATIONS: [(Language, &str); 6] = [
    (
        Language::En,
        include_str!("../fixtures/abbreviations/en.txt"),
    ),
    (
        Language::Es,
        include_str!("../fixtures/abbreviations/es.txt"),
    ),
    (
        Language::Pt,
        include_str!("../fixtures/abbreviations/pt.txt"),
    ),
    (
        Language::Fr,
        include_str!("../fixtures/abbreviations/fr.txt"),
    ),
    (
        Language::It,
        include_str!("../fixtures/abbreviations/it.txt"),
    ),
    (
        Language::De,
        include_str!("../fixtures/abbreviations/de.txt"),
    ),
];

/// A checkout on Windows can change the line ends of a fixture, and the source ranges of the
/// snapshots count bytes.
fn line_feeds(text: &str) -> String {
    text.replace("\r\n", "\n")
}

pub(crate) fn fixtures() -> Vec<(Language, String)> {
    FIXTURES
        .iter()
        .map(|(language, text)| (*language, line_feeds(text)))
        .collect()
}

pub(crate) fn fixture(language: Language) -> String {
    line_feeds(
        FIXTURES
            .iter()
            .find(|(candidate, _)| *candidate == language)
            .unwrap()
            .1,
    )
}

pub(crate) fn markdown() -> String {
    line_feeds(MARKDOWN)
}

pub(crate) fn limits() -> SegmentLimits {
    SegmentLimits::new(NonZeroUsize::new(MAX_CHARS).unwrap())
}

pub(crate) fn segments(text: &str, format: TextFormat, language: Language) -> Vec<Segment> {
    let document = Document::new(text, format).unwrap();

    prepare(&document, language, limits()).unwrap()
}

pub(crate) fn texts(segments: &[Segment]) -> Vec<&str> {
    segments.iter().map(Segment::text).collect()
}

pub(crate) fn slice<'a>(text: &'a str, segment: &Segment) -> &'a str {
    text.get(segment.source()).unwrap()
}

pub(crate) fn snapshot(text: &str, language: Language) -> String {
    let lines = segments(text, TextFormat::Markdown, language)
        .into_iter()
        .map(|segment| {
            let source = slice(text, &segment);
            format!(
                "{}: {:?}\n  {:?} {source:?}",
                segment.index(),
                segment.text(),
                segment.source()
            )
        });
    lines.collect::<Vec<_>>().join("\n")
}

pub(crate) fn alphanumerics(text: &str) -> Vec<char> {
    text.chars()
        .filter(|character| character.is_alphanumeric())
        .collect()
}

pub(crate) fn collapse(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}
