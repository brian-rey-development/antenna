//! Round trips of the export to MP3, WAV and Ogg Opus, the loudness and the failure cases.

#[cfg(test)]
mod failure;
#[cfg(test)]
#[path = "../fixtures/mod.rs"]
mod fixtures;
#[cfg(test)]
mod loudness;
#[cfg(test)]
mod mp3;
#[cfg(test)]
mod ogg;
#[cfg(test)]
mod probe;
#[cfg(test)]
mod support;
#[cfg(test)]
mod wav;
