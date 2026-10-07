use std::sync::Arc;

use crate::DocumentMeta;

/// The new data of an index entry. `folded_text` is `None` when the text did not change.
#[derive(Debug)]
pub(crate) struct Replacement {
    pub(crate) metadata: DocumentMeta,
    pub(crate) folded_text: Option<Arc<str>>,
}

impl Replacement {
    pub(crate) fn for_metadata(metadata: DocumentMeta) -> Self {
        Self {
            metadata,
            folded_text: None,
        }
    }
}

/// The result of `Index::replace`.
#[derive(Debug)]
pub(crate) enum Swap {
    /// The entry held the expected metadata. It holds the new metadata now.
    Done(Arc<DocumentMeta>),
    /// Another writer replaced the metadata first. Nothing changed.
    Stale,
}
