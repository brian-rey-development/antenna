//! The developer tasks of Antenna.
//!
//! `cargo xtask check` runs the check sequence of `docs/standards.md` section 1.
//! `cargo xtask lint-repo` runs the repository checks of `docs/standards.md` section 13.

mod commands;
mod error;
mod graph;
mod lint;
mod output;

use std::env;
use std::process::ExitCode;

use cargo_metadata::MetadataCommand;

use error::XtaskError;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            output::error(&error);
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), XtaskError> {
    let argument = env::args_os().nth(1).ok_or(XtaskError::MissingCommand)?;
    let command = match argument.to_str() {
        Some("check") => commands::check,
        Some("lint-repo") => commands::lint_repo,
        Some(_) | None => {
            let argument = argument.to_string_lossy().into_owned();
            return Err(XtaskError::UnknownCommand { argument });
        }
    };
    let metadata = MetadataCommand::new()
        .no_deps()
        .exec()
        .map_err(XtaskError::Metadata)?;
    command(&metadata)
}
