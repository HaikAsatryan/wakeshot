//! Drives a terminal agent the way a person does: one command in a pseudo console, a step file of waits and keys,
//! the screen kept in a log. Test helper for the live cards (`docs/tasks/p4-live-harness.md`), not product code.
//!
//! `agent-drive <folder>` reads `<folder>\steps.txt` and writes, in the same folder: `started.json` (driver and
//! child pids, at once), `screen.txt` (the screen now), `log.txt`, `done.json` (the result, last). It is started at
//! Medium through `explorer.exe`, so the caller cannot wait on it: it waits for `done.json`. After a `serve` step it
//! runs each `<folder>\in\*.txt` step file dropped in (results in `<name>.result`) until `end`, the child's end or
//! the timeout. At the end it always ends the child's whole tree (a job object) and reports what was left.

mod conpty;
mod script;
mod term;

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

use conpty::Child;
use script::{Action, EnvChange, Script, Step};
use term::Term;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const TICK: Duration = Duration::from_millis(50);
/// Output can lag the root's exit a little; a wait looks this much longer before it calls the child gone.
const EXIT_GRACE: Duration = Duration::from_millis(700);

fn main() -> ExitCode {
    let Some(folder) = std::env::args_os().nth(1).map(PathBuf::from) else {
        eprintln!("usage: agent-drive <folder with steps.txt>");
        return ExitCode::from(2);
    };
    for name in [
        "started.json",
        "screen.txt",
        "log.txt",
        "done.json",
        "out.raw",
    ] {
        let _ = fs::remove_file(folder.join(name));
    }
    let started = now_text();
    let script = match fs::read_to_string(folder.join("steps.txt"))
        .map_err(|e| format!("steps.txt: {e}"))
        .and_then(|text| script::parse_script(&text))
    {
        Ok(script) => script,
        Err(e) => return finish_early(&folder, &started, &e),
    };
    let vars = match child_env(&script.settings.env) {
        Ok(vars) => vars,
        Err(e) => return finish_early(&folder, &started, &e),
    };
    let (tx, rx) = mpsc::channel();
    let child = match Child::spawn(
        &script.command_line,
        script.settings.cwd.as_deref(),
        &vars,
        script.settings.cols,
        script.settings.rows,
        tx,
    ) {
        Ok(child) => child,
        Err(e) => return finish_early(&folder, &started, &e),
    };
    let mut run = Run::new(folder, script, child, rx);
    run.started = started;
    run.go()
}

/// The parent's environment, minus what marks a shell as a Claude Code session (a test session started from one
/// would count as a child session and write no status file), plus the step file's changes.
fn child_env(changes: &[EnvChange]) -> Result<Vec<(OsString, OsString)>, String> {
    let mut vars: Vec<(OsString, OsString)> = std::env::vars_os()
        .filter(|(name, _)| {
            let upper = name.to_string_lossy().to_uppercase();
            !(upper == "AI_AGENT" || (upper.starts_with("CLAUDE") && upper != "CLAUDE_CONFIG_DIR"))
        })
        .collect();
    for change in changes {
        let (name, value) = match change {
            EnvChange::Set(name, value) => (name.clone(), Some(OsString::from(value))),
            EnvChange::Unset(name) => (name.clone(), None),
            EnvChange::FromProgram(name, argv) => {
                let out = Command::new(&argv[0])
                    .args(&argv[1..])
                    .stdin(Stdio::null())
                    .stderr(Stdio::null())
                    .creation_flags(CREATE_NO_WINDOW)
                    .output()
                    .map_err(|e| format!("env-from {name}: {} did not start: {e}", argv[0]))?;
                if !out.status.success() {
                    return Err(format!(
                        "env-from {name}: {} failed ({})",
                        argv[0], out.status
                    ));
                }
                let value = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if value.is_empty() {
                    return Err(format!("env-from {name}: empty output"));
                }
                (name.clone(), Some(OsString::from(value)))
            }
        };
        vars.retain(|(n, _)| !n.to_string_lossy().eq_ignore_ascii_case(&name));
        if let Some(value) = value {
            vars.push((OsString::from(name), value));
        }
    }
    Ok(vars)
}

