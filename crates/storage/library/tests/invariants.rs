//! Invariants of the search and of the grouping, checked with generated documents.

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use antenna_core::{Document, TextFormat};
    use antenna_library::{DocumentId, Filter, Group, Library, fold};
    use jiff::tz::TimeZone;
    use jiff::{SignedDuration, Timestamp, Zoned};
    use proptest::prelude::*;
    use proptest::test_runner::RngSeed;
    use tempfile::TempDir;

    const NOW_SECONDS: i64 = 1_791_374_400;
    const SECONDS_PER_DAY: i64 = 86_400;
    const SPAN_DAYS: i64 = 1_100;
    const SEED: u64 = 0x0a17_e22a;
    const FOLD_CASES: u32 = 4_096;
    const LIBRARY_CASES: u32 = 24;

    fn config(cases: u32) -> ProptestConfig {
        ProptestConfig {
            cases,
            rng_seed: RngSeed::Fixed(SEED),
            failure_persistence: None,
            ..ProptestConfig::default()
        }
    }

    fn now() -> Zoned {
        Timestamp::from_second(NOW_SECONDS)
            .unwrap()
            .to_zoned(TimeZone::UTC)
    }

    fn library_with(documents: &[(String, String, i64)]) -> (TempDir, Library, Vec<DocumentId>) {
        let root = tempfile::tempdir().unwrap();
        let library = Library::open(root.path()).unwrap();
        let ids = documents
            .iter()
            .map(|(title, text, days)| {
                let text = Document::new(format!("{text} text"), TextFormat::Plain).unwrap();
                let modified =
                    now().timestamp() + SignedDuration::from_secs(days * SECONDS_PER_DAY);
                library
                    .create(&format!("{title} title"), &text, modified)
                    .unwrap()
            })
            .collect();
        library.build_search_index().unwrap();
        (root, library, ids)
    }

    fn ids_of(groups: &[Group]) -> Vec<DocumentId> {
        let documents = groups.iter().flat_map(|group| &group.documents);
        documents.map(|summary| summary.meta.id).collect()
    }

    fn document_strategy() -> impl Strategy<Value = Vec<(String, String, i64)>> {
        let word = "[a-zA-Z\u{e9}\u{f1}\u{c9} ]{0,10}";
        let age = -SPAN_DAYS..SPAN_DAYS;
        prop::collection::vec((word, word, age), 1..6)
    }

    proptest! {
        #![proptest_config(config(FOLD_CASES))]

        #[test]
        fn fold_is_idempotent(text in any::<String>()) {
            let once = fold(&text);

            prop_assert_eq!(fold(&once), once);
        }
    }

    proptest! {
        #![proptest_config(config(LIBRARY_CASES))]

        #[test]
        fn more_terms_never_match_more(
            documents in document_strategy(),
            query in "[a-z\u{e9} ]{0,8}",
            extra in "[a-z]{1,3}",
        ) {
            let (_root, library, _ids) = library_with(&documents);
            let wider = library.list(Filter::All, &query, &now());
            let narrower = library.list(Filter::All, &format!("{query} {extra}"), &now());

            let wider_ids: HashSet<_> = ids_of(&wider).into_iter().collect();

            prop_assert!(ids_of(&narrower).iter().all(|id| wider_ids.contains(id)));
        }

        #[test]
        fn each_document_in_one_group(documents in document_strategy()) {
            let (_root, library, ids) = library_with(&documents);

            let groups = library.list(Filter::All, "", &now());

            let mut listed = ids_of(&groups);
            listed.sort();
            let mut expected = ids;
            expected.sort();
            prop_assert_eq!(listed, expected);
            prop_assert!(groups.iter().all(|group| !group.documents.is_empty()));
            let periods: HashSet<_> = groups.iter().map(|group| group.period).collect();
            prop_assert_eq!(periods.len(), groups.len());
        }

        #[test]
        fn grouping_is_deterministic(documents in document_strategy()) {
            let (root, library, _ids) = library_with(&documents);

            let first = library.list(Filter::All, "", &now());
            let second = library.list(Filter::All, "", &now());
            let reopened = Library::open(root.path()).unwrap().list(Filter::All, "", &now());

            prop_assert_eq!(&first, &second);
            prop_assert_eq!(&first, &reopened);
        }
    }
}
