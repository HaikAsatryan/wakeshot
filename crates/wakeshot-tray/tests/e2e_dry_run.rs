//! The two exes together as a user runs them, in dry run: every CLI command against a running dry-run tray, Keep
//! awake, `wakeshot run`, Automatic, restore, quit, a crash and the next start, and the log
//! (`docs/tasks/p6-int-dry-run-e2e.md`). No power setting is written: the dry-run tray uses `DryRunPower`, dry-run
//! requests, lock and hibernate.
//!
//! Owner: `p6-int-dry-run-e2e`; the plain taskkill test `p6-polite-quit-notice-retry`. Every test is ignored and named
//! `real_e2e_*`: the dry-run icon shows on the taskbar,
//! so they run only in a screen slot, alone, after the CLI is built into the same target folder:
//!
//! ```text
//! cargo build --locked -p wakeshot-cli --target x86_64-pc-windows-msvc
//! cargo test --locked -p wakeshot-tray --target x86_64-pc-windows-msvc --test e2e_dry_run -- --ignored --test-threads=1
//! ```

mod e2e;

use std::thread;
use std::time::{Duration, Instant};

use e2e::{
    Claude, EXIT_WITHIN, FOLLOW_WITHIN, LogMark, SAFETY_TICK, SECOND_START_WITHIN, Started,
    TempData, Tray, not_plain, parse_line, skip, trays_of_this_build, wait_until,
};
use wakeshot_core::files::StateFile;
use wakeshot_core::holds::file::format_expires;
use wakeshot_core::text;
use wakeshot_core::view::KeepAwake;
use wakeshot_windows::clock;
use windows::Win32::Foundation::{HWND, LPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetClassNameW, GetWindowThreadProcessId,
};
use windows::core::BOOL;

const IDLE_STATUS: &str = "Wakeshot is running.\nAutomatic is on.\nNothing holds the PC awake.\nWakeshot has not \
                           changed your power settings.\n";
const NOT_RUNNING: &str = "Wakeshot is not running.\n";
const RESTORED: &str = "Your power settings are restored.\n";
/// A run hold is labelled with the program's name only (`docs/hold-files.md`).
const PING: &str = "ping";
/// The last minute may tick over between the CLI's write and the tray's state write.
const THIRTY_MINUTES: [KeepAwake; 2] = [KeepAwake::Minutes(30), KeepAwake::Minutes(29)];

fn until_off() -> String {
    text::keep_awake_group(KeepAwake::UntilOff)
}

fn keep_awake_only(state: &StateFile) -> bool {
    state.holding == [until_off()]
}

fn holds(state: &StateFile, label: &str) -> bool {
    state.holding.iter().any(|h| h == label)
}

fn idle(state: &StateFile) -> bool {
    state.holding.is_empty()
}

/// Keep awake until off, through the CLI, and the state and the dry-run apply that follow it. The apply writes the
/// dry run's own journal only when this PC's values differ from Wakeshot's (lid Sleep, a battery sleep timer here).
fn keep_awake_on(data: &TempData) {
    let said = data.cli("keep-awake on");
    assert_eq!(said.code, 0, "{said:?}");
    data.wait_state(FOLLOW_WITHIN, "Keep awake until off", keep_awake_only);
    wait_until(FOLLOW_WITHIN, "the dry-run journal", || {
        data.file("journal.json").exists().then_some(())
    });
    data.wait_state(FOLLOW_WITHIN, "overridden by the dry-run apply", |s| {
        keep_awake_only(s) && !s.overridden.is_empty()
    });
}

/// The pid in a `run-<pid>.json` name.
fn run_hold_pid(data: &TempData) -> u32 {
    let name = wait_until(FOLLOW_WITHIN, "a run hold file", || {
        data.hold_names()
            .into_iter()
            .find(|n| n.starts_with("run-"))
    });
    name.strip_prefix("run-")
        .and_then(|n| n.strip_suffix(".json"))
        .and_then(|n| n.parse().ok())
        .unwrap_or_else(|| panic!("{name} is not run-<pid>.json"))
}

