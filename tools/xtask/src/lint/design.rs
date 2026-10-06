//! The check of raw design values in the desktop app and the UI crate.

use super::source::is_word_character;
use super::{Check, RustFile, Violation};

const SCOPES: [&str; 2] = ["apps/desktop/src", "crates/ui/src"];
const THEME: &str = "crates/ui/src/theme";
const VALUE_CALLS: [&str; 4] = ["rgb(", "rgba(", "hsla(", "px("];
const HEX_COLOR_DIGITS: usize = 6;
const SIZE_METHODS: [&str; 29] = [
    "p", "px", "py", "pt", "pr", "pb", "pl", "m", "mx", "my", "mt", "mr", "mb", "ml", "gap",
    "gap_x", "gap_y", "w", "h", "size", "min_w", "min_h", "max_w", "max_h", "top", "right",
    "bottom", "left", "inset",
];
const ROUNDED_PREFIX: &str = "rounded_";
const ROUNDED_SIZES: [&str; 7] = ["sm", "md", "lg", "xl", "2xl", "3xl", "full"];
const TEXT_PREFIX: &str = "text_";
const TEXT_SIZES: [&str; 7] = ["xs", "sm", "base", "lg", "xl", "2xl", "3xl"];

/// Finds each line with a raw color, a raw size or a GPUI style method with a size in its name.
pub(super) fn check(file: &RustFile<'_>) -> Vec<Violation> {
    let is_in_scope = SCOPES.iter().any(|scope| file.path.starts_with(scope));
    if !is_in_scope || file.path.starts_with(THEME) {
        return Vec::new();
    }
    file.text
        .lines()
        .enumerate()
        .filter(|(_, line)| has_raw_value(line))
        .map(|(index, _)| {
            let detail = "the line has a raw design value instead of a design token";
            Violation::with_line(Check::RawDesignValues, file.path, index, detail)
        })
        .collect()
}

fn has_raw_value(line: &str) -> bool {
    VALUE_CALLS.iter().any(|call| line.contains(call))
        || has_hex_color(line)
        || line
            .match_indices('.')
            .filter_map(|(index, _)| line.get(index + 1..))
            .any(is_sized_method)
}

/// Matches `0x` and six hex digits at the end of a word, or `"#` and six hex digits and `"`.
fn has_hex_color(line: &str) -> bool {
    let after = |prefix: &'static str| {
        line.match_indices(prefix)
            .filter_map(move |(index, _)| line.get(index + prefix.len()..))
    };
    let hex_then = |rest: &str| {
        let mut characters = rest.chars();
        let digits = characters
            .by_ref()
            .take(HEX_COLOR_DIGITS)
            .filter(char::is_ascii_hexdigit)
            .count();
        (digits == HEX_COLOR_DIGITS).then(|| characters.next())
    };
    after("0x").any(|rest| {
        hex_then(rest).is_some_and(|next| next.is_none_or(|next| !is_word_character(next)))
    }) || after("\"#").any(|rest| hex_then(rest) == Some(Some('"')))
}

/// Matches the text after a `.` that starts with a GPUI style method with a size in its name.
fn is_sized_method(rest: &str) -> bool {
    let has_size_digit = SIZE_METHODS.iter().any(|method| {
        rest.strip_prefix(method)
            .and_then(|after| after.strip_prefix('_'))
            .is_some_and(|after| after.starts_with(|next: char| next.is_ascii_digit()))
    });
    has_size_digit
        || has_size_word(rest, ROUNDED_PREFIX, &ROUNDED_SIZES)
        || has_size_word(rest, TEXT_PREFIX, &TEXT_SIZES)
}

fn has_size_word(rest: &str, prefix: &str, sizes: &[&str]) -> bool {
    rest.strip_prefix(prefix).is_some_and(|after| {
        sizes.iter().any(|size| {
            after
                .strip_prefix(size)
                .is_some_and(|end| !end.starts_with(is_word_character))
        })
    })
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    fn lines_of(path: &str, text: &str) -> Vec<Option<usize>> {
        let file = RustFile::new(Path::new(path), text);
        check(&file)
            .into_iter()
            .map(|violation| violation.line)
            .collect()
    }

    #[test]
    fn lint_repo_finds_raw_design_values() {
        let fixture = include_str!("../../tests/fixtures/raw_design_values.txt");

        let lines = lines_of("crates/ui/src/components/button.rs", fixture);

        assert_eq!(lines, [1, 2, 3, 4, 5, 6].map(Some));
    }

    #[test]
    fn raw_design_values_skips_theme_and_other_crates() {
        let fixture = include_str!("../../tests/fixtures/raw_design_values.txt");

        for path in ["crates/ui/src/theme/colors.rs", "crates/audio/src/lib.rs"] {
            assert!(lines_of(path, fixture).is_empty(), "{path}");
        }
    }
}
