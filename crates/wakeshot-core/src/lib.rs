//! Wakeshot's core: it takes `(Now, Event)` and returns commands. It has no OS code, so it builds and tests on any OS.
//!
//! The public types here are the contracts between the parallel tracks (`docs/plan.md`, Phases). Each type's doc
//! names the track that owns it and what the other side may rely on. A change to one goes through a contract card and
//! the orchestrator, never as a side edit in a track. Each module's first doc line names the card that owns the file.

#![forbid(unsafe_code)]

pub mod command;
pub mod core;
pub mod event;
pub mod files;
pub mod holds;
pub mod ownership;
pub mod policy;
pub mod present;
pub mod process;
pub mod settings;
pub mod sources;
mod state;
pub mod text;
pub mod time;
pub mod view;

#[cfg(test)]
mod scenarios;
#[cfg(any(test, feature = "test-support"))]
pub mod sim;
#[cfg(any(test, feature = "test-support"))]
pub mod testing;
