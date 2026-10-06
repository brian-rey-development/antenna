//! The repository checks of `docs/standards.md` section 13.

mod code;
mod comments;
mod dashes;
mod dependencies;
mod design;
mod manifest;
mod names;
mod source;

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::Command;

use cargo_metadata::Metadata;
use strum::Display;

use crate::error::XtaskError;
use source::SourceLine;

/// The directories that contain the Rust files of the workspace.
const RUST_ROOTS: [&str; 3] = ["crates", "apps", "tools"];
const MANIFEST_NAME: &str = "Cargo.toml";

/// A repository check, with the name of its row in `docs/standards.md` section 13.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Display)]
#[strum(serialize_all = "lowercase")]
pub(crate) enum Check {
    #[strum(serialize = "file length")]
    FileLength,
    #[strum(serialize = "banned markers")]
    BannedMarkers,
    #[strum(serialize = "banner comments")]
    BannerComments,
    #[strum(serialize = "commented-out code")]
    CommentedOutCode,
    #[strum(serialize = "comment density")]
    CommentDensity,
    Dashes,
    Names,
    #[strum(serialize = "crate names")]
    CrateNames,
    #[strum(serialize = "lint override")]
    LintOverride,
    Versions,
    #[strum(serialize = "dependency graph")]
    DependencyGraph,
    #[strum(serialize = "exhaustive enums")]
    ExhaustiveEnums,
    #[strum(serialize = "unsafe code")]
    UnsafeCode,
    #[strum(serialize = "raw design values")]
    RawDesignValues,
    #[strum(serialize = "component library")]
    ComponentLibrary,
    #[strum(serialize = "desktop layers")]
    DesktopLayers,
}

/// A failure of one repository check at one location.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Violation {
    /// The path of the file, relative to the workspace root.
    pub(crate) path: PathBuf,
    /// The line number, from 1, when the failure has a line.
    pub(crate) line: Option<usize>,
    pub(crate) check: Check,
    /// The failure condition that the file meets.
    pub(crate) detail: String,
}

impl Violation {
    /// Makes a violation at a line index, from 0.
    fn with_line(check: Check, path: &Path, index: usize, detail: impl Into<String>) -> Self {
        Self {
            path: path.to_owned(),
            line: Some(index + 1),
            check,
            detail: detail.into(),
        }
    }

    /// Makes a violation of a complete file.
    fn new(check: Check, path: &Path, detail: impl Into<String>) -> Self {
        Self {
            path: path.to_owned(),
            line: None,
            check,
            detail: detail.into(),
        }
    }
}

/// A Rust file of the workspace, lexed one time for all checks.
#[derive(Debug)]
struct RustFile<'a> {
    path: &'a Path,
    text: &'a str,
    lines: Vec<SourceLine>,
}

impl<'a> RustFile<'a> {
    fn new(path: &'a Path, text: &'a str) -> Self {
        Self {
            path,
            text,
            lines: source::lex(text),
        }
    }
}

/// Runs all repository checks on the files that git tracks.
///
/// # Errors
///
/// Returns an error if git, a file read or a manifest parse fails. The violations are not errors.
pub(crate) fn run(metadata: &Metadata) -> Result<Vec<Violation>, XtaskError> {
    let root = metadata.workspace_root.as_std_path();
    let files = tracked_files(root)?;
    let mut violations = names::check_directories(&files);
    for path in &files {
        if let Some(text) = read_text(&root.join(path))? {
            violations.extend(check_file(path, &text)?);
        }
    }
    violations.extend(dependencies::check(&dependencies::members(metadata)));
    violations.sort_by(|first, second| (&first.path, first.line).cmp(&(&second.path, second.line)));
    Ok(violations)
}

fn check_file(path: &Path, text: &str) -> Result<Vec<Violation>, XtaskError> {
    let mut violations = dashes::check(path, text);
    if is_rust_file(path) {
        let file = RustFile::new(path, text);
        violations.extend(code::check(&file));
        violations.extend(comments::check(&file));
        violations.extend(design::check(&file));
        violations.extend(names::check_file(&file));
    }
    if path.file_name().is_some_and(|name| name == MANIFEST_NAME) {
        violations.extend(manifest::check(path, text)?);
    }
    Ok(violations)
}

fn is_rust_file(path: &Path) -> bool {
    path.extension().is_some_and(|extension| extension == "rs")
        && RUST_ROOTS.iter().any(|root| path.starts_with(root))
}

fn tracked_files(root: &Path) -> Result<Vec<PathBuf>, XtaskError> {
    let output = Command::new("git")
        .args(["ls-files", "-z"])
        .current_dir(root)
        .output()
        .map_err(|source| XtaskError::Spawn {
            program: "git".to_owned(),
            source,
        })?;
    if !output.status.success() {
        return Err(XtaskError::Failed {
            command: "git ls-files".to_owned(),
            status: output.status,
        });
    }
    let listing = String::from_utf8(output.stdout).map_err(XtaskError::GitListing)?;
    Ok(listing
        .split('\0')
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .collect())
}

/// Reads a tracked file. Returns `None` for a file that is not UTF-8 text or that the work tree
/// deleted.
fn read_text(path: &Path) -> Result<Option<String>, XtaskError> {
    match fs::read(path) {
        Ok(bytes) => Ok(String::from_utf8(bytes).ok()),
        Err(source) if source.kind() == ErrorKind::NotFound => Ok(None),
        Err(source) => Err(XtaskError::Read {
            path: path.to_owned(),
            source,
        }),
    }
}
