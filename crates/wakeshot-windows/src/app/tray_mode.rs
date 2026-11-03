//! The composition root: the real objects, thread starts, startup and shutdown order, `RealRunners`.
//!
//! Owner: `p6-int-tray-mode`, then `p6-int-update-wiring`; fixes by the e2e cards.
//!
//! `run` follows `docs/phases/06-integration.md` note 6, steps 3 to 9, with three changes of order inside step 8, each
//! for a reason the note did not know:
//!
//! - The watch thread starts before `Dispatcher::new`, so its handle can go into `RealRunners`. Its first posts wait
//!   in the loop window's queue until the pump runs, and `Dispatcher::new` feeds `Started` to the core directly, so
//!   the core still sees `Started` first.
//! - The IO thread starts right after `Dispatcher::new`. The core gives the first-run `Notify` before the first `Tray`
//!   in the `Started` batch; the loop runs that `Tray` (the icon's add) inside `Dispatcher::new`, so the notice only
//!   reaches the shell once the icon exists. A running IO thread could show it first, and it would fail for good.
//! - The session notifications go when the dispatcher quits, inside the pump: `LoopWindow::run` destroys the window
//!   before it returns, and Microsoft asks for the unregister before the window is destroyed.

use std::cell::{Cell, RefCell};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::rc::Rc;
use std::sync::Arc;
use std::thread::{self, JoinHandle};

use wakeshot_core::command::{Command, CountdownReason, Failure, HibernateEnd, Outcome};
use wakeshot_core::core::Core;
use wakeshot_core::event::{Event, SessionChange};
use wakeshot_core::ownership::io::PlanIo;
use wakeshot_core::sources::AgentSource;
use wakeshot_core::sources::claude::{ClaudeSource, claude_folder};
use wakeshot_core::text::NoticeClick;
use wakeshot_core::view::{Cue, MenuCommand, TrayView};
use windows::Win32::Foundation::{ERROR_ACCESS_DENIED, ERROR_NOT_SUPPORTED, HWND};

use super::dispatch::{Dispatcher, LoopRunners};
use super::io_thread::{FINISH_MS, IoParts, IoThread};
use super::startup::{self, Boot};
use super::window::LoopWindow;
use super::{Ack, Flow, LoopHandler};
use crate::WinError;
use crate::clock::{self, SystemClock};
use crate::io_queue::IoQueue;
use crate::localtime::SystemZone;
use crate::log::{Level, Log, LogWriter, Proc};
use crate::notify::BalloonNotifier;
use crate::power::executor::PowerExecutor;
use crate::power::hibernate::{DryRunHibernate, HibernateDeps, Win32Hibernate, run_hibernate};
use crate::power::journal::{FileNet, GuardControl};
use crate::power::plan::{DryRunPower, Win32Power};
use crate::power::requests::{DryRunRequests, PowerRequests, RequestApi};
use crate::power::system::{self, Subscriptions};
use crate::process::{Liveness, SystemLiveness};
use crate::recovery::TrayGuard;
use crate::session::{self, DryRunLocker, Locker, SessionNotifications, Win32Locker};
use crate::shell;
use crate::sink::EventSink;
use crate::sound::{SoundPlayer, WasapiPlayer};
use crate::store::{AutoStart, HeldMutex, NamedMutex};
use crate::tray::icon::{self, IconKind, TrayIcon};
use crate::tray::menu::{self, OwnerDrawnMenu};
use crate::watch::thread::{WatchHandle, WatchThread};

const AREA: &str = "tray";

/// The exit code of a tray that could not start: the same 1 the guard and restore mode give after a give-up.
const EXIT_FAILED: u8 = 1;

/// `log::LOG_FILE`, private there; a balloon click opens it.
const LOG_FILE: &str = "wakeshot.log";

/// What the first-run notice's click opens (track note 3.6 of `05-tray-ux.md`).
const TASKBAR_SETTINGS: &str = "ms-settings:taskbar";

/// Claude Code's own variable for its folder (`docs/tasks/p4-live-claude.md`, m6).
const CLAUDE_CONFIG_DIR: &str = "CLAUDE_CONFIG_DIR";

/// How long a lost-event reading waits for the lid on the loop thread. Its first callback came within 1 ms in every
/// test (research, "Lid notification"), and a PC with no lid answers nothing, so the wait must stay short.
const LID_READ_MS: u64 = 200;

/// The answer to `ApplyUpdate` until `p6-int-update-wiring` starts the update thread. 50 is `ERROR_NOT_SUPPORTED`.
const UPDATE_NOT_WIRED: Failure = Failure {
    call: "no update thread",
    code: ERROR_NOT_SUPPORTED.0,
};

