//! Hold files, `config.json` and the restore request: the data folder as events (architecture section 6).
//!
//! Driven by calls, with no thread or folder watch of its own: `p4-watch-thread` calls it on a data folder notice,
//! when a retry is due and on the tick. Every file goes through `wakeshot_core::files::read` (the version rule, first
//! JSON value only, a byte order mark skipped). Reads only: deleting a hold whose process is gone is the core's
//! `DeleteHold`, done by the IO thread, and the IO thread deletes the restore request after `state.json` (track
//! note 15).
//!
//! Owner: `p4-watch-data`; fixes by `p4-live-claude`.

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use wakeshot_core::event::Event;
use wakeshot_core::files::{self, Config, Read, RestoreRequest};
use wakeshot_core::holds::file::{self as hold_file, Parsed};
use wakeshot_core::holds::{HoldFacts, HoldId};
use wakeshot_core::process::{Alive, FileTime, Lookup, ProcessRef};
use wakeshot_core::view::MenuCommand;

use crate::log::{Level, Log};
use crate::process::Liveness;
use crate::store::{DataDir, DataFile};
use crate::watch::read::{names, read_whole_written};

/// How long after a bad hold file read the caller runs [`DataFolder::holds`] again: scripts may not write their file
/// in one step (track note 10).
pub const RETRY_MS: u64 = 200;

const AREA: &str = "watch";
const CONFIG_NAME: &str = "config.json";
const RESTORE_NAME: &str = "restore-request.json";

/// One [`DataFolder::holds`] read.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HoldsRead {
    /// The full list for `Event::Holds`, by file name. Every file that parsed is in it, whatever its liveness: `Gone`
    /// lets the core delete the file, `Unknown` keeps it.
    pub holds: Vec<HoldFacts>,
    /// The processes of the holds that are alive, so the thread can wait on their exit.
    pub live: Vec<ProcessRef>,
    /// Files that did not read or parse, the first time for this version of the file: run `holds` again
    /// [`RETRY_MS`] later. They stay out of `holds` until they read well.
    pub retry: Vec<HoldId>,
}

/// Why a data file was left out, for one log line. Fixed texts and numbers only, never file content.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Why {
    Unreadable(Option<i32>),
    Bad(&'static str),
    Newer(u32),
}

impl Why {
    fn unreadable(e: &io::Error) -> Self {
        Why::Unreadable(e.raw_os_error())
    }

    fn text(&self) -> String {
        match self {
            Why::Unreadable(code) => format!("it could not be read ({})", code_text(*code)),
            Why::Bad(reason) => (*reason).to_owned(),
            Why::Newer(version) => format!("version {version} is newer than this Wakeshot reads"),
        }
    }
}

/// One version of a file: its last write time and length, or `None` when it could not be read. A new version of a
/// bad file is retried and logged again.
type Version = Option<(FileTime, u64)>;

/// A hold file that is left out, so one bad version is retried once and logged once.
#[derive(Debug)]
struct Trouble {
    version: Version,
    retried: bool,
    logged: bool,
}

impl Trouble {
    fn new(version: Version) -> Self {
        Self {
            version,
            retried: false,
            logged: false,
        }
    }
}

/// The data folder's files as events, with what it last saw of them.
pub struct DataFolder {
    dir: DataDir,
    log: Arc<dyn Log>,
    config: Config,
    config_logged: Option<(Why, Version)>,
    restore: Restore,
    troubles: HashMap<String, Trouble>,
    listed: Vec<PathBuf>,
    listing_logged: bool,
}

/// The restore request's state: `Armed` until it is seen, then the write time it was seen with (`None`: it could not
/// be read).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Restore {
    Armed,
    Fired(Option<FileTime>),
}

impl DataFolder {
    /// `config` is the value the core already has (the one `Started` carried, or `Config::default()`), so the first
    /// [`config`](Self::config) call posts only a difference.
    pub fn new(dir: DataDir, config: Config, log: Arc<dyn Log>) -> Self {
        Self {
            dir,
            log,
            config,
            config_logged: None,
            restore: Restore::Armed,
            troubles: HashMap::new(),
            listed: Vec::new(),
            listing_logged: false,
        }
    }

