//! Process tests of the real exes: the tray starts at Medium through `explorer.exe <launch .cmd>`, as a user's start would, on a temp data folder (`WAKESHOT_DATA_DIR`, read by debug
//! builds only), and `wakeshot.exe` runs next to it with the same folder. A dry run (`TempData::new`,
//! `TempData::with_config`) starts both exes with `--dry-run`, so they use `<temp>\dry-run` and no power setting is
//! written; a real run (`TempData::real`) starts them without it, so they use `<temp>` itself and write real power
//! settings, which `e2e_real.rs` allows only on its own plan copies.
//!
//! The CLI runs as this test's own child, so its output and exit code come back directly. In an agent shell that is
//! High, while a user's terminal is Medium; the tray stays Medium either way, so files the CLI writes are read and
//! deleted across the two levels. Change it when a check needs a Medium CLI: then run it through
//! `process::spawn_outside_tree` under `cmd /c` with its output sent to files.

// Each suite uses only part of these helpers, and each compiles this module on its own.
#![allow(dead_code)]

use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};
use std::{fs, thread};

use wakeshot_core::files::{self, JournalFile, Read, StateFile};
use wakeshot_core::ownership::io::{JournalRead, SafetyNet};
use wakeshot_core::process::{Lookup, ProcessRef};
use wakeshot_windows::app::window::ask_quit;
use wakeshot_windows::power::journal::FileNet;
use wakeshot_windows::process::{Liveness, ProcessHandle, SystemLiveness};
use wakeshot_windows::store::DataDir;
use wakeshot_windows::testing::MemoryLog;
use windows::Win32::Foundation::{CloseHandle, HWND};
use windows::Win32::System::Threading::{
    GetExitCodeProcess, OpenProcess, PROCESS_NAME_WIN32, PROCESS_TERMINATE,
    QueryFullProcessImageNameW, TerminateProcess,
};
use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, GetWindowThreadProcessId};
use windows::core::{PCWSTR, PWSTR, w};

pub const TRAY_EXE: &str = env!("CARGO_BIN_EXE_wakeshot-tray");
const TRAY_NAME: &str = "wakeshot-tray.exe";
const CLI_NAME: &str = "wakeshot.exe";
/// Read by debug builds of both exes only (`store::DataDir::real`); a dry run uses `<it>\dry-run`.
const DATA_DIR_ENV: &str = "WAKESHOT_DATA_DIR";
/// Claude Code's own folder variable, which the tray follows.
const CLAUDE_DIR_ENV: &str = "CLAUDE_CONFIG_DIR";
/// The loop window's classes (`app::window`), which `ask_quit` looks for.
const LOOP_CLASS: PCWSTR = w!("Wakeshot.Loop");
const DRY_RUN_LOOP_CLASS: PCWSTR = w!("Wakeshot.Loop.DryRun");
/// `whoami /groups` names the Medium level by this SID.
const MEDIUM_SID: &str = "S-1-16-8192";
/// A real run's temp folder starts with this, so a clean-up finds the ones a cut-off run left.
pub const REAL_PREFIX: &str = "wakeshot-e2e-real-";

/// The limit for the first `state.json`, counted from the launch.
pub const START_WITHIN: Duration = Duration::from_secs(10);
/// For the state to follow a change: the watch thread sees a file at once, and `state.json` is written at most once a
/// second (`core/view.rs`).
pub const FOLLOW_WITHIN: Duration = Duration::from_secs(10);
/// The tray's safety tick; a Gone hold drops by the next one at the latest.
pub const SAFETY_TICK: Duration = Duration::from_secs(30);
/// The IO thread may take 10 s to finish (`IoThread::finish`), the watch thread 2 s to stop.
pub const EXIT_WITHIN: Duration = Duration::from_secs(30);
/// A second start through Explorer: the shell and `cmd` take about a second before the tray even runs.
pub const SECOND_START_WITHIN: Duration = Duration::from_secs(10);

/// `true` when this test must not run: a release exe ignores `WAKESHOT_DATA_DIR` and would use the real data folder.
/// Also checks that `wakeshot.exe` was built next to the tray exe.
pub fn skip() -> bool {
    if !cfg!(debug_assertions) {
        eprintln!(
            "skipped: a release exe ignores {DATA_DIR_ENV} and would use the real data folder"
        );
        return true;
    }
    assert!(
        cli_exe().exists(),
        "{} is missing: build it first with `cargo build --locked -p wakeshot-cli --target <t>` into the same \
         target folder",
        cli_exe().display()
    );
    false
}

