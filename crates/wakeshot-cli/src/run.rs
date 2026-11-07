//! `wakeshot run -- <command>`: hold the PC awake while a command runs (decision 10; architecture section 4, "CLI").

use std::ffi::OsStr;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, ExitCode};
use std::sync::Arc;

use wakeshot_core::files::{self, Config, HoldFile, Read};
use wakeshot_core::holds::file::render;
use wakeshot_windows::WinError;
use wakeshot_windows::child::{self, Resolved};
use wakeshot_windows::log::{Level, Log};
use wakeshot_windows::process::{self, Liveness, SystemLiveness};
use wakeshot_windows::store::{self, DataDir, DataFile};

use crate::args;
use crate::command::Global;
use crate::status::{self, AREA, Tray};

/// The wrapper's own exit codes, as GNU `env` and `timeout` use them, so a script can tell them from the child's.
const FAILED: i32 = 125;
const CANNOT_START: i32 = 126;
const NOT_FOUND: i32 = 127;

const NOT_RUNNING: &str =
    "Wakeshot is not running, so nothing keeps the PC awake for this command until it starts.";
const AUTOMATIC_OFF: &str = "Automatic is off, so this command does not keep the PC awake. Turn it on with \
                             'wakeshot automatic on'.";

pub fn main(global: &Global) -> ExitCode {
    // From the raw command line, not `args_os`, so the child gets its text exactly as typed.
    let Some((program, rest)) =
        args::run_tail(&child::command_line()).and_then(|tail| args::split_program(&tail))
    else {
        return args::bad("cannot find the command after 'run --'");
    };
    let mut stderr = std::io::stderr();
    let (dir, log): (Option<DataDir>, Arc<dyn Log>) = match status::open(global) {
        Ok((dir, log)) => (Some(dir), log),
        Err(answer) => {
            say(
                &mut stderr,
                &format!("{} The command runs anyway.", answer.text),
            );
            (None, Arc::new(NoLog))
        }
    };
    let parts = Parts {
        liveness: &SystemLiveness,
        log: &*log,
        ignore_ctrl_c: child::ignore_ctrl_c,
    };
    let code = run(dir.as_ref(), &parts, &mut stderr, &program, &rest);
    // `ExitCode` holds 0 to 255 only; a code such as 0xC000013A (ended by Ctrl+C) must pass through whole.
    std::process::exit(code)
}

pub(crate) struct Parts<'a> {
    pub liveness: &'a dyn Liveness,
    pub log: &'a dyn Log,
    /// `child::ignore_ctrl_c`; tests pass one that leaves the test process as it is.
    pub ignore_ctrl_c: fn() -> Result<(), WinError>,
}

