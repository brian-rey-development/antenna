use std::fs::File;
use std::path::Path;
use std::time::Duration;

use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, FormatReader, TrackType};
use symphonia::core::io::{MediaSourceStream, MediaSourceStreamOptions};
use symphonia::core::meta::{MetadataOptions, StandardTag, Tag};
use symphonia::default::{get_codecs, get_probe};

const NANOS_PER_SECOND: u64 = 1_000_000_000;

pub(crate) struct Probe {
    pub(crate) playable_frames: u64,
    pub(crate) rate: u32,
    tags: Vec<(&'static str, String)>,
}

impl Probe {
    pub(crate) fn duration(&self) -> Duration {
        Duration::from_nanos(self.playable_frames * NANOS_PER_SECOND / u64::from(self.rate))
    }

    pub(crate) fn tag(&self, key: &str) -> Option<&str> {
        let found = self.tags.iter().find(|(name, _)| *name == key);
        found.map(|(_, value)| value.as_str())
    }
}

pub(crate) fn probe(path: &Path) -> Probe {
    let mut format = open(path);
    let track = format.default_track(TrackType::Audio).unwrap();
    let parameters = track.codec_params.as_ref().unwrap().audio().unwrap();
    let rate = parameters.sample_rate.unwrap();
    let playable_frames = track.num_frames.unwrap();
    let tags = format
        .metadata()
        .current()
        .map_or_else(Vec::new, |revision| {
            revision.media.tags.iter().filter_map(named).collect()
        });
    Probe {
        playable_frames,
        rate,
        tags,
    }
}

fn open(path: &Path) -> Box<dyn FormatReader> {
    let file = Box::new(File::open(path).unwrap());
    let source = MediaSourceStream::new(file, MediaSourceStreamOptions::default());
    let mut hint = Hint::new();
    hint.with_extension(path.extension().unwrap().to_str().unwrap());
    get_probe()
        .probe(
            &hint,
            source,
            FormatOptions::default(),
            MetadataOptions::default(),
        )
        .unwrap()
}

#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "the tests read three of the many standard tags of symphonia"
)]
fn named(tag: &Tag) -> Option<(&'static str, String)> {
    let key = match tag.std.as_ref()? {
        StandardTag::TrackTitle(_) => "title",
        StandardTag::Artist(_) => "artist",
        StandardTag::Comment(_) => "comment",
        _ => return None,
    };
    Some((key, tag.raw.value.to_string()))
}

/// Decodes all packets of the file. The decoder of symphonia removes the encoder delay and padding.
pub(crate) fn decode_mono(path: &Path) -> Vec<f32> {
    let mut format = open(path);
    let track = format.default_track(TrackType::Audio).unwrap();
    let id = track.id;
    let parameters = track.codec_params.as_ref().unwrap().audio().unwrap();
    let mut decoder = get_codecs()
        .make_audio_decoder(parameters, &AudioDecoderOptions::default())
        .unwrap();
    let (mut samples, mut block) = (Vec::new(), Vec::new());
    while let Some(packet) = format.next_packet().unwrap() {
        if packet.track_id == id {
            let buffer = decoder.decode(&packet).unwrap();
            buffer.copy_to_vec_interleaved::<f32>(&mut block);
            samples.extend_from_slice(&block);
        }
    }
    samples
}
