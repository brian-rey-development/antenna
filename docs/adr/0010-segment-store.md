# ADR 0010. Segment store

## Status

Accepted, 2026-10-06.

## Context

Synthesis runs faster than real time, but a long document needs minutes. The user can play, seek and export a document while a job runs. The user also edits a document after a job and expects Antenna to synthesize only the changed text.

The Library screen shows the status of 1000 documents. The app must start in 300 ms or less. Thus it cannot read the audio files of each document at start.

## Decision

1. Generation and playback are separate. The pipeline writes each segment to the segment store. The player, the seek and the export read from the store. A job that runs also sends its chunks to the player, so playback starts before the first segment is complete.
2. The segment key is the SHA-256 of the engine id, the engine version, the voice, the quality and the segment text. Each field enters the hash with its length as a `u64` before its bytes.
3. Because the key contains the text, an edit changes only the keys of the changed segments. Two documents with the same sentence and voice share one file.
4. The store keeps 16-bit mono PCM in WAV files, named `<key>.wav`. The writer and the reader convert with the `PCM_SCALE` constant of `antenna-core`.
5. The writer makes a temporary file and renames it at `commit`. A reader never sees a partial file. If the key exists already, `commit` keeps the existing file.
6. The status of a document comes from `document.toml` only. The file holds the text hash, the voice, the segment keys, the completion flag and the last export. The progress of a job that runs is in memory only.
7. A background thread deletes each segment file that no document uses and that is older than one hour. The age limit protects a job that runs now, also in another process.

## Consequences

1. An edit costs the synthesis of the changed segments only. A change of the voice or the quality makes new keys for all segments.
2. A 16-bit sample has a rounding error of 1/32767 or less. A file is half the size of an `f32` file, approximately 2.9 MB for each minute at 24 kHz.
3. `Library::open` reads one small file for each document. It reads no text file and no audio file.
4. After a crash during a job, the status is `Draft`, because the app never stored the completion flag.
5. If a user deletes a segment file outside Antenna, the status stays `Ready`. The next job synthesizes the missing segment.
6. Segments of deleted documents stay for one hour at least. Then garbage collection deletes them.
7. A crash between the two writes of `save_text` leaves the new text with the old metadata. The status stays `Ready` until `Library::load` reads the text, compares its hash and stores it. Then the status is `Draft`.
8. A function that changes a document reads, changes and writes it. The apps must call these functions from one thread.