    /// Reads every `holds\*.json` (temp names never end in `.json`) and checks each good file's process once.
    ///
    /// A file that does not read or parse is left out and named in `retry` the first time; still bad on the next
    /// call, or newer than this version, it is named in one log line, once per file and version. If the folder cannot
    /// be listed, the last listing is read again (and the error logged once), so a listing error never reads as
    /// every hold gone.
    pub fn holds(&mut self, liveness: &dyn Liveness) -> HoldsRead {
        let listing = match names(&self.dir.holds(), 0, is_hold_name) {
            Ok(listing) => {
                self.listing_logged = false;
                self.listed.clone_from(&listing);
                listing
            }
            Err(e) => {
                if !std::mem::replace(&mut self.listing_logged, true) {
                    let code = code_text(e.raw_os_error());
                    self.warn(&format!(
                        "the holds folder could not be listed ({code}), the last list is read again"
                    ));
                }
                self.listed.clone()
            }
        };
        self.troubles
            .retain(|name, _| listing.iter().any(|path| file_name(path) == name));

        let mut read = HoldsRead::default();
        for path in &listing {
            let name = file_name(path);
            match read_whole_written(path, is_hold_name) {
                // Deleted since the listing: its delete notice runs this again.
                Err(e) if e.kind() == io::ErrorKind::NotFound => {}
                Err(e) => self.bad(name, None, Why::unreadable(&e), &mut read.retry),
                Ok((bytes, written)) => {
                    let version = Some((written, bytes.len() as u64));
                    match hold_file::parse(name, &bytes) {
                        Parsed::Ok(hold) => {
                            self.troubles.remove(name);
                            let lookup = liveness.start_of_pid(hold.file.pid);
                            let alive = hold_file::hold_alive(&hold.file, lookup, written);
                            if let (Alive::Yes, Lookup::Running(start)) = (alive, lookup) {
                                read.live.push(ProcessRef {
                                    pid: hold.file.pid,
                                    start,
                                });
                            }
                            let id = HoldId(name.to_owned());
                            read.holds.push(hold_file::facts(id, &hold, alive));
                        }
                        Parsed::Newer(v) => self.newer(name, version, v),
                        Parsed::Bad(reason) => {
                            self.bad(name, version, Why::Bad(reason), &mut read.retry);
                        }
                    }
                }
            }
        }
        read
    }

    fn bad(&mut self, name: &str, version: Version, why: Why, retry: &mut Vec<HoldId>) {
        let trouble = self.trouble(name, version);
        if !std::mem::replace(&mut trouble.retried, true) {
            retry.push(HoldId(name.to_owned()));
            return;
        }
        if !std::mem::replace(&mut trouble.logged, true) {
            self.warn(&format!("hold file {name} left out: {}", why.text()));
        }
    }

    /// A newer file is not retried: a second read gives the same version.
    fn newer(&mut self, name: &str, version: Version, found: u32) {
        let trouble = self.trouble(name, version);
        trouble.retried = true;
        if !std::mem::replace(&mut trouble.logged, true) {
            let why = Why::Newer(found).text();
            self.warn(&format!("hold file {name} left out: {why}"));
        }
    }

    fn trouble(&mut self, name: &str, version: Version) -> &mut Trouble {
        let trouble = self
            .troubles
            .entry(name.to_owned())
            .or_insert_with(|| Trouble::new(version));
        if trouble.version != version {
            *trouble = Trouble::new(version);
        }
        trouble
    }

