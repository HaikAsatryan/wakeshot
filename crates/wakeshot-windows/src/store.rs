//! The data folder, atomic file writes, named mutexes, and the Run and RunOnce values (architecture sections 4 and 6).
//!
//! Owner: power track (`p3-store`). The signatures are contract: the sources and ship tracks use them. Only the path
//! code is filled; the rest waits for `p3-store`.

use std::path::{Path, PathBuf};

use crate::WinError;

/// One file in the data folder, by its fixed name (architecture section 6).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DataFile {
    Config,
    State,
    Journal,
    JournalCorrupt,
    JournalCorruptSeen,
    RestoreRequest,
    KeepAwake,
}

/// The data folder: `%LocalAppData%\Wakeshot`, its `dry-run` subfolder for `--dry-run`, or any folder for tests.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DataDir {
    root: PathBuf,
}

impl DataDir {
    /// `%LocalAppData%\Wakeshot`; in debug builds `WAKESHOT_DATA_DIR` wins, so tests never touch the real folder.
    pub fn real() -> Result<Self, WinError> {
        unimplemented!("p3-store")
    }

    /// `%LocalAppData%\Wakeshot\dry-run`.
    pub fn dry_run() -> Result<Self, WinError> {
        unimplemented!("p3-store")
    }

    pub fn at(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn file(&self, file: DataFile) -> PathBuf {
        match file {
            DataFile::Config => self.root.join("config.json"),
            DataFile::State => self.root.join("state.json"),
            DataFile::Journal => self.root.join("journal.json"),
            DataFile::JournalCorrupt => self.root.join("journal.json.corrupt"),
            DataFile::JournalCorruptSeen => self.root.join("journal.json.corrupt.seen"),
            DataFile::RestoreRequest => self.root.join("restore-request.json"),
            DataFile::KeepAwake => self.holds().join("keep-awake.json"),
        }
    }

    pub fn holds(&self) -> PathBuf {
        self.root.join("holds")
    }

    pub fn logs(&self) -> PathBuf {
        self.root.join("logs")
    }
}

/// Writes `bytes` to a temp file beside `path` and moves it over `path`, write-through. The temp name is
/// `<name>.<pid>.<n>.tmp`, never ending in `.json`, so the watch thread never reads it as a hold.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), WinError> {
    let _ = (path, bytes);
    unimplemented!("p3-store")
}

/// The whole file, or `None` when it does not exist.
pub fn read_whole(path: &Path) -> Result<Option<Vec<u8>>, WinError> {
    let _ = path;
    unimplemented!("p3-store")
}

/// `true`: the file existed and is gone now.
pub fn remove(path: &Path) -> Result<bool, WinError> {
    let _ = path;
    unimplemented!("p3-store")
}

/// The three mutex names (architecture section 4, "Processes"). A dry run uses its own names, so it runs beside the
/// real tray; tests use their own, so they never block a real tray.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Names {
    pub tray: String,
    pub settings: String,
    pub journal: String,
}

impl Names {
    pub fn real() -> Self {
        Self {
            tray: r"Local\Wakeshot.Tray".to_owned(),
            settings: r"Global\Wakeshot.Settings".to_owned(),
            journal: r"Local\Wakeshot.Journal".to_owned(),
        }
    }

    pub fn dry_run() -> Self {
        Self {
            tray: r"Local\Wakeshot.Tray.DryRun".to_owned(),
            settings: r"Global\Wakeshot.Settings.DryRun".to_owned(),
            journal: r"Local\Wakeshot.Journal.DryRun".to_owned(),
        }
    }

    /// Names no real or dry-run tray uses, with `tag` in them.
    pub fn test(tag: &str) -> Self {
        let _ = tag;
        unimplemented!("p3-store")
    }
}

/// A named mutex, opened or created.
#[derive(Debug)]
pub struct NamedMutex {
    _private: (),
}

impl NamedMutex {
    pub fn open(name: &str) -> Result<Self, WinError> {
        let _ = name;
        unimplemented!("p3-store")
    }

    /// `None`: not taken within `timeout_ms`.
    pub fn take(&self, timeout_ms: u32) -> Result<Option<HeldMutex>, WinError> {
        let _ = timeout_ms;
        unimplemented!("p3-store")
    }
}

/// A taken mutex. It owns its handle and releases on drop.
#[derive(Debug)]
pub struct HeldMutex {
    _private: (),
}

impl HeldMutex {
    /// The previous owner ended while holding it (`WAIT_ABANDONED`); it counts as taken (architecture section 4).
    pub fn abandoned(&self) -> bool {
        unimplemented!("p3-store")
    }
}

/// `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` or `RunOnce`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StartKey {
    Run,
    RunOnce,
}

/// The Run and RunOnce values under one base key: the real one, or a test key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AutoStart {
    base: String,
}

impl AutoStart {
    pub fn real() -> Self {
        Self::at(r"Software\Microsoft\Windows\CurrentVersion")
    }

    /// `base` is a key under `HKCU`; `Run` and `RunOnce` are its subkeys.
    pub fn at(base: &str) -> Self {
        Self {
            base: base.to_owned(),
        }
    }

    pub fn set(&self, key: StartKey, name: &str, command: &str) -> Result<(), WinError> {
        let _ = (&self.base, key, name, command);
        unimplemented!("p3-store")
    }

    pub fn get(&self, key: StartKey, name: &str) -> Result<Option<String>, WinError> {
        let _ = (key, name);
        unimplemented!("p3-store")
    }

    /// `true`: the value existed and is gone now.
    pub fn remove(&self, key: StartKey, name: &str) -> Result<bool, WinError> {
        let _ = (key, name);
        unimplemented!("p3-store")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_files_have_their_fixed_names() {
        let dir = DataDir::at(PathBuf::from(r"C:\data"));
        assert_eq!(
            dir.file(DataFile::Journal),
            Path::new(r"C:\data\journal.json")
        );
        assert_eq!(
            dir.file(DataFile::KeepAwake),
            Path::new(r"C:\data\holds\keep-awake.json")
        );
        assert_eq!(dir.logs(), Path::new(r"C:\data\logs"));
        assert_ne!(Names::real().journal, Names::dry_run().journal);
    }
}
