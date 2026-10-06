//! The checks of the `Cargo.toml` files of the members: crate names, lint override and versions.

mod lines;

use std::path::{Component, Path};

use toml::{Table, Value};

use super::{Check, Violation};
use crate::error::XtaskError;
use lines::{Scope, header_line, line_of};

const GROUPS: [&str; 3] = ["storage", "inference", "engines"];
const ENGINE_GROUP: &str = "engines";
const FIXED_NAMES: [(&str, &str, &str); 4] = [
    ("apps", "cli", "antenna-cli"),
    ("apps", "desktop", "antenna-desktop"),
    ("tools", "eval", "antenna-eval"),
    ("tools", "xtask", "xtask"),
];
const DEPENDENCY_TABLES: [&str; 3] = ["dependencies", "dev-dependencies", "build-dependencies"];
const PACKAGE_TABLE: &str = "package";
const NAME_KEY: &str = "name";
const LINTS_TABLE: &str = "lints";

/// Runs the checks of this module on one manifest. A manifest with no `[package]` is the
/// workspace manifest, and the checks skip it.
///
/// # Errors
///
/// Returns [`XtaskError::Manifest`] if the file is not valid TOML.
pub(super) fn check(path: &Path, text: &str) -> Result<Vec<Violation>, XtaskError> {
    let manifest: Table = text.parse().map_err(|source| XtaskError::Manifest {
        path: path.to_owned(),
        source,
    })?;
    let Some(package) = manifest.get(PACKAGE_TABLE).and_then(Value::as_table) else {
        return Ok(Vec::new());
    };
    let mut violations: Vec<_> = crate_name(path, text, package).into_iter().collect();
    violations.extend(lint_override(path, text, &manifest));
    violations.extend(versions(path, text, &manifest));
    Ok(violations)
}

fn crate_name(path: &Path, text: &str, package: &Table) -> Option<Violation> {
    let name = package.get(NAME_KEY).and_then(Value::as_str);
    let detail = match path.parent().and_then(expected_name) {
        Some(expected) if name == Some(expected.as_str()) => return None,
        Some(expected) => format!("the package name must be {expected}"),
        None => "docs/architecture.md section 3 has no crate in this directory".to_owned(),
    };
    let line = line_of(text, Scope::Package, PACKAGE_TABLE, NAME_KEY);
    Some(violation_at(Check::CrateNames, path, line, detail))
}

/// Returns the package name that `docs/standards.md` section 3.1 gives to a crate directory.
fn expected_name(directory: &Path) -> Option<String> {
    let names: Vec<_> = directory
        .components()
        .map(|component| match component {
            Component::Normal(name) => name.to_str(),
            Component::Prefix(_)
            | Component::RootDir
            | Component::CurDir
            | Component::ParentDir => None,
        })
        .collect::<Option<_>>()?;
    match names.as_slice() {
        ["crates", group, name] if *group == ENGINE_GROUP => Some(format!("antenna-engine-{name}")),
        ["crates", group, name] if GROUPS.contains(group) => Some(format!("antenna-{name}")),
        ["crates", name] if !GROUPS.contains(name) => Some(format!("antenna-{name}")),
        [root, name] => FIXED_NAMES
            .iter()
            .find(|(fixed_root, fixed_name, _)| fixed_root == root && fixed_name == name)
            .map(|(_, _, package)| (*package).to_owned()),
        _ => None,
    }
}

fn lint_override(path: &Path, text: &str, manifest: &Table) -> Option<Violation> {
    let lints = manifest.get(LINTS_TABLE).and_then(Value::as_table);
    let is_inherited = lints.is_some_and(|lints| {
        lints.len() == 1 && lints.get("workspace").and_then(Value::as_bool) == Some(true)
    });
    (!is_inherited).then(|| {
        let line = header_line(text, LINTS_TABLE);
        let detail = "the [lints] table must contain only workspace = true";
        violation_at(Check::LintOverride, path, line, detail)
    })
}

/// Makes a violation at a line index, or a violation of the complete file when the manifest has
/// no line for it.
fn violation_at(
    check: Check,
    path: &Path,
    index: Option<usize>,
    detail: impl Into<String>,
) -> Violation {
    match index {
        Some(index) => Violation::with_line(check, path, index, detail),
        None => Violation::new(check, path, detail),
    }
}

