# ADR 0007. MP3 as the default export format

## Status

Accepted, 2026-10-06.

## Context

An export is an episode file that the user shares or uploads to a podcast host. The export formats are MP3, WAV and Ogg Opus. Each podcast host and each player accepts MP3. Some hosts do not accept Ogg Opus. WAV files are large, approximately 2.9 MB for each minute of 24 kHz 16-bit mono audio.

## Decision

1. `ExportFormat::default()` is `Mp3`.
2. The MP3 encoder is LAME through `mp3lame-encoder`, at 64 kbps CBR mono.
3. WAV exports are 16-bit PCM mono. Ogg exports are Opus at 48 kbps mono.
4. `ExportFormat` is in `antenna-core`, because `antenna-audio`, `antenna-library` and the apps use it. Its text form is the file extension.

## Consequences

1. A first export plays on all common podcast hosts and players with no change of a setting.
2. One minute of MP3 audio is approximately 0.5 MB.
3. LAME has the LGPL license, which is compatible with GPL-3.0-or-later.
4. The user can select WAV or Ogg Opus in Settings and in the inspector of the Studio.
