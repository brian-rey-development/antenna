//! Tests on the real output device through the public API.

#[cfg(test)]
#[path = "fixtures/mod.rs"]
mod fixtures;

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::path::{Path, PathBuf};
    use std::time::Duration;

    use antenna_audio::{Player, PlayerEvent, PlayerState, Track};
    use antenna_core::SampleRate;
    use flume::RecvTimeoutError;

    use crate::fixtures;

    const RATE: SampleRate = SampleRate::HZ_24000;
    const SEGMENTS: usize = 3;
    const SEGMENT_SECONDS: usize = 1;
    const AMPLITUDE: f32 = 0.05;
    const SINE_HZ: f32 = 220.0;
    const POLL: Duration = Duration::from_millis(5);
    const POLL_LIMIT: usize = 2_000;

    fn segment(directory: &Path, index: usize) -> (PathBuf, Duration) {
        let path = directory.join(format!("segment-{index}.wav"));
        let frames = SEGMENT_SECONDS * RATE.hz() as usize;
        let samples = fixtures::sine(SINE_HZ, RATE, frames, AMPLITUDE);
        fixtures::write_segment(&path, RATE, &samples).unwrap();
        (path, Duration::from_secs(SEGMENT_SECONDS as u64))
    }

    #[test]
    #[ignore = "needs audio device"]
    fn player_plays_track_when_device_available() {
        let directory = tempfile::tempdir().unwrap();
        let entries = (0..SEGMENTS).map(|index| Some(segment(directory.path(), index)));
        let player = Player::start().unwrap();
        let handle = player.load(Track::new(RATE, entries));
        handle.play();

        let mut seen = BTreeSet::new();
        for _ in 0..POLL_LIMIT {
            seen.extend(handle.position().map(|position| position.segment.get()));
            match player.events().recv_timeout(POLL) {
                Ok(PlayerEvent::StateChanged {
                    state: PlayerState::Ended,
                    ..
                }) => break,
                Ok(PlayerEvent::DeviceLost) => panic!("the player has no usable output device"),
                Ok(_) | Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => panic!("the player stopped"),
            }
        }

        assert_eq!(handle.state(), PlayerState::Ended);
        assert_eq!(seen, BTreeSet::from([0, 1, 2]));
        assert_eq!(player.underruns(), 0);
    }
}
