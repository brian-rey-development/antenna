use std::sync::Arc;
use std::time::Duration;

use antenna_core::{ExportFormat, Language, TextHash};
use jiff::Timestamp;

use crate::meta_file::write_meta;
use crate::{
    DocumentId, DocumentMeta, ExportRecord, Library, LibraryError, SegmentKey, SegmentList,
    StoredVoice,
};

impl Library {
    /// Changes the title of a document.
    ///
    /// # Errors
    ///
    /// Returns [`LibraryError::EmptyTitle`] if the title is blank, [`LibraryError::NotFound`] if no
    /// document has the id, and [`LibraryError::Io`] or [`LibraryError::MetaWrite`] if
    /// `document.toml` cannot be written.
    pub fn rename(&self, id: DocumentId, title: &str, now: Timestamp) -> Result<(), LibraryError> {
        let title = parse_title(title)?;
        self.change(id, |meta| {
            Some(DocumentMeta {
                title,
                modified: now,
                ..meta.clone()
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
    pub fn set_voice(
        &self,
        id: DocumentId,
        voice: StoredVoice,
        now: Timestamp,
    ) -> Result<(), LibraryError> {
        self.change(id, |meta| {
            if meta.voice.as_ref() == Some(&voice) {
                return None;
            }
            Some(DocumentMeta {
                voice: Some(voice),
                segments: None,
                modified: now,
                ..meta.clone()
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
        self.change(id, |meta| {
            (meta.language != language).then(|| DocumentMeta {
                language,
                ..meta.clone()
            })
        })
    }

    /// Records the segment keys of a job that started. If the text hash or the voice is not the
    /// one of the document, the function does nothing.
    ///
    /// # Errors
    ///
    /// Returns [`LibraryError::NotFound`] if no document has the id, and [`LibraryError::Io`] or
    /// [`LibraryError::MetaWrite`] if `document.toml` cannot be written.
    pub fn record_segments(
        &self,
        id: DocumentId,
        text_hash: TextHash,
        voice: &StoredVoice,
        keys: Vec<SegmentKey>,
    ) -> Result<(), LibraryError> {
        self.change(id, |meta| {
            if !meta.is_for_job(text_hash, voice) {
                return None;
            }
            let segments = SegmentList {
                text_hash,
                voice: voice.clone(),
                keys,
                is_complete: false,
                duration: Duration::ZERO,
            };
            Some(DocumentMeta {
                segments: Some(segments),
                ..meta.clone()
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
        self.change(id, |meta| {
            let list = meta.segments.as_ref()?;
            let is_current = list.text_hash == text_hash && list.voice == *voice;
            if !is_current || !meta.is_for_job(text_hash, voice) {
                return None;
            }
            let segments = SegmentList {
                is_complete: true,
                duration,
                ..list.clone()
            };
            Some(DocumentMeta {
                segments: Some(segments),
                ..meta.clone()
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
        self.change(id, |meta| {
            Some(DocumentMeta {
                last_export: Some(ExportRecord { format, at: now }),
                ..meta.clone()
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
        self.change(id, |meta| {
            Some(DocumentMeta {
                opened: now,
                ..meta.clone()
            })
        })
    }

    /// Sets the progress of a running job, as the completed and the total segments, or `None`
    /// when the job ends. The progress is in memory only. A call with an unknown id does nothing.
    pub fn set_progress(&self, id: DocumentId, progress: Option<(u32, u32)>) {
        self.index.set_progress(id, progress);
    }

    pub(crate) fn commit(&self, meta: DocumentMeta) -> Result<Arc<DocumentMeta>, LibraryError> {
        write_meta(&self.directory(meta.id), &meta)?;
        self.index.replace(meta)
    }

    pub(crate) fn change(
        &self,
        id: DocumentId,
        edit: impl FnOnce(&DocumentMeta) -> Option<DocumentMeta>,
    ) -> Result<(), LibraryError> {
        let meta = self.index.meta(id)?;
        if let Some(changed) = edit(&meta) {
            self.commit(changed)?;
        }
        Ok(())
    }
}

pub(crate) fn parse_title(title: &str) -> Result<String, LibraryError> {
    match title.trim() {
        "" => Err(LibraryError::EmptyTitle),
        trimmed => Ok(trimmed.to_owned()),
    }
}
