//! The core's state, private to the core: one struct owned by the loop thread (architecture section 3, "State is one
//! struct"). Every field the three core cards need is declared here, so a later card fills a field instead of
//! reshaping the struct; a field a later card fills names that card and carries an `expect(dead_code)` it removes.
//!
//! Owner: `p2-core-holds`, then `p2-core-settings`, then `p2-core-lid`.

use std::collections::{BTreeMap, BTreeSet};

use crate::command::CountdownReason;
use crate::files::{Config, StateFile};
use crate::holds::{HoldFacts, HoldId, HoldKind};
use crate::policy::floor;
use crate::policy::lid::SourceLock;
use crate::policy::requests::Requests;
use crate::policy::sessions::SessionMemory;
use crate::process::{Alive, FileTime, ProcessRef};
use crate::settings::{JournalEntry, PlanValues, SettingKey};
use crate::sources::{SessionFacts, SessionKey, SourceId};
use crate::time::Deadline;
use crate::view::{KeepAwake, Notice, TrayView};

/// The Keep awake hold file the tray writes (architecture section 6). Found again by its kind, never by this name,
/// since the parser ignores case (`holds::file::kind_of`).
pub(crate) const KEEP_AWAKE_FILE: &str = "keep-awake.json";

#[derive(Debug)]
pub(crate) struct State {
    /// `Event::Started` came. Every event before it is dropped: it has no tray to name and no config yet, and the
    /// next full snapshot mends a lost one.
    pub started: bool,
    /// `Started.tray`: `state.json` and the Keep awake hold name it.
    pub tray: ProcessRef,
    pub automatic: bool,
    /// This tray holds the settings mutex.
    #[expect(dead_code, reason = "p2-core-settings fills it")]
    pub owner: bool,
    /// This session is on the PC's own screen and keyboard.
    pub on_console: bool,

    /// The last snapshot of each source, with what the core remembers per session.
    pub sessions: BTreeMap<SourceId, BTreeMap<SessionKey, Session>>,
    /// The last `Holds` snapshot.
    pub files: BTreeMap<HoldId, FileHold>,
    /// Hold files with a `DeleteHold` not yet answered, so a snapshot that comes first asks no second time.
    pub deleting: BTreeSet<HoldId>,
    /// Which Keep awake length the current `keep-awake.json` stands for.
    pub keep_awake: Option<KeepAwakeMemo>,
    /// This tray wrote `keep-awake.json` in this run. From then on a Gone one in a snapshot can only be a view from
    /// before that write (the file names this tray, which lives), and deleting on it would delete the new file.
    pub keep_awake_written: bool,
    /// Agents already given "update Wakeshot" since this start (decision 37).
    pub update_shown: BTreeSet<&'static str>,
    /// When each source last logged a format problem (run time), for at most one line a minute.
    pub format_logged_mono: BTreeMap<SourceId, u64>,

    /// The requests as last sent.
    pub requests: Requests,
    /// The last answer to `Requests` or `Retake` failed while a hold wanted them.
    pub requests_failed: bool,
    /// From the first counted hold to the last.
    pub episode: Option<Episode>,
    /// The last plan read.
    #[expect(dead_code, reason = "p2-core-settings fills it")]
    pub plan: Option<PlanValues>,
    /// A copy of the journal from `Started`, `Done(Applied)` and `Done(Restored)`, less the keys `UserChanged` names.
    pub journal: Vec<JournalEntry>,
    /// `Started` brought a journal from an earlier run: restore it on each tick until it is gone (track note 9).
    #[expect(dead_code, reason = "p2-core-settings fills it")]
    pub leftover: bool,
    /// Settings are blocked for this run (access denied, 5 or 1260): requests only.
    pub requests_only_run: bool,
    /// `Session(Ending)` came and no `EndCancelled` yet: no `Apply`.
    #[expect(dead_code, reason = "p2-core-settings fills it")]
    pub session_ending: bool,
    /// `None` until the first `Lid` event.
    #[expect(dead_code, reason = "p2-core-lid fills it")]
    pub lid_closed: Option<bool>,
    /// `None` until the first `Power` event; the first one is no source change (`policy::lid::source_change`).
    #[expect(dead_code, reason = "p2-core-lid fills it")]
    pub ac: Option<bool>,
    #[expect(dead_code, reason = "p2-core-lid fills it")]
    pub battery_pct: Option<u8>,
    /// From the plan's energy saver threshold (`policy::floor::floor_pct`).
    #[expect(dead_code, reason = "p2-core-lid fills it")]
    pub floor_pct: u8,
    /// On battery under the floor: no requests, no apply, until AC.
    pub stepped_back: bool,
    /// A lock waiting for a new power source to last (`policy::lid::settle`).
    #[expect(dead_code, reason = "p2-core-lid fills it")]
    pub source_lock: Option<SourceLock>,
    /// The countdown to hibernate.
    #[expect(dead_code, reason = "p2-core-lid fills it")]
    pub countdown: Option<Countdown>,
    /// For the idle clamp at resume (`policy::ending::clamped_idle_ms`).
    #[expect(dead_code, reason = "p2-core-lid fills it")]
    pub last_resume_mono: Option<u64>,
    /// `Session(Locked)` came and no `Unlocked` yet.
    #[expect(dead_code, reason = "p2-core-lid fills it")]
    pub session_locked: bool,
    /// When the Keep screen on idle lock last locked (`policy::lock::idle_lock`).
    #[expect(dead_code, reason = "p2-core-lid fills it")]
    pub idle_locked_at_mono: Option<u64>,
    /// Notices raised while locked or off the console, sent at `Unlocked` or `ConsoleBack` (track note 18).
    #[expect(dead_code, reason = "p2-core-lid fills it")]
    pub held_notices: Vec<Notice>,