/// Runs tray mode, or dry-run mode when `dry_run`, until Quit: tray startup steps 3 to 9
/// (`docs/phases/06-integration.md` note 6). Steps 1 and 2, Velopack and the mode, are the tray `main`'s.
pub fn run(dry_run: bool) -> ExitCode {
    // Step 3. Without a data folder there is no log to tell.
    let Ok(boot) = Boot::new(dry_run) else {
        return ExitCode::from(EXIT_FAILED);
    };
    // Step 4.
    let tray_lock = match startup::take_tray(&boot) {
        Ok(Some(lock)) => lock,
        Ok(None) => return ExitCode::SUCCESS,
        Err(e) => {
            // No roll: another tray may be starting and rolling the log right now.
            if let Ok(log) = LogWriter::open(&boot.dir.logs(), Proc::Tray) {
                let text = format!(
                    "tray mutex not taken: {} failed with {}; the tray stops",
                    e.call, e.code
                );
                log.line(Level::Error, AREA, &text);
            }
            return ExitCode::from(EXIT_FAILED);
        }
    };
    // Step 5.
    let log = startup::open_log(&boot);
    let mode = if dry_run { "dry run" } else { "tray" };
    let text = format!(
        "Wakeshot {} starts: {mode}, pid {}",
        env!("CARGO_PKG_VERSION"),
        boot.tray.pid
    );
    log.line(Level::Info, AREA, &text);
    if tray_lock.abandoned() {
        log.line(
            Level::Info,
            AREA,
            "the last tray of this session ended without quitting",
        );
    }
    startup::install_panic_hook(Arc::clone(&log));
    let liveness: Arc<dyn Liveness> = Arc::new(SystemLiveness);
    let code = if dry_run {
        dry_run_tray(&boot, &log, liveness)
    } else {
        real_tray(&boot, &log, liveness)
    };
    log.line(
        Level::Info,
        AREA,
        &format!("Wakeshot ends with code {code}"),
    );
    drop(tray_lock);
    ExitCode::from(code)
}

/// Every Windows object that writes, locks or hibernates comes in a real and a dry-run form (architecture section 4,
/// "Tray startup" step 3).
struct Picks {
    requests: Box<dyn RequestApi>,
    locker: Box<dyn Locker>,
    hibernate: Arc<dyn HibernateDeps>,
    icon: IconKind,
}

impl Picks {
    fn real() -> Self {
        Self {
            requests: Box::new(PowerRequests::new()),
            locker: Box::new(Win32Locker),
            hibernate: Arc::new(Win32Hibernate),
            icon: IconKind::Product,
        }
    }

    fn dry_run(log: &Arc<dyn Log>) -> Self {
        Self {
            requests: Box::new(DryRunRequests::new(Arc::clone(log))),
            locker: Box::new(DryRunLocker::new(Arc::clone(log))),
            hibernate: Arc::new(DryRunHibernate::new(Arc::clone(log))),
            icon: IconKind::DryRun,
        }
    }
}

/// The real executor: plan writes, RunOnce and a guard.
fn real_tray(boot: &Boot, log: &Arc<dyn Log>, liveness: Arc<dyn Liveness>) -> u8 {
    let exe = match std::env::current_exe() {
        Ok(exe) => exe,
        Err(e) => {
            let text = format!(
                "the tray exe path does not read ({e}), so no guard or RunOnce value could name it; the tray stops"
            );
            log.line(Level::Error, AREA, &text);
            return EXIT_FAILED;
        }
    };
    let guard = TrayGuard::new(exe.clone(), boot.tray, Arc::clone(log));
    let net = FileNet::tray(
        boot.dir.clone(),
        AutoStart::real(),
        &exe,
        guard,
        Arc::clone(log),
    );
    let executor = PowerExecutor::new(
        Win32Power::new(Arc::clone(log)),
        net,
        boot.journal_owner(),
        boot.names.clone(),
        boot.dir.clone(),
        Arc::clone(&liveness),
        Arc::clone(log),
    );
    serve(boot, log, liveness, executor, Picks::real())
}

/// The dry run: real reads, logged writes, its own journal, no RunOnce and no guard.
fn dry_run_tray(boot: &Boot, log: &Arc<dyn Log>, liveness: Arc<dyn Liveness>) -> u8 {
    let executor = PowerExecutor::new(
        DryRunPower::new(Win32Power::new(Arc::clone(log)), Arc::clone(log)),
        FileNet::dry_run(boot.dir.clone(), Arc::clone(log)),
        boot.journal_owner(),
        boot.names.clone(),
        boot.dir.clone(),
        Arc::clone(&liveness),
        Arc::clone(log),
    );
    serve(boot, log, liveness, executor, Picks::dry_run(log))
}