fn count(lines: &[String], part: &str) -> usize {
    lines.iter().filter(|l| l.contains(part)).count()
}

/// The notice's line as the IO thread logs it: shown or not.
fn first_run_notices(lines: &[String]) -> usize {
    count(lines, "notice shown: FirstRun") + count(lines, "notice FirstRun not shown")
}

#[test]
#[ignore = "starts the dry-run tray, whose icon shows on the taskbar: screen slot only"]
fn real_e2e_start_writes_state_and_a_second_start_exits_at_once() {
    if skip() {
        return;
    }
    let data = TempData::with_config("start");
    let mut tray = Tray::start(&data, Claude::Empty);
    let state = data.state().expect("state.json");
    assert!(idle(&state), "holding at start: {:?}", state.holding);
    assert!(state.automatic);

    let (code, took) = data.start_expecting_exit();
    assert_eq!(code, 0, "the second start's exit code");
    assert!(took < SECOND_START_WITHIN, "{took:?}");
    assert_eq!(
        tray.exit_within(Duration::ZERO),
        None,
        "the first tray ended"
    );
    let running: Vec<u32> = trays_of_this_build()
        .iter()
        .map(|h| h.process().pid)
        .collect();
    assert_eq!(running, [tray.pid()], "the second start still runs");
    assert_eq!(
        data.state().map(|s| s.pid),
        Some(tray.pid()),
        "state.json changed hands"
    );
    // The second start ends at the tray mutex, before the log opens.
    let starts = count(&data.log_since(LogMark::START), "starts: dry run");
    assert_eq!(starts, 1, "start lines in the log");

    assert_eq!(tray.quit(), 0);
}

#[test]
#[ignore = "starts the dry-run tray, whose icon shows on the taskbar: screen slot only"]
fn real_e2e_first_run_notice_once_then_none() {
    if skip() {
        return;
    }
    let data = TempData::new("first-run");
    let mut tray = Tray::start(&data, Claude::Empty);
    let line = data.wait_log(LogMark::START, "FirstRun", FOLLOW_WITHIN);
    println!("first run: {line}");
    wait_until(
        FOLLOW_WITHIN,
        "config.json written at the first run",
        || data.file("config.json").exists().then_some(()),
    );
    assert_eq!(tray.quit(), 0);
    let first = data.log_since(LogMark::START);
    assert_eq!(first_run_notices(&first), 1, "first-run notice lines");
    assert_eq!(
        count(&first, "notice FirstRun not shown"),
        0,
        "the balloon failed"
    );

    let mark = data.log_mark();
    let mut again = Tray::start(&data, Claude::Empty);
    // The notice would come in the `Started` batch, which the IO thread runs within milliseconds.
    thread::sleep(Duration::from_secs(2));
    assert_eq!(again.quit(), 0);
    let second = data.log_since(mark);
    assert_eq!(
        first_run_notices(&second),
        0,
        "a first-run notice at the second start"
    );
    assert_eq!(count(&second, "FirstRun"), 0, "{second:#?}");
}

