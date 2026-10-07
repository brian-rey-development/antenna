use std::fs;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use antenna_core::Document;
use jiff::{Timestamp, Zoned};

use crate::index::Index;
use crate::meta_file::{read_meta, write_meta};
use crate::metadata::parse_title;
use crate::period::group_by_period;
use crate::{
    DocumentId, DocumentMeta, DocumentSummary, Filter, Group, LibraryError, SegmentStore, atomic,
    fold, paths, text_hash,
};

/// The documents of the user and their segment store, in one data directory.
///
/// The library takes the time as a parameter. The functions that change a document must not run
/// at the same time on two threads, because each one reads the document, changes it and writes it.
#[derive(Debug)]
pub struct Library {
    root: PathBuf,
    pub(crate) index: Index,
    segments: SegmentStore,
    skipped: Vec<PathBuf>,
}

impl Library {
    /// Opens the library in the platform data directory, or in the directory of the environment
    /// variable `ANTENNA_DATA_DIR`.
    ///
    /// # Errors
    ///
    /// Returns [`LibraryError::NoDataDir`] if no data directory is known, and
    /// [`LibraryError::Io`] if a directory cannot be read or made.
    pub fn open_default() -> Result<Self, LibraryError> {
        Self::open(paths::data_root()?)
    }

    /// Opens the library in a data root. The function reads each `document.toml` file, and no
    /// text file. It skips each document directory that it cannot read, see [`Library::skipped`].
    ///
    /// # Errors
    ///
    /// Returns [`LibraryError::Io`] if a directory cannot be read or made.
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, LibraryError> {
        let root = root.into();
        let segments = SegmentStore::open(&root)?;
        let library_dir = paths::library_dir(&root);
        fs::create_dir_all(&library_dir).map_err(LibraryError::io(&library_dir))?;
        let index = Index::default();
        let skipped = load_documents(&library_dir, &index)?;
        Ok(Self {
            root,
            index,
            segments,
            skipped,
        })
    }

    /// Returns the document directories that `open` skipped, in path order. A directory is
    /// skipped if its `document.toml` is absent or damaged, or if its name is not the document id.
    pub fn skipped(&self) -> &[PathBuf] {
        &self.skipped
    }

    /// Returns the segment store of the library. A clone of it uses the same directory.
    pub fn segments(&self) -> &SegmentStore {
        &self.segments
    }

    /// Saves a new document and returns its id. The document starts with no voice and no segments.
    ///
    /// # Errors
    ///
    /// Returns [`LibraryError::EmptyTitle`] if the title is blank, and [`LibraryError::Io`] or
    /// [`LibraryError::MetaWrite`] if the files cannot be written.
    pub fn create(
        &self,
        title: &str,
        document: &Document,
        now: Timestamp,
    ) -> Result<DocumentId, LibraryError> {
        let title = parse_title(title)?;
        let id = DocumentId::new(now);
        let directory = self.directory(id);
        fs::create_dir_all(&directory).map_err(LibraryError::io(&directory))?;
        write_text(&directory, document)?;
        let meta = DocumentMeta {
            id,
            title,
            format: document.format(),
            language: None,
            voice: None,
            created: now,
            modified: now,
            opened: now,
            text_hash: text_hash(document),
            segments: None,
            last_export: None,
        };
        write_meta(&directory, &meta)?;
        self.index.insert(meta, Some(fold(document.text())));
        Ok(id)
    }

