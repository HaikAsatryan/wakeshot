//! What the core asks the outside to do, and how it went (architecture section 3). Each command's result comes back
//! as `Event::Done(Outcome)`, so the core picks every fallback.

use crate::files::{Config, HoldFile, StateFile};
use crate::holds::HoldId;
use crate::settings::{Guid, JournalEntry, SettingKey};
use crate::view::{Cue, Notice, TrayView};

/// A failed Windows call, as the core sees it: which call and its code, enough to log and to pick a fallback.
///
/// Owner: power track (Phase 3), from `wakeshot_windows::WinError`. The core never branches on `call`; it may branch
/// on `code` only where the design names one (access denied 5 and 1260, section 8).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Failure {
    pub call: &'static str,
    pub code: u32,
}

/// Why a restore runs, for the log and for the core to match the outcome to its reason.
///
/// Owner: core track (Phase 2). `Leftover`: a journal left at start by a dead tray; the IO side runs it only when the
/// journal's owner is Gone or is this tray, so Unknown never starts a restore (architecture section 5, "Liveness").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RestoreWhy {
    WorkEnded,
    UserAsked,
    PlanSwitch,
    Floor,
    Quit,
    SessionEnd,
    ConsoleLeft,
    Leftover,
}

/// Why a hibernate countdown runs, because the last check differs: `Floor` also needs the PC still on battery and
/// under the floor.
///
/// Owner: core track (Phase 2) starts the countdown; power track (Phase 3) runs the last check.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CountdownReason {
    WorkEnded,
    Floor,
}

/// Which file a write or delete was for, so the core can tell which outcome is which.
///
/// Owner: core track (Phase 2). `RestoreRequest` answers `FinishRestoreRequest`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum FileKind {
    State,
    Config,
    Hold(HoldId),
    RestoreRequest,
}

/// One thing for the outside to do. The loop runs the cheap ones itself and queues the rest on the IO thread
/// (architecture section 4, "Tray threads").
///
/// Owner: core track (Phase 2) issues them; power track (Phase 3) runs `Requests`, `Retake`, `ReadPlan`, `Apply`,
/// `Restore`, `Lock`, `Hibernate`; tray track (Phase 5) runs `Tray`, `Notify`, `Play`; ship track (Phase 6) runs
/// `ApplyUpdate`; the integration step wires the rest. Each runner may rely on getting commands in the order the core
/// returned them.
///
/// - `Requests`: `system` means SYSTEM plus EXECUTION; `display` only for Keep screen on.
/// - `Retake`: answers as `Outcome::Requests`.
/// - `Apply`: `writes` is the three keys less the ones the user released, never less a key already at its target;
///   the apply restores this plan's journal entries for other keys in the same run (architecture section 5, Apply).
/// - `Restore`: `plan: None` takes every entry, `Some` only that plan's.
/// - `Hibernate`: the hibernate thread checks again right before the call (section 5), so it gets what that check
///   needs: the reason, the battery floor, and the last resume (idle counts from it). The hibernate thread plays
///   `Cue::GoingToSleep` after its last check; the core never sends `Play(GoingToSleep)`.
/// - `WriteHold`: Keep awake from the menu; the hold file carries the tray's own pid and start.
/// - `FinishRestoreRequest`: after `Done(Restored { why: UserAsked })`, write this `state.json` at once, then delete
///   `restore-request.json`, so `wakeshot restore` never prints the state from before the restore. Answers as
///   `Written { file: FileKind::RestoreRequest }`.
/// - `Log`: a line about a decision only the core knows (hold start and end with labels, a release, a countdown, a
///   step back, a format problem at most once a minute per source); never prompts or file content. The runner writes
///   it at info level, area `core`, and answers nothing.
/// - `ApplyUpdate`: only when no hold counts, no hold file names a live process and no key is owned (decision 42).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    Requests {
        system: bool,
        display: bool,
    },
    Retake,
    ReadPlan,
    Apply {
        plan: Guid,
        writes: Vec<(SettingKey, u32)>,
    },
    Restore {
        plan: Option<Guid>,
        why: RestoreWhy,
    },
    Lock,
    Hibernate {
        reason: CountdownReason,
        floor_pct: u8,
        last_resume_mono_ms: Option<u64>,
    },
    Tray(TrayView),
    Notify(Notice),
    Play(Cue),
    WriteState(StateFile),
    WriteConfig(Config),
    WriteHold {
        id: HoldId,
        hold: HoldFile,
    },
    DeleteHold(HoldId),
    FinishRestoreRequest(StateFile),
    Log(String),
    OpenLog,
    ApplyUpdate,
    Quit,
}

/// Why an apply wrote nothing, or not all it meant to (architecture section 5, "Apply").
///
/// Owner: power track (Phase 3). The core may rely on: `PlanChanged` at step 1 means nothing written; at step 6 it
/// means written to a plan now inactive and journaled, left to the plan switch. `SafetyNet` means nothing written.
/// `Write` (a write, the read-back, the second plan read or `PowerSetActiveScheme` failed) means the restore routine
/// already ran on this plan's entries (Apply step 6); the entries it left are in the outcome's `journal`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApplyError {
    PlanChanged,
    SafetyNet(Failure),
    Write(Failure),
}

/// Why the hibernate thread did not hibernate after its last check.
///
/// Owner: power track (Phase 3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HibernateSkip {
    LidOpen,
    LidUnknown,
    InputSeen,
    OnAc,
    AboveFloor,
}

/// How a `Hibernate` command ended. `Resumed` comes after the PC woke from it.
///
/// Owner: power track (Phase 3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HibernateEnd {
    Skipped(HibernateSkip),
    NotAllowed,
    Failed(Failure),
    Resumed,
}

/// How a sound ended. `Skipped`: the user's sound scheme is "No sounds", so nothing played.
///
/// Owner: tray track (Phase 5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SoundEnd {
    Played,
    Skipped,
}

/// The result of one command, posted back as `Event::Done`.
///
/// Owner: whoever runs the command (see `Command`). In `Applied` and `Restored`, `journal` is every entry left after
/// the command, so the core's mirror is a copy of it, never its own guess. A failed `Restored` kept the journal; the
/// core retries on the tick while a restore is still wanted. `Update(Ok)`: Velopack's updater now waits (at most
/// 60 s) for this process to exit, so the core issues `Quit`; `Update(Err)`: nothing changed, the next `UpdateReady`
/// tries again.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    Requests(Result<(), Failure>),
    Applied {
        plan: Guid,
        written: Vec<SettingKey>,
        result: Result<(), ApplyError>,
        journal: Vec<JournalEntry>,
    },
    Restored {
        plan: Option<Guid>,
        why: RestoreWhy,
        result: Result<(), Failure>,
        journal: Vec<JournalEntry>,
    },
    Locked(Result<(), Failure>),
    Hibernate(HibernateEnd),
    Sound(Result<SoundEnd, Failure>),
    Notified(Result<(), Failure>),
    Written {
        file: FileKind,
        result: Result<(), Failure>,
    },
    Update(Result<(), Failure>),
}
