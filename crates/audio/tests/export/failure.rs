use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use antenna_audio::{AudioError, ExportRate, Loudness, export};
use antenna_core::{ExportFormat, SampleRate};

use crate::support::{read_wav, run, sine, spec, write_segments};

fn assert_only_segments_remain(directory: &Path) {
    let names: Vec<_> = fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert!(
        names
            .iter()
            .all(|name| name.to_string_lossy().starts_with("segment-")),
        "{names:?}"
    );
}

fn assert_cancel_leaves_no_file(format: ExportFormat, loudness: Loudness) {
    let directory = tempfile::tempdir().unwrap();
    let samples = sine(1, 0.5);
    let segments = write_segments(directory.path(), &[&samples, &samples, &samples]);
    let mut spec = spec(directory.path(), segments, format, ExportRate::Hz24000);
    spec.loudness = loudness;
    let cancel = AtomicBool::new(false);

    let result = export(&spec, &cancel, &|_| cancel.store(true, Ordering::Relaxed));

    assert!(matches!(result, Err(AudioError::Cancelled)), "{format}");
    assert_only_segments_remain(directory.path());
}

fn assert_failure_leaves_no_file(format: ExportFormat) {
    let directory = tempfile::tempdir().unwrap();
    let samples = sine(1, 0.5);
    let segments = write_segments(directory.path(), &[&samples, &samples]);
    fs::remove_file(&segments[1]).unwrap();
    let spec = spec(directory.path(), segments, format, ExportRate::Hz24000);

    let result = export(&spec, &AtomicBool::new(false), &|_| {});

    assert!(
        matches!(result, Err(AudioError::ReadSegment { .. })),
        "{format}"
    );
    assert_only_segments_remain(directory.path());
}

#[test]
fn export_leaves_no_file_when_cancelled() {
    assert_cancel_leaves_no_file(ExportFormat::Mp3, Loudness::Keep);
    assert_cancel_leaves_no_file(ExportFormat::Wav, Loudness::Keep);
    assert_cancel_leaves_no_file(ExportFormat::Ogg, Loudness::Keep);
}

#[test]
fn export_leaves_no_file_when_cancelled_while_measuring() {
    assert_cancel_leaves_no_file(ExportFormat::Wav, Loudness::Normalize);
}

#[test]
fn export_leaves_no_file_when_encoder_fails() {
    assert_failure_leaves_no_file(ExportFormat::Mp3);
    assert_failure_leaves_no_file(ExportFormat::Wav);
    assert_failure_leaves_no_file(ExportFormat::Ogg);
}

#[test]
fn export_leaves_no_file_when_commit_fails() {
    let directory = tempfile::tempdir().unwrap();
    let segments = write_segments(directory.path(), &[&sine(1, 0.5)]);
    let mut spec = spec(
        directory.path(),
        segments,
        ExportFormat::Wav,
        ExportRate::Hz24000,
    );
    spec.destination = directory.path().join("taken");
    fs::create_dir(&spec.destination).unwrap();

    let result = export(&spec, &AtomicBool::new(false), &|_| {});

    assert!(matches!(result, Err(AudioError::Write { .. })));
    assert!(!directory.path().join("taken.part").exists());
}

#[test]
fn export_fails_when_segment_rate_differs() {
    let directory = tempfile::tempdir().unwrap();
    let segments = write_segments(directory.path(), &[&sine(1, 0.5)]);
    let mut spec = spec(
        directory.path(),
        segments,
        ExportFormat::Wav,
        ExportRate::Hz24000,
    );
    spec.source_rate = SampleRate::HZ_48000;

    let result = export(&spec, &AtomicBool::new(false), &|_| {});

    assert!(matches!(
        result,
        Err(AudioError::RateMismatch {
            expected: 48_000,
            actual: 24_000,
            ..
        })
    ));
}

#[test]
fn export_fails_when_segment_rate_differs_while_measuring() {
    let directory = tempfile::tempdir().unwrap();
    let segments = write_segments(directory.path(), &[&sine(1, 0.5)]);
    let mut spec = spec(
        directory.path(),
        segments,
        ExportFormat::Wav,
        ExportRate::Hz24000,
    );
    spec.source_rate = SampleRate::HZ_48000;
    spec.loudness = Loudness::Normalize;

    let result = export(&spec, &AtomicBool::new(false), &|_| {});

    assert!(matches!(result, Err(AudioError::RateMismatch { .. })));
}

fn progress_of(loudness: Loudness) -> Vec<f32> {
    let directory = tempfile::tempdir().unwrap();
    let segments = write_segments(directory.path(), &[&sine(1, 0.5), &sine(1, 0.5)]);
    let mut spec = spec(
        directory.path(),
        segments,
        ExportFormat::Wav,
        ExportRate::Hz24000,
    );
    spec.loudness = loudness;
    let (sender, receiver) = flume::unbounded();

    export(&spec, &AtomicBool::new(false), &move |fraction| {
        sender.send(fraction).unwrap();
    })
    .unwrap();

    receiver.drain().collect()
}

#[test]
fn export_reports_progress_when_loudness_normalized() {
    assert_eq!(progress_of(Loudness::Normalize), [0.25, 0.5, 0.75, 1.0]);
}

#[test]
fn export_reports_progress_when_loudness_kept() {
    assert_eq!(progress_of(Loudness::Keep), [0.5, 1.0]);
}

#[test]
fn export_replaces_file_when_destination_exists() {
    let directory = tempfile::tempdir().unwrap();
    let segments = write_segments(directory.path(), &[&sine(1, 0.5)]);
    let spec = spec(
        directory.path(),
        segments,
        ExportFormat::Wav,
        ExportRate::Hz24000,
    );
    fs::write(&spec.destination, b"old").unwrap();

    run(&spec);

    let (wav, decoded) = read_wav(&spec.destination);
    assert_eq!(wav.sample_rate, 24_000);
    assert_eq!(decoded.len(), 24_000);
}

#[test]
fn export_fails_when_destination_directory_missing() {
    let directory = tempfile::tempdir().unwrap();
    let segments = write_segments(directory.path(), &[&sine(1, 0.5)]);
    let mut spec = spec(
        directory.path(),
        segments,
        ExportFormat::Wav,
        ExportRate::Hz24000,
    );
    spec.destination = directory.path().join("missing").join("episode.wav");

    let result = export(&spec, &AtomicBool::new(false), &|_| {});

    assert!(matches!(result, Err(AudioError::Write { .. })));
}