/// Steps 6 to 9 for either executor. Returns the exit code.
fn serve<P, G>(
    boot: &Boot,
    log: &Arc<dyn Log>,
    liveness: Arc<dyn Liveness>,
    mut executor: PowerExecutor<P, FileNet<G>>,
    picks: Picks,
) -> u8
where
    P: PlanIo + Send + 'static,
    G: GuardControl + Send + 'static,
{
    // Step 6, before any other thread starts.
    let restored = startup::startup_restore(&mut executor, &**log);
    // Step 7.
    let mut settings = Settings::new(boot.names.settings.clone());
    if let Some((level, text)) = settings.try_take() {
        log.line(level, AREA, &text);
    }
    let (config, first_run) = startup::read_config(boot, &**log);
    startup::remove_stale_temps(boot, &*liveness, &**log);

    // Step 8.
    let window = match LoopWindow::new(boot.dry_run) {
        Ok(window) => window,
        Err(e) => return fatal(&**log, "the loop window", e),
    };
    let sink: Arc<dyn EventSink> = window.sink();
    let io = Arc::new(IoQueue::new());
    let loop_log: Arc<dyn Log> = Arc::new(QueuedLog(Arc::clone(&io)));
    let mut menu = match OwnerDrawnMenu::new(Arc::clone(&sink)) {
        Ok(menu) => menu,
        Err(e) => return fatal(&**log, "the menu window", e),
    };
    menu.on_note(menu_notes(Arc::clone(&loop_log)));
    let mut icon = match TrayIcon::new(picks.icon, Box::new(menu)) {
        Ok(icon) => icon,
        Err(e) => return fatal(&**log, "the tray icon window", e),
    };
    icon.on_note(icon_notes(Arc::clone(&loop_log)));
    let notifier = Arc::new(BalloonNotifier::new(icon.identity()));
    icon.on_balloon_click(balloon_click(
        Arc::clone(&notifier),
        boot.dir.logs().join(LOG_FILE),
        Arc::clone(log),
        Arc::clone(&loop_log),
    ));
    let sound: Arc<dyn SoundPlayer> =
        Arc::new(WasapiPlayer::new(Arc::clone(&sink), Arc::clone(log)));
    let session = Rc::new(SessionSlot::register(window.hwnd(), &**log));
    let subscriptions = Subscriptions::start(Arc::clone(&sink));
    for e in subscriptions.failures() {
        let text = format!(
            "a power notification is not registered: {} failed with {}",
            e.call, e.code
        );
        log.line(Level::Warn, AREA, &text);
    }
    let watch = match WatchThread::start(
        boot.dir.clone(),
        config.clone(),
        sources(&**log),
        Arc::clone(&liveness),
        Arc::clone(&sink),
        Arc::clone(log),
    ) {
        Ok(watch) => watch,
        Err(e) => return fatal(&**log, "the watch thread", e),
    };
    let owner = settings.held();
    let mut first = vec![startup::started(
        boot,
        config.clone(),
        first_run,
        &restored,
        owner,
        &**log,
    )];
    if startup::locked_at_start(&**log) {
        first.push(Event::Session(SessionChange::Locked));
    }
    if restored.corrupt {
        first.push(Event::JournalCorrupt);
    }
    let ack = Ack::new();
    let runners = RealRunners {
        requests: picks.requests,
        locker: picks.locker,
        icon,
        sound: Arc::clone(&sound),
        hibernator: Hibernator {
            deps: picks.hibernate,
            sound,
            sink: Arc::clone(&sink),
            log: Arc::clone(log),
        },
        settings,
        watch,
        subscriptions,
        lost: LostPosts::default(),
        session: Rc::clone(&session),
        sink: Arc::clone(&sink),
        log: Arc::clone(&loop_log),
        idle_failed: false,
    };
    let dispatcher = Dispatcher::new(
        Core::new(config),
        Box::new(SystemClock),
        Arc::clone(&io),
        runners,
        ack.clone(),
        first,
    );
    let io_thread = match IoThread::start(IoParts {
        queue: Arc::clone(&io),
        power: Box::new(executor),
        dir: boot.dir.clone(),
        notifier: Box::new(Arc::clone(&notifier)),
        open: Box::new(shell::open_with_default_app),
        log: Arc::clone(log),
        sink,
        ack,
    }) {
        Ok(thread) => thread,
        Err(e) => {
            // Nothing was applied: applies run only on that thread. The rest goes in the shutdown order.
            let code = fatal(&**log, "the IO thread", e);
            dispatcher.into_runners().shut_down();
            return code;
        }
    };

    let handler = TrayLoop {
        dispatcher: &dispatcher,
        session: &session,
        log: &*loop_log,
    };
    let mut code = 0;
    if let Err(e) = window.run(&handler) {
        let text = format!(
            "the message loop failed: {} failed with {}; the tray quits",
            e.call, e.code
        );
        log.line(Level::Error, AREA, &text);
        // The core restores and lets the requests go, as for Quit from the menu; the IO thread still runs.
        dispatcher.on_event(Event::Menu(MenuCommand::Quit));
        session.close(&**log);
        code = EXIT_FAILED;
    }

    // Step 9. `finish` logs a time-out itself; the guard covers a restore it left.
    io_thread.finish(FINISH_MS);
    dispatcher.into_runners().shut_down();
    code
}

/// A start failure that leaves the tray useless: one line, then the caller drops what it built and exits.
fn fatal(log: &dyn Log, what: &str, e: WinError) -> u8 {
    let text = format!(
        "{what} could not be made: {} failed with {}; the tray stops",
        e.call, e.code
    );
    log.line(Level::Error, AREA, &text);
    EXIT_FAILED
}

/// The watch thread's sources: Claude Code only in v1 (Codex and Copilot after v1; Haik, 2026-11-03). The folder is
/// found the way Claude Code finds it: `CLAUDE_CONFIG_DIR`, else `.claude` in the profile folder, which
/// `std::env::home_dir` reads in the same order as Node's `os.homedir` (`USERPROFILE` first).
fn sources(log: &dyn Log) -> Vec<Box<dyn AgentSource + Send>> {
    let Some(home) = std::env::home_dir() else {
        log.line(
            Level::Warn,
            AREA,
            "the profile folder is unknown, so Claude Code is not watched",
        );
        return Vec::new();
    };
    let config_dir = std::env::var_os(CLAUDE_CONFIG_DIR).filter(|dir| !dir.is_empty());
    let text = if config_dir.is_some() {
        "Claude Code is watched in the folder CLAUDE_CONFIG_DIR names"
    } else {
        "Claude Code is watched in .claude of the profile folder"
    };
    log.line(Level::Info, AREA, text);
    let claude = claude_folder(&home, config_dir.as_deref());
    vec![Box::new(ClaudeSource::in_folder(
        claude,
        Box::new(SystemZone::current()),
    ))]
}

