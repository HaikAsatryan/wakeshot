//! Process identity as plain data. The Windows layer checks it; the core only reads the answer.

use std::fmt;

use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize, Serializer};

/// A process creation time as a Windows FILETIME (100 ns units since 1601-01-01 UTC).
///
/// Kept raw so a pid plus this value compares exactly with what Windows and Claude's status files report. Files store
/// it as a decimal string (`"procStart"`), the form Claude uses; a JSON number is rejected so a script that writes one
/// finds out at once.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FileTime(pub u64);

impl Serialize for FileTime {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for FileTime {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let text = String::deserialize(d)?;
        // `u64::from_str` takes a leading '+', which no FILETIME writer emits.
        if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
            return Err(de::Error::custom("procStart must be a decimal string"));
        }
        text.parse().map(FileTime).map_err(de::Error::custom)
    }
}

impl fmt::Display for FileTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// A pid with its creation time, so a reused pid never passes for the process it once was.
///
/// `wakeshot_windows::process` checks it. The core, the tray and the CLI may rely on pid plus start naming at most one
/// process ever.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProcessRef {
    pub pid: u32,
    pub start: FileTime,
}

/// The answer of one liveness check (architecture section 5, "Liveness").
///
/// `Unknown` means the check itself failed: the core keeps the last state and never deletes a file or starts a
/// restore on it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Alive {
    Yes,
    Gone,
    Unknown,
}

/// What runs under a pid now, for a hold file that gives only a pid (no start time).
///
/// `Liveness::start_of_pid` in `wakeshot_windows::process` answers it. `Unknown`: the lookup itself failed, so
/// nothing may be deleted on it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lookup {
    Running(FileTime),
    NotRunning,
    Unknown,
}
