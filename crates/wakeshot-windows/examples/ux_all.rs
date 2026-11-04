//! Haik's check of the whole tray UX (`p5-haik-check`): the demo icon with the real menu, notices, chimes, the
//! first-run click and Open log in one run.
//!
//! `ux_all [--minutes N] [--logs <dir>]`
//!
//! Without a version resource, Windows heads each balloon's banner with the file name, `ux_all.exe`. For the tray
//! exe's header, "Wakeshot", link the same version info `crates/build-resources.rs` gives that exe: an `.rc` with its
//! `VERSIONINFO` compiled by `rc.exe`, then `cargo rustc -p wakeshot-windows --example ux_all -- -C
//! link-arg=<ux_all.res>`. Windows writes the name it read as `DisplayName` under
//! `HKCU\Software\Classes\AppUserModelId\NotifyIconGeneratedAumid_<id>`; delete that key after a run without the
//! resource, so the name is read again.
//!
//! Start it at Medium (Phase 5 track note 4). The menu is the product's, built by `present::build` from a sample
//! state, and a pick acts as the product would look: Automatic and Keep awake change the icon and the status line,
//! Restore shows its notice, Open log opens the sample log, Quit quits. Keys in the console window:
//!
//! - `n`: the next notice, every kind in turn; `f`: the first-run notice, whose click opens the taskbar settings.
//! - `1`, `2`, `3`: the lid-close, going-to-sleep and battery-floor chimes.
//! - `l`: lock the PC, then show a notice 10 s later.
//! - `a`: the sample Claude Code sessions stop or start working, so the idle icon shows with Automatic on.
//! - `q`, Ctrl+C or closing the console: quit.
//!
//! As in the product, notices and opens run on a thread of their own and the icon lives on the main thread. It
//! prints each event with the wall time. `--logs` is the folder of the sample log (default `ux-all-logs` next to the
//! exe); the chimes' log lines go there too. It quits after N minutes (default 60) or on `WM_CLOSE` (`taskkill /PID
//! <pid>` without `/F`), removing its icon either way. It changes no setting.

use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use wakeshot_core::event::Event;
use wakeshot_core::holds::HoldKind;
use wakeshot_core::present::{self, Holder, Snapshot};
use wakeshot_core::process::{FileTime, ProcessRef};
use wakeshot_core::settings::SettingKey;
use wakeshot_core::text::{self, NoticeClick};
use wakeshot_core::time::Now;
use wakeshot_core::view::{Cue, KeepAwake, MenuCommand, MenuItem, Notice, TrayView};
use wakeshot_windows::WinError;
use wakeshot_windows::clock;
use wakeshot_windows::log::{self, Level, Log, LogWriter, Proc};
use wakeshot_windows::notify::{BalloonNotifier, Notifier};
use wakeshot_windows::shell;
use wakeshot_windows::sink::EventSink;
use wakeshot_windows::sound::{SoundPlayer, WasapiPlayer};
use wakeshot_windows::tray::icon::{self, IconKind, TrayIcon};
use wakeshot_windows::tray::menu::OwnerDrawnMenu;
use windows::Win32::Foundation::{LPARAM, TRUE, WPARAM};
use windows::Win32::System::Console::{
    CTRL_BREAK_EVENT, CTRL_C_EVENT, GetStdHandle, INPUT_RECORD, KEY_EVENT, ReadConsoleInputW,
    STD_INPUT_HANDLE, SetConsoleCtrlHandler,
};
use windows::Win32::System::Shutdown::LockWorkStation;
use windows::Win32::System::SystemInformation::GetLocalTime;
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, MSG, PostThreadMessageW, SetTimer, TranslateMessage, WM_APP,
    WM_CLOSE, WM_TIMER,
};
use windows::core::BOOL;

const KEYS: &str = "keys: n next notice, f first-run notice, 1 2 3 chimes, l lock and a notice 10 s later, \
                    a agents stop or start, q quit";

/// The wake message can be lost while the menu is open, so this timer drains the inbox too, and keeps Keep awake's
/// minutes left current.
const PUMP_MS: u32 = 250;

