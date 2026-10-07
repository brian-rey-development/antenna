use std::fs::{self, File};
use std::time::SystemTime;

use antenna_core::Quality;
use antenna_library::{Filter, Library, Status, text_hash};

use super::support::{
    at, complete, document_dir, library, now, plain, reopen, status_of, titles as listed_titles,
    voice,
};

#[test]
fn status_is_draft_when_meta_is_older_than_text() {
    let (root, library) = library();
    let document = plain("Hello");
    let id = library.create("Notes", &document, at(100)).unwrap();
    complete(
        &library,
        id,
        &document,
        &voice("en-alba", Quality::Balanced),
    );
    fs::write(document_dir(&root, id).join("text.txt"), "Hello again").unwrap();
    let reopened = reopen(&root);

    reopened.load(id).unwrap();

    assert_eq!(status_of(&reopened, id), Status::Draft);
}

#[test]
fn load_stores_text_hash_when_text_file_is_newer_than_metadata() {
    let (root, library) = library();
    let id = library.create("Notes", &plain("Hello"), at(100)).unwrap();
    fs::write(document_dir(&root, id).join("text.txt"), "Hello again").unwrap();
    let reopened = reopen(&root);

    reopened.load(id).unwrap();
    let metadata = reopen(&root).load(id).unwrap().0;

    assert_eq!(metadata.text_hash, text_hash(&plain("Hello again")));
}

#[test]
fn open_skips_document_when_meta_is_damaged() {
    let (root, library) = library();
    let [good, damaged, absent] = ["Good", "Damaged", "Absent"]
        .map(|title| library.create(title, &plain(title), at(100)).unwrap());
    let library_dir = root.path().join("library");
    fs::write(
        document_dir(&root, damaged).join("document.toml"),
        "title = {",
    )
    .unwrap();
    fs::remove_file(document_dir(&root, absent).join("document.toml")).unwrap();
    fs::create_dir(library_dir.join("not-an-id")).unwrap();
    fs::write(library_dir.join(".DS_Store"), "x").unwrap();

    let reopened = reopen(&root);

    let mut expected = vec![
        document_dir(&root, damaged),
        document_dir(&root, absent),
        library_dir.join("not-an-id"),
    ];
    expected.sort();
    assert_eq!(reopened.count(), 1);
    assert!(reopened.load(good).is_ok());
    assert_eq!(reopened.skipped(), expected);
    assert!(expected.iter().all(|path| path.exists()));
}

#[test]
fn open_skips_directory_when_name_differs_from_id() {
    let (root, library) = library();
    let id = library.create("Notes", &plain("Hello"), at(100)).unwrap();
    let renamed = root.path().join("library").join("copy");
    fs::rename(document_dir(&root, id), &renamed).unwrap();

    let reopened = reopen(&root);

    assert_eq!(reopened.count(), 0);
    assert_eq!(reopened.skipped(), [renamed]);
}

#[test]
fn open_keeps_temp_files_when_stale() {
    let (root, library) = library();
    let id = library.create("Notes", &plain("Hello"), at(100)).unwrap();
    let old_temp = document_dir(&root, id).join("document.toml.tmp-1-0");
    fs::write(&old_temp, "partial").unwrap();
    File::options()
        .write(true)
        .open(&old_temp)
        .unwrap()
        .set_modified(SystemTime::UNIX_EPOCH)
        .unwrap();

    let reopened = reopen(&root);

    assert!(old_temp.is_file());
    assert_eq!(reopened.count(), 1);
}

fn titles(library: &Library, query: &str) -> Vec<String> {
    listed_titles(&library.list(Filter::All, query, &now()))
}

#[test]
fn search_matches_new_text_when_load_repairs_metadata() {
    let (root, library) = library();
    let id = library
        .create("Plan", &plain("Compra leche"), at(100))
        .unwrap();
    let reopened = reopen(&root);
    reopened.build_search_index().unwrap();
    fs::write(document_dir(&root, id).join("text.txt"), "Compra caf\u{e9}").unwrap();

    reopened.load(id).unwrap();

    assert_eq!(titles(&reopened, "cafe"), ["Plan"]);
    assert_eq!(titles(&reopened, "leche"), Vec::<String>::new());
}
