use std::time::Duration;

use antenna_core::{ExportFormat, Quality};
use antenna_library::{Status, StoredVoice, text_hash};

use super::support::{at, complete, foreign_id, key, library, plain, reopen, status_of, voice};

fn alba() -> StoredVoice {
    voice("en-alba", Quality::Balanced)
}

#[test]
fn record_segments_keeps_keys_until_record_complete() {
    let (_root, library) = library();
    let document = plain("Hello");
    let id = library.create("Notes", &document, at(100)).unwrap();
    let hash = text_hash(&document);
    library.set_voice(id, alba(), at(200)).unwrap();

    library
        .record_segments(id, hash, &alba(), vec![key('a')])
        .unwrap();
    let before = status_of(&library, id);
    library
        .record_complete(id, hash, &alba(), Duration::from_secs(3))
        .unwrap();

    let list = library.load(id).unwrap().0.segments.clone().unwrap();
    assert_eq!(before, Status::Draft);
    assert_eq!(status_of(&library, id), Status::Ready);
    assert_eq!(
        (list.keys, list.duration),
        (vec![key('a')], Duration::from_secs(3))
    );
}

#[test]
fn record_segments_ignored_when_voice_differs() {
    let (_root, library) = library();
    let document = plain("Hello");
    let id = library.create("Notes", &document, at(100)).unwrap();
    library.set_voice(id, alba(), at(200)).unwrap();

    let other = voice("en-bruno", Quality::Balanced);
    library
        .record_segments(id, text_hash(&document), &other, vec![key('a')])
        .unwrap();

    assert!(library.load(id).unwrap().0.segments.is_none());
}

#[test]
fn record_segments_ignored_when_text_hash_differs() {
    let (_root, library) = library();
    let id = library.create("Notes", &plain("Hello"), at(100)).unwrap();
    library.set_voice(id, alba(), at(200)).unwrap();

    let old_hash = text_hash(&plain("Hell"));
    library
        .record_segments(id, old_hash, &alba(), vec![key('a')])
        .unwrap();

    assert!(library.load(id).unwrap().0.segments.is_none());
}

#[test]
fn record_complete_ignored_when_text_hash_differs() {
    let (_root, library) = library();
    let id = library.create("Notes", &plain("Hello"), at(100)).unwrap();
    let old_hash = text_hash(&plain("Hello"));
    library.set_voice(id, alba(), at(200)).unwrap();
    library
        .record_segments(id, old_hash, &alba(), vec![key('a')])
        .unwrap();
    library
        .save_text(id, &plain("Hello again"), at(300))
        .unwrap();

    library
        .record_complete(id, old_hash, &alba(), Duration::from_secs(3))
        .unwrap();

    assert_eq!(status_of(&library, id), Status::Draft);
    assert!(
        !library
            .load(id)
            .unwrap()
            .0
            .segments
            .clone()
            .unwrap()
            .is_complete
    );
}

#[test]
fn record_complete_ignored_when_voice_differs() {
    let (_root, library) = library();
    let document = plain("Hello");
    let id = library.create("Notes", &document, at(100)).unwrap();
    let hash = text_hash(&document);
    library.set_voice(id, alba(), at(200)).unwrap();
    library
        .record_segments(id, hash, &alba(), vec![key('a')])
        .unwrap();

    let other = voice("en-alba", Quality::Max);
    library
        .record_complete(id, hash, &other, Duration::from_secs(3))
        .unwrap();

    assert!(
        !library
            .load(id)
            .unwrap()
            .0
            .segments
            .clone()
            .unwrap()
            .is_complete
    );
}

#[test]
fn record_complete_ignored_when_segments_not_recorded() {
    let (_root, library) = library();
    let document = plain("Hello");
    let id = library.create("Notes", &document, at(100)).unwrap();
    library.set_voice(id, alba(), at(200)).unwrap();

    library
        .record_complete(id, text_hash(&document), &alba(), Duration::from_secs(3))
        .unwrap();

    assert!(library.load(id).unwrap().0.segments.is_none());
}

#[test]
fn status_leaves_exported_when_text_saved() {
    let (_root, library) = library();
    let document = plain("Hello");
    let id = library.create("Notes", &document, at(100)).unwrap();
    complete(&library, id, &document, &alba());
    library
        .record_export(id, ExportFormat::Mp3, at(300))
        .unwrap();
    let before = status_of(&library, id);

    library
        .save_text(id, &plain("Hello again"), at(400))
        .unwrap();

    assert_eq!(before, Status::Exported(ExportFormat::Mp3));
    assert_eq!(status_of(&library, id), Status::Draft);
}

#[test]
fn progress_changes_status_until_cleared() {
    let (_root, library) = library();
    let document = plain("Hello");
    let id = library.create("Notes", &document, at(100)).unwrap();
    complete(&library, id, &document, &alba());

    library.set_progress(id, Some((1, 4)));
    let running = status_of(&library, id);
    library.set_progress(id, None);

    assert_eq!(running, Status::Generating { done: 1, total: 4 });
    assert_eq!(status_of(&library, id), Status::Ready);
}

#[test]
fn progress_absent_after_reopen() {
    let (root, library) = library();
    let document = plain("Hello");
    let id = library.create("Notes", &document, at(100)).unwrap();
    complete(&library, id, &document, &alba());
    library.set_progress(id, Some((1, 4)));

    let reopened = reopen(&root);

    assert_eq!(status_of(&reopened, id), Status::Ready);
}

#[test]
fn set_progress_does_nothing_when_id_unknown() {
    let (_root, library) = library();
    let id = foreign_id();

    library.set_progress(id, Some((1, 2)));

    assert_eq!(library.count(), 0);
}
