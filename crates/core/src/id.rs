use std::fmt::{self, Display, Formatter};

/// The separator between the engine id and the key in the text form of a voice id.
const SEPARATOR: u8 = b'/';

/// The identifier of an engine type, for example "qwen3".
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EngineId(&'static str);

impl EngineId {
    /// Makes an engine id from a constant text.
    ///
    /// ```
    /// use antenna_core::EngineId;
    ///
    /// const QWEN3: EngineId = EngineId::new("qwen3");
    /// assert_eq!(QWEN3.as_str(), "qwen3");
    /// ```
    ///
    /// An empty id does not compile.
    ///
    /// ```compile_fail,E0080
    /// use antenna_core::EngineId;
    ///
    /// const EMPTY: EngineId = EngineId::new("");
    /// assert_eq!(EMPTY.as_str(), "");
    /// ```
    ///
    /// An id with a `/` does not compile.
    ///
    /// ```compile_fail,E0080
    /// use antenna_core::EngineId;
    ///
    /// const SPLIT: EngineId = EngineId::new("qwen3/large");
    /// assert_eq!(SPLIT.as_str(), "qwen3/large");
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if `id` is empty or contains a `/`. In a `const` context, the panic is a compile error.
    pub const fn new(id: &'static str) -> Self {
        assert!(is_valid_part(id), "the engine id is empty or contains '/'");
        Self(id)
    }

    /// Returns the text of the id.
    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

impl Display for EngineId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

/// The identifier of a voice. The text form is "engine/key", for example "qwen3/es-lucia".
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct VoiceId {
    engine: EngineId,
    key: &'static str,
}

impl VoiceId {
    /// Makes a voice id from the engine id and a constant key.
    ///
    /// ```
    /// use antenna_core::{EngineId, VoiceId};
    ///
    /// const LUCIA: VoiceId = VoiceId::new(EngineId::new("qwen3"), "es-lucia");
    /// assert_eq!(LUCIA.to_string(), "qwen3/es-lucia");
    /// ```
    ///
    /// An empty key does not compile.
    ///
    /// ```compile_fail,E0080
    /// use antenna_core::{EngineId, VoiceId};
    ///
    /// const EMPTY: VoiceId = VoiceId::new(EngineId::new("qwen3"), "");
    /// assert_eq!(EMPTY.key(), "");
    /// ```
    ///
    /// A key with a `/` does not compile.
    ///
    /// ```compile_fail,E0080
    /// use antenna_core::{EngineId, VoiceId};
    ///
    /// const SPLIT: VoiceId = VoiceId::new(EngineId::new("qwen3"), "es/lucia");
    /// assert_eq!(SPLIT.key(), "es/lucia");
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if `key` is empty or contains a `/`. In a `const` context, the panic is a compile error.
    pub const fn new(engine: EngineId, key: &'static str) -> Self {
        assert!(is_valid_part(key), "the voice key is empty or contains '/'");
        Self { engine, key }
    }

    /// Returns the id of the engine of the voice.
    pub const fn engine(self) -> EngineId {
        self.engine
    }

    /// Returns the key of the voice in its engine, for example "es-lucia".
    pub const fn key(self) -> &'static str {
        self.key
    }
}

impl Display for VoiceId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}{}{}",
            self.engine,
            char::from(SEPARATOR),
            self.key
        )
    }
}

/// Returns `true` if `part` is not empty and has no separator, so the text form of a voice id
/// has exactly one separator.
const fn is_valid_part(part: &str) -> bool {
    let mut rest = part.as_bytes();
    if rest.is_empty() {
        return false;
    }
    while let [first, tail @ ..] = rest {
        if *first == SEPARATOR {
            return false;
        }
        rest = tail;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn voice_id_shows_engine_and_key() {
        let voice = VoiceId::new(EngineId::new("fake"), "en-alba");

        assert_eq!(voice.to_string(), "fake/en-alba");
        assert_eq!(voice.engine().as_str(), "fake");
        assert_eq!(voice.key(), "en-alba");
    }

    #[test]
    #[should_panic(expected = "the voice key is empty or contains")]
    fn voice_id_panics_when_key_has_separator_at_runtime() {
        let key = String::from("en/alba").leak();

        VoiceId::new(EngineId::new("fake"), key);
    }
}
