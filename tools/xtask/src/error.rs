use std::io;
use std::path::PathBuf;
use std::process::ExitStatus;
use std::string::FromUtf8Error;

use thiserror::Error;
use toml::de;

/// An error of an `xtask` command.
#[derive(Debug, Error)]
pub(crate) enum XtaskError {
    #[error("the command is missing, the commands are \"check\" and \"lint-repo\"")]
    MissingCommand,
    #[error("the command {argument} is unknown, the commands are \"check\" and \"lint-repo\"")]
    UnknownCommand { argument: String },
    #[error("cannot start {program}")]
    Spawn {
        program: String,
        #[source]
        source: io::Error,
    },
    #[error("{command} failed with {status}")]
    Failed { command: String, status: ExitStatus },
    #[error("git listed a path that is not UTF-8")]
    GitListing(#[source] FromUtf8Error),
    #[error("cannot read {}", .path.display())]
    Read {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("cannot parse {}", .path.display())]
    Manifest {
        path: PathBuf,
        #[source]
        source: de::Error,
    },
    #[error("cannot read the workspace metadata")]
    Metadata(#[source] cargo_metadata::Error),
    #[error("the repository checks found {count} violations")]
    Violations { count: usize },
}