/// The wait between the lock and its notice: long enough for the lock screen to be up.
const LOCK_NOTICE_WAIT: Duration = Duration::from_secs(10);

/// The sample agent: two Claude Code sessions, as in the product's menu sketch.
const AGENT: &str = "Claude Code";
const AGENT_SESSIONS: usize = 2;

const MINUTE_MS: i64 = 60_000;

fn now() -> String {
    // SAFETY: plain call.
    let t = unsafe { GetLocalTime() };
    format!(
        "{:02}:{:02}:{:02}.{:03}",
        t.wHour, t.wMinute, t.wSecond, t.wMilliseconds
    )
}

macro_rules! say {
    ($($arg:tt)*) => { println!("{} {}", now(), format!($($arg)*)) };
}

/// What reaches the main thread from the others.
enum Input {
    Key(char),
    Event(Event),
    Quit(&'static str),
}

/// What the notice thread does: the product's IO thread runs `Command::Notify` and opens the log the same way.
enum Job {
    Show(Notice),
    Open(OsString),
}

/// The main thread's inbox. It is the menu's and the player's event sink, as the loop window is in the product.
#[derive(Clone)]
struct Inbox {
    tx: Sender<Input>,
    thread: u32,
}

impl Inbox {
    fn send(&self, input: Input) {
        if self.tx.send(input).is_ok() {
            // A failed wake is fine: the pump timer drains the inbox anyway.
            // SAFETY: plain call; the message carries no pointer.
            let _ = unsafe { PostThreadMessageW(self.thread, WM_APP, WPARAM(0), LPARAM(0)) };
        }
    }
}

impl EventSink for Inbox {
    fn post(&self, event: Event) -> Result<(), WinError> {
        self.send(Input::Event(event));
        Ok(())
    }
}

/// For the console handler, which runs on a thread Windows makes and takes no context.
static INBOX: OnceLock<Inbox> = OnceLock::new();

/// Set once the icon is removed, so a console close waits for it.
static ICON_GONE: AtomicBool = AtomicBool::new(false);

/// Ctrl+C, Ctrl+Break and closing the console quit through the main loop, so the icon is removed: a killed demo
/// leaves a dead icon on the taskbar until the mouse passes over it.
unsafe extern "system" fn on_console_ctrl(kind: u32) -> BOOL {
    if let Some(inbox) = INBOX.get() {
        inbox.send(Input::Quit("console closed or Ctrl+C"));
    }
    if kind != CTRL_C_EVENT && kind != CTRL_BREAK_EVENT {
        // Windows ends the process as soon as this returns from a close, sign-out or shutdown, and allows about 5 s.
        let start = Instant::now();
        while !ICON_GONE.load(Ordering::Acquire) && start.elapsed() < Duration::from_secs(4) {
            std::thread::sleep(Duration::from_millis(50));
        }
    }
    TRUE
}

/// The chimes' log lines: printed, and written to the sample log, so Open log shows lines in the product's format.
struct TeeLog(Option<LogWriter>);

impl Log for TeeLog {
    fn line(&self, level: Level, area: &'static str, text: &str) {
        say!("log {level:?} {area}: {text}");
        if let Some(file) = &self.0 {
            file.line(level, area, text);
        }
    }
}

struct Args {
    minutes: u32,
    logs: Option<PathBuf>,
}

fn args() -> Result<Args, String> {
    let mut out = Args {
        minutes: 60,
        logs: None,
    };
    let mut it = std::env::args_os().skip(1);
    while let Some(arg) = it.next() {
        match arg.to_str() {
            Some("--minutes") => {
                out.minutes = it
                    .next()
                    .and_then(|n| n.to_str().and_then(|n| n.parse().ok()))
                    .ok_or("--minutes needs a whole number")?;
            }
            Some("--logs") => out.logs = Some(it.next().ok_or("--logs needs a folder")?.into()),
            _ => return Err(format!("unknown argument {}", arg.to_string_lossy())),
        }
    }
    Ok(out)
}

/// The name a notice prints with. No wildcard: a new notice does not compile here until it has a name and a sample.
fn name(notice: &Notice) -> &'static str {
    match notice {
        Notice::FirstRun => "first-run",
        Notice::UserChanged(_) => "user-changed",
        Notice::RestoredByUser => "restored-by-user",
        Notice::StepBack { .. } => "step-back",
        Notice::RequestsOnly => "requests-only",
        Notice::CannotHold => "cannot-hold",
        Notice::RestoreFailed => "restore-failed",
        Notice::HibernateFailed => "hibernate-failed",
        Notice::LockFailed => "lock-failed",
        Notice::ClosedLidReport { .. } => "closed-lid-report",
        Notice::UpdateWakeshot { .. } => "update-wakeshot",
        Notice::JournalLost { failed: true, .. } => "journal-lost-failed",
        Notice::JournalLost { .. } => "journal-lost",
        Notice::IdleLocked => "idle-locked",
        Notice::Updated { .. } => "updated",
    }
}