    /// `ConfigChanged` when `config.json` gives a value other than the last one posted; a missing file gives
    /// `Config::default()`. A newer or bad file gives no event (the tray keeps what it has) and one log line per
    /// version.
    pub fn config(&mut self) -> Option<Event> {
        let path = self.dir.file(DataFile::Config);
        let read = match read_whole_written(&path, |n| n.eq_ignore_ascii_case(CONFIG_NAME)) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Config::default()),
            Err(e) => Err((Why::unreadable(&e), None)),
            Ok((bytes, written)) => {
                let version = Some((written, bytes.len() as u64));
                match files::read::<Config>(&bytes) {
                    Read::Value(config) => Ok(config),
                    Read::Newer(v) => Err((Why::Newer(v), version)),
                    Read::Bad => Err((
                        Why::Bad("not a config file: JSON with version and automatic"),
                        version,
                    )),
                }
            }
        };
        match read {
            Ok(config) => {
                self.config_logged = None;
                if config == self.config {
                    return None;
                }
                self.config.clone_from(&config);
                Some(Event::ConfigChanged(config))
            }
            Err(logged) => {
                if self.config_logged.as_ref() != Some(&logged) {
                    let why = logged.0.text();
                    self.warn(&format!("{CONFIG_NAME} ignored, settings kept: {why}"));
                    self.config_logged = Some(logged);
                }
                None
            }
        }
    }

    /// `Menu(Restore)` once per appearance of `restore-request.json`: nothing while the same file stays, armed again
    /// once it is gone. A request written again before the old one was seen gone (a new write time) counts again, so
    /// a delete and a new request that fall into one folder notice are not lost. A newer, bad or unreadable file
    /// counts too: the request has no fields to misread, and only the CLI writes it.
    pub fn restore_request(&mut self) -> Option<Event> {
        let path = self.dir.file(DataFile::RestoreRequest);
        let (written, why) =
            match read_whole_written(&path, |n| n.eq_ignore_ascii_case(RESTORE_NAME)) {
                Err(e) if e.kind() == io::ErrorKind::NotFound => {
                    self.restore = Restore::Armed;
                    return None;
                }
                Err(e) => (None, Some(Why::unreadable(&e))),
                Ok((bytes, written)) => match files::read::<RestoreRequest>(&bytes) {
                    Read::Value(_) => (Some(written), None),
                    Read::Newer(v) => (Some(written), Some(Why::Newer(v))),
                    Read::Bad => (Some(written), Some(Why::Bad("not JSON with a version"))),
                },
            };
        let again = match self.restore {
            Restore::Armed => true,
            Restore::Fired(seen) => written.is_some() && seen.is_some() && written != seen,
        };
        if !again {
            return None;
        }
        self.restore = Restore::Fired(written);
        if let Some(why) = why {
            self.warn(&format!("{RESTORE_NAME} counts anyway: {}", why.text()));
        }
        Some(Event::Menu(MenuCommand::Restore))
    }

    fn warn(&self, text: &str) {
        self.log.line(Level::Warn, AREA, text);
    }
}

fn code_text(code: Option<i32>) -> String {
    code.map_or_else(
        || "no error code".to_owned(),
        |code| format!("error {code}"),
    )
}

fn is_hold_name(name: &str) -> bool {
    hold_file::kind_of(name).is_some()
}

/// `names` passes only names `is_hold_name` took, which are UTF-8.
fn file_name(path: &Path) -> &str {
    path.file_name().and_then(|n| n.to_str()).unwrap_or("")
}

#[cfg(test)]
mod tests {
    use std::fs::{self, File, OpenOptions};
    use std::os::windows::fs::OpenOptionsExt;
    use std::sync::Mutex;
    use std::time::{Duration, SystemTime};

    use wakeshot_core::holds::HoldKind;

    use super::*;
    use crate::log::MemoryLog;
    use crate::process::FakeLiveness;
    use crate::watch::read::TempDir;

    /// 2026-11-03 12:00:00 UTC as a FILETIME: 1793707200 s after 1970, plus the 11644473600 s from 1601 to 1970.
    const NOON: FileTime = FileTime((1_793_707_200 + 11_644_473_600) * 10_000_000);
    /// 2026-11-03 18:00:00 UTC in wall ms.
    const SIX_PM_MS: i64 = 1_793_728_800_000;

