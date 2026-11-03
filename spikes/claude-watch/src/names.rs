use std::io;
use std::path::{Path, PathBuf};

/// A file in the sessions folder that passed the name check.
///
/// The folder also holds `<pid>.<hash>.key` files with a peer token. The only code that reads a file from the
/// sessions folder is `StatusFile::read`, and a `StatusFile` exists only for names that pass `status_file_pid`, so a
/// `.key` file is never opened. Directory listing reads names only, not file contents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusFile {
    path: PathBuf,
    pid: u32,
}

impl StatusFile {
    pub fn from_name(dir: &Path, name: &str) -> Option<Self> {
        let pid = status_file_pid(name)?;
        Some(Self {
            path: dir.join(name),
            pid,
        })
    }

    pub fn pid(&self) -> u32 {
        self.pid
    }

    pub fn read(&self) -> io::Result<Vec<u8>> {
        // Checked again at the open itself, so a later change to how the path is built cannot reach a `.key` file.
        let name = self.path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if status_file_pid(name).is_none() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "not a status file name",
            ));
        }
        #[cfg(test)]
        opened::record(name);
        // std opens with FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE, so this read never blocks Claude
        // Code's own write or its delete on exit.
        std::fs::read(&self.path)
    }
}

/// Accepts `<digits>.json` and nothing else.
pub fn status_file_pid(name: &str) -> Option<u32> {
    let stem = name.strip_suffix(".json")?;
    if stem.is_empty() || !stem.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    stem.parse().ok()
}

#[cfg(test)]
pub mod opened {
    use std::cell::RefCell;

    thread_local! {
        static OPENED: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    }

    pub fn record(name: &str) {
        OPENED.with(|o| o.borrow_mut().push(name.to_string()));
    }

    pub fn take() -> Vec<String> {
        OPENED.with(|o| std::mem::take(&mut *o.borrow_mut()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_pid_json() {
        assert_eq!(status_file_pid("20636.json"), Some(20636));
        assert_eq!(status_file_pid("3032.json"), Some(3032));
        for name in [
            "20636.685bf0d758162e180716a9b47a33761d744d0f5da53275e73110abe1a22f559b.key",
            "20636.json.key",
            "20636.key",
            ".fleetview-heartbeat",
            ".json",
            "abc.json",
            "20636.JSON",
            "20636.json.tmp",
            "20 636.json",
            "99999999999.json",
        ] {
            assert_eq!(status_file_pid(name), None, "{name}");
        }
    }

    #[test]
    fn key_file_cannot_become_a_status_file() {
        let dir = Path::new(r"C:\x");
        assert!(StatusFile::from_name(dir, "20636.abc.key").is_none());
        assert!(StatusFile::from_name(dir, "20636.json").is_some());
    }
}
