mod advance;
mod apply;
mod atomics;
mod callback;
mod command;
mod current;
mod device;
mod event;
mod feed;
#[cfg(test)]
pub(crate) mod harness;
mod seek;
mod source;
mod stream;

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use antenna_core::{SampleRate, SegmentIndex};
use flume::{Receiver, Sender};

pub use event::{PlayerEvent, PlayerState};

use atomics::{PlayerAtomics, TrackAtomics};
pub(crate) use command::{FeedCommand, send};
use feed::{FLUSH_WAIT_LIMIT, Feed};

use crate::{AudioError, LiveOutput, Track, TrackId, TrackPosition, Volume};

/// The player of the audio. It plays one track at a time on the default output device.
///
/// A drop of the player stops its thread and waits for the thread. The wait can take as long as
/// the open of the output device.
#[derive(Debug)]
pub struct Player {
    commands: Sender<FeedCommand>,
    events: Receiver<PlayerEvent>,
    atomics: Arc<PlayerAtomics>,
    next_track: AtomicU64,
    thread: Option<JoinHandle<()>>,
}

impl Player {
    /// Starts the thread `antenna-player`. The player opens the output device at the first
    /// `play` of a track.
    ///
    /// # Errors
    ///
    /// Returns [`AudioError::SpawnThread`] if the operating system cannot start the thread.
    pub fn start() -> Result<Self, AudioError> {
        let (commands, command_receiver) = flume::unbounded();
        let (event_sender, events) = flume::unbounded();
        let atomics = Arc::new(PlayerAtomics::new());
        let thread = spawn_feed(command_receiver, commands.clone(), event_sender, &atomics)?;
        Ok(Self {
            commands,
            events,
            atomics,
            next_track: AtomicU64::new(0),
            thread: Some(thread),
        })
    }

    /// Replaces the current track with a new track. The old track pauses, and the new track
    /// starts paused at its first segment.
    pub fn load(&self, track: Track) -> TrackHandle {
        let id = TrackId::new(self.next_track.fetch_add(1, Ordering::Relaxed));
        let rate = track.rate();
        let atomics = Arc::new(TrackAtomics::new(track.segment_count()));
        let load = FeedCommand::Load {
            id,
            track,
            atomics: Arc::clone(&atomics),
        };
        send(&self.commands, load);
        TrackHandle {
            commands: self.commands.clone(),
            atomics,
            player: Arc::clone(&self.atomics),
            id,
            rate,
        }
    }

    /// Sets the volume of the output.
    pub fn set_volume(&self, volume: Volume) {
        self.atomics
            .volume_bits
            .store(volume.to_bits(), Ordering::Relaxed);
    }

    /// Returns the number of callbacks that found the ring buffer too short while playing.
    pub fn underruns(&self) -> u64 {
        self.atomics.underruns.load(Ordering::Relaxed)
    }

    /// Returns the receiver of the events of the player.
    pub fn events(&self) -> &Receiver<PlayerEvent> {
        &self.events
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        send(&self.commands, FeedCommand::Shutdown);
        if let Some(thread) = self.thread.take()
            && thread.join().is_err()
        {
            tracing::error!("the thread of the player panicked");
        }
    }
}

fn spawn_feed(
    receiver: Receiver<FeedCommand>,
    commands: Sender<FeedCommand>,
    events: Sender<PlayerEvent>,
    atomics: &Arc<PlayerAtomics>,
) -> Result<JoinHandle<()>, AudioError> {
    let atomics = Arc::clone(atomics);
    thread::Builder::new()
        .name("antenna-player".to_owned())
        .spawn(move || {
            let open = {
                let (atomics, events) = (Arc::clone(&atomics), events.clone());
                move || device::open_default(&atomics, &commands, &events)
            };
            Feed::new(receiver, events, atomics, FLUSH_WAIT_LIMIT, Box::new(open)).run();
        })
        .map_err(AudioError::SpawnThread)
}

/// The control of one track of a [`Player`]. A new track from [`Player::load`] makes the
/// commands of an old handle have no effect.
#[derive(Debug)]
pub struct TrackHandle {
    commands: Sender<FeedCommand>,
    atomics: Arc<TrackAtomics>,
    player: Arc<PlayerAtomics>,
    id: TrackId,
    rate: SampleRate,
}

impl TrackHandle {
    /// Returns the id that [`PlayerEvent::StateChanged`] uses for this track.
    pub fn id(&self) -> TrackId {
        self.id
    }

    /// Plays the track from the play position. After the track ended, a `seek` must come first,
    /// because `play` does nothing in the state `Ended`.
    pub fn play(&self) {
        send(&self.commands, FeedCommand::Play { id: self.id });
    }

    /// Pauses the track. The play position stays.
    pub fn pause(&self) {
        send(&self.commands, FeedCommand::Pause { id: self.id });
    }

    /// Moves the play position. An offset after the end of a stored segment is the end of it.
    pub fn seek(&self, position: TrackPosition) {
        send(
            &self.commands,
            FeedCommand::Seek {
                id: self.id,
                position,
            },
        );
    }

    /// Returns the play position, or `None` until the first `play` of the track gives it one.
    pub fn position(&self) -> Option<TrackPosition> {
        self.atomics.position(&self.player)
    }

    /// Returns the state of the track.
    pub fn state(&self) -> PlayerState {
        self.atomics.state()
    }

    /// Tells the player that a segment is now in the segment store. The player replaces the live
    /// audio of the segment with the file, and the output does not change.
    pub fn set_stored(&self, segment: SegmentIndex, path: PathBuf, duration: Duration) {
        let id = self.id;
        send(
            &self.commands,
            FeedCommand::Stored {
                id,
                segment,
                path,
                duration,
            },
        );
    }

    /// Returns an output that sends the chunks of a job to this track.
    pub fn live_output(&self) -> LiveOutput {
        LiveOutput::new(self.commands.clone(), self.id, self.rate)
    }
}

#[cfg(test)]
mod tests {
    use flume::RecvError;

    use super::*;

    fn track() -> Track {
        Track::new(SampleRate::HZ_24000, [None])
    }

    #[test]
    fn player_stops_thread_when_dropped() {
        let player = Player::start().unwrap();
        let events = player.events().clone();

        drop(player);

        assert_eq!(events.recv(), Err(RecvError::Disconnected));
    }

    #[test]
    fn player_gives_each_track_a_new_id_when_loaded() {
        let player = Player::start().unwrap();

        let first = player.load(track());
        let second = player.load(track());

        assert_ne!(first.id(), second.id());
    }

    #[test]
    fn track_handle_is_paused_without_position_when_loaded() {
        let player = Player::start().unwrap();

        let handle = player.load(track());

        assert_eq!(handle.state(), PlayerState::Paused);
        assert_eq!(handle.position(), None);
    }

    #[test]
    fn player_applies_volume_when_set() {
        let player = Player::start().unwrap();

        player.set_volume(Volume::new(0.5));

        let bits = player.atomics.volume_bits.load(Ordering::Relaxed);
        assert_eq!(bits, 0.5_f32.to_bits());
    }

    #[test]
    fn player_reports_no_underruns_when_nothing_played() {
        let player = Player::start().unwrap();

        assert_eq!(player.underruns(), 0);
    }
}