fn finish_early(folder: &Path, started: &str, reason: &str) -> ExitCode {
    let done = serde_json::json!({
        "result": "error",
        "reason": reason,
        "started": started,
        "ended": now_text(),
        "driver_pid": std::process::id(),
    });
    write_atomic(&folder.join("done.json"), &format!("{done:#}\n"));
    eprintln!("agent-drive: {reason}");
    ExitCode::from(2)
}

enum Flow {
    Next,
    Serve,
    End,
}

struct Run {
    folder: PathBuf,
    script: Script,
    child: Child,
    rx: Receiver<Vec<u8>>,
    term: Term,
    log: File,
    raw: Option<File>,
    started: String,
    deadline: Instant,
    text: String,
    changed_at: Instant,
    screen_written: Option<Instant>,
    screen_dirty: bool,
    pids_at: Instant,
    seen: BTreeMap<u32, String>,
    exited_at: Option<Instant>,
    exit_code: Option<u32>,
    steps_done: usize,
}

impl Run {
    fn new(folder: PathBuf, script: Script, child: Child, rx: Receiver<Vec<u8>>) -> Self {
        let log = OpenOptions::new()
            .create(true)
            .append(true)
            .open(folder.join("log.txt"))
            .expect("log.txt");
        let raw = script
            .settings
            .raw
            .then(|| File::create(folder.join("out.raw")).expect("out.raw"));
        let term = Term::new(script.settings.rows, script.settings.cols);
        let now = Instant::now();
        Self {
            deadline: now + script.settings.timeout,
            folder,
            script,
            child,
            rx,
            term,
            log,
            raw,
            started: String::new(),
            text: String::new(),
            changed_at: now,
            screen_written: None,
            screen_dirty: false,
            pids_at: now - Duration::from_secs(1),
            seen: BTreeMap::new(),
            exited_at: None,
            exit_code: None,
            steps_done: 0,
        }
    }

    fn go(mut self) -> ExitCode {
        let started = serde_json::json!({
            "driver_pid": std::process::id(),
            "child_pid": self.child.pid,
            "started": self.started,
        });
        write_atomic(&self.folder.join("started.json"), &format!("{started:#}\n"));
        self.note(&format!(
            "driver pid {} started child pid {}: {}",
            std::process::id(),
            self.child.pid,
            self.script.command_line
        ));
        let steps = self.script.steps.clone();
        let mut outcome = self.run_steps(&steps);
        if let Ok(Flow::Serve) = outcome {
            outcome = self.serve();
        }
        let (result, reason, failed) = match outcome {
            Ok(_) => ("pass", String::new(), None),
            Err((step, reason)) => {
                let kind = if Instant::now() >= self.deadline {
                    "timeout"
                } else {
                    "fail"
                };
                (kind, reason, step)
            }
        };
        if result != "pass" {
            self.snap(&format!("{result}: {reason}"));
        }
        self.finish(result, &reason, failed)
    }

    fn run_steps(&mut self, steps: &[Step]) -> Result<Flow, (Option<String>, String)> {
        for step in steps {
            let begun = Instant::now();
            self.note(&format!("line {}: {}", step.line, step.source));
            match self.step(&step.action) {
                Ok(Flow::Next) => {}
                Ok(flow) => return Ok(flow),
                Err(e) => {
                    self.note(&format!("line {} failed: {e}", step.line));
                    return Err((Some(format!("line {}: {}", step.line, step.source)), e));
                }
            }
            self.steps_done += 1;
            self.note(&format!(
                "line {} ok ({:.1} s)",
                step.line,
                begun.elapsed().as_secs_f64()
            ));
        }
        Ok(Flow::Next)
    }

