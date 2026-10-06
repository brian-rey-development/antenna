//! The checks that read the code of a Rust file: file length, exhaustive enums, unsafe code,
//! component library and desktop layers.

use std::path::Path;

use super::source::SourceLine;
use super::{Check, RustFile, Violation};

const MAX_FILE_LINES: usize = 300;
const UNSAFE_FILE: &str = "crates/inference/ml/src/weights.rs";
const UI_CRATE: &str = "crates/ui";
const DESKTOP_LAYERS: [&str; 2] = ["apps/desktop/src/screens", "apps/desktop/src/shell"];
const SERVICE_CRATES: [&str; 4] = [
    "antenna_audio",
    "antenna_library",
    "antenna_models",
    "antenna_pipeline",
];
const INPUT_PATH: &str = "Input::";

/// A check that one line of code can fail.
struct LineRule {
    check: Check,
    detail: &'static str,
    is_applicable: fn(&Path) -> bool,
    is_violation: fn(&SourceLine) -> bool,
}

const LINE_RULES: [LineRule; 4] = [
    LineRule {
        check: Check::ExhaustiveEnums,
        detail: "the line has #[non_exhaustive]",
        is_applicable: |_| true,
        is_violation: |line| has_word(line, "non_exhaustive"),
    },
    LineRule {
        check: Check::UnsafeCode,
        detail: "the line has the unsafe keyword outside the weights module of antenna-ml",
        is_applicable: |path| path != Path::new(UNSAFE_FILE),
        is_violation: |line| has_word(line, "unsafe"),
    },
    LineRule {
        check: Check::ComponentLibrary,
        detail: "the line names gpui_component outside crates/ui",
        is_applicable: |path| !path.starts_with(UI_CRATE),
        is_violation: |line| has_word(line, "gpui_component"),
    },
    LineRule {
        check: Check::DesktopLayers,
        detail: "a screen or the shell names a service crate or Input:: instead of an Action",
        is_applicable: |path| DESKTOP_LAYERS.iter().any(|layer| path.starts_with(layer)),
        is_violation: |line| {
            SERVICE_CRATES.iter().any(|name| has_word(line, name)) || has_input_path(&line.code)
        },
    },
];

/// Runs the checks of this module on one Rust file.
pub(super) fn check(file: &RustFile<'_>) -> Vec<Violation> {
    let rules = LINE_RULES
        .iter()
        .filter(|rule| (rule.is_applicable)(file.path));
    let mut violations: Vec<_> = rules
        .flat_map(|rule| {
            file.lines
                .iter()
                .enumerate()
                .filter(|(_, line)| (rule.is_violation)(line))
                .map(|(index, _)| Violation::with_line(rule.check, file.path, index, rule.detail))
        })
        .collect();
    let line_count = file.text.lines().count();
    if line_count > MAX_FILE_LINES {
        let detail = format!("the file has {line_count} lines, the limit is {MAX_FILE_LINES}");
        violations.push(Violation::new(Check::FileLength, file.path, detail));
    }
    violations
}

fn has_word(line: &SourceLine, word: &str) -> bool {
    line.words().any(|candidate| candidate == word)
}

/// Finds `Input::` as a complete path segment, so `TextInput::` does not match.
fn has_input_path(code: &str) -> bool {
    code.match_indices(INPUT_PATH).any(|(index, _)| {
        code.get(..index)
            .and_then(|before| before.chars().next_back())
            .is_none_or(|previous| !super::source::is_word_character(previous))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn checks_of(path: &str, text: &str) -> Vec<(Check, Option<usize>)> {
        let file = RustFile::new(Path::new(path), text);
        check(&file)
            .into_iter()
            .map(|violation| (violation.check, violation.line))
            .collect()
    }

    #[test]
    fn lint_repo_finds_file_length() {
        let text = include_str!("../../tests/fixtures/file_length.txt");

        let found = checks_of("crates/text/src/lib.rs", text);

        assert_eq!(found, [(Check::FileLength, None)]);
        assert!(checks_of("crates/text/src/lib.rs", &"x\n".repeat(MAX_FILE_LINES)).is_empty());
    }

    #[test]
    fn lint_repo_finds_non_exhaustive() {
        let text = include_str!("../../tests/fixtures/non_exhaustive.txt");

        let found = checks_of("crates/core/src/lib.rs", text);

        assert_eq!(found, [(Check::ExhaustiveEnums, Some(2))]);
    }

    #[test]
    fn lint_repo_finds_unsafe_code() {
        let text = include_str!("../../tests/fixtures/unsafe_code.txt");

        let found = checks_of("crates/audio/src/lib.rs", text);

        assert_eq!(found, [(Check::UnsafeCode, Some(4))]);
        assert!(checks_of(UNSAFE_FILE, text).is_empty());
    }

    #[test]
    fn lint_repo_finds_component_library() {
        let text = include_str!("../../tests/fixtures/component_library.txt");

        let found = checks_of("apps/desktop/src/main.rs", text);

        assert_eq!(found, [(Check::ComponentLibrary, Some(1))]);
        assert!(checks_of("crates/ui/src/components/button.rs", text).is_empty());
    }

    #[test]
    fn lint_repo_finds_desktop_layers() {
        let text = include_str!("../../tests/fixtures/desktop_layers.txt");

        let found = checks_of("apps/desktop/src/screens/studio/mod.rs", text);

        let expected = [2, 3].map(|line| (Check::DesktopLayers, Some(line)));
        assert_eq!(found, expected);
        assert!(checks_of("apps/desktop/src/state/mod.rs", text).is_empty());
    }
}
