# ADR 0013. Argument order of cargo deny

## Status

Accepted, 2026-10-06.

## Context

`docs/standards.md` section 1 step 6 and the check of AC-01-16 had the command `cargo deny check --config .config/deny.toml`. cargo-deny 0.20 accepts `--config` only before the subcommand. With the old order, cargo-deny 0.20.2 prints `error: unexpected argument '--config' found` and exits with code 2.

CI and the README install cargo-deny 0.20.2.

## Decision

1. The command is `cargo deny --config .config/deny.toml check`. `cargo xtask check` runs it. `docs/standards.md` section 1 step 6 and the check of AC-01-16 show it.
2. CI and the README pin cargo-deny 0.20.2.
3. Before a change of the pinned version, run `cargo xtask check` with the new version.

## Consequences

1. The documents, `cargo xtask check` and CI use the same argument order.
2. The rules of `docs/standards.md` did not change. Only the syntax of one command changed.
3. If a new cargo-deny version changes the argument order again, `cargo xtask check` fails. The fix changes `tools/xtask/src/commands.rs`, `docs/standards.md`, AC-01-16 and this ADR.
