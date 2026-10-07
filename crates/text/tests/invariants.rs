//! Invariants of `prepare` over generated documents.

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use antenna_core::{Document, Language, Segment, SegmentIndex, TextFormat};
    use antenna_text::{SegmentLimits, TextError, prepare};
    use proptest::prelude::*;
    use proptest::test_runner::{Config, RngAlgorithm, RngSeed};

    const CASES: u32 = 512;
    const SEED: u64 = 0x0A17_E22A;
    const MAX_CHARS_LIMIT: usize = 150;
    const TEXT_PATTERN: &str = concat!(
        "[A-Za-z0-9 .,;:!?\\n\\r\\t()'\"#*_`&<>\\[\\]|~\\\\-",
        "\\x{e9}\\x{a0}\\x{2013}\\x{2014}\\x{4e2d}\\x{1f600}]{1,400}"
    );

    fn config() -> Config {
        Config {
            cases: CASES,
            rng_seed: RngSeed::Fixed(SEED),
            rng_algorithm: RngAlgorithm::ChaCha,
            failure_persistence: None,
            ..Config::default()
        }
    }

    fn format() -> impl Strategy<Value = TextFormat> {
        prop_oneof![Just(TextFormat::Plain), Just(TextFormat::Markdown)]
    }

    fn language() -> impl Strategy<Value = Language> {
        prop::sample::select(Language::ALL.to_vec())
    }

    fn limits() -> impl Strategy<Value = SegmentLimits> {
        (1..=MAX_CHARS_LIMIT)
            .prop_map(|max_chars| SegmentLimits::new(NonZeroUsize::new(max_chars).unwrap()))
    }

    fn prepared(
        text: &str,
        format: TextFormat,
        language: Language,
        limits: SegmentLimits,
    ) -> Result<Vec<Segment>, TestCaseError> {
        prop_assume!(!text.trim().is_empty());
        let document =
            Document::new(text, format).map_err(|error| TestCaseError::fail(error.to_string()))?;
        match prepare(&document, language, limits) {
            Ok(segments) => Ok(segments),
            Err(TextError::NoSpeech) => Ok(Vec::new()),
            Err(error) => Err(TestCaseError::fail(error.to_string())),
        }
    }

    proptest! {
        #![proptest_config(config())]

        #[test]
        fn segments_respect_max_chars(
            text in TEXT_PATTERN, format in format(), language in language(), limits in limits(),
        ) {
            for segment in prepared(&text, format, language, limits)? {
                prop_assert!(segment.text().chars().count() <= limits.max_chars().get());
            }
        }

        #[test]
        fn source_ranges_are_ordered_and_valid(
            text in TEXT_PATTERN, format in format(), language in language(), limits in limits(),
        ) {
            let mut previous_end = 0;
            for segment in prepared(&text, format, language, limits)? {
                let source = segment.source();
                prop_assert!(source.start >= previous_end);
                prop_assert!(source.start <= source.end);
                prop_assert!(source.end <= text.len());
                prop_assert!(text.is_char_boundary(source.start));
                prop_assert!(text.is_char_boundary(source.end));
                previous_end = source.end;
            }
        }

        #[test]
        fn segments_keep_all_alphanumeric_chars_when_plain_text(
            text in TEXT_PATTERN, language in language(), limits in limits(),
        ) {
            let segments = prepared(&text, TextFormat::Plain, language, limits)?;

            let kept: Vec<char> = segments.iter().flat_map(|segment| segment.text().chars()).filter(|character| character.is_alphanumeric()).collect();
            let all: Vec<char> = text.chars().filter(|character| character.is_alphanumeric()).collect();
            prop_assert_eq!(kept, all);
        }

        #[test]
        fn prepare_is_deterministic(
            text in TEXT_PATTERN, format in format(), language in language(), limits in limits(),
        ) {
            let first = prepared(&text, format, language, limits)?;
            let second = prepared(&text, format, language, limits)?;

            prop_assert_eq!(first, second);
        }

        #[test]
        fn segment_indexes_are_sequential(
            text in TEXT_PATTERN, format in format(), language in language(), limits in limits(),
        ) {
            let segments = prepared(&text, format, language, limits)?;

            for (position, segment) in segments.iter().enumerate() {
                prop_assert_eq!(segment.index(), SegmentIndex::new(u32::try_from(position).unwrap()));
            }
        }

        #[test]
        fn slice_equals_segment_when_plain_text(
            text in TEXT_PATTERN, language in language(), limits in limits(),
        ) {
            for segment in prepared(&text, TextFormat::Plain, language, limits)? {
                let slice = text.get(segment.source()).unwrap();

                prop_assert_eq!(slice.split_whitespace().collect::<Vec<_>>().join(" "), segment.text());
            }
        }

        #[test]
        fn segments_stay_in_block_when_plain_text(
            text in TEXT_PATTERN, language in language(), limits in limits(),
        ) {
            for segment in prepared(&text, TextFormat::Plain, language, limits)? {
                let slice = text.get(segment.source()).unwrap();

                prop_assert!(slice.lines().all(|line| !line.trim().is_empty()));
            }
        }
    }
}