/// Every notice, with the sample data `ux_notices` uses; the journal loss twice, since its failed form has other
/// words and the same click.
fn samples() -> Vec<Notice> {
    vec![
        Notice::FirstRun,
        Notice::UserChanged(SettingKey::LidAc),
        Notice::RestoredByUser,
        Notice::StepBack { floor_pct: 20 },
        Notice::RequestsOnly,
        Notice::CannotHold,
        Notice::RestoreFailed,
        Notice::HibernateFailed,
        Notice::LockFailed,
        Notice::ClosedLidReport {
            ran_ms: 135 * 60_000,
            held_by: vec!["Claude Code".to_owned(), "cargo".to_owned()],
        },
        Notice::UpdateWakeshot {
            agent: "Claude Code",
        },
        Notice::JournalLost {
            reset: vec![SettingKey::LidAc, SettingKey::LidDc, SettingKey::SleepDc],
            failed: false,
        },
        Notice::JournalLost {
            reset: vec![SettingKey::LidAc, SettingKey::LidDc, SettingKey::SleepDc],
            failed: true,
        },
        Notice::IdleLocked,
        Notice::Updated {
            version: "1.1.0".to_owned(),
        },
    ]
}

/// Prints a notice's words and behavior, then hands it to the notice thread.
fn send_notice(jobs: &Sender<Job>, notice: Notice, why: &str) {
    let t = text::notice(&notice);
    say!(
        "{why}: notice {} {:?}, warning {} silent {} click {:?}",
        name(&notice),
        t.title,
        t.warning,
        t.silent,
        t.click
    );
    let _ = jobs.send(Job::Show(notice));
}

/// The notice thread: one job at a time, in order, as the product's IO thread.
fn run_jobs(jobs: Receiver<Job>, notifier: Box<dyn Notifier>) {
    for job in jobs {
        match job {
            Job::Show(notice) => {
                let result = notifier.show(&notice);
                say!("notice {} shown: {result:?}", name(&notice));
            }
            Job::Open(target) => {
                let result = shell::open_with_default_app(&target);
                say!("open {}: {result:?}", target.to_string_lossy());
            }
        }
    }
}

/// The console thread: each key press, as typed. `ReadConsoleInputW` gives one key at a time, with no Enter.
fn read_keys(inbox: Inbox) {
    // SAFETY: plain call.
    let input = match unsafe { GetStdHandle(STD_INPUT_HANDLE) } {
        Ok(handle) => handle,
        Err(e) => {
            say!("console input: {e}; keys are off, the menu still works");
            return;
        }
    };
    let mut records = [INPUT_RECORD::default(); 16];
    loop {
        let mut read = 0u32;
        // SAFETY: `records` and `read` are live locals; the handle is this process's standard input.
        if let Err(e) = unsafe { ReadConsoleInputW(input, &mut records, &mut read) } {
            say!("console input: {e}; keys are off, the menu still works");
            return;
        }
        for record in records.iter().take(read as usize) {
            if u32::from(record.EventType) != KEY_EVENT {
                continue;
            }
            // SAFETY: `EventType` says the union holds a key event.
            let key = unsafe { record.Event.KeyEvent };
            if !key.bKeyDown.as_bool() {
                continue;
            }
            // SAFETY: the W call fills the UTF-16 member.
            let unit = unsafe { key.uChar.UnicodeChar };
            // Shift, arrows and the like come with no character.
            if let Some(c) = char::from_u32(u32::from(unit)).filter(|c| !c.is_control()) {
                inbox.send(Input::Key(c.to_ascii_lowercase()));
            }
        }
    }
}

