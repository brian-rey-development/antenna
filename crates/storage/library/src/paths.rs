use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use antenna_core::{TextFormat, app_dirs};
use directories::ProjectDirs;

use crate::{DocumentId, LibraryError, SegmentKey};

const DATA_DIR_ENV: &str = "ANTENNA_DATA_DIR";
const LIBRARY_DIR_NAME: &str = "library";
const META_FILE_NAME: &str = "document.toml";
const MARKDOWN_FILE_NAME: &str = "text.md";
const PLAIN_FILE_NAME: &str = "text.txt";
const SEGMENTS_DIR_NAME: &str = "segments";
const SEGMENT_KEY_PREFIX_CHARS: usize = 2;
pub(crate) const SEGMENT_EXTENSION: &str = "wav";

pub(crate) fn data_root() -> Result<PathBuf, LibraryError> {
    let dirs = ProjectDirs::from(
        app_dirs::QUALIFIER,
        app_dirs::ORGANIZATION,
        app_dirs::APPLICATION,
    );
    default_root(env::var_os(DATA_DIR_ENV), dirs)
}

pub(crate) fn default_root(
    env: Option<OsString>,
    dirs: Option<ProjectDirs>,
) -> Result<PathBuf, LibraryError> {
    match env.filter(|path| !path.is_empty()) {
        Some(path) => Ok(PathBuf::from(path)),
        None => dirs
            .map(|dirs| dirs.data_dir().to_owned())
            .ok_or(LibraryError::NoDataDir),
    }
}

pub(crate) fn library_dir(root: &Path) -> PathBuf {
    root.join(LIBRARY_DIR_NAME)
}

pub(crate) fn document_dir(library_dir: &Path, id: DocumentId) -> PathBuf {
    library_dir.join(id.to_string())
}

pub(crate) fn metadata_path(document_dir: &Path) -> PathBuf {
    document_dir.join(META_FILE_NAME)
}

pub(crate) fn text_path(document_dir: &Path, format: TextFormat) -> PathBuf {
    document_dir.join(match format {
        TextFormat::Markdown => MARKDOWN_FILE_NAME,
        TextFormat::Plain => PLAIN_FILE_NAME,
    })
}

pub(crate) fn segments_dir(root: &Path) -> PathBuf {
    root.join(SEGMENTS_DIR_NAME)
}

pub(crate) fn segment_path(segments_dir: &Path, key: &SegmentKey) -> PathBuf {
    let text = key.to_string();
    let prefix: String = text.chars().take(SEGMENT_KEY_PREFIX_CHARS).collect();
    segments_dir
        .join(prefix)
        .join(format!("{text}.{SEGMENT_EXTENSION}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project_dirs() -> ProjectDirs {
        ProjectDirs::from(
            app_dirs::QUALIFIER,
            app_dirs::ORGANIZATION,
            app_dirs::APPLICATION,
        )
        .unwrap()
    }

    #[test]
    fn default_root_uses_env_when_set() {
        let root = default_root(Some("/data/antenna".into()), Some(project_dirs()));

        assert_eq!(root.unwrap(), PathBuf::from("/data/antenna"));
    }

    #[test]
    fn default_root_uses_data_dir_when_env_absent() {
        let dirs = project_dirs();

        let root = default_root(None, Some(dirs.clone()));

        assert_eq!(root.unwrap(), dirs.data_dir());
    }

    #[test]
    fn default_root_uses_data_dir_when_env_empty() {
        let dirs = project_dirs();

        let root = default_root(Some(OsString::new()), Some(dirs.clone()));

        assert_eq!(root.unwrap(), dirs.data_dir());
    }

    #[test]
    fn default_root_fails_when_env_and_data_dir_absent() {
        let result = default_root(None, None);

        assert!(matches!(result, Err(LibraryError::NoDataDir)));
    }

    #[test]
    fn segment_path_uses_key_prefix_directory() {
        let key: SegmentKey = format!("ab{}", "0".repeat(62)).parse().unwrap();

        let path = segment_path(Path::new("segments"), &key);

        let expected = Path::new("segments")
            .join("ab")
            .join(format!("ab{}.wav", "0".repeat(62)));
        assert_eq!(path, expected);
    }
}
