use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use flume::{Receiver, RecvTimeoutError, Sender};

use super::atomics::PlayerAtomics;
use super::command::{FeedCommand, send};
use super::current::CurrentTrack;
use super::stream::{Stream, StreamParts};
use super::{PlayerEvent, PlayerState};
use crate::AudioError;

pub(crate) const FLUSH_WAIT_LIMIT: Duration = Duration::from_millis(100);
const FEED_POLL: Duration = Duration::from_millis(10);

pub(super) type OpenStream<S> = Box<dyn FnMut() -> Result<StreamParts<S>, AudioError>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WaitMode {
    Poll(Duration),
    Block,
}

/// The feed loop can push more audio at once, or it has no audio to push.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Work {
    Pending,
    Idle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Flow {
    Continue,
    Stop,
}

fn wait_mode(state: PlayerState, work: Work) -> WaitMode {
    match (state, work) {
        (PlayerState::Playing, Work::Pending) => WaitMode::Poll(Duration::ZERO),
        (PlayerState::Playing, Work::Idle) => WaitMode::Poll(FEED_POLL),
        (PlayerState::Paused | PlayerState::Buffering | PlayerState::Ended, _) => WaitMode::Block,
    }
}

pub(crate) struct Feed<S> {
    pub(super) commands: Receiver<FeedCommand>,
    pub(super) events: Sender<PlayerEvent>,
    pub(super) atomics: Arc<PlayerAtomics>,
    pub(super) flush_wait: Duration,
    pub(super) open_stream: OpenStream<S>,
    pub(super) backlog: VecDeque<FeedCommand>,
    pub(super) stream: Option<Stream<S>>,
    pub(super) track: Option<CurrentTrack>,
    pub(super) block: Vec<f32>,
    pub(super) converted: Vec<f32>,
}

impl<S> Feed<S> {
    pub(crate) fn new(
        commands: Receiver<FeedCommand>,
        events: Sender<PlayerEvent>,
        atomics: Arc<PlayerAtomics>,
        flush_wait: Duration,
        open_stream: OpenStream<S>,
    ) -> Self {
        Self {
            commands,
            events,
            atomics,
            flush_wait,
            open_stream,
            backlog: VecDeque::new(),
            stream: None,
            track: None,
            block: Vec::new(),
            converted: Vec::new(),
        }
    }

    pub(crate) fn run(mut self) {
        loop {
            if self.drain_commands() == Flow::Stop {
                return;
            }
            let work = self.step();
            let mode = wait_mode(self.state(), work);
            if self.wait(mode) == Flow::Stop {
                return;
            }
        }
    }

    pub(super) fn state(&self) -> PlayerState {
        self.track
            .as_ref()
            .map_or(PlayerState::Paused, |track| track.state)
    }

    pub(super) fn is_active(&self) -> bool {
        matches!(self.state(), PlayerState::Playing | PlayerState::Buffering)
    }

    pub(super) fn transition(&mut self, state: PlayerState) {
        let Some(track) = self.track.as_mut().filter(|track| track.state != state) else {
            return;
        };
        track.state = state;
        track.atomics().set_state(state);
        let id = track.id();
        self.publish_flags();
        send(&self.events, PlayerEvent::StateChanged { track: id, state });
    }

    /// Sets the flags of the callback from the state of the track.
    pub(super) fn publish_flags(&self) {
        let state = self.state();
        let is_silent = matches!(state, PlayerState::Paused | PlayerState::Ended);
        self.atomics.paused.store(is_silent, Ordering::Relaxed);
        let is_buffering = state == PlayerState::Buffering;
        self.atomics
            .is_buffering
            .store(is_buffering, Ordering::Relaxed);
    }

    fn drain_commands(&mut self) -> Flow {
        while let Some(command) = self.next_command() {
            if self.apply(command) == Flow::Stop {
                return Flow::Stop;
            }
        }
        Flow::Continue
    }

    fn next_command(&mut self) -> Option<FeedCommand> {
        self.backlog
            .pop_front()
            .or_else(|| self.commands.try_recv().ok())
    }

    fn wait(&mut self, mode: WaitMode) -> Flow {
        let received = match mode {
            WaitMode::Poll(delay) => self.commands.recv_timeout(delay),
            WaitMode::Block => self
                .commands
                .recv()
                .map_err(|flume::RecvError::Disconnected| RecvTimeoutError::Disconnected),
        };
        match received {
            Ok(command) => {
                self.backlog.push_back(command);
                Flow::Continue
            }
            Err(RecvTimeoutError::Timeout) => Flow::Continue,
            Err(RecvTimeoutError::Disconnected) => Flow::Stop,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::harness::{Harness, RATE, track};
    use super::*;
    use crate::test_support::pcm_ramp;

    #[test]
    fn wait_mode_blocks_when_not_playing() {
        for state in [
            PlayerState::Paused,
            PlayerState::Buffering,
            PlayerState::Ended,
        ] {
            assert_eq!(
                wait_mode(state, Work::Pending),
                WaitMode::Block,
                "{state:?}"
            );
            assert_eq!(wait_mode(state, Work::Idle), WaitMode::Block, "{state:?}");
        }
    }

    #[test]
    fn wait_mode_polls_without_delay_when_playing_and_work_pending() {
        let mode = wait_mode(PlayerState::Playing, Work::Pending);

        assert_eq!(mode, WaitMode::Poll(Duration::ZERO));
    }

    #[test]
    fn wait_mode_polls_for_refill_when_playing_and_idle() {
        let mode = wait_mode(PlayerState::Playing, Work::Idle);

        assert_eq!(mode, WaitMode::Poll(FEED_POLL));
    }

    #[test]
    fn feed_ignores_command_when_track_replaced() {
        let directory = tempfile::tempdir().unwrap();
        let harness = Harness::start(RATE);
        let old = harness.load(1, track(directory.path(), RATE, &[Some(&pcm_ramp(0, 200))]));
        let current = harness.load(
            2,
            track(directory.path(), RATE, &[Some(&pcm_ramp(0, 9_000))]),
        );
        harness.play(current.id);
        let mut consumer = harness.next_stream();
        harness.wait_for(current.id, PlayerState::Playing);

        harness.pause(old.id);
        harness.barrier(&mut consumer, current.id);

        assert_eq!(current.atomics.state(), PlayerState::Playing);
    }
}
