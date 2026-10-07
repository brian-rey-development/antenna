use std::fs::{self, OpenOptions};

use antenna_models::{ModelError, engine_artifacts};

use crate::descriptor::{Alpha, alpha};
use crate::fixture::Fixture;
use crate::support::whole;

fn install_engine(fixture: &Fixture, alpha: &Alpha) {
    fixture.ensure(&engine_artifacts(alpha.descriptor)).unwrap();
}

#[test]
fn remove_engine_keeps_artifact_when_listed_in_keep() {
    let fixture = Fixture::new();
    let alpha = alpha(&fixture);
    install_engine(&fixture, &alpha);

    fixture
        .store
        .remove_engine(alpha.descriptor, [alpha.weights])
        .unwrap();

    assert!(fixture.store.has_artifact(alpha.weights));
    assert!(!fixture.file(alpha.prompt_a).exists());
    assert!(!fixture.file(alpha.prompt_b).exists());
}

#[test]
fn remove_engine_fails_when_artifact_is_locked() {
    let fixture = Fixture::new();
    let alpha = alpha(&fixture);
    install_engine(&fixture, &alpha);
    let lock = OpenOptions::new()
        .write(true)
        .open(fixture.lock(alpha.prompt_a))
        .unwrap();
    lock.try_lock().unwrap();

    let result = fixture.store.remove_engine(alpha.descriptor, []);

    assert!(matches!(
        result,
        Err(ModelError::Busy {
            artifact: "prompt-a"
        })
    ));
    assert!(fixture.file(alpha.prompt_a).exists());
    assert!(fixture.file(alpha.prompt_b).exists());
}

#[test]
fn remove_engine_returns_deleted_bytes_and_prunes_directories() {
    let fixture = Fixture::new();
    let alpha = alpha(&fixture);
    install_engine(&fixture, &alpha);
    let before = fixture.store.disk_usage().unwrap();

    let deleted = fixture.store.remove_engine(alpha.descriptor, []).unwrap();

    assert_eq!(deleted, before);
    assert_eq!(fs::read_dir(fixture.store.root()).unwrap().count(), 0);
}

#[test]
fn remove_engine_deletes_partial_file_and_verified_record() {
    let fixture = Fixture::new();
    let alpha = alpha(&fixture);
    fixture.ensure(&[alpha.weights]).unwrap();
    fixture.write_partial(alpha.prompt_a, b"half");

    fixture.store.remove_engine(alpha.descriptor, []).unwrap();

    assert!(!fixture.verified(alpha.weights).exists());
    assert!(!fixture.partial(alpha.prompt_a).exists());
}

#[test]
fn remove_engine_keeps_file_when_artifact_is_not_declared() {
    let fixture = Fixture::new();
    let alpha = alpha(&fixture);
    install_engine(&fixture, &alpha);
    let stranger = fixture.file(alpha.weights).with_file_name("stranger.txt");
    fs::write(&stranger, b"mine").unwrap();

    fixture.store.remove_engine(alpha.descriptor, []).unwrap();

    assert!(stranger.is_file());
}

#[test]
fn remove_engine_returns_zero_when_engine_not_installed() {
    let fixture = Fixture::new();
    let alpha = alpha(&fixture);

    let deleted = fixture.store.remove_engine(alpha.descriptor, []).unwrap();

    assert_eq!(deleted, 0);
}

#[test]
fn remove_engine_ignores_keep_entries_of_other_files() {
    let fixture = Fixture::new();
    let alpha = alpha(&fixture);
    install_engine(&fixture, &alpha);
    let stranger = whole("stranger", b"x");

    fixture
        .store
        .remove_engine(alpha.descriptor, [stranger])
        .unwrap();

    assert!(!fixture.file(alpha.weights).exists());
}