    struct Folder {
        temp: TempDir,
        dir: DataDir,
        log: Arc<MemoryLog>,
        data: DataFolder,
    }

    impl Folder {
        fn new(tag: &str) -> Self {
            Self::with_config(tag, Config::default())
        }

        fn with_config(tag: &str, config: Config) -> Self {
            let temp = TempDir::new(&format!("data-{tag}"));
            let dir = DataDir::at(temp.path().to_path_buf());
            let log = Arc::new(MemoryLog::default());
            let data = DataFolder::new(dir.clone(), config, log.clone());
            Self {
                temp,
                dir,
                log,
                data,
            }
        }

        fn hold(&self, name: &str, json: &str) -> PathBuf {
            self.hold_at(name, json, NOON)
        }

        fn hold_at(&self, name: &str, json: &str, written: FileTime) -> PathBuf {
            let path = self.temp.write(&format!("holds/{name}"), json.as_bytes());
            set_written(&path, written);
            path
        }

        fn file(&self, file: DataFile, bytes: &[u8], written: FileTime) -> PathBuf {
            let path = self.dir.file(file);
            fs::write(&path, bytes).unwrap();
            set_written(&path, written);
            path
        }

        fn lines(&self) -> Vec<String> {
            self.log
                .lines
                .lock()
                .unwrap()
                .iter()
                .map(|(level, area, text)| {
                    assert_eq!((*level, *area), (Level::Warn, AREA));
                    text.clone()
                })
                .collect()
        }
    }

    fn set_written(path: &Path, written: FileTime) {
        let since_1970 = (written.0 - 116_444_736_000_000_000) * 100;
        let when = SystemTime::UNIX_EPOCH + Duration::from_nanos(since_1970);
        let file = OpenOptions::new().write(true).open(path).unwrap();
        file.set_modified(when).unwrap();
    }

    fn later(by_s: u64) -> FileTime {
        FileTime(NOON.0 + by_s * 10_000_000)
    }

    fn liveness(starts: &[(u32, Lookup)]) -> FakeLiveness {
        FakeLiveness {
            starts: Mutex::new(starts.to_vec()),
            ..FakeLiveness::default()
        }
    }

    fn ids(read: &HoldsRead) -> Vec<&str> {
        read.holds.iter().map(|h| h.id.0.as_str()).collect()
    }

    fn alive_of(read: &HoldsRead, id: &str) -> Alive {
        read.holds.iter().find(|h| h.id.0 == id).unwrap().alive
    }

    #[test]
    fn each_hold_kind_is_read() {
        let mut f = Folder::new("kinds");
        let start = FileTime(134_381_211_391_959_336);
        let started = format!("\"procStart\":\"{}\"", start.0);
        f.hold(
            "keep-awake.json",
            &format!("{{\"version\":1,\"pid\":10,{started},\"screen\":true,\"reason\":\"menu\"}}"),
        );
        f.hold(
            "run-4242.json",
            &format!("{{\"version\":1,\"pid\":4242,{started},\"reason\":\"cargo\"}}"),
        );
        f.hold(
            "BUILD.JSON",
            &format!("{{\"version\":1,\"pid\":77,{started},\"screen\":true}}"),
        );
        let live = liveness(&[
            (10, Lookup::Running(start)),
            (4242, Lookup::Running(start)),
            (77, Lookup::Running(start)),
        ]);

        let read = f.data.holds(&live);
        assert_eq!(
            ids(&read),
            ["BUILD.JSON", "keep-awake.json", "run-4242.json"]
        );
        let kinds: Vec<_> = read.holds.iter().map(|h| (h.kind, h.screen)).collect();
        assert_eq!(
            kinds,
            [
                (HoldKind::Script, false),
                (HoldKind::KeepAwake, true),
                (HoldKind::Run, false)
            ]
        );
        assert_eq!(read.holds[1].reason.as_deref(), Some("menu"));
        assert_eq!(read.holds[2].reason.as_deref(), Some("cargo"));
        assert!(read.holds.iter().all(|h| h.alive == Alive::Yes));
        let pids: Vec<_> = read.live.iter().map(|p| (p.pid, p.start)).collect();
        assert_eq!(pids, [(77, start), (10, start), (4242, start)]);
        assert!(read.retry.is_empty());
        assert!(f.lines().is_empty());
    }

