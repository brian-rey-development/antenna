use std::mem;
use std::path::PathBuf;

use antenna_core::SampleRate;
use ogg::writing::{PacketWriteEndInfo, PacketWriter};
use opus::{Application, Bitrate, Channels, Encoder};

use super::part_file::{PartFile, write_error};
use super::{ARTIST, EpisodeTags, ExportRate};
use crate::AudioError;

const OPUS_BITRATE_BPS: i32 = 48_000;
const OPUS_FRAME_MS: u32 = 20;
const OGG_GRANULE_RATE_HZ: u64 = 48_000;
// A fixed serial number makes the file the same for the same input.
const OGG_STREAM_SERIAL: u32 = 1;
// 50 frames of 20 ms make a page of one second.
const OGG_FRAMES_PER_PAGE: u64 = 50;
const OPUS_HEAD_VERSION: u8 = 1;
const OPUS_MONO: u8 = 1;
const OPUS_OUTPUT_GAIN: i16 = 0;
const OPUS_MAPPING_FAMILY: u8 = 0;
const OPUS_VENDOR: &str = "antenna-audio";
const OPUS_MAX_PACKET_BYTES: usize = 4_000;
const MILLIS_PER_SECOND: u32 = 1_000;

/// Returns the encoder rate for an export rate. Opus has no 44.1 kHz mode.
pub(super) fn encoder_rate(rate: ExportRate) -> SampleRate {
    match rate {
        ExportRate::Hz24000 => SampleRate::HZ_24000,
        ExportRate::Hz44100 | ExportRate::Hz48000 => SampleRate::HZ_48000,
    }
}

pub(super) struct OggEncoder<'a> {
    encoder: Encoder,
    writer: PacketWriter<'static, &'a mut PartFile>,
    path: PathBuf,
    frame_samples: usize,
    granules_per_sample: u64,
    pre_skip: u64,
    pending: Vec<f32>,
    packet: Vec<u8>,
    held: Option<Vec<u8>>,
    packets: u64,
    samples: u64,
}

impl<'a> OggEncoder<'a> {
    pub(super) fn new(
        part: &'a mut PartFile,
        rate: SampleRate,
        tags: &EpisodeTags,
    ) -> Result<Self, AudioError> {
        let path = part.destination().to_owned();
        let mut encoder = Encoder::new(rate.hz(), Channels::Mono, Application::Audio)
            .map_err(AudioError::Opus)?;
        encoder
            .set_bitrate(Bitrate::Bits(OPUS_BITRATE_BPS))
            .map_err(AudioError::Opus)?;
        let granules_per_sample = OGG_GRANULE_RATE_HZ / u64::from(rate.hz());
        let lookahead = encoder.get_lookahead().map_err(AudioError::Opus)?;
        let pre_skip = u64::try_from(lookahead).unwrap_or(0) * granules_per_sample;
        let mut this = Self {
            encoder,
            writer: PacketWriter::new(part),
            path,
            frame_samples: (rate.hz() * OPUS_FRAME_MS / MILLIS_PER_SECOND) as usize,
            granules_per_sample,
            pre_skip,
            pending: Vec::new(),
            packet: vec![0; OPUS_MAX_PACKET_BYTES],
            held: None,
            packets: 0,
            samples: 0,
        };
        this.write_headers(rate, tags)?;
        Ok(this)
    }

    fn write_headers(&mut self, rate: SampleRate, tags: &EpisodeTags) -> Result<(), AudioError> {
        let head = opus_head(self.pre_skip, rate);
        self.write_packet(head, PacketWriteEndInfo::EndPage, 0)?;
        self.write_packet(opus_tags(tags), PacketWriteEndInfo::EndPage, 0)
    }

    fn write_packet(
        &mut self,
        packet: Vec<u8>,
        end: PacketWriteEndInfo,
        granule: u64,
    ) -> Result<(), AudioError> {
        self.writer
            .write_packet(packet, OGG_STREAM_SERIAL, end, granule)
            .map_err(|source| write_error(&self.path, source))
    }