fn versions(path: &Path, text: &str, manifest: &Table) -> Vec<Violation> {
    let mut violations: Vec<_> = dependencies(manifest)
        .filter(|(_, _, _, declaration)| !is_inherited(declaration))
        .map(|(scope, table, name, _)| {
            let detail = format!("the dependency {name} must use workspace = true");
            violation_at(
                Check::Versions,
                path,
                line_of(text, scope, table, name),
                detail,
            )
        })
        .collect();
    violations.sort_by_key(|violation| violation.line);
    violations
}

/// Returns each dependency declaration of the package and of its targets, with its scope and the
/// name of its table.
fn dependencies(manifest: &Table) -> impl Iterator<Item = (Scope, &'static str, &String, &Value)> {
    let targets = manifest
        .get("target")
        .and_then(Value::as_table)
        .into_iter()
        .flat_map(|targets| targets.values().filter_map(Value::as_table))
        .map(|target| (Scope::Target, target));
    [(Scope::Package, manifest)]
        .into_iter()
        .chain(targets)
        .flat_map(|(scope, tables)| {
            DEPENDENCY_TABLES.into_iter().filter_map(move |table| {
                let declarations = tables.get(table)?.as_table()?;
                Some(
                    declarations
                        .iter()
                        .map(move |(name, declaration)| (scope, table, name, declaration)),
                )
            })
        })
        .flatten()
}

fn is_inherited(declaration: &Value) -> bool {
    declaration.as_table().is_some_and(|table| {
        table.get("workspace").and_then(Value::as_bool) == Some(true)
            && !table.contains_key("version")
            && !table.contains_key("path")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const CRATE_NAMES: &str = include_str!("../../../tests/fixtures/crate_names.toml");

    fn found(path: &str, text: &str, check_kind: Check) -> Vec<Option<usize>> {
        check(Path::new(path), text)
            .unwrap()
            .into_iter()
            .filter(|violation| violation.check == check_kind)
            .map(|violation| violation.line)
            .collect()
    }

    #[test]
    fn lint_repo_finds_crate_names() {
        let lines = found(
            "crates/engines/qwen3/Cargo.toml",
            CRATE_NAMES,
            Check::CrateNames,
        );

        assert_eq!(lines, [Some(2)]);
    }

    #[test]
    fn crate_names_fails_when_directory_has_no_crate() {
        let lines = found("crates/sound/fx/Cargo.toml", CRATE_NAMES, Check::CrateNames);

        assert_eq!(lines, [Some(2)]);
    }

    #[test]
    fn crate_names_passes_when_name_follows_directory() {
        let lines = found(
            "crates/storage/qwen3/Cargo.toml",
            CRATE_NAMES,
            Check::CrateNames,
        );

        assert!(lines.is_empty());
    }

    #[test]
    fn crate_names_follow_directory_rules() {
        let cases = [
            ("crates/core", Some("antenna-core")),
            ("crates/storage/library", Some("antenna-library")),
            ("crates/engines/fake", Some("antenna-engine-fake")),
            ("apps/cli", Some("antenna-cli")),
            ("tools/xtask", Some("xtask")),
            ("crates/engines", None),
            ("apps/web", None),
        ];

        for (directory, expected) in cases {
            assert_eq!(
                expected_name(Path::new(directory)).as_deref(),
                expected,
                "{directory}"
            );
        }
    }

    #[test]
    fn lint_repo_finds_lint_override() {
        let fixture = include_str!("../../../tests/fixtures/lint_override.toml");

        let lines = found("crates/text/Cargo.toml", fixture, Check::LintOverride);

        assert_eq!(lines, [Some(5)]);
    }

    #[test]
    fn lint_override_has_no_line_when_lints_table_missing() {
        let text = "[package]\nname = \"antenna-text\"\n";

        let lines = found("crates/text/Cargo.toml", text, Check::LintOverride);

        assert_eq!(lines, [None]);
    }

    #[test]
    fn lint_repo_finds_versions() {
        let fixture = include_str!("../../../tests/fixtures/versions.toml");

        let lines = found("crates/text/Cargo.toml", fixture, Check::Versions);

        assert_eq!(lines, [Some(10), Some(11), Some(16), Some(18), Some(22)]);
    }

    #[test]
    fn manifest_checks_skip_workspace_manifest() {
        let text = "[workspace]\nmembers = []\n";

        assert!(check(Path::new("Cargo.toml"), text).unwrap().is_empty());
    }
}
