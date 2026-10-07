use std::fs;
use std::path::Path;

use serde::de;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{DocumentMeta, LibraryError, atomic, paths};

const META_VERSION: u32 = 1;

pub(crate) fn read_metadata(directory: &Path) -> Result<DocumentMeta, LibraryError> {
    let path = paths::metadata_path(directory);
    let text = fs::read_to_string(&path).map_err(LibraryError::io(&path))?;
    toml::from_str::<MetadataFile<DocumentMeta>>(&text)
        .map(|file| file.metadata)
        .map_err(|source| LibraryError::MetaRead { path, source })
}

pub(crate) fn write_metadata(
    directory: &Path,
    metadata: &DocumentMeta,
) -> Result<(), LibraryError> {
    let path = paths::metadata_path(directory);
    let file = MetadataFile {
        version: MetadataVersion,
        metadata,
    };
    let text = toml::to_string(&file).map_err(|source| LibraryError::MetaWrite {
        path: path.clone(),
        source,
    })?;
    atomic::write(directory, paths::META_FILE_NAME, text.as_bytes())
}

#[derive(Serialize, Deserialize)]
struct MetadataFile<M> {
    version: MetadataVersion,
    #[serde(flatten)]
    metadata: M,
}

struct MetadataVersion;

impl Serialize for MetadataVersion {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u32(META_VERSION)
    }
}

impl<'de> Deserialize<'de> for MetadataVersion {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match u32::deserialize(deserializer)? {
            META_VERSION => Ok(Self),
            other => Err(de::Error::custom(format!(
                "the metadata version {other} is not {META_VERSION}"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::ErrorKind;

    use super::*;
    use crate::fixtures::{bare_metadata, full_metadata};

    fn rewrite(directory: &Path, from: &str, to: &str) {
        let path = paths::metadata_path(directory);
        let text = fs::read_to_string(&path).unwrap().replacen(from, to, 1);
        fs::write(path, text).unwrap();
    }

    #[test]
    fn metadata_round_trips_when_written_and_read() {
        let directory = tempfile::tempdir().unwrap();
        let metadata = full_metadata();

        write_metadata(directory.path(), &metadata).unwrap();
        let read = read_metadata(directory.path()).unwrap();

        assert_eq!(read, metadata);
    }

    #[test]
    fn metadata_round_trips_when_options_absent() {
        let directory = tempfile::tempdir().unwrap();
        let metadata = bare_metadata();

        write_metadata(directory.path(), &metadata).unwrap();
        let read = read_metadata(directory.path()).unwrap();

        assert_eq!(read, metadata);
    }

    #[test]
    fn metadata_file_has_version_and_text_forms() {
        let directory = tempfile::tempdir().unwrap();

        write_metadata(directory.path(), &full_metadata()).unwrap();
        let text = fs::read_to_string(paths::metadata_path(directory.path())).unwrap();

        assert!(text.starts_with("version = 1\n"), "{text}");
        assert!(text.contains("format = \"markdown\""), "{text}");
        assert!(text.contains("language = \"es\""), "{text}");
        assert!(
            text.contains("created = \"1970-01-01T00:16:40Z\""),
            "{text}"
        );
    }

    #[test]
    fn read_metadata_fails_when_version_unknown() {
        let directory = tempfile::tempdir().unwrap();
        write_metadata(directory.path(), &bare_metadata()).unwrap();
        rewrite(directory.path(), "version = 1", "version = 2");

        let result = read_metadata(directory.path());

        assert!(matches!(result, Err(LibraryError::MetaRead { .. })));
    }

    #[test]
    fn read_metadata_fails_when_field_invalid() {
        let directory = tempfile::tempdir().unwrap();
        write_metadata(directory.path(), &bare_metadata()).unwrap();
        rewrite(directory.path(), "format = \"plain\"", "format = \"rtf\"");

        let result = read_metadata(directory.path());

        assert!(matches!(result, Err(LibraryError::MetaRead { .. })));
    }

    #[test]
    fn read_metadata_fails_when_file_is_not_utf8() {
        let directory = tempfile::tempdir().unwrap();
        fs::write(paths::metadata_path(directory.path()), [0xff, 0xfe, 0x00]).unwrap();

        let result = read_metadata(directory.path());

        assert!(
            matches!(&result, Err(LibraryError::Io { source, .. }) if source.kind() == ErrorKind::InvalidData)
        );
    }

    #[test]
    fn read_metadata_fails_when_file_absent() {
        let directory = tempfile::tempdir().unwrap();

        let result = read_metadata(directory.path());

        assert!(matches!(result, Err(LibraryError::Io { .. })));
    }
}
