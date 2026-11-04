//! The core's state, private to the core: one struct owned by the loop thread (architecture section 3, "State is one
//! struct"). Every field the three core cards need is declared here, so a later card fills a field instead of
//! reshaping the struct; a field a later card fills names that card and carries an `expect(dead_code)` it removes.
//!
//! Owner: `p2-core-holds`, then `p2-core-settings`, then `p2-core-lid`.

use std::collections::{BTreeMap, BTreeSet};

use crate::command::{CountdownReason, RestoreWhy};
use crate::files::{Config, StateFile};
use crate::holds::{HoldFacts, HoldId, HoldKind};
use crate::policy::lid::SourceLock;
use crate::policy::requests::Requests;
use crate::policy::sessions::SessionMemory;
use crate::process::{Alive, FileTime, ProcessRef};
use crate::settings::{Guid, JournalEntry, PlanValues, SettingKey};
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
    /// The last plan read that worked, with the values restores put back since (`core::lid`, power safety review
    /// finding 5).
    pub plan: Option<PlanValues>,
    /// When the last `ReadPlan` went out (run time).
    pub plan_read_mono: Option<u64>,
    /// A copy of the journal from `Started`, `Done(Applied)` and `Done(Restored)`, less the keys `UserChanged` names.
    /// A failed outcome with an empty `journal` read nothing (the load failed), so the copy before it stays.
    pub journal: Vec<JournalEntry>,
    /// The plan of each `Apply` sent and not yet answered, oldest first: what it writes is not in the copy yet.
    pub applying: Vec<Guid>,
    /// A restore failed and is still wanted, or `Started` brought a journal from an earlier run (`Leftover`, track
    /// note 9): retried on each tick, with this reason, until a restore of every plan works.
    pub retry: Option<RestoreWhy>,
    /// `Notice::RestoreFailed` was shown in this failure streak.
    pub retry_warned: bool,
    /// `Notice::RequestsOnly` was shown since the last apply that worked.
    pub requests_only_warned: bool,
    /// The one restore for a journal the apply found corrupt went out; `JournalCorrupt` says the file is set aside.
    pub corrupt_restore_sent: bool,
    /// `Menu(Restore)` waits for its `Done(Restored)`, then `FinishRestoreRequest` (track note 5).
    pub restore_request: bool,
    /// The next `state.json` goes out as `FinishRestoreRequest`, at once.
    pub finish_restore_request: bool,
    /// Settings are blocked for this run (access denied, 5 or 1260, from a settings access check or write): requests
    /// only.
    pub requests_only_run: bool,
    /// `Session(Ending)` or `Ended` came and no `EndCancelled` yet: no `Apply`.
    pub session_ending: bool,
    /// `Menu(Quit)` went out: nothing more.
    pub quit: bool,
    /// `None` until the first `Lid` event.
    pub lid_closed: Option<bool>,
    /// `None` until the first `Power` event; the first one is no source change (`policy::lid::source_change`).
    pub ac: Option<bool>,
    pub battery_pct: Option<u8>,
    /// When the current power source began (run time); `None` before the first change.
    pub source_since_mono: Option<u64>,
    /// On battery under the floor: no requests, no apply, until AC.
    pub stepped_back: bool,
    /// The step back's notice and sound went out in this step back: once, at the first moment a hold counts in it.
    pub step_back_shown: bool,
    /// A lock waiting for a new power source to last (`policy::lid::settle`).
    pub source_lock: Option<SourceLock>,
    /// Work ended with the lid closed and the countdown started; kept while the lid stays closed and no hold starts.
    pub work_end: Option<WorkEnd>,
    /// A step back with the lid closed wants the countdown (`CountdownReason::Floor`) until AC, the lid opens or
    /// holding starts again.
    pub floor_countdown: bool,
    /// The countdown to hibernate that runs now: derived from `work_end` and `floor_countdown` in each settle pass.
    pub countdown: Option<Countdown>,
    /// A `Hibernate` went out and its `Done` has not come back: never a second one at the same time.
    pub hibernating: bool,
    /// The hibernate thread found the battery above the floor: no second `Floor` hibernate before a new battery
    /// reading, or each input reading would send one more while the two readings disagree.
    pub above_floor_skipped: bool,
    /// The last `InputIdle` reading.
    pub idle: Option<IdleReading>,
    /// For the idle clamp at resume (`policy::ending::clamped_idle_ms`).
    pub last_resume_mono: Option<u64>,
    /// `Session(Locked)` came and no `Unlocked` yet.
    pub session_locked: bool,
    /// When the Keep screen on idle lock last locked (`policy::lock::idle_lock`).
    pub idle_locked_at_mono: Option<u64>,
    /// When the idle lock is due if no input comes.
    pub idle_lock_check: Option<Deadline>,
    /// Notices raised while locked or off the console, sent at `Unlocked` or `ConsoleBack` (track note 18).
    pub held_notices: Vec<Notice>,
    /// The PC ran with the lid closed on our Do nothing while holding, not reported yet.
    pub closed_lid: Option<ClosedLid>,
    /// The lid side's own memory (`core::lid`).
    pub lock: LockWatch,
    /// `UpdateReady`'s version, until `ApplyUpdate` goes out for it.
    pub update_ready: Option<String>,
    /// `ApplyUpdate` went out and its `Done` has not come back.
    pub updating: bool,
    /// `Notice::Updated` went out in this run.
    pub updated_shown: bool,

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
            plan_read_mono: None,
            journal: Vec::new(),
            applying: Vec::new(),
            retry: None,
            retry_warned: false,
            requests_only_warned: false,
            corrupt_restore_sent: false,
            restore_request: false,
            finish_restore_request: false,
            requests_only_run: false,
            session_ending: false,
            quit: false,
            lid_closed: None,
            ac: None,
            battery_pct: None,
            source_since_mono: None,
            stepped_back: false,
            step_back_shown: false,
            source_lock: None,
            work_end: None,
            floor_countdown: false,
            countdown: None,
            hibernating: false,
            above_floor_skipped: false,
            idle: None,
            last_resume_mono: None,
            session_locked: false,
            idle_locked_at_mono: None,
            idle_lock_check: None,
            held_notices: Vec::new(),
            closed_lid: None,
            lock: LockWatch::default(),
            update_ready: None,
            updating: false,
            updated_shown: false,
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
    pub released: BTreeSet<SettingKey>,
    /// An apply failed (`SafetyNet` or `Write`): requests only for the rest of the episode.
    pub requests_only: bool,
    /// Where this episode's apply stands.
    pub apply: ApplyStep,
}

