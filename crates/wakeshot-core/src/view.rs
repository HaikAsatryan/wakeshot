//! What the user sees and hears, as data. The core builds it (`present`); the Windows layer draws and plays it. Every
//! user-facing word comes from `text` (decision 1: English only, no translation layer), so wording changes stay
//! there.
//!
//! Owner: contracts card. Each type's own doc names the track that owns its meaning.

use crate::settings::SettingKey;

/// The tray icon's three states (decision 34).
///
/// Owner: tray track (Phase 5) draws one icon per state for each theme, high contrast mode and scale.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IconState {
    Holding,
    Idle,
    Off,
}

/// Everything the tray shows, rebuilt whole on each change so the drawing side keeps no state of its own.
///
/// Owner: core track (Phase 2) builds it; the tray track (Phase 5) draws it. `tooltip` is final text; the tray cuts
/// it only where Windows' tooltip limit forces it. `menu` is the menu to show on the next click.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrayView {
    pub icon: IconState,
    pub tooltip: String,
    pub menu: MenuModel,
}

/// The menu as a list of items, so how it is drawn (`p1-tray-menu-look`) never changes the core.
///
/// Owner: core track (Phase 2) builds it; any `MenuView` draws it as given, in order, with the texts as given.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MenuModel {
    pub items: Vec<MenuItem>,
}

/// One menu row. `Label` is a line that cannot be picked (the "Holding awake: ..." status); its `icon` is the tray
/// state it describes, which the drawing side shows beside it as the tray glyph (Haik, 2026-11-05), `None` for a
/// line with no state. `checked` marks the current choice; the drawing side picks the mark (tick or bullet). A text
/// may hold `&` before its access key; `&&` shows one `&` (the Win32 rule).
///
/// Owner: core track (Phase 2) builds it; tray track (Phase 5) draws it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MenuItem {
    Label {
        text: String,
        icon: Option<IconState>,
    },
    Separator,
    Action {
        text: String,
        command: MenuCommand,
        checked: bool,
        enabled: bool,
    },
    Submenu {
        text: String,
        items: Vec<MenuItem>,
    },
}

/// A Keep awake choice (product, UX): Off, a number of minutes, or Until I turn it off.
///
/// Owner: core track (Phase 2). The menu offers 30, 60, 120 and 240 minutes; the CLI's `30m|1h|2h|4h|on|off` maps
/// onto the same values.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum KeepAwake {
    Off,
    Minutes(u32),
    UntilOff,
}

/// A user's choice from the menu, or from the CLI where the CLI asks the tray (`restore-request.json` gives
/// `Restore`).
///
/// Owner: core track (Phase 2). A `MenuView` returns exactly the `command` of the item picked, as `Event::Menu`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MenuCommand {
    Automatic(bool),
    KeepAwake(KeepAwake),
    KeepScreenOn(bool),
    Restore,
    OpenLog,
    Quit,
}

/// A notice for something Wakeshot did on its own or could not do (decision 0).
///
/// Owner: core track (Phase 2) decides when; tray track (Phase 5) words it in `text::notice` and shows it.
/// `StepBack` carries the battery floor it stepped back at; `ClosedLidReport` how long the PC ran with the lid closed
/// (`mono_ms`) and the labels of what held it; `UpdateWakeshot` the agent label. `JournalLost`: the journal could not
/// be read, so the keys in `reset` were set to Windows' default (product "User settings safety" 7); `failed`: a call of
/// that reset failed, so a value of Wakeshot's may be left. `IdleLocked`: Keep screen on locked the PC after the
/// user's idle time. `Updated`: Wakeshot updated itself to `version` (ship Question 3, Haik 2026-11-03: yes); the
/// answer to `Event::Updated`. `HooksAdded`: Wakeshot added its hook folder to `agent` (a label from Wakeshot's own
/// table, never file text), the answer to `Event::HooksAdded` (`docs/agent-hooks.md`, "Adding").
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Notice {
    FirstRun,
    UserChanged(SettingKey),
    RestoredByUser,
    StepBack {
        floor_pct: u8,
    },
    RequestsOnly,
    CannotHold,
    RestoreFailed,
    HibernateFailed,
    LockFailed,
    ClosedLidReport {
        ran_ms: u64,
        held_by: Vec<String>,
    },
    UpdateWakeshot {
        agent: &'static str,
    },
    JournalLost {
        reset: Vec<SettingKey>,
        failed: bool,
    },
    IdleLocked,
    Updated {
        version: String,
    },
    HooksAdded {
        agent: &'static str,
    },
}

/// The three sounds (decision 19). Each is one of Wakeshot's own chimes, built into the exe.
///
/// Owner: tray track (Phase 5) makes the chimes; the core track (Phase 2) decides when they play.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Cue {
    LidClosedHolding,
    GoingToSleep,
    BatteryFloor,
}
