//! The checks of the comments of a Rust file: banned markers, banner comments, commented-out
//! code and comment density.

use super::{Check, RustFile, Violation};

const BANNED_MARKERS: [&str; 4] = ["TODO", "FIXME", "XXX", "HACK"];
const BANNED_WORD: &str = "Note";
const BANNER_CHARACTERS: [char; 4] = ['=', '-', '*', '#'];
const BANNER_RUN: usize = 3;
const CODE_ENDINGS: [char; 3] = [';', '{', '}'];
const SAFETY_PREFIX: &str = "// SAFETY:";
const MAX_COMMENT_PERCENT: usize = 15;
const PERCENT: usize = 100;
/// The pictograph blocks of Unicode that contain the emoji.
const EMOJI_RANGES: [(char, char); 4] = [
    ('\u{2600}', '\u{27BF}'),
    ('\u{2B00}', '\u{2BFF}'),
    ('\u{FE0F}', '\u{FE0F}'),
    ('\u{1F000}', '\u{1FAFF}'),
];

/// Runs the checks of this module on one Rust file.
pub(super) fn check(file: &RustFile<'_>) -> Vec<Violation> {
    let mut violations = Vec::new();
    for (index, line) in file.lines.iter().enumerate() {
        let at = |check, detail| Violation::with_line(check, file.path, index, detail);
        if has_banned_marker(&line.comment) {
            violations.push(at(
                Check::BannedMarkers,
                "the comment has a banned marker or an emoji",
            ));
        }
        let Some(comment) = line.plain_comment() else {
            continue;
        };
        if is_banner(comment) {
            violations.push(at(Check::BannerComments, "the comment is a banner line"));
        }
        if comment.trim_end().ends_with(CODE_ENDINGS) {
            violations.push(at(
                Check::CommentedOutCode,
                "the comment ends like a line of code",
            ));
        }
    }
    violations.extend(comment_density(file));
    violations
}

fn has_banned_marker(comment: &str) -> bool {
    BANNED_MARKERS.iter().any(|marker| comment.contains(marker))
        || comment
            .split(|character: char| !character.is_alphanumeric())
            .any(|word| word == BANNED_WORD)
        || comment.chars().any(is_emoji)
}

fn is_emoji(character: char) -> bool {
    EMOJI_RANGES
        .iter()
        .any(|(first, last)| (*first..=*last).contains(&character))
}

fn is_banner(comment: &str) -> bool {
    let mut run = 0;
    comment.chars().any(|character| {
        run = if BANNER_CHARACTERS.contains(&character) {
            run + 1
        } else {
            0
        };
        run >= BANNER_RUN
    })
}

fn comment_density(file: &RustFile<'_>) -> Option<Violation> {
    let non_blank = file
        .text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count();
    let comments = file
        .lines
        .iter()
        .filter_map(|line| line.plain_comment())
        .filter(|comment| !comment.starts_with(SAFETY_PREFIX))
        .count();
    let is_dense = comments * PERCENT > non_blank * MAX_COMMENT_PERCENT;
    is_dense.then(|| {
        let detail = format!(
            "{comments} of {non_blank} non-blank lines are // comments, the limit is {MAX_COMMENT_PERCENT}%"
        );
        Violation::new(Check::CommentDensity, file.path, detail)
    })
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    fn lines_of(check_kind: Check, fixture: &str) -> Vec<Option<usize>> {
        let file = RustFile::new(Path::new("crates/text/src/lib.rs"), fixture);
        check(&file)
            .into_iter()
            .filter(|violation| violation.check == check_kind)
            .map(|violation| violation.line)
            .collect()
    }

    #[test]
    fn lint_repo_finds_banned_markers() {
        let fixture = include_str!("../../tests/fixtures/banned_markers.txt");

        let lines = lines_of(Check::BannedMarkers, fixture);

        assert_eq!(lines, [2, 3, 4, 5, 6, 7, 8].map(Some));
    }

    #[test]
    fn lint_repo_finds_banner_comments() {
        let fixture = include_str!("../../tests/fixtures/banner_comments.txt");

        let lines = lines_of(Check::BannerComments, fixture);

        assert_eq!(lines, [1, 3, 5].map(Some));
    }

    #[test]
    fn lint_repo_finds_commented_out_code() {
        let fixture = include_str!("../../tests/fixtures/commented_out_code.txt");

        let lines = lines_of(Check::CommentedOutCode, fixture);

        assert_eq!(lines, [2, 3, 4].map(Some));
    }

    #[test]
    fn lint_repo_finds_comment_density() {
        let fixture = include_str!("../../tests/fixtures/comment_density.txt");

        let lines = lines_of(Check::CommentDensity, fixture);

        assert_eq!(lines, [None]);
    }

    #[test]
    fn comment_density_ignores_doc_and_safety_comments() {
        let text = "/// doc\n//! inner\n// SAFETY: why\nfn a() {}\n";

        assert!(lines_of(Check::CommentDensity, text).is_empty());
    }
}
