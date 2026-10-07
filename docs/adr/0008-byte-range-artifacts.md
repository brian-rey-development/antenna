# ADR 0008. Byte-range artifacts

## Status

Accepted, 2026-10-06.

## Context

The Magpie G2P dictionaries exist only inside an uncompressed `.nemo` archive of 1.47 GB in the Hugging Face repository of the model. The engine needs a few megabytes of this archive. A download of the whole archive for these files wastes time and disk space.

The files of a voice are large, and the first use of a voice downloads gigabytes. A connection can fail in the middle of a file. The user must not lose the bytes that arrived.

## Decision

1. `Artifact` has the field `extent` with two variants. `Extent::Whole` is a complete file. `Extent::Range` is a part of a file with an offset and a size.
2. The model store downloads a range with an HTTP `Range` request. The SHA-256 in the artifact is the hash of the downloaded bytes.
3. The local file of a range has the name `<path>@<offset>+<bytes>`. Two ranges of one archive have two local files. The same range in two engines has one local file.
4. The store writes the bytes into a `.part` file. After a failure, the next attempt sends a `Range` header from the length of the `.part` file. This resume works for a whole file and for a range.
5. If the server ignores the `Range` header, the store restarts a whole file from zero. For a range, the store returns `ModelError::RangeUnsupported`, because a restart would download the full archive.
6. The store does not use `hf-hub`. The store has one download path and one retry policy, and the tests control both against a local HTTP server. `hf-hub` has its own cache layout, so it cannot give the layout of this ADR.

## Consequences

1. Stage 14 declares the Magpie G2P files as ranges of the `.nemo` archive and downloads only their bytes.
2. A server without range support cannot serve a range artifact. Hugging Face supports ranges.
3. The store depends on `ureq` and its redirect handling. A test makes sure that a redirect keeps the `Range` header.
4. A change of the revision of a repository makes new artifacts and new local files. The old files stay until the user removes the engine.
