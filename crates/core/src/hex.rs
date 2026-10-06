//! Lowercase hex text, the text form of revisions and SHA-256 values.

/// The number of bits that one hex digit holds.
pub(crate) const BITS_PER_DIGIT: u32 = 4;

/// Returns the value of a lowercase hex digit, or `None` for another byte.
pub(crate) fn digit_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

/// Returns `true` if `text` has exactly `length` lowercase hex digits.
pub(crate) fn is_lowercase_hex(text: &str, length: usize) -> bool {
    text.len() == length && text.bytes().all(|byte| digit_value(byte).is_some())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digit_value_accepts_only_lowercase_hex() {
        assert_eq!(digit_value(b'0'), Some(0));
        assert_eq!(digit_value(b'9'), Some(9));
        assert_eq!(digit_value(b'a'), Some(10));
        assert_eq!(digit_value(b'f'), Some(15));
        assert_eq!(digit_value(b'A'), None);
        assert_eq!(digit_value(b'g'), None);
    }

    #[test]
    fn is_lowercase_hex_checks_length_and_digits() {
        assert!(is_lowercase_hex("00ff", 4));
        assert!(!is_lowercase_hex("00ff", 3));
        assert!(!is_lowercase_hex("00FF", 4));
    }
}