    #[test]
    fn proc_start_must_match_the_running_process() {
        let mut f = Folder::new("start-match");
        f.hold(
            "a.json",
            "{\"version\":1,\"pid\":5,\"procStart\":\"134381211391959336\"}",
        );
        f.hold(
            "b.json",
            "{\"version\":1,\"pid\":6,\"procStart\":\"134381211391959336\"}",
        );
        let live = liveness(&[
            (5, Lookup::Running(FileTime(134_381_211_391_959_336))),
            (6, Lookup::Running(FileTime(134_381_211_391_959_337))),
        ]);
        let read = f.data.holds(&live);
        assert_eq!(alive_of(&read, "a.json"), Alive::Yes);
        assert_eq!(alive_of(&read, "b.json"), Alive::Gone, "a reused pid");
        assert_eq!(read.live.len(), 1);
    }

    #[test]
    fn missing_proc_start_counts_a_process_started_by_the_write_time() {
        let mut f = Folder::new("start-missing");
        for name in ["before.json", "at.json", "after.json"] {
            f.hold(name, &format!("{{\"version\":1,\"pid\":{}}}", name.len()));
        }
        // The pids are the name lengths: before 11, at 7, after 10.
        let live = liveness(&[
            (11, Lookup::Running(FileTime(NOON.0 - 1))),
            (7, Lookup::Running(NOON)),
            (10, Lookup::Running(FileTime(NOON.0 + 1))),
        ]);
        let read = f.data.holds(&live);
        assert_eq!(alive_of(&read, "before.json"), Alive::Yes);
        assert_eq!(alive_of(&read, "at.json"), Alive::Yes);
        assert_eq!(alive_of(&read, "after.json"), Alive::Gone);
        let mut pids: Vec<_> = read.live.iter().map(|p| p.pid).collect();
        pids.sort_unstable();
        assert_eq!(pids, [7, 11]);
    }

    #[test]
    fn a_dead_pid_is_gone_and_an_unknown_lookup_stays_in_the_list() {
        let mut f = Folder::new("dead");
        let dead = f.hold("dead.json", "{\"version\":1,\"pid\":8}");
        let unknown = f.hold("unknown.json", "{\"version\":1,\"pid\":9}");
        let live = liveness(&[(9, Lookup::Unknown)]);
        let read = f.data.holds(&live);
        assert_eq!(ids(&read), ["dead.json", "unknown.json"]);
        assert_eq!(alive_of(&read, "dead.json"), Alive::Gone);
        assert_eq!(alive_of(&read, "unknown.json"), Alive::Unknown);
        assert!(read.live.is_empty());
        // Deleting is the core's decision, done by the IO thread.
        assert!(dead.is_file() && unknown.is_file());
    }

    #[test]
    fn expires_past_and_future_are_carried_for_the_core() {
        let mut f = Folder::new("expires");
        f.hold(
            "past.json",
            "{\"version\":1,\"pid\":3,\"expires\":\"2026-11-03T18:00:00Z\"}",
        );
        f.hold(
            "future.json",
            "{\"version\":1,\"pid\":3,\"expires\":\"9999-12-31T23:59:59Z\"}",
        );
        f.hold("none.json", "{\"version\":1,\"pid\":3}");
        let read = f.data.holds(&liveness(&[(3, Lookup::Running(NOON))]));
        let expires: Vec<_> = read
            .holds
            .iter()
            .map(|h| (h.id.0.as_str(), h.expires_wall_ms))
            .collect();
        assert_eq!(
            expires,
            [
                ("future.json", Some(253_402_300_799_000)),
                ("none.json", None),
                ("past.json", Some(SIX_PM_MS)),
            ]
        );
        assert!(read.holds.iter().all(|h| h.alive == Alive::Yes));
    }

