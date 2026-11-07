//! The composition root: the real objects, thread starts, startup and shutdown order, `RealRunners`.
//!
//! `run` follows architecture section 4, "Tray startup", steps 3 to 8: the data folder, the tray mutex, the log and
//! the panic hook, the startup restore before any other thread starts, the settings mutex, then in `serve`:
//!
//! 1. The loop window, the menu, the tray icon, the notifier, the sound player and the session and power
//!    notifications, all on the loop thread.
//! 2. The watch thread, before `Dispatcher::new`, so its handle can go into `RealRunners`. Its first posts wait in the
//!    loop window's queue until the pump runs, and `Dispatcher::new` feeds `Started` to the core directly, so the core
//!    still sees `Started` first.
//! 3. The update thread, for the same reason: `RealRunners` owns its handle, so `ApplyUpdate` reaches it without a
//!    slot shared with `serve`. It posts nothing for its first 5 minutes, and the core asks for no apply before the
//!    thread's own `UpdateReady`. Then the agent hooks thread, which needs the sink.
//! 4. `Dispatcher::new`, then the IO thread. The core gives the first-run `Notify` before the first `Tray` in the
//!    `Started` batch; the loop runs that `Tray` (the icon's add) inside `Dispatcher::new`, so the notice only reaches
//!    the shell once the icon exists. A running IO thread could show it first and be refused; the notifier then keeps
//!    it for the add, so the order only spares that refusal.
//! 5. The pump. The session notifications go when the dispatcher quits, inside the pump: `LoopWindow::run` destroys
//!    the window before it returns, and Microsoft asks for the unregister before the window is destroyed.
//! 6. After Quit: the IO thread runs what is queued, then `RealRunners::shut_down` stops the update thread first,
//!    then the rest; the tray mutex goes last.

use std::cell::{Cell, RefCell};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::rc::Rc;
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use wakeshot_core::command::{Command, CountdownReason, Failure, Outcome};
use wakeshot_core::core::Core;
use wakeshot_core::event::{Event, SessionChange};
use wakeshot_core::ownership::io::PlanIo;
use wakeshot_core::sources::AgentSource;
use wakeshot_core::sources::claude::{ClaudeSource, claude_folder};
use wakeshot_core::sources::codex::{CodexSource, codex_home};
use wakeshot_core::sources::copilot::CopilotSource;
use wakeshot_core::sources::opencode::{OpenCodeSource, opencode_db};
use wakeshot_core::text::NoticeClick;
use wakeshot_core::view::{Cue, MenuCommand, Notice, TrayView};
use windows::Win32::Foundation::{
    ERROR_ACCESS_DENIED, ERROR_NOT_ENOUGH_QUOTA, ERROR_NOT_SUPPORTED, HWND,
};

use super::dispatch::{Dispatcher, LoopRunners};
use super::io_thread::{FINISH_MS, IoParts, IoThread};
use super::startup::{self, Boot};
use super::window::LoopWindow;
use super::{Ack, Flow, LoopHandler};
use crate::WinError;
use crate::clock::{self, SystemClock};
use crate::diagnostics;
use crate::io_queue::IoQueue;
use crate::localtime::SystemZone;
use crate::log::{self, Level, Log, LogWriter, Proc};
use crate::notify::BalloonNotifier;
use crate::power::executor::PowerExecutor;
use crate::power::hibernate::{
    DryRunHibernate, HibernateDeps, Win32Hibernate, run_hibernate, run_sleep_check,
};
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
use crate::update::{FeedError, UpdateFeed, UpdateHandle, UpdateThread};
use crate::watch::thread::{WatchHandle, WatchThread};

const AREA: &str = "tray";

/// The exit code of a tray that could not start: the same 1 the guard and restore mode give after a give-up.
const EXIT_FAILED: u8 = 1;

/// How often the hibernate thread posts its end while the loop's queue is full (1816), and how long apart. The
/// dispatcher starts no other hibernate before that end is fed, the battery floor's included, so a lost one would
/// block them all for the rest of the run. A full queue empties once the loop runs again; a minute covers a loop held
/// up by a slow shell call. Any other error means the window is gone: the tray quits.
const HIBERNATE_POST_TRIES: u32 = 60;
const HIBERNATE_POST_WAIT: Duration = Duration::from_secs(1);

/// What the first-run notice's click opens.
const TASKBAR_SETTINGS: &str = "ms-settings:taskbar";

/// Claude Code's own variable for its folder.
const CLAUDE_CONFIG_DIR: &str = "CLAUDE_CONFIG_DIR";

/// Codex CLI's own variable for its home folder (`sources::codex::codex_home`).
const CODEX_HOME: &str = "CODEX_HOME";

/// Copilot CLI's own variable for its home folder (`sources::copilot::CopilotSource::new`).
const COPILOT_HOME: &str = "COPILOT_HOME";

/// The XDG variable OpenCode finds its data folder by (`sources::opencode::opencode_db`).
const XDG_DATA_HOME: &str = "XDG_DATA_HOME";

/// How long a lost-event reading waits for the lid on the loop thread. Its first callback came within 1 ms in every
/// test (research, "Lid notification"), and a PC with no lid answers nothing, so the wait must stay short.
const LID_READ_MS: u64 = 200;

/// The answer to `ApplyUpdate` when no update thread runs: not installed, a thread that did not start, or one that
/// ended. 50 is `ERROR_NOT_SUPPORTED`.
const NO_UPDATE_THREAD: Failure = Failure {
    call: "no update thread",
    code: ERROR_NOT_SUPPORTED.0,
};

/// Makes the update feed. It runs on the update thread, so a feed that is not `Send` (Velopack's) still works.
pub type MakeFeed = Box<dyn FnOnce() -> Box<dyn UpdateFeed> + Send>;

/// What tray mode gets from Velopack: `velo.rs` in the tray bin, the only crate that has `velopack`. The dry run gets
/// `Updates::default()`.
#[derive(Default)]
pub struct Updates {
    /// Makes the feed. `None`: this exe is not an install (a build run from `target\`), so no update thread runs.
    pub feed: Option<MakeFeed>,
    /// The version Velopack restarted the tray with after it applied an update: the core gets `Event::Updated`.
    pub restarted: Option<String>,
}