    fn serve(&mut self) -> Result<Flow, (Option<String>, String)> {
        let inbox = self.folder.join("in");
        let _ = fs::create_dir_all(&inbox);
        self.note("serving step files from in\\");
        loop {
            let mut names: Vec<PathBuf> = fs::read_dir(&inbox)
                .map(|it| it.filter_map(|e| e.ok().map(|e| e.path())).collect())
                .unwrap_or_default();
            names.retain(|p| p.extension().is_some_and(|x| x == "txt"));
            names.sort();
            for path in names {
                let taken = path.with_extension("taken");
                if fs::rename(&path, &taken).is_err() {
                    continue;
                }
                let name = path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
                self.note(&format!("step file {name}"));
                let outcome = fs::read_to_string(&taken)
                    .map_err(|e| e.to_string())
                    .and_then(|text| script::parse_steps(&text));
                let (result, end) = match outcome {
                    Err(e) => (format!("fail: {e}"), false),
                    Ok(steps) => match self.run_steps(&steps) {
                        Ok(Flow::End) => ("pass".to_string(), true),
                        Ok(_) => ("pass".to_string(), false),
                        Err((step, e)) => {
                            self.snap(&format!("{name} failed"));
                            (format!("fail: {}: {e}", step.unwrap_or_default()), false)
                        }
                    },
                };
                write_atomic(
                    &inbox.join(format!("{name}.result")),
                    &format!("{result}\n"),
                );
                if end {
                    return Ok(Flow::End);
                }
            }
            if self.gone_for_good() {
                self.note("child ended while serving");
                return Ok(Flow::End);
            }
            if Instant::now() >= self.deadline {
                return Err((None, "timeout while serving".into()));
            }
            self.pump(Duration::from_millis(200));
        }
    }

    fn step(&mut self, action: &Action) -> Result<Flow, String> {
        match action {
            Action::Wait { limit, any } => {
                let until = Instant::now() + *limit;
                loop {
                    if let Some(hit) = any.iter().find(|t| self.text.contains(t.as_str())) {
                        self.note(&format!("seen `{hit}`"));
                        return Ok(Flow::Next);
                    }
                    self.check_time(until, "not seen")?;
                    self.pump(TICK);
                }
            }
            Action::WaitNew { limit, text } => {
                let until = Instant::now() + *limit;
                let before = self.text.matches(text.as_str()).count();
                while self.text.matches(text.as_str()).count() <= before {
                    self.check_time(until, "no new one")?;
                    self.pump(TICK);
                }
            }
            Action::WaitGone { limit, text } => {
                let until = Instant::now() + *limit;
                while self.text.contains(text.as_str()) {
                    self.check_time(until, "still on screen")?;
                    self.pump(TICK);
                }
            }
            Action::Quiet { limit, still } => {
                let until = Instant::now() + *limit;
                while self.changed_at.elapsed() < *still {
                    self.check_time(until, "screen kept changing")?;
                    self.pump(TICK);
                }
            }
            Action::Type(text) => {
                let delay = self.script.settings.type_delay;
                let mut buf = [0u8; 4];
                for c in text.chars() {
                    self.child.write(c.encode_utf8(&mut buf).as_bytes())?;
                    self.pump(delay);
                }
            }
            Action::Paste(text) => {
                // What a terminal does on paste: bracketed only when the app asked for it.
                let mut bytes = Vec::new();
                let bracketed = self.term.bracketed_paste();
                if bracketed {
                    bytes.extend_from_slice(b"\x1b[200~");
                }
                bytes.extend_from_slice(text.as_bytes());
                if bracketed {
                    bytes.extend_from_slice(b"\x1b[201~");
                }
                self.child.write(&bytes)?;
                self.pump(TICK);
            }
            Action::Send(bytes) => {
                self.child.write(bytes)?;
                self.pump(TICK);
            }
            Action::Sleep(d) => {
                let until = Instant::now() + *d;
                while Instant::now() < until {
                    if Instant::now() >= self.deadline {
                        return Err("timeout".into());
                    }
                    self.pump(TICK);
                }
            }
            Action::Snap(label) => self.snap(label),
            Action::Kill(target) => {
                let pid = self.child.kill_one(target)?;
                self.note(&format!("killed {target}: pid {pid}"));
            }
            Action::KillTree => {
                self.child.kill_tree()?;
                self.note("killed the tree");
            }
            Action::WaitExit(limit) => {
                let until = Instant::now() + *limit;
                while self.exit_code.is_none() {
                    if Instant::now() >= until {
                        return Err("child still running".into());
                    }
                    self.pump(TICK);
                }
            }
            Action::Pids => {
                self.refresh_pids();
                let list = self.live_tree();
                self.note(&format!("tree: {list}"));
            }
            Action::Serve => return Ok(Flow::Serve),
            Action::End => return Ok(Flow::End),
        }
        Ok(Flow::Next)
    }