    #[test]
    fn a_bad_file_is_retried_once_then_logged_once() {
        let mut f = Folder::new("bad");
        let live = liveness(&[(40, Lookup::Running(NOON))]);
        let path = f.hold("job.json", "{\"version\":1,\"pid\":40,\"reas");

        let first = f.data.holds(&live);
        assert!(first.holds.is_empty());
        assert_eq!(first.retry, [HoldId("job.json".into())]);
        assert!(f.lines().is_empty(), "no line before the retry");

        let second = f.data.holds(&live);
        assert!(second.holds.is_empty() && second.retry.is_empty());
        assert_eq!(
            f.lines(),
            ["hold file job.json left out: not a hold file: JSON with version and pid"]
        );

        let third = f.data.holds(&live);
        assert!(third.holds.is_empty() && third.retry.is_empty());
        assert_eq!(f.lines().len(), 1, "one line per file and version");

        fs::write(&path, "{\"version\":1,\"pid\":40}").unwrap();
        set_written(&path, later(1));
        assert_eq!(ids(&f.data.holds(&live)), ["job.json"]);

        // A new bad version is retried and logged again.
        fs::write(&path, "{\"version\":1,").unwrap();
        set_written(&path, later(2));
        assert_eq!(f.data.holds(&live).retry.len(), 1);
        assert!(f.data.holds(&live).retry.is_empty());
        assert_eq!(f.lines().len(), 2);
    }

    #[test]
    fn a_bad_file_fixed_before_its_retry_counts_with_no_line() {
        let mut f = Folder::new("fixed");
        let live = liveness(&[(40, Lookup::Running(NOON))]);
        let path = f.hold("job.json", "{\"vers");
        assert_eq!(f.data.holds(&live).retry.len(), 1);
        fs::write(&path, "{\"version\":1,\"pid\":40}").unwrap();
        set_written(&path, later(1));
        let read = f.data.holds(&live);
        assert_eq!(ids(&read), ["job.json"]);
        assert!(read.retry.is_empty());
        assert!(f.lines().is_empty());
    }

    #[test]
    fn a_bad_expires_is_left_out_like_a_bad_file() {
        let mut f = Folder::new("bad-expires");
        let live = liveness(&[(40, Lookup::Running(NOON))]);
        f.hold(
            "job.json",
            "{\"version\":1,\"pid\":40,\"expires\":\"2026-11-03 18:00\"}",
        );
        assert_eq!(f.data.holds(&live).retry.len(), 1);
        let read = f.data.holds(&live);
        assert!(read.holds.is_empty());
        assert_eq!(
            f.lines(),
            ["hold file job.json left out: expires is not an RFC 3339 time with Z or an offset"]
        );
    }

    #[test]
    fn an_unreadable_file_is_retried_then_logged() {
        let mut f = Folder::new("unreadable");
        let live = liveness(&[(40, Lookup::Running(NOON))]);
        let path = f.hold("job.json", "{\"version\":1,\"pid\":40}");
        let held = OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(&path)
            .unwrap();
        assert_eq!(f.data.holds(&live).retry.len(), 1);
        assert!(f.data.holds(&live).holds.is_empty());
        assert_eq!(
            f.lines(),
            ["hold file job.json left out: it could not be read (error 32)"]
        );
        drop(held);
        assert_eq!(ids(&f.data.holds(&live)), ["job.json"]);
    }

    #[test]
    fn a_newer_file_is_left_out_and_logged_once_without_a_retry() {
        let mut f = Folder::new("newer");
        let live = liveness(&[(40, Lookup::Running(NOON))]);
        f.hold("job.json", "{\"version\":2,\"pid\":40,\"shape\":[1]}");
        let read = f.data.holds(&live);
        assert!(read.holds.is_empty() && read.retry.is_empty());
        f.data.holds(&live);
        assert_eq!(
            f.lines(),
            ["hold file job.json left out: version 2 is newer than this Wakeshot reads"]
        );
    }

