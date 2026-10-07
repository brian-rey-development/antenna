use std::time::Duration;

use antenna_core::{ExportFormat, Language, TextHash};
use jiff::Timestamp;

use crate::{
    DocumentId, DocumentMeta, ExportRecord, Library, LibraryError, SegmentKey, SegmentList,
    StoredVoice,
};

impl Library {
    /// Changes the title of a document. The function removes the whitespace around the title.
    ///
    /// # Errors
    ///
    /// Returns [`LibraryError::EmptyTitle`] if the title is blank, [`LibraryError::NotFound`] if no
    /// document has the id, and [`LibraryError::Io`] or [`LibraryError::MetaWrite`] if
    /// `document.toml` cannot be written.
    pub fn rename(&self, id: DocumentId, title: &str, now: Timestamp) -> Result<(), LibraryError> {
        let title = parse_title(title)?;
        self.change(id, |metadata| {
            Some(DocumentMeta {
                title: title.clone(),
                modified: now,
                ..metadata.clone()
            })
        })
    }

    /// Sets the voice of a document. A different voice removes the segment list, so the audio of
    /// the old voice never gives the status `Ready`.
    ///
    /// # Errors
    ///
    /// Returns [`LibraryError::NotFound`] if no document has the id, and [`LibraryError::Io`] or
    /// [`LibraryError::MetaWrite`] if `document.toml` cannot be written.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "the stage plan fixes this signature, and the retry loop clones the voice"
    )]
    pub fn set_voice(
        &self,
        id: DocumentId,
        voice: StoredVoice,
        now: Timestamp,
    ) -> Result<(), LibraryError> {
        self.change(id, |metadata| {
            if metadata.voice.as_ref() == Some(&voice) {
                return None;
            }
            Some(DocumentMeta {
                voice: Some(voice.clone()),
                segments: None,
                modified: now,
                ..metadata.clone()
            })
        })
    }

    /// Sets the language of a document. The apps identify the language with `antenna-text`.
    ///
    /// # Errors
    ///
    /// Returns [`LibraryError::NotFound`] if no document has the id, and [`LibraryError::Io`] or
    /// [`LibraryError::MetaWrite`] if `document.toml` cannot be written.
    pub fn set_language(
        &self,
        id: DocumentId,
        language: Option<Language>,
    ) -> Result<(), LibraryError> {
        self.change(id, |metadata| {
            (metadata.language != language).then(|| DocumentMeta {
                language,
                ..metadata.clone()
            })
        })
    }

    /// Records the segment keys of a job that started. If the text hash or the voice is not the
    /// one of the document, the function does nothing. If the document has the same keys already,
    /// it keeps the list and its completion flag.
    ///
    /// # Errors
    ///
    /// Returns [`LibraryError::NotFound`] if no document has the id, and [`LibraryError::Io`] or
    /// [`LibraryError::MetaWrite`] if `document.toml` cannot be written.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "the stage plan fixes this signature, and the retry loop clones the keys"
    )]
    pub fn record_segments(
        &self,
        id: DocumentId,
        text_hash: TextHash,
        voice: &StoredVoice,
        keys: Vec<SegmentKey>,
    ) -> Result<(), LibraryError> {
        self.change(id, |metadata| {
            if !metadata.is_for_job(text_hash, voice) {
                return None;
            }
            let is_recorded = metadata
                .segments
                .as_ref()
                .is_some_and(|list| list.is_for(text_hash, voice) && list.keys == keys);
            if is_recorded {
                return None;
            }
            let segments = SegmentList {
                text_hash,
                voice: voice.clone(),
                keys: keys.clone(),
                is_complete: false,
                duration: Duration::ZERO,
            };
            Some(DocumentMeta {
                segments: Some(segments),
                ..metadata.clone()
            })
        })
    }

    /// Marks the segments of a finished job as complete. If the text hash or the voice is not the
    /// one of the document or of its segment list, the function does nothing.
    ///
    /// # Errors
    ///
    /// Returns [`LibraryError::NotFound`] if no document has the id, and [`LibraryError::Io`] or
    /// [`LibraryError::MetaWrite`] if `document.toml` cannot be written.
    pub fn record_complete(
        &self,
        id: DocumentId,
        text_hash: TextHash,
        voice: &StoredVoice,
        duration: Duration,
    ) -> Result<(), LibraryError> {
        self.change(id, |metadata| {
            let list = metadata.segments.as_ref()?;
            if !list.is_for(text_hash, voice) || !metadata.is_for_job(text_hash, voice) {
                return None;
            }
            let segments = SegmentList {
                is_complete: true,
                duration,
                ..list.clone()
            };
            Some(DocumentMeta {
                segments: Some(segments),
                ..metadata.clone()
            })
        })
    }

    /// Records the last export of a document.
    ///
    /// # Errors
    ///
    /// Returns [`LibraryError::NotFound`] if no document has the id, and [`LibraryError::Io`] or
    /// [`LibraryError::MetaWrite`] if `document.toml` cannot be written.
    pub fn record_export(
        &self,
        id: DocumentId,
        format: ExportFormat,
        now: Timestamp,
    ) -> Result<(), LibraryError> {
        self.change(id, |metadata| {
            Some(DocumentMeta {
                last_export: Some(ExportRecord { format, at: now }),
                ..metadata.clone()
            })
        })
    }

    /// Records the time when the user opened a document.
    ///
    /// # Errors
    ///
    /// Returns [`LibraryError::NotFound`] if no document has the id, and [`LibraryError::Io`] or
    /// [`LibraryError::MetaWrite`] if `document.toml` cannot be written.
    pub fn mark_opened(&self, id: DocumentId, now: Timestamp) -> Result<(), LibraryError> {
        self.change(id, |metadata| {
            Some(DocumentMeta {
                opened: now,
                ..metadata.clone()
            })
        })
    }

    /// Sets the progress of a job that runs, as the completed and the total segments, or `None`
    /// when the job ends. The progress is in memory only. A call with an unknown id does nothing.
    pub fn set_progress(&self, id: DocumentId, progress: Option<(u32, u32)>) {
        self.index.set_progress(id, progress);
    }
}

pub(crate) fn parse_title(title: &str) -> Result<String, LibraryError> {
    match title.trim() {
        "" => Err(LibraryError::EmptyTitle),
        trimmed => Ok(trimmed.to_owned()),
    }
}