fn cli_exe() -> PathBuf {
    Path::new(TRAY_EXE).with_file_name(CLI_NAME)
}

/// A fresh data folder under `%TEMP%`, removed with everything in it when the test ends. A dry run keeps its data in
/// the `dry-run` subfolder, a real run in the folder itself; the launch files sit in the folder.
pub struct TempData {
    root: PathBuf,
    dry_run: bool,
}

impl TempData {
    /// Dry run, empty: a tray started on it makes its first run.
    pub fn new(tag: &str) -> Self {
        Self::make(tag, true)
    }

    /// Dry run with `config.json`, so the start is not a first run: no first-run balloon for tests that do not check
    /// it.
    pub fn with_config(tag: &str) -> Self {
        let data = Self::new(tag);
        data.write_config();
        data
    }

    /// Real, with `config.json`. The exes write real power settings and the real RunOnce value from it, so only a
    /// test that guards the user's plan may use it (`e2e_real.rs`).
    pub fn real(tag: &str) -> Self {
        let data = Self::make(&format!("real-{tag}"), false);
        data.write_config();
        data
    }

    fn make(tag: &str, dry_run: bool) -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let root =
            std::env::temp_dir().join(format!("wakeshot-e2e-{tag}-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("make the temp folder");
        Self { root, dry_run }
    }

    fn write_config(&self) {
        fs::create_dir_all(self.data_dir()).expect("make the data folder");
        fs::write(
            self.file("config.json"),
            r#"{ "version": 1, "automatic": true }"#,
        )
        .expect("write config.json");
    }

    /// The folder `WAKESHOT_DATA_DIR` names, where the launch files also go.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Where the exes keep their files: `<root>\dry-run` in a dry run, `<root>` in a real one.
    pub fn data_dir(&self) -> PathBuf {
        if self.dry_run {
            self.root.join("dry-run")
        } else {
            self.root.clone()
        }
    }

    /// A file in the data folder, such as `journal.json`.
    pub fn file(&self, name: &str) -> PathBuf {
        self.data_dir().join(name)
    }

    pub fn holds(&self) -> PathBuf {
        self.data_dir().join("holds")
    }

    pub fn keep_awake(&self) -> PathBuf {
        self.holds().join("keep-awake.json")
    }

    /// The names of the hold files, `.json` only, as the tray reads them.
    pub fn hold_names(&self) -> Vec<String> {
        let Ok(entries) = fs::read_dir(self.holds()) else {
            return Vec::new();
        };
        let mut names: Vec<String> = entries
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|name| name.to_ascii_lowercase().ends_with(".json"))
            .collect();
        names.sort();
        names
    }

    pub fn state(&self) -> Option<StateFile> {
        let bytes = fs::read(self.file("state.json")).ok()?;
        match files::read::<StateFile>(&bytes) {
            Read::Value(state) => Some(state),
            Read::Newer(_) | Read::Bad => None,
        }
    }

    /// Polls `state.json` until `done` holds for it, at most `limit`.
    pub fn wait_state(
        &self,
        limit: Duration,
        what: &str,
        done: impl Fn(&StateFile) -> bool,
    ) -> StateFile {
        let started = Instant::now();
        let state = wait_until(limit, what, || self.state().filter(|s| done(s)));
        println!(
            "state {what} after {} ms: automatic {}, holding {:?}, overridden {}",
            started.elapsed().as_millis(),
            state.automatic,
            state.holding,
            state.overridden.len()
        );
        state
    }

    /// `journal.json` through the reader every process uses. A dry-run net is enough for a read and can never touch
    /// the RunOnce value.
    pub fn journal(&self) -> Result<Option<JournalFile>, String> {
        let mut net =
            FileNet::dry_run(DataDir::at(self.data_dir()), Arc::new(MemoryLog::default()));
        match net.load_journal() {
            JournalRead::Missing => Ok(None),
            JournalRead::Read(file) => Ok(Some(file)),
            JournalRead::Corrupt => Err("journal.json does not parse".to_owned()),
            JournalRead::Failed(f) => Err(format!("journal.json not read: {f:?}")),
        }
    }

