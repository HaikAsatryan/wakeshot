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

use super::settings::{KEYS, TARGET};
use super::{Core, label, ms_ahead, nearest};
use crate::command::{Command, CountdownReason, Failure, HibernateEnd, HibernateSkip, RestoreWhy};
use crate::policy::ending::{self, Cancel, Check};
use crate::policy::floor;
use crate::policy::lid::{self, DO_NOTHING, SOURCE_SETTLE_MS, Settle};
use crate::policy::lock::{self, IdleFacts};
use crate::settings::{JournalEntry, PlanValues, SettingKey};
use crate::state::{ClosedLid, Countdown, IdleReading, WorkEnd};
use crate::time::{Deadline, Now};
use crate::view::{Cue, Notice};

/// Both power sources, for a rule asked before the first `Power` event: then it must hold for either.
const EITHER: [bool; 2] = [true, false];

impl Core {
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
    /// before the first plan read: nothing was applied.
    pub(super) fn in_force(&self) -> Option<(PlanValues, Vec<JournalEntry>)> {
        let mut plan = self.state.plan?;
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

    /// Our Do nothing keeps a closed laptop running on the current source (either one before the first `Power`).
    fn lid_ours(&self) -> bool {
        self.in_force().is_some_and(|(plan, journal)| {
            self.sources()
                .iter()
                .any(|&ac| lid::ours(&plan, ac, &journal))
        })
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
                        self.state.floor_countdown = true;
                    }
                }
            }
            (Some(true), false) => self.lid_opens(now, out),
            _ => {}
        }
    }

    /// The laptop now runs with the lid closed on our Do nothing: `Lock` first, then the chime, side by side; the lock
    /// never waits for the sound (the chime was heard in a session already locked, research "Lid-close results").
    /// While holding, the closed-lid stretch begins for the report.
    fn lock_and_chime(&mut self, now: Now, out: &mut Vec<Command>) {
        out.push(Command::Lock);
        out.push(Command::Play(Cue::LidClosedHolding));
        let state = &mut self.state;
        if let Some(episode) = state.episode.as_ref() {
            let run = state.closed_lid.get_or_insert_with(ClosedLid::default);
            run.since_mono.get_or_insert(now.mono_ms);
            run.held_by.extend(episode.holding.iter().cloned());
        }
    }

    fn lid_opens(&mut self, now: Now, out: &mut Vec<Command>) {
        self.cancel_countdowns(Cancel::LidOpened, "the lid opened", out);
        if let Some(run) = self.state.closed_lid.as_mut()
            && let Some(since) = run.since_mono.take()
        {
            run.ran_ms = run.ran_ms.saturating_add(now.mono_ms.saturating_sub(since));
        }
        // Signed in already (an external screen): the user is back now.
        if !self.state.session_locked {
            self.report(now, out);
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
        self.state.source_lock = self.in_force().and_then(|(plan, journal)| {
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

    /// `InputIdle`: the countdown and the Keep screen on idle lock decide only on a reading.
    pub(super) fn take_input_idle(&mut self, ms: u64, now: Now, out: &mut Vec<Command>) {
        self.state.idle = Some(IdleReading {
            ms,
            at_mono: now.mono_ms,
        });
        self.check_countdown(now, out);
        self.idle_lock(ms, now, out);
    }

    /// `Resumed`, and a `Hibernate` that resumed: a resume counts as input for the idle clamp.
    pub(super) fn take_resumed(&mut self, now: Now) {
        self.state.last_resume_mono = Some(now.mono_ms);
    }

    pub(super) fn take_locked(&mut self) {
        self.state.session_locked = true;
    }

    /// `Unlocked`: the notices that waited, in order, then the closed-lid report once the lid is open. A sign-in on an
    /// external screen with the lid still closed gets no report yet: the stretch goes on.
    pub(super) fn take_unlocked(&mut self, now: Now, out: &mut Vec<Command>) {
        self.state.session_locked = false;
        self.send_held(out);
        if self.state.lid_closed != Some(true) {
            self.report(now, out);
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
        // Once per step back, at the first moment a hold counts in it: a step back with nothing held changes nothing.
        if self.state.stepped_back && self.state.episode.is_some() && !self.state.step_back_shown {
            self.state.step_back_shown = true;
            self.restore_all(RestoreWhy::Floor, out);
            let floor_pct = self.floor_pct();
            self.notify(Notice::StepBack { floor_pct }, out);
            out.push(Command::Play(Cue::BatteryFloor));
            if self.state.lid_closed == Some(true) {
                self.state.floor_countdown = true;
            }
        }
        // A hold that appears while stepped back does not hold, so it ends no countdown (`Cancel::HoldingStarts`).
        if self.state.episode.is_some() && !self.state.stepped_back {
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
        nearest(nearest(source_due, countdown_due, now), idle_due, now)
    }

    fn settle_source_lock(&mut self, now: Now, out: &mut Vec<Command>) -> Option<Deadline> {
        let pending = self.state.source_lock?;
        let closed = self.state.lid_closed == Some(true);
        let answer = match (self.in_force(), self.state.ac) {
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
        if self.state.lid_closed != Some(true) {
            return;
        }
        // No plan read: nothing was applied, so Wakeshot changed nothing that keeps the laptop running.
        let Some((plan, journal)) = self.in_force() else {
            return;
        };
        let work_end = WorkEnd {
            own_lid_ac: lid::user_action(&plan, true, &journal),
            own_lid_dc: lid::user_action(&plan, false, &journal),
            paused: false,
        };
        let starts = self
            .sources()
            .iter()
            .any(|&ac| ending::starts_at_work_end(true, work_end.own_action(ac)));
        if starts {
            self.state.work_end = Some(work_end);
        }
    }

    /// Ends the wants that `cancel` ends (`policy::ending::cancels`), with a log line when a countdown ran.
    fn cancel_countdowns(&mut self, cancel: Cancel, why: &str, out: &mut Vec<Command>) {
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

    /// A failed lock gets a notice (section 8). The core sends `Lock` also into a session already locked, since a lost
    /// `Unlocked` must never skip a lock; whether `LockWorkStation` fails then is not verified, so a failure while the
    /// session is locked, which changes nothing, is logged only.
    pub(super) fn done_locked(&mut self, result: Result<(), Failure>, out: &mut Vec<Command>) {
        if let Err(Failure { call, code }) = result {
            out.push(Command::Log(format!("Lock failed ({call}, code {code})")));
            if !self.state.session_locked {
                self.notify(Notice::LockFailed, out);
            }
        }
    }

    /// How the hibernate thread ended. Not allowed, failed or an unknown lid: a notice, and the countdown ends; settings
    /// came back at the work end or the step back, so Windows' own timers most likely apply (section 8). A skip for
    /// the lid open ends it too; one for input goes on to the next reading, one for the floor to the next battery
    /// reading; one for AC ends only a `Floor` countdown (`policy::ending::cancels`). `Resumed` counts as a resume,
    /// and the countdown is done.
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
