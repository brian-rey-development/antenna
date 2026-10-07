use std::fs;
use std::path::PathBuf;

use antenna_audio::ExportRate;
use antenna_core::ExportFormat;

use crate::probe::{decode_mono, probe};
use crate::support::{SOURCE_RATE, TITLE, measure, run, sine, sine_frames, spec, write_segments};

const SECONDS: usize = 3;
const ID3_HEADER_BYTES: usize = 10;
const LAME_TAG_SEARCH_BYTES: usize = 64;
const SYNCSAFE_BITS: u32 = 7;
const LEVEL_TOLERANCE_LU: f64 = 1.0;

fn export_mp3(rate: ExportRate, samples: &[f32]) -> (tempfile::TempDir, PathBuf) {
    let directory = tempfile::tempdir().unwrap();
    let segments = write_segments(directory.path(), &[samples]);
    let spec = spec(directory.path(), segments, ExportFormat::Mp3, rate);
    run(&spec);
    let destination = spec.destination;
    (directory, destination)
}

fn assert_decodes_to_duration(rate: ExportRate) {
    let (_directory, path) = export_mp3(rate, &sine(SECONDS, 0.5));

    let decoded = decode_mono(&path);

    assert_eq!(probe(&path).rate, rate.sample_rate().hz());
    assert_eq!(decoded.len(), SECONDS * rate.sample_rate().hz() as usize);
}

fn id3_size(bytes: &[u8]) -> usize {
    let size = bytes[6..ID3_HEADER_BYTES]
        .iter()
        .fold(0, |size, byte| size << SYNCSAFE_BITS | usize::from(*byte));
    ID3_HEADER_BYTES + size
}

#[test]
fn mp3_export_duration_matches_when_24000() {
    assert_decodes_to_duration(ExportRate::Hz24000);
}

#[test]
fn mp3_export_duration_matches_when_44100() {
    assert_decodes_to_duration(ExportRate::Hz44100);
}

#[test]
fn mp3_export_duration_matches_when_48000() {
    assert_decodes_to_duration(ExportRate::Hz48000);
}

#[test]
fn mp3_export_keeps_end_of_audio_when_length_not_multiple_of_frame() {
    let frames = 24_123;
    let (_directory, path) = export_mp3(ExportRate::Hz24000, &sine_frames(frames, 0.5));

    let decoded = decode_mono(&path);

    assert_eq!(decoded.len(), frames);
}

#[test]
fn mp3_export_keeps_level_when_decoded() {
    let source = sine(SECONDS, 0.5);
    let (_directory, path) = export_mp3(ExportRate::Hz24000, &source);

    let decoded = decode_mono(&path);

    let loudness = |samples: &[f32]| measure(samples, SOURCE_RATE).loudness_global().unwrap();
    let difference = loudness(&decoded) - loudness(&source);
    assert!(
        difference.abs() <= LEVEL_TOLERANCE_LU,
        "difference {difference} LU"
    );
}

#[test]
fn mp3_export_has_lame_tag_when_id3_tag_precedes() {
    let (_directory, path) = export_mp3(ExportRate::Hz24000, &sine(1, 0.5));

    let bytes = fs::read(&path).unwrap();

    let start = id3_size(&bytes);
    assert_eq!(bytes[start], 0xFF);
    assert!(tag_follows(&bytes, start));
}

fn tag_follows(bytes: &[u8], start: usize) -> bool {
    bytes[start..start + LAME_TAG_SEARCH_BYTES]
        .windows(4)
        .any(|window| window == b"Info")
}

#[test]
fn mp3_export_writes_file_when_no_segments() {
    let directory = tempfile::tempdir().unwrap();
    let spec = spec(
        directory.path(),
        Vec::new(),
        ExportFormat::Mp3,
        ExportRate::Hz24000,
    );

    run(&spec);

    assert!(decode_mono(&spec.destination).is_empty());
}

#[test]
fn mp3_export_has_title_tag() {
    let (_directory, path) = export_mp3(ExportRate::Hz24000, &sine(1, 0.5));

    assert_eq!(probe(&path).tag("title"), Some(TITLE));
}

#[test]
fn mp3_export_has_artist_and_comment_tags() {
    let (_directory, path) = export_mp3(ExportRate::Hz24000, &sine(1, 0.5));

    let decoded = probe(&path);

    assert_eq!(decoded.tag("artist"), Some("Antenna"));
    assert_eq!(decoded.tag("comment"), Some("es qwen3/es-lucia"));
}