/// The loop's own lines go to the IO thread as `Command::Log`, as the dispatcher's do, so the loop thread never
/// writes the log file (architecture section 4, "Tray threads"). The IO thread writes them in area `core`, so the
/// text names the area and any level above Info.
struct QueuedLog(Arc<IoQueue>);

impl Log for QueuedLog {
    fn line(&self, level: Level, area: &'static str, text: &str) {
        let text = match level {
            Level::Info => format!("{area}: {text}"),
            Level::Warn => format!("{area}, warning: {text}"),
            Level::Error => format!("{area}, error: {text}"),
        };
        self.0.push(Command::Log(text));
    }
}

/// The icon's notes worth a line: a failed add once per outage (the retry runs every 30 s at most), the add that ends
/// it, a failed change, and a taskbar restart.
fn icon_notes(log: Arc<dyn Log>) -> icon::NoteHook {
    Box::new(move |note: &icon::Note| match note {
        icon::Note::AddFailed {
            attempt: 1,
            retry_in,
        } => {
            let text = format!(
                "tray icon not added yet; next try in {} s",
                retry_in.as_secs()
            );
            log.line(Level::Warn, AREA, &text);
        }
        icon::Note::Added { attempt } if *attempt > 1 => {
            log.line(
                Level::Info,
                AREA,
                &format!("tray icon added at try {attempt}"),
            );
        }
        icon::Note::ModifyFailed => {
            log.line(
                Level::Warn,
                AREA,
                "tray icon change failed; it is added again",
            );
        }
        icon::Note::TaskbarCreated => {
            log.line(
                Level::Info,
                AREA,
                "the taskbar started again; the icon is added again",
            );
        }
        _ => {}
    })
}

/// The menu's notes worth a line (`p5-menu-view`): a failure, which may have lost the pick, and a menu that opened
/// without the foreground, which may not close on an outside click.
fn menu_notes(log: Arc<dyn Log>) -> menu::NoteHook {
    Box::new(move |note: &menu::Note| match note {
        menu::Note::Failed(e) => {
            let text = format!("menu: {} failed with {}", e.call, e.code);
            log.line(Level::Warn, AREA, &text);
        }
        menu::Note::Opened {
            foreground: false, ..
        } => {
            log.line(
                Level::Warn,
                AREA,
                "menu opened without the foreground; it may not close on an outside click",
            );
        }
        _ => {}
    })
}

/// What a balloon click opens, and how the log names it (never the path, which holds the user's name).
fn click_target(click: NoticeClick, log_file: &Path) -> Option<(OsString, &'static str)> {
    match click {
        NoticeClick::Nothing => None,
        NoticeClick::OpenLog => Some((log_file.as_os_str().to_owned(), "the log")),
        NoticeClick::OpenTaskbarSettings => {
            Some((OsString::from(TASKBAR_SETTINGS), "taskbar settings"))
        }
    }
}

/// The balloon click hook, on the loop thread. The open runs on a short thread of its own, which logs the result:
/// `open_with_default_app` can wait 10 s for the shell, and the loop never waits.
fn balloon_click(
    notifier: Arc<BalloonNotifier>,
    log_file: PathBuf,
    log: Arc<dyn Log>,
    loop_log: Arc<dyn Log>,
) -> Box<dyn FnMut()> {
    Box::new(move || {
        let Some((target, what)) = click_target(notifier.take_click(), &log_file) else {
            return;
        };
        let thread_log = Arc::clone(&log);
        let spawned = thread::Builder::new()
            .name("wakeshot-open".to_owned())
            .spawn(move || match shell::open_with_default_app(&target) {
                Ok(()) => {
                    thread_log.line(Level::Info, AREA, &format!("notice click opened {what}"))
                }
                Err(e) => {
                    let text = format!(
                        "notice click: {what} not opened: {} failed with {}",
                        e.call, e.code
                    );
                    thread_log.line(Level::Warn, AREA, &text);
                }
            });
        if let Err(e) = spawned {
            let text = format!("notice click: {what} not opened, no thread: {e}");
            loop_log.line(Level::Warn, AREA, &text);
        }
    })
}

/// `WTSRegisterSessionNotification` for the loop window, retried on each safety tick while it fails: the docs say it
/// fails with `RPC_S_INVALID_BINDING` (1702) before the Remote Desktop services have started, which a tray started at
/// sign-in may meet (`p3-session`). On the loop thread only.
struct SessionSlot {
    hwnd: HWND,
    notes: RefCell<Option<SessionNotifications>>,
    failed: Cell<u32>,
    closed: Cell<bool>,
}

impl SessionSlot {
    fn register(hwnd: HWND, log: &dyn Log) -> Self {
        let slot = Self {
            hwnd,
            notes: RefCell::new(None),
            failed: Cell::new(0),
            closed: Cell::new(false),
        };
        if let Err(e) = slot.try_now() {
            let text = format!(
                "session notifications not registered: {} failed with {}; tried again every 30 s, and until then no \
                 lock, unlock or console change reaches the tray",
                e.call, e.code
            );
            log.line(Level::Warn, AREA, &text);
        }
        slot
    }

    fn try_now(&self) -> Result<(), WinError> {
        match session::register(self.hwnd) {
            Ok(notes) => {
                *self.notes.borrow_mut() = Some(notes);
                Ok(())
            }
            Err(e) => {
                self.failed.set(self.failed.get() + 1);
                Err(e)
            }
        }
    }

