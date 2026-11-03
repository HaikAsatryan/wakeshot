//! Wakeshot's Windows layer. Every Windows call lives here, so the core stays free of OS code.
//!
//! The traits here are the seams where an implementation is swapped (architecture section 2, "Boundary traits"):
//! real, dry run, or a fake from the `testing` module (feature `test-support`). Each module's doc names the card that
//! owns the file in its `Owner:` line.
//!
//! Owner: contracts card (this module list).

pub mod app;
pub mod child;
pub mod clock;
pub mod error;
pub mod hooks;
pub mod install;
pub mod io_queue;
pub mod localtime;
pub mod lockowner;
pub mod log;
pub mod notify;
pub mod power;
pub mod process;
pub mod recovery;
pub mod session;
pub mod shell;
pub mod sink;
pub mod sound;
pub mod store;
pub mod tray;
pub mod update;
pub mod watch;

pub use error::WinError;

#[cfg(any(test, feature = "test-support"))]
pub mod testing;
