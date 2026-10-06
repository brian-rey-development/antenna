//! The check of prohibited names of modules, files, types and directories.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use super::{Check, RUST_ROOTS, RustFile, Violation};

const PROHIBITED: [&str; 15] = [
    "util",
    "utils",
    "helper",
    "helpers",
    "common",
    "misc",
    "shared",
    "manager",
    "base",
    "foundation",
    "platform",
    "infra",
    "libs",
    "services",
    "core-utils",
];
const TYPE_KEYWORDS: [&str; 4] = ["struct", "enum", "trait", "type"];
const MODULE_KEYWORD: &str = "mod";

/// Finds each directory under the Rust roots with a prohibited name.
pub(super) fn check_directories(files: &[PathBuf]) -> Vec<Violation> {
    let directories: BTreeSet<&Path> = files
        .iter()
        .flat_map(|file| file.ancestors().skip(1))
        .filter(|directory| RUST_ROOTS.iter().any(|root| directory.starts_with(root)))
        .collect();
    directories
        .into_iter()
        .filter(|directory| {
            directory
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(is_prohibited)
        })
        .map(|directory| {
            Violation::new(
                Check::Names,
                directory,
                "the directory has a prohibited name",
            )
        })
        .collect()
}

/// Finds a prohibited file stem, module name or word of a type name in one Rust file.
pub(super) fn check_file(file: &RustFile<'_>) -> Vec<Violation> {
    let stem = file.path.file_stem().and_then(|stem| stem.to_str());
    let mut violations: Vec<_> = stem
        .filter(|stem| is_prohibited(stem))
        .map(|_| Violation::new(Check::Names, file.path, "the file has a prohibited name"))
        .into_iter()
        .collect();
    for (index, line) in file.lines.iter().enumerate() {
        let words: Vec<_> = line.words().collect();
        let is_prohibited_declaration = words.windows(2).any(|pair| match pair {
            [keyword, name] if *keyword == MODULE_KEYWORD => is_prohibited(name),
            [keyword, name] if TYPE_KEYWORDS.contains(keyword) => {
                type_words(name).any(is_prohibited)
            }
            _ => false,
        });
        if is_prohibited_declaration {
            let detail = "the line declares a module or a type with a prohibited name";
            violations.push(Violation::with_line(Check::Names, file.path, index, detail));
        }
    }
    violations
}

fn is_prohibited(name: &str) -> bool {
    PROHIBITED.contains(&name.to_lowercase().as_str())
}

/// Splits a type name into its words, at each uppercase letter and each `_`.
fn type_words(name: &str) -> impl Iterator<Item = &str> {
    name.split('_').flat_map(|part| {
        let starts: Vec<_> = part
            .char_indices()
            .filter(|(index, character)| *index == 0 || character.is_uppercase())
            .map(|(index, _)| index)
            .chain([part.len()])
            .collect();
        let words: Vec<_> = starts
            .windows(2)
            .filter_map(|pair| match pair {
                [start, end] => part.get(*start..*end),
                _ => None,
            })
            .collect();
        words
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lint_repo_finds_names() {
        let fixture = include_str!("../../tests/fixtures/names.txt");
        let file = RustFile::new(Path::new("crates/text/src/common.rs"), fixture);

        let lines: Vec<_> = check_file(&file)
            .into_iter()
            .map(|violation| violation.line)
            .collect();

        assert_eq!(lines, [None, Some(1), Some(2), Some(3), Some(4), Some(5)]);
    }

    #[test]
    fn names_finds_prohibited_directories_under_rust_roots() {
        let files = [
            "crates/storage/shared/src/lib.rs",
            "tools/core-utils/main.rs",
            "docs/misc/notes.md",
            "crates/text/src/segment/mod.rs",
        ]
        .map(PathBuf::from);

        let paths: Vec<_> = check_directories(&files)
            .into_iter()
            .map(|violation| violation.path)
            .collect();

        assert_eq!(
            paths,
            [
                PathBuf::from("crates/storage/shared"),
                PathBuf::from("tools/core-utils")
            ]
        );
    }

    #[test]
    fn type_words_split_camel_case() {
        let words: Vec<_> = type_words("PlayerShared_Url").collect();

        assert_eq!(words, ["Player", "Shared", "Url"]);
    }
}
