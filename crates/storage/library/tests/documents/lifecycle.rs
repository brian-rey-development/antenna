use std::fs;

use antenna_core::{ExportFormat, Language, Quality, SampleRate};
use antenna_library::{Library, LibraryError};

use super::support::{
    at, complete, document_dir, foreign_id, key, library, markdown, plain, reopen, voice,
};

#[test]
fn document_round_trips_when_created_and_loaded() {
    let (_root, library) = library();
    let document = markdown("# Plan\n\nBuy milk.\n");

    let id = library.create("Weekly plan", &document, at(1_000)).unwrap();
    let (metadata, loaded) = library.load(id).unwrap();

    assert_eq!(loaded, document);
    assert_eq!(metadata.id, id);
    assert_eq!(metadata.title, "Weekly plan");
    assert_eq!(metadata.format, document.format());
    assert_eq!(
        (metadata.created, metadata.modified, metadata.opened),
        (at(1_000), at(1_000), at(1_000))
    );
}

#[test]
fn create_leaves_voice_and_segments_empty_when_document_new() {
    let (_root, library) = library();

    let id = library.create("Notes", &plain("Hello"), at(1_000)).unwrap();
    let metadata = library.load(id).unwrap().0;

    assert!(metadata.voice.is_none());
    assert!(metadata.segments.is_none());
}

#[test]
fn document_persists_when_library_reopened() {
    let (root, library) = library();
    let id = library.create("Notes", &plain("Hello"), at(1_000)).unwrap();

    let reopened = reopen(&root);
    let (metadata, loaded) = reopened.load(id).unwrap();

    assert_eq!(metadata.title, "Notes");
    assert_eq!(loaded, plain("Hello"));
    assert_eq!(reopened.count(), 1);
}

#[test]
fn create_fails_when_title_is_blank() {
    let (_root, library) = library();

    for title in ["", "  ", "\t\n"] {
        let result = library.create(title, &plain("Hello"), at(1_000));

        assert!(matches!(result, Err(LibraryError::EmptyTitle)), "{title:?}");
    }
}

#[test]
fn create_leaves_no_document_when_title_is_blank() {
    let (root, library) = library();

    library.create(" ", &plain("Hello"), at(1_000)).unwrap_err();

    assert_eq!(library.count(), 0);
    assert_eq!(
        fs::read_dir(root.path().join("library")).unwrap().count(),
        0
    );
}

#[test]
fn create_stores_trimmed_title_when_title_has_spaces() {
    let (_root, library) = library();

    let id = library
        .create("  Notes \n", &plain("Hello"), at(1_000))
        .unwrap();

    assert_eq!(library.load(id).unwrap().0.title, "Notes");
}

#[test]
fn load_fails_when_id_unknown() {
    let (_root, library) = library();
    let id = foreign_id();

    let result = library.load(id);

    assert!(matches!(result, Err(LibraryError::NotFound(missing)) if missing == id));
}

#[test]
fn load_fails_when_text_file_is_blank() {
    let (root, library) = library();
    let id = library.create("Notes", &plain("Hello"), at(1_000)).unwrap();
    fs::write(document_dir(&root, id).join("text.txt"), "  \n").unwrap();

    let result = library.load(id);

    assert!(matches!(result, Err(LibraryError::EmptyText { .. })));
}

#[test]
fn load_fails_when_text_file_is_not_utf8() {
    let (root, library) = library();
    let id = library.create("Notes", &plain("Hello"), at(1_000)).unwrap();
    fs::write(document_dir(&root, id).join("text.txt"), [0xff, 0xfe]).unwrap();

    let result = library.load(id);

    assert!(matches!(result, Err(LibraryError::Io { .. })));
}

#[test]
fn save_text_changes_text_and_modified_when_text_differs() {
    let (_root, library) = library();
    let id = library.create("Notes", &plain("Hello"), at(1_000)).unwrap();

    library
        .save_text(id, &plain("Hello again"), at(2_000))
        .unwrap();
    let (metadata, loaded) = library.load(id).unwrap();

    assert_eq!(loaded, plain("Hello again"));
    assert_eq!(
        (metadata.created, metadata.modified),
        (at(1_000), at(2_000))
    );
}

#[test]
fn save_text_keeps_modified_when_text_unchanged() {
    let (_root, library) = library();
    let id = library.create("Notes", &plain("Hello"), at(1_000)).unwrap();

    library.save_text(id, &plain("Hello"), at(2_000)).unwrap();

    assert_eq!(library.load(id).unwrap().0.modified, at(1_000));
}