    /// On each safety tick while not registered. A late registration reads the lock state, as the start does: a lock
    /// in the gap reached nobody.
    fn retry(&self, log: &dyn Log) -> Option<Event> {
        if self.closed.get() || self.notes.borrow().is_some() {
            return None;
        }
        self.try_now().ok()?;
        let text = format!(
            "session notifications registered after {} failed tries",
            self.failed.get()
        );
        log.line(Level::Info, AREA, &text);
        startup::locked_at_start(log).then_some(Event::Session(SessionChange::Locked))
    }

    /// Unregisters and stops the retry; a second call does nothing.
    fn close(&self, log: &dyn Log) {
        self.closed.set(true);
        let notes = self.notes.borrow_mut().take();
        if let Some(Err(e)) = notes.map(SessionNotifications::unregister) {
            let text = format!(
                "session notifications not unregistered: {} failed with {}",
                e.call, e.code
            );
            log.line(Level::Warn, AREA, &text);
        }
    }
}

/// Why the settings mutex is not this tray's, so each reason is logged once.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Miss {
    Busy,
    Failed(WinError),
}

/// `Global\Wakeshot.Settings` (or its dry-run name): taken at start and tried again at each safety tick while another
/// tray has it. `startup::try_settings` gives `None` for both a held and a denied mutex, and the log should say which,
/// so the tries are made here.
struct Settings {
    name: String,
    held: Option<HeldMutex>,
    missed: Option<Miss>,
}

impl Settings {
    fn new(name: String) -> Self {
        Self {
            name,
            held: None,
            missed: None,
        }
    }

    fn held(&self) -> bool {
        self.held.is_some()
    }

    /// One try, unless held. The line to log: the take, and each new reason for a miss once, so a try every 30 s never
    /// buries the log.
    fn try_take(&mut self) -> Option<(Level, String)> {
        if self.held.is_some() {
            return None;
        }
        let miss = match NamedMutex::open(&self.name).and_then(|mutex| mutex.take(0)) {
            Ok(Some(held)) => {
                let text = if held.abandoned() {
                    "settings mutex taken from a tray that ended without letting it go: this tray may change power \
                     settings"
                } else {
                    "settings mutex taken: this tray may change power settings"
                };
                self.held = Some(held);
                self.missed = None;
                return Some((Level::Info, text.to_owned()));
            }
            Ok(None) => Miss::Busy,
            Err(e) => Miss::Failed(e),
        };
        if self.missed.replace(miss) == Some(miss) {
            return None;
        }
        let text = match miss {
            Miss::Busy => "settings mutex held by the tray of another session: requests only, the power settings \
                           untouched; tried again every 30 s"
                .to_owned(),
            Miss::Failed(e) if e.code == ERROR_ACCESS_DENIED.0 => format!(
                "settings mutex denied ({} failed with {}), so another user's tray most likely made it: requests only; \
                 tried again every 30 s",
                e.call, e.code
            ),
            Miss::Failed(e) => format!(
                "settings mutex not taken: {} failed with {}; requests only; tried again every 30 s",
                e.call, e.code
            ),
        };
        Some((Level::Info, text))
    }
}

/// The count of failed system event posts the last safety tick saw.
#[derive(Debug, Default)]
struct LostPosts(u64);

impl LostPosts {
    /// How many more failed since the last call; `None` when none did.
    fn grew(&mut self, total: u64) -> Option<u64> {
        let more = total.checked_sub(self.0).filter(|&n| n > 0)?;
        self.0 = total;
        Some(more)
    }
}

/// Fresh events after lost posts: the lid and the power source as they are now, and a `Resumed`. A lost `Resumed`
/// leaves the requests not honored while work runs (a sleep ends them until they are taken again); a spare one costs
/// a retake and restarts the idle count, so a countdown waits at most 5 minutes more. A lost `ActivePlan` needs
/// nothing here: the plan read during a hold sees the plan.
fn fresh_readings(
    lid: Option<bool>,
    power: Result<(bool, Option<u8>), WinError>,
    log: &dyn Log,
) -> Vec<Event> {
    let mut events: Vec<Event> = lid
        .map(|closed| Event::Lid { closed })
        .into_iter()
        .collect();
    match power {
        Ok((ac, battery_pct)) => events.push(Event::Power { ac, battery_pct }),
        Err(e) => {
            let text = format!(
                "power source not read again: {} failed with {}",
                e.call, e.code
            );
            log.line(Level::Warn, AREA, &text);
        }
    }
    events.push(Event::Resumed);
    events
}

/// A failed read counts as input (0), so it never starts a hibernate; one line per run of failures.
fn idle_or_input(read: Result<u64, WinError>, failed: &mut bool, log: &dyn Log) -> u64 {
    match read {
        Ok(ms) => {
            if std::mem::take(failed) {
                log.line(Level::Info, AREA, "input idle reads again");
            }
            ms
        }
        Err(e) => {
            if !std::mem::replace(failed, true) {
                let text = format!(
                    "input idle does not read: {} failed with {}; counted as input, so no hibernate",
                    e.call, e.code
                );
                log.line(Level::Warn, AREA, &text);
            }
            0
        }
    }
}

/// `apply_update` until the update thread is wired: the core logs the failure and waits for the next update.
fn answer_update(sink: &dyn EventSink, log: &dyn Log) {
    log.line(
        Level::Warn,
        AREA,
        "update not applied: this build has no update thread yet",
    );
    if let Err(e) = sink.post(Event::Done(Outcome::Update(Err(UPDATE_NOT_WIRED)))) {
        let text = format!(
            "update answer not posted: {} failed with {}",
            e.call, e.code
        );
        log.line(Level::Warn, AREA, &text);
    }
}

