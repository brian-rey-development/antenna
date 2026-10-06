//! The checks of the `Cargo.toml` files of the members: crate names, lint override and versions.

use std::path::{Component, Path};

use toml::{Table, Value};

use super::{Check, Violation};
use crate::error::XtaskError;

const GROUPS: [&str; 3] = ["storage", "inference", "engines"];
const ENGINE_GROUP: &str = "engines";
const FIXED_NAMES: [(&str, &str, &str); 4] = [
    ("apps", "cli", "antenna-cli"),
    ("apps", "desktop", "antenna-desktop"),
    ("tools", "eval", "antenna-eval"),
    ("tools", "xtask", "xtask"),
];
const DEPENDENCY_TABLES: [&str; 3] = ["dependencies", "dev-dependencies", "build-dependencies"];

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
    let Some(package) = manifest.get("package").and_then(Value::as_table) else {
        return Ok(Vec::new());
    };
    let mut violations: Vec<_> = crate_name(path, package).into_iter().collect();
    violations.extend(lint_override(path, &manifest));
    violations.extend(versions(path, text, &manifest));
    Ok(violations)
}

fn crate_name(path: &Path, package: &Table) -> Option<Violation> {
    let name = package.get("name").and_then(Value::as_str);
    let expected = path.parent().and_then(expected_name);
    match (expected, name) {
        (Some(expected), Some(name)) if expected == name => None,
        (Some(expected), _) => Some(Violation::new(
            Check::CrateNames,
            path,
            format!("the package name must be {expected}"),
        )),
        (None, _) => Some(Violation::new(
            Check::CrateNames,
            path,
            "docs/architecture.md section 3 has no crate in this directory",
        )),
    }
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

fn lint_override(path: &Path, manifest: &Table) -> Option<Violation> {
    let lints = manifest.get("lints").and_then(Value::as_table);
    let is_inherited = lints.is_some_and(|lints| {
        lints.len() == 1 && lints.get("workspace").and_then(Value::as_bool) == Some(true)
    });
    (!is_inherited).then(|| {
        let detail = "the [lints] table must contain only workspace = true";
        Violation::new(Check::LintOverride, path, detail)
    })
}

/// The place of a dependency table in a manifest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Scope {
    /// A table of the package, for example `[dependencies]`.
    Package,
    /// A table of a target, for example `[target.'cfg(windows)'.dependencies]`.
    Target,
}

fn versions(path: &Path, text: &str, manifest: &Table) -> Vec<Violation> {
    let mut violations: Vec<_> = dependencies(manifest)
        .filter(|(_, _, _, declaration)| !is_inherited(declaration))
        .map(|(scope, table, name, _)| {
            let detail = format!("the dependency {name} must use workspace = true");
            match line_of(text, scope, table, name) {
                Some(index) => Violation::with_line(Check::Versions, path, index, detail),
                None => Violation::new(Check::Versions, path, detail),
            }
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

/// Returns the index of the line that declares the dependency `name` in a dependency table, as a
/// key of the table or as the header `[<table>.<name>]`.
fn line_of(text: &str, scope: Scope, table: &str, name: &str) -> Option<usize> {
    let own_header = format!("{table}.{name}");
    let mut header = "";
    text.lines().position(|line| {
        let line = line.trim();
        if let Some(inner) = line
            .strip_prefix('[')
            .and_then(|rest| rest.strip_suffix(']'))
        {
            header = inner.trim();
            return is_table(header, scope, &own_header);
        }
        is_table(header, scope, table) && declares(line, name)
    })
}

fn is_table(header: &str, scope: Scope, table: &str) -> bool {
    match scope {
        Scope::Package => header == table,
        Scope::Target => {
            header.starts_with("target.")
                && header
                    .strip_suffix(table)
                    .is_some_and(|rest| rest.ends_with('.'))
        }
    }
}

fn declares(line: &str, name: &str) -> bool {
    line.strip_prefix(name)
        .is_some_and(|rest| rest.trim_start().starts_with(['=', '.']))
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let fixture = include_str!("../../tests/fixtures/crate_names.toml");

        assert_eq!(
            found(
                "crates/engines/qwen3/Cargo.toml",
                fixture,
                Check::CrateNames
            ),
            [None]
        );
        assert_eq!(
            found("crates/sound/fx/Cargo.toml", fixture, Check::CrateNames),
            [None]
        );
        assert!(
            found(
                "crates/storage/qwen3/Cargo.toml",
                fixture,
                Check::CrateNames
            )
            .is_empty()
        );
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
        let fixture = include_str!("../../tests/fixtures/lint_override.toml");

        let lines = found("crates/text/Cargo.toml", fixture, Check::LintOverride);

        assert_eq!(lines, [None]);
    }

    #[test]
    fn lint_repo_finds_versions() {
        let fixture = include_str!("../../tests/fixtures/versions.toml");

        let lines = found("crates/text/Cargo.toml", fixture, Check::Versions);

        assert_eq!(lines, [Some(10), Some(11), Some(16), Some(18), Some(22)]);
    }

    #[test]
    fn manifest_checks_skip_workspace_manifest() {
        let text = "[workspace]\nmembers = []\n";

        assert!(check(Path::new("Cargo.toml"), text).unwrap().is_empty());
    }
}
