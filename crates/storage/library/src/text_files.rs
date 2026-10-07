use std::fs;
use std::path::Path;
use std::sync::Arc;

use antenna_core::Document;
use jiff::Timestamp;

use crate::swap::Replacement;
use crate::{DocumentId, DocumentMeta, Library, LibraryError, atomic, fold, paths, text_hash};

impl Library {
    /// Loads the metadata and the text of a document.
    ///
    /// A crash between the two writes of `save_text` leaves a text file that is newer than the
    /// metadata. Then the function stores the hash of the text, so the status is `Draft`, and it
    /// updates the folded text of the search index.
    ///
    /// # Errors
    ///
    /// Returns [`LibraryError::NotFound`] if no document has the id, [`LibraryError::Io`] if the
    /// text file is absent or is not UTF-8, and [`LibraryError::EmptyText`] if it is blank.
    pub fn load(&self, id: DocumentId) -> Result<(Arc<DocumentMeta>, Document), LibraryError> {
        self.update(id, |current| {
            let document = self.read_document(current)?;
            Ok((repair(current, &document), document))
        })
    }

    /// Saves the text of a document. If the text and the format did not change, the function does
    /// nothing. The function writes the text file first and `document.toml` second.
    ///
    /// # Errors
    ///
    /// Returns [`LibraryError::NotFound`] if no document has the id, and [`LibraryError::Io`] or
    /// [`LibraryError::MetaWrite`] if the files cannot be written.
    pub fn save_text(
        &self,
        id: DocumentId,
        document: &Document,
        now: Timestamp,
    ) -> Result<(), LibraryError> {
        let hash = text_hash(document);
        let folded_text: Arc<str> = fold(document.text()).into();
        let directory = self.directory(id);
        let (_, previous_format) = self.update(id, |current| {
            if current.text_hash == hash {
                return Ok((None, current.format));
            }
            write_text(&directory, document)?;
            let replacement = replace_text(current, document, &folded_text, now);
            Ok((Some(replacement), current.format))
        })?;
        if previous_format != document.format() {
            // The metadata names the format, so nothing reads the old file. If the removal fails,
            // the file stays unused, and a later save in that format replaces it.
            drop(fs::remove_file(paths::text_path(
                &directory,
                previous_format,
            )));
        }
        Ok(())
    }

    /// Reads the text files and makes the search match the text of the documents. Until this
    /// function returns, [`Library::list`] matches only the titles. The apps call it on a
    /// background thread.
    ///
    /// # Errors
    ///
    /// Returns the first [`LibraryError::Io`] of a text file that cannot be read. The function
    /// still indexes the other documents. The search matches only the title of the failed one.
    pub fn build_search_index(&self) -> Result<(), LibraryError> {
        let mut texts = Vec::new();
        let mut first_error = None;
        for metadata in self.index.unindexed() {
            let path = paths::text_path(&self.directory(metadata.id), metadata.format);
            match fs::read_to_string(&path) {
                Ok(text) => texts.push((metadata.id, fold(&text))),
                Err(source) => {
                    first_error.get_or_insert(LibraryError::Io { path, source });
                }
            }
        }
        self.index.finish_text_index(texts);
        first_error.map_or(Ok(()), Err)
    }

    fn read_document(&self, current: &DocumentMeta) -> Result<Document, LibraryError> {
        let path = paths::text_path(&self.directory(current.id), current.format);
        let text = fs::read_to_string(&path).map_err(LibraryError::io(&path))?;
        Document::new(text, current.format)
            .map_err(|source| LibraryError::EmptyText { path, source })
    }
}

fn replace_text(
    current: &DocumentMeta,
    document: &Document,
    folded_text: &Arc<str>,
    now: Timestamp,
) -> Replacement {
    let metadata = DocumentMeta {
        format: document.format(),
        text_hash: text_hash(document),
        modified: now,
        ..current.clone()
    };
    Replacement {
        metadata,
        folded_text: Some(Arc::clone(folded_text)),
    }
}

fn repair(current: &DocumentMeta, document: &Document) -> Option<Replacement> {
    let hash = text_hash(document);
    (hash != current.text_hash).then(|| Replacement {
        metadata: DocumentMeta {
            text_hash: hash,
            ..current.clone()
        },
        folded_text: Some(fold(document.text()).into()),
    })
}

pub(crate) fn write_text(directory: &Path, document: &Document) -> Result<(), LibraryError> {
    let file_name = paths::text_file_name(document.format());
    atomic::write(directory, file_name, document.text().as_bytes())
}