#[test]
#[ignore = "starts the dry-run tray, whose icon shows on the taskbar: screen slot only"]
fn real_e2e_keep_awake_lengths_screen_and_off() {
    if skip() {
        return;
    }
    let data = TempData::with_config("keep-awake");
    let mut tray = Tray::start(&data, Claude::Empty);

    let mark = data.log_mark();
    let said = data.cli("keep-awake 30m");
    assert_eq!(said.code, 0, "{said:?}");
    assert!(
        said.out
            .starts_with("Keep awake is on for 30 minutes, until "),
        "{said:?}"
    );
    let labels = THIRTY_MINUTES.map(text::keep_awake_group);
    data.wait_state(FOLLOW_WITHIN, "Keep awake 30 minutes", |s| {
        s.holding.len() == 1 && labels.contains(&s.holding[0])
    });
    data.wait_log(mark, "apply to plan", FOLLOW_WITHIN);
    let lines = data.log_since(mark);
    assert_eq!(
        count(&lines, "dry run: PowerSetRequest SYSTEM"),
        1,
        "{lines:#?}"
    );
    assert_eq!(count(&lines, "PowerSetRequest DISPLAY"), 0, "{lines:#?}");
    assert_eq!(count(&lines, "keep-awake: set until "), 1, "{lines:#?}");

    let mark = data.log_mark();
    let said = data.cli("keep-awake off");
    assert_eq!((said.code, said.out.as_str()), (0, "Keep awake is off.\n"));
    data.wait_state(FOLLOW_WITHIN, "idle after off", idle);
    data.wait_log(mark, "restore of every plan", FOLLOW_WITHIN);
    let lines = data.log_since(mark);
    assert_eq!(
        count(&lines, "dry run: PowerClearRequest SYSTEM"),
        1,
        "{lines:#?}"
    );
    assert!(!data.keep_awake().exists());

    let mark = data.log_mark();
    let said = data.cli("keep-awake on --screen");
    assert_eq!(
        (said.code, said.out.as_str()),
        (
            0,
            "Keep awake is on until you turn it off, and the screen stays on.\n"
        )
    );
    data.wait_state(FOLLOW_WITHIN, "Keep awake until off", keep_awake_only);
    data.wait_log(mark, "apply to plan", FOLLOW_WITHIN);
    let lines = data.log_since(mark);
    assert_eq!(
        count(&lines, "dry run: PowerSetRequest SYSTEM"),
        1,
        "{lines:#?}"
    );
    assert_eq!(
        count(&lines, "dry run: PowerSetRequest DISPLAY"),
        1,
        "{lines:#?}"
    );

    let mark = data.log_mark();
    assert_eq!(data.cli("keep-awake off").code, 0);
    data.wait_state(FOLLOW_WITHIN, "idle after off", idle);
    data.wait_log(mark, "restore of every plan", FOLLOW_WITHIN);
    let lines = data.log_since(mark);
    assert_eq!(
        count(&lines, "dry run: PowerClearRequest DISPLAY"),
        1,
        "{lines:#?}"
    );

    assert_eq!(tray.quit(), 0);
}