/// The sample state the menu shows and changes. No Windows call, so the tests check what each of Haik's steps shows.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Sample {
    automatic: bool,
    /// The sample Claude Code sessions are working (key `a`).
    agents: bool,
    keep_awake: KeepAwake,
    keep_awake_ends: Option<i64>,
    screen_on: bool,
}

impl Sample {
    /// The product's menu sketch: Automatic on, Claude Code working, Keep awake off.
    const START: Sample = Sample {
        automatic: true,
        agents: true,
        keep_awake: KeepAwake::Off,
        keep_awake_ends: None,
        screen_on: false,
    };

    fn snapshot(&self) -> Snapshot {
        let mut holds = Vec::new();
        // Automatic watches the agents; with it off they hold nothing (product, "Automatic").
        if self.automatic && self.agents {
            holds.extend(std::iter::repeat_n(
                Holder {
                    kind: HoldKind::Agent,
                    label: Some(AGENT.to_owned()),
                    expires_wall_ms: None,
                },
                AGENT_SESSIONS,
            ));
        }
        if self.keep_awake != KeepAwake::Off {
            holds.push(Holder {
                kind: HoldKind::KeepAwake,
                label: None,
                expires_wall_ms: self.keep_awake_ends,
            });
        }
        Snapshot {
            automatic: self.automatic,
            requests_on: !holds.is_empty(),
            holds,
            keep_awake: self.keep_awake,
            screen_on: self.screen_on,
            requests_only: false,
            stepped_back: false,
            overridden: Vec::new(),
            tray: ProcessRef {
                pid: std::process::id(),
                start: FileTime(0),
            },
        }
    }

    fn view(&self, now: Now) -> TrayView {
        present::build(&self.snapshot(), now).0
    }

    /// The picks that change what the menu shows; the demo runs the others itself.
    fn pick(&mut self, command: MenuCommand, now: Now) {
        match command {
            MenuCommand::Automatic(on) => self.automatic = on,
            MenuCommand::KeepAwake(choice) => self.set_keep_awake(choice, now),
            MenuCommand::KeepScreenOn(on) => self.screen_on = on,
            MenuCommand::Restore | MenuCommand::OpenLog | MenuCommand::Quit => {}
        }
    }

    /// Ends a Keep awake whose time is up; true when it did.
    fn expire(&mut self, now: Now) -> bool {
        let over = self.keep_awake_ends.is_some_and(|end| now.wall_ms >= end);
        if over {
            self.set_keep_awake(KeepAwake::Off, now);
        }
        over
    }

    fn set_keep_awake(&mut self, choice: KeepAwake, now: Now) {
        self.keep_awake = choice;
        self.keep_awake_ends = match choice {
            KeepAwake::Minutes(minutes) => {
                Some(now.wall_ms.saturating_add(i64::from(minutes) * MINUTE_MS))
            }
            KeepAwake::Off | KeepAwake::UntilOff => None,
        };
        // Keep screen on lives in the Keep awake file, which goes when Keep awake ends.
        if choice == KeepAwake::Off {
            self.screen_on = false;
        }
    }
}

/// The main thread's part: the icon, the player, and the sample behind the menu.
struct Demo {
    icon: TrayIcon,
    player: WasapiPlayer,
    jobs: Sender<Job>,
    log_file: PathBuf,
    sample: Sample,
    shown: Option<TrayView>,
    notices: Vec<Notice>,
    next_notice: usize,
}

impl Demo {
    /// Shows the view if it changed, as the core sends `Tray` only on a change.
    fn refresh(&mut self) {
        let now = clock::now();
        if self.sample.expire(now) {
            say!("Keep awake ran out");
        }
        let view = self.sample.view(now);
        if self.shown.as_ref() == Some(&view) {
            return;
        }
        let line = match view.menu.items.first() {
            Some(MenuItem::Label(line)) => line.as_str(),
            _ => "",
        };
        say!("shown: icon {:?}, status line {line:?}", view.icon);
        self.icon.show(&view);
        self.shown = Some(view);
    }

