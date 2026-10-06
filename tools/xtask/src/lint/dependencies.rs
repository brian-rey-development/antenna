//! The check of the workspace dependencies against the graph in `graph.rs`.

use std::path::{Path, PathBuf};

use cargo_metadata::{DependencyKind, Metadata};
use strum::Display;

use super::{Check, MANIFEST_NAME, Violation};
use crate::graph::{GRAPH, Row};

/// The column of the graph that permits a dependency.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Display)]
#[strum(serialize_all = "lowercase")]
pub(super) enum Kind {
    Normal,
    Dev,
}

/// A workspace member, with its manifest path and the directories of its path dependencies.
#[derive(Debug)]
pub(super) struct Member {
    manifest: PathBuf,
    dependencies: Vec<(Kind, PathBuf)>,
}

/// Reads the members and their path dependencies from the workspace metadata. Each path is
/// relative to the workspace root.
pub(super) fn members(metadata: &Metadata) -> Vec<Member> {
    let root = metadata.workspace_root.as_std_path();
    let relative = |path: &Path| path.strip_prefix(root).unwrap_or(path).to_owned();
    metadata
        .workspace_packages()
        .into_iter()
        .map(|package| Member {
            manifest: relative(package.manifest_path.as_std_path()),
            dependencies: package
                .dependencies
                .iter()
                .filter_map(|dependency| {
                    let path = dependency.path.as_ref()?;
                    Some((kind_of(dependency.kind), relative(path.as_std_path())))
                })
                .collect(),
        })
        .collect()
}

fn kind_of(kind: DependencyKind) -> Kind {
    match kind {
        DependencyKind::Development => Kind::Dev,
        DependencyKind::Normal | DependencyKind::Build | DependencyKind::Unknown => Kind::Normal,
    }
}

/// Finds each member with no row in the graph and each dependency that its row does not permit.
pub(super) fn check(members: &[Member]) -> Vec<Violation> {
    members.iter().flat_map(check_member).collect()
}

fn check_member(member: &Member) -> Vec<Violation> {
    let violation = |detail| Violation::new(Check::DependencyGraph, &member.manifest, detail);
    let Some(row) = GRAPH
        .iter()
        .find(|row| Path::new(row.path).join(MANIFEST_NAME) == member.manifest)
    else {
        return vec![violation(
            "the workspace member has no row in the graph".to_owned(),
        )];
    };
    member
        .dependencies
        .iter()
        .filter(|(kind, dependency)| !is_permitted(row, *kind, dependency))
        .map(|(kind, dependency)| {
            let dependency = dependency.display();
            violation(format!(
                "the graph does not permit the {kind} dependency on {dependency}"
            ))
        })
        .collect()
}

fn is_permitted(row: &Row, kind: Kind, dependency: &Path) -> bool {
    let permitted = match kind {
        Kind::Normal => row.normal,
        Kind::Dev => row.dev,
    };
    permitted.iter().any(|path| Path::new(path) == dependency)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Parses a fixture with one line for each member and one line for each dependency, in the
    /// form `<member> [normal|dev <dependency>]`.
    fn parse(fixture: &str) -> Vec<Member> {
        let mut members: Vec<Member> = Vec::new();
        for line in fixture.lines() {
            let words: Vec<_> = line.split_whitespace().collect();
            let manifest = Path::new(words[0]).join(MANIFEST_NAME);
            if !members.iter().any(|member| member.manifest == manifest) {
                members.push(Member {
                    manifest: manifest.clone(),
                    dependencies: Vec::new(),
                });
            }
            if let [_, kind, dependency] = words.as_slice() {
                let kind = if *kind == "dev" {
                    Kind::Dev
                } else {
                    Kind::Normal
                };
                let member = members
                    .iter_mut()
                    .find(|member| member.manifest == manifest)
                    .unwrap();
                member.dependencies.push((kind, PathBuf::from(dependency)));
            }
        }
        members
    }

    fn details(fixture: &str) -> Vec<(PathBuf, String)> {
        check(&parse(fixture))
            .into_iter()
            .map(|violation| (violation.path, violation.detail))
            .collect()
    }

    #[test]
    fn lint_repo_finds_dependency_graph() {
        let fixture = include_str!("../../tests/fixtures/dependency_graph.txt");

        let found = details(fixture);

        let detail = "the workspace member has no row in the graph".to_owned();
        assert_eq!(found, [(PathBuf::from("crates/sound/Cargo.toml"), detail)]);
    }

    #[test]
    fn lint_repo_finds_forbidden_dependency() {
        let fixture = include_str!("../../tests/fixtures/forbidden_dependency.txt");

        let found = details(fixture);

        let expected = [
            ("crates/core/Cargo.toml", "normal dependency on crates/text"),
            (
                "crates/text/Cargo.toml",
                "dev dependency on crates/engines/fake",
            ),
        ]
        .map(|(path, end)| {
            (
                PathBuf::from(path),
                format!("the graph does not permit the {end}"),
            )
        });
        assert_eq!(found, expected);
    }

    #[test]
    fn graph_has_one_row_for_each_crate() {
        let mut paths: Vec<_> = GRAPH.iter().map(|row| row.path).collect();
        paths.sort_unstable();
        paths.dedup();

        assert_eq!(paths.len(), GRAPH.len());
    }
}
