//! Builders that the unit tests of several modules share.

use std::time::Duration;

use antenna_core::{ExportFormat, Language, Quality, TextFormat, TextHash};
use jiff::Timestamp;

use crate::{DocumentId, DocumentMeta, ExportRecord, SegmentList, StoredVoice};

pub(crate) fn at(seconds: i64) -> Timestamp {
    Timestamp::from_second(seconds).unwrap()
}

pub(crate) fn voice(quality: Quality) -> StoredVoice {
    StoredVoice {
        id: "fake/en-alba".to_owned(),
        quality,
    }
}

pub(crate) fn bare_metadata() -> DocumentMeta {
    DocumentMeta {
        id: DocumentId::new(at(1_000)),
        title: "Notes".to_owned(),
        format: TextFormat::Plain,
        language: None,
        voice: None,
        created: at(1_000),
        modified: at(1_000),
        opened: at(1_000),
        text_hash: TextHash::new([7; 32]),
        segments: None,
        last_export: None,
    }
}

pub(crate) fn full_metadata() -> DocumentMeta {
    let keys = ["a", "b"].map(|digit| digit.repeat(64).parse().unwrap());
    DocumentMeta {
        format: TextFormat::Markdown,
        language: Some(Language::Es),
        voice: Some(voice(Quality::Max)),
        segments: Some(SegmentList {
            text_hash: TextHash::new([7; 32]),
            voice: voice(Quality::Max),
            keys: keys.to_vec(),
            is_complete: true,
            duration: Duration::new(61, 500),
        }),
        last_export: Some(ExportRecord {
            format: ExportFormat::Ogg,
            at: at(2_000),
        }),
        ..bare_metadata()
    }
}

pub(crate) fn ready_metadata() -> DocumentMeta {
    DocumentMeta {
        id: DocumentId::new(at(100)),
        voice: Some(voice(Quality::Balanced)),
        created: at(100),
        modified: at(100),
        opened: at(100),
        text_hash: TextHash::new([1; 32]),
        segments: Some(SegmentList {
            text_hash: TextHash::new([1; 32]),
            voice: voice(Quality::Balanced),
            keys: Vec::new(),
            is_complete: true,
            duration: Duration::from_secs(5),
        }),
        ..bare_metadata()
    }
}