    #[test]
    fn temp_names_and_folders_are_never_opened() {
        let mut f = Folder::new("temp");
        let live = liveness(&[(40, Lookup::Running(NOON))]);
        let temps = [
            "keep-awake.json.1234.5.tmp",
            "run-4242.json.tmp",
            "job.json~",
            "notes.txt",
        ];
        // Held with no sharing: an open of any of them would fail and show up as a retry.
        let held: Vec<File> = temps
            .iter()
            .map(|name| {
                let path = f.hold(name, "{\"version\":1,\"pid\":40}");
                OpenOptions::new()
                    .read(true)
                    .share_mode(0)
                    .open(path)
                    .unwrap()
            })
            .collect();
        fs::create_dir_all(f.dir.holds().join("folder.json")).unwrap();
        f.hold("real.json", "{\"version\":1,\"pid\":40}");
        let read = f.data.holds(&live);
        assert_eq!(ids(&read), ["real.json"]);
        assert!(read.retry.is_empty());
        assert!(f.lines().is_empty());
        drop(held);
    }

    #[test]
    fn a_missing_holds_folder_is_an_empty_list() {
        let mut f = Folder::new("no-holds");
        assert_eq!(f.data.holds(&liveness(&[])), HoldsRead::default());
        assert!(f.lines().is_empty());
    }

    #[test]
    fn a_holds_folder_that_cannot_be_listed_is_logged_once() {
        let mut f = Folder::new("holds-file");
        let live = liveness(&[(40, Lookup::Running(NOON))]);
        f.hold("job.json", "{\"version\":1,\"pid\":40}");
        assert_eq!(ids(&f.data.holds(&live)), ["job.json"]);

        // `holds` becomes a file: the listing fails, and the last listing's file, read again, is gone.
        fs::remove_dir_all(f.dir.holds()).unwrap();
        fs::write(f.dir.holds(), b"not a folder").unwrap();
        assert!(f.data.holds(&live).holds.is_empty());
        assert!(f.data.holds(&live).holds.is_empty());
        let lines = f.lines();
        assert_eq!(lines.len(), 1, "{lines:?}");
        assert!(lines[0].starts_with("the holds folder could not be listed (error "));
        assert!(lines[0].ends_with("), the last list is read again"));

        // Listed again, a later failure is logged again.
        fs::remove_file(f.dir.holds()).unwrap();
        f.hold("job.json", "{\"version\":1,\"pid\":40}");
        assert_eq!(ids(&f.data.holds(&live)), ["job.json"]);
        fs::remove_dir_all(f.dir.holds()).unwrap();
        fs::write(f.dir.holds(), b"not a folder").unwrap();
        f.data.holds(&live);
        assert_eq!(f.lines().len(), 2);
    }

    #[test]
    fn config_missing_changed_and_unchanged() {
        let mut f = Folder::new("config");
        assert_eq!(
            f.data.config(),
            None,
            "missing, and the core has the default"
        );

        f.file(
            DataFile::Config,
            b"{\"version\":1,\"automatic\":false}",
            NOON,
        );
        assert_eq!(
            f.data.config(),
            Some(Event::ConfigChanged(Config::new(false)))
        );
        assert_eq!(f.data.config(), None);
        f.file(
            DataFile::Config,
            b"\xEF\xBB\xBF{\"version\":1,\"automatic\":false}\n",
            later(1),
        );
        assert_eq!(f.data.config(), None, "same value, new write");

        fs::remove_file(f.dir.file(DataFile::Config)).unwrap();
        assert_eq!(
            f.data.config(),
            Some(Event::ConfigChanged(Config::default()))
        );
        assert_eq!(f.data.config(), None);
        assert!(f.lines().is_empty());
    }

    #[test]
    fn config_starts_from_the_value_the_core_has() {
        let mut f = Folder::with_config("config-start", Config::new(false));
        assert_eq!(
            f.data.config(),
            Some(Event::ConfigChanged(Config::default()))
        );
    }

