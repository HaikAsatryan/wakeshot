//! The tray exe's guard and restore modes, run for real: real processes and a temp data folder with no journal in
//! it, so no power plan, RunOnce value or real data folder is touched.

use std::ffi::OsString;
use std::path::PathBuf;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::{fs, thread};

use wakeshot_core::process::{Lookup, ProcessRef};
use wakeshot_windows::power::journal::GuardControl;
use wakeshot_windows::process::{self, Liveness, SystemLiveness};
use wakeshot_windows::recovery::{GuardArgs, STOP_EVENT_PREFIX, StopEvent, TrayGuard};
use wakeshot_windows::testing::MemoryLog;

const TRAY_EXE: &str = env!("CARGO_BIN_EXE_wakeshot-tray");
/// Read by debug builds of the exe only (`store::DataDir::real`).
const DATA_DIR_ENV: &str = "WAKESHOT_DATA_DIR";
const SLEEP_ARG: &str = "wakeshot-recovery-sleep-ms=";
/// A sleeping child that outlives its test (a crashed run) ends on its own after this.
const SLEEP_MAX_MS: u64 = 120_000;
/// The limit for a guard to leave once its tray ended or its event was set.
const LEAVE_WITHIN: Duration = Duration::from_secs(2);

/// A release exe ignores `WAKESHOT_DATA_DIR` and would look at the real data folder, so these tests run only
/// against a debug build.
fn release_build() -> bool {
    if cfg!(debug_assertions) {
        return false;
    }
    eprintln!("skipped: a release exe reads the real data folder");
    true
}

/// A fresh folder under `%TEMP%`, removed with everything in it when the test ends.
struct TempDir(PathBuf);

impl TempDir {
    fn new(tag: &str) -> Self {
        static N: AtomicU64 = AtomicU64::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "wakeshot-tray-recovery-{tag}-{}-{n}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    fn log(&self) -> String {
        fs::read_to_string(self.0.join("logs").join("wakeshot.log")).unwrap_or_default()
    }

    /// Waits up to 5 s for a log line that holds `part`.
    fn wait_for_line(&self, part: &str) {
        let started = Instant::now();
        while !self.log().contains(part) {
            assert!(
                started.elapsed() < Duration::from_secs(5),
                "no log line with {part:?} in:\n{}",
                self.log()
            );
            thread::sleep(Duration::from_millis(20));
        }
    }

