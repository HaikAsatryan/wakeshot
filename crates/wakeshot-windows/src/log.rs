//! The text log in `logs\wakeshot.log` (architecture section 8).
//!
//! Owner: power track (`p3-log`). The trait, the types and `MemoryLog` are contract: the sources, tray and ship tracks
//! log through `Log` and test with `MemoryLog` until `p3-log` is done.

use std::path::Path;
#[cfg(any(test, feature = "test-support"))]
use std::sync::Mutex;

use crate::WinError;

/// Which process wrote a line.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Proc {
    Tray,
    Guard,
    Restore,
    Cli,
    Hook,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Level {
    Info,
    Warn,
    Error,
}

/// One log line. `area` names the part that writes it (`core`, `power`, `watch`, ...); `text` is plain English and
/// never holds prompts or file content (architecture section 8). Callers may rely on `line` never failing: a write
/// that fails is dropped.
pub trait Log: Send + Sync {
    fn line(&self, level: Level, area: &'static str, text: &str);
}

/// Appends to the log file, shared by every process.
#[derive(Debug)]
pub struct LogWriter {
    _private: (),
}

impl LogWriter {
    pub fn open(path: &Path, proc: Proc) -> Result<Self, WinError> {
        let _ = (path, proc);
        unimplemented!("p3-log")
    }

    /// Moves `wakeshot.log` to `wakeshot.1.log` and that one to `wakeshot.2.log`.
    pub fn roll(path: &Path) -> Result<(), WinError> {
        let _ = path;
        unimplemented!("p3-log")
    }
}

impl Log for LogWriter {
    fn line(&self, level: Level, area: &'static str, text: &str) {
        let _ = (level, area, text);
        unimplemented!("p3-log")
    }
}

/// Keeps every line in memory, for tests.
#[cfg(any(test, feature = "test-support"))]
#[derive(Debug, Default)]
pub struct MemoryLog {
    pub lines: Mutex<Vec<(Level, &'static str, String)>>,
}

#[cfg(any(test, feature = "test-support"))]
impl Log for MemoryLog {
    fn line(&self, level: Level, area: &'static str, text: &str) {
        self.lines
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push((level, area, text.to_owned()));
    }
}