/// Starts the hibernate thread for one `Command::Hibernate`; the dispatcher keeps it to one at a time.
#[derive(Clone)]
struct Hibernator {
    deps: Arc<dyn HibernateDeps>,
    sound: Arc<dyn SoundPlayer>,
    sink: Arc<dyn EventSink>,
    log: Arc<dyn Log>,
}

impl Hibernator {
    /// The thread runs `run_hibernate`, which blocks through the hibernate until resume, then posts its end once. The
    /// handle is for tests; the tray lets the thread go.
    fn start(
        &self,
        reason: CountdownReason,
        floor_pct: u8,
        last_resume_mono_ms: Option<u64>,
    ) -> Result<JoinHandle<()>, WinError> {
        let run = self.clone();
        thread::Builder::new()
            .name("wakeshot-hibernate".to_owned())
            .spawn(move || run.run(reason, floor_pct, last_resume_mono_ms))
            .map_err(|e| WinError {
                call: "CreateThread",
                code: e.raw_os_error().map_or(u32::MAX, |c| c as u32),
            })
    }

    fn run(&self, reason: CountdownReason, floor_pct: u8, last_resume_mono_ms: Option<u64>) {
        let end = run_hibernate(
            reason,
            floor_pct,
            last_resume_mono_ms,
            &*self.deps,
            &*self.sound,
            &*self.log,
        );
        if let Err(e) = self.sink.post(Event::Done(Outcome::Hibernate(end))) {
            let text = format!(
                "hibernate result not posted: {} failed with {}; no other hibernate starts in this run",
                e.call, e.code
            );
            self.log.line(Level::Warn, AREA, &text);
        }
    }
}

/// The loop's runners over the real or dry-run objects (phase file note 4's table). On the loop thread only.
struct RealRunners {
    requests: Box<dyn RequestApi>,
    locker: Box<dyn Locker>,
    icon: TrayIcon,
    sound: Arc<dyn SoundPlayer>,
    hibernator: Hibernator,
    settings: Settings,
    watch: WatchHandle,
    subscriptions: Subscriptions,
    lost: LostPosts,
    session: Rc<SessionSlot>,
    sink: Arc<dyn EventSink>,
    /// Lines through the IO thread (`QueuedLog`).
    log: Arc<dyn Log>,
    idle_failed: bool,
}

impl RealRunners {
    /// Step 9 after the IO thread finished, in the note's order. The update thread stops first once it is wired. Lines
    /// from here on are lost with the IO thread gone; each part that can fail logs through its own log.
    fn shut_down(self) {
        let Self {
            requests,
            icon,
            settings,
            watch,
            subscriptions,
            session,
            log,
            ..
        } = self;
        watch.stop();
        drop(subscriptions);
        session.close(&*log);
        drop(icon);
        drop(requests);
        drop(settings);
    }
}

impl LoopRunners for RealRunners {
    fn requests(&mut self, system: bool, display: bool) -> Result<(), WinError> {
        self.requests.set(system, display)
    }

    fn retake(&mut self) -> Result<(), WinError> {
        self.requests.retake()
    }

    fn lock(&mut self) -> Result<(), WinError> {
        self.locker.lock()
    }

    fn show_tray(&mut self, view: &TrayView) {
        self.icon.show(view);
    }

    fn play(&mut self, cue: Cue) {
        self.sound.play(cue);
    }

    fn start_hibernate(
        &mut self,
        reason: CountdownReason,
        floor_pct: u8,
        last_resume_mono_ms: Option<u64>,
    ) {
        let Err(e) = self
            .hibernator
            .start(reason, floor_pct, last_resume_mono_ms)
        else {
            return;
        };
        let text = format!(
            "hibernate thread not started: {} failed with {}",
            e.call, e.code
        );
        self.log.line(Level::Warn, AREA, &text);
        // The dispatcher starts no other hibernate before this answer.
        if let Err(e) = self
            .sink
            .post(Event::Done(Outcome::Hibernate(HibernateEnd::Failed(
                e.into(),
            ))))
        {
            let text = format!(
                "hibernate answer not posted: {} failed with {}",
                e.call, e.code
            );
            self.log.line(Level::Warn, AREA, &text);
        }
    }

    fn apply_update(&mut self) {
        answer_update(&*self.sink, &*self.log);
    }

    fn input_idle_ms(&mut self) -> u64 {
        idle_or_input(clock::input_idle_ms(), &mut self.idle_failed, &*self.log)
    }

    fn try_settings_owner(&mut self) -> bool {
        if let Some((level, text)) = self.settings.try_take() {
            self.log.line(level, AREA, &text);
        }
        self.settings.held()
    }

    /// Also the late session registration: its `Locked` is a session change that reached nobody, like a lost post.
    fn lost_events(&mut self) -> Vec<Event> {
        let mut events: Vec<Event> = self.session.retry(&*self.log).into_iter().collect();
        if let Some(more) = self.lost.grew(self.subscriptions.lost_posts()) {
            let text = format!(
                "{more} system event posts failed since the last tick; the lid and the power source are read again"
            );
            self.log.line(Level::Warn, AREA, &text);
            events.extend(fresh_readings(
                system::lid_once(LID_READ_MS),
                system::power_status(),
                &*self.log,
            ));
        }
        events
    }

