use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use antenna_core::Document;
use jiff::{Timestamp, Zoned};

use crate::edits::parse_title;
use crate::index::Index;
use crate::metadata_file::{read_metadata, write_metadata};
use crate::period::group_by_period;
use crate::text_files::write_text;
use crate::{
    DocumentId, DocumentMeta, DocumentSummary, Filter, GcReport, Group, LibraryError, SegmentStore,
    atomic, fold, gc, paths, text_hash,
};

/// The documents of the user and their segment store, in one data directory.
///
/// The library takes the time as a parameter. Many threads can change the same document at the
/// same time. A change that finds newer data in the index runs again on the newer data, so no
/// change is lost.
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
    /// The function removes the whitespace around the title.
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
        let metadata = DocumentMeta {
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
        if let Err(error) = self.write_new(&metadata, document) {
            drop(fs::remove_dir_all(self.directory(id)));
            return Err(error);
        }
        self.index.insert(metadata, Some(fold(document.text())));
        Ok(id)
    }

    /// Deletes a document with its text. The segment files stay until garbage collection.
    ///
    /// A change that runs at the same time fails and does not make the directory again, because
    /// each write needs the directory.
    ///
    /// # Errors
    ///
    /// Returns [`LibraryError::NotFound`] if no document has the id, and [`LibraryError::Io`] if
    /// the directory cannot be deleted.
    pub fn delete(&self, id: DocumentId) -> Result<(), LibraryError> {
        self.index.metadata(id)?;
        let directory = self.directory(id);
        fs::remove_dir_all(&directory).map_err(LibraryError::io(&directory))?;
        self.index.remove(id);
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

    /// Deletes each segment file that no document uses and that is older than one hour at `now`.
    /// It also deletes each temporary file in the directories `library` and `segments` that is
    /// older than one hour at `now`.
    /// The age limit protects the segments of a job that started but did not record its keys, and
    /// the files of another process. The apps call this function on a background thread at start.
    ///
    /// # Errors
    ///
    /// Returns the first [`LibraryError::Io`] of a directory that cannot be read or of a file
    /// that cannot be deleted. The function still handles the other files.
    pub fn collect_garbage(&self, now: SystemTime) -> Result<GcReport, LibraryError> {
        let directories = [
            paths::library_dir(&self.root),
            paths::segments_dir(&self.root),
        ];
        gc::collect(&directories, || self.index.used_keys(), now)
    }

    pub(crate) fn directory(&self, id: DocumentId) -> PathBuf {
        paths::document_dir(&paths::library_dir(&self.root), id)
    }

    fn write_new(&self, metadata: &DocumentMeta, document: &Document) -> Result<(), LibraryError> {
        let directory = self.directory(metadata.id);
        fs::create_dir_all(&directory).map_err(LibraryError::io(&directory))?;
        atomic::sync_parent(&paths::library_dir(&self.root))?;
        write_text(&directory, document)?;
        write_metadata(&directory, metadata)
    }
}

fn load_documents(library_dir: &Path, index: &Index) -> Result<Vec<PathBuf>, LibraryError> {
    let mut skipped = Vec::new();
    let entries = fs::read_dir(library_dir).map_err(LibraryError::io(library_dir))?;
    for entry in entries {
        let directory = entry.map_err(LibraryError::io(library_dir))?.path();
        if !directory.is_dir() {
            continue;
        }
        match read_metadata(&directory) {
            Ok(metadata) if paths::document_dir(library_dir, metadata.id) == directory => {
                index.insert(metadata, None);
            }
            Ok(_) | Err(_) => skipped.push(directory),
        }
    }
    skipped.sort();
    Ok(skipped)
}
