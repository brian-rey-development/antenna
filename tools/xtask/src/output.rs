#![expect(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "this module is the only terminal output of xtask"
)]

use std::error::Error;

use crate::error::XtaskError;
use crate::lint::Violation;

/// Prints the command of a check step before it runs.
pub(crate) fn step(command: &str) {
    println!("xtask: {command}");
}

/// Prints one violation with its file and line.
pub(crate) fn violation(violation: &Violation) {
    let path = violation.path.display();
    let location = violation
        .line
        .map_or_else(|| path.to_string(), |line| format!("{path}:{line}"));
    println!("{location}: {}: {}", violation.check, violation.detail);
}

/// Prints the success message of a command.
pub(crate) fn passed(command: &str) {
    println!("xtask: {command} passed");
}

/// Prints an error and its chain of sources.
pub(crate) fn error(error: &XtaskError) {
    eprintln!("xtask: {error}");
    let mut source = error.source();
    while let Some(cause) = source {
        eprintln!("  caused by: {cause}");
        source = cause.source();
    }
}
