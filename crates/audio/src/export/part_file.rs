use std::fs::{self, File};
use std::io::{self, BufWriter, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use crate::AudioError;

const EXPORT_BUFFER_BYTES: usize = 64 * 1024;
const PART_EXTENSION: &str = ".part";

/// A file that an export writes to. It has the path `<destination>.part` until `commit` renames
/// it. A drop before `commit` deletes it.
// The fields drop in this order. The file closes before the cleanup deletes it, and Windows
// cannot delete an open file.
#[derive(Debug)]
pub(crate) struct PartFile {
    writer: BufWriter<File>,
    destination: PathBuf,
    cleanup: Cleanup,
}

#[derive(Debug)]
struct Cleanup {
    part: PathBuf,
    is_committed: bool,
}

impl PartFile {
    pub(crate) fn open(destination: &Path) -> Result<Self, AudioError> {
        let mut part = destination.as_os_str().to_owned();
        part.push(PART_EXTENSION);
        let part = PathBuf::from(part);
        let file = File::create(&part).map_err(|source| write_error(destination, source))?;
        Ok(Self {
            writer: BufWriter::with_capacity(EXPORT_BUFFER_BYTES, file),
            destination: destination.to_owned(),
            cleanup: Cleanup {
                part,
                is_committed: false,
            },
        })
    }

    pub(crate) fn destination(&self) -> &Path {
        &self.destination
    }

    pub(crate) fn commit(self) -> Result<u64, AudioError> {
        let Self {
            writer,
            destination,
            mut cleanup,
        } = self;
        let file = writer
            .into_inner()
            .map_err(|error| write_error(&destination, error.into_error()))?;
        let bytes = file
            .metadata()
            .map_err(|source| write_error(&destination, source))?
            .len();
        drop(file);
        fs::rename(&cleanup.part, &destination)
            .map_err(|source| write_error(&destination, source))?;
        cleanup.is_committed = true;
        Ok(bytes)
    }
}

pub(crate) fn write_error(path: &Path, source: io::Error) -> AudioError {
    AudioError::Write {
        path: path.to_owned(),
        source,
    }
}

impl Write for PartFile {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.writer.write(bytes)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

impl Seek for PartFile {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        self.writer.seek(position)
    }
}

impl Drop for Cleanup {
    fn drop(&mut self) {
        if self.is_committed {
            return;
        }
        if let Err(error) = fs::remove_file(&self.part) {
            tracing::warn!(path = %self.part.display(), ?error, "cannot delete the partial export file");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn part_exists(directory: &Path) -> bool {
        directory.join("episode.wav.part").exists()
    }

    #[test]
    fn part_file_writes_to_part_path_when_open() {
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("episode.wav");

        let mut part = PartFile::open(&destination).unwrap();
        part.write_all(b"abc").unwrap();
        part.flush().unwrap();

        assert!(part_exists(directory.path()));
        assert!(!destination.exists());
    }

    #[test]
    fn part_file_renames_when_committed() {
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("episode.wav");
        let mut part = PartFile::open(&destination).unwrap();
        part.write_all(b"abc").unwrap();

        let bytes = part.commit().unwrap();

        assert_eq!(bytes, 3);
        assert_eq!(fs::read(&destination).unwrap(), b"abc");
        assert!(!part_exists(directory.path()));
    }

    #[test]
    fn part_file_deletes_file_when_dropped_before_commit() {
        let directory = tempfile::tempdir().unwrap();
        let mut part = PartFile::open(&directory.path().join("episode.wav")).unwrap();
        part.write_all(b"abc").unwrap();

        drop(part);

        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 0);
    }

    #[test]
    fn part_file_deletes_file_when_rename_fails() {
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("episode.wav");
        fs::create_dir(&destination).unwrap();
        let part = PartFile::open(&destination).unwrap();

        let result = part.commit();

        assert!(matches!(result, Err(AudioError::Write { .. })));
        assert!(!part_exists(directory.path()));
    }

    #[test]
    fn part_file_fails_when_directory_missing() {
        let directory = tempfile::tempdir().unwrap();

        let result = PartFile::open(&directory.path().join("missing").join("episode.wav"));

        assert!(matches!(result, Err(AudioError::Write { .. })));
    }

    #[test]
    fn part_file_supports_seek_when_header_rewritten() {
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("episode.bin");
        let mut part = PartFile::open(&destination).unwrap();
        part.write_all(b"0123456789").unwrap();

        part.seek(SeekFrom::Start(2)).unwrap();
        part.write_all(b"ab").unwrap();
        part.commit().unwrap();

        assert_eq!(fs::read(&destination).unwrap(), b"01ab456789");
    }
}