    fn log_path(&self) -> PathBuf {
        self.data_dir().join("logs").join("wakeshot.log")
    }

    /// The whole log, every process's lines; empty while there is none.
    pub fn log(&self) -> String {
        fs::read_to_string(self.log_path()).unwrap_or_default()
    }

    /// Where the log ends now, so a check can look only at the lines that come after.
    pub fn log_mark(&self) -> LogMark {
        LogMark(self.log().lines().count())
    }

    pub fn log_since(&self, mark: LogMark) -> Vec<String> {
        self.log().lines().skip(mark.0).map(str::to_owned).collect()
    }

    /// Waits for a line after `mark` that holds `part`, and gives it.
    pub fn wait_log(&self, mark: LogMark, part: &str, limit: Duration) -> String {
        wait_until(limit, &format!("a log line with {part:?}"), || {
            self.log_since(mark).into_iter().find(|l| l.contains(part))
        })
    }

    /// Runs `wakeshot.exe <args>` on this folder (with `--dry-run` first in a dry run) and waits for it. `args` is the
    /// raw command line text, as a user types it.
    pub fn cli(&self, args: &str) -> Said {
        let output = self
            .cli_command(args)
            .stdin(Stdio::null())
            .output()
            .expect("run wakeshot.exe");
        let said = Said {
            code: output.status.code().expect("Windows always gives a code"),
            out: String::from_utf8_lossy(&output.stdout).into_owned(),
            err: String::from_utf8_lossy(&output.stderr).into_owned(),
        };
        println!("wakeshot {args} -> {said:?}");
        said
    }