/// Runs the child and gives the code to exit with. `warn` gets the lines for the user. No `dir`: no hold, no warning.
pub(crate) fn run(
    dir: Option<&DataDir>,
    parts: &Parts,
    warn: &mut dyn Write,
    program: &OsStr,
    rest: &OsStr,
) -> i32 {
    let log = parts.log;
    let resolved = child::resolve(program);
    let path = match &resolved {
        Resolved::Exe(path) | Resolved::Batch(path) => path,
        Resolved::NotFound => {
            let text = format!("run: {} not found", name_of(Path::new(program)));
            log.line(Level::Warn, AREA, &text);
            say(warn, &not_found(program));
            return NOT_FOUND;
        }
        Resolved::NotRunnable(extension) => {
            let name = name_of(Path::new(program));
            let text = format!("run: {name} not started, a .{extension} file");
            log.line(Level::Warn, AREA, &text);
            say(warn, &not_runnable(program, extension));
            return CANNOT_START;
        }
    };
    let name = name_of(path);
    if let Some(warning) = dir.and_then(|dir| warning(dir, parts.liveness)) {
        say(warn, warning);
    }
    let mut child = match child::start(&resolved, rest) {
        Ok(child) => child,
        Err(e) => {
            log.line(Level::Warn, AREA, &format!("run: {name} not started: {e}"));
            say(warn, &format!("could not start '{}': {e}", shown(program)));
            return CANNOT_START;
        }
    };
    // Only now: Ctrl+C before the child exists still ends this process, as the user meant.
    if let Err(e) = (parts.ignore_ctrl_c)() {
        let text = format!("run: Ctrl+C handler not set, {}", status::error_text(e));
        log.line(Level::Warn, AREA, &text);
    }
    log.line(
        Level::Info,
        AREA,
        &format!("run: {name} started, pid {}", child.id()),
    );
    let hold = dir.and_then(|dir| write_hold(dir, &child, &name, log, warn));
    let status = match child.wait() {
        Ok(status) => status,
        Err(e) => {
            // The hold file stays: the child may still run, and the tray deletes the file once it is gone.
            log.line(Level::Error, AREA, &format!("run: lost {name}: {e}"));
            say(warn, &format!("lost track of '{}': {e}", shown(program)));
            return FAILED;
        }
    };
    if let Some(hold) = hold
        && let Err(e) = store::remove(&hold)
    {
        // Harmless: the tray deletes a hold file whose process is gone.
        let text = format!("run: hold file not deleted, {}", status::error_text(e));
        log.line(Level::Warn, AREA, &text);
    }
    // Windows always gives a code; `None` is for a Unix signal.
    let code = status.code().unwrap_or(FAILED);
    let text = format!("run: {name} exited with {}", code_text(code));
    log.line(Level::Info, AREA, &text);
    code
}

/// `run` always runs; it only says when nothing will hold the PC awake for it (`product.md`, UX).
fn warning(dir: &DataDir, liveness: &dyn Liveness) -> Option<&'static str> {
    match status::tray(dir, liveness) {
        Tray::NotRunning | Tray::Unreadable(_) => Some(NOT_RUNNING),
        Tray::Running(_) if automatic_off(dir) => Some(AUTOMATIC_OFF),
        // Cannot tell, or a newer tray, which reads version 1 hold files too: nothing to say for sure.
        Tray::Running(_) | Tray::Unknown | Tray::Newer(_) => None,
    }
}

/// Only a `config.json` that says off: a missing one means on, and a bad or newer one is the tray's to judge.
fn automatic_off(dir: &DataDir) -> bool {
    match store::read_whole(&dir.file(DataFile::Config)) {
        Ok(Some(bytes)) => matches!(files::read::<Config>(&bytes), Read::Value(c) if !c.automatic),
        _ => false,
    }
}

/// Writes `holds\run-<child pid>.json`; `None` when it could not (the user is told, and the command runs on).
fn write_hold(
    dir: &DataDir,
    child: &Child,
    name: &str,
    log: &dyn Log,
    warn: &mut dyn Write,
) -> Option<PathBuf> {
    let pid = child.id();
    // Read while this process holds the child's handle, so the pid cannot be reused yet. Without it the hold still
    // counts: the child started before the file was written (architecture section 6).
    let start = match process::start_of(child) {
        Ok(start) => Some(start),
        Err(e) => {
            let text = format!(
                "run: start of pid {pid} not read, {}",
                status::error_text(e)
            );
            log.line(Level::Warn, AREA, &text);
            None
        }
    };
    // Only the program's name: the arguments may hold secrets (architecture section 6).
    let file = HoldFile {
        reason: Some(name.to_owned()),
        ..HoldFile::new(pid, start)
    };
    let path = dir.holds().join(format!("run-{pid}.json"));
    match store::write_atomic(&path, render(&file).as_bytes()) {
        Ok(()) => Some(path),
        Err(e) => {
            let text = format!("run: hold file not written, {}", status::error_text(e));
            log.line(Level::Warn, AREA, &text);
            say(
                warn,
                &format!(
                    "could not write the hold file ({}), so this command does not keep the PC awake.",
                    status::error_text(e)
                ),
            );
            None
        }
    }
}