    fn check_time(&self, until: Instant, what: &str) -> Result<(), String> {
        if Instant::now() >= self.deadline {
            return Err(format!("{what}: timeout of the whole run"));
        }
        if Instant::now() >= until {
            return Err(format!("{what} in time"));
        }
        if self.gone_for_good() {
            return Err(format!("{what}: child ended (exit {:?})", self.exit_code));
        }
        Ok(())
    }

    fn gone_for_good(&self) -> bool {
        self.exited_at.is_some_and(|at| at.elapsed() >= EXIT_GRACE)
    }

    /// Reads output for at most `max`, feeds the screen, answers queries, and notes the child's end.
    fn pump(&mut self, max: Duration) {
        let until = Instant::now() + max;
        loop {
            let left = until.saturating_duration_since(Instant::now());
            match self.rx.recv_timeout(left.min(TICK)) {
                Ok(chunk) => self.feed(&chunk),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => std::thread::sleep(left.min(TICK)),
            }
            while let Ok(chunk) = self.rx.try_recv() {
                self.feed(&chunk);
            }
            if self.exit_code.is_none()
                && let Some(code) = self.child.exit_code()
            {
                self.exit_code = Some(code);
                self.exited_at = Some(Instant::now());
                self.note(&format!(
                    "child pid {} ended, exit {code:#x}",
                    self.child.pid
                ));
            }
            if self.pids_at.elapsed() >= Duration::from_millis(500) {
                self.refresh_pids();
            }
            self.write_screen(false);
            if Instant::now() >= until {
                return;
            }
        }
    }

    fn feed(&mut self, chunk: &[u8]) {
        if let Some(raw) = self.raw.as_mut() {
            let _ = raw.write_all(chunk);
        }
        let replies = self.term.feed(chunk);
        if !replies.is_empty() {
            let _ = self.child.write(&replies);
        }
        for kind in self.term.new_kinds() {
            self.note(&format!("sequence {kind}"));
        }
        let text = self.term.text();
        if text != self.text {
            self.text = text;
            self.changed_at = Instant::now();
            self.screen_dirty = true;
        }
    }

    fn write_screen(&mut self, force: bool) {
        let due = self
            .screen_written
            .is_none_or(|at| at.elapsed() >= Duration::from_millis(250));
        if self.screen_dirty && (due || force) {
            write_atomic(&self.folder.join("screen.txt"), &self.text);
            self.screen_written = Some(Instant::now());
            self.screen_dirty = false;
        }
    }

    fn refresh_pids(&mut self) {
        self.pids_at = Instant::now();
        for pid in self.child.job_pids() {
            if !self.seen.contains_key(&pid) {
                let image = conpty::image_name(pid).unwrap_or_else(|| "?".into());
                self.note(&format!("tree gained pid {pid} {image}"));
                self.seen.insert(pid, image);
            }
        }
    }

