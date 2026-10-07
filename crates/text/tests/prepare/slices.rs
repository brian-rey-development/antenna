use antenna_core::{Language, TextFormat};

use crate::support::{FIXTURES, MARKDOWN, alphanumerics, collapse, segments, slice};

#[test]
fn slice_equals_segment_when_plain_text() {
    let documents = FIXTURES.into_iter().chain([(Language::En, MARKDOWN)]);
    for (language, text) in documents {
        for segment in segments(text, TextFormat::Plain, language) {
            assert_eq!(
                collapse(slice(text, &segment)),
                segment.text(),
                "{language}"
            );
        }
    }
}

#[test]
fn slice_contains_segment_letters_when_markdown() {
    let documents = FIXTURES.into_iter().chain([(Language::En, MARKDOWN)]);
    for (language, text) in documents {
        for segment in segments(text, TextFormat::Markdown, language) {
            let mut slice_letters = alphanumerics(slice(text, &segment)).into_iter();
            let has_all = alphanumerics(segment.text())
                .into_iter()
                .all(|letter| slice_letters.any(|candidate| candidate == letter));
            assert!(has_all, "{language} segment {}", segment.index());
        }
    }
}