#[test]
#[ignore = "starts the dry-run tray, whose icon shows on the taskbar: screen slot only"]
fn real_e2e_run_holds_while_the_command_runs_and_passes_its_code() {
    if skip() {
        return;
    }
    let data = TempData::with_config("run");
    let mut tray = Tray::start(&data, Claude::Empty);

    let started = Instant::now();
    let wrapper = data.cli_spawn(r#"run -- cmd /d /c "ping -n 4 127.0.0.1 >nul""#);
    let pid = run_hold_pid(&data);
    println!("run-{pid}.json after {} ms", started.elapsed().as_millis());
    data.wait_state(FOLLOW_WITHIN, "holding for cmd", |s| holds(s, "cmd"));
    let output = wrapper.wait_with_output().expect("wait for wakeshot run");
    println!(
        "wakeshot run ended after {} ms",
        started.elapsed().as_millis()
    );
    assert_eq!(output.status.code(), Some(0));
    // The tray runs and Automatic is on: nothing to warn about.
    assert_eq!(String::from_utf8_lossy(&output.stderr), "");
    assert_eq!(
        data.hold_names(),
        Vec::<String>::new(),
        "the run hold is left"
    );
    data.wait_state(FOLLOW_WITHIN, "idle after the run", idle);

    let three = data.cli("run -- cmd /d /c exit 3");
    assert_eq!((three.code, three.err.as_str()), (3, ""));
    let log = data.log_since(LogMark::START);
    assert_eq!(count(&log, "run: cmd exited with 0"), 1, "{log:#?}");
    assert_eq!(count(&log, "run: cmd exited with 3"), 1, "{log:#?}");

    assert_eq!(tray.quit(), 0);
}

#[test]
#[ignore = "starts the dry-run tray, whose icon shows on the taskbar: screen slot only"]
fn real_e2e_run_wrapper_ended_by_pid_hold_follows_the_child() {
    if skip() {
        return;
    }
    let data = TempData::with_config("run-ended");
    let mut tray = Tray::start(&data, Claude::Empty);

    let mut wrapper = data.cli_spawn("run -- ping -n 120 127.0.0.1");
    let pid = run_hold_pid(&data);
    let ping = Started::open(pid, "ping.exe");
    data.wait_state(FOLLOW_WITHIN, "holding for ping", |s| holds(s, PING));

    // The hold names the child, not the wrapper: ending the wrapper leaves the work, and the hold, running.
    wrapper.kill().expect("end wakeshot run by pid");
    let _ = wrapper.wait();
    thread::sleep(Duration::from_secs(3));
    assert!(ping.running(), "ping ended with its wrapper");
    let state = data.state().expect("state.json");
    assert!(
        holds(&state, PING),
        "the hold dropped while ping runs: {state:?}"
    );
    assert_eq!(data.hold_names(), [format!("run-{pid}.json")]);

    // Nobody deletes the file now: the tray must see the process go.
    ping.end();
    let ended = Instant::now();
    data.wait_state(
        SAFETY_TICK + Duration::from_secs(5),
        "idle after ping ended",
        idle,
    );
    println!(
        "hold dropped {} ms after ping ended",
        ended.elapsed().as_millis()
    );
    wait_until(FOLLOW_WITHIN, "the tray deletes the Gone hold file", || {
        data.hold_names().is_empty().then_some(())
    });

    assert_eq!(tray.quit(), 0);
}

#[test]
#[ignore = "starts the dry-run tray, whose icon shows on the taskbar: screen slot only"]
fn real_e2e_automatic_off_drops_run_holds_not_keep_awake() {
    if skip() {
        return;
    }
    let data = TempData::with_config("automatic");
    let mut tray = Tray::start(&data, Claude::Empty);

    keep_awake_on(&data);
    let wrapper = data.cli_spawn("run -- ping -n 120 127.0.0.1");
    let ping = Started::open(run_hold_pid(&data), "ping.exe");
    data.wait_state(FOLLOW_WITHIN, "holding for ping and Keep awake", |s| {
        holds(s, PING) && holds(s, &until_off())
    });

    let said = data.cli("automatic off");
    assert_eq!((said.code, said.out.as_str()), (0, "Automatic is off.\n"));
    data.wait_state(FOLLOW_WITHIN, "Automatic off: Keep awake only", |s| {
        !s.automatic && keep_awake_only(s)
    });

    let said = data.cli("automatic on");
    assert_eq!((said.code, said.out.as_str()), (0, "Automatic is on.\n"));
    data.wait_state(FOLLOW_WITHIN, "Automatic on: ping again", |s| {
        s.automatic && holds(s, PING) && holds(s, &until_off())
    });

    // The wrapper sees its child end, deletes its hold and passes the code on.
    ping.end();
    let output = wrapper.wait_with_output().expect("wait for wakeshot run");
    assert_eq!(output.status.code(), Some(1), "ping ended by pid gives 1");
    data.wait_state(FOLLOW_WITHIN, "Keep awake only", keep_awake_only);
    assert_eq!(data.cli("keep-awake off").code, 0);
    data.wait_state(FOLLOW_WITHIN, "idle", idle);

    assert_eq!(tray.quit(), 0);
}

#[test]
#[ignore = "starts the dry-run tray, whose icon shows on the taskbar: screen slot only"]
fn real_e2e_status_while_running_and_after_quit() {
    if skip() {
        return;
    }
    let data = TempData::with_config("status");
    let mut tray = Tray::start(&data, Claude::Empty);

    let said = data.cli("status");
    assert_eq!((said.code, said.out.as_str()), (0, IDLE_STATUS));

    keep_awake_on(&data);
    let said = data.cli("status");
    assert_eq!(said.code, 0, "{said:?}");
    let want = format!("Holding awake: {}\n", until_off());
    assert!(said.out.contains(&want), "{said:?}");
    assert!(said.out.contains("Changed by Wakeshot: "), "{said:?}");

    assert_eq!(tray.quit(), 0);
    let said = data.cli("status");
    assert_eq!((said.code, said.out.as_str()), (0, NOT_RUNNING));
}

#[test]
#[ignore = "starts the dry-run tray, whose icon shows on the taskbar: screen slot only"]
fn real_e2e_restore_asks_the_running_tray() {
    if skip() {
        return;
    }
    let data = TempData::with_config("restore");
    let mut tray = Tray::start(&data, Claude::Empty);
    keep_awake_on(&data);

    let mark = data.log_mark();
    let started = Instant::now();
    let said = data.cli("restore");
    let took = started.elapsed();
    println!("wakeshot restore took {} ms", took.as_millis());
    assert_eq!((said.code, said.out.as_str()), (0, RESTORED), "{said:?}");
    assert!(took < Duration::from_secs(5), "{took:?}");
    assert!(
        !data.file("restore-request.json").exists(),
        "the request is left"
    );
    let line = data.wait_log(mark, "restore: owner pid", Duration::ZERO);
    assert!(
        line.ends_with(&format!(
            "owner pid {} alive, asked the tray: restored",
            tray.pid()
        )),
        "{line}"
    );
    // The user's restore keeps the requests: Keep awake still holds, with nothing overridden.
    let state = data.state().expect("state.json");
    assert!(
        keep_awake_only(&state) && state.overridden.is_empty(),
        "{state:?}"
    );

    assert_eq!(data.cli("keep-awake off").code, 0);
    assert_eq!(tray.quit(), 0);
}

#[test]
#[ignore = "starts the dry-run tray, whose icon shows on the taskbar: screen slot only"]
fn real_e2e_quit_during_keep_awake_cleans_up() {
    if skip() {
        return;
    }
    let data = TempData::with_config("quit");
    let mut tray = Tray::start(&data, Claude::Empty);
    keep_awake_on(&data);
    assert!(data.keep_awake().exists());
    assert!(data.file("journal.json").exists());

    let mark = data.log_mark();
    assert_eq!(tray.quit(), 0);
    assert!(
        !data.keep_awake().exists(),
        "the Keep awake hold file is left"
    );
    assert!(
        !data.file("journal.json").exists(),
        "the dry-run journal is left"
    );
    let lines = data.log_since(mark);
    assert_eq!(count(&lines, "Wakeshot ends with code 0"), 1, "{lines:#?}");
}

/// The class names of `pid`'s top-level windows, in z-order: the first is the one a plain `taskkill` closes.
fn top_windows_of(pid: u32) -> Vec<String> {
    unsafe extern "system" fn each(hwnd: HWND, found: LPARAM) -> BOOL {
        // SAFETY: `found` is the `Vec` below, alive for the whole `EnumWindows` call.
        let found = unsafe { &mut *(found.0 as *mut (u32, Vec<String>)) };
        let mut owner = 0u32;
        let mut name = [0u16; 128];
        // SAFETY: plain calls; `owner` and `name` are live locals of the sizes given.
        let len = unsafe {
            GetWindowThreadProcessId(hwnd, Some(&mut owner));
            GetClassNameW(hwnd, &mut name)
        };
        if owner == found.0 {
            let len = usize::try_from(len).unwrap_or(0);
            found.1.push(String::from_utf16_lossy(&name[..len]));
        }
        true.into()
    }
    let mut found = (pid, Vec::new());
    // SAFETY: the callback reads `found` only during the call.
    unsafe { EnumWindows(Some(each), LPARAM(&raw mut found as isize)) }.expect("EnumWindows");
    found.1
}

/// `taskkill /PID` without `/F` posts one `WM_CLOSE`, to the tray's first top-level window in z-order, which is seldom
/// the loop window (`docs/tasks/p6-polite-quit-notice-retry.md`). It must end in the same Quit: restore, the Keep
/// awake hold gone, exit 0.
#[test]
#[ignore = "starts the dry-run tray, whose icon shows on the taskbar: screen slot only"]
fn real_e2e_plain_taskkill_quits_with_the_restore() {
    if skip() {
        return;
    }
    let data = TempData::with_config("taskkill");
    let tray = Tray::start(&data, Claude::Empty);
    keep_awake_on(&data);
    let windows = top_windows_of(tray.pid());
    println!("the tray's top-level windows in z-order: {windows:?}");
    assert_ne!(
        windows.first().map(String::as_str),
        Some("Wakeshot.Loop.DryRun"),
        "the loop window is first: this run would not test the pass"
    );

    let mark = data.log_mark();
    let system = std::env::var("SystemRoot").expect("SystemRoot");
    let said = std::process::Command::new(format!(r"{system}\System32\taskkill.exe"))
        .args(["/PID", &tray.pid().to_string()])
        .output()
        .expect("run taskkill");
    let out = String::from_utf8_lossy(&said.stdout);
    println!("taskkill: {} {out}", said.status);
    assert!(said.status.success(), "{said:?}");
    let code = tray.exit_within(EXIT_WITHIN);
    assert_eq!(code, Some(0), "the tray did not quit after taskkill");

    assert!(
        !data.keep_awake().exists(),
        "the Keep awake hold file is left"
    );
    assert!(
        !data.file("journal.json").exists(),
        "the dry-run journal is left"
    );
    let lines = data.log_since(mark);
    assert_eq!(
        count(
            &lines,
            "restore of every plan: found journal entries; 0 entries left"
        ),
        1,
        "{lines:#?}"
    );
    assert_eq!(count(&lines, "Wakeshot ends with code 0"), 1, "{lines:#?}");
}

#[test]
#[ignore = "starts the dry-run tray, whose icon shows on the taskbar: screen slot only"]
fn real_e2e_ended_by_pid_during_a_hold_then_started_again() {
    if skip() {
        return;
    }
    let data = TempData::with_config("crash");
    let tray = Tray::start(&data, Claude::Empty);
    keep_awake_on(&data);
    tray.kill();
    // The dry run starts no guard, so what the crash left stays for the next start.
    assert!(
        data.file("journal.json").exists(),
        "no journal left by the crash"
    );
    assert!(
        data.keep_awake().exists(),
        "no Keep awake file left by the crash"
    );

    let mark = data.log_mark();
    let mut again = Tray::start(&data, Claude::Empty);
    // Tray startup step 6 runs before any thread starts, so the journal is gone by the first state write.
    assert!(
        !data.file("journal.json").exists(),
        "the journal is left after the start"
    );
    let lines = data.log_since(mark);
    assert!(lines[0].contains("starts: dry run"), "{lines:#?}");
    assert_eq!(
        count(&lines, "restore of every plan: found journal entries"),
        1,
        "the start does not say it restored the leftover: {lines:#?}"
    );
    // The Keep awake file names the tray that ended: Gone, so it holds nothing and goes.
    data.wait_state(FOLLOW_WITHIN, "idle after the start", idle);
    wait_until(
        FOLLOW_WITHIN,
        "the dead tray's Keep awake file deleted",
        || (!data.keep_awake().exists()).then_some(()),
    );

    assert_eq!(again.quit(), 0);
}

#[test]
#[ignore = "starts the dry-run tray, whose icon shows on the taskbar: screen slot only"]
fn real_e2e_log_lines_have_the_fields_and_no_file_content() {
    if skip() {
        return;
    }
    let data = TempData::with_config("log");
    let mut tray = Tray::start(&data, Claude::Empty);
    // A command line that must never reach the log (architecture section 8, "Never logged").
    let secret = format!("e2e-secret-{}", std::process::id());
    let said = data.cli(&format!(r#"run -- cmd /d /c "exit 0 & rem {secret}""#));
    assert_eq!(said.code, 0, "{said:?}");
    assert_eq!(data.cli("keep-awake 30m --screen").code, 0);
    data.wait_state(FOLLOW_WITHIN, "overridden by the dry-run apply", |s| {
        !s.overridden.is_empty()
    });
    assert_eq!(data.cli("status").code, 0);
    assert_eq!(data.cli("restore").code, 0);
    assert_eq!(data.cli("automatic off").code, 0);
    assert_eq!(data.cli("automatic on").code, 0);
    assert_eq!(data.cli("keep-awake off").code, 0);
    data.wait_state(FOLLOW_WITHIN, "idle", idle);
    assert_eq!(tray.quit(), 0);

    let log = data.log();
    assert!(log.ends_with("\r\n"), "the last line is cut");
    let mut problems = Vec::new();
    let mut seen = std::collections::BTreeMap::<(String, String), usize>::new();
    for (n, line) in log.lines().enumerate() {
        match parse_line(line) {
            Ok(parsed) => {
                *seen
                    .entry((parsed.process.to_owned(), parsed.level.to_owned()))
                    .or_default() += 1;
                if parsed.level == "ERROR" {
                    problems.push(format!("line {n}: an error in a plain session: {line}"));
                }
                if let Some(why) = not_plain(parsed.text) {
                    problems.push(format!("line {n}: {why}: {line}"));
                }
                if parsed.level == "WARN" {
                    println!("warning, line {n}, area {}: {}", parsed.area, parsed.text);
                }
            }
            Err(why) => problems.push(format!("line {n}: {why}: {line}")),
        }
        for never in [secret.as_str(), "exit 0 &", "/d /c"] {
            if line.contains(never) {
                problems.push(format!("line {n}: the command line is logged: {line}"));
            }
        }
    }
    println!("lines by process and level: {seen:?}");
    for process in ["tray", "cli"] {
        assert!(
            seen.keys().any(|(p, _)| p == process),
            "no {process} line: {seen:?}"
        );
    }
    assert!(problems.is_empty(), "{problems:#?}");
}

/// Not a check that passes on its own: it records what a Claude Code session does to a dry-run tray that watches the
/// real `~\.claude`, while a person or an agent runs a test session beside it (the card's Part B). Set
/// `WAKESHOT_E2E_CLAUDE_WATCH_S` to the seconds to watch; unset, it does nothing. Other Claude sessions on this PC
/// count too, so it prints every change of the holding list with its time, to read against the test session's own
/// times.
#[test]
#[ignore = "starts the dry-run tray and watches the real Claude folder: agent-session lane only"]
fn real_e2e_claude_session_timeline() {
    let Some(seconds) = std::env::var("WAKESHOT_E2E_CLAUDE_WATCH_S")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
    else {
        println!("skipped: set WAKESHOT_E2E_CLAUDE_WATCH_S to the seconds to watch");
        return;
    };
    if skip() {
        return;
    }
    let data = TempData::with_config("claude");
    let mut tray = Tray::start(&data, Claude::Home);
    let started = Instant::now();
    let mut last: Option<Vec<String>> = None;
    let mut claude_seen = false;
    while started.elapsed() < Duration::from_secs(seconds) {
        if let Some(state) = data.state()
            && last.as_ref() != Some(&state.holding)
        {
            println!(
                "{} UTC, {:>6.1} s: holding {:?}",
                format_expires(clock::now().wall_ms),
                started.elapsed().as_secs_f64(),
                state.holding
            );
            claude_seen |= state.holding.iter().any(|h| h.starts_with("Claude Code"));
            last = Some(state.holding);
        }
        thread::sleep(Duration::from_millis(200));
    }
    let code = tray.quit();
    assert!(claude_seen, "no Claude Code hold in {seconds} s");
    assert_eq!(code, 0);
}
