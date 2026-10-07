use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use antenna_core::SegmentIndex;
use flume::Sender;

use super::atomics::TrackAtomics;
use crate::{Track, TrackId, TrackPosition};

#[derive(Debug)]
pub(crate) enum FeedCommand {
    Load {
        id: TrackId,
        track: Track,
        atomics: Arc<TrackAtomics>,
    },
    Play {
        id: TrackId,
    },
    Pause {
        id: TrackId,
    },
    Seek {
        id: TrackId,
        position: TrackPosition,
    },
    Stored {
        id: TrackId,
        segment: SegmentIndex,
        path: PathBuf,
        duration: Duration,
    },
    LiveBegin {
        id: TrackId,
        segment: SegmentIndex,
    },
    LiveChunk {
        id: TrackId,
        samples: Vec<f32>,
    },
    LiveEnd {
        id: TrackId,
    },
    DeviceLost,
    Shutdown,
}

impl FeedCommand {
    pub(crate) fn track(&self) -> Option<TrackId> {
        match self {
            Self::Play { id }
            | Self::Pause { id }
            | Self::Seek { id, .. }
            | Self::Stored { id, .. }
            | Self::LiveBegin { id, .. }
            | Self::LiveChunk { id, .. }
            | Self::LiveEnd { id } => Some(*id),
            Self::Load { .. } | Self::DeviceLost | Self::Shutdown => None,
        }
    }
}

pub(crate) fn send<T>(sender: &Sender<T>, message: T) {
    if sender.send(message).is_err() {
        tracing::debug!("the receiver is gone, so the message is dropped");
    }
}