/// Runs tray mode, or dry-run mode when `dry_run`, until Quit: tray startup steps 3 to 8 (architecture section 4).
/// Steps 1 and 2, Velopack and the mode, are the tray `main`'s.
pub fn run(dry_run: bool, updates: Updates) -> ExitCode {
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
    // Before the startup restore, so a value a dead tray left shows in the header and its restore in the lines after.
    log_start_header(&*log);
    if tray_lock.abandoned() {
        log.line(
            Level::Info,
            AREA,
            "the last tray of this session ended without quitting",
        );
    }
    startup::install_panic_hook(Arc::clone(&log));
    let liveness: Arc<dyn Liveness> = Arc::new(SystemLiveness);
    // The dry run drops `updates` even if given: run from an installed copy, an apply would end that copy's real tray
    // and run its hooks, which write the real Run value and Path.
    let code = if dry_run {
        dry_run_tray(&boot, &log, liveness)
    } else {
        real_tray(&boot, &log, liveness, updates)
    };
    log.line(
        Level::Info,
        AREA,
        &format!("Wakeshot ends with code {code}"),
    );
    drop(tray_lock);
    ExitCode::from(code)
}

/// The start header: one line per fact of this PC (`diagnostics::machine_facts`), area `start`, so the rest of the log
/// reads alone (architecture section 8).
fn log_start_header(log: &dyn Log) {
    for fact in diagnostics::machine_facts() {
        log.line(
            Level::Info,
            "start",
            &format!("{}: {}", fact.name, fact.value),
        );
    }
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

/// The real executor: plan writes, RunOnce and a guard; updates when installed.
fn real_tray(boot: &Boot, log: &Arc<dyn Log>, liveness: Arc<dyn Liveness>, updates: Updates) -> u8 {
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
    if updates.feed.is_none() {
        log.line(
            Level::Info,
            AREA,
            "Wakeshot is not installed, so it does not check for updates",
        );
    }
    serve(boot, log, liveness, executor, Picks::real(), updates)
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
    serve(
        boot,
        log,
        liveness,
        executor,
        Picks::dry_run(log),
        Updates::default(),
    )
}

/// Steps 6 to 8 for either executor. Returns the exit code.
fn serve<P, G>(
    boot: &Boot,
    log: &Arc<dyn Log>,
    liveness: Arc<dyn Liveness>,
    mut executor: PowerExecutor<P, FileNet<G>>,
    picks: Picks,
    updates: Updates,
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
    let notifier = Arc::new(BalloonNotifier::new(icon.identity()).with_log(Arc::clone(log)));
    icon.on_note(icon_notes(
        Arc::clone(&loop_log),
        Arc::clone(&notifier),
        Arc::clone(&io),
    ));
    icon.on_balloon_click(balloon_click(
        Arc::clone(&notifier),
        log::log_file(&boot.dir.logs()),
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
    let watched = watched_sources(std::env::home_dir().as_deref(), AgentEnv::read(), log);
    let watch = match WatchThread::start(
        boot.dir.clone(),
        config.clone(),
        watched,
        Arc::clone(&liveness),
        Arc::clone(&sink),
        Arc::clone(log),
    ) {
        Ok(watch) => watch,
        Err(e) => return fatal(&**log, "the watch thread", e),
    };
    // Not fatal: without updates the tray still does all its work.
    let updater = Updater::start(updates.feed, Arc::clone(&sink), log, Arc::clone(&loop_log));
    let _agent_hooks = start_agent_hooks(boot, &sink, log);
    if let Some(version) = &updates.restarted {
        let text = format!("Velopack restarted the tray after the update to {version}");
        log.line(Level::Info, AREA, &text);
    }
    let owner = settings.held();
    let started = startup::started(boot, config.clone(), first_run, &restored, owner, &**log);
    let first = first_events(
        started,
        startup::locked_at_start(&**log),
        restored.corrupt,
        updates.restarted,
    );
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
            post_wait: HIBERNATE_POST_WAIT,
        },
        settings,
        watch,
        updater,
        subscriptions,
        lost: LostPosts::default(),
        session: Rc::clone(&session),
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

    // After Quit (step 8). `finish` logs a time-out itself; the guard covers a restore it left. `shut_down` stops the
    // update thread first.
    io_thread.finish(FINISH_MS);
    dispatcher.into_runners().shut_down();
    code
}

/// The events `Dispatcher::new` feeds before any other, in the order `event.rs` promises the core: `Started`, then
/// `Session(Locked)`, `JournalCorrupt` and `Updated`, each only when it applies. The dispatcher sorts them again.
fn first_events(
    started: Event,
    locked: bool,
    corrupt: bool,
    updated: Option<String>,
) -> Vec<Event> {
    let mut first = vec![started];
    if locked {
        first.push(Event::Session(SessionChange::Locked));
    }
    if corrupt {
        first.push(Event::JournalCorrupt);
    }
    first.extend(updated.map(|version| Event::Updated { version }));
    first
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

/// The watch thread's sources: Claude Code, Codex CLI, Copilot CLI and OpenCode, all from the profile folder `home`.
/// `home` is what `std::env::home_dir` reads, in the same order as Node's `os.homedir` (`USERPROFILE` first); `env`
/// holds the agents' own variables from the tray's environment, so a value set in one shell only is not seen. `serve`
/// and the tests both call this, so a test sees every source the tray starts.
///
/// Out of v1 by "No guessing" (`docs/product.md`); their sources stay, compiled and tested, for a later version:
///
/// - VS Code's own Local chats: a window closed mid-turn writes nothing, so the chat stays working on disk. VS Code's
///   Copilot harness chats are Copilot CLI sessions of its `copilot-runtime.exe`, so the Copilot source holds them.
/// - Antigravity: Esc and Ctrl+C fire no hook; only an undocumented log shows them, and `agy` releases almost daily,
///   so it would need a version lock.
/// - Cursor: a chat deleted while it runs sends no `stop`, only the `sessionEnd` that Ctrl+F4 also sends, so its end
///   cannot be seen.
fn watched_sources(
    home: Option<&Path>,
    env: AgentEnv,
    log: &Arc<dyn Log>,
) -> Vec<Box<dyn AgentSource + Send>> {
    match home {
        Some(home) => sources_in(home, env, log),
        None => {
            log.line(
                Level::Warn,
                AREA,
                "the profile folder is unknown, so no agent is watched",
            );
            Vec::new()
        }
    }
}

/// The variables that move an agent's folder, as the tray's environment has them.
#[derive(Debug, Default)]
struct AgentEnv {
    claude_config_dir: Option<OsString>,
    codex_home: Option<OsString>,
    copilot_home: Option<OsString>,
    xdg_data_home: Option<OsString>,
}

impl AgentEnv {
    fn read() -> Self {
        Self {
            claude_config_dir: std::env::var_os(CLAUDE_CONFIG_DIR),
            codex_home: std::env::var_os(CODEX_HOME),
            copilot_home: std::env::var_os(COPILOT_HOME),
            xdg_data_home: std::env::var_os(XDG_DATA_HOME),
        }
    }
}

/// The profile folder's agents for [`watched_sources`]. An empty value counts as unset for each. Claude Code:
/// `CLAUDE_CONFIG_DIR`, else `.claude`. Codex CLI: `CODEX_HOME`, else `.codex`. Copilot CLI: `COPILOT_HOME`, else
/// `.copilot` (its `--config-dir` is per session, so not seen). OpenCode: `XDG_DATA_HOME`, else `.local\share`
/// (`sources::opencode::opencode_db`). One line per agent says which; never a path, which holds the user's name. A
/// zone logs its own failures: boxed into the source, nobody could ask it for them.
fn sources_in(home: &Path, env: AgentEnv, log: &Arc<dyn Log>) -> Vec<Box<dyn AgentSource + Send>> {
    let set = |value: Option<OsString>| value.filter(|v| !v.is_empty());
    let line = |text: &str| log.line(Level::Info, AREA, text);
    let zone = || Box::new(SystemZone::current_logged(Arc::clone(log)));

    let config_dir = set(env.claude_config_dir);
    line(match config_dir {
        Some(_) => "Claude Code is watched in the folder CLAUDE_CONFIG_DIR names",
        None => "Claude Code is watched in .claude of the profile folder",
    });
    let claude = claude_folder(home, config_dir.as_deref());

    let codex_dir = set(env.codex_home);
    line(match codex_dir {
        Some(_) => "Codex CLI is watched in the folder CODEX_HOME names",
        None => "Codex CLI is watched in .codex of the profile folder",
    });
    let codex = codex_home(home, codex_dir.as_deref());

    // What Copilot does with an empty value is not verified; Codex treats its own as unset.
    let copilot = match set(env.copilot_home) {
        Some(dir) => {
            line("Copilot CLI is watched in the folder COPILOT_HOME names");
            PathBuf::from(dir)
        }
        None => {
            line("Copilot CLI is watched in .copilot of the profile folder");
            home.join(".copilot")
        }
    };

    let data_home = set(env.xdg_data_home);
    line(match data_home {
        Some(_) => "OpenCode is watched in the folder XDG_DATA_HOME names",
        None => "OpenCode is watched in .local\\share of the profile folder",
    });
    let opencode = opencode_db(home, data_home.as_deref());

    vec![
        Box::new(ClaudeSource::in_folder(claude, zone())),
        Box::new(CodexSource::new(codex)),
        Box::new(CopilotSource::new(copilot, zone())),
        Box::new(OpenCodeSource::new(opencode)),
    ]
}

/// Wakeshot's hook folders in the agents (`startup::add_agent_hooks`), once per start, after the loop's sink exists:
/// v1 adds none and removes the folders an earlier build added; a later version's added folder posts its notice there.
/// Never joined: the thread ends by itself, and a quit during it leaves at most a staging folder, which the next start
/// removes.
fn start_agent_hooks(
    boot: &Boot,
    sink: &Arc<dyn EventSink>,
    log: &Arc<dyn Log>,
) -> Option<JoinHandle<()>> {
    startup::add_agent_hooks(boot, Arc::clone(sink), Arc::clone(log))
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
/// it, a failed change, and a taskbar restart. At each add the notices the shell refused before it are shown again.
fn icon_notes(
    log: Arc<dyn Log>,
    notifier: Arc<BalloonNotifier>,
    io: Arc<IoQueue>,
) -> icon::NoteHook {
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
        icon::Note::Added { attempt } => {
            if *attempt > 1 {
                log.line(
                    Level::Info,
                    AREA,
                    &format!("tray icon added at try {attempt}"),
                );
            }
            show_again(notifier.icon_added(), &io, &*log);
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

/// The notices the shell refused before the icon's add go to the IO thread again, in order: the loop never waits on
/// the shell, and the IO thread logs each as it shows it.
fn show_again(notices: Vec<Notice>, io: &IoQueue, log: &dyn Log) {
    if notices.is_empty() {
        return;
    }
    let text = format!(
        "the icon is added: {} notices refused before it are shown again",
        notices.len()
    );
    log.line(Level::Info, AREA, &text);
    for notice in notices {
        io.push(Command::Notify(notice));
    }
}

/// The menu's notes worth a line: a failure, which may have lost the pick, and a menu that opened without the
/// foreground, which may not close on an outside click.
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
/// sign-in may meet. On the loop thread only.
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

/// The update thread's handle when one runs (installed tray mode only), and the answer to `ApplyUpdate` when none
/// does. On the loop thread.
struct Updater {
    handle: Option<UpdateHandle>,
    sink: Arc<dyn EventSink>,
    /// Lines from the loop thread (`QueuedLog`); the thread itself writes the log file.
    log: Arc<dyn Log>,
}

impl Updater {
    /// Starts the update thread when a feed is given. `log` goes to the thread, and takes the start line: the IO
    /// thread does not run yet.
    fn start(
        feed: Option<MakeFeed>,
        sink: Arc<dyn EventSink>,
        log: &Arc<dyn Log>,
        loop_log: Arc<dyn Log>,
    ) -> Self {
        let handle = feed.and_then(|make| {
            let started = UpdateThread::start(
                move || AnyFeed(make()),
                SystemClock,
                Arc::clone(&sink),
                Arc::clone(log),
            );
            match started {
                Ok(handle) => {
                    log.line(
                        Level::Info,
                        AREA,
                        "Wakeshot is installed, so it checks for updates",
                    );
                    Some(handle)
                }
                Err(e) => {
                    let text = format!(
                        "update thread not started: {} failed with {}; no update checks in this run",
                        e.call, e.code
                    );
                    log.line(Level::Warn, AREA, &text);
                    None
                }
            }
        });
        Self {
            handle,
            sink,
            log: loop_log,
        }
    }

    /// `Command::ApplyUpdate`: hands it to the thread, which answers. With no thread to answer, the answer is
    /// `Update(Err)` from here, so the core logs it and waits for the next `UpdateReady`.
    fn apply(&self) {
        if self.handle.as_ref().is_some_and(UpdateHandle::apply) {
            return;
        }
        self.log.line(
            Level::Warn,
            AREA,
            "update not applied: no update thread runs",
        );
        if let Err(e) = self
            .sink
            .post(Event::Done(Outcome::Update(Err(NO_UPDATE_THREAD))))
        {
            let text = format!(
                "update answer not posted: {} failed with {}",
                e.call, e.code
            );
            self.log.line(Level::Warn, AREA, &text);
        }
    }

    /// After Quit. The handle logs a thread that did not stop in time through the log the thread got.
    fn stop(self) {
        if let Some(handle) = self.handle {
            handle.stop();
        }
    }
}

/// The boxed feed `MakeFeed` gives, as the `UpdateFeed` the thread takes.
struct AnyFeed(Box<dyn UpdateFeed>);

impl UpdateFeed for AnyFeed {
    fn check_and_download(&mut self) -> Result<Option<String>, FeedError> {
        self.0.check_and_download()
    }

    fn apply(&mut self) -> Result<(), FeedError> {
        self.0.apply()
    }
}

/// Starts the hibernate thread for one `Command::Hibernate` or `Command::SleepCheck`; the dispatcher keeps it to one
/// at a time.
#[derive(Clone)]
struct Hibernator {
    deps: Arc<dyn HibernateDeps>,
    sound: Arc<dyn SoundPlayer>,
    sink: Arc<dyn EventSink>,
    log: Arc<dyn Log>,
    /// `HIBERNATE_POST_WAIT`; tests wait less.
    post_wait: Duration,
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
        self.spawn(move |run| run.run(reason, floor_pct, last_resume_mono_ms))
    }

    /// The same thread for `Command::SleepCheck`: the check and the sound, no hibernate.
    fn start_check(
        &self,
        reason: CountdownReason,
        floor_pct: u8,
        last_resume_mono_ms: Option<u64>,
    ) -> Result<JoinHandle<()>, WinError> {
        self.spawn(move |run| {
            let result = run_sleep_check(
                reason,
                floor_pct,
                last_resume_mono_ms,
                &*run.deps,
                &*run.sound,
                &*run.log,
            );
            run.post_end(&Event::Done(Outcome::SleepCheck(result)));
        })
    }

    fn spawn(
        &self,
        body: impl FnOnce(Hibernator) + Send + 'static,
    ) -> Result<JoinHandle<()>, WinError> {
        let run = self.clone();
        thread::Builder::new()
            .name("wakeshot-hibernate".to_owned())
            .spawn(move || body(run))
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
        self.post_end(&Event::Done(Outcome::Hibernate(end)));
    }

    /// Posts the end, again while the queue is full (`HIBERNATE_POST_TRIES`).
    fn post_end(&self, event: &Event) {
        let mut tries = 1;
        loop {
            let Err(e) = self.sink.post(event.clone()) else {
                if tries > 1 {
                    let text = format!("hibernate result posted at try {tries}");
                    self.log.line(Level::Info, AREA, &text);
                }
                return;
            };
            if e.code != ERROR_NOT_ENOUGH_QUOTA.0 || tries >= HIBERNATE_POST_TRIES {
                let text = format!(
                    "hibernate result not posted: {} failed with {} at try {tries}; no other hibernate starts in \
                     this run",
                    e.call, e.code
                );
                self.log.line(Level::Warn, AREA, &text);
                return;
            }
            tries += 1;
            thread::sleep(self.post_wait);
        }
    }
}

/// The loop's runners over the real or dry-run objects. On the loop thread only.
struct RealRunners {
    requests: Box<dyn RequestApi>,
    locker: Box<dyn Locker>,
    icon: TrayIcon,
    sound: Arc<dyn SoundPlayer>,
    hibernator: Hibernator,
    settings: Settings,
    watch: WatchHandle,
    updater: Updater,
    subscriptions: Subscriptions,
    lost: LostPosts,
    session: Rc<SessionSlot>,
    /// Lines through the IO thread (`QueuedLog`).
    log: Arc<dyn Log>,
    idle_failed: bool,
}

impl RealRunners {
    /// After Quit, once the IO thread finished: the update and watch threads stop, then the notifications, the icon,
    /// the requests and the settings mutex go. Lines from here on are lost with the IO thread gone; each part that can
    /// fail logs through its own log.
    fn shut_down(self) {
        let Self {
            requests,
            icon,
            settings,
            watch,
            updater,
            subscriptions,
            session,
            log,
            ..
        } = self;
        updater.stop();
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
    ) -> Result<(), WinError> {
        let started = self
            .hibernator
            .start(reason, floor_pct, last_resume_mono_ms);
        if let Err(e) = &started {
            let text = format!(
                "hibernate thread not started: {} failed with {}",
                e.call, e.code
            );
            self.log.line(Level::Warn, AREA, &text);
        }
        started.map(|_| ())
    }

    fn start_sleep_check(
        &mut self,
        reason: CountdownReason,
        floor_pct: u8,
        last_resume_mono_ms: Option<u64>,
    ) -> Result<(), WinError> {
        let started = self
            .hibernator
            .start_check(reason, floor_pct, last_resume_mono_ms);
        if let Err(e) = &started {
            let text = format!(
                "sleep check thread not started: {} failed with {}",
                e.call, e.code
            );
            self.log.line(Level::Warn, AREA, &text);
        }
        started.map(|_| ())
    }

    fn apply_update(&mut self) {
        self.updater.apply();
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
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::mpsc;

    use wakeshot_core::command::HibernateEnd;
    use wakeshot_core::process::{Alive, FileTime, ProcessRef};
    use wakeshot_core::sim::{Sim, Start};
    use wakeshot_core::sources::vscode::{VsCodeSource, installs};
    use wakeshot_core::sources::{Activity, SourceId, WantedFile};

    use super::*;
    use crate::lockowner::LockOwners;
    use crate::power::hibernate::FakeHibernate;
    use crate::process::{FakeLiveness, ProcessEntry};
    use crate::store::Names;
    use crate::testing::{FakeEventSink, FakeSoundPlayer, MemoryLog};
    use crate::update::{FakeUpdateFeed, FeedCall};
    use crate::watch::agents::Agents;
    use crate::watch::read::TempDir;

    fn lines(log: &MemoryLog) -> Vec<(Level, String)> {
        log.lines
            .lock()
            .unwrap()
            .iter()
            .map(|(level, _, text)| (*level, text.clone()))
            .collect()
    }

    /// The start header has one line per fact of this PC, read for real, in area `start` (architecture section 8).
    #[test]
    fn the_start_header_names_this_pc() {
        let log = MemoryLog::default();
        log_start_header(&log);
        let got = log.lines.lock().unwrap().clone();
        let names: Vec<&str> = got
            .iter()
            .map(|(_, area, text)| {
                assert_eq!(*area, "start", "{text}");
                text.split_once(": ").map_or("", |(name, _)| name)
            })
            .collect();
        assert_eq!(
            names,
            [
                "Wakeshot",
                "Windows",
                "Processor",
                "PC",
                "Sleep",
                "Power",
                "Power plan",
                "Elevated"
            ]
        );
        let text = |name: &str| {
            got.iter()
                .find(|(_, _, t)| t.starts_with(&format!("{name}: ")))
                .map(|(_, _, t)| t.clone())
                .unwrap_or_default()
        };
        assert!(
            text("Wakeshot").contains(env!("CARGO_PKG_VERSION")),
            "{got:?}"
        );
        assert!(text("Windows").contains(", version 10.0."), "{got:?}");
        assert!(
            text("Sleep").starts_with("Sleep: Modern Standby "),
            "{got:?}"
        );
        assert!(text("Power plan").contains("lidAc "), "{got:?}");
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
            post_wait: Duration::ZERO,
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
            post_wait: Duration::ZERO,
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
    fn the_sleep_check_thread_posts_its_result_and_never_hibernates() {
        let deps = Arc::new(FakeHibernate::default());
        let (h, sink, _) = hibernator(deps.clone());
        h.start_check(CountdownReason::WorkEnded, 20, None)
            .unwrap()
            .join()
            .unwrap();
        assert_eq!(sink.events(), [Event::Done(Outcome::SleepCheck(Ok(())))]);
        assert!(
            !deps
                .calls()
                .contains(&crate::power::hibernate::HibernateCall::Hibernate),
            "{:?}",
            deps.calls()
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

    /// Fails the first `fail` posts with `code`, then takes them; counts every try.
    struct FlakySink {
        fail: u32,
        code: u32,
        tries: AtomicU32,
        posted: Mutex<Vec<Event>>,
    }

    impl FlakySink {
        fn new(fail: u32, code: u32) -> Arc<Self> {
            Arc::new(Self {
                fail,
                code,
                tries: AtomicU32::new(0),
                posted: Mutex::new(Vec::new()),
            })
        }
    }

    impl EventSink for FlakySink {
        fn post(&self, event: Event) -> Result<(), WinError> {
            if self.tries.fetch_add(1, Ordering::SeqCst) < self.fail {
                return Err(WinError {
                    call: "PostMessageW",
                    code: self.code,
                });
            }
            self.posted.lock().unwrap().push(event);
            Ok(())
        }
    }

    /// A hibernate thread whose lid check skips at once, over `sink`.
    fn run_over(sink: &Arc<FlakySink>) -> Vec<(Level, String)> {
        let log = Arc::new(MemoryLog::default());
        let h = Hibernator {
            deps: Arc::new(FakeHibernate {
                lid: Some(false),
                ..FakeHibernate::default()
            }),
            sound: Arc::new(FakeSoundPlayer::default()),
            sink: Arc::clone(sink) as Arc<dyn EventSink>,
            log: log.clone(),
            post_wait: Duration::ZERO,
        };
        h.start(CountdownReason::Floor, 20, None)
            .unwrap()
            .join()
            .unwrap();
        lines(&log)
    }

    #[test]
    fn a_hibernate_end_is_posted_again_while_the_queue_is_full() {
        let sink = FlakySink::new(2, ERROR_NOT_ENOUGH_QUOTA.0);
        let logged = run_over(&sink);
        assert_eq!(sink.tries.load(Ordering::SeqCst), 3);
        let posted = sink.posted.lock().unwrap().clone();
        assert!(
            matches!(
                posted.as_slice(),
                [Event::Done(Outcome::Hibernate(HibernateEnd::Skipped(_)))]
            ),
            "exactly one end: {posted:?}"
        );
        assert!(
            logged.contains(&(Level::Info, "hibernate result posted at try 3".to_owned())),
            "{logged:?}"
        );
        assert!(
            logged.iter().all(|(level, _)| *level != Level::Warn),
            "{logged:?}"
        );
    }

    #[test]
    fn a_hibernate_end_that_cannot_be_posted_is_logged() {
        // A queue that stays full: every try, then one line.
        let sink = FlakySink::new(u32::MAX, ERROR_NOT_ENOUGH_QUOTA.0);
        let logged = run_over(&sink);
        assert_eq!(sink.tries.load(Ordering::SeqCst), HIBERNATE_POST_TRIES);
        let warned: Vec<_> = logged.iter().filter(|(l, _)| *l == Level::Warn).collect();
        assert_eq!(warned.len(), 1, "{logged:?}");
        assert!(
            warned[0].1.starts_with(
                "hibernate result not posted: PostMessageW failed with 1816 at try 60"
            ),
            "{logged:?}"
        );

        // The window is gone (1400, the tray quits): no second try.
        let sink = FlakySink::new(u32::MAX, 1400);
        let logged = run_over(&sink);
        assert_eq!(sink.tries.load(Ordering::SeqCst), 1);
        assert!(
            logged.iter().any(|(level, t)| *level == Level::Warn
                && t.starts_with(
                    "hibernate result not posted: PostMessageW failed with 1400 at try 1"
                )),
            "{logged:?}"
        );
    }

    /// Hands each event to the test as it comes, so a test waits on a channel, not a sleep.
    struct ChannelSink(mpsc::Sender<Event>);

    impl EventSink for ChannelSink {
        fn post(&self, event: Event) -> Result<(), WinError> {
            let _ = self.0.send(event);
            Ok(())
        }
    }

    fn make_feed(feed: &FakeUpdateFeed) -> MakeFeed {
        let feed = feed.clone();
        Box::new(move || Box::new(feed) as Box<dyn UpdateFeed>)
    }

    /// An updater over the real thread, with what it posts and both logs.
    fn updater(
        feed: Option<MakeFeed>,
    ) -> (
        Updater,
        mpsc::Receiver<Event>,
        Arc<MemoryLog>,
        Arc<MemoryLog>,
    ) {
        let (events, received) = mpsc::channel();
        let log = Arc::new(MemoryLog::default());
        let loop_log = Arc::new(MemoryLog::default());
        let thread_log: Arc<dyn Log> = log.clone();
        let u = Updater::start(
            feed,
            Arc::new(ChannelSink(events)),
            &thread_log,
            loop_log.clone(),
        );
        (u, received, log, loop_log)
    }

    const WAIT: Duration = Duration::from_secs(10);

    #[test]
    fn a_feed_starts_the_thread_and_apply_update_reaches_it_once() {
        let feed = FakeUpdateFeed::default();
        let (u, events, log, loop_log) = updater(Some(make_feed(&feed)));
        assert!(u.handle.is_some(), "the thread runs");
        assert_eq!(
            lines(&log),
            [(
                Level::Info,
                "Wakeshot is installed, so it checks for updates".to_owned()
            )]
        );
        u.apply();
        assert_eq!(
            events.recv_timeout(WAIT),
            Ok(Event::Done(Outcome::Update(Ok(()))))
        );
        assert_eq!(feed.calls(), [FeedCall::Apply]);
        u.stop();
        assert_eq!(feed.calls(), [FeedCall::Apply], "nothing after the stop");
        assert!(events.try_recv().is_err(), "one answer only");
        assert!(lines(&loop_log).is_empty(), "{:?}", lines(&loop_log));
    }

    #[test]
    fn no_feed_starts_nothing_and_apply_update_answers_err() {
        let (u, events, log, loop_log) = updater(None);
        assert!(u.handle.is_none());
        assert!(lines(&log).is_empty(), "{:?}", lines(&log));
        u.apply();
        assert_eq!(
            events.recv_timeout(WAIT),
            Ok(Event::Done(Outcome::Update(Err(NO_UPDATE_THREAD))))
        );
        assert_eq!(
            lines(&loop_log),
            [(
                Level::Warn,
                "update not applied: no update thread runs".to_owned()
            )]
        );
        u.stop();
    }

    #[test]
    fn an_update_thread_that_ended_gets_err_from_the_loop() {
        // A successful apply ends the thread (the updater then waits for this process); a second apply is answered
        // here, so the core is never left waiting.
        let feed = FakeUpdateFeed::default();
        let (u, events, _, loop_log) = updater(Some(make_feed(&feed)));
        u.apply();
        assert_eq!(
            events.recv_timeout(WAIT),
            Ok(Event::Done(Outcome::Update(Ok(()))))
        );
        let deadline = std::time::Instant::now() + WAIT;
        while u.handle.as_ref().is_some_and(UpdateHandle::apply) {
            assert!(
                std::time::Instant::now() < deadline,
                "the thread did not end"
            );
            thread::yield_now();
        }
        u.apply();
        assert_eq!(
            events.recv_timeout(WAIT),
            Ok(Event::Done(Outcome::Update(Err(NO_UPDATE_THREAD))))
        );
        assert_eq!(feed.calls(), [FeedCall::Apply]);
        assert_eq!(lines(&loop_log).len(), 1);
        u.stop();
    }

    #[test]
    fn a_lost_update_answer_is_logged() {
        let sink = Arc::new(FakeEventSink {
            fail_with: Some(WinError {
                call: "PostMessageW",
                code: 1400,
            }),
            ..FakeEventSink::default()
        });
        let loop_log = Arc::new(MemoryLog::default());
        let log: Arc<dyn Log> = Arc::new(MemoryLog::default());
        let u = Updater::start(None, sink, &log, loop_log.clone());
        u.apply();
        let logged = lines(&loop_log);
        assert_eq!(logged.len(), 2, "{logged:?}");
        assert_eq!(
            logged[1],
            (
                Level::Warn,
                "update answer not posted: PostMessageW failed with 1400".to_owned()
            )
        );
    }

    #[test]
    fn updated_comes_after_started_locked_and_journal_corrupt_never_first() {
        let started = Start::default().into_event();
        let updated = Event::Updated {
            version: "1.1.0".to_owned(),
        };
        let v = || Some("1.1.0".to_owned());
        assert_eq!(
            first_events(started.clone(), false, false, v()),
            [started.clone(), updated.clone()]
        );
        assert_eq!(
            first_events(started.clone(), true, false, v()),
            [
                started.clone(),
                Event::Session(SessionChange::Locked),
                updated.clone()
            ]
        );
        assert_eq!(
            first_events(started.clone(), false, true, v()),
            [started.clone(), Event::JournalCorrupt, updated.clone()]
        );
        assert_eq!(
            first_events(started.clone(), true, true, v()),
            [
                started.clone(),
                Event::Session(SessionChange::Locked),
                Event::JournalCorrupt,
                updated
            ]
        );
        assert_eq!(
            first_events(started.clone(), false, false, None),
            [started],
            "no restart, no Updated"
        );
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
    fn the_tray_watches_claude_code_and_codex_cli_where_each_writes() {
        let home = Path::new(r"C:\Users\x");
        for (codex_env, codex_dir, text) in [
            (
                None,
                r"C:\Users\x\.codex",
                "Codex CLI is watched in .codex of the profile folder",
            ),
            (
                Some(""),
                r"C:\Users\x\.codex",
                "Codex CLI is watched in .codex of the profile folder",
            ),
            (
                Some(r"D:\cx"),
                r"D:\cx",
                "Codex CLI is watched in the folder CODEX_HOME names",
            ),
        ] {
            let memory = Arc::new(MemoryLog::default());
            let log: Arc<dyn Log> = memory.clone();
            let env = AgentEnv {
                codex_home: codex_env.map(OsString::from),
                ..AgentEnv::default()
            };
            let picked = sources_in(home, env, &log);
            let ids: Vec<SourceId> = picked.iter().map(|s| s.id()).collect();
            assert_eq!(
                ids[..2],
                [SourceId::ClaudeCode, SourceId::Codex],
                "{codex_env:?}"
            );
            let Some(WantedFile::Names { dir, .. }) = picked[1].wanted().into_iter().next() else {
                panic!("Codex starts with its lock folder");
            };
            assert_eq!(dir, Path::new(codex_dir).join("thread-writer-locks"));
            assert_eq!(
                lines(&memory)[..2],
                [
                    (
                        Level::Info,
                        "Claude Code is watched in .claude of the profile folder".to_owned()
                    ),
                    (Level::Info, text.to_owned()),
                ]
            );
        }
    }

    /// Where a source reads first: Copilot's lock folder, OpenCode's database.
    fn first_place(source: &dyn AgentSource) -> PathBuf {
        match source.wanted().into_iter().next() {
            Some(WantedFile::Names { dir, .. }) => dir,
            Some(WantedFile::Query { db, .. }) => db,
            _ => panic!("{:?} starts with another want", source.id()),
        }
    }

    #[test]
    fn the_tray_watches_copilot_cli_and_opencode_where_each_writes() {
        let home = Path::new(r"C:\Users\x");
        let unset = AgentEnv::default();
        let empty = AgentEnv {
            copilot_home: Some(OsString::new()),
            xdg_data_home: Some(OsString::new()),
            ..AgentEnv::default()
        };
        let profile = [
            r"C:\Users\x\.copilot\session-state",
            r"C:\Users\x\.local\share\opencode\opencode.db",
        ];
        let profile_lines = [
            "Copilot CLI is watched in .copilot of the profile folder",
            "OpenCode is watched in .local\\share of the profile folder",
        ];
        let all_set = AgentEnv {
            copilot_home: Some(r"D:\cp".into()),
            xdg_data_home: Some(r"D:\xdg".into()),
            ..AgentEnv::default()
        };
        for (env, places, texts) in [
            (unset, profile, profile_lines),
            (empty, profile, profile_lines),
            (
                all_set,
                [r"D:\cp\session-state", r"D:\xdg\opencode\opencode.db"],
                [
                    "Copilot CLI is watched in the folder COPILOT_HOME names",
                    "OpenCode is watched in the folder XDG_DATA_HOME names",
                ],
            ),
        ] {
            let shown = format!("{env:?}");
            let memory = Arc::new(MemoryLog::default());
            let log: Arc<dyn Log> = memory.clone();
            let picked = sources_in(home, env, &log);
            let ids: Vec<SourceId> = picked.iter().map(|s| s.id()).collect();
            assert_eq!(
                ids,
                [
                    SourceId::ClaudeCode,
                    SourceId::Codex,
                    SourceId::Copilot,
                    SourceId::OpenCode
                ],
                "{shown}"
            );
            let found: Vec<PathBuf> = picked[2..].iter().map(|s| first_place(&**s)).collect();
            let want: Vec<PathBuf> = places.iter().map(PathBuf::from).collect();
            assert_eq!(found, want, "{shown}");
            let want: Vec<(Level, String)> = texts
                .iter()
                .map(|text| (Level::Info, (*text).to_owned()))
                .collect();
            assert_eq!(lines(&memory)[2..], want, "{shown}");
        }
    }

    // What the tray watches, read by the watch's own `Agents` from real files in a scratch profile, as the watch
    // thread reads them at start and on each tick, with the events fed to the core.

    /// The v1 agents. VS Code's Local chats, Antigravity and Cursor are out of v1 (`watched_sources`): this fails if
    /// one comes back.
    #[test]
    fn the_tray_starts_the_v1_sources_only() {
        let home = Path::new(r"C:\Users\x");
        for profile in [Some(home), None] {
            let log: Arc<dyn Log> = Arc::new(MemoryLog::default());
            let ids: Vec<SourceId> = watched_sources(profile, AgentEnv::default(), &log)
                .iter()
                .map(|s| s.id())
                .collect();
            let want: &[SourceId] = match profile {
                Some(_) => &[
                    SourceId::ClaudeCode,
                    SourceId::Codex,
                    SourceId::Copilot,
                    SourceId::OpenCode,
                ],
                None => &[],
            };
            assert_eq!(ids, want, "{profile:?}");
        }
    }

    /// A Cursor chat Wakeshot's hook marked working, from an earlier build's hook folder, with its `Cursor.exe`
    /// running: it holds through Cursor's own source (the control), and holds nothing through the tray, which reads
    /// no folder under `agents`.
    #[test]
    fn a_cursor_chat_holds_nothing_through_the_tray() {
        let dir = TempDir::new("tray-cursor");
        let data = dir.path().join("data");
        let home = dir.path().join("home");
        let chat = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(r"..\wakeshot-core\tests\fixtures\cursor\live\state-working.json");
        dir.write(
            r"data\agents\cursor\7ae3e396-0000-4000-8000-000000000001.json",
            &std::fs::read(&chat).unwrap(),
        );
        // The pid and start the live chat file names.
        let editor = running(4120, 134_384_524_042_912_323, "Cursor.exe");
        let held = ProcessRef {
            pid: editor.pid,
            start: editor.start,
        };
        let live = FakeLiveness::default();
        live.processes.lock().unwrap().push(editor);
        live.alive.lock().unwrap().push((held, Alive::Yes));

        let control: Vec<Box<dyn AgentSource + Send>> =
            vec![Box::new(wakeshot_core::sources::cursor::source(&data))];
        let mut agents = Agents::with_owners(
            control,
            Arc::new(MemoryLog::default()),
            Box::new(HeldBy(Vec::new())),
        );
        let mut sim = started_core();
        assert_eq!(
            watch_round(&mut agents, &live, &mut sim),
            [(SourceId::Cursor, Activity::Working, Alive::Yes)]
        );
        assert!(sim.requests().system, "{}", sim.trace());

        let log: Arc<dyn Log> = Arc::new(MemoryLog::default());
        let tray = watched_sources(Some(&home), AgentEnv::default(), &log);
        let mut agents = Agents::with_owners(tray, log, Box::new(HeldBy(Vec::new())));
        let mut sim = started_core();
        assert_eq!(watch_round(&mut agents, &live, &mut sim), []);
        assert!(!sim.requests().any(), "{}", sim.trace());
        let folders = agents.folders();
        assert!(
            folders.iter().all(|f| !f.dir.starts_with(&data)),
            "no folder of the data folder is watched: {folders:?}"
        );
    }

    /// The Local chat VS Code left working when its window closed mid-turn (seen live): nothing on disk ever ends it.
    const LOCAL_CHAT: &str = r"home\AppData\Roaming\Code\User\workspaceStorage\0a1b2c3d\chatSessions\0b1c2d3e-0000-4000-8000-0000000000aa.jsonl";
    /// The live Copilot harness chat's session folder.
    const HARNESS: &str = r"home\.copilot\session-state\0199c0de-0000-4000-9000-000000000001";
    /// The harness log's lines up to the first command it runs (`sleep 60`, working); its Stop is the next two,
    /// `abort` and `assistant.turn_end`.
    const HARNESS_WORKING: usize = 18;
    const HARNESS_STOP: usize = 2;

    fn vscode_live_fixture(rel: &str) -> Vec<u8> {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(r"..\wakeshot-core\tests\fixtures\vscode\live")
            .join(rel);
        std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
    }

    /// Restart Manager's answer for any lock file.
    struct HeldBy(Vec<ProcessRef>);

    impl LockOwners for HeldBy {
        fn holders(&self, _: &Path) -> Result<Vec<ProcessRef>, WinError> {
            Ok(self.0.clone())
        }
    }

    fn running(pid: u32, start: u64, image: &str) -> ProcessEntry {
        ProcessEntry {
            pid,
            parent: 0,
            start: FileTime(start),
            image: image.to_owned(),
        }
    }

    /// A scratch PC with the Local chat and a `Code.exe` that started before its send, so the chat holds whenever
    /// VS Code's source runs.
    fn pc_with_a_local_chat(tag: &str) -> (TempDir, FakeLiveness) {
        let dir = TempDir::new(tag);
        dir.write(
            LOCAL_CHAT,
            &vscode_live_fixture("close-while-working.jsonl"),
        );
        let live = FakeLiveness::default();
        live.processes
            .lock()
            .unwrap()
            .push(running(4100, 1, "Code.exe"));
        (dir, live)
    }

    fn started_core() -> Sim {
        let mut sim = Sim::new();
        sim.start();
        sim
    }

    /// One watch round, its events fed to the core. Gives each session of the events: source, activity, liveness.
    fn watch_round(
        agents: &mut Agents,
        live: &FakeLiveness,
        sim: &mut Sim,
    ) -> Vec<(SourceId, Activity, Alive)> {
        let retries = agents.on_tick();
        assert!(retries.is_empty(), "{retries:?}");
        let events = agents.snapshots(live, sim.now()).events;
        assert!(!events.is_empty(), "each round here changes something");
        let mut seen = Vec::new();
        for event in events {
            if let Event::Sources { sessions, .. } = &event {
                seen.extend(
                    sessions
                        .iter()
                        .map(|(facts, alive)| (facts.key.source, facts.activity, *alive)),
                );
            }
            sim.send(event);
        }
        seen
    }

    #[test]
    fn a_local_vs_code_chat_holds_nothing_through_the_tray() {
        let (dir, live) = pc_with_a_local_chat("tray-vscode-local");
        let home = dir.path().join("home");

        // The proof the file and the process would hold: VS Code's own source, which the tray no longer starts.
        let vscode: Vec<Box<dyn AgentSource + Send>> =
            vec![Box::new(VsCodeSource::new(installs(&home, None, None)))];
        let mut agents = Agents::with_owners(
            vscode,
            Arc::new(MemoryLog::default()),
            Box::new(HeldBy(Vec::new())),
        );
        let mut sim = started_core();
        assert_eq!(
            watch_round(&mut agents, &live, &mut sim),
            [(SourceId::VsCode, Activity::Working, Alive::Yes)]
        );
        assert!(sim.requests().system, "{}", sim.trace());

        let log: Arc<dyn Log> = Arc::new(MemoryLog::default());
        let tray = watched_sources(Some(&home), AgentEnv::default(), &log);
        let mut agents = Agents::with_owners(tray, log, Box::new(HeldBy(Vec::new())));
        let mut sim = started_core();
        assert_eq!(watch_round(&mut agents, &live, &mut sim), []);
        assert!(!sim.requests().any(), "{}", sim.trace());
        let roaming = home.join(r"AppData\Roaming");
        let folders = agents.folders();
        assert!(
            folders.iter().all(|f| !f.dir.starts_with(&roaming)),
            "no VS Code folder is watched: {folders:?}"
        );
    }

    #[test]
    fn a_copilot_harness_chat_in_vs_code_holds_through_the_tray_until_its_stop() {
        use std::io::Write as _;

        let (dir, live) = pc_with_a_local_chat("tray-vscode-harness");
        let home = dir.path().join("home");
        let harness = vscode_live_fixture(r"harness\stop-approve-deny-close.jsonl");
        let lines: Vec<&[u8]> = harness.split_inclusive(|&b| b == b'\n').collect();
        let events_file = dir.write(
            &format!(r"{HARNESS}\events.jsonl"),
            &lines[..HARNESS_WORKING].concat(),
        );
        // VS Code's `copilot-runtime.exe` names both in-use files after its pid and holds the `.hold` open.
        dir.write(&format!(r"{HARNESS}\inuse.2728.hold"), b"");
        dir.write(&format!(r"{HARNESS}\inuse.2728.lock"), b"2728");
        let runtime = running(2728, 5, "copilot-runtime.exe");
        let held = ProcessRef {
            pid: runtime.pid,
            start: runtime.start,
        };
        live.processes.lock().unwrap().push(runtime);
        live.alive.lock().unwrap().push((held, Alive::Yes));

        let log: Arc<dyn Log> = Arc::new(MemoryLog::default());
        let tray = watched_sources(Some(&home), AgentEnv::default(), &log);
        let mut agents = Agents::with_owners(tray, log, Box::new(HeldBy(vec![held])));
        let mut sim = started_core();
        assert_eq!(
            watch_round(&mut agents, &live, &mut sim),
            [(SourceId::Copilot, Activity::Working, Alive::Yes)]
        );
        assert!(sim.requests().system, "{}", sim.trace());

        // Stop. The hold ends with the harness turn, while the Local chat on disk still says working.
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&events_file)
            .unwrap();
        file.write_all(&lines[HARNESS_WORKING..HARNESS_WORKING + HARNESS_STOP].concat())
            .unwrap();
        drop(file);
        assert_eq!(
            watch_round(&mut agents, &live, &mut sim),
            [(SourceId::Copilot, Activity::Idle, Alive::Yes)]
        );
        assert!(!sim.requests().any(), "{}", sim.trace());
    }

    /// The call `serve` makes. This test runs from `target\`, as a dev tray does, so its thread adds no hook folder to
    /// any agent and posts nothing; a dry run starts no thread.
    #[test]
    fn a_tray_outside_velopacks_folder_adds_no_agent_hooks() {
        use crate::store::DataDir;

        let dir = TempDir::new("tray-agent-hooks");
        for dry_run in [false, true] {
            let boot = Boot::at(
                DataDir::at(dir.path().to_path_buf()),
                Names::test("tray-agent-hooks"),
                dry_run,
            )
            .unwrap();
            let events = Arc::new(FakeEventSink::default());
            let sink: Arc<dyn EventSink> = events.clone();
            let memory = Arc::new(MemoryLog::default());
            let log: Arc<dyn Log> = memory.clone();
            let started = start_agent_hooks(&boot, &sink, &log);
            assert_eq!(started.is_some(), !dry_run);
            if let Some(thread) = started {
                thread.join().unwrap();
            }
            assert!(events.events().is_empty(), "dry run {dry_run}");
            let said: Vec<String> = lines(&memory).into_iter().map(|(_, text)| text).collect();
            let want: &[&str] = match dry_run {
                false => {
                    &["this tray is not Velopack's install, so it touches no agent's hook folder"]
                }
                true => &[],
            };
            assert_eq!(said, want, "dry run {dry_run}");
            assert!(
                !dir.path().join("agent-hooks.json").exists(),
                "no record either"
            );
        }
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
        // Never shows a notice here, so it makes no shell call.
        let notifier = Arc::new(BalloonNotifier::new(crate::tray::IconIdentity {
            hwnd: 0x1234,
            guid: IconKind::Demo.guid(),
        }));
        let mut icon_hook = icon_notes(
            Arc::new(QueuedLog(Arc::clone(&io))),
            notifier,
            Arc::clone(&io),
        );
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

    #[test]
    fn notices_refused_before_the_add_go_to_the_io_thread_again() {
        let io = IoQueue::new();
        let log = MemoryLog::default();
        show_again(Vec::new(), &io, &log);
        assert_eq!(io.pop(0), None);
        assert!(lines(&log).is_empty(), "nothing to say");

        show_again(vec![Notice::FirstRun, Notice::RestoreFailed], &io, &log);
        let popped: Vec<Command> = std::iter::from_fn(|| io.pop(0)).collect();
        assert_eq!(
            popped,
            [
                Command::Notify(Notice::FirstRun),
                Command::Notify(Notice::RestoreFailed),
            ]
        );
        assert_eq!(
            lines(&log),
            [(
                Level::Info,
                "the icon is added: 2 notices refused before it are shown again".to_owned()
            )]
        );
    }
}
