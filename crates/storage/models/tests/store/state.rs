use std::fs::{self, OpenOptions};

use antenna_core::{Artifact, Extent, Quality};
use antenna_models::{
    ModelStore, Source, engine_artifacts, engine_download_bytes, keep_artifacts, voice_artifacts,
};

use crate::descriptor::{
    Alpha, PROMPT_A_BYTES, PROMPT_B_BYTES, Parts, WEIGHTS_BYTES, alpha, engine, single_file_engine,
};
use crate::fixture::Fixture;
use crate::support::{leak, pattern, whole};

fn keys(artifacts: &[&'static Artifact]) -> Vec<&'static str> {
    artifacts.iter().map(|artifact| artifact.key).collect()
}

fn install_voice(fixture: &Fixture, alpha: &Alpha, voice: usize) {
    let voice = &alpha.descriptor.voices[voice];
    let artifacts = voice_artifacts(alpha.descriptor, voice, Quality::Balanced);
    fixture.ensure(&artifacts).unwrap();
}

#[test]
fn open_leaves_root_absent_when_directory_missing() {
    let parent = tempfile::tempdir().unwrap();
    let root = parent.path().join("models").join("nested");

    let store = ModelStore::open(&root, Source::Directory(parent.path().to_owned())).unwrap();

    assert!(!root.exists());
    assert_eq!(store.root(), root);
}

#[test]
fn disk_usage_is_zero_when_root_missing() {
    let parent = tempfile::tempdir().unwrap();
    let root = parent.path().join("models");
    let store = ModelStore::open(root, Source::Directory(parent.path().to_owned())).unwrap();

    let usage = store.disk_usage();

    assert_eq!(usage.unwrap(), 0);
}

#[test]
fn is_installed_true_when_variant_and_voice_installed() {
    let fixture = Fixture::new();
    let alpha = alpha(&fixture);
    install_voice(&fixture, &alpha, 0);

    let is_installed = fixture.store.is_installed(
        alpha.descriptor,
        &alpha.descriptor.voices[0],
        Quality::Balanced,
    );

    assert!(is_installed);
}

#[test]
fn is_installed_false_when_variant_missing() {
    let fixture = Fixture::new();
    let alpha = alpha(&fixture);
    fixture.ensure(&[alpha.prompt_a]).unwrap();

    let is_installed = fixture.store.is_installed(
        alpha.descriptor,
        &alpha.descriptor.voices[0],
        Quality::Balanced,
    );

    assert!(!is_installed);
}

#[test]
fn engine_is_installed_when_all_voices_installed() {
    let fixture = Fixture::new();
    let alpha = alpha(&fixture);
    install_voice(&fixture, &alpha, 0);
    install_voice(&fixture, &alpha, 1);

    let is_installed = fixture
        .store
        .is_engine_installed(alpha.descriptor, Quality::Balanced);

    assert!(is_installed);
}

#[test]
fn engine_is_not_installed_when_one_voice_missing() {
    let fixture = Fixture::new();
    let alpha = alpha(&fixture);
    install_voice(&fixture, &alpha, 0);

    let is_installed = fixture
        .store
        .is_engine_installed(alpha.descriptor, Quality::Balanced);

    assert!(!is_installed);
}

#[test]
fn has_artifact_false_when_verified_record_missing() {
    let fixture = Fixture::new();
    let alpha = alpha(&fixture);
    fixture.ensure(&[alpha.weights]).unwrap();
    fs::remove_file(fixture.verified(alpha.weights)).unwrap();

    let has_artifact = fixture.store.has_artifact(alpha.weights);

    assert!(!has_artifact);
}

#[test]
fn has_artifact_false_when_file_truncated() {
    let fixture = Fixture::new();
    let alpha = alpha(&fixture);
    fixture.ensure(&[alpha.weights]).unwrap();
    OpenOptions::new()
        .write(true)
        .open(fixture.file(alpha.weights))
        .unwrap()
        .set_len(10)
        .unwrap();

    let has_artifact = fixture.store.has_artifact(alpha.weights);

    assert!(!has_artifact);
}

#[test]
fn missing_bytes_subtracts_partial_files() {
    let fixture = Fixture::new();
    let alpha = alpha(&fixture);
    fixture.write_partial(alpha.weights, &pattern(300));

    let missing = fixture.store.missing_bytes([alpha.weights, alpha.prompt_a]);

    assert_eq!(missing, (WEIGHTS_BYTES - 300 + PROMPT_A_BYTES) as u64);
}

#[test]
fn missing_bytes_counts_full_size_when_partial_file_is_longer() {
    let fixture = Fixture::new();
    let alpha = alpha(&fixture);
    fixture.write_partial(alpha.prompt_a, &pattern(PROMPT_A_BYTES + 1));

    let missing = fixture.store.missing_bytes([alpha.prompt_a]);

    assert_eq!(missing, PROMPT_A_BYTES as u64);
}

#[test]
fn missing_bytes_is_zero_when_artifacts_installed() {
    let fixture = Fixture::new();
    let alpha = alpha(&fixture);
    fixture.ensure(&[alpha.weights]).unwrap();

    let missing = fixture.store.missing_bytes([alpha.weights]);

    assert_eq!(missing, 0);
}

#[test]
fn engine_disk_usage_ignores_missing_artifacts() {
    let fixture = Fixture::new();
    let alpha = alpha(&fixture);
    fixture.ensure(&[alpha.weights, alpha.prompt_b]).unwrap();

    let usage = fixture.store.engine_disk_usage(alpha.descriptor);

    assert_eq!(usage, (WEIGHTS_BYTES + PROMPT_B_BYTES) as u64);
}

#[test]
fn disk_usage_counts_partial_files() {
    let fixture = Fixture::new();
    let alpha = alpha(&fixture);
    fixture.ensure(&[alpha.weights]).unwrap();
    let installed = fixture.store.disk_usage().unwrap();
    fixture.write_partial(alpha.prompt_a, &pattern(123));

    let total = fixture.store.disk_usage().unwrap();

    assert!(installed >= WEIGHTS_BYTES as u64);
    assert_eq!(total, installed + 123);
}

#[test]
fn engine_download_bytes_counts_shared_file_once() {
    let first = whole("first", &pattern(1_000));
    let same_file = leak(Artifact {
        key: "second",
        ..*first
    });
    let other = whole("other", &pattern(40));
    let descriptor = engine(
        "shared",
        &Parts {
            fast: &[other],
            balanced: &[first],
            max: &[other],
            voices: &[&[same_file], &[other]],
        },
    );

    let total = engine_download_bytes(descriptor, Quality::Balanced);

    assert_eq!(total, 1_040);
}

#[test]
fn engine_artifacts_lists_each_local_file_once() {
    let first = whole("first", &pattern(10));
    let same_file = leak(Artifact {
        key: "second",
        ..*first
    });
    let ranged = leak(Artifact {
        extent: Extent::Range {
            offset: 0,
            bytes: 10,
        },
        key: "ranged",
        ..*first
    });
    let descriptor = engine(
        "listing",
        &Parts {
            fast: &[first],
            balanced: &[first],
            max: &[ranged],
            voices: &[&[same_file, ranged]],
        },
    );

    let artifacts = engine_artifacts(descriptor);

    assert_eq!(keys(&artifacts), ["first", "ranged"]);
}

#[test]
fn voice_artifacts_lists_variant_then_voice() {
    let fixture = Fixture::new();
    let alpha = alpha(&fixture);

    let artifacts = voice_artifacts(
        alpha.descriptor,
        &alpha.descriptor.voices[1],
        Quality::Balanced,
    );

    assert_eq!(keys(&artifacts), ["weights", "prompt-b"]);
}

#[test]
fn keep_artifacts_includes_other_engines_and_extra() {
    let fixture = Fixture::new();
    let alpha = alpha(&fixture);
    let beta_file = whole("beta-file", &pattern(10));
    let beta = single_file_engine("beta", beta_file);
    let review = whole("review", &pattern(10));

    let kept = keep_artifacts([beta, alpha.descriptor], [review]);

    assert_eq!(
        keys(&kept),
        [
            "beta-file",
            "fast",
            "weights",
            "max",
            "prompt-a",
            "prompt-b",
            "review"
        ]
    );
}
