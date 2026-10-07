use std::sync::Arc;

use antenna_core::ExportFormat;

use crate::DocumentMeta;

/// The state of a document, derived from its stored data and from the progress of a job that runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    /// Not all segments of the current text and voice are stored, and no job runs.
    Draft,
    /// A job runs for the document.
    Generating {
        /// The number of segments that the job completed.
        done: u32,
        /// The number of segments of the document.
        total: u32,
    },
    /// All segments of the current text and voice are stored.
    Ready,
    /// The last export is not older than the last change.
    Exported(ExportFormat),
}

/// A selection of documents by status.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Filter {
    /// Each document.
    All,
    /// The documents with the status `Ready` or `Exported`.
    Ready,
    /// The documents with the status `Draft`.
    Drafts,
}

impl Filter {
    pub(crate) fn is_selected(self, status: Status) -> bool {
        match self {
            Self::All => true,
            Self::Ready => matches!(status, Status::Ready | Status::Exported(_)),
            Self::Drafts => status == Status::Draft,
        }
    }
}

/// A document with its status, as the Library screen lists it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocumentSummary {
    /// The stored data of the document.
    pub meta: Arc<DocumentMeta>,
    /// The status of the document.
    pub status: Status,
}

pub(crate) fn status(metadata: &DocumentMeta, progress: Option<(u32, u32)>) -> Status {
    if let Some((done, total)) = progress {
        return Status::Generating { done, total };
    }
    if !has_current_audio(metadata) {
        return Status::Draft;
    }
    match &metadata.last_export {
        Some(record) if record.at >= metadata.modified => Status::Exported(record.format),
        Some(_) | None => Status::Ready,
    }
}

fn has_current_audio(metadata: &DocumentMeta) -> bool {
    match (&metadata.segments, &metadata.voice) {
        (Some(list), Some(voice)) => {
            list.is_complete && list.text_hash == metadata.text_hash && list.voice == *voice
        }
        (Some(_) | None, None) | (None, Some(_)) => false,
    }
}

#[cfg(test)]
mod tests {
    use antenna_core::{Quality, TextHash};

    use super::*;
    use crate::ExportRecord;
    use crate::SegmentList;
    use crate::fixtures::{at, ready_metadata, voice};

    fn with_segments(change: impl FnOnce(&mut SegmentList)) -> DocumentMeta {
        let mut metadata = ready_metadata();
        change(metadata.segments.as_mut().unwrap());
        metadata
    }

    fn exported_at(seconds: i64) -> DocumentMeta {
        DocumentMeta {
            last_export: Some(ExportRecord {
                format: ExportFormat::Wav,
                at: at(seconds),
            }),
            ..ready_metadata()
        }
    }

    #[test]
    fn status_is_generating_when_progress_set() {
        let status = status(&ready_metadata(), Some((2, 9)));

        assert_eq!(status, Status::Generating { done: 2, total: 9 });
    }

    #[test]
    fn status_is_draft_when_segments_absent() {
        let metadata = DocumentMeta {
            segments: None,
            ..ready_metadata()
        };

        let status = status(&metadata, None);

        assert_eq!(status, Status::Draft);
    }

    #[test]
    fn status_is_draft_when_text_changed() {
        let metadata = with_segments(|list| list.text_hash = TextHash::new([2; 32]));

        let status = status(&metadata, None);

        assert_eq!(status, Status::Draft);
    }

    #[test]
    fn status_is_draft_when_segments_incomplete() {
        let metadata = with_segments(|list| list.is_complete = false);

        let status = status(&metadata, None);

        assert_eq!(status, Status::Draft);
    }

    #[test]
    fn status_is_draft_when_segments_voice_differs() {
        let metadata = with_segments(|list| list.voice = voice(Quality::Max));

        let status = status(&metadata, None);

        assert_eq!(status, Status::Draft);
    }

    #[test]
    fn status_is_draft_when_voice_absent() {
        let metadata = DocumentMeta {
            voice: None,
            ..ready_metadata()
        };

        let status = status(&metadata, None);

        assert_eq!(status, Status::Draft);
    }

    #[test]
    fn status_is_exported_when_export_after_change() {
        let status = status(&exported_at(101), None);

        assert_eq!(status, Status::Exported(ExportFormat::Wav));
    }

    #[test]
    fn status_is_exported_when_export_time_equals_change_time() {
        let status = status(&exported_at(100), None);

        assert_eq!(status, Status::Exported(ExportFormat::Wav));
    }

    #[test]
    fn status_is_ready_when_export_before_change() {
        let status = status(&exported_at(99), None);

        assert_eq!(status, Status::Ready);
    }

    #[test]
    fn status_is_ready_when_complete_and_not_exported() {
        let status = status(&ready_metadata(), None);

        assert_eq!(status, Status::Ready);
    }

    #[test]
    fn status_is_draft_when_export_exists_and_text_changed() {
        let metadata = DocumentMeta {
            text_hash: TextHash::new([3; 32]),
            ..exported_at(200)
        };

        let status = status(&metadata, None);

        assert_eq!(status, Status::Draft);
    }
}