    #[test]
    fn config_newer_or_bad_gives_no_event_and_one_line_per_version() {
        let mut f = Folder::new("config-bad");
        f.file(
            DataFile::Config,
            b"{\"version\":1,\"automatic\":false}",
            NOON,
        );
        assert!(f.data.config().is_some());

        f.file(
            DataFile::Config,
            b"{\"version\":2,\"automatic\":\"on\"}",
            later(1),
        );
        assert_eq!(f.data.config(), None);
        assert_eq!(f.data.config(), None);
        f.file(DataFile::Config, b"{\"automatic\":true}", later(2));
        assert_eq!(f.data.config(), None);
        assert_eq!(
            f.lines(),
            [
                "config.json ignored, settings kept: version 2 is newer than this Wakeshot reads",
                "config.json ignored, settings kept: not a config file: JSON with version and automatic",
            ]
        );

        f.file(
            DataFile::Config,
            b"{\"version\":1,\"automatic\":true}",
            later(3),
        );
        assert_eq!(
            f.data.config(),
            Some(Event::ConfigChanged(Config::new(true)))
        );
    }

    #[test]
    fn restore_request_counts_once_per_appearance() {
        let mut f = Folder::new("restore");
        assert_eq!(f.data.restore_request(), None);
        let path = f.file(DataFile::RestoreRequest, b"{\"version\":1}", NOON);
        assert_eq!(
            f.data.restore_request(),
            Some(Event::Menu(MenuCommand::Restore))
        );
        for _ in 0..3 {
            assert_eq!(
                f.data.restore_request(),
                None,
                "a change notice for the same file"
            );
        }
        assert!(path.is_file(), "the IO thread deletes it, not the watch");
        fs::remove_file(&path).unwrap();
        assert_eq!(f.data.restore_request(), None);
        f.file(DataFile::RestoreRequest, b"{\"version\":1}", later(5));
        assert_eq!(
            f.data.restore_request(),
            Some(Event::Menu(MenuCommand::Restore))
        );
        assert_eq!(f.data.restore_request(), None);
        assert!(f.lines().is_empty());
    }

    #[test]
    fn restore_request_written_again_before_it_was_seen_gone_counts_again() {
        let mut f = Folder::new("restore-again");
        f.file(DataFile::RestoreRequest, b"{\"version\":1}", NOON);
        assert!(f.data.restore_request().is_some());
        f.file(DataFile::RestoreRequest, b"{\"version\":1}", later(1));
        assert_eq!(
            f.data.restore_request(),
            Some(Event::Menu(MenuCommand::Restore))
        );
        assert_eq!(f.data.restore_request(), None);
    }

    #[test]
    fn restore_request_newer_bad_or_unreadable_still_counts() {
        let mut f = Folder::new("restore-odd");
        let path = f.file(DataFile::RestoreRequest, b"{\"version\":3,\"x\":[]}", NOON);
        assert!(f.data.restore_request().is_some());
        fs::remove_file(&path).unwrap();
        assert!(f.data.restore_request().is_none());

        f.file(DataFile::RestoreRequest, b"restore please", later(1));
        assert!(f.data.restore_request().is_some());
        fs::remove_file(&path).unwrap();
        assert!(f.data.restore_request().is_none());

        f.file(DataFile::RestoreRequest, b"{\"version\":1}", later(2));
        let held = OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(&path)
            .unwrap();
        assert!(f.data.restore_request().is_some());
        assert!(f.data.restore_request().is_none(), "still the same request");
        drop(held);
        assert!(
            f.data.restore_request().is_none(),
            "now readable, still the same request"
        );
        assert_eq!(
            f.lines(),
            [
                "restore-request.json counts anyway: version 3 is newer than this Wakeshot reads",
                "restore-request.json counts anyway: not JSON with a version",
                "restore-request.json counts anyway: it could not be read (error 32)",
            ]
        );
    }
}
