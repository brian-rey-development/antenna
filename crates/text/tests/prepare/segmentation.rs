use std::num::NonZeroUsize;

use antenna_core::{Document, Language, Segment, SegmentIndex, TextFormat};
use antenna_text::{FIRST_SEGMENT_CHARS, SegmentLimits, TextError, prepare};

use crate::support::{ABBREVIATIONS, limits, segments, texts};

#[test]
fn segmenter_keeps_abbreviation_when_name_follows() {
    for (language, lines) in ABBREVIATIONS {
        for line in lines.lines() {
            let (count, text) = line.split_once('\t').unwrap();

            let found = segments(text, TextFormat::Plain, language).len();

            assert_eq!(found, count.parse::<usize>().unwrap(), "{language}: {text}");
        }
    }
}

#[test]
fn segmenter_keeps_decimal_when_digits_follow() {
    for (language, text) in [
        (Language::En, "Pi is about 3.14 today."),
        (Language::En, "The value is 3,14 or 3.14 here."),
        (Language::Es, "Pi vale 3,14 hoy."),
        (Language::De, "Pi beträgt 3,14 heute."),
    ] {
        assert_eq!(
            segments(text, TextFormat::Plain, language).len(),
            1,
            "{text}"
        );
    }
}

#[test]
fn prepare_isolates_heading_when_paragraph_follows() {
    let found = segments(
        "# Title\nA sentence follows here.",
        TextFormat::Markdown,
        Language::En,
    );

    assert_eq!(texts(&found), ["Title", "A sentence follows here."]);
}

#[test]
fn prepare_splits_first_sentence_when_clause_in_range() {
    let text = "The quick brown fox jumps over the lazy dog, and then it runs away from the farm.";

    let found = segments(text, TextFormat::Plain, Language::En);

    assert_eq!(found.len(), 2);
    assert!(found[0].text().chars().count() <= FIRST_SEGMENT_CHARS);
    assert_eq!(
        found[0].text(),
        "The quick brown fox jumps over the lazy dog,"
    );
}

#[test]
fn prepare_fails_when_only_code_block() {
    let document = Document::new("```\nlet x = 1;\n```\n", TextFormat::Markdown).unwrap();

    let result = prepare(&document, Language::En, limits());

    assert!(matches!(result, Err(TextError::NoSpeech)));
}

#[test]
fn prepare_drops_sentence_when_no_letter_or_digit() {
    let found = segments("First. ... * * * Last.", TextFormat::Plain, Language::En);

    assert_eq!(texts(&found), ["First.", "* * * Last."]);
    assert_eq!(found[0].index(), SegmentIndex::new(0));
    assert_eq!(found[1].index(), SegmentIndex::new(1));
}

#[test]
fn prepare_drops_block_when_no_letter_or_digit() {
    let found = segments("First.\n\n* * *\n\nLast.", TextFormat::Plain, Language::En);

    assert_eq!(texts(&found), ["First.", "Last."]);
    assert_eq!(found[1].index(), SegmentIndex::new(1));
}

#[test]
fn prepare_collapses_whitespace_when_sentence_has_runs() {
    let found = segments("One \t two\n three.", TextFormat::Plain, Language::En);

    assert_eq!(texts(&found), ["One two three."]);
}

#[test]
fn prepare_keeps_source_range_when_whitespace_is_collapsed() {
    let found = segments("One \t two\n three.", TextFormat::Plain, Language::En);

    assert_eq!(found[0].source(), 0..17);
}

#[test]
fn prepare_decodes_entity_when_markdown_has_entity() {
    let found = segments("Fish &amp; chips.", TextFormat::Markdown, Language::En);

    assert_eq!(texts(&found), ["Fish & chips."]);
}

#[test]
fn prepare_maps_to_whole_entity_when_markdown_has_entity() {
    let found = segments("Fish &amp; chips.", TextFormat::Markdown, Language::En);

    assert_eq!(found[0].source(), 0..17);
}

#[test]
fn prepare_splits_sentences_when_plain_text_has_carriage_returns() {
    let found = segments(
        "One two.\r\nThree four.\r\nFive.",
        TextFormat::Plain,
        Language::En,
    );

    assert_eq!(texts(&found), ["One two.", "Three four.", "Five."]);
}

#[test]
fn prepare_splits_sentences_when_line_ends_with_space() {
    let found = segments(
        "One two. \nThree four.\nFive.",
        TextFormat::Plain,
        Language::En,
    );

    assert_eq!(texts(&found), ["One two.", "Three four.", "Five."]);
}

#[test]
fn prepare_maps_each_sentence_when_code_span_holds_two_sentences() {
    let found = segments("Use `One. Two` now.", TextFormat::Markdown, Language::En);

    assert_eq!(texts(&found), ["Use One.", "Two now."]);
    assert_eq!(found[0].source(), 0..9);
    assert_eq!(found[1].source(), 10..19);
}

#[test]
fn prepare_maps_each_sentence_when_code_span_holds_three_sentences() {
    let found = segments("`One. Two. Three`", TextFormat::Markdown, Language::En);

    assert_eq!(texts(&found), ["One.", "Two.", "Three"]);
    let ranges: Vec<_> = found.iter().map(Segment::source).collect();
    assert_eq!(ranges, [1..5, 6..10, 11..16]);
}

#[test]
fn prepare_maps_each_sentence_when_code_span_starts_mid_sentence() {
    let found = segments("Use `a. B. C` end.", TextFormat::Markdown, Language::En);

    assert_eq!(texts(&found), ["Use a. B.", "C end."]);
    let ranges: Vec<_> = found.iter().map(Segment::source).collect();
    assert_eq!(ranges, [0..10, 11..18]);
}

#[test]
fn prepare_maps_each_sentence_when_code_span_has_long_backtick_fence() {
    let found = segments("``One. `Two` Three``", TextFormat::Markdown, Language::En);

    let ranges: Vec<_> = found.iter().map(Segment::source).collect();
    assert_eq!(ranges, [2..6, 7..18]);
}

#[test]
fn prepare_gives_empty_range_when_code_span_with_edge_spaces_holds_two_sentences() {
    let found = segments("`` One. Two ``", TextFormat::Markdown, Language::En);

    assert_eq!(texts(&found), ["One.", "Two"]);
    assert_eq!(found[0].source(), 2..12);
    assert!(found[1].source().is_empty());
}

#[test]
fn prepare_splits_sentence_when_longer_than_max_chars() {
    let document =
        Document::new("aaaa bbbb cccc, dddd eeee ffff gggg.", TextFormat::Plain).unwrap();
    let small = SegmentLimits::new(NonZeroUsize::new(20).unwrap());

    let found = prepare(&document, Language::En, small).unwrap();

    assert_eq!(texts(&found), ["aaaa bbbb cccc,", "dddd eeee ffff gggg."]);
}