    /// Loads the metadata and the text of a document. If the text file is newer than the
    /// metadata, because a crash came between the two writes of `save_text`, the function stores
    /// the hash of the text. Then the status of the document is `Draft`.
    ///
    /// # Errors
    ///
    /// Returns [`LibraryError::NotFound`] if no document has the id, and [`LibraryError::Io`] if
    /// the text file is absent, is not UTF-8 or is blank.
    pub fn load(&self, id: DocumentId) -> Result<(Arc<DocumentMeta>, Document), LibraryError> {
        let meta = self.index.meta(id)?;
        let path = paths::text_path(&self.directory(id), meta.format);
        let text = fs::read_to_string(&path).map_err(LibraryError::io(&path))?;
        let document = Document::new(text, meta.format).map_err(|source| LibraryError::Io {
            path,
            source: io::Error::new(ErrorKind::InvalidData, source),
        })?;
        let hash = text_hash(&document);
        if hash == meta.text_hash {
            return Ok((meta, document));
        }
        let repaired = DocumentMeta {
            text_hash: hash,
            ..(*meta).clone()
        };
        Ok((self.commit(repaired)?, document))
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
        let meta = self.index.meta(id)?;
        let hash = text_hash(document);
        if hash == meta.text_hash {
            return Ok(());
        }
        let directory = self.directory(id);
        write_text(&directory, document)?;
        let changed = DocumentMeta {
            format: document.format(),
            text_hash: hash,
            modified: now,
            ..(*meta).clone()
        };
        self.commit(changed)?;
        self.index.set_folded_text(id, fold(document.text()));
        if meta.format == document.format() {
            return Ok(());
        }
        let old_path = paths::text_path(&directory, meta.format);
        fs::remove_file(&old_path).map_err(LibraryError::io(&old_path))
    }

    /// Deletes a document with its text. The segment files stay until garbage collection.
    ///
    /// # Errors
    ///
    /// Returns [`LibraryError::NotFound`] if no document has the id, and [`LibraryError::Io`] if
    /// the directory cannot be deleted.
    pub fn delete(&self, id: DocumentId) -> Result<(), LibraryError> {
        self.index.meta(id)?;
        let directory = self.directory(id);
        fs::remove_dir_all(&directory).map_err(LibraryError::io(&directory))?;
        self.index.remove(id);
        Ok(())
    }

    /// Reads the text files and makes the search match the text of the documents. Until this
    /// function returns, [`Library::list`] matches only the titles. The apps call it on a
    /// background thread.
    ///
    /// # Errors
    ///
    /// Returns [`LibraryError::Io`] if a text file cannot be read. The search then matches
    /// titles only.
    pub fn build_search_index(&self) -> Result<(), LibraryError> {
        let mut texts = Vec::new();
        for meta in self.index.unindexed() {
            let path = paths::text_path(&self.directory(meta.id), meta.format);
            let text = fs::read_to_string(&path).map_err(LibraryError::io(&path))?;
            texts.push((meta.id, fold(&text)));
        }
        self.index.finish_text_index(texts);
        Ok(())
    }

    /// Lists the documents that have the filter status and match each term of the query. The
    /// search ignores case and accents. The groups go from the newest period, and the documents
    /// of a group go from the newest `modified` time.
    pub fn list(&self, filter: Filter, query: &str, now: &Zoned) -> Vec<Group> {
        let terms: Vec<String> = fold(query).split_whitespace().map(str::to_owned).collect();
        group_by_period(self.index.matching(filter, &terms), now)
    }

    /// Returns the documents that the user opened last, the newest first.
    pub fn recent(&self) -> Vec<DocumentSummary> {
        self.index.recent()
    }

    /// Returns the number of documents.
    pub fn count(&self) -> usize {
        self.index.len()
    }

    pub(crate) fn directory(&self, id: DocumentId) -> PathBuf {
        paths::document_dir(&paths::library_dir(&self.root), id)
    }
}

fn write_text(directory: &Path, document: &Document) -> Result<(), LibraryError> {
    let path = paths::text_path(directory, document.format());
    atomic::write(&path, document.text().as_bytes())
}

fn load_documents(library_dir: &Path, index: &Index) -> Result<Vec<PathBuf>, LibraryError> {
    let mut skipped = Vec::new();
    let entries = fs::read_dir(library_dir).map_err(LibraryError::io(library_dir))?;
    for entry in entries {
        let directory = entry.map_err(LibraryError::io(library_dir))?.path();
        if !directory.is_dir() {
            continue;
        }
        match read_meta(&directory) {
            Ok(meta) if paths::document_dir(library_dir, meta.id) == directory => {
                index.insert(meta, None);
            }
            Ok(_) | Err(_) => skipped.push(directory),
        }
    }
    skipped.sort();
    Ok(skipped)
}