    fn watch_tick(&mut self) {
        self.watch.tick();
    }
}

/// The window's handler: the dispatcher, plus the session unregister at Quit, before `LoopWindow::run` destroys the
/// window.
struct TrayLoop<'a> {
    dispatcher: &'a Dispatcher<RealRunners>,
    session: &'a SessionSlot,
    log: &'a dyn Log,
}

impl TrayLoop<'_> {
    fn after(&self, flow: Flow) -> Flow {
        if flow == Flow::Quit {
            self.session.close(self.log);
        }
        flow
    }
}

impl LoopHandler for TrayLoop<'_> {
    fn event(&self, event: Event) -> Flow {
        self.after(self.dispatcher.on_event(event))
    }

    fn session(&self, change: SessionChange) -> Flow {
        self.after(self.dispatcher.on_session(change))
    }

    fn timer(&self) -> Flow {
        self.after(self.dispatcher.on_timer())
    }

    fn next_timer_ms(&self) -> u32 {
        self.dispatcher.timer_ms()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;
    use std::time::Duration;

    use super::*;
    use crate::power::hibernate::FakeHibernate;
    use crate::store::Names;
    use crate::testing::{FakeEventSink, FakeSoundPlayer, MemoryLog};

    fn lines(log: &MemoryLog) -> Vec<(Level, String)> {
        log.lines
            .lock()
            .unwrap()
            .iter()
            .map(|(level, _, text)| (*level, text.clone()))
            .collect()
    }

    fn hibernator(
        deps: Arc<dyn HibernateDeps>,
    ) -> (Hibernator, Arc<FakeEventSink>, Arc<MemoryLog>) {
        let sink = Arc::new(FakeEventSink::default());
        let log = Arc::new(MemoryLog::default());
        let h = Hibernator {
            deps,
            sound: Arc::new(FakeSoundPlayer::default()),
            sink: sink.clone(),
            log: log.clone(),
        };
        (h, sink, log)
    }

    fn hibernate_ends(sink: &FakeEventSink) -> Vec<HibernateEnd> {
        sink.events()
            .into_iter()
            .filter_map(|e| match e {
                Event::Done(Outcome::Hibernate(end)) => Some(end),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn dry_run_hibernate_thread_posts_one_done_and_never_hibernates() {
        let log = Arc::new(MemoryLog::default());
        // Every check passes, so only the dry run stands between the thread and `SetSuspendState`.
        let deps = Arc::new(DryRunHibernate::over(FakeHibernate::default(), log.clone()));
        let sound = Arc::new(FakeSoundPlayer::default());
        let sink = Arc::new(FakeEventSink::default());
        let h = Hibernator {
            deps,
            sound: sound.clone(),
            sink: sink.clone(),
            log: log.clone(),
        };
        h.start(CountdownReason::WorkEnded, 20, None)
            .unwrap()
            .join()
            .unwrap();
        assert_eq!(
            sink.events(),
            [Event::Done(Outcome::Hibernate(HibernateEnd::Resumed))]
        );
        assert_eq!(*sound.played.lock().unwrap(), [Cue::GoingToSleep]);
        let logged = lines(&log);
        assert!(
            logged
                .iter()
                .any(|(_, t)| t == "dry run: SetSuspendState(TRUE, FALSE, FALSE)"),
            "{logged:?}"
        );
    }

    #[test]
    fn dry_run_hibernate_on_real_reads_posts_exactly_one_done() {
        let (h, sink, _) = hibernator(Arc::new(DryRunHibernate::new(Arc::new(
            MemoryLog::default(),
        ))));
        h.start(CountdownReason::Floor, 20, None)
            .unwrap()
            .join()
            .unwrap();
        assert_eq!(hibernate_ends(&sink).len(), 1, "{:?}", sink.events());
        assert_eq!(sink.events().len(), 1);
    }

    #[test]
    fn a_hibernate_end_that_cannot_be_posted_is_logged() {
        let deps = Arc::new(FakeHibernate {
            lid: Some(false),
            ..FakeHibernate::default()
        });
        let log = Arc::new(MemoryLog::default());
        let sink = Arc::new(FakeEventSink {
            fail_with: Some(WinError {
                call: "PostMessageW",
                code: 1816,
            }),
            ..FakeEventSink::default()
        });
        let h = Hibernator {
            deps,
            sound: Arc::new(FakeSoundPlayer::default()),
            sink,
            log: log.clone(),
        };
        h.start(CountdownReason::WorkEnded, 20, None)
            .unwrap()
            .join()
            .unwrap();
        let logged = lines(&log);
        assert!(
            logged.iter().any(|(level, t)| *level == Level::Warn
                && t.starts_with("hibernate result not posted: PostMessageW failed with 1816")),
            "{logged:?}"
        );
    }

    #[test]
    fn apply_update_answers_err_until_the_update_thread_is_wired() {
        let sink = FakeEventSink::default();
        let log = MemoryLog::default();
        answer_update(&sink, &log);
        assert_eq!(
            sink.events(),
            [Event::Done(Outcome::Update(Err(UPDATE_NOT_WIRED)))]
        );
        assert_eq!(lines(&log).len(), 1);
    }

    #[test]
    fn loop_lines_go_to_the_io_queue_with_area_and_level() {
        let io = Arc::new(IoQueue::new());
        let log = QueuedLog(Arc::clone(&io));
        log.line(Level::Info, "tray", "one");
        log.line(Level::Warn, "tray", "two");
        log.line(Level::Error, "power", "three");
        let popped: Vec<Command> = std::iter::from_fn(|| io.pop(0)).collect();
        assert_eq!(
            popped,
            [
                Command::Log("tray: one".into()),
                Command::Log("tray, warning: two".into()),
                Command::Log("power, error: three".into()),
            ]
        );
    }

    #[test]
    fn balloon_clicks_open_the_log_or_taskbar_settings() {
        let log_file = Path::new(r"C:\data\logs\wakeshot.log");
        assert_eq!(click_target(NoticeClick::Nothing, log_file), None);
        assert_eq!(
            click_target(NoticeClick::OpenLog, log_file),
            Some((OsString::from(r"C:\data\logs\wakeshot.log"), "the log"))
        );
        assert_eq!(
            click_target(NoticeClick::OpenTaskbarSettings, log_file),
            Some((OsString::from("ms-settings:taskbar"), "taskbar settings"))
        );
    }

    #[test]
    fn settings_mutex_says_why_once_and_takes_it_when_free() {
        let name = Names::test("tray-mode").settings;
        let (held_tx, held_rx) = mpsc::channel();
        let (free_tx, free_rx) = mpsc::channel::<()>();
        let other = {
            let name = name.clone();
            thread::spawn(move || {
                let mutex = NamedMutex::open(&name).unwrap();
                let held = mutex.take(0).unwrap().expect("free at first");
                held_tx.send(()).unwrap();
                free_rx.recv().unwrap();
                drop(held);
            })
        };
        held_rx.recv().unwrap();
        let mut settings = Settings::new(name);
        let first = settings.try_take().expect("the first miss is logged");
        assert!(
            first
                .1
                .starts_with("settings mutex held by the tray of another session")
        );
        assert_eq!(settings.try_take(), None, "the same miss twice");
        assert!(!settings.held());
        free_tx.send(()).unwrap();
        other.join().unwrap();
        let taken = settings.try_take().expect("the take is logged");
        assert_eq!(
            taken,
            (
                Level::Info,
                "settings mutex taken: this tray may change power settings".to_owned()
            )
        );
        assert!(settings.held());
        assert_eq!(settings.try_take(), None, "no try while held");
    }

    #[test]
    fn lost_posts_count_only_growth() {
        let mut lost = LostPosts::default();
        assert_eq!(lost.grew(0), None);
        assert_eq!(lost.grew(3), Some(3));
        assert_eq!(lost.grew(3), None);
        assert_eq!(lost.grew(5), Some(2));
    }

    #[test]
    fn fresh_readings_give_lid_power_and_resumed() {
        let log = MemoryLog::default();
        assert_eq!(
            fresh_readings(Some(true), Ok((false, Some(40))), &log),
            [
                Event::Lid { closed: true },
                Event::Power {
                    ac: false,
                    battery_pct: Some(40)
                },
                Event::Resumed,
            ]
        );
        let failed = WinError {
            call: "GetSystemPowerStatus",
            code: 31,
        };
        assert_eq!(fresh_readings(None, Err(failed), &log), [Event::Resumed]);
        assert_eq!(lines(&log).len(), 1, "the failed read is logged");
    }

    #[test]
    fn a_failed_idle_read_counts_as_input_and_logs_once_per_run() {
        let log = MemoryLog::default();
        let mut failed = false;
        let error = WinError {
            call: "GetLastInputInfo",
            code: 87,
        };
        assert_eq!(idle_or_input(Err(error), &mut failed, &log), 0);
        assert_eq!(idle_or_input(Err(error), &mut failed, &log), 0);
        assert_eq!(idle_or_input(Ok(42), &mut failed, &log), 42);
        assert_eq!(idle_or_input(Ok(43), &mut failed, &log), 43);
        let logged = lines(&log);
        assert_eq!(logged.len(), 2, "{logged:?}");
        assert_eq!(logged[0].0, Level::Warn);
        assert_eq!(
            logged[1],
            (Level::Info, "input idle reads again".to_owned())
        );
    }

    #[test]
    fn icon_and_menu_notes_log_only_what_matters() {
        let io = Arc::new(IoQueue::new());
        let mut icon_hook = icon_notes(Arc::new(QueuedLog(Arc::clone(&io))));
        for note in [
            icon::Note::AddFailed {
                attempt: 1,
                retry_in: Duration::from_secs(2),
            },
            icon::Note::AddFailed {
                attempt: 2,
                retry_in: Duration::from_secs(4),
            },
            icon::Note::Added { attempt: 3 },
            icon::Note::Added { attempt: 1 },
            icon::Note::SizeChanged(24),
        ] {
            icon_hook(&note);
        }
        let mut menu_hook = menu_notes(Arc::new(QueuedLog(Arc::clone(&io))));
        menu_hook(&menu::Note::Closed { command: None });
        menu_hook(&menu::Note::Failed(WinError {
            call: "TrackPopupMenuEx",
            code: 1401,
        }));
        let popped: Vec<Command> = std::iter::from_fn(|| io.pop(0)).collect();
        assert_eq!(
            popped,
            [
                Command::Log("tray, warning: tray icon not added yet; next try in 2 s".into()),
                Command::Log("tray: tray icon added at try 3".into()),
                Command::Log("tray, warning: menu: TrackPopupMenuEx failed with 1401".into()),
            ]
        );
    }
}
