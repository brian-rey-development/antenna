use crate::TrackId;

/// The state of the current track of the player.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum PlayerState {
    /// The track does not play. The play position stays.
    Paused,
    /// The track plays.
    Playing,
    /// The track waits for the audio of a segment that is not ready.
    Buffering,
    /// The track played its last sample.
    Ended,
}

/// A message from the player to the app.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayerEvent {
    /// The state of a track changed.
    StateChanged {
        /// The track that changed.
        track: TrackId,
        /// The new state.
        state: PlayerState,
    },
    /// The default output device changed. The stream continues on the new device.
    DeviceChanged,
    /// The output device is not available. The player paused and dropped its stream.
    DeviceLost,
}
