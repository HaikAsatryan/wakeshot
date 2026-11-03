//! One dry-run start of the real tray exe, at Medium through Explorer, on a temp data folder: it starts, holds for a
//! script hold, goes idle when the hold goes, and quits clean (`docs/phases/06-integration.md` note 7).
//!
//! Owner: `p6-int-tray-mode`. Ignored, and named `real_e2e_*`: the dry-run icon shows on the taskbar, so it runs only
//! in a screen slot, alone, with its test target as the filter (`--test smoke -- --ignored --test-threads=1`).

use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};
use std::{fs, thread};

use wakeshot_core::files::{self, Read, StateFile};
use wakeshot_core::process::ProcessRef;
use wakeshot_windows::app::window::ask_quit;
use wakeshot_windows::process::{self, ProcessHandle};
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::System::Threading::{
    GetExitCodeProcess, OpenProcess, PROCESS_NAME_WIN32, PROCESS_TERMINATE,
    QueryFullProcessImageNameW, TerminateProcess,
};
use windows::Win32::UI::WindowsAndMessaging::FindWindowW;
use windows::core::{PCWSTR, PWSTR, w};

const TRAY_EXE: &str = env!("CARGO_BIN_EXE_wakeshot-tray");
/// Read by debug builds of the exe only (`store::DataDir::real`); a dry run uses `<it>\dry-run`.
const DATA_DIR_ENV: &str = "WAKESHOT_DATA_DIR";
/// Claude Code's own folder variable, which the tray follows. An empty folder keeps the agent sessions that run on this
/// PC (this test runs inside one) from holding the dry run.
const CLAUDE_DIR_ENV: &str = "CLAUDE_CONFIG_DIR";
/// The dry-run loop window's class (`app::window`), which `ask_quit(true)` looks for.
const LOOP_CLASS: PCWSTR = w!("Wakeshot.Loop.DryRun");
/// `whoami /groups` names the Medium level by this SID.
const MEDIUM_SID: &str = "S-1-16-8192";
/// The tray reads the folder, rolls the log, runs the startup restore and draws the icon before its first state write.
const START_WITHIN: Duration = Duration::from_secs(30);
/// The card's limit for the state to follow a hold.
const FOLLOW_WITHIN: Duration = Duration::from_secs(10);
/// The IO thread may take 10 s to finish (`IoThread::finish`), the watch thread 2 s to stop.
const EXIT_WITHIN: Duration = Duration::from_secs(30);
const HOLD_REASON: &str = "smoke test";

