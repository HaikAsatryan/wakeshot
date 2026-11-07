//! Wakeshot's core: it takes `(Now, Event)` and returns commands. It has no OS code, so it builds and tests on any OS.
//!
//! Its public types are what the Windows layer and the exes use. Each type's doc says what the other side may rely on.

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
