#![expect(
    clippy::disallowed_types,
    reason = "the library index is the one permitted lock"
)]

use std::cmp::Reverse;
use std::collections::{BTreeMap, HashSet};
use std::sync::{Arc, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::search::{self, fold};
use crate::status::status;
use crate::swap::{Replacement, Swap};
use crate::{DocumentId, DocumentMeta, DocumentSummary, Filter, LibraryError, SegmentKey};

/// The number of documents that `recent` returns.
pub const RECENT_LIMIT: usize = 4;

#[derive(Debug)]
struct Entry {
    metadata: Arc<DocumentMeta>,
    folded_title: Arc<str>,
    folded_text: Option<Arc<str>>,
    progress: Option<(u32, u32)>,
}

impl Entry {
    fn summary(&self) -> DocumentSummary {
        DocumentSummary {
            meta: Arc::clone(&self.metadata),
            status: status(&self.metadata, self.progress),
        }
    }

    fn candidate(&self, text_index: TextIndex) -> Candidate {
        let text = match text_index {
            TextIndex::Built => self.folded_text.clone(),
            TextIndex::Pending => None,
        };
        Candidate {
            summary: self.summary(),
            title: Arc::clone(&self.folded_title),
            text,
        }
    }
}

/// The data of a document for a search outside the lock.
struct Candidate {
    summary: DocumentSummary,
    title: Arc<str>,
    text: Option<Arc<str>>,
}

impl Candidate {
    fn is_match(&self, terms: &[String]) -> bool {
        search::matches(terms, &self.title, self.text.as_deref().unwrap_or_default())
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
    // No code panics while it holds the guard, so a poisoned lock still holds consistent data.
    fn read(&self) -> RwLockReadGuard<'_, State> {
        self.state.read().unwrap_or_else(PoisonError::into_inner)
    }

    fn write(&self) -> RwLockWriteGuard<'_, State> {
        self.state.write().unwrap_or_else(PoisonError::into_inner)
    }

    pub(crate) fn insert(&self, metadata: DocumentMeta, folded_text: Option<String>) {
        let entry = Entry {
            folded_title: fold(&metadata.title).into(),
            metadata: Arc::new(metadata),
            folded_text: folded_text.map(Arc::from),
            progress: None,
        };
        self.write().entries.insert(entry.metadata.id, entry);
    }

    pub(crate) fn metadata(&self, id: DocumentId) -> Result<Arc<DocumentMeta>, LibraryError> {
        let state = self.read();
        let entry = state.entries.get(&id).ok_or(LibraryError::NotFound(id))?;
        Ok(Arc::clone(&entry.metadata))
    }

    /// Replaces the entry only if it still holds `expected`, the handle that the writer read.
    pub(crate) fn replace(
        &self,
        expected: &Arc<DocumentMeta>,
        replacement: Replacement,
    ) -> Result<Swap, LibraryError> {
        let id = replacement.metadata.id;
        let folded_title: Arc<str> = fold(&replacement.metadata.title).into();
        let metadata = Arc::new(replacement.metadata);
        let mut state = self.write();
        let entry = state
            .entries
            .get_mut(&id)
            .ok_or(LibraryError::NotFound(id))?;
        if !Arc::ptr_eq(&entry.metadata, expected) {
            return Ok(Swap::Stale);
        }
        entry.metadata = Arc::clone(&metadata);
        entry.folded_title = folded_title;
        if replacement.folded_text.is_some() {
            entry.folded_text = replacement.folded_text;
        }
        Ok(Swap::Done(metadata))
    }

    pub(crate) fn remove(&self, id: DocumentId) {
        self.write().entries.remove(&id);
    }

    pub(crate) fn set_progress(&self, id: DocumentId, progress: Option<(u32, u32)>) {
        if let Some(entry) = self.write().entries.get_mut(&id) {
            entry.progress = progress;
        }
    }

    pub(crate) fn unindexed(&self) -> Vec<Arc<DocumentMeta>> {
        let state = self.read();
        let entries = state.entries.values();
        entries
            .filter(|entry| entry.folded_text.is_none())
            .map(|entry| Arc::clone(&entry.metadata))
            .collect()
    }

    pub(crate) fn finish_text_index(&self, texts: Vec<(DocumentId, String)>) {
        let mut state = self.write();
        for (id, folded_text) in texts {
            if let Some(entry) = state.entries.get_mut(&id) {
                entry.folded_text.get_or_insert_with(|| folded_text.into());
            }
        }
        state.text_index = TextIndex::Built;
    }

    pub(crate) fn matching(&self, filter: Filter, terms: &[String]) -> Vec<DocumentSummary> {
        let candidates: Vec<Candidate> = {
            let state = self.read();
            let entries = state.entries.values();
            let candidates = entries.map(|entry| entry.candidate(state.text_index));
            candidates
                .filter(|candidate| filter.is_selected(candidate.summary.status))
                .collect()
        };
        candidates
            .into_iter()
            .filter(|candidate| candidate.is_match(terms))
            .map(|candidate| candidate.summary)
            .collect()
    }

    pub(crate) fn recent(&self) -> Vec<DocumentSummary> {
        let state = self.read();
        let mut entries: Vec<&Entry> = state.entries.values().collect();
        entries.sort_by_key(|entry| Reverse((entry.metadata.opened, entry.metadata.id)));
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
        let handles: Vec<Arc<DocumentMeta>> = {
            let state = self.read();
            let entries = state.entries.values();
            entries.map(|entry| Arc::clone(&entry.metadata)).collect()
        };
        let lists = handles
            .iter()
            .filter_map(|metadata| metadata.segments.as_ref());
        lists.flat_map(|list| list.keys.iter().copied()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::{at, bare_metadata};

    fn index_with_document() -> (Index, DocumentMeta) {
        let index = Index::default();
        let metadata = bare_metadata();
        index.insert(metadata.clone(), Some("old".to_owned()));
        (index, metadata)
    }

    fn opened(metadata: &DocumentMeta, seconds: i64) -> Replacement {
        Replacement::for_metadata(DocumentMeta {
            opened: at(seconds),
            ..metadata.clone()
        })
    }

    #[test]
    fn replace_stores_metadata_when_expected_is_current() {
        let (index, metadata) = index_with_document();
        let current = index.metadata(metadata.id).unwrap();

        let swap = index.replace(&current, opened(&metadata, 5)).unwrap();

        assert!(matches!(swap, Swap::Done(stored) if stored.opened == at(5)));
        assert_eq!(index.metadata(metadata.id).unwrap().opened, at(5));
    }

    #[test]
    fn replace_is_stale_when_expected_was_replaced() {
        let (index, metadata) = index_with_document();
        let old = index.metadata(metadata.id).unwrap();
        index.replace(&old, opened(&metadata, 5)).unwrap();

        let swap = index.replace(&old, opened(&metadata, 9)).unwrap();

        assert!(matches!(swap, Swap::Stale));
        assert_eq!(index.metadata(metadata.id).unwrap().opened, at(5));
    }

    #[test]
    fn replace_is_stale_when_expected_is_a_copy_with_equal_content() {
        let (index, metadata) = index_with_document();
        let copy = Arc::new(metadata.clone());

        let swap = index.replace(&copy, opened(&metadata, 5)).unwrap();

        assert!(matches!(swap, Swap::Stale));
    }

    #[test]
    fn replace_fails_when_entry_removed() {
        let (index, metadata) = index_with_document();
        let current = index.metadata(metadata.id).unwrap();
        index.remove(metadata.id);

        let result = index.replace(&current, opened(&metadata, 5));

        assert!(matches!(result, Err(LibraryError::NotFound(id)) if id == metadata.id));
    }

    #[test]
    fn replace_keeps_folded_text_when_replacement_has_none() {
        let (index, metadata) = index_with_document();
        let current = index.metadata(metadata.id).unwrap();
        index.replace(&current, opened(&metadata, 5)).unwrap();
        index.finish_text_index(Vec::new());

        let found = index.matching(Filter::All, &["old".to_owned()]);

        assert_eq!(found.len(), 1);
    }
}
