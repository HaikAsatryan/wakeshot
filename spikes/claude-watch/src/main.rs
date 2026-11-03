//! Spike for card `p0-claude-live-tests`. Logs every change to Claude Code status files
//! (`%USERPROFILE%\.claude\sessions\<pid>.json`) and the transcript lines about timers and background work for the
//! matching sessions, plus task output files and `scheduled_tasks.json` / `.lock` in each session's folder.
//!
//! Usage: `claude-watch [--log FILE] [--interval-ms N] [--kinds] [--backlog]`
//!
//! - `--kinds`: also log a short kind line for every other transcript line (type, subtype, tool names, no text).
//! - `--backlog`: read transcripts of sessions that already run at start from the beginning.
//!
//! One-shot modes for card `p0-unelevated-liveness` (see `probe.rs`):
//!
//! - `--probe [--pid N]... [--wait-ms N] [--notify-ms N]`: for every status file, the token levels of both sides,
//!   which `OpenProcess` rights work, start time, image path, a wait on the handle, the process security descriptor,
//!   the system process list, and the transcript read; then change notifications on the sessions folder.
//! - `--wait-exit PID [--timeout-ms N]`: waits on one process handle and logs when it signals.
//!
//! Strings in logged transcript values become `<str N>` unless the key is on a short allow list, so prompts,
//! commands and output never reach the log. Files named `<pid>.<hash>.key` are never opened: see `names.rs`.

mod names;
mod probe;
mod procs;
mod summary;
mod tail;
mod watch;

use std::path::PathBuf;
use std::time::Duration;

const USAGE: &str = "usage: claude-watch [--log FILE] [--interval-ms N] [--kinds] [--backlog]\n       \
claude-watch --probe [--log FILE] [--pid N]... [--wait-ms N] [--notify-ms N]\n       \
claude-watch --wait-exit PID [--log FILE] [--timeout-ms N]";

fn number<T: std::str::FromStr>(value: Option<String>, flag: &str) -> T {
    match value.and_then(|v| v.parse().ok()) {
        Some(n) => n,
        None => {
            eprintln!("{flag} needs a number\n{USAGE}");
            std::process::exit(2);
        }
    }
}

fn main() {
    let mut log_path: Option<PathBuf> = None;
    let mut interval = Duration::from_millis(200);
    let mut kinds = false;
    let mut backlog = false;
    let mut probe = false;
    let mut wait_exit: Option<u32> = None;
    let mut extra_pids: Vec<u32> = Vec::new();
    let mut wait_ms: u32 = 1000;
    let mut notify_ms: u64 = 0;
    let mut timeout_ms: u32 = 600_000;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--log" => log_path = args.next().map(PathBuf::from),
            "--interval-ms" => {
                let ms = args.next().and_then(|v| v.parse().ok()).unwrap_or(200);
                interval = Duration::from_millis(ms);
            }
            "--kinds" => kinds = true,
            "--backlog" => backlog = true,
            "--probe" => probe = true,
            "--pid" => extra_pids.push(number(args.next(), &arg)),
            "--wait-ms" => wait_ms = number(args.next(), &arg),
            "--notify-ms" => notify_ms = number(args.next(), &arg),
            "--wait-exit" => wait_exit = Some(number(args.next(), &arg)),
            "--timeout-ms" => timeout_ms = number(args.next(), &arg),
            other => {
                eprintln!("unknown argument {other}\n{USAGE}");
                std::process::exit(2);
            }
        }
    }
    let Some(home) = std::env::var_os("USERPROFILE").map(PathBuf::from) else {
        eprintln!("USERPROFILE is not set");
        std::process::exit(2);
    };
    let mut log = match watch::Logger::new(log_path.as_deref()) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("cannot open log file: {e}");
            std::process::exit(2);
        }
    };
    if let Some(pid) = wait_exit {
        probe::wait_exit(&mut log, pid, timeout_ms);
        return;
    }
    if probe {
        let opts = probe::ProbeOptions {
            home: &home,
            extra_pids: &extra_pids,
            wait_ms,
            notify_ms,
        };
        probe::run(&mut log, &opts);
        return;
    }
    let opts = watch::Options {
        home,
        temp: std::env::temp_dir(),
        kinds,
        backlog,
    };
    let mut watcher = watch::Watcher::new(opts, log);
    watcher.log.line(&format!(
        "claude-watch started, every {} ms, kinds={kinds}, backlog={backlog}",
        interval.as_millis()
    ));
    loop {
        watcher.round();
        std::thread::sleep(interval);
    }
}
