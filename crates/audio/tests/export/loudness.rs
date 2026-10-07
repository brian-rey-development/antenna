use std::path::PathBuf;

use antenna_audio::{ExportRate, ExportSummary, Loudness};
use antenna_core::ExportFormat;
use tempfile::TempDir;

use crate::support::{
    SOURCE_RATE, clicks, measure, read_wav, run, scale_to_loudness, sine, spec, write_segments,
};

const TARGET_LUFS: f64 = -16.0;
const TRUE_PEAK_LIMIT_DBTP: f64 = -1.0;
const TOLERANCE_LU: f64 = 0.5;
const TRUE_PEAK_TOLERANCE_DB: f64 = 0.01;
const CLICK_LUFS: f64 = -20.0;
const DECIBELS_PER_DECADE: f64 = 20.0;

fn export_normalized(
    parts: &[&[f32]],
    format: ExportFormat,
    rate: ExportRate,
) -> (TempDir, ExportSummary, PathBuf) {
    let directory = tempfile::tempdir().unwrap();
    let segments = write_segments(directory.path(), parts);
    let mut spec = spec(directory.path(), segments, format, rate);
    spec.loudness = Loudness::Normalize;
    let summary = run(&spec);
    let destination = spec.destination;
    (directory, summary, destination)
}

fn normalized_wav(parts: &[&[f32]]) -> (TempDir, ExportSummary, Vec<f32>) {
    let (directory, summary, path) =
        export_normalized(parts, ExportFormat::Wav, ExportRate::Hz24000);
    let (_, decoded) = read_wav(&path);
    (directory, summary, decoded)
}

fn true_peak_db(samples: &[f32]) -> f64 {
    let peak = measure(samples, SOURCE_RATE).true_peak(0).unwrap();
    DECIBELS_PER_DECADE * peak.log10()
}

fn loudness_of(samples: &[f32]) -> f64 {
    measure(samples, SOURCE_RATE).loudness_global().unwrap()
}

fn assert_normalizes(source_lufs: f64) {
    let samples = scale_to_loudness(&sine(6, 1.0), source_lufs);
    assert!(
        samples.iter().all(|sample| sample.abs() < 1.0),
        "the fixture clips"
    );

    let (_directory, summary, decoded) = normalized_wav(&[&samples]);

    let loudness = loudness_of(&decoded);
    assert!(
        (loudness - TARGET_LUFS).abs() <= TOLERANCE_LU,
        "loudness {loudness}"
    );
    let expected_gain = TARGET_LUFS - source_lufs;
    assert!(
        (summary.gain_db - expected_gain).abs() <= TOLERANCE_LU,
        "gain {}",
        summary.gain_db
    );
}

#[test]
fn export_normalizes_loudness_when_minus_30_lufs() {
    assert_normalizes(-30.0);
}

#[test]
fn export_normalizes_loudness_when_minus_6_lufs() {
    assert_normalizes(-6.0);
}

#[test]
fn export_normalizes_joined_segments_when_segments_differ_in_loudness() {
    let loud = scale_to_loudness(&sine(4, 1.0), -12.0);
    let quiet = scale_to_loudness(&sine(4, 1.0), -30.0);
    let joined: Vec<f32> = loud.iter().chain(&quiet).copied().collect();

    let (_directory, summary, _) = normalized_wav(&[&loud, &quiet]);

    let expected_gain = TARGET_LUFS - loudness_of(&joined);
    assert!(
        (summary.gain_db - expected_gain).abs() <= 0.1,
        "gain {}",
        summary.gain_db
    );
}

#[test]
fn export_ignores_silent_segment_when_measuring() {
    let samples = scale_to_loudness(&sine(4, 1.0), -30.0);
    let silence = vec![0.0; samples.len()];

    let (_directory, summary, _) = normalized_wav(&[&silence, &samples]);

    assert!(
        (summary.gain_db - (TARGET_LUFS + 30.0)).abs() <= TOLERANCE_LU,
        "gain {}",
        summary.gain_db
    );
}

#[test]
fn export_applies_same_gain_when_format_differs() {
    let samples = scale_to_loudness(&sine(4, 1.0), -30.0);

    let gains = ExportFormat::ALL.map(|format| {
        export_normalized(&[&samples], format, ExportRate::Hz44100)
            .1
            .gain_db
            .to_bits()
    });

    assert_eq!(gains[0], gains[1]);
    assert_eq!(gains[1], gains[2]);
}

#[test]
fn export_limits_true_peak_when_crest_high() {
    let samples = scale_to_loudness(&clicks(6, 10), CLICK_LUFS);
    assert!(
        samples.iter().all(|sample| sample.abs() < 1.0),
        "the fixture clips"
    );
    let unlimited_peak = true_peak_db(&samples) + (TARGET_LUFS - CLICK_LUFS);
    assert!(
        unlimited_peak > TRUE_PEAK_LIMIT_DBTP,
        "the fixture needs no limit"
    );

    let (_directory, _summary, decoded) = normalized_wav(&[&samples]);

    let peak = true_peak_db(&decoded);
    assert!(
        (peak - TRUE_PEAK_LIMIT_DBTP).abs() <= TRUE_PEAK_TOLERANCE_DB,
        "true peak {peak}"
    );
}

#[test]
fn export_keeps_silence_when_loudness_infinite() {
    let (_directory, summary, decoded) =
        normalized_wav(&[&vec![0.0; 2 * SOURCE_RATE.hz() as usize]]);

    assert_eq!(summary.gain_db.to_bits(), 0.0_f64.to_bits());
    assert!(
        decoded
            .iter()
            .all(|sample| sample.to_bits() == 0.0_f32.to_bits())
    );
}

#[test]
fn export_keeps_level_when_loudness_keep() {
    let directory = tempfile::tempdir().unwrap();
    let samples = sine(1, 0.25);
    let segments = write_segments(directory.path(), &[&samples]);
    let spec = spec(
        directory.path(),
        segments,
        ExportFormat::Wav,
        ExportRate::Hz24000,
    );

    let summary = run(&spec);

    assert_eq!(summary.gain_db.to_bits(), 0.0_f64.to_bits());
}