/// A fresh folder under `%TEMP%`, removed with everything in it when the test ends.
struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!("wakeshot-tray-smoke-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("make the temp folder");
        Self(dir)
    }

    /// The dry run's own data folder under the temp one.
    fn dry_run(&self) -> PathBuf {
        self.0.join("dry-run")
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// The tray this test started, found by the pid its `state.json` names and checked by its exe path. Holding the
/// handle keeps the pid from being reused, so the clean-up can never end another process.
struct Tray {
    handle: ProcessHandle,
    quit_sent: bool,
}

impl Tray {
    fn find(state: &StateFile) -> Self {
        let process = ProcessRef {
            pid: state.pid,
            start: state.proc_start,
        };
        let handle = ProcessHandle::open(process)
            .expect("open the tray")
            .expect("the tray state.json names runs");
        let image = image_path(&handle);
        assert!(
            same_file(&image, Path::new(TRAY_EXE)),
            "pid {} runs {}, not this target's tray exe",
            state.pid,
            image.display()
        );
        println!("tray pid {} runs {}", state.pid, image.display());
        Self {
            handle,
            quit_sent: false,
        }
    }

    fn pid(&self) -> u32 {
        self.handle.process().pid
    }

    /// The exit code once the tray ended within `limit`.
    fn exit_within(&self, limit: Duration) -> Option<u32> {
        let ms = u32::try_from(limit.as_millis()).expect("a short limit");
        if !self.handle.wait(ms).expect("wait on the tray") {
            return None;
        }
        let mut code = 0;
        // SAFETY: the handle has QUERY_LIMITED_INFORMATION; `code` is a live local.
        unsafe { GetExitCodeProcess(self.handle.raw(), &mut code) }.expect("read the exit code");
        Some(code)
    }
}

impl Drop for Tray {
    /// A failed assert still leaves no tray: Quit first (it restores and removes the icon), then an end by pid.
    fn drop(&mut self) {
        if self.exit_within(Duration::ZERO).is_some() {
            return;
        }
        if !self.quit_sent {
            let _ = ask_quit(true);
        }
        if self.exit_within(EXIT_WITHIN).is_some() {
            return;
        }
        // SAFETY: plain calls on the pid this test's open handle keeps from reuse; the new handle is closed here.
        unsafe {
            if let Ok(kill) = OpenProcess(PROCESS_TERMINATE, false, self.pid()) {
                let _ = TerminateProcess(kill, 1);
                let _ = CloseHandle(kill);
            }
        }
        eprintln!("tray pid {} did not quit; ended by pid", self.pid());
    }
}

fn image_path(handle: &ProcessHandle) -> PathBuf {
    let mut buffer = vec![0u16; 32_768];
    let mut size = buffer.len() as u32;
    // SAFETY: the handle has QUERY_LIMITED_INFORMATION; `size` holds the buffer's length in units.
    unsafe {
        QueryFullProcessImageNameW(
            handle.raw(),
            PROCESS_NAME_WIN32,
            PWSTR(buffer.as_mut_ptr()),
            &mut size,
        )
    }
    .expect("read the tray's exe path");
    PathBuf::from(OsString::from_wide(&buffer[..size as usize]))
}

fn same_file(a: &Path, b: &Path) -> bool {
    let full = |p: &Path| {
        fs::canonicalize(p)
            .unwrap_or_else(|_| p.to_path_buf())
            .to_string_lossy()
            .to_lowercase()
    };
    full(a) == full(b)
}

fn read_state(dry_run: &Path) -> Option<StateFile> {
    let bytes = fs::read(dry_run.join("state.json")).ok()?;
    match files::read::<StateFile>(&bytes) {
        Read::Value(state) => Some(state),
        Read::Newer(_) | Read::Bad => None,
    }
}

/// Polls `state.json` until `done` holds for it, at most `limit`.
fn wait_for_state(dry_run: &Path, limit: Duration, done: impl Fn(&StateFile) -> bool) -> StateFile {
    let started = Instant::now();
    loop {
        if let Some(state) = read_state(dry_run).filter(|s| done(s)) {
            println!(
                "state after {} ms: holding {:?}",
                started.elapsed().as_millis(),
                state.holding
            );
            return state;
        }
        assert!(
            started.elapsed() < limit,
            "state.json not as expected within {limit:?}: {:?}",
            read_state(dry_run)
        );
        thread::sleep(Duration::from_millis(100));
    }
}

/// `docs/hold-files.md`: written under a temp name, then renamed, so the tray never reads half a file.
fn write_hold(dry_run: &Path, holder: ProcessRef) -> PathBuf {
    let holds = dry_run.join("holds");
    fs::create_dir_all(&holds).expect("make holds");
    let text = format!(
        r#"{{ "version": 1, "pid": {}, "procStart": "{}", "reason": "{HOLD_REASON}" }}"#,
        holder.pid, holder.start.0
    );
    let temp = holds.join("smoke.tmp");
    let hold = holds.join("smoke.json");
    fs::write(&temp, text).expect("write the hold");
    fs::rename(&temp, &hold).expect("rename the hold");
    hold
}

/// Explorer starts the `.cmd`, so the tray runs at Medium as a user's start would (05-tray-ux.md note 4), with the
/// data folder and the Claude folder set for it alone. `start` lets the window close at once while the tray runs on.
fn launch_through_explorer(temp: &TempDir) -> PathBuf {
    let groups = temp.0.join("groups.txt");
    let launch = temp.0.join("launch.cmd");
    let claude = temp.0.join("claude");
    fs::create_dir_all(&claude).expect("make the empty Claude folder");
    let lines = [
        "@echo off".to_owned(),
        format!("set \"{DATA_DIR_ENV}={}\"", temp.0.display()),
        format!("set \"{CLAUDE_DIR_ENV}={}\"", claude.display()),
        format!("whoami /groups > \"{}\"", groups.display()),
        format!("start \"\" \"{TRAY_EXE}\" --dry-run"),
    ];
    let script: String = lines.iter().map(|line| format!("{line}\r\n")).collect();
    fs::write(&launch, script).expect("write the launch .cmd");
    // Explorer hands the file to another instance and its exit code says nothing; the state file is the check.
    let _ = Command::new("explorer.exe").arg(&launch).status();
    groups
}

#[test]
#[ignore = "starts the dry-run tray, whose icon shows on the taskbar: screen slot only"]
fn real_e2e_smoke_dry_run() {
    if !cfg!(debug_assertions) {
        eprintln!(
            "skipped: a release exe ignores {DATA_DIR_ENV} and would use the real data folder"
        );
        return;
    }
    // Only a look: `ask_quit` would close a dry-run tray this test did not start.
    // SAFETY: a static, NUL-terminated class name and no window name.
    let other = unsafe { FindWindowW(LOOP_CLASS, PCWSTR::null()) };
    assert!(
        other.is_err(),
        "a dry-run tray already runs in this session; it would make this test's tray exit at once"
    );
    let temp = TempDir::new();
    let dry_run = temp.dry_run();
    fs::create_dir_all(&dry_run).expect("make the dry-run folder");
    // Not a first run: the first-run balloon could play the notification sound, and this slot plays no sound.
    fs::write(
        dry_run.join("config.json"),
        r#"{ "version": 1, "automatic": true }"#,
    )
    .expect("write config.json");

    let groups = launch_through_explorer(&temp);
    let state = wait_for_state(&dry_run, START_WITHIN, |_| true);
    let mut tray = Tray::find(&state);
    let groups = fs::read_to_string(&groups).expect("the .cmd wrote its groups");
    assert!(
        groups.contains(MEDIUM_SID),
        "the .cmd did not run at Medium:\n{groups}"
    );
    assert!(
        state.holding.is_empty(),
        "holding at start: {:?}",
        state.holding
    );

    let hold = write_hold(&dry_run, process::current().expect("this test's process"));
    let held = wait_for_state(&dry_run, FOLLOW_WITHIN, |s| !s.holding.is_empty());
    assert_eq!(held.pid, tray.pid(), "another tray wrote state.json");
    println!(
        "journal during the hold: {}",
        dry_run.join("journal.json").exists()
    );

    fs::remove_file(&hold).expect("delete the hold");
    wait_for_state(&dry_run, FOLLOW_WITHIN, |s| s.holding.is_empty());

    assert!(
        ask_quit(true).expect("post WM_CLOSE"),
        "no dry-run loop window found"
    );
    tray.quit_sent = true;
    let code = tray.exit_within(EXIT_WITHIN);
    assert_eq!(code, Some(0), "the tray's exit code (None: still running)");
    assert!(
        !dry_run.join("journal.json").exists(),
        "the dry-run journal is left after Quit"
    );
}
