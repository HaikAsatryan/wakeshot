//! The closed-lid and battery side of the episode: the lid-close lock and chime, the lock when a closed laptop changes
//! power source, the battery floor step back, the countdown to sleep, the Keep screen on idle lock, the closed-lid
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
//! steps back as at the battery floor (section 8, "Lock fails"; power safety review finding 1).
//!
//! The sleep route (`p2-6-sleep-route`, Haik 2026-11-04: "just make it sleep"). On Modern Standby no app call starts
//! a sleep, so when the countdown is due the hibernate thread runs the last check and the going to sleep sound
//! (`SleepCheck`), then an `Apply` sets the sleep and screen timeouts to 1 minute where the user's own is longer or
//! Never, and Windows' own idle sleep takes the laptop (`p3-idle-sleep-check`). They are Wakeshot's journal entries
//! (`State::short`) and go back at an unlock, input, the lid open, a hold start, the countdown's end and every other
//! restore point; not at a wake alone, so a laptop that wakes with the lid closed sleeps again. If it has not slept
//! [`ending::FALLBACK_MS`] after the write (a sleep is `Suspending`, then `Resumed` or the clock gap at the next
//! tick), `Hibernate` as before. A hibernate that did not happen leaves the short timeouts, and from then on the closed
//! laptop is locked, with the chime, whenever input has been idle 5 minutes, until the lid opens (power safety verify
//! minor 1).

