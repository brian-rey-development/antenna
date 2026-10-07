use std::sync::{Arc, Barrier};
use std::thread;

use antenna_core::ExportFormat;
use antenna_library::{DocumentId, DocumentMeta, Library, text_hash};

use super::support::{at, library, plain, reopen};

const ROUNDS: i64 = 30;
const OPENED_BASE: i64 = 10_000;
const EXPORT_BASE: i64 = 50_000;

fn run_together(first: impl Fn() + Sync, second: impl Fn() + Sync) {
    let start = Barrier::new(2);
    thread::scope(|scope| {
        let writers = [&first as &(dyn Fn() + Sync), &second].map(|write| {
            let start = &start;
            thread::Builder::new()
                .name("antenna-test-writer".to_owned())
                .spawn_scoped(scope, move || {
                    start.wait();
                    write();
                })
                .unwrap()
        });
        for writer in writers {
            writer.join().unwrap();
        }
    });
}

fn stored(library: &Library, id: DocumentId) -> Arc<DocumentMeta> {
    library.load(id).unwrap().0
}

#[test]
fn changes_of_two_fields_survive_when_two_threads_write_one_document() {
    let (root, library) = library();
    let id = library.create("Notes", &plain("Hello"), at(100)).unwrap();

    run_together(
        || {
            for round in 1..=ROUNDS {
                library.mark_opened(id, at(OPENED_BASE + round)).unwrap();
            }
        },
        || {
            for round in 1..=ROUNDS {
                let at = at(EXPORT_BASE + round);
                library.record_export(id, ExportFormat::Wav, at).unwrap();
            }
        },
    );

    for reader in [&library, &reopen(&root)] {
        let metadata = stored(reader, id);
        let export = metadata.last_export.clone().unwrap();
        assert_eq!(metadata.opened, at(OPENED_BASE + ROUNDS));
        assert_eq!(export.at, at(EXPORT_BASE + ROUNDS));
    }
}

#[test]
fn text_and_opened_time_agree_with_disk_when_two_threads_write_one_document() {
    let (root, library) = library();
    let id = library.create("Notes", &plain("Hello"), at(100)).unwrap();
    let last = plain(&format!("Text {ROUNDS}"));

    run_together(
        || {
            for round in 1..=ROUNDS {
                let document = plain(&format!("Text {round}"));
                library.save_text(id, &document, at(round)).unwrap();
            }
        },
        || {
            for round in 1..=ROUNDS {
                library.mark_opened(id, at(OPENED_BASE + round)).unwrap();
            }
        },
    );

    for reader in [&library, &reopen(&root)] {
        let (metadata, document) = reader.load(id).unwrap();
        assert_eq!(document, last);
        assert_eq!(metadata.text_hash, text_hash(&last));
        assert_eq!(metadata.opened, at(OPENED_BASE + ROUNDS));
    }
}
