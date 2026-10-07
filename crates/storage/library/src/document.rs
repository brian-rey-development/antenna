use std::fmt::{self, Display, Formatter};
use std::str::FromStr;
use std::time::Duration;

use antenna_core::{Document, ExportFormat, Language, Quality, TextFormat, TextHash, VoiceId};
use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;
use uuid::timestamp::context::NoContext;

use crate::SegmentKey;
use crate::meta_file::as_text;

/// The identifier of a document. It is a UUID v7, so identifiers sort by creation time.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DocumentId(Uuid);

impl DocumentId {
    pub(crate) fn new(now: Timestamp) -> Self {
        let seconds = now.as_second().max(0).unsigned_abs();
        let nanos = now.subsec_nanosecond().max(0).unsigned_abs();
        Self(Uuid::new_v7(uuid::Timestamp::from_unix(
            NoContext, seconds, nanos,
        )))
    }

    /// Returns the UUID of the identifier.
    pub fn uuid(self) -> Uuid {
        self.0
    }
}

impl Display for DocumentId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, formatter)
    }
}

impl FromStr for DocumentId {
    type Err = uuid::Error;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        text.parse().map(Self)
    }
}

/// The voice and the quality of a document, as they are stored.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredVoice {
    /// The text form of a voice id, for example "qwen3/es-lucia".
    pub id: String,
    /// The quality of the voice.
    #[serde(with = "as_text")]
    pub quality: Quality,
}

impl StoredVoice {
    /// Makes the stored form of a voice and a quality.
    pub fn new(voice: VoiceId, quality: Quality) -> Self {
        Self {
            id: voice.to_string(),
            quality,
        }
    }
}

/// The segments of a document that a job synthesized for one text and one voice.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SegmentList {
    /// The text hash of the document when the job started.
    #[serde(with = "as_text")]
    pub text_hash: TextHash,
    /// The voice of the job.
    pub voice: StoredVoice,
    /// The segment keys in document order.
    #[serde(with = "as_text::list")]
    pub keys: Vec<SegmentKey>,
    /// `true` if the segment store has all segments.
    pub is_complete: bool,
    /// The duration of all segments, or zero while the job runs.
    pub duration: Duration,
}

/// The last export of a document.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportRecord {
    /// The format of the export file.
    #[serde(with = "as_text")]
    pub format: ExportFormat,
    /// The time of the export.
    pub at: Timestamp,
}

/// The stored data of a document, without its text.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentMeta {
    /// The identifier, also the name of the document directory.
    pub id: DocumentId,
    /// The title that the user sees. It is never blank.
    pub title: String,
    /// The format of the text file.
    #[serde(with = "as_text")]
    pub format: TextFormat,
    /// The language of the text, if the app identified it.
    #[serde(default, with = "as_text::option")]
    pub language: Option<Language>,
    /// The voice of the document, if the user selected one.
    pub voice: Option<StoredVoice>,
    /// The time of creation.
    pub created: Timestamp,
    /// The time of the last change of the text, the title or the voice.
    pub modified: Timestamp,
    /// The time of the last opening.
    pub opened: Timestamp,
    /// The hash of the text file that this metadata describes.
    #[serde(with = "as_text")]
    pub text_hash: TextHash,
    /// The segments of the last job for the current voice.
    pub segments: Option<SegmentList>,
    /// The last export.
    pub last_export: Option<ExportRecord>,
}

impl DocumentMeta {
    pub(crate) fn is_for_job(&self, text_hash: TextHash, voice: &StoredVoice) -> bool {
        self.text_hash == text_hash && self.voice.as_ref() == Some(voice)
    }
}

/// Returns the SHA-256 of the format and the text of a document.
pub fn text_hash(document: &Document) -> TextHash {
    let digest = Sha256::new()
        .chain_update(document.format().to_string())
        .chain_update(document.text());
    TextHash::new(digest.finalize().into())
}

#[cfg(test)]
mod tests {
    use antenna_core::EngineId;

    use super::*;

    fn at(seconds: i64) -> Timestamp {
        Timestamp::from_second(seconds).unwrap()
    }

    #[test]
    fn document_ids_sort_by_creation_time() {
        let ids = [3_000, 1_000, 2_000].map(|seconds| DocumentId::new(at(seconds)));

        let mut sorted = ids;
        sorted.sort();

        assert_eq!(sorted, [ids[1], ids[2], ids[0]]);
    }

    #[test]
    fn document_id_round_trips_when_text_parsed() {
        let id = DocumentId::new(at(1_000));

        let parsed: DocumentId = id.to_string().parse().unwrap();

        assert_eq!(parsed, id);
        assert_eq!(id.to_string(), id.uuid().hyphenated().to_string());
        assert!("not-a-uuid".parse::<DocumentId>().is_err());
    }

    #[test]
    fn text_hash_differs_when_format_or_text_differs() {
        let plain = Document::new("Hello", TextFormat::Plain).unwrap();
        let markdown = Document::new("Hello", TextFormat::Markdown).unwrap();
        let other = Document::new("Hello!", TextFormat::Plain).unwrap();

        assert_ne!(text_hash(&plain), text_hash(&markdown));
        assert_ne!(text_hash(&plain), text_hash(&other));
        assert_eq!(text_hash(&plain), text_hash(&plain.clone()));
    }

    #[test]
    fn stored_voice_keeps_display_text_of_voice_id() {
        let voice = VoiceId::new(EngineId::new("qwen3"), "es-lucia");

        let stored = StoredVoice::new(voice, Quality::Balanced);

        assert_eq!(stored.id, "qwen3/es-lucia");
        assert_eq!(stored.quality, Quality::Balanced);
    }
}
