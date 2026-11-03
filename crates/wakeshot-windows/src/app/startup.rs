//! Tray startup steps 3 to 7, `Started`, the corrupt journal left by others and the panic hook.
//!
//! Owner: `p6-int-startup`; fixes by the e2e cards (`docs/phases/06-integration.md` note 6).
//!
//! `tray_mode::run` calls these on the loop (main) thread, in this order: `Boot::new`, `take_tray`, `open_log`,
//! `install_panic_hook`, `startup_restore`, `try_settings`, `read_config`, `remove_stale_temps`; after the window and
//! the other threads are up, `started`, then `locked_at_start`. The mutex locks are not `Send` (only the thread that
//! took a mutex can release it), so the loop thread takes them and keeps them until the tray exits.
//!
//! A corrupt journal left by restore mode, the guard or a hook needs no function here: the executor's `startup` finds
//! `journal.json.corrupt`, renames it `.seen` and says so in `Startup::corrupt`, so the tray posts `JournalCorrupt`
//! after `Started` (and after `Session(Locked)`) when that is `true`.

use std::io::ErrorKind;
use std::os::windows::fs::MetadataExt;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use wakeshot_core::event::Event;
use wakeshot_core::files::{self, Config, Read};
use wakeshot_core::ownership::io::PlanIo;
use wakeshot_core::process::{FileTime, Lookup, ProcessRef};
use wakeshot_core::settings::JournalOwner;

use crate::WinError;
use crate::log::{Level, Log, LogWriter, Proc};
use crate::power::executor::{PowerExecutor, Startup};
use crate::power::journal::{FileNet, GuardControl};
use crate::process::{self, Liveness};
use crate::session;
use crate::store::{self, DataDir, DataFile, HeldMutex, NamedMutex, Names};

const AREA: &str = "startup";

/// How long the startup restore waits before it reads a journal that did not read once more. Long enough for what
/// makes a read fail for a moment (a pending delete, an antivirus scan); short, because the tray shows no icon yet.
const JOURNAL_RETRY_MS: u64 = 500;

/// The most of a panic message that is logged. A library's error text can quote the input it failed on, and the log
/// never holds file content (architecture section 8); the start of the message says what failed.
const PANIC_TEXT_MAX: usize = 200;

/// What the later startup steps need: the data folder, the mutex names and this tray's identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Boot {
    pub dir: DataDir,
    pub names: Names,
    pub dry_run: bool,
    /// This process, read once, so the journal owner and `Started.tray` always name the same process.
    pub tray: ProcessRef,
    /// This process's session id, for the journal owner.
    pub session: u32,
}

impl Boot {
    /// Step 3. `--dry-run` gets its own folder (`dry-run` under the real one) and mutex names ending in `.DryRun`, so
    /// it never meets the real journal or a real tray. In debug builds `WAKESHOT_DATA_DIR` moves the real folder
    /// (`DataDir::real`). An `Err` (the folder or this process cannot be read) leaves the tray nothing to run on.
    pub fn new(dry_run: bool) -> Result<Self, WinError> {
        let (dir, names) = if dry_run {
            (DataDir::dry_run()?, Names::dry_run())
        } else {
            (DataDir::real()?, Names::real())
        };
        Self::at(dir, names, dry_run)
    }

    /// Any folder and names: tests use a temp folder and `Names::test`.
    pub fn at(dir: DataDir, names: Names, dry_run: bool) -> Result<Self, WinError> {
        Ok(Self {
            dir,
            names,
            dry_run,
            tray: process::current()?,
            session: process::current_session()?,
        })
    }

    /// The executor's `me`: every apply writes it as the journal's owner.
    pub fn journal_owner(&self) -> JournalOwner {
        JournalOwner {
            pid: self.tray.pid,
            proc_start: self.tray.start,
            session: self.session,
        }
    }
}

/// `Local\Wakeshot.Tray` (or its dry-run name), held: the one tray of this session.
#[derive(Debug)]
pub struct TrayLock(HeldMutex);

impl TrayLock {
    /// The previous tray of this session ended while it held the lock (a crash or a kill). It counts as taken; worth a
    /// log line once the log is open.
    pub fn abandoned(&self) -> bool {
        self.0.abandoned()
    }
}

/// Step 4. `None`: another tray of this session holds it, and the caller exits 0. No wait, so a second start ends at
/// once. It comes before the log, so two trays that start together never roll the log under each other.
pub fn take_tray(boot: &Boot) -> Result<Option<TrayLock>, WinError> {
    let mutex = NamedMutex::open(&boot.names.tray)?;
    Ok(mutex.take(0)?.map(TrayLock))
}

/// `Global\Wakeshot.Settings` (or its dry-run name), held: this tray may change power settings.
#[derive(Debug)]
pub struct SettingsLock(HeldMutex);