    /// Starts `wakeshot.exe <args>` as `cli` does and leaves it running; its output is collected for
    /// `wait_with_output`.
    pub fn cli_spawn(&self, args: &str) -> Child {
        let child = self
            .cli_command(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("start wakeshot.exe");
        println!("wakeshot {args} started, pid {}", child.id());
        child
    }

    fn cli_command(&self, args: &str) -> Command {
        let mut command = Command::new(cli_exe());
        let line = if self.dry_run {
            format!("--dry-run {args}")
        } else {
            args.to_owned()
        };
        command.raw_arg(line).env(DATA_DIR_ENV, &self.root);
        command
    }

    /// The tray's command line as a `.cmd` line: the quoted exe, then `--dry-run` in a dry run.
    fn tray_line(&self) -> String {
        if self.dry_run {
            format!("\"{TRAY_EXE}\" --dry-run")
        } else {
            format!("\"{TRAY_EXE}\"")
        }
    }

    /// Starts the tray through Explorer and waits for its exit, for a start that should end at once. Gives its exit
    /// code and how long the launch took, Explorer and `cmd` included.
    pub fn start_expecting_exit(&self) -> (u32, Duration) {
        let code_file = self.root.join("second-start-code.txt");
        let _ = fs::remove_file(&code_file);
        let temp = self.root.join("second-start-code.tmp");
        // A batch file waits for a GUI exe it runs (unlike the prompt), so the next line sees its exit code. Written
        // under a temp name and moved, so the test never reads half a number.
        let tail = [
            self.tray_line(),
            format!("> \"{}\" echo %ERRORLEVEL%", temp.display()),
            format!(
                "move /y \"{}\" \"{}\" > nul",
                temp.display(),
                code_file.display()
            ),
        ];
        let launched = Instant::now();
        let groups = self.launch(Claude::Empty, &tail, "second-start");
        let code = wait_until(SECOND_START_WITHIN, "the second start's exit code", || {
            let text = fs::read_to_string(&code_file).ok()?;
            text.trim().parse::<u32>().ok()
        });
        let took = launched.elapsed();
        assert_medium(&groups);
        println!(
            "second start exited with {code} after {} ms",
            took.as_millis()
        );
        (code, took)
    }

    /// Writes `<name>.cmd` and hands it to Explorer, so it runs at Medium as a user's start would, with the data folder
    /// and the Claude folder set for it alone, then the `tail` lines. Gives the file `whoami /groups` writes.
    pub fn launch(&self, claude: Claude, tail: &[String], name: &str) -> PathBuf {
        let groups = self.root.join(format!("{name}-groups.txt"));
        let launch = self.root.join(format!("{name}.cmd"));
        let claude_line = match claude {
            Claude::Empty => {
                let empty = self.root.join("claude");
                fs::create_dir_all(&empty).expect("make the empty Claude folder");
                format!("set \"{CLAUDE_DIR_ENV}={}\"", empty.display())
            }
            // Explorer's own value, if it has one, would move the folder; cleared, the tray reads `~\.claude`.
            Claude::Home => format!("set \"{CLAUDE_DIR_ENV}=\""),
        };
        let mut lines = vec![
            "@echo off".to_owned(),
            format!("set \"{DATA_DIR_ENV}={}\"", self.root.display()),
            claude_line,
            format!("whoami /groups > \"{}\"", groups.display()),
        ];
        lines.extend(tail.iter().cloned());
        let script: String = lines.iter().map(|line| format!("{line}\r\n")).collect();
        fs::write(&launch, script).expect("write the launch .cmd");
        // Explorer hands the file to the running shell and its exit code says nothing; the caller's wait is the check.
        let _ = Command::new("explorer.exe").arg(&launch).status();
        groups
    }
}

impl Drop for TempData {
    /// The tray's log goes with the folder, so it is printed first: cargo shows it when the test fails, and
    /// `--nocapture` shows it on a pass. Every `Tray` of the test is made later, so it has ended by now.
    fn drop(&mut self) {
        let log = self.log();
        if !log.is_empty() {
            println!("--- the log ---\n{log}--- end of the log ---");
        }
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// A line count in the log.
#[derive(Clone, Copy, Debug)]
pub struct LogMark(usize);

impl LogMark {
    /// The whole log, from its first line.
    pub const START: Self = Self(0);
}

/// What `wakeshot.exe` printed and its exit code.
#[derive(Debug)]
pub struct Said {
    pub code: i32,
    pub out: String,
    pub err: String,
}

/// The Claude folder the tray watches.
#[derive(Clone, Copy, Debug)]
pub enum Claude {
    /// An empty folder, so the agent sessions on this PC (this test runs inside one) never hold the tray.
    Empty,
    /// The user's own `~\.claude`: every Claude Code session on this PC counts.
    Home,
}

/// A tray this test started, found by its exe path in this target folder. Holding the handle keeps the pid from
/// being reused, so the clean-up can never end another process.
pub struct Tray {
    handle: ProcessHandle,
    quit_sent: bool,
    dry_run: bool,
}

impl Tray {
    /// Starts a tray on `data`, dry run or real as `data` is, and waits for its first `state.json`.
    pub fn start(data: &TempData, claude: Claude) -> Self {
        assert_no_tray(data.dry_run);
        let before: Vec<ProcessRef> = trays_of_this_build()
            .iter()
            .map(ProcessHandle::process)
            .collect();
        let launched = Instant::now();
        let tail = [format!("start \"\" {}", data.tray_line())];
        let groups = data.launch(claude, &tail, "launch");
        let handle = wait_until(START_WITHIN, "a new tray from this target folder", || {
            trays_of_this_build()
                .into_iter()
                .find(|h| !before.contains(&h.process()))
        });
        let tray = Self {
            handle,
            quit_sent: false,
            dry_run: data.dry_run,
        };
        let pid = tray.pid();
        let left = START_WITHIN.saturating_sub(launched.elapsed());
        let state = wait_until(left, "the new tray's first state.json", || {
            if let Some(code) = tray.exit_within(Duration::ZERO) {
                panic!("tray pid {pid} ended with {code} before its first state.json");
            }
            data.state().filter(|s| s.pid == pid)
        });
        assert_eq!(
            state.proc_start,
            tray.handle.process().start,
            "state.json names another start"
        );
        assert_medium(&groups);
        println!(
            "tray pid {pid} started; state.json after {} ms, holding {:?}",
            launched.elapsed().as_millis(),
            state.holding
        );
        tray
    }

    pub fn pid(&self) -> u32 {
        self.handle.process().pid
    }

    /// Its pid and start time, as the journal's owner names it.
    pub fn process(&self) -> ProcessRef {
        self.handle.process()
    }

    /// The exit code once the tray ended within `limit`.
    pub fn exit_within(&self, limit: Duration) -> Option<u32> {
        exit_code_within(&self.handle, limit)
    }

    /// Quit as the menu does, through `ask_quit`, once the loop window is checked to be this tray's. Gives the exit
    /// code.
    pub fn quit(&mut self) -> u32 {
        assert_eq!(
            loop_window_pid(self.dry_run),
            Some(self.pid()),
            "the loop window is not this tray's"
        );
        assert!(
            ask_quit(self.dry_run).expect("post WM_CLOSE"),
            "no loop window found"
        );
        self.quit_sent = true;
        let code = self.exit_within(EXIT_WITHIN);
        println!("tray pid {} quit with {code:?}", self.pid());
        code.unwrap_or_else(|| {
            panic!(
                "tray pid {} still runs {EXIT_WITHIN:?} after the quit",
                self.pid()
            )
        })
    }

    /// Ends the tray by its pid, as a crash, Task Manager or the end of a session would: no restore, no clean-up.
    pub fn kill(&self) {
        terminate(&self.handle);
        let code = self.exit_within(Duration::from_secs(5));
        assert_eq!(code, Some(1), "tray pid {} not ended by pid", self.pid());
        println!("tray pid {} ended by pid", self.pid());
    }
}

impl Drop for Tray {
    /// A failed assert still leaves no tray: Quit first (it restores and removes the icon), then an end by pid.
    fn drop(&mut self) {
        if self.exit_within(Duration::ZERO).is_some() {
            return;
        }
        if !self.quit_sent && loop_window_pid(self.dry_run) == Some(self.pid()) {
            let _ = ask_quit(self.dry_run);
        }
        if self.exit_within(EXIT_WITHIN).is_some() {
            return;
        }
        terminate(&self.handle);
        eprintln!("tray pid {} did not quit; ended by pid", self.pid());
    }
}

/// A process this test started indirectly (a `wakeshot run` child), opened by pid and checked by its exe name, so
/// only that process can ever be ended.
pub struct Started {
    handle: ProcessHandle,
}

impl Started {
    pub fn open(pid: u32, exe_name: &str) -> Self {
        let Lookup::Running(start) = SystemLiveness.start_of_pid(pid) else {
            panic!("pid {pid} does not run");
        };
        let handle = ProcessHandle::open(ProcessRef { pid, start })
            .expect("open the process")
            .unwrap_or_else(|| panic!("pid {pid} ended before it was opened"));
        let image = image_path(&handle).expect("read the exe path");
        let name = image.file_name().map(|n| n.to_string_lossy().into_owned());
        assert!(
            name.as_deref()
                .is_some_and(|n| n.eq_ignore_ascii_case(exe_name)),
            "pid {pid} runs {}, not {exe_name}",
            image.display()
        );
        Self { handle }
    }

    pub fn running(&self) -> bool {
        matches!(self.handle.wait(0), Ok(false))
    }

    /// Ends it by its pid and waits for the end.
    pub fn end(&self) {
        terminate(&self.handle);
        assert!(
            matches!(self.handle.wait(5_000), Ok(true)),
            "pid {} did not end",
            self.handle.process().pid
        );
    }
}

impl Drop for Started {
    fn drop(&mut self) {
        if self.running() {
            terminate(&self.handle);
        }
    }
}

/// `TerminateProcess` on the pid the open handle keeps from reuse; the new handle is closed here.
pub fn terminate(handle: &ProcessHandle) {
    let pid = handle.process().pid;
    // SAFETY: plain calls on a pid this test's open handle keeps from reuse; the new handle is closed once.
    unsafe {
        if let Ok(kill) = OpenProcess(PROCESS_TERMINATE, false, pid) {
            let _ = TerminateProcess(kill, 1);
            let _ = CloseHandle(kill);
        }
    }
}

/// The exit code once the process ended within `limit`.
pub fn exit_code_within(handle: &ProcessHandle, limit: Duration) -> Option<u32> {
    let ms = u32::try_from(limit.as_millis()).unwrap_or(u32::MAX);
    if !matches!(handle.wait(ms), Ok(true)) {
        return None;
    }
    let mut code = 0;
    // SAFETY: the handle has QUERY_LIMITED_INFORMATION; `code` is a live local.
    unsafe { GetExitCodeProcess(handle.raw(), &mut code) }.ok()?;
    Some(code)
}

/// Every running `wakeshot-tray.exe` whose full path is this target folder's exe: trays and their guards.
pub fn trays_of_this_build() -> Vec<ProcessHandle> {
    let list = SystemLiveness.list().expect("the process list");
    list.into_iter()
        .filter(|e| e.image.eq_ignore_ascii_case(TRAY_NAME))
        .filter_map(|e| {
            ProcessHandle::open(ProcessRef {
                pid: e.pid,
                start: e.start,
            })
            .ok()
            .flatten()
        })
        .filter(|h| image_path(h).is_some_and(|p| full_path(&p) == full_path(Path::new(TRAY_EXE))))
        .collect()
}

fn image_path(handle: &ProcessHandle) -> Option<PathBuf> {
    let mut buffer = vec![0u16; 32_768];
    let mut size = u32::try_from(buffer.len()).ok()?;
    // SAFETY: the handle has QUERY_LIMITED_INFORMATION; `size` holds the buffer's length in units.
    unsafe {
        QueryFullProcessImageNameW(
            handle.raw(),
            PROCESS_NAME_WIN32,
            PWSTR(buffer.as_mut_ptr()),
            &mut size,
        )
    }
    .ok()?;
    Some(PathBuf::from(OsString::from_wide(&buffer[..size as usize])))
}

/// A path in one form for comparing: the full path, lowercase, without the `\\?\` that `canonicalize` adds.
pub fn full_path(path: &Path) -> String {
    let full = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let text = full.to_string_lossy().to_lowercase();
    text.strip_prefix(r"\\?\").unwrap_or(&text).to_owned()
}

/// This session's loop window of that kind and its pid, if one exists.
pub fn loop_window(dry_run: bool) -> Option<(HWND, u32)> {
    let class = if dry_run {
        DRY_RUN_LOOP_CLASS
    } else {
        LOOP_CLASS
    };
    // SAFETY: a static, NUL-terminated class name and no window name.
    let hwnd = unsafe { FindWindowW(class, PCWSTR::null()) }.ok()?;
    let mut pid = 0u32;
    // SAFETY: `pid` is a live local the call writes once.
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    (pid != 0).then_some((hwnd, pid))
}

fn loop_window_pid(dry_run: bool) -> Option<u32> {
    loop_window(dry_run).map(|(_, pid)| pid)
}

/// Only a look: another tray of the same kind in this session (another smoke test, or an installed Wakeshot)
/// would make this test's tray exit at once, and `ask_quit` would close it.
fn assert_no_tray(dry_run: bool) {
    if let Some(pid) = loop_window_pid(dry_run) {
        let kind = if dry_run { "dry-run" } else { "real" };
        panic!(
            "a {kind} tray (pid {pid}) already runs in this session; wait for it to end and run again"
        );
    }
}

/// The `.cmd` that wrote `groups` ran at Medium.
pub fn assert_medium(groups: &Path) {
    let text = wait_until(FOLLOW_WITHIN, "the .cmd's whoami output", || {
        fs::read(groups)
            .ok()
            .map(|b| String::from_utf8_lossy(&b).into_owned())
            .filter(|t| t.contains("S-1-16-"))
    });
    assert!(
        text.contains(MEDIUM_SID),
        "the .cmd did not run at Medium:\n{text}"
    );
}

/// Polls `look` every 50 ms until it gives a value, at most `limit`.
pub fn wait_until<T>(limit: Duration, what: &str, mut look: impl FnMut() -> Option<T>) -> T {
    let started = Instant::now();
    loop {
        if let Some(found) = look() {
            return found;
        }
        assert!(started.elapsed() < limit, "not within {limit:?}: {what}");
        thread::sleep(Duration::from_millis(50));
    }
}

/// Polls `look` every 200 ms until it gives `Ok`, at most `limit`; a timeout names the last thing seen, so a failed
/// wait says what it found instead.
pub fn wait_for<T>(limit: Duration, what: &str, look: impl FnMut() -> Result<T, String>) -> T {
    match wait_for_result(limit, look) {
        Ok(found) => found,
        Err(last) => panic!("not within {limit:?}: {what}; last seen: {last}"),
    }
}

/// `wait_for` that gives the last `Err` back instead of failing.
pub fn wait_for_result<T>(
    limit: Duration,
    mut look: impl FnMut() -> Result<T, String>,
) -> Result<T, String> {
    let started = Instant::now();
    loop {
        match look() {
            Ok(found) => return Ok(found),
            Err(last) if started.elapsed() >= limit => return Err(last),
            Err(_) => thread::sleep(Duration::from_millis(200)),
        }
    }
}

/// One log line split into the fields of architecture section 8: local wall time with its UTC offset, monotonic
/// seconds, process, level, area, text.
#[derive(Debug)]
pub struct LogLine<'a> {
    pub process: &'a str,
    pub level: &'a str,
    pub area: &'a str,
    pub text: &'a str,
}

const PROCESSES: [&str; 5] = ["tray", "guard", "restore", "cli", "hook"];
const LEVELS: [&str; 3] = ["INFO", "WARN", "ERROR"];
/// `0` stands for a digit, `+` for either sign; anything else is itself.
const STAMP: &str = "0000-00-00T00:00:00.000+00:00";

/// `2026-11-04T04:01:02.345+04:00 [1234.567] tray    INFO  core: text`, as `log.rs` writes it.
pub fn parse_line(line: &str) -> Result<LogLine<'_>, String> {
    let (stamp, rest) = line.split_once(' ').ok_or("no blank after the time")?;
    let stamp_ok = stamp.len() == STAMP.len()
        && stamp.chars().zip(STAMP.chars()).all(|(c, t)| match t {
            '0' => c.is_ascii_digit(),
            '+' => c == '+' || c == '-',
            t => c == t,
        });
    if !stamp_ok {
        return Err(format!(
            "the time {stamp:?} is not local time with its offset"
        ));
    }
    let rest = rest.strip_prefix('[').ok_or("no monotonic seconds")?;
    let (mono, rest) = rest
        .split_once("] ")
        .ok_or("the monotonic seconds do not close")?;
    let (seconds, millis) = mono
        .split_once('.')
        .ok_or("no fraction in the monotonic seconds")?;
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    if !digits(seconds) || !digits(millis) || millis.len() != 3 {
        return Err(format!("the monotonic seconds {mono:?} are not seconds"));
    }
    let process = rest.get(..7).map(str::trim_end).ok_or("no process")?;
    let level = rest.get(8..13).map(str::trim_end).ok_or("no level")?;
    if !PROCESSES.contains(&process) || rest.get(7..8) != Some(" ") {
        return Err(format!(
            "the process {process:?} is not one of {PROCESSES:?}"
        ));
    }
    if !LEVELS.contains(&level) || rest.get(13..14) != Some(" ") {
        return Err(format!("the level {level:?} is not one of {LEVELS:?}"));
    }
    let (area, text) = rest
        .get(14..)
        .and_then(|r| r.split_once(": "))
        .ok_or("no area")?;
    if area.is_empty() || area.contains(' ') {
        return Err(format!("the area {area:?} is not one word"));
    }
    if text.trim().is_empty() {
        return Err("no text".to_owned());
    }
    Ok(LogLine {
        process,
        level,
        area,
        text,
    })
}

/// Text that is not plain English: a Rust debug form, an escaped path or quote, or the content of one of Wakeshot's
/// JSON files. `None` when the text reads as words.
pub fn not_plain(text: &str) -> Option<&'static str> {
    const DEBUG: [&str; 5] = ["Some(", "Ok(", "Err(", "::", "\\\""];
    const FILE_CONTENT: [&str; 4] = ["{\"", "\"version\"", "\"pid\"", "\"procStart\""];
    if DEBUG.iter().any(|d| text.contains(d)) {
        return Some("a Rust debug form");
    }
    if text.contains("\\\\") {
        return Some("an escaped path");
    }
    if FILE_CONTENT.iter().any(|f| text.contains(f)) {
        return Some("file content");
    }
    if text.chars().any(char::is_control) {
        return Some("a control character");
    }
    let has_word = text
        .split(|c: char| !c.is_ascii_alphabetic())
        .any(|word| word.len() >= 3);
    if !has_word {
        return Some("no word");
    }
    None
}
