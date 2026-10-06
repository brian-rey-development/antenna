# ADR 0001. GPL-3.0-or-later license

## Status

Accepted, 2026-10-06.

## Context

Antenna is a desktop app that converts documents into speech with local models. The code depends on some crates with a copyleft license. LAME, the MP3 encoder of `mp3lame-encoder`, has the LGPL license. The project owner wants each copy and each fork of Antenna to stay free software.

The model weights are separate works. Antenna downloads them at runtime from Hugging Face. Each weight file keeps its own license, for example Apache-2.0 or the NVIDIA Open Model License.

## Decision

1. The license of all code in this repository is GPL-3.0-or-later.
2. `[workspace.package]` sets `license = "GPL-3.0-or-later"`, and each member inherits it.
3. `LICENSE` contains the full GPL-3.0 text from gnu.org.
4. `cargo deny` permits only the licenses that are compatible with GPL-3.0-or-later. Stage 01 sets the allowlist in `.config/deny.toml`.
5. The Models screen and the About section show the license and the attribution text of each model.

## Consequences

1. A distributor of Antenna must also give the source code under the same license.
2. A new dependency with a license that is not in the allowlist fails `cargo xtask check`. A human must approve a change to the allowlist.
3. The weights do not change the license of the code, because Antenna does not include them.
4. Code from a repository with no license file cannot go into Antenna.
