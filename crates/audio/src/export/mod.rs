mod loudness;
mod mp3;
mod ogg;
mod part_file;
mod progress;
mod spec;
mod wav;

use std::sync::atomic::AtomicBool;

use antenna_core::{ExportFormat, SampleRate};

pub use spec::{EpisodeTags, ExportRate, ExportSpec, ExportSummary, Loudness};

use mp3::Mp3Encoder;
use ogg::OggEncoder;
use part_file::PartFile;
use progress::Progress;
use wav::WavEncoder;

use crate::AudioError;
use crate::resample::Conversion;
use crate::segment_file::read_segment_at_rate;

const ARTIST: &str = "Antenna";

enum Encoder<'a> {
    Mp3(Mp3Encoder<'a>),
    Wav(WavEncoder<'a>),
    Ogg(Box<OggEncoder<'a>>),
}

impl<'a> Encoder<'a> {
    fn new(
        spec: &ExportSpec,
        rate: SampleRate,
        part: &'a mut PartFile,
    ) -> Result<Self, AudioError> {
        Ok(match spec.format {
            ExportFormat::Mp3 => Self::Mp3(Mp3Encoder::new(part, rate, &spec.tags)?),
            ExportFormat::Wav => Self::Wav(WavEncoder::new(part, rate)?),
            ExportFormat::Ogg => Self::Ogg(Box::new(OggEncoder::new(part, rate, &spec.tags)?)),
        })
    }

    fn write(&mut self, samples: &[f32]) -> Result<(), AudioError> {
        match self {
            Self::Mp3(encoder) => encoder.write(samples),
            Self::Wav(encoder) => encoder.write(samples),
            Self::Ogg(encoder) => encoder.write(samples),
        }
    }

    fn finish(self) -> Result<(), AudioError> {
        match self {
            Self::Mp3(encoder) => encoder.finish(),
            Self::Wav(encoder) => encoder.finish(),
            Self::Ogg(encoder) => encoder.finish(),
        }
    }
}

/// Writes stored segments to a file.
///
/// The export reads one segment at a time. With loudness normalization it has two passes over
/// the segments, so the memory does not grow with the length of the document. It writes to
/// `<destination>.part`. At the end, it renames the file. After each segment, it calls
/// `progress` with a value from 0.0 to 1.0.
///
/// # Errors
///
/// Returns [`AudioError::Cancelled`] if `cancel` is set after a segment. Returns another
/// [`AudioError`] if a segment cannot be read, a segment has another sample rate than
/// `source_rate`, or the file cannot be written. An export that fails leaves no file and no
/// `.part` file.
pub fn export(
    spec: &ExportSpec,
    cancel: &AtomicBool,
    progress: &dyn Fn(f32),
) -> Result<ExportSummary, AudioError> {
    let steps = spec.segments.len() * spec.loudness.passes();
    let mut progress = Progress::new(cancel, progress, steps);
    let gain_db = match spec.loudness {
        Loudness::Normalize => {
            loudness::measure_gain(&spec.segments, spec.source_rate, &mut progress)?
        }
        Loudness::Keep => 0.0,
    };
    let mut part = PartFile::open(&spec.destination)?;
    let source_samples = encode(
        spec,
        loudness::linear_gain(gain_db),
        &mut part,
        &mut progress,
    )?;
    Ok(ExportSummary {
        duration: spec.source_rate.duration_of(source_samples),
        gain_db,
        bytes: part.commit()?,
    })
}

fn encode(
    spec: &ExportSpec,
    amplitude: f32,
    part: &mut PartFile,
    progress: &mut Progress<'_>,
) -> Result<u64, AudioError> {
    let rate = encoder_rate(spec);
    let mut encoder = Encoder::new(spec, rate, part)?;
    let mut conversion = Conversion::new(spec.source_rate, rate);
    let mut converted = Vec::new();
    let mut source_samples = 0;
    for path in &spec.segments {
        let samples = read_segment_at_rate(path, spec.source_rate)?;
        source_samples += encode_segment(
            samples,
            amplitude,
            &mut conversion,
            &mut encoder,
            &mut converted,
        )?;
        progress.complete_step()?;
    }
    converted.clear();
    conversion.finish(&mut converted);
    encoder.write(&converted)?;
    encoder.finish()?;
    Ok(source_samples)
}

fn encode_segment(
    mut samples: Vec<f32>,
    amplitude: f32,
    conversion: &mut Conversion,
    encoder: &mut Encoder<'_>,
    converted: &mut Vec<f32>,
) -> Result<u64, AudioError> {
    for sample in &mut samples {
        *sample *= amplitude;
    }
    converted.clear();
    conversion.convert(&samples, converted);
    encoder.write(converted)?;
    Ok(samples.len() as u64)
}

fn encoder_rate(spec: &ExportSpec) -> SampleRate {
    match spec.format {
        ExportFormat::Ogg => ogg::encoder_rate(spec.export_rate),
        ExportFormat::Mp3 | ExportFormat::Wav => spec.export_rate.sample_rate(),
    }
}
