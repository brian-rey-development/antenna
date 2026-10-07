use std::fs::File;
use std::iter;
use std::path::{Path, PathBuf};
use std::time::Duration;

use antenna_audio::ExportRate;
use antenna_core::ExportFormat;
use ogg::{Packet, PacketReader};
use tempfile::TempDir;

use crate::probe::probe;
use crate::support::{SOURCE_RATE, TITLE, run, sine_frames, spec, write_segments};

const OGG_RATE_HZ: u64 = 48_000;
const OPUS_FRAME_GRANULES: u64 = 960;
const PRE_SKIP_FIELD: (usize, usize) = (10, 12);
const INPUT_RATE_FIELD: (usize, usize) = (12, 16);
const PAGE_FRAMES: u64 = 50;
const TOLERANCE: Duration = Duration::from_millis(50);

fn source_rate() -> usize {
    SOURCE_RATE.hz() as usize
}

fn export_ogg(rate: ExportRate, frames: usize) -> (TempDir, PathBuf) {
    let directory = tempfile::tempdir().unwrap();
    let samples = sine_frames(frames, 0.5);
    let parts: &[&[f32]] = if frames == 0 { &[] } else { &[&samples] };
    let segments = write_segments(directory.path(), parts);
    let spec = spec(directory.path(), segments, ExportFormat::Ogg, rate);
    run(&spec);
    let destination = spec.destination;
    (directory, destination)
}

fn packets(path: &Path) -> Vec<Packet> {
    let mut reader = PacketReader::new(File::open(path).unwrap());
    iter::from_fn(|| reader.read_packet().unwrap()).collect()
}

fn head_field(head: &[u8], (from, to): (usize, usize)) -> u64 {
    head[from..to]
        .iter()
        .rev()
        .fold(0, |value, byte| value << 8 | u64::from(*byte))
}

fn final_granule_and_pre_skip(path: &Path) -> (u64, u64) {
    let packets = packets(path);
    let pre_skip = head_field(&packets[0].data, PRE_SKIP_FIELD);
    (packets.last().unwrap().absgp_page(), pre_skip)
}

#[test]
fn ogg_export_has_opus_head_and_tags_when_exported() {
    let (_directory, path) = export_ogg(ExportRate::Hz48000, source_rate());

    let packets = packets(&path);

    assert!(packets[0].data.starts_with(b"OpusHead"));
    assert!(packets[1].data.starts_with(b"OpusTags"));
}

#[test]
fn ogg_export_has_valid_pages_and_granule() {
    let (_directory, path) = export_ogg(ExportRate::Hz48000, 2 * source_rate());

    let (granule, pre_skip) = final_granule_and_pre_skip(&path);

    assert_eq!(granule, pre_skip + 2 * OGG_RATE_HZ);
    assert!(packets(&path).last().unwrap().last_in_stream());
}

#[test]
fn ogg_export_ends_stream_on_last_packet_only() {
    let (_directory, path) = export_ogg(ExportRate::Hz48000, 2 * source_rate());

    let packets = packets(&path);

    let ending = packets
        .iter()
        .filter(|packet| packet.last_in_stream())
        .count();
    assert_eq!(ending, 1);
}

#[test]
fn ogg_export_ends_at_final_granule_when_rate_is_24000() {
    let (_directory, path) = export_ogg(ExportRate::Hz24000, 2 * source_rate());

    let (granule, pre_skip) = final_granule_and_pre_skip(&path);

    assert_eq!(granule, pre_skip + 2 * OGG_RATE_HZ);
}

#[test]
fn ogg_export_ends_at_final_granule_when_rate_is_44100() {
    let (_directory, path) = export_ogg(ExportRate::Hz44100, 2 * source_rate());

    let (granule, pre_skip) = final_granule_and_pre_skip(&path);

    assert_eq!(granule, pre_skip + 2 * OGG_RATE_HZ);
}

#[test]
fn ogg_export_ends_at_true_end_when_length_not_multiple_of_frame() {
    let frames = source_rate() + 123;
    let (_directory, path) = export_ogg(ExportRate::Hz48000, frames);

    let (granule, pre_skip) = final_granule_and_pre_skip(&path);

    assert_eq!(granule, pre_skip + 2 * frames as u64);
}

#[test]
fn ogg_export_ends_at_true_end_when_input_is_one_sample() {
    let (_directory, path) = export_ogg(ExportRate::Hz48000, 1);

    let (granule, pre_skip) = final_granule_and_pre_skip(&path);

    assert_eq!(granule, pre_skip + 2);
}

#[test]
fn ogg_export_ends_at_pre_skip_when_input_is_empty() {
    let (_directory, path) = export_ogg(ExportRate::Hz48000, 0);

    let (granule, pre_skip) = final_granule_and_pre_skip(&path);

    assert_eq!(granule, pre_skip);
}

#[test]
fn ogg_export_ends_page_every_second_when_audio_is_long() {
    let (_directory, path) = export_ogg(ExportRate::Hz48000, 3 * source_rate());

    let granules: Vec<u64> = packets(&path)
        .iter()
        .filter(|packet| packet.last_in_page())
        .map(Packet::absgp_page)
        .collect();

    let page_granule = PAGE_FRAMES * OPUS_FRAME_GRANULES;
    assert_eq!(granules[..2], [0, 0]);
    assert_eq!(
        granules[2..5],
        [page_granule, 2 * page_granule, 3 * page_granule]
    );
    assert_eq!(granules.len(), 6);
}

fn assert_encoder_rate(rate: ExportRate, expected: u64) {
    let (_directory, path) = export_ogg(rate, source_rate());

    let packets = packets(&path);

    assert_eq!(head_field(&packets[0].data, INPUT_RATE_FIELD), expected);
}

#[test]
fn ogg_export_uses_encoder_rate_when_24000() {
    assert_encoder_rate(ExportRate::Hz24000, 24_000);
}

#[test]
fn ogg_export_uses_encoder_rate_when_44100() {
    assert_encoder_rate(ExportRate::Hz44100, 48_000);
}

#[test]
fn ogg_export_uses_encoder_rate_when_48000() {
    assert_encoder_rate(ExportRate::Hz48000, 48_000);
}

#[test]
fn ogg_export_duration_matches_when_hz44100() {
    let (_directory, path) = export_ogg(ExportRate::Hz44100, 3 * source_rate());

    let decoded = probe(&path);

    assert_eq!(u64::from(decoded.rate), OGG_RATE_HZ);
    assert!(decoded.duration().abs_diff(Duration::from_secs(3)) <= TOLERANCE);
}

#[test]
fn ogg_export_duration_matches_when_granule_gives_it() {
    let (_directory, path) = export_ogg(ExportRate::Hz44100, 3 * source_rate());

    let (granule, pre_skip) = final_granule_and_pre_skip(&path);

    let duration = Duration::from_micros((granule - pre_skip) * 1_000_000 / OGG_RATE_HZ);
    assert!(duration.abs_diff(Duration::from_secs(3)) <= TOLERANCE);
}

#[test]
fn ogg_export_has_title_artist_and_language_tags() {
    let (_directory, path) = export_ogg(ExportRate::Hz48000, source_rate());

    let packets = packets(&path);

    let tags = String::from_utf8_lossy(&packets[1].data).into_owned();
    for expected in [
        format!("TITLE={TITLE}"),
        "ARTIST=Antenna".to_owned(),
        "LANGUAGE=es".to_owned(),
    ] {
        assert!(tags.contains(&expected), "{expected} is not in {tags:?}");
    }
}
