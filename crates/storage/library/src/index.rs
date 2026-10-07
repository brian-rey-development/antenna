#![expect(
    clippy::disallowed_types,
    reason = "the library index is the one permitted lock"
)]

use std::cmp::Reverse;
use std::collections::{BTreeMap, HashSet};
use std::sync::{Arc, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::search::{self, fold};
use crate::status::status;
use crate::{DocumentId, DocumentMeta, DocumentSummary, Filter, LibraryError, SegmentKey};

/// The number of documents that `recent` returns.
pub const RECENT_LIMIT: usize = 4;

#[derive(Debug)]
struct Entry {
    meta: Arc<DocumentMeta>,
    folded_title: String,
    folded_text: Option<String>,
    progress: Option<(u32, u32)>,
}

impl Entry {
    fn summary(&self) -> DocumentSummary {
        DocumentSummary {
            meta: Arc::clone(&self.meta),
            status: status(&self.meta, self.progress),
        }
    }

    fn matches(&self, terms: &[String], text_index: TextIndex) -> bool {
        let text = match text_index {
            TextIndex::Built => self.folded_text.as_deref().unwrap_or_default(),
            TextIndex::Pending => "",
        };
        search::matches(terms, &self.folded_title, text)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum TextIndex {
    #[default]
    Pending,
    Built,
}

#[derive(Debug, Default)]
struct State {
    entries: BTreeMap<DocumentId, Entry>,
    text_index: TextIndex,
}

/// The documents in memory. Each method holds the lock for the time of one memory operation.
#[derive(Debug, Default)]
pub(crate) struct Index {
    state: RwLock<State>,
}

impl Index {
    fn read(&self) -> RwLockReadGuard<'_, State> {
        self.state.read().unwrap_or_else(PoisonError::into_inner)
    }

    fn write(&self) -> RwLockWriteGuard<'_, State> {
        self.state.write().unwrap_or_else(PoisonError::into_inner)
    }

    pub(crate) fn insert(&self, meta: DocumentMeta, folded_text: Option<String>) {
        let entry = Entry {
            folded_title: fold(&meta.title),
            meta: Arc::new(meta),
            folded_text,
            progress: None,
        };
        self.write().entries.insert(entry.meta.id, entry);
    }

    pub(crate) fn meta(&self, id: DocumentId) -> Result<Arc<DocumentMeta>, LibraryError> {
        let state = self.read();
        let entry = state.entries.get(&id).ok_or(LibraryError::NotFound(id))?;
        Ok(Arc::clone(&entry.meta))
    }

    pub(crate) fn replace(&self, meta: DocumentMeta) -> Result<Arc<DocumentMeta>, LibraryError> {
        let id = meta.id;
        let folded_title = fold(&meta.title);
        let mut state = self.write();
        let entry = state
            .entries
            .get_mut(&id)
            .ok_or(LibraryError::NotFound(id))?;
        entry.meta = Arc::new(meta);
        entry.folded_title = folded_title;
        Ok(Arc::clone(&entry.meta))
    }

    pub(crate) fn remove(&self, id: DocumentId) {
        self.write().entries.remove(&id);
    }

    pub(crate) fn set_progress(&self, id: DocumentId, progress: Option<(u32, u32)>) {
        if let Some(entry) = self.write().entries.get_mut(&id) {
            entry.progress = progress;
        }
    }

    pub(crate) fn set_folded_text(&self, id: DocumentId, folded_text: String) {
        if let Some(entry) = self.write().entries.get_mut(&id) {
            entry.folded_text = Some(folded_text);
        }
    }

    pub(crate) fn unindexed(&self) -> Vec<Arc<DocumentMeta>> {
        let state = self.read();
        let entries = state.entries.values();
        entries
            .filter(|entry| entry.folded_text.is_none())
            .map(|entry| Arc::clone(&entry.meta))
            .collect()
    }

    pub(crate) fn finish_text_index(&self, texts: Vec<(DocumentId, String)>) {
        let mut state = self.write();
        for (id, folded_text) in texts {
            if let Some(entry) = state.entries.get_mut(&id) {
                entry.folded_text.get_or_insert(folded_text);
            }
        }
        state.text_index = TextIndex::Built;
    }

    pub(crate) fn matching(&self, filter: Filter, terms: &[String]) -> Vec<DocumentSummary> {
        let state = self.read();
        let entries = state.entries.values();
        entries
            .filter(|entry| entry.matches(terms, state.text_index))
            .map(Entry::summary)
            .filter(|summary| filter.keeps(summary.status))
            .collect()
    }

    pub(crate) fn recent(&self) -> Vec<DocumentSummary> {
        let state = self.read();
        let mut entries: Vec<&Entry> = state.entries.values().collect();
        entries.sort_by_key(|entry| Reverse((entry.meta.opened, entry.meta.id)));
        entries
            .into_iter()
            .take(RECENT_LIMIT)
            .map(Entry::summary)
            .collect()
    }

    pub(crate) fn len(&self) -> usize {
        self.read().entries.len()
    }

    pub(crate) fn used_keys(&self) -> HashSet<SegmentKey> {
        let state = self.read();
        let metas = state.entries.values().map(|entry| &entry.meta);
        let lists = metas.filter_map(|meta| meta.segments.as_ref());
        lists.flat_map(|list| list.keys.iter().copied()).collect()
    }
}
