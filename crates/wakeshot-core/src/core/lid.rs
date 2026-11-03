//! The closed-lid and battery side of the episode: the lid-close lock and chime, the lock when a closed laptop changes
//! power source, the battery floor step back, the countdown to hibernate, the Keep screen on idle lock, the closed-lid
//! report, and the notices that wait while the user is away (architecture section 3, "Episode flow").
//!
//! Owner: `p2-core-lid`.
//!
//! The rules are `policy::lid`, `lock`, `floor` and `ending`; this side keeps their state and feeds them what is in
//! force now ([`Core::in_force`]), not the last plan read, which an apply leaves up to 10 s behind. The countdown is
//! derived in each settle pass from two wants, "work ended with the lid closed" and "a step back with the lid closed",
//! so every rule that starts, stops or restarts it is one line in one place.
//!
//! A closed-lid lock is watched until the session reports it (`State::lock`): one that fails, or that no
//! `Session(Locked)` follows within [`LOCK_CHECK_MS`], is sent once more, and when that one goes the same way the core
//! steps back as at the battery floor (section 8, "Lock fails"; power safety review finding 1). A hibernate that did
//! not happen leaves a closed laptop running: from then on it is locked, with the chime, whenever input has been idle
//! 5 minutes, until the lid opens (power safety verify minor 1).

use super::settings::{KEYS, TARGET};
use super::{Core, label, ms_ahead, nearest};
use crate::command::{Command, CountdownReason, Failure, HibernateEnd, HibernateSkip, RestoreWhy};
use crate::policy::ending::{self, Cancel, Check};
use crate::policy::floor;
use crate::policy::lid::{self, DO_NOTHING, SOURCE_SETTLE_MS, Settle};
use crate::policy::lock::{self, IdleFacts};
use crate::settings::{JournalEntry, PlanValues, SettingKey};
use crate::state::{ClosedLid, Countdown, IdleReading, PendingLock, WorkEnd};
use crate::time::{Deadline, Now};
use crate::view::{Cue, Notice};

/// Both power sources, for a rule asked before the first `Power` event: then it must hold for either.
const EITHER: [bool; 2] = [true, false];

/// How long the session may take to report `Locked` after a closed-lid `Lock`. `LockWorkStation` only starts the lock
/// (research, "`LockWorkStation`"); in the lid spike the session read locked 1 s after it (research, "Lock").
const LOCK_CHECK_MS: u64 = 5_000;

/// The lid action Sleep: the stand-in for a lid key no plan read has shown and no journal entry names, so never ours.
const SLEEP: u32 = 1;

/// How often a closed laptop that a hibernate did not stop is locked again while it stays idle and unlocked: the
/// countdown's own 5 minutes, so each lock comes with its chime at most that often.
const RELOCK_MS: u64 = ending::IDLE_MS;

impl Core {
    /// No hold holds now: the battery floor, or a closed laptop that could not be locked (`policy::requests`).
    pub(super) fn holds_back(&self) -> bool {
        self.state.stepped_back || self.state.lock.stepped_back
    }

