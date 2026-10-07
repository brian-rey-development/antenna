use std::num::{NonZeroU16, NonZeroUsize};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use antenna_core::SampleRate;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, ErrorKind, FromSample, SampleFormat, SizedSample, Stream, StreamConfig};
use flume::Sender;
use rtrb::{Consumer, RingBuffer};

use super::PlayerEvent;
use super::atomics::PlayerAtomics;
use super::callback::fill;
use super::command::{FeedCommand, send};
use super::stream::StreamParts;
use crate::AudioError;
use crate::frames::sample_count;

const RING_BUFFER_DURATION_MS: u64 = 2_000;

/// Opens the default output device and starts a stream that plays the ring buffer.
pub(crate) fn open_default(
    atomics: &Arc<PlayerAtomics>,
    commands: &Sender<FeedCommand>,
    events: &Sender<PlayerEvent>,
) -> Result<StreamParts<Stream>, AudioError> {
    let device = cpal::default_host()
        .default_output_device()
        .ok_or(AudioError::NoDevice)?;
    let supported = device
        .default_output_config()
        .map_err(AudioError::DeviceConfig)?;
    let config = supported.config();
    let (channels, rate) = parse_config(&config)?;
    let (producer, consumer) = RingBuffer::new(ring_buffer_frames(rate));
    let playback = Playback {
        channels,
        consumer,
        atomics: Arc::clone(atomics),
    };
    let on_error = error_handler(atomics, commands, events);
    let stream = playback.build(&device, supported.sample_format(), config, on_error)?;
    stream.play().map_err(AudioError::PlayStream)?;
    Ok(StreamParts {
        stream,
        rate,
        producer,
    })
}

fn parse_config(config: &StreamConfig) -> Result<(NonZeroUsize, SampleRate), AudioError> {
    let invalid = || AudioError::InvalidDeviceConfig {
        channels: config.channels,
        sample_rate: config.sample_rate,
    };
    let channels = NonZeroU16::new(config.channels).ok_or_else(invalid)?;
    let rate = SampleRate::try_from(config.sample_rate)
        .ok()
        .ok_or_else(invalid)?;
    Ok((NonZeroUsize::from(channels), rate))
}

fn ring_buffer_frames(rate: SampleRate) -> usize {
    sample_count(Duration::from_millis(RING_BUFFER_DURATION_MS), rate)
}

struct Playback {
    channels: NonZeroUsize,
    consumer: Consumer<f32>,
    atomics: Arc<PlayerAtomics>,
}

impl Playback {
    #[expect(
        clippy::wildcard_enum_match_arm,
        reason = "cpal::SampleFormat is non_exhaustive, so a new format is unsupported until a stage adds it"
    )]
    fn build(
        self,
        device: &Device,
        format: SampleFormat,
        config: StreamConfig,
        on_error: impl FnMut(cpal::Error) + Send + 'static,
    ) -> Result<Stream, AudioError> {
        match format {
            SampleFormat::F32 => self.build_typed::<f32>(device, config, on_error),
            SampleFormat::I16 => self.build_typed::<i16>(device, config, on_error),
            SampleFormat::U16 => self.build_typed::<u16>(device, config, on_error),
            SampleFormat::I32 => self.build_typed::<i32>(device, config, on_error),
            other => Err(AudioError::UnsupportedFormat(other)),
        }
    }

    fn build_typed<T>(
        mut self,
        device: &Device,
        config: StreamConfig,
        on_error: impl FnMut(cpal::Error) + Send + 'static,
    ) -> Result<Stream, AudioError>
    where
        T: SizedSample + FromSample<f32>,
    {
        let callback = move |output: &mut [T], _: &cpal::OutputCallbackInfo| {
            fill(output, self.channels, &mut self.consumer, &self.atomics);
        };
        device
            .build_output_stream(config, callback, on_error, None)
            .map_err(AudioError::BuildStream)
    }
}

fn error_handler(
    atomics: &Arc<PlayerAtomics>,
    commands: &Sender<FeedCommand>,
    events: &Sender<PlayerEvent>,
) -> impl FnMut(cpal::Error) + Send + 'static {
    let (atomics, commands, events) = (Arc::clone(atomics), commands.clone(), events.clone());
    move |error| on_stream_error(error.kind(), &atomics, &commands, &events)
}

/// Handles an error of the output stream. It runs on a thread of the audio library.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "cpal::ErrorKind is non_exhaustive, and the other kinds need no action"
)]
pub(crate) fn on_stream_error(
    kind: ErrorKind,
    atomics: &PlayerAtomics,
    commands: &Sender<FeedCommand>,
    events: &Sender<PlayerEvent>,
) {
    match kind {
        ErrorKind::DeviceChanged => send(events, PlayerEvent::DeviceChanged),
        ErrorKind::DeviceNotAvailable => {
            atomics.paused.store(true, Ordering::Relaxed);
            send(events, PlayerEvent::DeviceLost);
            send(commands, FeedCommand::DeviceLost);
        }
        other => tracing::warn!(kind = ?other, "the output stream reported an error"),
    }
}

#[cfg(test)]
mod tests {
    use rtrb::Consumer;
    use tempfile::TempDir;

    use super::super::PlayerState;
    use super::super::harness::{Harness, RATE, TestTrack};
    use super::*;

    fn playing() -> (Harness, TestTrack, Consumer<f32>, TempDir) {
        let directory = tempfile::tempdir().unwrap();
        let harness = Harness::start(RATE);
        let (loaded, mut consumer) = harness.play_two_segments(directory.path());
        harness.drain_samples(&mut consumer, 10);
        harness.wait_for(loaded.id, PlayerState::Playing);
        (harness, loaded, consumer, directory)
    }

    fn report(harness: &Harness, kind: ErrorKind) {
        on_stream_error(
            kind,
            &harness.atomics,
            &harness.commands,
            &harness.event_sender,
        );
    }

    #[test]
    fn device_lost_pauses_track_when_device_not_available() {
        let (harness, loaded, _consumer, _directory) = playing();

        report(&harness, ErrorKind::DeviceNotAvailable);

        let seen = harness.wait_for(loaded.id, PlayerState::Paused);
        assert!(seen.contains(&PlayerEvent::DeviceLost));
        assert_eq!(loaded.atomics.state(), PlayerState::Paused);
    }

    #[test]
    fn device_lost_sets_paused_flag_when_device_not_available() {
        let (harness, _loaded, _consumer, _directory) = playing();

        report(&harness, ErrorKind::DeviceNotAvailable);

        assert!(harness.atomics.paused.load(Ordering::Relaxed));
    }

    #[test]
    fn device_changed_keeps_state_when_stream_continues() {
        let (harness, loaded, mut consumer, _directory) = playing();

        report(&harness, ErrorKind::DeviceChanged);
        harness.barrier(&mut consumer, loaded.id);

        assert_eq!(harness.next_event(), PlayerEvent::DeviceChanged);
        assert!(harness.events.try_recv().is_err());
        assert_eq!(loaded.atomics.state(), PlayerState::Playing);
    }

    #[test]
    fn stream_error_changes_nothing_when_kind_is_other() {
        let (harness, loaded, mut consumer, _directory) = playing();

        report(&harness, ErrorKind::Xrun);
        harness.barrier(&mut consumer, loaded.id);

        assert!(harness.events.try_recv().is_err());
        assert_eq!(loaded.atomics.state(), PlayerState::Playing);
    }
}