impl SettingsLock {
    /// A tray that held it ended without releasing it. It counts as taken.
    pub fn abandoned(&self) -> bool {
        self.0.abandoned()
    }
}

/// Step 7, and the dispatcher's retry on each safety tick while it is not the owner. `None`: another session's tray
/// holds it, or it cannot be opened (a denied open across users); both mean requests only (architecture section 4,
/// "Processes"). No error comes back: the retry runs every 30 s, so a line per failure would bury the log.
pub fn try_settings(boot: &Boot) -> Option<SettingsLock> {
    let mutex = NamedMutex::open(&boot.names.settings).ok()?;
    mutex.take(0).ok().flatten().map(SettingsLock)
}

/// Step 5: rolls the log when it is over 1 MB (three files kept), then opens it for process `tray`. A failed roll is
/// logged and the log grows until a later start rolls it. A log that cannot be opened gives a log that drops every
/// line: keeping the PC awake and restoring settings must not depend on the log.
pub fn open_log(boot: &Boot) -> Arc<dyn Log> {
    let logs = boot.dir.logs();
    let rolled = LogWriter::roll(&logs);
    let Ok(writer) = LogWriter::open(&logs, Proc::Tray) else {
        return Arc::new(NoLog);
    };
    if let Err(e) = rolled {
        let text = format!(
            "log not rolled: {} failed with {}; it grows until a later start rolls it",
            e.call, e.code
        );
        writer.line(Level::Warn, AREA, &text);
    }
    Arc::new(writer)
}

struct NoLog;