    /// The power sources the PC may be on: the known one, or both before the first `Power` event.
    fn sources(&self) -> &'static [bool] {
        match self.state.ac {
            Some(true) => &[true],
            Some(false) => &[false],
            None => &EITHER,
        }
    }

    /// The plan and journal as the lid and lock rules must see them: what is in force now. The plan copy is the last
    /// read, and after an apply it still shows the user's values for up to 10 s, so `policy::lid::ours` would miss our
    /// Do nothing at a lid close in the first seconds of a hold. So each journal entry on the active plan sets its key
    /// to `written` (the IO thread's check drops an entry once a read shows the value changed), and an apply not
    /// answered yet on the active plan counts as written, since it may have landed. Both err toward a lock. `None`
    /// before the first plan read. The plan copy is taken as the last restores left it ([`Core::plan_copy`]).
    pub(super) fn in_force(&self) -> Option<(PlanValues, Vec<JournalEntry>)> {
        let mut plan = self.plan_copy()?;
        let mut journal = self.state.journal.clone();
        if self.state.applying.contains(&plan.active) {
            for key in KEYS {
                let live = value(&plan, key);
                let entered = journal
                    .iter()
                    .any(|e| e.plan == plan.active && e.key == key);
                // The apply skips a key already at its target with no entry: the user's own value.
                if live != TARGET && !entered {
                    journal.push(JournalEntry {
                        plan: plan.active,
                        key,
                        original: live,
                        written: TARGET,
                        restored: false,
                    });
                }
            }
        }
        let active = plan.active;
        for entry in journal.iter().filter(|e| e.plan == active) {
            set(&mut plan, entry.key, entry.written);
        }
        Some((plan, journal))
    }

    /// The plan copy with what restores put back since the last settle pass. After a restore the copy still shows
    /// what Wakeshot wrote until the next read, up to 30 s once nothing is owned, so a work end then would read the
    /// user's own lid action as Do nothing and start no countdown (power safety review finding 5). An entry that left
    /// the journal copy, on the copy's active plan, whose key the copy still shows at `written`, was restored (or the
    /// user changed it, and the read that shows it comes next): its `original` stands for the value in force. A key
    /// the copy shows at another value was read since, and the read wins.
    fn plan_copy(&self) -> Option<PlanValues> {
        let mut plan = self.state.plan?;
        let active = plan.active;
        let journal = &self.state.journal;
        let gone = self.state.lock.journal_seen.iter().filter(|seen| {
            seen.plan == active
                && !journal
                    .iter()
                    .any(|e| (e.plan, e.key) == (seen.plan, seen.key))
        });
        for entry in gone {
            if value(&plan, entry.key) == entry.written {
                set(&mut plan, entry.key, entry.original);
            }
        }
        Some(plan)
    }

    /// The end of each settle pass: the plan copy keeps what this pass's restores put back, until a read replaces it,
    /// and the next pass compares with this journal copy.
    fn keep_plan_copy(&mut self) {
        self.state.plan = self.plan_copy();
        self.state.lock.journal_seen.clone_from(&self.state.journal);
    }

    /// What the lock rules see: what is in force, or before the first plan read a guess from the journal `Started`
    /// carried (`p2-core-props` finding 2). A crashed tray's values may be in force then, and a lid close is an edge, so
    /// a lock missed then never comes. The guess takes each lid key a journal entry names, on whichever plan, as ours
    /// in force on one stand-in plan, and any other lid key as Sleep. Only the lid keys are guessed, so only the
    /// lid-close and source lock rules read it; like the in-force view it errs toward a lock.
    fn lock_view(&self) -> Option<(PlanValues, Vec<JournalEntry>)> {
        if self.state.plan.is_some() {
            return self.in_force();
        }
        let active = self.state.journal.first()?.plan;
        let mut plan = PlanValues {
            active,
            lid_ac: SLEEP,
            lid_dc: SLEEP,
            sleep_ac: 0,
            sleep_dc: 0,
            saver_threshold_dc: 0,
        };
        let mut journal = Vec::new();
        for key in [SettingKey::LidAc, SettingKey::LidDc] {
            if let Some(&entry) = self.state.journal.iter().find(|e| e.key == key) {
                set(&mut plan, key, entry.written);
                journal.push(JournalEntry {
                    plan: active,
                    ..entry
                });
            }
        }
        Some((plan, journal))
    }

    /// Our Do nothing keeps a closed laptop running on the current source (either one before the first `Power`).
    fn lid_ours(&self) -> bool {
        self.lock_view().is_some_and(|(plan, journal)| {
            self.sources()
                .iter()
                .any(|&ac| lid::ours(&plan, ac, &journal))
        })
    }

    /// A step back with the lid closed wants the `Floor` countdown only when the user's own battery lid action is not
    /// Do nothing, the same rule as at work end (Haik, 2026-11-03; power safety review finding 7): with their own Do
    /// nothing the closed laptop would run without Wakeshot too. No plan read: not known yet.
    fn floor_countdown_starts(&self) -> bool {
        self.state.lid_closed == Some(true)
            && self.in_force().is_some_and(|(plan, journal)| {
                ending::starts_at_work_end(true, lid::user_action(&plan, false, &journal))
            })
    }

    /// The `Floor` countdown at a step back with the lid closed. Before the first plan read the user's own battery lid
    /// action is not known, yet a crashed tray's values from the `Started` journal may keep the laptop running: then the
    /// read decides (power safety verify minor 3).
    fn floor_countdown_wanted(&mut self) {
        if self.floor_countdown_starts() {
            self.state.floor_countdown = true;
        } else if self.state.plan.is_none() && self.state.lid_closed == Some(true) && self.may_own()
        {
            self.state.lock.floor_waits = true;
        }
    }

    /// The battery floor from the last plan read (`policy::floor::floor_pct`).
    pub(super) fn floor_pct(&self) -> u8 {
        self.state.plan.map_or(floor::DEFAULT_PCT, |p| {
            floor::floor_pct(p.saver_threshold_dc)
        })
    }

    /// `Lid`. The first reading says how the lid is, not that it moved, so a tray that starts with the lid closed
    /// locks nothing.
    pub(super) fn take_lid(&mut self, closed: bool, now: Now, out: &mut Vec<Command>) {
        match (self.state.lid_closed.replace(closed), closed) {
            (Some(false), true) => {
                // A user whose own lid action is Do nothing gets neither (decision 18); with the user's own Sleep
                // Windows sleeps the laptop itself.
                if self.lid_ours() {
                    out.push(Command::Log(
                        "The lid closed while Wakeshot keeps the laptop running: lock".to_owned(),
                    ));
                    self.lock_and_chime(now, out);
                    // Nothing holds or it stepped back, yet our Do nothing (a restore that failed or has not landed)
                    // keeps the closed laptop running, with our battery sleep Never: so it must hibernate itself, as
                    // after work that ended with the lid closed.
                    if self.state.episode.is_none() {
                        self.work_ended();
                    } else if self.state.stepped_back {
                        self.floor_countdown_wanted();
                    }
                }
            }
            (Some(true), false) => self.lid_opens(now, out),
            _ => {}
        }
    }

    /// The laptop now runs with the lid closed on our Do nothing: `Lock` first, then the chime, side by side; the lock
    /// never waits for the sound (the chime was heard in a session already locked, research "Lid-close results").
    /// While holding, the closed-lid stretch begins for the report. A session the core knows as locked will not report
    /// `Locked` again, so only a lock into an unlocked one is watched.
    fn lock_and_chime(&mut self, now: Now, out: &mut Vec<Command>) {
        out.push(Command::Lock);
        out.push(Command::Play(Cue::LidClosedHolding));
        if !self.state.session_locked {
            self.state.lock.pending = Some(PendingLock {
                retry: false,
                shown: false,
                check_mono: now.mono_ms.saturating_add(LOCK_CHECK_MS),
            });
        }
        let state = &mut self.state;
        if let Some(episode) = state.episode.as_ref() {
            let run = state.closed_lid.get_or_insert_with(ClosedLid::default);
            run.since_mono.get_or_insert(now.mono_ms);
            run.held_by.extend(episode.holding.iter().cloned());
        }
    }

    fn lid_opens(&mut self, now: Now, out: &mut Vec<Command>) {
        self.cancel_countdowns(Cancel::LidOpened, "the lid opened", out);
        self.state.lock.pending = None;
        self.state.lock.relock = None;
        self.end_lock_step_back("the lid opened", out);
        if let Some(run) = self.state.closed_lid.as_mut()
            && let Some(since) = run.since_mono.take()
        {
            run.ran_ms = run.ran_ms.saturating_add(now.mono_ms.saturating_sub(since));
        }
        // Signed in already (an external screen): the user is back now.
        if !self.state.session_locked {
            self.back_at_the_laptop(now, out);
        }
    }

    /// `Power`. A source change with the lid closed may lock once the new source lasts (`policy::lid`); AC back ends
    /// a `Floor` countdown.
    pub(super) fn take_power(
        &mut self,
        ac: bool,
        battery_pct: Option<u8>,
        now: Now,
        out: &mut Vec<Command>,
    ) {
        let from = self.state.ac.replace(ac);
        self.state.battery_pct = battery_pct;
        self.state.above_floor_skipped = false;
        // The first reading has no source before it: no change.
        let Some(from) = from.filter(|&from| from != ac) else {
            return;
        };
        self.state.source_since_mono = Some(now.mono_ms);
        let closed = self.state.lid_closed == Some(true);
        // In place of any earlier one, so a change back inside the settle drops it.
        self.state.source_lock = self.lock_view().and_then(|(plan, journal)| {
            lid::source_change(&plan, &journal, closed, from, ac, now.mono_ms)
        });
        if self.state.source_lock.is_some() {
            out.push(Command::Log(format!(
                "The power source changed with the lid closed: lock once it has lasted {} s",
                SOURCE_SETTLE_MS / 1000
            )));
        }
        if ac {
            self.cancel_countdowns(Cancel::AcBack, "AC is back", out);
        }
    }

    /// `InputIdle`: the countdown, the Keep screen on idle lock and the lock after a hibernate that did not happen
    /// decide only on a reading.
    pub(super) fn take_input_idle(&mut self, ms: u64, now: Now, out: &mut Vec<Command>) {
        self.state.idle = Some(IdleReading {
            ms,
            at_mono: now.mono_ms,
        });
        self.check_countdown(now, out);
        self.idle_lock(ms, now, out);
        self.relock(ms, now, out);
    }

    /// `Resumed`, and a `Hibernate` that resumed: a resume counts as input for the idle clamp.
    pub(super) fn take_resumed(&mut self, now: Now) {
        self.state.last_resume_mono = Some(now.mono_ms);
    }

    /// `Locked`: a watched lock is confirmed, and a closed laptop that is locked after all may hold again.
    pub(super) fn take_locked(&mut self, out: &mut Vec<Command>) {
        self.state.session_locked = true;
        self.state.lock.pending = None;
        self.end_lock_step_back("the session is locked now", out);
    }

    /// `Unlocked`: the notices that waited, in order, then the closed-lid report once the lid is open. A sign-in on an
    /// external screen with the lid still closed gets no report yet: the stretch goes on.
    pub(super) fn take_unlocked(&mut self, now: Now, out: &mut Vec<Command>) {
        self.state.session_locked = false;
        self.send_held(out);
        if self.state.lid_closed != Some(true) {
            self.back_at_the_laptop(now, out);
        }
    }

    /// The user is at the open laptop and signed in: the notices a closed laptop showed to nobody, then the closed-lid
    /// report (power safety verify minor 2).
    fn back_at_the_laptop(&mut self, now: Now, out: &mut Vec<Command>) {
        for notice in std::mem::take(&mut self.state.lock.kept) {
            self.notify(notice, out);
        }
        self.report(now, out);
    }

    /// A notice raised with the lid closed and the session unlocked goes to a screen nobody may look at (the lid
    /// closed, no external screen): it goes out again when the user is back.
    fn keep_for_return(&mut self, notice: Notice) {
        if self.state.lid_closed == Some(true)
            && !self.state.session_locked
            && !self.state.lock.kept.contains(&notice)
        {
            self.state.lock.kept.push(notice);
        }
    }

    /// The notices that waited, in order, once the user is back: unlocked and on the PC's own screen (track note 18).
    pub(super) fn send_held(&mut self, out: &mut Vec<Command>) {
        if self.state.session_locked || !self.state.on_console {
            return;
        }
        out.extend(self.state.held_notices.drain(..).map(Command::Notify));
    }

    fn report(&mut self, now: Now, out: &mut Vec<Command>) {
        let Some(run) = self.state.closed_lid.take() else {
            return;
        };
        // Run time skips suspended time, so a hibernate in the stretch does not count (architecture section 3, "Clocks").
        let ran_ms = run.ran_ms.saturating_add(
            run.since_mono
                .map_or(0, |since| now.mono_ms.saturating_sub(since)),
        );
        let held_by = run.held_by.iter().map(|h| label(h).to_owned()).collect();
        self.notify(Notice::ClosedLidReport { ran_ms, held_by }, out);
    }

    /// The battery floor from the facts now, before the requests: stepped back takes none (`policy::requests`). AC
    /// back holds again as at a hold start.
    pub(super) fn settle_floor(&mut self, out: &mut Vec<Command>) {
        let was = self.state.stepped_back;
        let floor_pct = self.floor_pct();
        let stepped_back = self.state.ac.map_or(was, |ac| {
            floor::stepped_back(was, ac, self.state.battery_pct, floor_pct)
        });
        if stepped_back == was {
            return;
        }
        self.state.stepped_back = stepped_back;
        if stepped_back {
            let pct = self
                .state
                .battery_pct
                .map_or_else(|| "?".to_owned(), |p| p.to_string());
            out.push(Command::Log(format!(
                "On battery at {pct} %, under the floor of {floor_pct} %: step back, no hold keeps the PC awake until AC"
            )));
        } else {
            self.state.step_back_shown = false;
            out.push(Command::Log("AC is back: the step back ends".to_owned()));
            self.want_apply();
        }
    }

    /// The settle pass's part after the episode edges: the step back's actions, the countdown, the closed-lid stretch,
    /// the source lock. Gives the nearest deadline of these.
    pub(super) fn settle_lid(
        &mut self,
        now: Now,
        tick: bool,
        out: &mut Vec<Command>,
    ) -> Option<Deadline> {
        // The first plan read tells the user's own lid actions: the countdown rules that met the lid closed before it
        // decide now, before the cancels below.
        if self.state.plan.is_some() {
            if let Some(sources) = self.state.lock.work_end_waits.take() {
                self.work_ended_on(sources);
            }
            if std::mem::take(&mut self.state.lock.floor_waits) && self.state.stepped_back {
                self.floor_countdown_wanted();
            }
        }
        // Once per step back, at the first moment a hold counts in it: a step back with nothing held changes nothing.
        if self.state.stepped_back && self.state.episode.is_some() && !self.state.step_back_shown {
            self.state.step_back_shown = true;
            self.restore_all(RestoreWhy::Floor, out);
            let floor_pct = self.floor_pct();
            self.notify(Notice::StepBack { floor_pct }, out);
            out.push(Command::Play(Cue::BatteryFloor));
            self.floor_countdown_wanted();
        }
        // Our Do nothing kept the closed laptop running and it could not be locked: give the settings back and let it
        // hibernate as after work that ended, read before the restore lands. The requests went off in this pass.
        if self.state.lock.stepped_back && !self.state.lock.step_back_sent {
            self.state.lock.step_back_sent = true;
            self.restore_all(RestoreWhy::Floor, out);
            self.work_ended();
        }
        // A hold that appears while stepped back does not hold, so it ends no countdown (`Cancel::HoldingStarts`).
        if self.state.episode.is_some() && !self.holds_back() {
            self.cancel_countdowns(Cancel::HoldingStarts, "holding starts again", out);
        }
        let state = &mut self.state;
        if let Some(run) = state.closed_lid.as_mut()
            && run.since_mono.is_some()
            && let Some(episode) = state.episode.as_ref()
        {
            run.held_by.extend(episode.holding.iter().cloned());
        }

        let source_due = self.settle_source_lock(now, out);
        let countdown_due = self.settle_countdown(now, tick, out);
        // A timer that fired without a reading: the next reading decides, at the latest at the safety tick.
        if tick
            && self
                .state
                .idle_lock_check
                .is_some_and(|at| ms_ahead(at, now) <= 0)
        {
            self.state.idle_lock_check = None;
        }
        let idle_due = self.state.idle_lock_check;
        let lock_due = self
            .state
            .lock
            .pending
            .map(|p| Deadline::Mono(p.check_mono));
        self.keep_plan_copy();
        let due = nearest(nearest(source_due, countdown_due, now), idle_due, now);
        nearest(due, lock_due, now)
    }

    /// At a tick, before the settle pass: a watched lock that failed, or that the session did not confirm in time,
    /// gets the one retry; when the retry goes the same way, the step back, whose requests go off in this pass. Both
    /// only while the lid is closed on our Do nothing; else the lock no longer guards anything Wakeshot keeps running.
    pub(super) fn check_lock(&mut self, now: Now, out: &mut Vec<Command>) {
        let Some(pending) = self
            .state
            .lock
            .pending
            .filter(|p| now.mono_ms >= p.check_mono)
        else {
            return;
        };
        self.state.lock.pending = None;
        if self.state.lid_closed != Some(true) || !self.lid_ours() {
            out.push(Command::Log(
                "The lock is not confirmed, but Wakeshot no longer keeps a closed laptop running: nothing to do"
                    .to_owned(),
            ));
            return;
        }
        if !pending.shown {
            self.notify(Notice::LockFailed, out);
        }
        if pending.retry {
            out.push(Command::Log(
                "The lock failed twice with the lid closed: step back as at the battery floor until the session \
                 locks or the lid opens"
                    .to_owned(),
            ));
            self.state.lock.stepped_back = true;
            // The only notice that says why the hold stopped went to a closed laptop's screen.
            self.keep_for_return(Notice::LockFailed);
            return;
        }
        out.push(Command::Log(
            "The lock failed or the session did not report it locked: lock once more".to_owned(),
        ));
        out.push(Command::Lock);
        self.state.lock.pending = Some(PendingLock {
            retry: true,
            shown: true,
            check_mono: now.mono_ms.saturating_add(LOCK_CHECK_MS),
        });
    }

    /// A hibernate that did not happen (not allowed, failed, or the lid unknown) leaves the closed laptop running until
    /// the user's own sleep timeout, since putting the lid action back does not sleep a laptop whose lid is already
    /// closed; unlocked, if the lid-close lock failed or the user signed in on an external screen (power safety verify
    /// minor 1). So at an input reading 5 minutes idle (clamped at resume, as the countdown), `Lock` with the chime,
    /// then at most every [`RELOCK_MS`], while the lid stays closed, the session is unlocked and on the PC's screen,
    /// and the user's own lid action for the power source, once it lasted the settle, is not Do nothing: with their
    /// own Do nothing the laptop would run unlocked without Wakeshot too. A lock that works shows as `Session(Locked)`
    /// and the next one waits for an unlock. Not watched: a failure changes nothing, and the next one comes anyway.
    /// The chime goes with each, as every `Lock` comes with a sound or a notice (decision 0).
    fn relock(&mut self, reported_ms: u64, now: Now, out: &mut Vec<Command>) {
        if self.state.lock.relock.is_none_or(|at| now.mono_ms < at) {
            return;
        }
        let idle = ending::clamped_idle_ms(reported_ms, now.mono_ms, self.state.last_resume_mono);
        let state = &self.state;
        let settled = state
            .source_since_mono
            .is_none_or(|at| now.mono_ms >= at.saturating_add(SOURCE_SETTLE_MS));
        let wanted = ending::check(idle, now.mono_ms) == Check::Due
            && settled
            && state.lid_closed == Some(true)
            && !state.session_locked
            && state.on_console
            && self.state.lock.pending.is_none()
            && self.in_force().is_some_and(|(plan, journal)| {
                self.sources()
                    .iter()
                    .all(|&ac| lid::user_action(&plan, ac, &journal) != DO_NOTHING)
            });
        if !wanted {
            return;
        }
        out.push(Command::Log(
            "The closed laptop still runs unlocked after the hibernate did not happen, 5 minutes without input: lock"
                .to_owned(),
        ));
        out.push(Command::Lock);
        out.push(Command::Play(Cue::LidClosedHolding));
        self.state.lock.relock = Some(now.mono_ms.saturating_add(RELOCK_MS));
    }

    /// The closed laptop is locked or in front of the user again: holds hold as at a hold start.
    fn end_lock_step_back(&mut self, why: &str, out: &mut Vec<Command>) {
        if std::mem::take(&mut self.state.lock.stepped_back) {
            self.state.lock.step_back_sent = false;
            out.push(Command::Log(format!(
                "The step back after the failed lock ends: {why}"
            )));
            self.want_apply();
        }
    }

    fn settle_source_lock(&mut self, now: Now, out: &mut Vec<Command>) -> Option<Deadline> {
        let pending = self.state.source_lock?;
        let closed = self.state.lid_closed == Some(true);
        let answer = match (self.lock_view(), self.state.ac) {
            (Some((plan, journal)), Some(ac)) => {
                lid::settle(pending, &plan, &journal, closed, ac, now.mono_ms)
            }
            _ => Settle::Drop,
        };
        match answer {
            Settle::Wait(at) => Some(at),
            Settle::Lock => {
                self.state.source_lock = None;
                out.push(Command::Log(
                    "The new power source lasted with the lid closed, and Wakeshot keeps the laptop running: lock"
                        .to_owned(),
                ));
                self.lock_and_chime(now, out);
                None
            }
            Settle::Drop => {
                self.state.source_lock = None;
                None
            }
        }
    }

    /// Holding ended, before its restore lands: work ended with the lid closed wants the countdown when the user's own
    /// lid action is not Do nothing (`policy::ending::starts_at_work_end`). Their own actions for both sources are
    /// kept, read now while the journal's `original` still answers, for a later power source change.
    pub(super) fn work_ended(&mut self) {
        let sources = self.sources();
        self.work_ended_on(sources);
    }

    /// [`Core::work_ended`] on the power sources the PC was on when work ended. Before the first plan read the user's
    /// own lid actions are not known: with nothing owned, nothing was applied and Wakeshot changed nothing; else a
    /// crashed tray's values from the `Started` journal may keep the closed laptop running, and the read decides
    /// (power safety verify minor 3). A guess from the journal would take a key it does not name, often the user's own
    /// Do nothing, for Sleep.
    fn work_ended_on(&mut self, sources: &'static [bool]) {
        if self.state.lid_closed != Some(true) {
            return;
        }
        let Some((plan, journal)) = self.in_force() else {
            if self.may_own() {
                self.state.lock.work_end_waits = Some(sources);
            }
            return;
        };
        let work_end = WorkEnd {
            own_lid_ac: lid::user_action(&plan, true, &journal),
            own_lid_dc: lid::user_action(&plan, false, &journal),
            paused: false,
        };
        let starts = sources
            .iter()
            .any(|&ac| ending::starts_at_work_end(true, work_end.own_action(ac)));
        if starts {
            self.state.work_end = Some(work_end);
        }
    }

    /// Ends the wants that `cancel` ends (`policy::ending::cancels`), with a log line when a countdown ran.
    fn cancel_countdowns(&mut self, cancel: Cancel, why: &str, out: &mut Vec<Command>) {
        // A want still waiting for the first plan read ends the same way.
        if ending::cancels(CountdownReason::WorkEnded, cancel) {
            self.state.lock.work_end_waits = None;
        }
        if ending::cancels(CountdownReason::Floor, cancel) {
            self.state.lock.floor_waits = false;
        }
        let work =
            self.state.work_end.is_some() && ending::cancels(CountdownReason::WorkEnded, cancel);
        let floor = self.state.floor_countdown && ending::cancels(CountdownReason::Floor, cancel);
        if work {
            self.state.work_end = None;
        }
        if floor {
            self.state.floor_countdown = false;
        }
        if (work || floor) && self.state.countdown.is_some() {
            out.push(Command::Log(format!(
                "Countdown to hibernate cancelled: {why}"
            )));
        }
    }

    fn end_countdowns(&mut self) {
        self.state.work_end = None;
        self.state.floor_countdown = false;
        self.state.countdown = None;
    }

    /// The countdown from its two wants. Work that ended wins over the floor (`policy::ending::merged`): AC back must
    /// not end it. The user's own lid action for the current source decides whether the work-end want counts
    /// (orchestrator decision 2026-11-03 21:22, Haik's yes 21:23): their own Do nothing keeps a closed laptop running,
    /// so Wakeshot only releases; a source whose own action is not Do nothing starts it again once that source has
    /// lasted `SOURCE_SETTLE_MS`, so a dock's short AC drop never hibernates a docked laptop left idle. Never while
    /// another session has the PC's screen: input idle is this session's, so the other user's typing would not count.
    fn settle_countdown(
        &mut self,
        now: Now,
        tick: bool,
        out: &mut Vec<Command>,
    ) -> Option<Deadline> {
        let sources = self.sources();
        let source_since = self.state.source_since_mono;
        let on_console = self.state.on_console;
        let mut restart_due = None;
        let work = self.state.work_end.as_mut().is_some_and(|work_end| {
            let own_do_nothing = sources
                .iter()
                .all(|&ac| work_end.own_action(ac) == DO_NOTHING);
            if own_do_nothing {
                work_end.paused = true;
            } else if work_end.paused {
                let due =
                    source_since.map_or(now.mono_ms, |at| at.saturating_add(SOURCE_SETTLE_MS));
                if now.mono_ms >= due {
                    work_end.paused = false;
                } else {
                    restart_due = Some(Deadline::Mono(due));
                }
            }
            !work_end.paused
        });
        let floor = self.state.floor_countdown;
        let reason = match (on_console, work, floor) {
            (false, ..) | (true, false, false) => None,
            (true, true, _) => Some(CountdownReason::WorkEnded),
            (true, false, true) => Some(CountdownReason::Floor),
        };
        match (self.state.countdown, reason) {
            (None, Some(reason)) => {
                out.push(Command::Log(format!(
                    "Countdown to hibernate starts ({reason:?}): due after {} minutes without input",
                    ending::IDLE_MS / 60_000
                )));
                self.state.countdown = Some(Countdown {
                    reason,
                    check: Some(Deadline::Mono(now.mono_ms)),
                });
                self.check_countdown(now, out);
            }
            (Some(_), None) => {
                self.state.countdown = None;
                // Still wanted, so this is a pause; a cancel logged its own line.
                if self.state.work_end.is_some() || floor {
                    let why = if on_console {
                        "the user's own lid action for this power source is Do nothing"
                    } else {
                        "another session has the PC's screen"
                    };
                    out.push(Command::Log(format!("Countdown to hibernate stops: {why}")));
                }
            }
            (Some(running), Some(reason)) if running.reason != reason => {
                if let Some(countdown) = self.state.countdown.as_mut() {
                    countdown.reason = reason;
                }
            }
            _ => {}
        }
        let countdown = self.state.countdown.as_mut();
        let check = countdown.and_then(|countdown| {
            // A timer that fired without a reading: the next reading decides, at the latest at the safety tick.
            if tick && countdown.check.is_some_and(|at| ms_ahead(at, now) <= 0) {
                countdown.check = None;
            }
            countdown.check
        });
        nearest(check, restart_due, now)
    }

    /// The countdown at a reading taken now. A reading from before now cannot tell that no input came since, so only
    /// a fresh one may make it due.
    fn check_countdown(&mut self, now: Now, out: &mut Vec<Command>) {
        let Some(reading) = self.state.idle.filter(|r| r.at_mono == now.mono_ms) else {
            return;
        };
        let Some(countdown) = self.state.countdown else {
            return;
        };
        let idle = ending::clamped_idle_ms(reading.ms, now.mono_ms, self.state.last_resume_mono);
        let floor_pct = self.floor_pct();
        let check = match ending::check(idle, now.mono_ms) {
            Check::Wait(at) => Some(at),
            // One at a time; and the floor as the hibernate thread will check it, so a percent that reads at the
            // floor again sends none.
            Check::Due
                if self.state.hibernating
                    || (countdown.reason == CountdownReason::Floor
                        && (self.state.above_floor_skipped
                            || !floor::below(self.state.battery_pct, floor_pct))) =>
            {
                None
            }
            Check::Due => {
                self.state.hibernating = true;
                out.push(Command::Log(format!(
                    "Countdown to hibernate due ({:?}): {} s without input",
                    countdown.reason,
                    idle / 1000
                )));
                // The hibernate thread plays the going to sleep sound after its last check (track note 4).
                out.push(Command::Hibernate {
                    reason: countdown.reason,
                    floor_pct,
                    last_resume_mono_ms: self.state.last_resume_mono,
                });
                None
            }
        };
        if let Some(countdown) = self.state.countdown.as_mut() {
            countdown.check = check;
        }
    }

    /// The Keep screen on idle lock, once per idle period (`policy::lock`, track note 11). The timeout is the user's own
    /// for the current source; before the first `Power` event the shorter of the two, the safe side.
    fn idle_lock(&mut self, reported_ms: u64, now: Now, out: &mut Vec<Command>) {
        let timeout_ms = self.in_force().and_then(|(plan, journal)| {
            self.sources()
                .iter()
                .filter_map(|&ac| lock::sleep_timeout_ms(&plan, ac, &journal))
                .min()
        });
        let facts = IdleFacts {
            display_on: self.state.requests.display,
            session_locked: self.state.session_locked,
            idle_ms: ending::clamped_idle_ms(reported_ms, now.mono_ms, self.state.last_resume_mono),
            timeout_ms,
            now_mono_ms: now.mono_ms,
        };
        let answer = lock::idle_lock(self.state.idle_locked_at_mono, facts);
        self.state.idle_locked_at_mono = answer.locked_at_mono_ms;
        self.state.idle_lock_check = answer.next_check;
        if answer.lock {
            out.push(Command::Log(
                "Keep screen on: idle past the user's own sleep timeout: lock".to_owned(),
            ));
            out.push(Command::Lock);
            self.notify(Notice::IdleLocked, out);
        }
    }

    /// A failed lock gets a notice (section 8), one per watched lock and its retry; the next tick decides on a watched
    /// one ([`Core::check_lock`]). `Ok` only means the lock started, so `Session(Locked)` or the check decides. The core
    /// sends `Lock` also into a session already locked, since a lost `Unlocked` must never skip a lock; whether
    /// `LockWorkStation` fails then is not verified, so a failure while the session is locked, which changes nothing,
    /// is logged only.
    pub(super) fn done_locked(
        &mut self,
        result: Result<(), Failure>,
        now: Now,
        out: &mut Vec<Command>,
    ) {
        let Err(Failure { call, code }) = result else {
            return;
        };
        out.push(Command::Log(format!("Lock failed ({call}, code {code})")));
        let first = match self.state.lock.pending.as_mut() {
            Some(pending) => {
                pending.check_mono = now.mono_ms;
                !std::mem::replace(&mut pending.shown, true)
            }
            // A lock after a hibernate that did not happen: `HibernateFailed` said so, and each comes with its chime.
            None => !self.state.session_locked && self.state.lock.relock.is_none(),
        };
        if first {
            self.notify(Notice::LockFailed, out);
        }
    }

    /// How the hibernate thread ended. Not allowed, failed or an unknown lid: a notice, and the countdown ends; settings
    /// came back at the work end or the step back, so Windows' own timers most likely apply (section 8), and with the
    /// lid still closed the laptop is locked whenever it is left idle ([`Core::relock`]). A skip for the lid open ends
    /// it too; one for input goes on to the next reading, one for the floor to the next battery reading; one for AC
    /// ends only a `Floor` countdown (`policy::ending::cancels`). `Resumed` counts as a resume, and the countdown is
    /// done.
    pub(super) fn done_hibernate(&mut self, end: HibernateEnd, now: Now, out: &mut Vec<Command>) {
        self.state.hibernating = false;
        match end {
            HibernateEnd::Resumed => {
                self.take_resumed(now);
                out.push(Command::Log(
                    "Hibernated and resumed: the countdown is done".to_owned(),
                ));
                self.end_countdowns();
            }
            HibernateEnd::NotAllowed
            | HibernateEnd::Failed(_)
            | HibernateEnd::Skipped(HibernateSkip::LidUnknown) => {
                let what = match end {
                    HibernateEnd::Failed(Failure { call, code }) => {
                        format!("failed ({call}, code {code})")
                    }
                    HibernateEnd::NotAllowed => "is not allowed".to_owned(),
                    _ => "skipped: the lid state is unknown".to_owned(),
                };
                out.push(Command::Log(format!(
                    "Hibernate {what}: the countdown ends"
                )));
                self.end_countdowns();
                self.notify(Notice::HibernateFailed, out);
                self.keep_for_return(Notice::HibernateFailed);
                if self.state.lid_closed == Some(true) {
                    out.push(Command::Log(
                        "The closed laptop keeps running: lock it whenever it is left 5 minutes without input, until \
                         the lid opens"
                            .to_owned(),
                    ));
                    self.state.lock.relock = Some(now.mono_ms);
                }
            }
            HibernateEnd::Skipped(HibernateSkip::LidOpen) => {
                out.push(Command::Log(
                    "Hibernate skipped: the lid is open".to_owned(),
                ));
                self.end_countdowns();
            }
            HibernateEnd::Skipped(skip) => {
                match skip {
                    HibernateSkip::OnAc => {
                        self.cancel_countdowns(Cancel::AcBack, "the PC is on AC", out);
                    }
                    HibernateSkip::AboveFloor => self.state.above_floor_skipped = true,
                    _ => {}
                }
                out.push(Command::Log(format!(
                    "Hibernate skipped ({skip:?}): the countdown goes on"
                )));
                if let Some(countdown) = self.state.countdown.as_mut() {
                    countdown.check = None;
                }
            }
        }
    }
}

impl WorkEnd {
    /// The user's own lid action for a power source, as it was when work ended.
    fn own_action(&self, ac: bool) -> u32 {
        if ac { self.own_lid_ac } else { self.own_lid_dc }
    }
}

fn value(plan: &PlanValues, key: SettingKey) -> u32 {
    match key {
        SettingKey::LidAc => plan.lid_ac,
        SettingKey::LidDc => plan.lid_dc,
        SettingKey::SleepDc => plan.sleep_dc,
    }
}

fn set(plan: &mut PlanValues, key: SettingKey, value: u32) {
    match key {
        SettingKey::LidAc => plan.lid_ac = value,
        SettingKey::LidDc => plan.lid_dc = value,
        SettingKey::SleepDc => plan.sleep_dc = value,
    }
}