#[test]
fn save_text_replaces_text_file_when_format_changes() {
    let (root, library) = library();
    let id = library.create("Notes", &plain("Hello"), at(1_000)).unwrap();

    library
        .save_text(id, &markdown("Hello"), at(2_000))
        .unwrap();

    let directory = document_dir(&root, id);
    assert!(directory.join("text.md").is_file());
    assert!(!directory.join("text.txt").exists());
    assert_eq!(library.load(id).unwrap().1, markdown("Hello"));
}

#[test]
fn rename_changes_title_and_modified() {
    let (_root, library) = library();
    let id = library.create("Notes", &plain("Hello"), at(1_000)).unwrap();

    library.rename(id, " Ideas ", at(2_000)).unwrap();
    let metadata = library.load(id).unwrap().0;

    assert_eq!(
        (metadata.title.as_str(), metadata.modified),
        ("Ideas", at(2_000))
    );
}

#[test]
fn rename_fails_when_title_is_blank() {
    let (_root, library) = library();
    let id = library.create("Notes", &plain("Hello"), at(1_000)).unwrap();

    let result = library.rename(id, "   ", at(2_000));

    assert!(matches!(result, Err(LibraryError::EmptyTitle)));
    assert_eq!(library.load(id).unwrap().0.title, "Notes");
}

#[test]
fn set_voice_clears_segments_when_voice_changes() {
    let (_root, library) = library();
    let document = plain("Hello");
    let id = library.create("Notes", &document, at(1_000)).unwrap();
    complete(
        &library,
        id,
        &document,
        &voice("en-alba", Quality::Balanced),
    );

    library
        .set_voice(id, voice("en-bruno", Quality::Balanced), at(3_000))
        .unwrap();
    let metadata = library.load(id).unwrap().0;

    assert!(metadata.segments.is_none());
    assert_eq!(metadata.voice, Some(voice("en-bruno", Quality::Balanced)));
    assert_eq!(metadata.modified, at(3_000));
}

#[test]
fn set_voice_keeps_segments_when_voice_same() {
    let (_root, library) = library();
    let document = plain("Hello");
    let id = library.create("Notes", &document, at(1_000)).unwrap();
    complete(
        &library,
        id,
        &document,
        &voice("en-alba", Quality::Balanced),
    );

    let modified = library.load(id).unwrap().0.modified;

    library
        .set_voice(id, voice("en-alba", Quality::Balanced), at(3_000))
        .unwrap();
    let metadata = library.load(id).unwrap().0;

    assert!(metadata.segments.is_some());
    assert_eq!(metadata.modified, modified);
}

#[test]
fn set_language_persists_when_library_reopened() {
    let (root, library) = library();
    let id = library.create("Notes", &plain("Hola"), at(1_000)).unwrap();

    library.set_language(id, Some(Language::Es)).unwrap();
    let reopened = reopen(&root);

    assert_eq!(reopened.load(id).unwrap().0.language, Some(Language::Es));
}

#[test]
fn record_export_persists_when_library_reopened() {
    let (root, library) = library();
    let id = library.create("Notes", &plain("Hola"), at(1_000)).unwrap();

    library
        .record_export(id, ExportFormat::Ogg, at(5_000))
        .unwrap();
    let record = reopen(&root)
        .load(id)
        .unwrap()
        .0
        .last_export
        .clone()
        .unwrap();

    assert_eq!((record.format, record.at), (ExportFormat::Ogg, at(5_000)));
}

#[test]
fn delete_removes_document_when_id_exists() {
    let (root, library) = library();
    let id = library.create("Notes", &plain("Hello"), at(1_000)).unwrap();
    let mut writer = library
        .segments()
        .writer(key('c'), SampleRate::HZ_24000)
        .unwrap();
    writer.write(&[0.5; 8]).unwrap();
    writer.commit().unwrap();

    library.delete(id).unwrap();

    assert!(!document_dir(&root, id).exists());
    assert_eq!(library.count(), 0);
    assert!(matches!(library.load(id), Err(LibraryError::NotFound(_))));
    assert!(library.segments().contains(&key('c')));
    assert_eq!(reopen(&root).count(), 0);
}

#[test]
fn delete_fails_when_id_unknown() {
    let (_root, library) = library();
    let id = foreign_id();

    let result = library.delete(id);

    assert!(matches!(result, Err(LibraryError::NotFound(_))));
}

#[test]
fn library_is_send_and_sync() {
    fn assert_send_and_sync<T: Send + Sync>() {}

    assert_send_and_sync::<Library>();
}