fn not_found(program: &OsStr) -> String {
    let typed = shown(program);
    match Path::new(program).file_name() == Some(program) {
        true => format!(
            "'{typed}' was not found in PATH. For a program in this folder, write '.\\{typed}'."
        ),
        false => format!("'{typed}' was not found."),
    }
}

fn not_runnable(program: &OsStr, extension: &str) -> String {
    let typed = shown(program);
    match extension.eq_ignore_ascii_case("ps1") {
        true => {
            let quoted = match typed.contains([' ', '\t']) {
                true => format!("\"{typed}\""),
                false => typed.clone(),
            };
            format!(
                "'{typed}' is a PowerShell script; run it as 'wakeshot run -- pwsh -File {quoted}'."
            )
        }
        false => format!(
            "Wakeshot starts .exe, .com, .bat and .cmd files, not '{typed}'. Run the program that opens it, such \
             as 'wakeshot run -- pwsh -File script.ps1'."
        ),
    }
}

/// The program's file name without extension: what the hold file and the log name, never the arguments.
fn name_of(path: &Path) -> String {
    path.file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn code_text(code: i32) -> String {
    match code {
        0..=255 => code.to_string(),
        _ => format!("{code} (0x{:08X})", code as u32),
    }
}

fn shown(program: &OsStr) -> String {
    program.to_string_lossy().into_owned()
}

// Write errors are ignored: `eprintln!` would panic on a closed pipe, and the release build aborts on panic.
fn say(warn: &mut dyn Write, text: &str) {
    let _ = writeln!(warn, "wakeshot: {text}");
}

/// For a data folder that cannot be found: the command still runs, unlogged.
struct NoLog;

impl Log for NoLog {
    fn line(&self, _: Level, _: &'static str, _: &str) {}
}

#[cfg(test)]
mod tests {
    use std::thread;
    use std::time::{Duration, Instant};

    use wakeshot_core::holds::HoldKind;
    use wakeshot_core::holds::file::{Parsed, kind_of, parse};
    use wakeshot_core::process::{Alive, ProcessRef};
    use wakeshot_windows::log::MemoryLog;

    use super::*;
    use crate::status::fixture::{TempData, dead, me, state_json};

    fn leave_ctrl_c() -> Result<(), WinError> {
        Ok(())
    }

    fn handler_fails() -> Result<(), WinError> {
        Err(WinError {
            call: "SetConsoleCtrlHandler",
            code: 6,
        })
    }

    struct Ran {
        code: i32,
        said: String,
        log: Vec<(Level, String)>,
    }

    fn run_with(
        dir: Option<&DataDir>,
        ignore_ctrl_c: fn() -> Result<(), WinError>,
        program: &str,
        rest: &str,
    ) -> Ran {
        let log = MemoryLog::default();
        let parts = Parts {
            liveness: &SystemLiveness,
            log: &log,
            ignore_ctrl_c,
        };
        let mut said = Vec::new();
        let code = run(
            dir,
            &parts,
            &mut said,
            OsStr::new(program),
            OsStr::new(rest),
        );
        let log = log.lines.into_inner().unwrap();
        Ran {
            code,
            said: String::from_utf8(said).unwrap(),
            log: log
                .into_iter()
                .map(|(level, _, text)| (level, text))
                .collect(),
        }
    }

    fn ran(data: &TempData, program: &str, rest: &str) -> Ran {
        run_with(Some(&data.dir), leave_ctrl_c, program, rest)
    }

    fn hold_files(data: &TempData) -> Vec<String> {
        std::fs::read_dir(data.dir.holds())
            .map(|entries| {
                entries
                    .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// A tray that runs (this test process) with Automatic as given; `None` writes no `config.json`.
    fn tray_running(data: &TempData, automatic: Option<bool>) {
        data.write(DataFile::State, &state_json(me(), true, &[], &[], false));
        if let Some(on) = automatic {
            data.write(
                DataFile::Config,
                &format!(r#"{{"version":1,"automatic":{on}}}"#),
            );
        }
    }

    #[test]
    fn the_childs_exit_code_comes_back() {
        let data = TempData::new("run-code");
        tray_running(&data, None);
        let seven = ran(&data, "cmd", "/d /c exit 7");
        assert_eq!(seven.code, 7);
        assert_eq!(seven.said, "");
        let pid = match &seven.log[..] {
            [(Level::Info, started), (Level::Info, ended)] => {
                assert_eq!(ended, "run: cmd exited with 7");
                started
                    .strip_prefix("run: cmd started, pid ")
                    .unwrap()
                    .to_owned()
            }
            other => panic!("{other:?}"),
        };
        assert!(pid.parse::<u32>().is_ok(), "{pid}");
        assert_eq!(ran(&data, "cmd", "/d /c exit 300").code, 300);
        assert_eq!(ran(&data, "cmd.exe", "/d /c exit 0").code, 0);
        assert_eq!(hold_files(&data), Vec::<String>::new());
    }

    #[test]
    fn a_crash_code_passes_through_whole() {
        let data = TempData::new("run-crash");
        tray_running(&data, None);
        let ctrl_c = ran(&data, "cmd", "/d /c exit -1073741510");
        assert_eq!(ctrl_c.code, 0xC000_013A_u32 as i32);
        assert_eq!(
            ctrl_c.log.last().unwrap().1,
            "run: cmd exited with -1073741510 (0xC000013A)"
        );
    }

    /// Runs `cmd <rest>` on a thread and checks the hold file while the child runs: its name, kind, pid, start and
    /// reason. Gives the file's text and the exit code.
    fn held_while_running(tag: &str, rest: &str) -> (String, i32) {
        let data = TempData::new(tag);
        tray_running(&data, None);
        let holds = data.dir.holds();
        let (text, code) = thread::scope(|scope| {
            let running = scope.spawn(|| ran(&data, "cmd", rest).code);
            let deadline = Instant::now() + Duration::from_secs(10);
            let (name, bytes) = loop {
                let found = std::fs::read_dir(&holds).ok().and_then(|entries| {
                    entries.flatten().find_map(|e| {
                        let name = e.file_name().to_string_lossy().into_owned();
                        // The atomic write's temp file comes first; it never ends in `.json`.
                        if !name.ends_with(".json") {
                            return None;
                        }
                        let bytes = std::fs::read(e.path()).ok()?;
                        Some((name, bytes))
                    })
                });
                if let Some(found) = found {
                    break found;
                }
                assert!(Instant::now() < deadline, "no hold file within 10 s");
                thread::sleep(Duration::from_millis(20));
            };
            assert_eq!(kind_of(&name), Some(HoldKind::Run), "{name}");
            let hold = match parse(&name, &bytes) {
                Parsed::Ok(hold) => hold,
                other => panic!("{other:?}"),
            };
            assert_eq!(name, format!("run-{}.json", hold.file.pid));
            assert_eq!(hold.reason.as_deref(), Some("cmd"));
            assert_eq!(hold.expires_wall_ms, None);
            let start = hold.file.proc_start.expect("procStart is written");
            let child = ProcessRef {
                pid: hold.file.pid,
                start,
            };
            assert_eq!(SystemLiveness.check(child), Alive::Yes);
            let code = running.join().unwrap();
            assert_eq!(SystemLiveness.check(child), Alive::Gone);
            (String::from_utf8(bytes).unwrap(), code)
        });
        assert_eq!(hold_files(&data), Vec::<String>::new());
        (text, code)
    }

    #[test]
    fn the_hold_file_exists_while_the_command_runs_and_is_gone_after() {
        let (_, code) = held_while_running("run-held", r#"/d /c "ping -n 3 127.0.0.1 >nul""#);
        assert_eq!(code, 0);
    }

    #[test]
    fn the_reason_never_holds_the_arguments() {
        let rest = r#"/d /c "ping -n 2 127.0.0.1 >nul & rem secret-token""#;
        let (text, _) = held_while_running("run-reason", rest);
        for word in ["secret", "ping", "127.0.0.1", "/d"] {
            assert!(!text.contains(word), "{word} in {text}");
        }
    }

    #[test]
    fn a_cmd_file_sees_its_arguments_as_typed() {
        let data = TempData::new("run-batch");
        tray_running(&data, None);
        let folder = data.dir.root().join("bin dir");
        std::fs::create_dir_all(&folder).unwrap();
        let script = folder.join("show args.cmd");
        std::fs::write(
            &script,
            "@echo off\r\n>\"%~dp0seen.txt\" echo(%*\r\n>>\"%~dp0seen.txt\" echo(%1\r\nexit /b 3\r\n",
        )
        .unwrap();
        let rest = r#""a  b" "c d" e\f --x"#;
        let batch = ran(&data, script.to_str().unwrap(), rest);
        assert_eq!(batch.code, 3);
        assert_eq!(
            batch.log[0].1.split(',').next(),
            Some("run: show args started")
        );
        let seen = std::fs::read_to_string(folder.join("seen.txt")).unwrap();
        assert_eq!(seen, format!("{rest}\r\n\"a  b\"\r\n"));
    }

    #[test]
    fn a_program_not_found_starts_nothing() {
        let data = TempData::new("run-missing");
        let bare = ran(&data, "wakeshot-test-no-such-program", "a b");
        assert_eq!(bare.code, 127);
        assert_eq!(
            bare.said,
            "wakeshot: 'wakeshot-test-no-such-program' was not found in PATH. For a program in this folder, write \
             '.\\wakeshot-test-no-such-program'.\n"
        );
        assert_eq!(
            bare.log,
            [(
                Level::Warn,
                "run: wakeshot-test-no-such-program not found".into()
            )]
        );
        let missing = data.dir.root().join("missing.exe");
        let path = ran(&data, missing.to_str().unwrap(), "");
        assert_eq!(path.code, 127);
        assert_eq!(
            path.said,
            format!("wakeshot: '{}' was not found.\n", missing.display())
        );
        assert!(!data.dir.holds().exists());
    }

    #[test]
    fn a_script_wakeshot_does_not_start_is_named() {
        let data = TempData::new("run-script");
        let script = data.dir.root().join("deploy.ps1");
        std::fs::write(&script, "Write-Output hi").unwrap();
        let ps1 = ran(&data, script.to_str().unwrap(), "-Force");
        assert_eq!(ps1.code, 126);
        assert_eq!(
            ps1.said,
            format!(
                "wakeshot: '{0}' is a PowerShell script; run it as 'wakeshot run -- pwsh -File {0}'.\n",
                script.display()
            )
        );
        assert_eq!(
            ps1.log,
            [(Level::Warn, "run: deploy not started, a .ps1 file".into())]
        );
        let text = data.dir.root().join("notes.txt");
        std::fs::write(&text, "").unwrap();
        let other = ran(&data, text.to_str().unwrap(), "");
        assert_eq!(other.code, 126);
        assert!(
            other
                .said
                .contains("Wakeshot starts .exe, .com, .bat and .cmd files, not '"),
            "{}",
            other.said
        );
        assert!(!data.dir.holds().exists());
    }

    #[test]
    fn a_spaced_script_path_is_quoted_in_the_hint() {
        let hint = not_runnable(OsStr::new(r"C:\my dir\x.ps1"), "PS1");
        assert_eq!(
            hint,
            r#"'C:\my dir\x.ps1' is a PowerShell script; run it as 'wakeshot run -- pwsh -File "C:\my dir\x.ps1"'."#
        );
    }

    #[test]
    fn warns_when_the_tray_is_not_running_and_runs_anyway() {
        let warned = format!("wakeshot: {NOT_RUNNING}\n");
        let data = TempData::new("run-no-tray");
        let missing = ran(&data, "cmd", "/d /c exit 7");
        assert_eq!((missing.code, missing.said.as_str()), (7, warned.as_str()));
        data.write(DataFile::State, &state_json(dead(), true, &[], &[], false));
        let gone = ran(&data, "cmd", "/d /c exit 7");
        assert_eq!((gone.code, gone.said.as_str()), (7, warned.as_str()));
        data.write(DataFile::State, "not json");
        let unreadable = ran(&data, "cmd", "/d /c exit 7");
        assert_eq!(
            (unreadable.code, unreadable.said.as_str()),
            (7, warned.as_str())
        );
        assert_eq!(hold_files(&data), Vec::<String>::new());
    }

    #[test]
    fn warns_when_automatic_is_off() {
        let data = TempData::new("run-automatic");
        tray_running(&data, Some(false));
        let off = ran(&data, "cmd", "/d /c exit 0");
        assert_eq!(off.said, format!("wakeshot: {AUTOMATIC_OFF}\n"));
        assert_eq!(off.code, 0);
        tray_running(&data, Some(true));
        assert_eq!(ran(&data, "cmd", "/d /c exit 0").said, "");
        data.write(DataFile::Config, "{ broken");
        assert_eq!(ran(&data, "cmd", "/d /c exit 0").said, "");
    }

    #[test]
    fn a_newer_tray_gets_no_warning() {
        let data = TempData::new("run-newer");
        data.write(DataFile::State, r#"{"version":9,"pid":4}"#);
        assert_eq!(ran(&data, "cmd", "/d /c exit 0").said, "");
    }

    #[test]
    fn a_failed_hold_write_warns_and_the_command_still_runs() {
        let data = TempData::new("run-no-holds");
        tray_running(&data, None);
        // A file where the folder belongs: the write cannot make `holds\`.
        std::fs::write(data.dir.holds(), "").unwrap();
        let blocked = ran(&data, "cmd", "/d /c exit 5");
        assert_eq!(blocked.code, 5);
        assert!(
            blocked
                .said
                .starts_with("wakeshot: could not write the hold file (")
                && blocked
                    .said
                    .ends_with("), so this command does not keep the PC awake.\n"),
            "{}",
            blocked.said
        );
        assert!(
            blocked.log.iter().any(|(level, text)| *level == Level::Warn
                && text.starts_with("run: hold file not written, ")),
            "{:?}",
            blocked.log
        );
    }

    #[test]
    fn no_data_folder_still_runs_the_command() {
        let none = run_with(None, leave_ctrl_c, "cmd", "/d /c exit 4");
        assert_eq!((none.code, none.said.as_str()), (4, ""));
    }

    #[test]
    fn a_failed_ctrl_c_handler_is_logged_and_the_command_runs() {
        let data = TempData::new("run-handler");
        tray_running(&data, None);
        let failed = run_with(Some(&data.dir), handler_fails, "cmd", "/d /c exit 2");
        assert_eq!(failed.code, 2);
        assert_eq!(
            failed.log[0],
            (
                Level::Warn,
                "run: Ctrl+C handler not set, SetConsoleCtrlHandler failed with 6".into()
            )
        );
    }

    #[test]
    fn the_name_is_the_file_name_without_extension() {
        assert_eq!(
            name_of(Path::new(r"C:\Program Files\nodejs\npm.cmd")),
            "npm"
        );
        assert_eq!(name_of(Path::new(r"C:\x\my.tool.exe")), "my.tool");
        assert_eq!(name_of(Path::new("tool")), "tool");
        assert_eq!(code_text(0), "0");
        assert_eq!(code_text(255), "255");
        assert_eq!(code_text(256), "256 (0x00000100)");
        assert_eq!(code_text(-1), "-1 (0xFFFFFFFF)");
    }
}