    /// False: quit.
    fn on_input(&mut self, input: Input) -> bool {
        match input {
            Input::Key(key) => self.on_key(key),
            Input::Event(Event::Menu(command)) => {
                say!("menu pick: {command:?}");
                self.on_pick(command)
            }
            Input::Event(event) => {
                say!("event: {event:?}");
                true
            }
            Input::Quit(why) => {
                say!("{why}: quitting");
                false
            }
        }
    }

    fn on_pick(&mut self, command: MenuCommand) -> bool {
        self.sample.pick(command, clock::now());
        match command {
            MenuCommand::Restore => send_notice(&self.jobs, Notice::RestoredByUser, "Restore"),
            MenuCommand::OpenLog => {
                let _ = self.jobs.send(Job::Open(self.log_file.clone().into()));
            }
            MenuCommand::Quit => return false,
            MenuCommand::Automatic(_)
            | MenuCommand::KeepAwake(_)
            | MenuCommand::KeepScreenOn(_) => {}
        }
        true
    }

    fn on_key(&mut self, key: char) -> bool {
        say!("key {key:?}");
        match key {
            'n' => {
                let i = self.next_notice % self.notices.len();
                self.next_notice = i + 1;
                let why = format!("{} of {}", i + 1, self.notices.len());
                send_notice(&self.jobs, self.notices[i].clone(), &why);
            }
            'f' => send_notice(&self.jobs, Notice::FirstRun, "key f"),
            '1' => self.player.play(Cue::LidClosedHolding),
            '2' => self.player.play(Cue::GoingToSleep),
            '3' => self.player.play(Cue::BatteryFloor),
            'l' => self.lock(),
            'a' => {
                self.sample.agents = !self.sample.agents;
                say!("sample agent sessions working: {}", self.sample.agents);
            }
            'q' => return false,
            _ => say!("{KEYS}"),
        }
        true
    }

    fn lock(&self) {
        // SAFETY: plain call.
        match unsafe { LockWorkStation() } {
            Ok(()) => {
                say!(
                    "lock: Windows asked to lock; notice in {} s",
                    LOCK_NOTICE_WAIT.as_secs()
                );
                let jobs = self.jobs.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(LOCK_NOTICE_WAIT);
                    // The notice the product shows while the PC is locked: Keep screen on locked it.
                    send_notice(&jobs, Notice::IdleLocked, "lock");
                });
            }
            Err(e) => say!("lock: LockWorkStation failed: {e}"),
        }
    }
}

/// The sample log next to the exe, so nothing is left outside the build folder.
fn logs_dir(given: Option<PathBuf>) -> PathBuf {
    given.unwrap_or_else(|| {
        std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(|dir| dir.join("ux-all-logs")))
            .unwrap_or_else(|| PathBuf::from("ux-all-logs"))
    })
}

