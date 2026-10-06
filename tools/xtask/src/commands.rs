//! The two commands of `xtask`.

use std::env;
use std::ffi::{OsStr, OsString};
use std::path::Path;
use std::process::Command;

use cargo_metadata::Metadata;

use crate::error::XtaskError;
use crate::{lint, output};

const LINT_REPO: &str = "cargo xtask lint-repo";
/// `cargo run` gives xtask the variables of its package. A step must not inherit them, because
/// cargo-machete reads `CARGO_PKG_NAME` to parse its arguments.
const PACKAGE_VARIABLE_PREFIX: &str = "CARGO_PKG_";

/// One step of `cargo xtask check`: the arguments of a cargo subcommand and its environment.
struct Step {
    args: &'static [&'static str],
    env: &'static [(&'static str, &'static str)],
}

impl Step {
    fn command(&self) -> String {
        format!("cargo {}", self.args.join(" "))
    }
}

/// The steps of `docs/standards.md` section 1, before the repository checks. cargo-deny 0.20
/// takes `--config` before the `check` subcommand.
const STEPS: [Step; 7] = [
    Step {
        args: &["fmt", "--all", "--check"],
        env: &[],
    },
    Step {
        args: &[
            "clippy",
            "--workspace",
            "--all-targets",
            "--all-features",
            "--",
            "-D",
            "warnings",
        ],
        env: &[],
    },
    Step {
        args: &["nextest", "run", "--workspace", "--all-features"],
        env: &[],
    },
    Step {
        args: &["test", "--workspace", "--all-features", "--doc"],
        env: &[],
    },
    Step {
        args: &["doc", "--workspace", "--all-features", "--no-deps"],
        env: &[("RUSTDOCFLAGS", "-D warnings")],
    },
    Step {
        args: &["deny", "--config", ".config/deny.toml", "check"],
        env: &[],
    },
    Step {
        args: &["machete"],
        env: &[],
    },
];

/// Runs each check step in sequence and stops at the first failure.
///
/// # Errors
///
/// Returns the error of the first step that fails.
pub(crate) fn check(metadata: &Metadata) -> Result<(), XtaskError> {
    let cargo = env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"));
    for step in &STEPS {
        output::step(&step.command());
        run_step(&cargo, step, metadata.workspace_root.as_std_path())?;
    }
    output::step(LINT_REPO);
    lint_repo(metadata)?;
    output::passed("cargo xtask check");
    Ok(())
}

fn run_step(cargo: &OsStr, step: &Step, root: &Path) -> Result<(), XtaskError> {
    let mut command = Command::new(cargo);
    command
        .args(step.args)
        .envs(step.env.iter().copied())
        .current_dir(root);
    for (name, _) in env::vars_os() {
        if name.to_string_lossy().starts_with(PACKAGE_VARIABLE_PREFIX) {
            command.env_remove(name);
        }
    }
    let status = command.status().map_err(|source| XtaskError::Spawn {
        program: cargo.to_string_lossy().into_owned(),
        source,
    })?;
    if !status.success() {
        return Err(XtaskError::Failed {
            command: step.command(),
            status,
        });
    }
    Ok(())
}

/// Runs the repository checks and prints each violation.
///
/// # Errors
///
/// Returns [`XtaskError::Violations`] if a check fails, or the error that stopped the checks.
pub(crate) fn lint_repo(metadata: &Metadata) -> Result<(), XtaskError> {
    let violations = lint::run(metadata)?;
    violations.iter().for_each(output::violation);
    if violations.is_empty() {
        output::passed(LINT_REPO);
        Ok(())
    } else {
        Err(XtaskError::Violations {
            count: violations.len(),
        })
    }
}