impl Log for NoLog {
    fn line(&self, _: Level, _: &'static str, _: &str) {}
}

/// Step 5: every panic, on any thread, is logged and ends the whole tray, in every build profile. Dev and test builds
/// unwind, and an IO thread that ended alone would leave the tray running with nothing to restore. No restore here:
/// the IO thread may be mid-apply; the guard restores when the tray dies, and the next start covers the rest. Only the
/// tray calls this, so tests that need unwinding keep the default hook.
pub fn install_panic_hook(log: Arc<dyn Log>) {
    std::panic::set_hook(Box::new(move |info| {
        let place = info.location().map_or_else(
            || "an unknown place".to_owned(),
            |at| format!("{}:{}", at.file(), at.line()),
        );
        let message: String = info
            .payload_as_str()
            .unwrap_or("no message")
            .chars()
            .take(PANIC_TEXT_MAX)
            .collect();
        let current = thread::current();
        let name = current.name().unwrap_or("unnamed");
        let text = format!("panic on thread {name} at {place}: {message}; the tray stops");
        log.line(Level::Error, "panic", &text);
        std::process::abort();
    }));
}

/// Step 6, before any other thread starts: the executor's startup restore, with one quick retry when `journal.json`
/// did not read (orchestrator, card Log 21:06). `Startup::unread` already makes the core retry it on each tick; this
/// retry catches a failure of a moment before the tray is up, so a leftover override goes back at once and not at the
/// first safety tick, while a lid close in between would find it still in force (`p2-6-unread-journal`). If the
/// second read fails too, that is logged and the tray goes on.
pub fn startup_restore<P: PlanIo, G: GuardControl>(
    executor: &mut PowerExecutor<P, FileNet<G>>,
    log: &dyn Log,
) -> Startup {
    restore_with_retry(executor, log, || {
        thread::sleep(Duration::from_millis(JOURNAL_RETRY_MS));
    })
}

/// `pause` runs between the two tries; tests change the folder there.
fn restore_with_retry<P: PlanIo, G: GuardControl>(
    executor: &mut PowerExecutor<P, FileNet<G>>,
    log: &dyn Log,
    pause: impl FnOnce(),
) -> Startup {
    let first = executor.startup();
    // Only a read that failed is worth a second try: a missing, corrupt or read file has given its answer.
    if !first.unread {
        return first;
    }
    let text = format!("journal.json did not read at start; one more try in {JOURNAL_RETRY_MS} ms");
    log.line(Level::Info, AREA, &text);
    pause();
    let second = executor.startup();
    if second.unread {
        // The executor's own line before this one names the failed call.
        log.line(
            Level::Warn,
            AREA,
            "journal.json still does not read after one retry; the core retries it on each tick",
        );
    }
    Startup {
        journal: second.journal,
        corrupt: first.corrupt || second.corrupt,
        unread: second.unread,
    }
}

/// `config.json` at start, and whether this is the first run. `first_run` only for a missing file: the core then shows
/// the first-run notice and writes the default. A newer file is kept for the version that wrote it, and a bad or
/// unreadable one is left for the user to see; each means the default (Automatic on) for this run, and a log line.
pub fn read_config(boot: &Boot, log: &dyn Log) -> (Config, bool) {
    let warn = |text: &str| log.line(Level::Warn, AREA, text);
    let bytes = match store::read_whole(&boot.dir.file(DataFile::Config)) {
        Ok(Some(bytes)) => bytes,
        Ok(None) => return (Config::default(), true),
        Err(e) => {
            warn(&format!(
                "config.json does not read: {} failed with {}; the default (Automatic on) for this run",
                e.call, e.code
            ));
            return (Config::default(), false);
        }
    };
    match files::read::<Config>(&bytes) {
        Read::Value(config) => (config, false),
        Read::Newer(version) => {
            warn(&format!(
                "config.json is version {version}, newer than this version's {}: ignored and kept as it is; the \
                 default (Automatic on) for this run",
                Config::VERSION
            ));
            (Config::default(), false)
        }
        Read::Bad => {
            warn(&format!(
                "config.json is not a version {} config: ignored; the default (Automatic on) for this run",
                Config::VERSION
            ));
            (Config::default(), false)
        }
    }
}

/// Removes the temp files `store::write_atomic` leaves (`<name>.<pid>.<n>.tmp`) when its process stops between the
/// temp write and the move (`p3-store`, Found outside scope), in the data folder and `holds\`, where files are
/// written. A temp is stale when its pid runs no process, or runs one that started after the temp's last write, so it
/// cannot be the writer (the hold file rule, architecture section 6). A pid of Unknown liveness keeps its file. So a
/// live writer's temp is never removed, and this is safe at any time.
pub fn remove_stale_temps(boot: &Boot, liveness: &dyn Liveness, log: &dyn Log) {
    for folder in [boot.dir.root().to_path_buf(), boot.dir.holds()] {
        let entries = match std::fs::read_dir(&folder) {
            Ok(entries) => entries,
            Err(e) if e.kind() == ErrorKind::NotFound => continue,
            Err(e) => {
                let text = format!("temp files in {} not checked: {e}", folder.display());
                log.line(Level::Warn, AREA, &text);
                continue;
            }
        };
        for entry in entries.flatten() {
            let file_name = entry.file_name();
            let Some((name, pid)) = file_name.to_str().and_then(|n| temp_pid(n).map(|p| (n, p)))
            else {
                continue;
            };
            let Ok(meta) = entry.metadata() else {
                continue;
            };
            if !meta.is_file() {
                continue;
            }
            let written = FileTime(meta.last_write_time());
            let stale = match liveness.start_of_pid(pid) {
                Lookup::NotRunning => true,
                Lookup::Running(start) => start > written,
                Lookup::Unknown => false,
            };
            if !stale {
                continue;
            }
            let (level, text) = match store::remove(&entry.path()) {
                Ok(_) => (
                    Level::Info,
                    format!("removed {name}, a temp file left by pid {pid}"),
                ),
                Err(e) => (
                    Level::Warn,
                    format!(
                        "temp file {name} left by pid {pid} not removed: {} failed with {}",
                        e.call, e.code
                    ),
                ),
            };
            log.line(level, AREA, &text);
        }
    }
}

/// The pid in a `store::write_atomic` temp name, `<name>.<pid>.<n>.tmp`; `None` for any other name.
fn temp_pid(file_name: &str) -> Option<u32> {
    let mut parts = file_name.strip_suffix(".tmp")?.rsplitn(3, '.');
    parts.next()?.parse::<u64>().ok()?;
    let pid = parts.next()?.parse::<u32>().ok()?;
    parts.next().filter(|name| !name.is_empty()).map(|_| pid)
}

/// The `Started` event, first into the core. `owner`: `try_settings` gave a lock. A console check that fails counts as
/// off the console, the safe side (requests only, the user's settings untouched), with a log line. Build it after
/// `session::register`: a console attach still in progress at sign-in reads as off the console (`p3-session`), and
/// the `WTS_CONSOLE_CONNECT` that ends it can then reach the window (not verified).
pub fn started(
    boot: &Boot,
    config: Config,
    first_run: bool,
    startup: &Startup,
    owner: bool,
    log: &dyn Log,
) -> Event {
    let on_console = session::on_console().unwrap_or_else(|e| {
        let text = format!(
            "console check failed: {} failed with {}; counted as off the console",
            e.call, e.code
        );
        log.line(Level::Warn, AREA, &text);
        false
    });
    Event::Started {
        config,
        journal: startup.journal.clone(),
        journal_unread: startup.unread,
        owner,
        on_console,
        tray: boot.tray,
        first_run,
    }
}

/// Whether to post `Session(SessionChange::Locked)` right after `Started`: the only way the core learns of a lock that
/// began before the tray (`docs/reviews/p1-close-verify.md` Major 1). Read it after `session::register`, so a lock
/// that comes between the two is still seen: at worst twice, which leaves the core locked as before.
pub fn locked_at_start(log: &dyn Log) -> bool {
    locked_from(session::locked_now(), log)
}

/// An unknown lock state counts as unlocked. If the session was locked after all, a lid close then looks like a lock
/// that failed, and the core steps back toward sleep (`core/lid.rs`): the safe side. Counted as locked, a lock that
/// really failed would go unseen.
fn locked_from(state: Option<bool>, log: &dyn Log) -> bool {
    state.unwrap_or_else(|| {
        log.line(
            Level::Warn,
            AREA,
            "lock state at start unknown; counted as unlocked",
        );
        false
    })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::{Child, Command, ExitStatus, Stdio};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::Instant;

    use wakeshot_core::files::{JournalFile, JournalFileEntry, JournalKey};
    use wakeshot_core::ownership::fake::{FakePlans, SetBy};
    use wakeshot_core::process::Alive;
    use wakeshot_core::settings::{Guid, Journal, JournalEntry, PlanValues, SettingKey};

    use super::*;
    use crate::log::MemoryLog;
    use crate::power::journal::NoGuard;
    use crate::process::FakeLiveness;

    const BALANCED: Guid = Guid(0x381b4222_f694_41f0_9685_ff5bb260df2e);
    const ME: JournalOwner = JournalOwner {
        pid: 4242,
        proc_start: FileTime(134_381_300_000_000_000),
        session: 1,
    };
    const DEAD: JournalOwner = JournalOwner {
        pid: 9120,
        proc_start: FileTime(134_381_211_391_959_336),
        session: 1,
    };
    const KEYS: [SettingKey; 3] = [SettingKey::LidAc, SettingKey::LidDc, SettingKey::SleepDc];

    /// `std::process::abort()` on Windows is a fast fail, which ends the process with `STATUS_STACK_BUFFER_OVERRUN`.
    const ABORT_EXIT: u32 = 0xC000_0409;
    /// A test that panics with the default hook fails, and the test binary exits with this.
    const PANIC_EXIT: i32 = 101;
    const PANIC_CHILD: &str = "WAKESHOT_STARTUP_TEST_PANIC_LOGS";

    /// A fresh folder under `%TEMP%`, removed with everything in it when the test ends.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(tag: &str) -> Self {
            static N: AtomicU64 = AtomicU64::new(0);
            let n = N.fetch_add(1, Ordering::Relaxed);
            let dir = std::env::temp_dir()
                .join(format!("wakeshot-startup-{tag}-{}-{n}", std::process::id()));
            fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn boot(tag: &str) -> (TempDir, Boot) {
        let dir = TempDir::new(tag);
        let names = Names::test(&format!("startup-{tag}"));
        let boot = Boot::at(DataDir::at(dir.0.clone()), names, false).unwrap();
        (dir, boot)
    }

    fn names_in(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    fn lines(log: &MemoryLog) -> Vec<(Level, &'static str, String)> {
        log.lines.lock().unwrap().clone()
    }

    /// Haik's values (CLAUDE.md, "Test laptop").
    fn haiks() -> PlanValues {
        PlanValues {
            active: BALANCED,
            lid_ac: 1,
            lid_dc: 1,
            sleep_ac: 10_800,
            sleep_dc: 14_400,
            saver_threshold_dc: 30,
        }
    }

    fn user_value(key: SettingKey) -> u32 {
        match key {
            SettingKey::LidAc | SettingKey::LidDc => 1,
            SettingKey::SleepDc => 14_400,
        }
    }

    fn entry(key: SettingKey) -> JournalEntry {
        JournalEntry {
            plan: BALANCED,
            key,
            original: user_value(key),
            written: 0,
            restored: false,
        }
    }

    /// The plans, the log and the liveness answers one test's executors share.
    struct Rig {
        fake: FakePlans,
        liveness: Arc<FakeLiveness>,
        log: Arc<MemoryLog>,
    }

    impl Rig {
        fn new() -> Self {
            Self {
                fake: FakePlans::new(haiks()),
                liveness: Arc::default(),
                log: Arc::default(),
            }
        }

        /// The tray's executor over a real journal file in the boot folder, with the dry run's safety net (no RunOnce,
        /// no guard) and fake plans.
        fn executor(&self, boot: &Boot) -> PowerExecutor<FakePlans, FileNet<NoGuard>> {
            PowerExecutor::new(
                self.fake.clone(),
                FileNet::dry_run(boot.dir.clone(), self.log.clone()),
                ME,
                boot.names.clone(),
                boot.dir.clone(),
                self.liveness.clone(),
                self.log.clone(),
            )
        }

        /// What an apply by `owner` left: our 0 stored and in force for every key, and its journal on disk.
        fn overridden_by(&self, boot: &Boot, owner: JournalOwner) {
            {
                let mut s = self.fake.state();
                for key in KEYS {
                    s.set_stored(BALANCED, key, 0, SetBy::Override);
                    s.set_in_force(BALANCED, key, 0, SetBy::Override);
                }
            }
            let file = JournalFile {
                version: JournalFile::VERSION,
                owner,
                entries: KEYS
                    .iter()
                    .map(|&key| JournalFileEntry {
                        plan: BALANCED,
                        key: JournalKey::Known(key),
                        original: user_value(key),
                        written: 0,
                        restored: false,
                    })
                    .collect(),
            };
            let bytes = serde_json::to_vec(&file).unwrap();
            fs::write(boot.dir.file(DataFile::Journal), bytes).unwrap();
        }

        fn alive(&self, owner: JournalOwner, alive: Alive) {
            let process = ProcessRef {
                pid: owner.pid,
                start: owner.proc_start,
            };
            self.liveness.alive.lock().unwrap().push((process, alive));
        }

        fn stored(&self) -> Vec<Option<u32>> {
            let s = self.fake.state();
            KEYS.iter().map(|&key| s.stored(BALANCED, key)).collect()
        }

        fn warned(&self, part: &str) -> usize {
            lines(&self.log)
                .iter()
                .filter(|(level, _, text)| *level == Level::Warn && text.contains(part))
                .count()
        }
    }

    fn restored_values() -> Vec<Option<u32>> {
        KEYS.iter().map(|&key| Some(user_value(key))).collect()
    }

    #[test]
    fn boot_names_and_folders() {
        let real = Boot::new(false).unwrap();
        let dry = Boot::new(true).unwrap();
        assert_eq!(real.names, Names::real());
        assert_eq!(dry.names, Names::dry_run());
        assert!(dry.names.tray.ends_with(".DryRun"));
        assert_eq!(dry.dir.root(), real.dir.root().join("dry-run"));
        assert!(!real.dry_run && dry.dry_run);

        let me = process::current().unwrap();
        assert_eq!(real.tray, me);
        assert_eq!(
            real.journal_owner(),
            JournalOwner {
                pid: me.pid,
                proc_start: me.start,
                session: process::current_session().unwrap(),
            }
        );
    }

    /// `take` on the thread that holds a mutex succeeds again (Windows counts per owning thread), so the second tray
    /// and the second settings owner are other threads.
    fn on_other_thread<T: Send + 'static>(boot: &Boot, take: fn(&Boot) -> T) -> T {
        let boot = boot.clone();
        thread::spawn(move || take(&boot)).join().unwrap()
    }

    #[test]
    fn second_tray_is_refused_while_the_first_holds_the_lock() {
        let (_dir, boot) = boot("tray");
        let second = |boot: &Boot| take_tray(boot).unwrap().is_some();

        let first = take_tray(&boot)
            .unwrap()
            .expect("the first tray takes the lock");
        assert!(!first.abandoned());
        assert!(!on_other_thread(&boot, second));

        drop(first);
        assert!(on_other_thread(&boot, second));
    }

    #[test]
    fn a_tray_lock_left_by_an_ended_tray_counts_as_taken() {
        let (_dir, boot) = boot("tray-abandoned");
        // The thread ends while it holds the lock, as a tray that crashed.
        on_other_thread(&boot, |boot| {
            std::mem::forget(take_tray(boot).unwrap().unwrap())
        });

        let lock = take_tray(&boot)
            .unwrap()
            .expect("an abandoned lock is taken");
        assert!(lock.abandoned());
    }

    #[test]
    fn settings_lock_has_one_owner() {
        let (_dir, boot) = boot("settings");
        let second = |boot: &Boot| try_settings(boot).is_some();

        let first = try_settings(&boot).expect("the first tray owns the settings");
        assert!(!first.abandoned());
        assert!(!on_other_thread(&boot, second));

        drop(first);
        assert!(on_other_thread(&boot, second));
    }

    #[test]
    fn read_config_missing_is_the_first_run() {
        let (_dir, boot) = boot("config-missing");
        let log = MemoryLog::default();

        assert_eq!(read_config(&boot, &log), (Config::default(), true));
        assert!(lines(&log).is_empty());
    }

    #[test]
    fn read_config_takes_its_own_version() {
        let (_dir, boot) = boot("config-own");
        let log = MemoryLog::default();
        // A BOM, as Windows PowerShell 5.1 writes, and a field a later version may add.
        let bytes = b"\xEF\xBB\xBF{ \"version\": 1, \"automatic\": false, \"later\": 3 }\r\n";
        fs::write(boot.dir.file(DataFile::Config), bytes).unwrap();

        assert_eq!(read_config(&boot, &log), (Config::new(false), false));
        assert!(lines(&log).is_empty());
    }

    #[test]
    fn read_config_newer_keeps_the_file_byte_for_byte() {
        let (_dir, boot) = boot("config-newer");
        let log = MemoryLog::default();
        let path = boot.dir.file(DataFile::Config);
        let bytes = b"{ \"version\": 2, \"automatic\": { \"mode\": \"off\" } }";
        fs::write(&path, bytes).unwrap();

        assert_eq!(read_config(&boot, &log), (Config::default(), false));
        assert_eq!(fs::read(&path).unwrap(), bytes);
        let lines = lines(&log);
        assert_eq!(lines.len(), 1, "{lines:?}");
        assert_eq!(lines[0].0, Level::Warn);
        assert!(lines[0].2.contains("version 2"), "{lines:?}");
    }

    #[test]
    fn read_config_bad_gives_the_default_and_a_line() {
        let (_dir, boot) = boot("config-bad");
        let log = MemoryLog::default();
        let path = boot.dir.file(DataFile::Config);
        fs::write(&path, b"{ \"version\": 1, \"automatic\": \"yes\"").unwrap();

        assert_eq!(read_config(&boot, &log), (Config::default(), false));
        let bad = lines(&log);
        assert_eq!(bad.len(), 1, "{bad:?}");
        assert!(bad[0].2.contains("not a version 1 config"), "{bad:?}");

        // A file that does not open (a folder in its place) is the same: the default, a line, no first run.
        fs::remove_file(&path).unwrap();
        fs::create_dir(&path).unwrap();
        let log = MemoryLog::default();
        assert_eq!(read_config(&boot, &log), (Config::default(), false));
        let unread = lines(&log);
        assert_eq!(unread.len(), 1, "{unread:?}");
        assert!(unread[0].2.contains("does not read"), "{unread:?}");
    }

    #[test]
    fn corrupt_journal_left_by_others_is_reported_once() {
        let (_dir, boot) = boot("corrupt-left");
        let rig = Rig::new();
        fs::write(
            boot.dir.file(DataFile::JournalCorrupt),
            b"{ set aside by restore mode",
        )
        .unwrap();

        let first = startup_restore(&mut rig.executor(&boot), rig.log.as_ref());
        assert!(first.corrupt);
        assert_eq!(first.journal, None);
        assert_eq!(names_in(boot.dir.root()), ["journal.json.corrupt.seen"]);

        let second = startup_restore(&mut rig.executor(&boot), rig.log.as_ref());
        assert!(!second.corrupt);
        assert_eq!(names_in(boot.dir.root()), ["journal.json.corrupt.seen"]);
    }

    #[test]
    fn startup_restore_restores_a_gone_owner() {
        let (_dir, boot) = boot("restore-gone");
        let rig = Rig::new();
        rig.overridden_by(&boot, DEAD);

        let startup = startup_restore(&mut rig.executor(&boot), rig.log.as_ref());

        assert_eq!(
            startup,
            Startup {
                journal: None,
                corrupt: false,
                unread: false,
            }
        );
        assert_eq!(rig.stored(), restored_values());
        assert!(!boot.dir.file(DataFile::Journal).exists());
    }

    #[test]
    fn startup_restore_keeps_a_live_owners_entries() {
        for alive in [Alive::Yes, Alive::Unknown] {
            let (_dir, boot) = boot("restore-alive");
            let rig = Rig::new();
            rig.overridden_by(&boot, DEAD);
            rig.alive(DEAD, alive);
            let mut paused = false;

            let startup = restore_with_retry(&mut rig.executor(&boot), rig.log.as_ref(), || {
                paused = true;
            });

            let journal = Journal {
                owner: DEAD,
                entries: KEYS.iter().map(|&key| entry(key)).collect(),
            };
            assert_eq!(startup.journal, Some(journal), "{alive:?}");
            assert!(!paused, "a journal that read is not tried again");
            assert_eq!(rig.stored(), [Some(0); 3]);
            assert!(boot.dir.file(DataFile::Journal).exists());
        }
    }

    #[test]
    fn startup_restore_without_a_journal_tries_once() {
        let (_dir, boot) = boot("restore-none");
        let rig = Rig::new();
        let mut paused = false;

        let startup = restore_with_retry(&mut rig.executor(&boot), rig.log.as_ref(), || {
            paused = true;
        });

        assert_eq!(
            startup,
            Startup {
                journal: None,
                corrupt: false,
                unread: false,
            }
        );
        assert!(!paused);
    }

    #[test]
    fn a_journal_read_that_fails_is_tried_once_more() {
        let (_dir, boot) = boot("retry-works");
        let rig = Rig::new();
        rig.overridden_by(&boot, DEAD);
        let journal = boot.dir.file(DataFile::Journal);
        let left = fs::read(&journal).unwrap();
        // A folder in the file's place makes the open fail with 5, as a pending delete does.
        fs::remove_file(&journal).unwrap();
        fs::create_dir(&journal).unwrap();
        let mut pauses = 0;

        let startup = restore_with_retry(&mut rig.executor(&boot), rig.log.as_ref(), || {
            pauses += 1;
            fs::remove_dir(&journal).unwrap();
            fs::write(&journal, &left).unwrap();
        });

        assert_eq!(pauses, 1);
        assert_eq!(startup.journal, None);
        assert!(!startup.unread, "the second read worked");
        assert_eq!(rig.stored(), restored_values());
        assert!(!journal.exists());
        assert_eq!(rig.warned("still does not read"), 0);
    }

    #[test]
    fn a_journal_that_still_does_not_read_is_logged() {
        let (_dir, boot) = boot("retry-fails");
        let rig = Rig::new();
        fs::create_dir(boot.dir.file(DataFile::Journal)).unwrap();
        let mut pauses = 0;

        let startup = restore_with_retry(&mut rig.executor(&boot), rig.log.as_ref(), || {
            pauses += 1;
        });

        assert_eq!(pauses, 1);
        assert_eq!(
            startup,
            Startup {
                journal: None,
                corrupt: false,
                unread: true,
            }
        );
        assert_eq!(rig.warned("still does not read after one retry"), 1);
        // The executor's own line for each of the two reads.
        assert_eq!(rig.warned("startup: journal.json does not read"), 2);
    }

    #[test]
    fn started_carries_this_tray() {
        let (_dir, boot) = boot("started");
        let log = MemoryLog::default();
        let journal = Journal {
            owner: DEAD,
            entries: vec![entry(SettingKey::LidDc)],
        };
        let me = process::current().unwrap();

        // `unread` never comes with a journal from the executor; the copy is checked field by field all the same.
        for (owner, first_run, unread) in [(true, false, false), (false, true, true)] {
            let startup = Startup {
                journal: Some(journal.clone()),
                corrupt: true,
                unread,
            };
            let event = started(&boot, Config::new(false), first_run, &startup, owner, &log);
            let Event::Started {
                config,
                journal: got,
                journal_unread,
                owner: got_owner,
                on_console,
                tray,
                first_run: got_first_run,
            } = event
            else {
                panic!("not Started: {event:?}");
            };
            assert_eq!(tray, me);
            assert_eq!(config, Config::new(false));
            assert_eq!(got, Some(journal.clone()));
            assert_eq!(journal_unread, unread);
            assert_eq!((got_owner, got_first_run), (owner, first_run));
            assert_eq!(on_console, session::on_console().unwrap_or(false));
        }
    }

    #[test]
    fn lock_state_at_start() {
        let log = MemoryLog::default();
        assert!(locked_from(Some(true), &log));
        assert!(!locked_from(Some(false), &log));
        assert!(lines(&log).is_empty());

        assert!(!locked_from(None, &log));
        let unknown = lines(&log);
        assert_eq!(unknown.len(), 1, "{unknown:?}");
        assert!(unknown[0].2.contains("unknown"), "{unknown:?}");
    }

    #[test]
    fn temp_names_give_their_pid() {
        assert_eq!(temp_pid("config.json.1234.0.tmp"), Some(1234));
        assert_eq!(temp_pid("run-77.json.4.18446744073709551615.tmp"), Some(4));
        for other in [
            "config.json",
            "notes.tmp",
            "1234.0.tmp",
            ".1234.0.tmp",
            "config.json.x.0.tmp",
            "config.json.1234.x.tmp",
            "config.json.99999999999.0.tmp",
            "config.json.1234.0.tmp.json",
        ] {
            assert_eq!(temp_pid(other), None, "{other}");
        }
    }

    #[test]
    fn stale_temp_files_are_removed() {
        let (_dir, boot) = boot("temps");
        let liveness = FakeLiveness::default();
        let log = MemoryLog::default();
        let holds = boot.dir.holds();
        fs::create_dir(&holds).unwrap();
        let root = boot.dir.root();
        // Pid 10 runs no process (`FakeLiveness` answers `NotRunning` for any pid it was not given).
        fs::write(root.join("config.json.10.0.tmp"), b"{}").unwrap();
        fs::write(holds.join("run-5.json.10.1.tmp"), b"{}").unwrap();
        // Pid 20 started before the write, so it may be the writer; pid 30 started after it, a reused pid.
        fs::write(root.join("state.json.20.2.tmp"), b"{}").unwrap();
        fs::write(root.join("journal.json.30.3.tmp"), b"{}").unwrap();
        // Pid 40's liveness is Unknown.
        fs::write(holds.join("keep-awake.json.40.4.tmp"), b"{}").unwrap();
        // Not temp files of ours, or not files.
        fs::write(root.join("config.json"), b"{}").unwrap();
        fs::write(root.join("notes.tmp"), b"").unwrap();
        fs::create_dir(root.join("old.json.10.5.tmp")).unwrap();
        *liveness.starts.lock().unwrap() = vec![
            (20, Lookup::Running(FileTime(1))),
            (30, Lookup::Running(FileTime(u64::MAX))),
            (40, Lookup::Unknown),
        ];

        remove_stale_temps(&boot, &liveness, &log);

        assert_eq!(
            names_in(root),
            [
                "config.json",
                "holds",
                "notes.tmp",
                "old.json.10.5.tmp",
                "state.json.20.2.tmp"
            ]
        );
        assert_eq!(names_in(&holds), ["keep-awake.json.40.4.tmp"]);
        let mut removed: Vec<String> = lines(&log)
            .into_iter()
            .map(|(level, _, text)| {
                assert_eq!(level, Level::Info, "{text}");
                text
            })
            .collect();
        removed.sort();
        assert_eq!(
            removed,
            [
                "removed config.json.10.0.tmp, a temp file left by pid 10",
                "removed journal.json.30.3.tmp, a temp file left by pid 30",
                "removed run-5.json.10.1.tmp, a temp file left by pid 10",
            ]
        );
    }

    #[test]
    fn stale_temps_in_a_new_folder_need_nothing() {
        let (dir, _) = boot("temps-none");
        let boot = Boot::at(
            DataDir::at(dir.0.join("not-made-yet")),
            Names::test("t"),
            false,
        )
        .unwrap();
        let log = MemoryLog::default();
        remove_stale_temps(&boot, &FakeLiveness::default(), &log);
        assert!(lines(&log).is_empty());
    }

    #[test]
    fn open_log_rolls_then_opens_as_tray() {
        let (_dir, boot) = boot("log");
        let logs = boot.dir.logs();
        fs::create_dir(&logs).unwrap();
        let old = vec![b'x'; 1024 * 1024 + 1];
        fs::write(logs.join("wakeshot.log"), &old).unwrap();

        let log = open_log(&boot);
        log.line(Level::Info, "test", "first line after the roll");
        drop(log);

        assert_eq!(fs::read(logs.join("wakeshot.1.log")).unwrap(), old);
        let text = fs::read_to_string(logs.join("wakeshot.log")).unwrap();
        assert_eq!(text.lines().count(), 1, "{text}");
        assert!(
            text.contains(" tray ") && text.contains("test: first line after the roll"),
            "{text}"
        );
    }

    #[test]
    fn open_log_makes_the_folder() {
        let (_dir, boot) = boot("log-new");
        let log = open_log(&boot);
        log.line(Level::Info, "test", "hello");
        drop(log);
        let text = fs::read_to_string(boot.dir.logs().join("wakeshot.log")).unwrap();
        assert!(text.contains("test: hello"), "{text}");
    }

    fn wait_up_to(child: &mut Child, limit: Duration) -> ExitStatus {
        let end = Instant::now() + limit;
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                return status;
            }
            if Instant::now() > end {
                let _ = child.kill();
                panic!("the child did not end within {limit:?}");
            }
            thread::sleep(Duration::from_millis(50));
        }
    }

    #[test]
    fn panic_hook_logs_then_aborts() {
        if let Some(logs) = std::env::var_os(PANIC_CHILD) {
            let log = Arc::new(LogWriter::open(Path::new(&logs), Proc::Tray).unwrap());
            install_panic_hook(log);
            panic!("startup test panic {}", "x".repeat(PANIC_TEXT_MAX));
        }
        let dir = TempDir::new("panic");
        let logs = dir.0.join("logs");
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args([
                "app::startup::tests::panic_hook_logs_then_aborts",
                "--exact",
                "--test-threads=1",
            ])
            .env(PANIC_CHILD, &logs)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();

        let status = wait_up_to(&mut child, Duration::from_secs(60));

        assert_ne!(status.code(), Some(PANIC_EXIT), "the panic unwound");
        assert_eq!(status.code().map(|c| c as u32), Some(ABORT_EXIT));
        let text = fs::read_to_string(logs.join("wakeshot.log")).unwrap();
        let panics: Vec<&str> = text.lines().filter(|l| l.contains("panic")).collect();
        assert_eq!(panics.len(), 1, "{text}");
        let line = panics[0];
        assert!(line.contains(" tray ") && line.contains("ERROR"), "{line}");
        assert!(line.contains("startup.rs:"), "{line}");
        assert!(line.contains("startup test panic"), "{line}");
        // Cut to the limit: the message's tail never reaches the log.
        assert!(!line.contains(&"x".repeat(PANIC_TEXT_MAX)), "{line}");
    }
}