fn main() {
    let args = match args() {
        Ok(args) => args,
        Err(e) => {
            eprintln!("{e}\nusage: ux_all [--minutes N] [--logs <dir>]");
            std::process::exit(2);
        }
    };
    // The product exe gets this from its manifest; without it the menu, the icon size and the anchors would be scaled.
    // SAFETY: plain call, before any window exists.
    if let Err(e) =
        unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) }
    {
        say!("SetProcessDpiAwarenessContext failed: {e}");
    }

    let (tx, inbox_rx) = mpsc::channel();
    // SAFETY: plain call.
    let thread = unsafe { GetCurrentThreadId() };
    let inbox = Inbox { tx, thread };
    let _ = INBOX.set(inbox.clone());
    // SAFETY: the handler is a plain function that lives as long as the process.
    if let Err(e) = unsafe { SetConsoleCtrlHandler(Some(on_console_ctrl), true) } {
        say!("SetConsoleCtrlHandler failed: {e}; Ctrl+C leaves a dead icon");
    }

    let logs = logs_dir(args.logs);
    let writer = match LogWriter::open(&logs, Proc::Tray) {
        Ok(writer) => Some(writer),
        Err(e) => {
            say!("sample log not opened: {e:?}; Open log will fail");
            None
        }
    };
    let tee = Arc::new(TeeLog(writer));
    tee.line(
        Level::Info,
        "demo",
        &format!(
            "ux_all started (pid {}). This is a sample log for the Open log check.",
            std::process::id()
        ),
    );

    let mut menu = match OwnerDrawnMenu::new(Arc::new(inbox.clone())) {
        Ok(menu) => menu,
        Err(e) => {
            eprintln!("OwnerDrawnMenu::new failed: {e:?}");
            std::process::exit(1);
        }
    };
    menu.on_note(Box::new(|note| say!("menu: {note:?}")));
    let mut tray = match TrayIcon::new(IconKind::Demo, Box::new(menu)) {
        Ok(tray) => tray,
        Err(e) => {
            eprintln!("TrayIcon::new failed: {e:?}");
            std::process::exit(1);
        }
    };
    let id = tray.identity();
    let notifier = Arc::new(BalloonNotifier::new(id));
    let (jobs, jobs_rx) = mpsc::channel();
    {
        let notifier = Arc::clone(&notifier);
        let jobs = jobs.clone();
        tray.on_note(Box::new(move |note| {
            say!("icon: {note:?}");
            // As the product: a notice refused while the taskbar was gone shows once the icon is back.
            if let icon::Note::Added { .. } = note {
                for kept in notifier.icon_added() {
                    send_notice(&jobs, kept, "kept while the icon was gone");
                }
            }
        }));
    }
    {
        let notifier = Arc::clone(&notifier);
        let jobs = jobs.clone();
        let log_file = log::log_file(&logs);
        tray.on_balloon_click(Box::new(move || {
            let click = notifier.take_click();
            say!("balloon click: {click:?}");
            let target: Option<OsString> = match click {
                NoticeClick::Nothing => None,
                NoticeClick::OpenTaskbarSettings => Some("ms-settings:taskbar".into()),
                NoticeClick::OpenLog => Some(log_file.clone().into()),
            };
            if let Some(target) = target {
                let _ = jobs.send(Job::Open(target));
            }
        }));
    }
    say!(
        "pid {} window {:#x} guid {:032x} log {}",
        std::process::id(),
        id.hwnd,
        id.guid,
        log::log_file(&logs).display()
    );

    let player = WasapiPlayer::new(Arc::new(inbox.clone()), tee);
    let mut demo = Demo {
        icon: tray,
        player,
        jobs,
        log_file: log::log_file(&logs),
        sample: Sample::START,
        shown: None,
        notices: samples(),
        next_notice: 0,
    };
    // The first show adds the icon, which the first notice needs.
    demo.refresh();

    let shower: Box<dyn Notifier> = Box::new(Arc::clone(&notifier));
    std::thread::spawn(move || run_jobs(jobs_rx, shower));
    std::thread::spawn(move || read_keys(inbox));
    say!("{KEYS}");

    // Thread timers: their WM_TIMER comes with no window and is read here.
    // SAFETY: plain calls; no window and no timer procedure.
    let end = unsafe { SetTimer(None, 0, args.minutes.saturating_mul(60_000), None) };
    // SAFETY: as above.
    let pump = unsafe { SetTimer(None, 0, PUMP_MS, None) };
    let mut msg = MSG::default();
    // SAFETY: `msg` is a valid out pointer.
    'pump: while unsafe { GetMessageW(&mut msg, None, 0, 0) }.0 > 0 {
        if msg.message == WM_CLOSE {
            say!("WM_CLOSE: quitting");
            break;
        }
        if msg.hwnd.is_invalid() {
            if msg.message == WM_TIMER && msg.wParam.0 == end {
                say!("time is up");
                break;
            }
            if msg.message == WM_APP || (msg.message == WM_TIMER && msg.wParam.0 == pump) {
                while let Ok(input) = inbox_rx.try_recv() {
                    if !demo.on_input(input) {
                        break 'pump;
                    }
                }
                demo.refresh();
                continue;
            }
        }
        // SAFETY: `msg` came from `GetMessageW` just above.
        unsafe {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
    drop(demo);
    ICON_GONE.store(true, Ordering::Release);
    say!("icon removed");
}