use super::settings::{KEYS, TARGET};
use super::{Core, label, ms_ahead, nearest};
use crate::command::{
    ApplyError, Command, CountdownReason, Failure, HibernateEnd, HibernateSkip, RestoreWhy,
};
use crate::policy::ending::{self, Cancel, Check};
use crate::policy::floor;
use crate::policy::lid::{self, DO_NOTHING, SOURCE_SETTLE_MS, Settle};
use crate::policy::lock::{self, IdleFacts};
use crate::settings::{Guid, JournalEntry, PlanValues, SHORT_TIMEOUT_S, SettingKey};
use crate::state::{Applying, ClosedLid, Countdown, IdleReading, PendingLock, Short, WorkEnd};
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
        // Only a hold's apply writes the lid keys; the sleep route's touches no value a lid or lock rule reads.
        let holding_apply = |a: &Applying| a.plan == plan.active && !a.short;
        if self.state.applying.iter().any(holding_apply) {
            for key in KEYS {
                let Some(live) = value(&plan, key) else {
                    continue;
                };
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
            if value(&plan, entry.key) == Some(entry.written) {
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
    ///
    /// A journal the copy does not know ([`Core::journal_unknown`]) gets no guess: during its window Wakeshot does not
    /// lock at a lid close (product "User settings safety" 7, Haik 2026-11-04), and the window ends in the reset or a
    /// read, after which the countdown hook ([`Core::unknown_journal_ended`]) decides.
    fn lock_view(&self) -> Option<(PlanValues, Vec<JournalEntry>)> {
        if let Some(view) = self.in_force() {
            return Some(view);
        }
        let active = self.state.journal.first()?.plan;
        let mut plan = stand_in(active, SLEEP);
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

    /// The countdown hook at the end of an unknown journal's window, known or lost (`p2-6-record-lost` deliverable 4;
    /// `p2-6-sleep-route` builds on it). With the lid closed and nothing holding, a closed laptop may run on a value of
    /// Wakeshot's that the reset or a restore just took back, and putting the lid action back does not sleep a laptop
    /// whose lid is already closed. So the work-end want waits for the next plan read, which tells the user's own lid
    /// action (`work_end_waited`): the copy still shows the values from before the reset or restore. A step back with a
    /// hold that holds nothing waits the same way for the `Floor` countdown. The lock a lid close in the window did not
    /// get is not this hook's: it waits on its own until a hold holds ([`Core::settle_window_lock`]).
    pub(super) fn unknown_journal_ended(&mut self) {
        if self.state.lid_closed != Some(true) {
            return;
        }
        if self.state.episode.is_none() {
            if self.state.work_end.is_none() {
                let sources = self.sources();
                self.state.lock.work_end_waits.get_or_insert(sources);
            }
        } else if self.state.stepped_back && !self.state.floor_countdown {
            self.state.lock.floor_waits = true;
        }
    }

    /// The lock a lid close in the unknown window did not get (`p2-6-sleep-route` deliverable 8). Wakeshot is the
    /// reason the closed laptop runs only while a hold holds, or while its own Do nothing is in force (the lid-close
    /// rule), so the lock waits, while the lid stays closed, for the journal to be known and one of these: at the
    /// window's end, or later when a hold starts or a step back ends (`p2-6-sleep-route-fixes`, review major 1). Then
    /// it locks, with the chime, once a plan read shows the user's own lid action as not Do nothing, on a power source
    /// that has lasted the 30 s settle, so a dock's short AC drop does not lock a docked user (review minor 7). The
    /// copy may still show the values from before the reset or restore, so a Do nothing waits for the next read, as
    /// the countdown wants do; with the user's own Do nothing the laptop would run without Wakeshot too, and gets no
    /// lock (decision 18). A lock out already is watched first ([`Core::check_lock`]). Gives when the settle ends.
    fn settle_window_lock(&mut self, now: Now, out: &mut Vec<Command>) -> Option<Deadline> {
        let state = &self.state;
        // After a lock and its retry failed, the step back is the answer until the session locks or the lid opens. A
        // session locked already needs no lock, and a `Lock` into one is not watched, so it would be sent again at
        // every pass.
        let waits = state.lock.window_close
            && state.lid_closed == Some(true)
            && !state.session_locked
            && state.lock.pending.is_none()
            && !state.lock.stepped_back
            && !self.journal_unknown()
            && ((state.episode.is_some() && !self.holds_back()) || self.lid_ours());
        let own_sleeps = waits
            && self.in_force().is_some_and(|(plan, journal)| {
                self.sources()
                    .iter()
                    .any(|&ac| lid::user_action(&plan, ac, &journal) != DO_NOTHING)
            });
        if !own_sleeps {
            return None;
        }
        let settled = state
            .source_since_mono
            .map(|at| at.saturating_add(SOURCE_SETTLE_MS))
            .filter(|&at| now.mono_ms < at);
        if let Some(at) = settled {
            return Some(Deadline::Mono(at));
        }
        out.push(Command::Log(
            "Wakeshot keeps the closed laptop running since a lid close in the unknown journal window, which it could \
             not lock: lock"
                .to_owned(),
        ));
        // One lock for the one closed laptop: the source lock would only send a second.
        self.state.source_lock = None;
        self.lock_and_chime(now, out);
        None
    }

    /// The reset of a lost journal failed (`p2-6-sleep-route-fixes`, review minor 5). The journal now counts as known
    /// and empty, so a Do nothing the reset could not put back, most likely Wakeshot's, reads as the user's own, and no
    /// rule would ever lock or sleep a laptop whose lid closed in the window. So it is locked once, with the chime, as
    /// at a lid close. Whether that Do nothing is the user's own cannot be told now, so this errs toward the lock.
    pub(super) fn lost_reset_failed(&mut self, now: Now, out: &mut Vec<Command>) {
        let lock = &self.state.lock;
        if !lock.window_close || lock.pending.is_some() || self.state.lid_closed != Some(true) {
            return;
        }
        out.push(Command::Log(
            "The lost journal's reset failed with the lid closed since the window: lock".to_owned(),
        ));
        self.state.source_lock = None;
        self.lock_and_chime(now, out);
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

    /// The `Floor` countdown at a step back with the lid closed; gives whether it starts. It waits as a work end does
    /// ([`Core::work_ended_on`]): before the first plan read the user's own battery lid action is not known, yet a
    /// crashed tray's values from the `Started` journal may keep the laptop running (power safety verify minor 3); and
    /// while the journal is unknown ([`Core::journal_unknown`]) a Do nothing in force may be a crashed tray's.
    fn floor_countdown_wanted(&mut self) -> bool {
        let starts = self.floor_countdown_starts();
        if starts {
            self.state.floor_countdown = true;
        } else if self.state.lid_closed == Some(true)
            && self.may_own()
            && (self.state.plan.is_none() || self.journal_unknown())
        {
            self.state.lock.floor_waits = true;
        }
        starts
    }

    /// A `Floor` want that waited, at each settle pass once a plan read worked. For the reasons at work end
    /// ([`Core::work_end_waited`]) a Do nothing ends nothing: the want stays until a read shows the user's own battery
    /// lid action as not Do nothing, or AC or the lid opening ends it ([`Core::cancel_countdowns`]).
    fn floor_countdown_waited(&mut self) {
        if !self.floor_countdown_wanted() {
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
                // No lock now (`lock_view`): once the window ended, a hold that holds gets it. A session already
                // locked needs none.
                if self.journal_unknown() && !self.state.session_locked {
                    self.state.lock.window_close = true;
                }
                // A user whose own lid action is Do nothing gets neither (decision 18); with the user's own Sleep
                // Windows sleeps the laptop itself.
                if self.lid_ours() {
                    out.push(Command::Log(
                        "The lid closed while Wakeshot keeps the laptop running: lock".to_owned(),
                    ));
                    self.lock_and_chime(now, out);
                    // Nothing holds or it stepped back, yet our Do nothing (a restore that failed or has not landed)
                    // keeps the closed laptop running, with our battery sleep Never: so it must sleep as after work
                    // that ended with the lid closed.
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
        self.end_short("the lid opened", out);
        self.cancel_countdowns(Cancel::LidOpened, "the lid opened", out);
        self.state.lock.pending = None;
        self.state.lock.relock = None;
        self.state.lock.window_close = false;
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

    /// `InputIdle`: the countdown, the fallback hibernate, the Keep screen on idle lock and the lock after a hibernate
    /// that did not happen decide only on a reading. A reading under the one before means input came: input idle only
    /// grows while nobody types, through a sleep too, since it counts suspended time (research, "Clock"), and readings
    /// come at least every 30 s while the PC runs, far less than the 5 minutes of idle the short timeouts start from.
    /// So someone at an external screen gets their own timeouts back, and the countdown waits 5 minutes again. A read
    /// that failed gives 0, so it counts as input too: the safe side for the user at the screen.
    pub(super) fn take_input_idle(&mut self, ms: u64, now: Now, out: &mut Vec<Command>) {
        let before = self.state.idle.replace(IdleReading {
            ms,
            at_mono: now.mono_ms,
        });
        if before.is_some_and(|b| ms < b.ms) {
            self.end_short("input came", out);
        }
        self.check_countdown(now, out);
        self.idle_lock(ms, now, out);
        self.relock(ms, now, out);
    }

    /// `Resumed`, and a `Hibernate` that resumed: a resume counts as input for the idle clamp. With the short timeouts
    /// in force the laptop slept and woke with the lid closed, and must sleep again (product, work end): they stay,
    /// and the fallback counts again from now, as the hibernate thread's idle check counts from the resume. The resume
    /// tells the sleep, so the tick's clock gap check is not needed for it ([`Core::check_slept`]).
    pub(super) fn take_resumed(&mut self, now: Now, out: &mut Vec<Command>) {
        self.state.last_resume_mono = Some(now.mono_ms);
        if let Some(short) = self.state.short.as_mut() {
            short.suspended_at = None;
            out.push(Command::Log(
                "Resumed with the 1-minute timeouts in force and the lid closed: they stay, so the PC sleeps again"
                    .to_owned(),
            ));
        }
        self.rearm_fallback(now);
    }

    /// `Suspending`: only the short timeouts' fallback cares, and the next tick tells whether it was a sleep.
    pub(super) fn take_suspending(&mut self, now: Now) {
        if let Some(short) = self.state.short.as_mut() {
            short.suspended_at = Some(clock_gap(now));
        }
    }

    /// At a tick: a `Suspending` with the short timeouts in force and no `Resumed` since (its post lost) was a sleep
    /// when the wall clock moved past run time by [`ending::SLEPT_GAP_MS`] since, as run time stops during a sleep
    /// (research, "Clock"). Then the laptop woke with the lid still closed: the fallback counts again from now. A
    /// suspend that did not happen leaves the fallback as it was (`p3-idle-sleep-check`).
    pub(super) fn check_slept(&mut self, now: Now, out: &mut Vec<Command>) {
        let Some(at) = self
            .state
            .short
            .as_mut()
            .and_then(|short| short.suspended_at.take())
        else {
            return;
        };
        let gap_ms = clock_gap(now).saturating_sub(at);
        if gap_ms >= ending::SLEPT_GAP_MS {
            out.push(Command::Log(format!(
                "The PC slept for {} s with the 1-minute timeouts and woke with the lid closed: they stay, so it \
                 sleeps again",
                gap_ms / 1000
            )));
            self.rearm_fallback(now);
        } else {
            out.push(Command::Log(format!(
                "Suspending came, but the clocks show no sleep ({gap_ms} ms): the fallback stays as it was"
            )));
        }
    }

    /// The fallback hibernate [`ending::FALLBACK_MS`] from now, unless no hibernate is left in this countdown.
    fn rearm_fallback(&mut self, now: Now) {
        let Some(short) = self
            .state
            .short
            .as_mut()
            .filter(|short| short.fallback_mono.is_some())
        else {
            return;
        };
        let at = now.mono_ms.saturating_add(ending::FALLBACK_MS);
        short.fallback_mono = Some(at);
        if let Some(countdown) = self.state.countdown.as_mut() {
            countdown.check = Some(Deadline::Mono(at));
        }
    }

    /// The short timeouts go back, behind their own `Apply` when it is still out: the user is there, a hold starts, or
    /// the countdown stopped. Restore takes every entry, and nothing else is Wakeshot's while they are in force: the
    /// countdown runs only when no hold holds.
    fn end_short(&mut self, why: &str, out: &mut Vec<Command>) {
        if self.state.short.take().is_none() {
            return;
        }
        out.push(Command::Log(format!(
            "The 1-minute sleep and screen timeouts go back: {why}"
        )));
        self.restore_all(RestoreWhy::ShortTimeouts, out);
    }

    /// The short timeouts can be written: this tray owns the settings and its session has the screen, no policy blocked
    /// settings in this run, the session is not ending, a plan read worked, and some short key is not the user's.
    pub(super) fn can_write_short(&self) -> bool {
        let state = &self.state;
        state.owner
            && state.on_console
            && !state.requests_only_run
            && !state.session_ending
            && state.plan.is_some()
            && SettingKey::SHORT
                .iter()
                .any(|&k| !self.short_key_released(k))
    }

    /// The user changed this key in this countdown, or in the episode a battery floor step back keeps open: the work
    /// has not ended, so their change wins until it does (product "User settings safety" 3).
    fn short_key_released(&self, key: SettingKey) -> bool {
        self.state.short_released.contains(&key)
            || self
                .state
                .episode
                .as_ref()
                .is_some_and(|e| e.released.contains(&key))
    }

    /// The `Apply` of the short timeouts to `plan`: each short key the user did not take back, at 1 minute. The apply
    /// keeps a value already at or under it and not Never (`ownership::apply::meets`), the user's own.
    pub(super) fn send_short(&mut self, plan: Guid, out: &mut Vec<Command>) {
        let writes: Vec<(SettingKey, u32)> = SettingKey::SHORT
            .into_iter()
            .filter(|&k| !self.short_key_released(k))
            .map(|k| (k, SHORT_TIMEOUT_S))
            .collect();
        out.push(Command::Log(format!(
            "Sleep and screen off after {SHORT_TIMEOUT_S} s on plan {plan}, AC and battery, where the user's own is \
             longer or Never: Windows sleeps the closed laptop once nothing holds it"
        )));
        out.push(Command::Apply { plan, writes });
        self.state.applying.push(Applying { plan, short: true });
    }

    /// A short key the user just took back may still be written or held ([`Core::take_back`]): the short timeouts
    /// again without it, which restores the active plan's entry for it (`Command::Apply` doc), as a hold's apply does.
    /// With no short key left to write, or no write possible, a restore takes them back.
    pub(super) fn short_again(&mut self, out: &mut Vec<Command>) {
        let plan = self.state.plan.map(|p| p.active);
        match plan.filter(|_| self.can_write_short()) {
            Some(plan) => self.send_short(plan, out),
            None => self.restore_all(RestoreWhy::ShortTimeouts, out),
        }
    }

    /// A restore of every plan (a retry, or the take back of a released hold key) or a lost journal's reset put the
    /// short timeouts back while the countdown still wants them, or Windows' idle sleep never comes and only the
    /// fallback hibernates (`p2-6-reset-gaps`): they go out again, less the keys the user took back. A short apply
    /// still out was sent after that run, since answers come in order, so it writes them already.
    pub(super) fn rewrite_short(&mut self, out: &mut Vec<Command>) {
        let out_already = self.state.applying.iter().any(|a| a.short);
        if self.state.short.is_none() || out_already || !self.can_write_short() {
            return;
        }
        if let Some(plan) = self.state.plan.map(|p| p.active) {
            out.push(Command::Log(
                "The 1-minute timeouts went back with a restore or a reset while the countdown still wants them: \
                 again"
                    .to_owned(),
            ));
            self.send_short(plan, out);
        }
    }

    /// The sleep check passed: the short timeouts, with the fallback 5 minutes on. One that cannot be written (the
    /// check took seconds, and the facts may have changed) leaves the fallback due at once.
    fn write_short(&mut self, now: Now, out: &mut Vec<Command>) {
        let can_write = self.can_write_short();
        let Some(countdown) = self.state.countdown.as_mut() else {
            out.push(Command::Log(
                "The sleep check passed, but the countdown ended meanwhile: nothing to write"
                    .to_owned(),
            ));
            return;
        };
        let delay = if can_write { ending::FALLBACK_MS } else { 0 };
        let fallback = (!countdown.no_hibernate).then(|| now.mono_ms.saturating_add(delay));
        countdown.check = fallback.map(Deadline::Mono);
        self.state.short = Some(Short {
            fallback_mono: fallback,
            suspended_at: None,
        });
        match self.state.plan.map(|p| p.active).filter(|_| can_write) {
            Some(plan) => self.send_short(plan, out),
            None => out.push(Command::Log(
                "The 1-minute timeouts cannot be written now: the fallback is due at once"
                    .to_owned(),
            )),
        }
    }

    /// `Applied` for the short timeouts. A failed one wrote nothing that stays (the apply's own restore ran), so
    /// Windows' idle sleep will not come soon: the fallback is due at once. A plan that changed meanwhile: a read
    /// tells, and the plan switch writes the new plan ([`Core::take_plan`]).
    pub(super) fn done_short_applied(
        &mut self,
        plan: Guid,
        result: Result<(), ApplyError>,
        now: Now,
        out: &mut Vec<Command>,
    ) {
        let failure = match result {
            Ok(()) => return,
            Err(ApplyError::PlanChanged) => {
                self.read_plan(now, out);
                return;
            }
            Err(ApplyError::SafetyNet(failure) | ApplyError::Write(failure)) => failure,
        };
        let Failure { call, code } = failure;
        out.push(Command::Log(format!(
            "The 1-minute timeouts were not written on plan {plan} ({call}, code {code}): the fallback is due at once"
        )));
        if let Some(short) = self.state.short.as_mut()
            && let Some(at) = short.fallback_mono.as_mut()
        {
            *at = now.mono_ms;
            if let Some(countdown) = self.state.countdown.as_mut() {
                countdown.check = Some(Deadline::Mono(now.mono_ms));
            }
        }
    }

    /// `Locked`: a watched lock is confirmed, a lock a lid close in the unknown window waited for is not needed any
    /// more, and a closed laptop that is locked after all may hold again.
    pub(super) fn take_locked(&mut self, out: &mut Vec<Command>) {
        self.state.session_locked = true;
        self.state.lock.pending = None;
        self.state.lock.window_close = false;
        self.end_lock_step_back("the session is locked now", out);
    }

    /// `Unlocked`: the notices that waited, in order, then the closed-lid report once the lid is open. A sign-in on an
    /// external screen with the lid still closed gets no report yet: the stretch goes on.
    pub(super) fn take_unlocked(&mut self, now: Now, out: &mut Vec<Command>) {
        self.state.session_locked = false;
        self.state.last_unlock_mono = Some(now.mono_ms);
        self.end_short("the session is unlocked, so the user is there", out);
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
    pub(super) fn keep_for_return(&mut self, notice: Notice) {
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
        // A plan read tells the user's own lid actions: the countdown wants that wait for one decide at each pass once
        // one worked, before the cancels below.
        let mut window_due = None;
        if self.state.plan.is_some() {
            if let Some(sources) = self.state.lock.work_end_waits.take() {
                self.work_end_waited(sources);
            }
            if std::mem::take(&mut self.state.lock.floor_waits) && self.state.stepped_back {
                self.floor_countdown_waited();
            }
            window_due = self.settle_window_lock(now, out);
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
        // sleep as after work that ended, read before the restore lands. The requests went off in this pass.
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
        nearest(nearest(due, lock_due, now), window_due, now)
    }

    /// At a tick, before the settle pass: a watched lock that failed, or that the session did not confirm in time,
    /// gets the one retry; when the retry goes the same way, the step back, whose requests go off in this pass. Both
    /// only while the lid is closed on our Do nothing, or a hold holds through a lid close in the unknown window, which
    /// may run on requests only (review minor 7); else the lock no longer guards anything Wakeshot keeps running.
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
        let window_hold =
            self.state.lock.window_close && self.state.episode.is_some() && !self.holds_back();
        if self.state.lid_closed != Some(true) || !(self.lid_ours() || window_hold) {
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

    /// [`Core::work_ended`] on the power sources the PC was on when work ended; gives whether it wants the countdown.
    /// Before the first plan read the user's own lid actions are not known: with nothing owned, nothing was applied and
    /// Wakeshot changed nothing; else a crashed tray's values from the `Started` journal may keep the closed laptop
    /// running, and the read decides (power safety verify minor 3). A guess from the journal would take a key it does
    /// not name, often the user's own Do nothing, for Sleep. While the journal is unknown ([`Core::journal_unknown`]) a
    /// Do nothing in force may be a crashed tray's, so it tells nothing either: the want waits
    /// ([`Core::work_end_waited`]).
    fn work_ended_on(&mut self, sources: &'static [bool]) -> bool {
        if self.state.lid_closed != Some(true) {
            return false;
        }
        let Some((plan, journal)) = self.in_force() else {
            if self.may_own() {
                self.state.lock.work_end_waits = Some(sources);
            }
            return false;
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
        } else if self.journal_unknown() {
            self.state.lock.work_end_waits = Some(sources);
        }
        starts
    }

    /// A work-end want that waited ([`Core::work_ended_on`]), at each settle pass once a plan read worked. After the
    /// retry's restore of a journal that did not read, the plan copy still shows the crashed tray's Do nothing until
    /// the next read lands, and no entry puts the `original` back ([`Core::plan_copy`]); a restore at a hold end has no
    /// read right behind it at all. No pass can tell that read from an older one, so a Do nothing ends nothing: the
    /// want stays until a read shows the user's own action for its power source as not Do nothing, or the lid opens or
    /// a hold starts ([`Core::cancel_countdowns`]) (`p2-6-unread-lid-countdown`).
    fn work_end_waited(&mut self, sources: &'static [bool]) {
        if !self.work_ended_on(sources) {
            self.state.lock.work_end_waits = Some(sources);
        }
    }

    /// Ends the wants that `cancel` ends (`policy::ending::cancels`), with a log line when a countdown ran.
    fn cancel_countdowns(&mut self, cancel: Cancel, why: &str, out: &mut Vec<Command>) {
        // A want still waiting for a plan read ends the same way.
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
            out.push(Command::Log(format!("Countdown to sleep cancelled: {why}")));
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
    /// lasted `SOURCE_SETTLE_MS`, so a dock's short AC drop never sleeps a docked laptop left idle. Never while
    /// another session has the PC's screen: input idle is this session's, so the other user's typing would not count.
    /// With no countdown the short timeouts go back, and keys the user took back in it are Wakeshot's to write again.
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
                    "Countdown to sleep starts ({reason:?}): due after {} minutes without input",
                    ending::IDLE_MS / 60_000
                )));
                self.state.countdown = Some(Countdown {
                    reason,
                    check: Some(Deadline::Mono(now.mono_ms)),
                    no_hibernate: false,
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
                    out.push(Command::Log(format!("Countdown to sleep stops: {why}")));
                }
            }
            (Some(running), Some(reason)) if running.reason != reason => {
                if let Some(countdown) = self.state.countdown.as_mut() {
                    countdown.reason = reason;
                }
            }
            _ => {}
        }
        if self.state.countdown.is_none() {
            self.end_short("the countdown stopped", out);
            self.state.short_released.clear();
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
    /// a fresh one may make it due. Due: the sleep check, whose pass writes the short timeouts; where they cannot be
    /// written (settings blocked, another session owns them, the user took every short key back), `Hibernate` at once,
    /// as before the sleep route. With the short timeouts in force, the fallback instead.
    fn check_countdown(&mut self, now: Now, out: &mut Vec<Command>) {
        let Some(reading) = self.state.idle.filter(|r| r.at_mono == now.mono_ms) else {
            return;
        };
        let Some(countdown) = self.state.countdown else {
            return;
        };
        let floor_pct = self.floor_pct();
        // One at a time; and the floor as the hibernate thread will check it, so a percent that reads at the floor
        // again sends none.
        let waits = self.state.hibernating
            || (countdown.reason == CountdownReason::Floor
                && (self.state.above_floor_skipped
                    || !floor::below(self.state.battery_pct, floor_pct)));
        let clamp = self.countdown_clamp();
        let check = if let Some(short) = self.state.short {
            match short.fallback_mono {
                Some(at) if now.mono_ms < at => Some(Deadline::Mono(at)),
                Some(_) if !waits => {
                    out.push(Command::Log(format!(
                        "The PC did not sleep within {} minutes of the 1-minute timeouts ({:?}): hibernate",
                        ending::FALLBACK_MS / 60_000,
                        countdown.reason
                    )));
                    self.send_hibernate(countdown.reason, floor_pct, out);
                    None
                }
                _ => None,
            }
        } else {
            let idle = ending::clamped_idle_ms(reading.ms, now.mono_ms, clamp);
            match ending::check(idle, now.mono_ms) {
                Check::Wait(at) => Some(at),
                Check::Due if waits => None,
                Check::Due if self.can_write_short() => {
                    self.state.hibernating = true;
                    out.push(Command::Log(format!(
                        "Countdown to sleep due ({:?}): {} s without input; the last check, then the 1-minute \
                         timeouts",
                        countdown.reason,
                        idle / 1000
                    )));
                    // The hibernate thread plays the going to sleep sound after its last check (track note 4).
                    out.push(Command::SleepCheck {
                        reason: countdown.reason,
                        floor_pct,
                        last_resume_mono_ms: clamp,
                    });
                    None
                }
                Check::Due if !countdown.no_hibernate => {
                    out.push(Command::Log(format!(
                        "Countdown to sleep due ({:?}): {} s without input; the 1-minute timeouts cannot be \
                         written: hibernate",
                        countdown.reason,
                        idle / 1000
                    )));
                    self.send_hibernate(countdown.reason, floor_pct, out);
                    None
                }
                Check::Due => None,
            }
        };
        if let Some(countdown) = self.state.countdown.as_mut() {
            countdown.check = check;
        }
    }

    /// The hibernate thread plays the going to sleep sound after its last check (track note 4).
    fn send_hibernate(&mut self, reason: CountdownReason, floor_pct: u8, out: &mut Vec<Command>) {
        self.state.hibernating = true;
        out.push(Command::Hibernate {
            reason,
            floor_pct,
            last_resume_mono_ms: self.countdown_clamp(),
        });
    }

    /// Where the countdown's idle counts from at the latest: a resume, or an unlock, since a sign-in by face or
    /// fingerprint may give no input Windows counts, and the user who just signed in must not hear the going to sleep
    /// sound. The hibernate thread gets the same point, so its idle check agrees.
    fn countdown_clamp(&self) -> Option<u64> {
        self.state.last_resume_mono.max(self.state.last_unlock_mono)
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

    /// How the sleep check ended. Passed: the short timeouts. An unknown lid passes too, since the core's own reading
    /// says closed and the short timeouts go back when the lid opens (`Command::SleepCheck`). The lid open ends the
    /// countdown; input, AC and the floor go on as after a hibernate skip.
    pub(super) fn done_sleep_check(
        &mut self,
        result: Result<(), HibernateSkip>,
        now: Now,
        out: &mut Vec<Command>,
    ) {
        self.state.hibernating = false;
        match result {
            Ok(()) | Err(HibernateSkip::LidUnknown) => {
                out.push(Command::Log(
                    "Sleep check passed and the going to sleep sound played".to_owned(),
                ));
                self.write_short(now, out);
            }
            Err(HibernateSkip::LidOpen) => {
                out.push(Command::Log(
                    "Sleep check skipped: the lid is open".to_owned(),
                ));
                self.end_countdowns();
            }
            Err(skip) => self.skip_goes_on(skip, "Sleep check", out),
        }
    }

    /// A last check that found input, AC or the battery above the floor: input brings back the user's own timeouts,
    /// AC ends only a `Floor` countdown (`policy::ending::cancels`), the floor waits for the next battery reading; the
    /// next input reading decides again.
    fn skip_goes_on(&mut self, skip: HibernateSkip, what: &str, out: &mut Vec<Command>) {
        match skip {
            HibernateSkip::OnAc => {
                self.cancel_countdowns(Cancel::AcBack, "the PC is on AC", out);
            }
            HibernateSkip::AboveFloor => self.state.above_floor_skipped = true,
            HibernateSkip::InputSeen => self.end_short("the last check saw input", out),
            HibernateSkip::LidOpen | HibernateSkip::LidUnknown => {}
        }
        out.push(Command::Log(format!(
            "{what} skipped ({skip:?}): the countdown goes on"
        )));
        if let Some(countdown) = self.state.countdown.as_mut() {
            countdown.check = None;
        }
    }

    /// How the fallback hibernate ended. Not allowed, failed or an unknown lid: a notice, and no second hibernate in
    /// this countdown; the short timeouts stay, so Windows sleeps the laptop once nothing holds it, and with the lid
    /// still closed it is locked whenever it is left idle ([`Core::relock`]). A skip for the lid open ends the
    /// countdown; the other skips go on ([`Core::skip_goes_on`]). `Resumed` counts as a resume, and the countdown is
    /// done: the short timeouts go back with it.
    pub(super) fn done_hibernate(&mut self, end: HibernateEnd, now: Now, out: &mut Vec<Command>) {
        self.state.hibernating = false;
        match end {
            HibernateEnd::Resumed => {
                self.end_short("hibernated and resumed", out);
                self.take_resumed(now, out);
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
                    "Hibernate {what}: no second one in this countdown; the 1-minute timeouts stay where written"
                )));
                if let Some(countdown) = self.state.countdown.as_mut() {
                    countdown.no_hibernate = true;
                    countdown.check = None;
                }
                if let Some(short) = self.state.short.as_mut() {
                    short.fallback_mono = None;
                }
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
                self.end_short("the lid is open", out);
                self.end_countdowns();
            }
            HibernateEnd::Skipped(skip) => self.skip_goes_on(skip, "Hibernate", out),
        }
    }
}

impl WorkEnd {
    /// The user's own lid action for a power source, as it was when work ended.
    fn own_action(&self, ac: bool) -> u32 {
        if ac { self.own_lid_ac } else { self.own_lid_dc }
    }
}

/// A plan no read has shown, for the lock rules' guess: both lid keys at `lid`. Only the lid keys are guessed, so the
/// rest stay 0 and no rule reads them.
fn stand_in(active: Guid, lid: u32) -> PlanValues {
    PlanValues {
        active,
        lid_ac: lid,
        lid_dc: lid,
        sleep_ac: 0,
        sleep_dc: 0,
        saver_threshold_dc: 0,
    }
}

/// A key's value in the plan copy; `None` for the screen timeouts, which `PlanValues` does not carry and no lid or
/// lock rule reads.
fn value(plan: &PlanValues, key: SettingKey) -> Option<u32> {
    match key {
        SettingKey::LidAc => Some(plan.lid_ac),
        SettingKey::LidDc => Some(plan.lid_dc),
        SettingKey::SleepDc => Some(plan.sleep_dc),
        SettingKey::SleepAc => Some(plan.sleep_ac),
        SettingKey::VideoIdleAc | SettingKey::VideoIdleDc => None,
    }
}

fn set(plan: &mut PlanValues, key: SettingKey, value: u32) {
    match key {
        SettingKey::LidAc => plan.lid_ac = value,
        SettingKey::LidDc => plan.lid_dc = value,
        SettingKey::SleepDc => plan.sleep_dc = value,
        SettingKey::SleepAc => plan.sleep_ac = value,
        SettingKey::VideoIdleAc | SettingKey::VideoIdleDc => {}
    }
}

/// Wall time less run time: it grows by the time a sleep took, since run time stops then (research, "Clock").
fn clock_gap(now: Now) -> i64 {
    now.wall_ms
        .saturating_sub(i64::try_from(now.mono_ms).unwrap_or(i64::MAX))
}
