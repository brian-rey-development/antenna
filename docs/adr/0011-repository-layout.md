# ADR 0011. Repository layout

## Status

Accepted, 2026-10-06.

## Context

AI agents and humans do the 19 stages of the implementation, also at the same time. Each agent must find each part of the system at a fixed location. A layout with no rules grows container modules, cycles between crates and files in the repository root that nobody owns.

## Decision

1. The tree of `docs/architecture.md` section 3 is the only permitted layout. The repository root contains only the entries of that tree.
2. `crates/` has exactly three group directories, `storage/`, `inference/` and `engines/`. Each other directory under `crates/` is one crate.
3. The package name follows from the directory. The name of `crates/<name>` is `antenna-<name>`. A crate in `storage/` or `inference/` has the same rule. The name of a crate in `engines/` is `antenna-engine-<name>`. The packages outside `crates/` have fixed names.
4. The table in `docs/architecture.md` section 3.1 is the only permitted dependency graph. `tools/xtask/src/graph.rs` contains the same table as data.
5. Each member crate inherits its dependencies and its lints from the workspace.
6. The names `util`, `common`, `shared`, `manager` and the other names of `docs/standards.md` section 3.2 are prohibited.
7. `cargo xtask lint-repo` checks each rule of this list that a tool can check.

## Consequences

1. A reader who knows one crate can find each part of the other crates.
2. A dependency that the graph does not permit fails CI. A new crate or a new edge needs a human decision and a change to the table.
3. Each version of a third-party crate is in one place, the root `Cargo.toml`.
4. A new tool configuration file goes in `.config/`, not in the repository root.
