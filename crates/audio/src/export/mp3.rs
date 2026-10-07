use std::io::{Seek, SeekFrom, Write};
use std::num::NonZeroU32;

use antenna_core::SampleRate;
use mp3lame_encoder::{
    Bitrate, BuildError, Builder, Encoder, FlushGap, Id3Tag, Mode, MonoPcm, Quality,
    max_required_buffer_size,
};

use super::part_file::{PartFile, write_error};
use super::{ARTIST, EpisodeTags};
use crate::AudioError;

const MP3_BITRATE: Bitrate = Bitrate::Kbps64;
const MP3_LAME_QUALITY: Quality = Quality::NearBest;
// The LAME documentation asks for at least 7200 bytes for the final flush.
const MP3_FLUSH_BYTES: usize = 7_200;

pub(super) struct Mp3Encoder<'a> {
    encoder: Encoder,
    part: &'a mut PartFile,
    buffer: Vec<u8>,
}

impl<'a> Mp3Encoder<'a> {
    pub(super) fn new(
        part: &'a mut PartFile,
        rate: SampleRate,
        tags: &EpisodeTags,
    ) -> Result<Self, AudioError> {
        Ok(Self {
            encoder: build_encoder(rate, tags)?,
            part,
            buffer: Vec::new(),
        })
    }

    fn write_buffer(&mut self) -> Result<(), AudioError> {
        self.part
            .write_all(&self.buffer)
            .map_err(|source| write_error(self.part.destination(), source))
    }
}

impl Mp3Encoder<'_> {
    pub(super) fn write(&mut self, samples: &[f32]) -> Result<(), AudioError> {
        self.buffer.clear();
        self.buffer.reserve(max_required_buffer_size(samples.len()));
        self.encoder
            .encode_to_vec(MonoPcm(samples), &mut self.buffer)
            .map_err(AudioError::Mp3Encode)?;
        self.write_buffer()
    }

    pub(super) fn finish(mut self) -> Result<(), AudioError> {
        // FlushGap pads the last frame. FlushNoGap keeps the end of the audio in the encoder,
        // because it is for a file that continues.
        self.buffer.clear();
        self.buffer.reserve(MP3_FLUSH_BYTES);
        self.encoder
            .flush_to_vec::<FlushGap>(&mut self.buffer)
            .map_err(AudioError::Mp3Encode)?;
        self.write_buffer()?;
        self.write_lame_tag()
    }
}

impl Mp3Encoder<'_> {
    fn write_lame_tag(&mut self) -> Result<(), AudioError> {
        self.buffer.clear();
        self.buffer.reserve(self.encoder.lame_tag_size());
        // The tag frame replaces the empty first frame, which follows the ID3 tag.
        let offset = self.encoder.id3v2_tag_size() as u64;
        if self
            .encoder
            .lame_tag_encode_to_vec(&mut self.buffer)
            .is_none()
        {
            return Ok(());
        }
        let destination = self.part.destination().to_owned();
        self.part
            .seek(SeekFrom::Start(offset))
            .map_err(|source| write_error(&destination, source))?;
        self.write_buffer()
    }
}

fn configure(builder: &mut Builder, rate: SampleRate) -> Result<(), BuildError> {
    builder.set_num_channels(1)?;
    builder.set_sample_rate(rate.hz())?;
    // Without this, LAME can pick an output rate that differs from the input rate.
    builder.set_output_sample_rate(NonZeroU32::new(rate.hz()))?;
    builder.set_mode(Mode::Mono)?;
    builder.set_brate(MP3_BITRATE)?;
    builder.set_quality(MP3_LAME_QUALITY)
}

fn build_encoder(rate: SampleRate, tags: &EpisodeTags) -> Result<Encoder, AudioError> {
    let mut builder = Builder::new().ok_or(AudioError::Mp3Build(BuildError::NoMem))?;
    configure(&mut builder, rate).map_err(AudioError::Mp3Build)?;
    let title = latin1(&tags.title);
    let comment = latin1(&format!("{} {}", tags.language, tags.voice));
    let id3 = Id3Tag {
        title: &title,
        artist: ARTIST.as_bytes(),
        album: b"",
        album_art: b"",
        year: b"",
        comment: &comment,
    };
    builder.set_id3_tag(id3).map_err(AudioError::Mp3Tag)?;
    builder.build().map_err(AudioError::Mp3Build)
}

fn latin1(text: &str) -> Vec<u8> {
    text.chars()
        .map(|character| u8::try_from(u32::from(character)).unwrap_or(b'?'))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latin1_keeps_accents_when_characters_fit() {
        assert_eq!(latin1("Ñandú"), [0xD1, b'a', b'n', b'd', 0xFA]);
    }

    #[test]
    fn latin1_replaces_character_when_outside_latin1() {
        assert_eq!(latin1("a\u{4e2d}"), *b"a?");
    }
}
