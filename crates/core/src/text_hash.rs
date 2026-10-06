use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

use crate::CoreError;
use crate::hex::{BITS_PER_DIGIT, digit_value};

const HASH_BYTES: usize = 32;

/// The SHA-256 of the format and the text of a document. The text form is 64 lowercase hex
/// characters.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TextHash([u8; HASH_BYTES]);

impl TextHash {
    /// Makes a text hash from the bytes of a SHA-256.
    pub const fn new(bytes: [u8; HASH_BYTES]) -> Self {
        Self(bytes)
    }

    /// Returns the bytes of the hash.
    pub const fn as_bytes(&self) -> &[u8; HASH_BYTES] {
        &self.0
    }
}

impl Display for TextHash {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        self.0
            .iter()
            .try_for_each(|byte| write!(formatter, "{byte:02x}"))
    }
}

impl FromStr for TextHash {
    type Err = CoreError;

    /// Parses 64 lowercase hex characters.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let (pairs, rest) = text.as_bytes().as_chunks::<2>();
        if pairs.len() != HASH_BYTES || !rest.is_empty() {
            return Err(CoreError::InvalidTextHash);
        }
        let mut bytes = [0; HASH_BYTES];
        for (byte, [high, low]) in bytes.iter_mut().zip(pairs) {
            *byte = hex_value(*high)? << BITS_PER_DIGIT | hex_value(*low)?;
        }
        Ok(Self(bytes))
    }
}

fn hex_value(digit: u8) -> Result<u8, CoreError> {
    digit_value(digit).ok_or(CoreError::InvalidTextHash)
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEX: &str = "00ff10a5e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b";

    #[test]
    fn text_hash_round_trips_when_hex_parsed() {
        let hash: TextHash = HEX.parse().unwrap();

        assert_eq!(hash.to_string(), HEX);
        assert_eq!(hash.as_bytes()[..3], [0x00, 0xff, 0x10]);
    }

    #[test]
    fn text_hash_fails_when_text_not_64_hex_characters() {
        let invalid = [
            String::new(),
            "a".repeat(63),
            "a".repeat(65),
            format!("{}g", "a".repeat(63)),
            HEX.to_uppercase(),
            format!("{}\u{e9}", "a".repeat(62)),
        ];

        for text in invalid {
            assert_eq!(
                text.parse::<TextHash>(),
                Err(CoreError::InvalidTextHash),
                "{text:?}"
            );
        }
    }
}
