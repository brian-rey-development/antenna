# ADR 0012. Exhaustive enums

## Status

Accepted, 2026-10-06.

## Context

`#[non_exhaustive]` lets a published crate add enum variants, and the code of its users continues to compile. A user of such an enum must write a `_ =>` arm. Then the compiler does not find the matches that a new variant affects.

No crate of this workspace is published. All users of an enum are in the same workspace, so a new variant can change all matches in the same commit.

## Decision

1. No item of the workspace has `#[non_exhaustive]`. `cargo xtask lint-repo` fails on the attribute in any Rust file.
2. Code matches all variants of the enums of this workspace. Clippy `wildcard_enum_match_arm` rejects `_ =>` on an enum.
3. The error enums of `antenna-core`, for example `EngineError` and `SinkError`, are exhaustive. Other crates match them with no wildcard arm.

## Consequences

1. When a stage adds a variant, the compiler lists each match that must change.
2. A new variant of a core enum is a change to the contract. The stage that adds it updates each match in the same commit.
3. If a crate becomes a published library, this decision needs a new ADR for that crate.
