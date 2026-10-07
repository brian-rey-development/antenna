use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

use uuid::Uuid;

/// The identifier of a document. It is a UUID v7, so identifiers sort by creation time.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DocumentId(Uuid);

impl DocumentId {
    /// Returns the UUID of the identifier.
    pub fn uuid(self) -> Uuid {
        self.0
    }
}

impl Display for DocumentId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, formatter)
    }
}

impl FromStr for DocumentId {
    type Err = uuid::Error;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        text.parse().map(Self)
    }
}
