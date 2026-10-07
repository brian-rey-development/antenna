//! Benchmark of the export of 10 minutes of stored audio.

#[path = "../tests/fixtures/mod.rs"]
mod fixtures;

use std::sync::atomic::AtomicBool;

use antenna_audio::{EpisodeTags, ExportRate, ExportSpec, Loudness, export};
use antenna_core::{EngineId, ExportFormat, Language, SampleRate, VoiceId};
use divan::Bencher;
use tempfile::TempDir;

const SOURCE_RATE: SampleRate = SampleRate::HZ_24000;
const SEGMENT_COUNT: usize = 60;
const SEGMENT_SECONDS: usize = 10;
const SINE_HZ: f32 = 220.0;
const AMPLITUDE: f32 = 0.25;
const BENCHMARK_TITLE: &str = "Benchmark";
const BENCHMARK_VOICE_KEY: &str = "en-alba";

fn main() {
    divan::main();
}

#[expect(
    clippy::unwrap_used,
    reason = "a fixture that cannot be written stops the benchmark"
)]
fn fixture() -> (TempDir, ExportSpec) {
    let directory = tempfile::tempdir().unwrap();
    let frames = SEGMENT_SECONDS * SOURCE_RATE.hz() as usize;
    let samples = fixtures::sine(SINE_HZ, SOURCE_RATE, frames, AMPLITUDE);
    let segments = (0..SEGMENT_COUNT)
        .map(|index| {
            let path = directory.path().join(format!("segment-{index}.wav"));
            fixtures::write_segment(&path, SOURCE_RATE, &samples).unwrap();
            path
        })
        .collect();
    let spec = ExportSpec {
        segments,
        source_rate: SOURCE_RATE,
        format: ExportFormat::Mp3,
        export_rate: ExportRate::default(),
        loudness: Loudness::Normalize,
        tags: tags(),
        destination: directory.path().join("episode.mp3"),
    };
    (directory, spec)
}

fn tags() -> EpisodeTags {
    EpisodeTags {
        title: BENCHMARK_TITLE.to_owned(),
        voice: VoiceId::new(EngineId::new("fake"), BENCHMARK_VOICE_KEY),
        language: Language::En,
    }
}

/// Exports 10 minutes of stored 24 kHz audio to MP3 with loudness normalization.
#[divan::bench(sample_count = 5, sample_size = 1)]
#[expect(clippy::unwrap_used, reason = "a failed export stops the benchmark")]
fn export_ten_minutes_to_mp3(bencher: Bencher<'_, '_>) {
    let (_directory, spec) = fixture();
    let cancel = AtomicBool::new(false);

    bencher.bench_local(|| export(&spec, &cancel, &|_| {}).unwrap());
}