    /// The tray as last sent.
    pub view: Option<TrayView>,
    /// `state.json` as last sent, and when (run time). `None` after a failed write, so the next step sends it again.
    pub state_file: Option<StateFile>,
    pub state_written_mono: Option<u64>,
    /// `next_deadline`'s answer, worked out at the end of each `handle`.
    pub next: Option<Deadline>,
}

impl State {
    pub fn new(config: &Config) -> Self {
        Self {
            started: false,
            tray: ProcessRef {
                pid: 0,
                start: FileTime(0),
            },
            automatic: config.automatic,
            owner: false,
            on_console: true,
            sessions: BTreeMap::new(),
            files: BTreeMap::new(),
            deleting: BTreeSet::new(),
            keep_awake: None,
            keep_awake_written: false,
            update_shown: BTreeSet::new(),
            format_logged_mono: BTreeMap::new(),
            requests: Requests::NONE,
            requests_failed: false,
            episode: None,
            plan: None,
            journal: Vec::new(),
            leftover: false,
            requests_only_run: false,
            session_ending: false,
            lid_closed: None,
            ac: None,
            battery_pct: None,
            floor_pct: floor::DEFAULT_PCT,
            stepped_back: false,
            source_lock: None,
            countdown: None,
            last_resume_mono: None,
            session_locked: false,
            idle_locked_at_mono: None,
            held_notices: Vec::new(),
            view: None,
            state_file: None,
            state_written_mono: None,
            next: None,
        }
    }
}

/// One session of the last snapshot: its facts, the adapter's liveness answer, and the policy's memory.
#[derive(Clone, Debug)]
pub(crate) struct Session {
    pub facts: SessionFacts,
    pub alive: Alive,
    pub memory: SessionMemory,
}

/// One hold file of the last snapshot, and its last `Yes` or `Gone` (`policy::hold_files`, `last_alive`).
#[derive(Clone, Debug)]
pub(crate) struct FileHold {
    pub facts: HoldFacts,
    pub alive: Alive,
}

/// The `keep-awake.json` whose `expires` is this one stands for `choice`, so the menu checks the length the user
/// picked, not the minutes left (`present::Snapshot::keep_awake`). Set at a menu pick to the `expires` it writes, and
/// at the first sight of a file the tray did not write (`present::keep_awake_choice`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct KeepAwakeMemo {
    pub expires_wall_ms: Option<i64>,
    pub choice: KeepAwake,
}

/// What holds, as the log and the closed-lid report name it: the kind, and the agent's label or the file's reason.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Held {
    pub kind: HoldKind,
    pub label: Option<String>,
}

/// From the first counted hold to the last (architecture section 3).
#[derive(Clone, Debug, Default)]
pub(crate) struct Episode {
    /// What holds now, for the log line of each change.
    pub holding: BTreeSet<Held>,
    /// Everything that held in this episode, for the hold end's log line and the closed-lid report.
    pub held_by: BTreeSet<Held>,
    /// `Notice::CannotHold` was shown in this episode.
    pub cannot_hold_shown: bool,
    /// Keys the user took back in this episode, on every plan (decision 26).
    #[expect(dead_code, reason = "p2-core-settings fills it")]
    pub released: BTreeSet<SettingKey>,
    /// An apply failed (`SafetyNet` or `Write`): requests only for the rest of the episode.
    pub requests_only: bool,
    /// When the lid closed while holding (run time), for the closed-lid report.
    #[expect(dead_code, reason = "p2-core-lid fills it")]
    pub lid_closed_since_mono: Option<u64>,
}

/// A countdown to hibernate (`policy::ending`).
#[derive(Clone, Copy, Debug)]
#[expect(dead_code, reason = "p2-core-lid fills it")]
pub(crate) struct Countdown {
    pub reason: CountdownReason,
}
