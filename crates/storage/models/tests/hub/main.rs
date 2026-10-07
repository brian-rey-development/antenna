//! Behavior tests of the model store with a local HTTP server.

#[cfg(test)]
#[path = "../support/mod.rs"]
mod support;

#[cfg(test)]
mod failure;
#[cfg(test)]
mod resume;
#[cfg(test)]
mod server;
#[cfg(test)]
mod setup;
