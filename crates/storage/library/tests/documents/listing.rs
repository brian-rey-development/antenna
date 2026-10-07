use std::fs;

use antenna_core::{ExportFormat, Quality};
use antenna_library::{Filter, Library, LibraryError, Period, RECENT_LIMIT};

use super::support::{
    at, complete, create_all, document_dir, library, now, plain, reopen, sorted_titles, titles,
    utc, voice,
};

fn found(library: &Library, query: &str) -> Vec<String> {
    titles(&library.list(Filter::All, query, &now()))
}

#[test]
fn search_matches_when_accents_differ() {
    let (_root, library) = library();
    library
        .create("Canción de cuna", &plain("Duerme"), at(100))
        .unwrap();
    library
        .create("Weekly plan", &plain("Buy milk"), at(200))
        .unwrap();

    assert_eq!(found(&library, "CANCION"), ["Canción de cuna"]);
    assert_eq!(found(&library, "cancion de"), ["Canción de cuna"]);
    assert_eq!(found(&library, "weekly xyz"), Vec::<String>::new());
}

#[test]
fn search_matches_titles_only_when_index_not_built() {
    let (root, library) = library();
    library
        .create("Plan", &plain("Compra leche"), at(100))
        .unwrap();
    let reopened = reopen(&root);

    assert_eq!(found(&reopened, "leche"), Vec::<String>::new());
    assert_eq!(found(&reopened, "plan"), ["Plan"]);
}

#[test]
fn search_matches_text_when_index_built() {
    let (root, library) = library();
    library
        .create("Plan", &plain("Compra LECHE y pan"), at(100))
        .unwrap();
    library.create("Other", &plain("Nada"), at(200)).unwrap();
    let reopened = reopen(&root);

    reopened.build_search_index().unwrap();

    assert_eq!(found(&reopened, "leche pan"), ["Plan"]);
    assert_eq!(found(&reopened, "plan nada"), Vec::<String>::new());
}

#[test]
fn search_matches_new_text_when_text_saved() {
    let (root, library) = library();
    let id = library
        .create("Plan", &plain("Compra leche"), at(100))
        .unwrap();
    let reopened = reopen(&root);
    reopened.build_search_index().unwrap();

    reopened
        .save_text(id, &plain("Compra caf\u{e9}"), at(200))
        .unwrap();

    assert_eq!(found(&reopened, "cafe"), ["Plan"]);
    assert_eq!(found(&reopened, "leche"), Vec::<String>::new());
}

#[test]
fn build_search_index_fails_when_text_file_missing() {
    let (root, library) = library();
    let id = library
        .create("Plan", &plain("Compra leche"), at(100))
        .unwrap();
    let reopened = reopen(&root);
    fs::remove_file(document_dir(&root, id).join("text.txt")).unwrap();

    let result = reopened.build_search_index();

    assert!(matches!(result, Err(LibraryError::Io { .. })));
}

#[test]
fn build_search_index_indexes_other_documents_when_one_text_file_missing() {
    let (root, library) = library();
    let broken = library
        .create("Broken", &plain("Compra leche"), at(100))
        .unwrap();
    library
        .create("Fine", &plain("Compra leche"), at(200))
        .unwrap();
    let reopened = reopen(&root);
    fs::remove_file(document_dir(&root, broken).join("text.txt")).unwrap();

    reopened.build_search_index().unwrap_err();

    assert_eq!(found(&reopened, "leche"), ["Fine"]);
}

#[test]
fn search_matches_text_when_query_has_accents() {
    let (root, library) = library();
    library
        .create("Plan", &plain("Compra leche y caf\u{e9}"), at(100))
        .unwrap();
    let reopened = reopen(&root);
    reopened.build_search_index().unwrap();

    let titles = found(&reopened, "CAF\u{c9}");

    assert_eq!(titles, ["Plan"]);
}

#[test]
fn filter_keeps_matching_statuses() {
    let (_root, library) = library();
    let alba = voice("en-alba", Quality::Balanced);
    let [_, running, ready, exported] = ["Draft", "Running", "Ready", "Exported"]
        .map(|title| library.create(title, &plain(title), at(100)).unwrap());
    for (id, title) in [
        (running, "Running"),
        (ready, "Ready"),
        (exported, "Exported"),
    ] {
        complete(&library, id, &plain(title), &alba);
    }
    library.set_progress(running, Some((1, 2)));
    library
        .record_export(exported, ExportFormat::Wav, at(300))
        .unwrap();

    let all = ["Draft", "Exported", "Ready", "Running"];
    assert_eq!(sorted_titles(&library, Filter::All), all);
    assert_eq!(
        sorted_titles(&library, Filter::Ready),
        ["Exported", "Ready"]
    );
    assert_eq!(sorted_titles(&library, Filter::Drafts), ["Draft"]);
}

#[test]
fn groups_follow_period_rules() {
    let (_root, library) = library();
    let documents = [
        ("today", utc(2026, 10, 7)),
        ("monday", utc(2026, 10, 5)),
        ("tuesday", utc(2026, 10, 6)),
        ("early october", utc(2026, 10, 2)),
        ("september", utc(2026, 9, 15)),
        ("last year", utc(2025, 10, 1)),
    ];
    create_all(&library, &documents);

    let groups = library.list(Filter::All, "", &now());

    let month = |year, month| Period::Month { year, month };
    let periods = [
        Period::Today,
        Period::ThisWeek,
        month(2026, 10),
        month(2026, 9),
        month(2025, 10),
    ];
    assert_eq!(
        groups.iter().map(|group| group.period).collect::<Vec<_>>(),
        periods
    );
    assert_eq!(titles(&groups[1..2]), ["tuesday", "monday"]);
}

#[test]
fn recent_returns_four_newest_opened() {
    let (_root, library) = library();
    let ids: Vec<_> = (0..6)
        .map(|index| {
            library
                .create(&format!("Doc {index}"), &plain("Hello"), at(100 + index))
                .unwrap()
        })
        .collect();
    for (id, opened) in ids.iter().zip([500, 100, 900, 300, 700, 200]) {
        library.mark_opened(*id, at(opened)).unwrap();
    }

    let recent = library.recent();

    let order: Vec<_> = recent.iter().map(|summary| summary.meta.id).collect();
    assert_eq!(order.len(), RECENT_LIMIT);
    assert_eq!(order, [ids[2], ids[4], ids[0], ids[3]]);
}
