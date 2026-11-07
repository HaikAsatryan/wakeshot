//! Everything the core reacts to (architecture section 3). Every event comes with a `Now`.

use crate::command::{Command, Failure, Outcome};
use crate::files::Config;
use crate::holds::HoldFacts;
use crate::process::{Alive, ProcessRef};
use crate::settings::{Guid, Journal, OwnedKey, PlanValues};
use crate::sources::{SessionFacts, SourceId};
use crate::view::MenuCommand;

/// A change to this Windows session, from `WM_WTSSESSION_CHANGE` and `WM_QUERYENDSESSION` / `WM_ENDSESSION`.
///
/// `ConsoleLeft` and `ConsoleBack` are this session leaving and getting back the PC's own screen and keyboard
/// (`WTS_CONSOLE_DISCONNECT`, `WTS_CONSOLE_CONNECT`); the core restores while away.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SessionChange {
    Locked,
    Unlocked,
    Ending,
    EndCancelled,
    Ended,
    ConsoleLeft,
    ConsoleBack,
}

/// One input to the core. Snapshots (`Sources`, `Holds`) are full, so a lost event is mended by the next one.
///
/// The core may rely on:
///
/// - `Started` comes first and once. Its `config` wins over the one `Core::new` got. `journal`: the entries this
///   version knows, `None` when there are none. `journal_unread`: `journal.json` did not read at start (then
///   `journal` is `None`), so what it holds is not known and the core retries it as `Restore { why: Leftover }` on
///   each tick, for two minutes, then sends `ResetLost`. `owner`: this tray holds the settings mutex. `on_console`:
///   this session is on the PC's own screen. `tray`: this tray's own process, for `state.json` and the Keep awake
///   hold. `first_run`: `config.json` was missing at start, so the core shows `Notice::FirstRun` and writes the config.
/// - A tray that starts while the session is locked gets `Session(Locked)` right after `Started`, before
///   `JournalCorrupt` and `Updated`: the only way the core learns of a lock that began before the tray.
/// - `Sources` pairs each session with the adapter's liveness answer; the parser never checks liveness.
/// - `Plan(Err)` is a failed read: never values, and it releases nothing.
/// - `ActivePlan`: the active plan changed (`GUID_ACTIVE_POWERSCHEME`). The core sends `ReadPlan` when it differs
///   from the last read's `active` and never acts on it alone.
/// - `UserChanged` comes after the IO thread already dropped that entry from the journal.
/// - `JournalCorrupt`: a corrupt journal was set aside, once per corrupt file (at start for one left by restore mode,
///   the guard or a hook; at once for the tray's own restore). The journal is lost: the core sends
///   `ResetLost { damaged: true }` (product "User settings safety" 7).
/// - `Suspending`: `PBT_APMSUSPEND` from the suspend-resume callback, the only route that shows a Modern Standby sleep
///   (research, "Idle sleep with the lid closed"). The core counts it as a sleep at the next `Resumed`, or, when that
///   post was lost, once the next `Tick` shows the wall clock moved past run time.
/// - `UpdateReady`: an update is downloaded, at most once per version per run.
/// - `Updated`: Velopack restarted the tray after it installed `version`. At most once per run, at start, after
///   `Started` (and after `JournalCorrupt` when both come); the core answers with `Notify(Notice::Updated)`.
/// - `HooksAdded`: the tray added Wakeshot's hook folder to `agent` (its label, such as "Cursor"), once per folder it
///   adds (`docs/agent-hooks.md`, "Adding"). The core answers each with `Notify(Notice::HooksAdded)`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    Started {
        config: Config,
        journal: Option<Journal>,
        journal_unread: bool,
        owner: bool,
        on_console: bool,
        tray: ProcessRef,
        first_run: bool,
    },
    SettingsOwner,
    Tick,
    Sources {
        source: SourceId,
        sessions: Vec<(SessionFacts, Alive)>,
    },
    Holds(Vec<HoldFacts>),
    ConfigChanged(Config),
    Menu(MenuCommand),
    Lid {
        closed: bool,
    },
    Power {
        ac: bool,
        battery_pct: Option<u8>,
    },
    Plan(Result<PlanValues, Failure>),
    ActivePlan(Guid),
    UserChanged(OwnedKey),
    JournalCorrupt,
    InputIdle {
        ms: u64,
    },
    Suspending,
    Resumed,
    Session(SessionChange),
    UpdateReady {
        version: String,
    },
    Updated {
        version: String,
    },
    HooksAdded {
        agent: &'static str,
    },
    Done(Outcome),
}

// Events come from four threads and commands go to the IO thread. A variant that is not `Send` would break every
// poster, so it fails the core's own build instead.
const _: () = {
    const fn is_send<T: Send>() {}
    is_send::<Event>();
    is_send::<Command>();
};
