use std::fmt::Write as _;

use antenna_core::{Artifact, Extent};
use sha2::{Digest, Sha256};

pub(crate) const REPO: &str = "antenna/test";
pub(crate) const REVISION: &str = "0123456789abcdef0123456789abcdef01234567";

const BYTE_PERIOD: usize = 251;

pub(crate) fn leak<T>(value: T) -> &'static T {
    Box::leak(Box::new(value))
}

pub(crate) fn pattern(len: usize) -> Vec<u8> {
    (0..len)
        .map(|index| u8::try_from(index % BYTE_PERIOD).unwrap())
        .collect()
}

pub(crate) fn whole(key: &str, bytes: &[u8]) -> &'static Artifact {
    artifact(
        key,
        &format!("{key}.bin"),
        Extent::Whole {
            bytes: bytes.len() as u64,
        },
        bytes,
    )
}

pub(crate) fn range(key: &str, archive: &[u8], offset: usize, len: usize) -> &'static Artifact {
    let extent = Extent::Range {
        offset: offset as u64,
        bytes: len as u64,
    };
    artifact(key, "archive.nemo", extent, &archive[offset..offset + len])
}

fn artifact(key: &str, path: &str, extent: Extent, hashed: &[u8]) -> &'static Artifact {
    let hash = Sha256::digest(hashed)
        .iter()
        .fold(String::new(), |mut hex, byte| {
            write!(hex, "{byte:02x}").unwrap();
            hex
        });
    leak(Artifact {
        key: Box::leak(key.to_owned().into_boxed_str()),
        repo: REPO,
        revision: REVISION,
        path: Box::leak(path.to_owned().into_boxed_str()),
        extent,
        sha256: Box::leak(hash.into_boxed_str()),
    })
}
