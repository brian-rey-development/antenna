use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use antenna_audio::{EpisodeTags, ExportRate, ExportSpec, ExportSummary, Loudness, export};
use antenna_core::{EngineId, ExportFormat, Language, PCM_SCALE, SampleRate, VoiceId};

use crate::fixtures;
use ebur128::{EbuR128, Mode};
use hound::{WavReader, WavSpec};

pub(crate) const SOURCE_RATE: SampleRate = SampleRate::HZ_24000;
pub(crate) const TITLE: &str = "Ñandú at dawn";
const SINE_HZ: f32 = 1_000.0;

pub(crate) fn sine_frames(frames: usize, amplitude: f32) -> Vec<f32> {
    fixtures::sine(SINE_HZ, SOURCE_RATE, frames, amplitude)
}

pub(crate) fn sine(seconds: usize, amplitude: f32) -> Vec<f32> {
    sine_frames(seconds * SOURCE_RATE.hz() as usize, amplitude)
}

pub(crate) fn clicks(seconds: usize, burst_millis: usize) -> Vec<f32> {
    let burst_frames = burst_millis * SOURCE_RATE.hz() as usize / 1_000;
    let tone = sine(seconds, 1.0);
    let rate = SOURCE_RATE.hz() as usize;
    let gated = tone.iter().enumerate().map(|(index, sample)| {
        if index % rate < burst_frames {
            *sample
        } else {
            0.0
        }
    });
    gated.collect()
}

pub(crate) fn measure(samples: &[f32], rate: SampleRate) -> EbuR128 {
    let mut meter = EbuR128::new(1, rate.hz(), Mode::I | Mode::TRUE_PEAK).unwrap();
    meter.add_frames_f32(samples).unwrap();
    meter
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "the gain of a test signal is far inside the f32 range"
)]
pub(crate) fn scale_to_loudness(samples: &[f32], target_lufs: f64) -> Vec<f32> {
    let current = measure(samples, SOURCE_RATE).loudness_global().unwrap();
    let gain = 10_f64.powf((target_lufs - current) / 20.0) as f32;
    samples.iter().map(|sample| sample * gain).collect()
}

pub(crate) fn write_segments(directory: &Path, parts: &[&[f32]]) -> Vec<PathBuf> {
    parts
        .iter()
        .enumerate()
        .map(|(index, samples)| {
            let path = directory.join(format!("segment-{index}.wav"));
            fixtures::write_segment(&path, SOURCE_RATE, samples).unwrap();
            path
        })
        .collect()
}

pub(crate) fn spec(
    directory: &Path,
    segments: Vec<PathBuf>,
    format: ExportFormat,
    export_rate: ExportRate,
) -> ExportSpec {
    ExportSpec {
        segments,
        source_rate: SOURCE_RATE,
        format,
        export_rate,
        loudness: Loudness::Keep,
        tags: EpisodeTags {
            title: TITLE.to_owned(),
            voice: VoiceId::new(EngineId::new("qwen3"), "es-lucia"),
            language: Language::Es,
        },
        destination: directory.join(format!("episode.{format}")),
    }
}

pub(crate) fn run(spec: &ExportSpec) -> ExportSummary {
    export(spec, &AtomicBool::new(false), &|_| {}).unwrap()
}

pub(crate) fn read_wav(path: &Path) -> (WavSpec, Vec<f32>) {
    let mut reader = WavReader::open(path).unwrap();
    let spec = reader.spec();
    let samples = reader
        .samples::<i16>()
        .map(|sample| f32::from(sample.unwrap()) / PCM_SCALE)
        .collect();
    (spec, samples)
}