/// Where an episode's apply stands (architecture section 3, "Holding starts", step 2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum ApplyStep {
    /// Nothing to send: sent, or the core does not apply now.
    #[default]
    Idle,
    /// Send `Apply` at the end of this step, after the requests: a hold start, `SettingsOwner`, `ConsoleBack`,
    /// `EndCancelled` or a plan switch.
    Wanted,
    /// A fresh `Plan(Ok)` first: no plan read yet, or the last apply found another plan active.
    WaitPlan,
}

/// A countdown to hibernate (`policy::ending`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Countdown {
    pub reason: CountdownReason,
    /// When the next input reading decides, so the loop reads it on time; `None`: at the next reading, whenever it
    /// comes (a timer fired without one, or the last one was used).
    pub check: Option<Deadline>,
}

/// The user's own lid actions when work ended with the lid closed, read before the restore landed (the journal's
/// `original` while ours), so a later power source change finds the user's own action for the new source even while
/// the plan copy still shows what Wakeshot wrote.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WorkEnd {
    pub own_lid_ac: u32,
    pub own_lid_dc: u32,
    /// A power source change onto the user's own Do nothing stopped the countdown; it starts again only once a source
    /// whose own action is not Do nothing has lasted `policy::lid::SOURCE_SETTLE_MS`.
    pub paused: bool,
}

/// The lid side's memory (`core::lid`): a closed-lid lock until the session reports it, the step back when it never
/// does, the lock after a hibernate that did not happen, the notices kept for the user's return, the countdown rules
/// that wait for a plan read that tells the user's own lid action, and the journal copy the last settle pass saw.
#[derive(Debug, Default)]
pub(crate) struct LockWatch {
    /// A closed-lid `Lock` the session has not confirmed with `Session(Locked)` yet.
    pub pending: Option<PendingLock>,
    /// Both tries failed with the lid closed on our Do nothing: no hold holds until the session locks or the lid opens.
    pub stepped_back: bool,
    /// That step back's restore and countdown went out.
    pub step_back_sent: bool,
    /// A hibernate did not happen with the lid closed: from this run time on, lock again at an idle reading
    /// (`Core::relock`). Ends when the lid opens.
    pub relock: Option<u64>,
    /// Notices a closed laptop showed on a screen nobody may look at: they go out again with the closed-lid report.
    pub kept: Vec<Notice>,
    /// Work ended with the lid closed while the user's own lid action was not known (no plan read yet, or the journal
    /// unknown), on these power sources: it starts the countdown once a read shows their own action as not Do nothing
    /// (`core::lid::work_end_waited`).
    pub work_end_waits: Option<&'static [bool]>,
    /// A step back met the lid closed while the user's own battery lid action was not known (no plan read yet, or the
    /// journal unknown): it starts the `Floor` countdown once a read shows their own action as not Do nothing
    /// (`core::lid::floor_countdown_waited`).
    pub floor_waits: bool,
    /// The journal copy as the last settle pass ended, so the next one sees which entries a restore took away.
    pub journal_seen: Vec<JournalEntry>,
}

/// A closed-lid `Lock` waiting for `Session(Locked)`.
#[derive(Clone, Copy, Debug)]
pub(crate) struct PendingLock {
    /// This lock is the one retry.
    pub retry: bool,
    /// `LockFailed` went out for this lock or the one it retries.
    pub shown: bool,
    /// When a tick decides: `core::lid::LOCK_CHECK_MS` after the lock, or at once after it failed.
    pub check_mono: u64,
}

/// One input idle reading: `ms` at run time `at_mono`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct IdleReading {
    pub ms: u64,
    pub at_mono: u64,
}

/// How long the PC ran with the lid closed on our Do nothing while holding, and what held it (the closed-lid report).
/// It outlives the episode: after work ends with the lid closed the PC still runs until the hibernate.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ClosedLid {
    /// Run time of the stretches that ended (the lid opened).
    pub ran_ms: u64,
    /// When the stretch that runs now began (run time).
    pub since_mono: Option<u64>,
    pub held_by: BTreeSet<Held>,
}