    fn live_tree(&self) -> String {
        let pids = self.child.job_pids();
        let parts: Vec<String> = pids
            .iter()
            .map(|pid| format!("{pid} {}", self.seen.get(pid).map_or("?", String::as_str)))
            .collect();
        if parts.is_empty() {
            "empty".into()
        } else {
            parts.join(", ")
        }
    }

    fn snap(&mut self, label: &str) {
        let mut lines: Vec<&str> = self.text.lines().collect();
        while lines.last().is_some_and(|l| l.trim().is_empty()) {
            lines.pop();
        }
        let mut block = format!("{} screen ({label}):\n", clock());
        for line in lines {
            block.push_str("  | ");
            block.push_str(line.trim_end());
            block.push('\n');
        }
        let _ = self.log.write_all(block.as_bytes());
    }

    fn note(&mut self, line: &str) {
        let _ = writeln!(self.log, "{} {line}", clock());
    }

    fn finish(mut self, result: &str, reason: &str, failed: Option<String>) -> ExitCode {
        self.pump(Duration::from_millis(100));
        self.refresh_pids();
        let left_running = self.child.job_pids();
        let killed = !left_running.is_empty();
        if killed {
            let tree = self.live_tree();
            self.note(&format!("ending the tree: {tree}"));
            if let Err(e) = self.child.kill_tree() {
                self.note(&e);
            }
        }
        let until = Instant::now() + Duration::from_secs(5);
        // The job's count can reach zero a moment before the root's handle is signaled; wait for both.
        while (self.child.active_count() != 0 || self.child.exit_code().is_none())
            && Instant::now() < until
        {
            std::thread::sleep(TICK);
        }
        let active_after = self.child.active_count();
        if self.exit_code.is_none() {
            self.exit_code = self.child.exit_code();
        }
        self.write_screen(true);
        let pids: Vec<_> = self
            .seen
            .iter()
            .map(|(pid, image)| serde_json::json!({"pid": pid, "image": image}))
            .collect();
        let done = serde_json::json!({
            "result": result,
            "reason": reason,
            "failed_step": failed,
            "steps_done": self.steps_done,
            "started": self.started,
            "ended": now_text(),
            "driver_pid": std::process::id(),
            "child_pid": self.child.pid,
            "exit_code": self.exit_code,
            "killed_tree_at_end": killed,
            "left_running_at_end": left_running,
            "active_after": active_after,
            "pids": pids,
        });
        self.note(&format!(
            "result {result} {reason}; active after {active_after}"
        ));
        let folder = self.folder.clone();
        self.child.close();
        write_atomic(&folder.join("done.json"), &format!("{done:#}\n"));
        if result == "pass" && active_after == 0 {
            ExitCode::SUCCESS
        } else {
            ExitCode::from(1)
        }
    }
}

/// Write to a side file, then rename, so a reader never sees half a file. A reader holding the file open makes the
/// rename fail for a moment on Windows: retry, never give up the run for it.
fn write_atomic(path: &Path, text: &str) {
    let tmp = path.with_extension("part");
    if fs::write(&tmp, text).is_err() {
        return;
    }
    for _ in 0..20 {
        if fs::rename(&tmp, path).is_ok() {
            return;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    let _ = fs::remove_file(&tmp);
}

fn local_time() -> windows::Win32::Foundation::SYSTEMTIME {
    unsafe { windows::Win32::System::SystemInformation::GetLocalTime() }
}

fn clock() -> String {
    let t = local_time();
    format!(
        "{:02}:{:02}:{:02}.{:03}",
        t.wHour, t.wMinute, t.wSecond, t.wMilliseconds
    )
}

fn now_text() -> String {
    let t = local_time();
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:03}",
        t.wYear, t.wMonth, t.wDay, t.wHour, t.wMinute, t.wSecond, t.wMilliseconds
    )
}