    fn encode_frame(&mut self, frame: &[f32]) -> Result<(), AudioError> {
        let length = self
            .encoder
            .encode_float(frame, &mut self.packet)
            .map_err(AudioError::Opus)?;
        let packet = self.packet.iter().take(length).copied().collect();
        self.packets += 1;
        self.queue(packet)
    }

    fn encode_pending(&mut self) -> Result<(), AudioError> {
        let pending = mem::take(&mut self.pending);
        let result = self.encode_frame(&pending);
        self.pending = pending;
        self.pending.clear();
        result
    }

    fn queue(&mut self, packet: Vec<u8>) -> Result<(), AudioError> {
        let Some(previous) = self.held.replace(packet) else {
            return Ok(());
        };
        let granule = (self.packets - 1) * self.frame_granules();
        let is_page_end = (self.packets - 1).is_multiple_of(OGG_FRAMES_PER_PAGE);
        let end = if is_page_end {
            PacketWriteEndInfo::EndPage
        } else {
            PacketWriteEndInfo::NormalPacket
        };
        self.write_packet(previous, end, granule)
    }

    fn frame_granules(&self) -> u64 {
        self.frame_samples as u64 * self.granules_per_sample
    }
}

impl OggEncoder<'_> {
    pub(super) fn write(&mut self, samples: &[f32]) -> Result<(), AudioError> {
        self.samples += samples.len() as u64;
        let mut rest = samples;
        if !self.pending.is_empty() {
            let missing = self.frame_samples - self.pending.len();
            let (head, tail) = rest.split_at(missing.min(rest.len()));
            self.pending.extend_from_slice(head);
            rest = tail;
            if self.pending.len() < self.frame_samples {
                return Ok(());
            }
            self.encode_pending()?;
        }
        let mut frames = rest.chunks_exact(self.frame_samples);
        for frame in frames.by_ref() {
            self.encode_frame(frame)?;
        }
        self.pending.extend_from_slice(frames.remainder());
        Ok(())
    }

    pub(super) fn finish(mut self) -> Result<(), AudioError> {
        // RFC 7845 section 4. The granule position counts samples at 48 kHz from the start of
        // the decoder output, so the pre-skip is part of the last granule position.
        let end_granule = self.pre_skip + self.samples * self.granules_per_sample;
        while self.packets * self.frame_granules() < end_granule || self.held.is_none() {
            self.pending.resize(self.frame_samples, 0.0);
            self.encode_pending()?;
        }
        let last = self.held.take().unwrap_or_default();
        self.write_packet(last, PacketWriteEndInfo::EndStream, end_granule)
    }
}

fn opus_head(pre_skip: u64, rate: SampleRate) -> Vec<u8> {
    let mut head = b"OpusHead".to_vec();
    head.push(OPUS_HEAD_VERSION);
    head.push(OPUS_MONO);
    head.extend_from_slice(&u16::try_from(pre_skip).unwrap_or(u16::MAX).to_le_bytes());
    head.extend_from_slice(&rate.hz().to_le_bytes());
    head.extend_from_slice(&OPUS_OUTPUT_GAIN.to_le_bytes());
    head.push(OPUS_MAPPING_FAMILY);
    head
}

fn opus_tags(tags: &EpisodeTags) -> Vec<u8> {
    let comments = [
        format!("TITLE={}", tags.title),
        format!("ARTIST={ARTIST}"),
        format!("LANGUAGE={}", tags.language),
    ];
    let mut packet = b"OpusTags".to_vec();
    push_text(&mut packet, OPUS_VENDOR);
    packet.extend_from_slice(&length_field(comments.len()));
    for comment in &comments {
        push_text(&mut packet, comment);
    }
    packet
}

fn push_text(packet: &mut Vec<u8>, text: &str) {
    packet.extend_from_slice(&length_field(text.len()));
    packet.extend_from_slice(text.as_bytes());
}

fn length_field(length: usize) -> [u8; 4] {
    u32::try_from(length).unwrap_or(u32::MAX).to_le_bytes()
}
