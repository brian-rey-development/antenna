use std::path::PathBuf;
use std::time::Duration;

use antenna_core::{Document, EngineId, Quality, TextFormat, VoiceId};
use antenna_library::{
    DocumentId, Filter, Group, Library, SegmentKey, Status, StoredVoice, text_hash,
};
use jiff::civil;
use jiff::tz::TimeZone;
use jiff::{Timestamp, Zoned};
use tempfile::TempDir;

pub(crate) fn at(seconds: i64) -> Timestamp {
    Timestamp::from_second(seconds).unwrap()
}

pub(crate) fn utc(year: i16, month: i8, day: i8) -> Timestamp {
    let date_time = civil::date(year, month, day).at(12, 0, 0, 0);
    date_time.to_zoned(TimeZone::UTC).unwrap().timestamp()
}

pub(crate) fn now() -> Zoned {
    utc(2026, 10, 7).to_zoned(TimeZone::UTC)
}

pub(crate) fn library() -> (TempDir, Library) {
    let root = tempfile::tempdir().unwrap();
    let library = Library::open(root.path()).unwrap();
    (root, library)
}

pub(crate) fn foreign_id() -> DocumentId {
    let (_root, other) = library();
    other.create("Foreign", &plain("Hello"), at(100)).unwrap()
}

pub(crate) fn reopen(root: &TempDir) -> Library {
    Library::open(root.path()).unwrap()
}

pub(crate) fn plain(text: &str) -> Document {
    Document::new(text, TextFormat::Plain).unwrap()
}

pub(crate) fn markdown(text: &str) -> Document {
    Document::new(text, TextFormat::Markdown).unwrap()
}

pub(crate) fn voice(key: &'static str, quality: Quality) -> StoredVoice {
    StoredVoice::new(VoiceId::new(EngineId::new("fake"), key), quality)
}

pub(crate) fn key(digit: char) -> SegmentKey {
    digit.to_string().repeat(64).parse().unwrap()
}

pub(crate) fn document_dir(root: &TempDir, id: DocumentId) -> PathBuf {
    root.path().join("library").join(id.to_string())
}

pub(crate) fn complete(
    library: &Library,
    id: DocumentId,
    document: &Document,
    voice: &StoredVoice,
) {
    let hash = text_hash(document);
    library.set_voice(id, voice.clone(), at(200)).unwrap();
    library
        .record_segments(id, hash, voice, vec![key('a'), key('b')])
        .unwrap();
    library
        .record_complete(id, hash, voice, Duration::from_secs(5))
        .unwrap();
}

pub(crate) fn status_of(library: &Library, id: DocumentId) -> Status {
    let groups = library.list(Filter::All, "", &now());
    let summary = groups
        .iter()
        .flat_map(|group| &group.documents)
        .find(|summary| summary.meta.id == id);
    summary.unwrap().status
}

pub(crate) fn titles(groups: &[Group]) -> Vec<String> {
    let documents = groups.iter().flat_map(|group| &group.documents);
    documents
        .map(|summary| summary.meta.title.clone())
        .collect()
}

pub(crate) fn sorted_titles(library: &Library, filter: Filter) -> Vec<String> {
    let mut names = titles(&library.list(filter, "", &now()));
    names.sort();
    names
}

pub(crate) fn create_all(library: &Library, documents: &[(&str, Timestamp)]) {
    for (title, now) in documents {
        library.create(title, &plain(title), *now).unwrap();
    }
}
