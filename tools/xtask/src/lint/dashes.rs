//! The check of the dashes in all text files.

use std::ffi::OsStr;
use std::path::{Component, Path};

use super::{Check, Violation};

const DASHES: [char; 2] = ['\u{2014}', '\u{2013}'];
const EXCLUDED_PATHS: [&str; 3] = [
    "docs/design/pages",
    "docs/design/source-bundle.html",
    "tools/eval/corpus",
];
const FIXTURE_DIRECTORIES: [&str; 2] = ["tests", "fixtures"];

/// Finds each line with an em dash or an en dash.
pub(super) fn check(path: &Path, text: &str) -> Vec<Violation> {
    if is_excluded(path) {
        return Vec::new();
    }
    text.lines()
        .enumerate()
        .filter(|(_, line)| line.contains(DASHES))
        .map(|(index, _)| {
            Violation::with_line(
                Check::Dashes,
                path,
                index,
                "the line has an em dash or an en dash",
            )
        })
        .collect()
}

fn is_excluded(path: &Path) -> bool {
    let components: Vec<_> = path.components().collect();
    EXCLUDED_PATHS
        .iter()
        .any(|excluded| path.starts_with(excluded))
        || components.windows(2).any(|pair| {
            pair.iter()
                .zip(FIXTURE_DIRECTORIES)
                .all(|(component, name)| *component == Component::Normal(OsStr::new(name)))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lint_repo_finds_dashes() {
        let fixture = include_str!("../../tests/fixtures/dashes.txt");

        let lines: Vec<_> = check(Path::new("docs/architecture.md"), fixture)
            .into_iter()
            .map(|violation| violation.line)
            .collect();

        assert_eq!(lines, [Some(2)]);
    }

    #[test]
    fn dashes_finds_em_dash() {
        let text = format!("one\ntwo {} three\n", '\u{2014}');

        let found = check(Path::new("README.md"), &text);

        assert_eq!(found.len(), 1);
    }

    #[test]
    fn dashes_skips_excluded_paths() {
        let fixture = include_str!("../../tests/fixtures/dashes.txt");
        let excluded = [
            "docs/design/pages/estudio.html",
            "docs/design/source-bundle.html",
            "tools/eval/corpus/es.txt",
            "crates/engines/qwen3/tests/fixtures/notes.txt",
        ];

        for path in excluded {
            assert!(check(Path::new(path), fixture).is_empty(), "{path}");
        }
    }
}