/// What Haik's steps say each pick shows (`cargo test --example ux_all`). The words come from `text`, never literals.
#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use wakeshot_core::text::Status;
    use wakeshot_core::view::IconState;

    use super::*;

    const NOW: Now = Now {
        wall_ms: 1_767_225_600_000,
        mono_ms: 42,
    };

    fn status(icon: IconState, screen_on: bool) -> Status {
        Status {
            icon,
            stepped_back: false,
            requests_only: false,
            screen_on,
        }
    }

    /// The icon state and the menu's status line.
    fn shows(sample: &Sample, now: Now) -> (IconState, String) {
        let view = sample.view(now);
        let Some(MenuItem::Label(line)) = view.menu.items.first() else {
            panic!("no status line: {view:?}");
        };
        (view.icon, line.clone())
    }

    fn agents() -> String {
        text::hold_group(HoldKind::Agent, AGENT, 2)
    }

    #[test]
    fn starts_holding_for_the_sample_agent() {
        let line = text::status_line(&status(IconState::Holding, false), &[agents()]);
        assert_eq!(shows(&Sample::START, NOW), (IconState::Holding, line));
    }

    #[test]
    fn agents_stopping_shows_idle() {
        let mut sample = Sample::START;
        sample.agents = false;
        let line = text::status_line(&status(IconState::Idle, false), &[]);
        assert_eq!(shows(&sample, NOW), (IconState::Idle, line));
    }

    #[test]
    fn automatic_off_shows_off_with_the_agents_working() {
        let mut sample = Sample::START;
        sample.pick(MenuCommand::Automatic(false), NOW);
        let line = text::status_line(&status(IconState::Off, false), &[]);
        assert_eq!(shows(&sample, NOW), (IconState::Off, line));
        sample.pick(MenuCommand::Automatic(true), NOW);
        assert_eq!(sample, Sample::START);
    }

    #[test]
    fn keep_awake_holds_with_automatic_off_until_it_runs_out() {
        let mut sample = Sample::START;
        sample.pick(MenuCommand::Automatic(false), NOW);
        sample.pick(MenuCommand::KeepAwake(KeepAwake::Minutes(30)), NOW);
        let group = text::keep_awake_group(KeepAwake::Minutes(30));
        let line = text::status_line(&status(IconState::Holding, false), &[group]);
        assert_eq!(shows(&sample, NOW), (IconState::Holding, line));

        let almost = Now {
            wall_ms: NOW.wall_ms + 30 * MINUTE_MS - 1,
            ..NOW
        };
        assert!(!sample.expire(almost));
        let over = Now {
            wall_ms: NOW.wall_ms + 30 * MINUTE_MS,
            ..NOW
        };
        assert!(sample.expire(over));
        assert_eq!(sample.keep_awake, KeepAwake::Off);
        assert_eq!(shows(&sample, over).0, IconState::Off);
    }

    #[test]
    fn keep_screen_on_shows_and_ends_with_keep_awake() {
        let mut sample = Sample::START;
        sample.pick(MenuCommand::KeepAwake(KeepAwake::UntilOff), NOW);
        sample.pick(MenuCommand::KeepScreenOn(true), NOW);
        let groups = [agents(), text::keep_awake_group(KeepAwake::UntilOff)];
        let line = text::status_line(&status(IconState::Holding, true), &groups);
        assert_eq!(shows(&sample, NOW), (IconState::Holding, line));
        assert!(!sample.expire(NOW), "Until I turn it off never runs out");

        sample.pick(MenuCommand::KeepAwake(KeepAwake::Off), NOW);
        assert_eq!(sample, Sample::START);
    }

    #[test]
    fn restore_open_log_and_quit_leave_the_sample_alone() {
        let mut sample = Sample::START;
        for command in [
            MenuCommand::Restore,
            MenuCommand::OpenLog,
            MenuCommand::Quit,
        ] {
            sample.pick(command, NOW);
        }
        assert_eq!(sample, Sample::START);
    }

    #[test]
    fn every_sample_notice_has_its_own_name() {
        let all = samples();
        let names: HashSet<&str> = all.iter().map(name).collect();
        assert_eq!(names.len(), all.len());
    }
}
