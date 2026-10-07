use std::fs;
use std::time::Duration;

use antenna_audio::ExportRate;
use antenna_core::ExportFormat;

use crate::support::{SOURCE_RATE, read_wav, run, sine, spec, write_segments};

fn assert_wav_round_trips(rate: ExportRate) {
    let directory = tempfile::tempdir().unwrap();
    let samples = sine(1, 0.5);
    let (first, second) = samples.split_at(10_000);
    let segments = write_segments(directory.path(), &[first, second]);
    let spec = spec(directory.path(), segments, ExportFormat::Wav, rate);

    let summary = run(&spec);

    let hz = rate.sample_rate().hz();
    let (wav, decoded) = read_wav(&spec.destination);
    assert_eq!(
        (wav.channels, wav.sample_rate, wav.bits_per_sample),
        (1, hz, 16)
    );
    assert_eq!(decoded.len(), hz as usize);
    assert_eq!(summary.duration, Duration::from_secs(1));
    assert_eq!(
        summary.bytes,
        fs::metadata(&spec.destination).unwrap().len()
    );
}

#[test]
fn wav_export_round_trips_when_24000() {
    assert_wav_round_trips(ExportRate::Hz24000);
}

#[test]
fn wav_export_round_trips_when_44100() {
    assert_wav_round_trips(ExportRate::Hz44100);
}

#[test]
fn wav_export_round_trips_when_48000() {
    assert_wav_round_trips(ExportRate::Hz48000);
}

#[test]
fn wav_export_keeps_samples_when_rates_equal() {
    let directory = tempfile::tempdir().unwrap();
    let samples = sine(1, 0.5);
    let segments = write_segments(directory.path(), &[&samples]);
    let spec = spec(
        directory.path(),
        segments,
        ExportFormat::Wav,
        ExportRate::Hz24000,
    );
    assert_eq!(spec.source_rate, SOURCE_RATE);

    run(&spec);

    let (_, decoded) = read_wav(&spec.destination);
    let largest_difference = decoded
        .iter()
        .zip(&samples)
        .map(|(written, source)| (written - source).abs())
        .fold(0.0, f32::max);
    assert!(
        largest_difference < 1.0 / 32_767.0,
        "difference {largest_difference}"
    );
}

#[test]
fn wav_export_writes_empty_file_when_no_segments() {
    let directory = tempfile::tempdir().unwrap();
    let spec = spec(
        directory.path(),
        Vec::new(),
        ExportFormat::Wav,
        ExportRate::Hz24000,
    );

    run(&spec);

    let (wav, decoded) = read_wav(&spec.destination);
    assert_eq!(wav.sample_rate, 24_000);
    assert!(decoded.is_empty());
}
