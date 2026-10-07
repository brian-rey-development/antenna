use std::sync::Arc;

use crate::metadata_file::write_metadata;
use crate::swap::{Replacement, Swap};
use crate::{DocumentId, DocumentMeta, Library, LibraryError};

impl Library {
    pub(crate) fn change(
        &self,
        id: DocumentId,
        mut edit: impl FnMut(&DocumentMeta) -> Option<DocumentMeta>,
    ) -> Result<(), LibraryError> {
        self.update(id, |current| {
            Ok((edit(current).map(Replacement::for_metadata), ()))
        })?;
        Ok(())
    }

    /// Runs `prepare` on the current metadata of a document and stores the replacement that it
    /// returns. The function returns the stored metadata and the output of the last `prepare` call.
    pub(crate) fn update<T>(
        &self,
        id: DocumentId,
        mut prepare: impl FnMut(&DocumentMeta) -> Result<(Option<Replacement>, T), LibraryError>,
    ) -> Result<(Arc<DocumentMeta>, T), LibraryError> {
        // The file is written before the swap, and a pass that loses the swap runs again on the
        // new metadata and writes the file again. Thus the last file write belongs to the pass
        // whose swap succeeds last, and the file and the index agree. For the same reason, a
        // pass that ran before and finds nothing to change still writes the current metadata,
        // because its earlier write can be the last one on disk.
        let mut has_written = false;
        loop {
            let current = self.index.metadata(id)?;
            let (replacement, output) = prepare(&current)?;
            let replacement = match replacement {
                Some(replacement) => replacement,
                None if has_written => Replacement::for_metadata(DocumentMeta::clone(&current)),
                None => return Ok((current, output)),
            };
            write_metadata(&self.directory(id), &replacement.metadata)?;
            if let Swap::Done(stored) = self.index.replace(&current, replacement)? {
                return Ok((stored, output));
            }
            has_written = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::mem;

    use antenna_core::{Document, Language, TextFormat};
    use tempfile::TempDir;

    use super::*;
    use crate::fixtures::at;
    use crate::metadata_file::read_metadata;

    fn library_with_document() -> (TempDir, Library, DocumentId) {
        let root = tempfile::tempdir().unwrap();
        let library = Library::open(root.path()).unwrap();
        let document = Document::new("Hello", TextFormat::Plain).unwrap();
        let id = library.create("Notes", &document, at(100)).unwrap();
        (root, library, id)
    }

    fn set_spanish(current: &DocumentMeta) -> Option<DocumentMeta> {
        (current.language != Some(Language::Es)).then(|| DocumentMeta {
            language: Some(Language::Es),
            ..current.clone()
        })
    }

    #[test]
    fn change_keeps_other_update_when_swap_lost() {
        let (_root, library, id) = library_with_document();
        let mut is_first_pass = true;

        library
            .change(id, |current| {
                if mem::take(&mut is_first_pass) {
                    library.mark_opened(id, at(500)).unwrap();
                }
                set_spanish(current)
            })
            .unwrap();

        let stored = library.index.metadata(id).unwrap();
        assert_eq!(
            (stored.opened, stored.language),
            (at(500), Some(Language::Es))
        );
        assert_eq!(read_metadata(&library.directory(id)).unwrap(), *stored);
    }

    #[test]
    fn change_writes_current_metadata_when_retry_finds_nothing_to_change() {
        let (_root, library, id) = library_with_document();
        let mut is_first_pass = true;

        library
            .change(id, |current| {
                if mem::take(&mut is_first_pass) {
                    library.mark_opened(id, at(500)).unwrap();
                    library.set_language(id, Some(Language::Es)).unwrap();
                }
                set_spanish(current)
            })
            .unwrap();

        let stored = library.index.metadata(id).unwrap();
        assert_eq!(stored.opened, at(500));
        assert_eq!(read_metadata(&library.directory(id)).unwrap(), *stored);
    }

    #[test]
    fn change_fails_when_document_directory_removed() {
        let (_root, library, id) = library_with_document();
        let directory = library.directory(id);
        fs::remove_dir_all(&directory).unwrap();

        let result = library.mark_opened(id, at(500));

        assert!(matches!(result, Err(LibraryError::Io { .. })));
        assert!(!directory.exists());
    }
}