    /// Only the log folder: no journal, no other file.
    fn assert_only_logs(&self) {
        let names: Vec<OsString> = fs::read_dir(&self.0)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(names, ["logs"]);
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn event_name() -> String {
    static N: AtomicU64 = AtomicU64::new(2_000_000);
    let n = N.fetch_add(1, Ordering::Relaxed);
    format!("{STOP_EVENT_PREFIX}{}.{n}", std::process::id())
}

/// A child process ended and reaped on drop, so a failed assert leaves no process.
struct Kid(Child);

impl Kid {
    /// This test exe in its sleep role: the sacrificial "tray".
    fn sleeping() -> Self {
        let args = [
            "--exact".to_owned(),
            "--ignored".to_owned(),
            "--test-threads=1".to_owned(),
            "--quiet".to_owned(),
            "--".to_owned(),
            "sleep_role".to_owned(),
            format!("{SLEEP_ARG}{SLEEP_MAX_MS}"),
        ];
        let exe = std::env::current_exe().expect("test exe path");
        Self::start(Command::new(exe).args(args), "sleeping tray")
    }

    /// The tray exe as a guard, with `dir` as its data folder.
    fn guard(args: &GuardArgs, dir: &TempDir) -> Self {
        let mut command = Command::new(TRAY_EXE);
        command.args(args.command()).env(DATA_DIR_ENV, &dir.0);
        Self::start(&mut command, "guard")
    }

    fn start(command: &mut Command, what: &str) -> Self {
        let child = command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("start child");
        println!("{what} pid {}", child.id());
        Self(child)
    }

    fn process(&self) -> ProcessRef {
        ProcessRef {
            pid: self.0.id(),
            start: process::start_of(&self.0).expect("child start"),
        }
    }

    /// The exit status once it ends within `limit`.
    fn exit_within(&mut self, limit: Duration) -> Option<ExitStatus> {
        let started = Instant::now();
        loop {
            if let Some(status) = self.0.try_wait().unwrap() {
                return Some(status);
            }
            if started.elapsed() > limit {
                return None;
            }
            thread::sleep(Duration::from_millis(10));
        }
    }

    fn end(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

impl Drop for Kid {
    fn drop(&mut self) {
        self.end();
    }
}

/// Started by `Kid::sleeping`, it sleeps; started any other way it ends at once.
#[test]
#[ignore = "a child role for the other tests of this file"]
fn sleep_role() {
    let ms = std::env::args().find_map(|a| a.strip_prefix(SLEEP_ARG)?.parse::<u64>().ok());
    if let Some(ms) = ms {
        thread::sleep(Duration::from_millis(ms.min(SLEEP_MAX_MS)));
    }
}

#[test]
fn guard_exits_when_its_tray_dies() {
    if release_build() {
        return;
    }
    let dir = TempDir::new("tray-dies");
    let mut tray = Kid::sleeping();
    let name = event_name();
    let _event = StopEvent::create(&name).unwrap().expect("a new name");
    let args = GuardArgs {
        tray: tray.process(),
        stop_event: name,
    };
    let mut guard = Kid::guard(&args, &dir);
    dir.wait_for_line("guard started for tray pid");
    // Time to open the tray and its event, so the guard sees the end rather than a tray already gone.
    thread::sleep(Duration::from_millis(300));
    let ended = Instant::now();
    tray.end();
    let status = guard.exit_within(LEAVE_WITHIN);
    println!("guard left {:?} after the tray ended", ended.elapsed());
    assert_eq!(status.map(|s| s.code()), Some(Some(0)), "{}", dir.log());
    let log = dir.log();
    let pid = args.tray.pid;
    assert!(log.contains(&format!("tray pid {pid} is gone")), "{log}");
    assert!(
        log.contains(&format!(
            "guard for tray pid {pid}: no journal, nothing to restore"
        )),
        "{log}"
    );
    assert!(!log.contains("is not running"), "{log}");
    assert!(log.contains(" guard "), "the process column: {log}");
    dir.assert_only_logs();
}

#[test]
fn guard_exits_stopped_when_its_event_is_set() {
    if release_build() {
        return;
    }
    let dir = TempDir::new("stopped");
    let tray = Kid::sleeping();
    let name = event_name();
    let event = StopEvent::create(&name).unwrap().expect("a new name");
    let args = GuardArgs {
        tray: tray.process(),
        stop_event: name,
    };
    let mut guard = Kid::guard(&args, &dir);
    dir.wait_for_line("guard started for tray pid");
    thread::sleep(Duration::from_millis(300));
    assert_eq!(guard.exit_within(Duration::ZERO), None, "{}", dir.log());
    let set = Instant::now();
    event.set().unwrap();
    let status = guard.exit_within(LEAVE_WITHIN);
    println!("guard left {:?} after the set", set.elapsed());
    assert_eq!(status.map(|s| s.code()), Some(Some(0)), "{}", dir.log());
    let log = dir.log();
    let line = format!("guard for tray pid {}: stopped", args.tray.pid);
    assert!(log.contains(&line), "{log}");
    assert!(!log.contains("is gone"), "{log}");
    dir.assert_only_logs();
}

#[test]
fn guard_with_a_bad_command_line_exits_2() {
    if release_build() {
        return;
    }
    let dir = TempDir::new("bad-args");
    let mut command = Command::new(TRAY_EXE);
    command
        .args(["--guard", "1", "2"])
        .env(DATA_DIR_ENV, &dir.0);
    let mut guard = Kid::start(&mut command, "guard");
    let status = guard.exit_within(Duration::from_secs(5));
    assert_eq!(status.map(|s| s.code()), Some(Some(2)));
    assert!(
        dir.log().contains("guard: bad command line"),
        "{}",
        dir.log()
    );
}

#[test]
fn restore_without_journal_exits_at_once() {
    if release_build() {
        return;
    }
    let dir = TempDir::new("restore");
    let mut command = Command::new(TRAY_EXE);
    command.arg("--restore").env(DATA_DIR_ENV, &dir.0);
    let started = Instant::now();
    let mut restore = Kid::start(&mut command, "restore mode");
    let status = restore.exit_within(Duration::from_secs(5));
    println!("restore mode took {:?}", started.elapsed());
    assert_eq!(status.map(|s| s.code()), Some(Some(0)), "{}", dir.log());
    let log = dir.log();
    assert!(log.contains("restore mode: no journal"), "{log}");
    assert!(!log.contains("try 1"), "no retry: {log}");
    dir.assert_only_logs();
}

/// `TrayGuard` starts guards with this process's environment, so a test that uses it sets `WAKESHOT_DATA_DIR` for
/// the spawn only, one test at a time.
static SPAWN_ENV: Mutex<()> = Mutex::new(());

fn ensure_with_data_dir(guard: &mut TrayGuard, dir: &TempDir) -> bool {
    let _one = SPAWN_ENV
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    // SAFETY: std documents `set_var` and `remove_var` as always sound on Windows (library/std/src/env.rs); the
    // lock keeps two tests of this file from racing on the value a spawn copies.
    unsafe { std::env::set_var(DATA_DIR_ENV, &dir.0) };
    let started = guard.ensure();
    // SAFETY: as above.
    unsafe { std::env::remove_var(DATA_DIR_ENV) };
    started.expect("guard start")
}

/// Waits until no process has `pid`.
fn gone_within(pid: u32, limit: Duration) -> bool {
    let started = Instant::now();
    while SystemLiveness.start_of_pid(pid) != Lookup::NotRunning {
        if started.elapsed() > limit {
            return false;
        }
        thread::sleep(Duration::from_millis(10));
    }
    true
}

/// A `TrayGuard` over the real exe for a sacrificial tray, with the guard it started. The drop tells the guard to
/// stop; the tray's drop ends it too, since it waits on that tray.
struct RealGuard {
    guard: TrayGuard,
    pid: u32,
    log: Arc<MemoryLog>,
    // Dropped after `guard`'s stop: a guard whose tray ends finds no journal and leaves.
    _tray: Kid,
}

impl RealGuard {
    fn start(dir: &TempDir) -> Self {
        let tray = Kid::sleeping();
        let log = Arc::new(MemoryLog::default());
        let mut guard = TrayGuard::new(PathBuf::from(TRAY_EXE), tray.process(), log.clone());
        assert!(ensure_with_data_dir(&mut guard, dir));
        let pid = guard.guard_pid().expect("a guard");
        println!("guard pid {pid}");
        dir.wait_for_line("guard started for tray pid");
        Self {
            guard,
            pid,
            log,
            _tray: tray,
        }
    }

    fn outside_tree(&self) -> bool {
        let lines = self.log.lines.lock().unwrap();
        !lines
            .iter()
            .any(|(_, _, text)| text.contains("runs as the tray's own child"))
    }
}

impl Drop for RealGuard {
    fn drop(&mut self) {
        let _ = self.guard.stop();
    }
}

#[test]
fn tray_guard_runs_the_real_guard_until_stopped() {
    if release_build() {
        return;
    }
    let dir = TempDir::new("tray-guard");
    let mut real = RealGuard::start(&dir);
    println!("outside the tree: {}", real.outside_tree());
    assert!(real.guard.running());
    thread::sleep(Duration::from_millis(300));
    assert!(real.guard.running(), "{}", dir.log());
    let set = Instant::now();
    real.guard.stop().unwrap();
    assert!(gone_within(real.pid, LEAVE_WITHIN), "{}", dir.log());
    println!("guard left {:?} after the stop", set.elapsed());
    assert!(dir.log().contains(": stopped"), "{}", dir.log());
    dir.assert_only_logs();
}

/// With Explorer as its parent the guard lives on when the tray's process group is ended. Run with
/// `cargo test --locked -p wakeshot-tray --test recovery -- --ignored`.
#[test]
#[ignore = "needs Explorer as the shell"]
fn real_guard_parent_is_explorer() {
    if release_build() {
        return;
    }
    let dir = TempDir::new("parent");
    let mut real = RealGuard::start(&dir);
    assert!(
        real.outside_tree(),
        "the guard started as the tray's own child"
    );
    let script = format!(
        "$p = (Get-CimInstance Win32_Process -Filter 'ProcessId={}').ParentProcessId; \
         (Get-Process -Id $p).ProcessName + ' ' + $p",
        real.pid
    );
    let out = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .output()
        .expect("powershell");
    let answer = String::from_utf8_lossy(&out.stdout).trim().to_owned();
    println!("guard pid {} parent: {answer}", real.pid);
    let name = answer.split(' ').next().unwrap_or_default();
    assert!(name.eq_ignore_ascii_case("explorer"), "{answer}");
    real.guard.stop().unwrap();
    assert!(gone_within(real.pid, LEAVE_WITHIN), "{}", dir.log());
}
