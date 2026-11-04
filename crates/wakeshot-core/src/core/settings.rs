//! Power settings in the episode: when to `Apply` and `Restore`, the journal mirror, released keys, plan switches,
//! retries, the user's own actions, session end and fast user switching (architecture section 3, "Episode flow").
//! The IO side runs the commands (section 5); this side only decides and reads the outcomes back.
//!
//! Owner: `p2-core-settings`, then `p2-core-lid`.
//!
//! An apply is sent from the settle pass, after the requests, whenever the episode's apply is `Wanted` and the core
//! applies now. Every edge that may make the core apply sets `Wanted`: a hold start, `SettingsOwner`, `ConsoleBack`,
//! `EndCancelled`, a plan switch, the end of a step back (`core::lid`). A failed restore is kept as
//! `retry` and sent again at each tick, scoped so it never takes an entry a running hold uses.
//!
//! A journal the copy does not know (product "User settings safety" 7, `p2-6-record-lost`): a retry wanted while the
//! copy names no entry, or a damaged file just set aside. The retries go on for [`LOST_AFTER_MS`] of run time from the
//! first failure; then the journal is lost and `ResetLost` puts what looks like Wakeshot's back to Windows' defaults.
//! A damaged file is lost at once. Either way the window ends with `Core::unknown_journal_ended`.

use std::collections::BTreeSet;

use super::Core;
use crate::command::{ApplyError, Command, Failure, RestoreWhy};
use crate::event::SessionChange;
use crate::ownership::apply::JOURNAL_CORRUPT;
use crate::settings::{Guid, JournalEntry, OwnedKey, PlanValues, SettingKey};
use crate::state::{ApplyStep, Applying, Episode};
use crate::time::{Deadline, Now};
use crate::view::Notice;

/// How long a journal may stay unknown before it is lost: Haik's 2 minutes (product "User settings safety" 7). Run
/// time, so a PC that slept in between still gets its tries.
pub(super) const LOST_AFTER_MS: u64 = 120_000;

/// The keys a hold's apply writes, less the released ones (decision 17).
pub(super) const KEYS: [SettingKey; 3] = SettingKey::HOLD;

/// Do nothing for the lid and Never for sleep are both 0 (section 5, "Power settings").
pub(super) const TARGET: u32 = 0;

/// How often the plan is read while anything is owned (section 5, "Ownership"). Ticks read it no more often, so a
/// burst of short deadlines (the 1 s `state.json` wait) sends no burst of reads.
pub(super) const PLAN_READ_EVERY_MS: u64 = 10_000;

/// The access denied codes the design names (section 8): settings are blocked for the whole run, when a settings call
/// gives one.
const ACCESS_DENIED: [u32; 2] = [5, 1260];

/// The calls whose access denied means policy blocks the settings, as `power::plan` names them: the access check and
/// the two writes. The same code from any other call (the journal mutex an elevated `wakeshot restore` holds, a file
/// or registry write) says nothing about policy, so it holds requests only for the episode (power safety review
/// finding 8). A name the power track changes falls to the episode rule, the side that tries again.
const SETTINGS_CALLS: [&str; 3] = [
    "PowerSettingAccessCheckEx",
    "PowerWriteACValueIndex",
    "PowerWriteDCValueIndex",
];

impl Core {
    /// The core writes settings now: a hold counts, this tray owns the settings mutex, its session is on the PC's
    /// own screen, it has not stepped back (battery floor or a failed lock), the session is not ending, no apply
    /// failed in this episode or run, and some key is not released (architecture section 3, "Holding starts" and "A
    /// failed `Restore`").
    pub(super) fn applies(&self) -> bool {
        let state = &self.state;
        state
            .episode
            .as_ref()
            .is_some_and(|e| !e.requests_only && !every_hold_key_released(e))
            && state.owner
            && state.on_console
            && !self.holds_back()
            && !state.session_ending
            && !state.requests_only_run
    }

    /// Something may be overridden: the copy holds an entry, an apply is not answered yet, a restore failed and the
    /// copy may not know what the journal holds, or a lost journal's reset has not answered.
    pub(super) fn may_own(&self) -> bool {
        !self.state.journal.is_empty()
            || !self.state.applying.is_empty()
            || self.state.retry.is_some()
            || self.state.resetting.is_some()
    }

    /// The journal may hold entries the copy does not know: a restore is still wanted and the copy names none, or a
    /// reset of a lost journal is out. So it is from a `Started` whose `journal.json` did not read until a restore or
    /// apply reads the file (a failed load reports nothing, so the copy stays empty), after any restore that failed to
    /// load a journal the copy knew as empty, and from a damaged file's `JournalCorrupt` to its reset's answer.
    pub(super) fn journal_unknown(&self) -> bool {
        self.state.resetting.is_some()
            || (self.state.retry.is_some() && self.state.journal.is_empty())
    }

    /// The journal stayed unknown for [`LOST_AFTER_MS`], or a reset's answer did not come in that time (a post the
    /// IO thread could not make): the next reset is due, asking what the lost one asked.
    fn lost_due(&self, now: Now) -> bool {
        self.journal_unknown()
            && self
                .state
                .unknown_since
                .is_some_and(|at| now.mono_ms >= at.saturating_add(LOST_AFTER_MS))
    }

    /// `ResetLost`; the clock starts again, so an answer that never comes gets a second reset, not none.
    fn send_reset(&mut self, damaged: bool, now: Now, out: &mut Vec<Command>) {
        let why = match damaged {
            true => "journal.json was damaged and is set aside",
            false => "journal.json did not read for 2 minutes",
        };
        out.push(Command::Log(format!(
            "Journal lost ({why}): the lid, sleep and screen values that look like Wakeshot's go to Windows defaults"
        )));
        out.push(Command::ResetLost { damaged });
        self.state.resetting = Some(damaged);
        self.state.unknown_since = Some(now.mono_ms);
    }

    /// The settle pass's part for a journal the copy does not know: when it became so, and when the reset is due. The
    /// window's end, known or lost, goes to the countdown hook before the lid side settles.
    pub(super) fn settle_journal(&mut self, now: Now) -> Option<Deadline> {
        if self.journal_unknown() {
            let since = *self.state.unknown_since.get_or_insert(now.mono_ms);
            return Some(Deadline::Mono(since.saturating_add(LOST_AFTER_MS)));
        }
        if self.state.unknown_since.take().is_some() {
            self.unknown_journal_ended();
        }
        None
    }

    fn may_own_on(&self, plan: Guid) -> bool {
        self.state.journal.iter().any(|e| e.plan == plan)
            || self.state.applying.iter().any(|a| a.plan == plan)
    }

    /// Apply as at a hold start, once this step's requests are out. Settle drops it when the core does not apply.
    pub(super) fn want_apply(&mut self) {
        if let Some(episode) = self.state.episode.as_mut() {
            episode.apply = ApplyStep::Wanted;
        }
    }

    /// Every entry back, when something may be overridden (a hold end, the console left, a step back).
    pub(super) fn restore_all(&mut self, why: RestoreWhy, out: &mut Vec<Command>) {
        if self.may_own() {
            out.push(Command::Restore { plan: None, why });
        }
    }

    pub(super) fn read_plan(&mut self, now: Now, out: &mut Vec<Command>) {
        out.push(Command::ReadPlan);
        self.state.plan_read_mono = Some(now.mono_ms);
    }

    /// The settle pass's part: the apply the episode waits for, after the requests and the episode edges.
    pub(super) fn send_apply(&mut self, now: Now, out: &mut Vec<Command>) {
        let applies = self.applies();
        let active = self.state.plan.map(|p| p.active);
        let Some(episode) = self.state.episode.as_mut() else {
            return;
        };
        match (episode.apply, applies, active) {
            (ApplyStep::Idle, ..) | (ApplyStep::WaitPlan, true, _) => {}
            (_, false, _) => episode.apply = ApplyStep::Idle,
            (ApplyStep::Wanted, true, Some(plan)) => {
                episode.apply = ApplyStep::Idle;
                let writes = KEYS
                    .into_iter()
                    .filter(|key| !episode.released.contains(key))
                    .map(|key| (key, TARGET))
                    .collect();
                out.push(Command::Apply { plan, writes });
                self.state.applying.push(Applying { plan, short: false });
            }
            // Track note 15: no plan values yet, so read first and apply at the answer.
            (ApplyStep::Wanted, true, None) => {
                episode.apply = ApplyStep::WaitPlan;
                self.read_plan(now, out);
            }
        }
    }

    /// At each tick, at most once per [`PLAN_READ_EVERY_MS`]: the restore retry, then a plan read, so the read's
    /// ownership check sees the journal the retry left. A lost journal's reset goes in place of the retry, on its own
    /// clock, so the 2 minutes never wait for the read's 10 s; while it is out, its answer decides, so no retry.
    pub(super) fn tick(&mut self, now: Now, out: &mut Vec<Command>) {
        if self.lost_due(now) {
            let damaged = self.state.resetting.unwrap_or(false);
            self.send_reset(damaged, now, out);
        }
        let due = self
            .state
            .plan_read_mono
            .is_none_or(|at| now.mono_ms.saturating_sub(at) >= PLAN_READ_EVERY_MS);
        if !due {
            return;
        }
        if self.state.resetting.is_none()
            && let Some(why) = self.state.retry
        {
            out.extend(
                self.restore_scope()
                    .into_iter()
                    .map(|plan| Command::Restore { plan, why }),
            );
        }
        self.read_plan(now, out);
    }

    /// The next plan read while anything is owned, or while an apply waits for a plan.
    pub(super) fn plan_read_due(&self, now: Now) -> Option<Deadline> {
        let waits = self
            .state
            .episode
            .as_ref()
            .is_some_and(|e| e.apply == ApplyStep::WaitPlan);
        (waits || !self.state.journal.is_empty()).then(|| {
            let due = self
                .state
                .plan_read_mono
                .map_or(now.mono_ms, |at| at.saturating_add(PLAN_READ_EVERY_MS));
            Deadline::Mono(due.max(now.mono_ms))
        })
    }

    /// What a restore retry takes now. While the core applies, only the entries on other plans, one restore per
    /// plan: the hold's apply takes over the active plan's entries, or restores those whose key is released (track
    /// note 3), so a retry never undoes a running hold (power verify minor 4). Otherwise every entry, so after a
    /// failed apply it does not wait for an apply that never comes; the countdown's short timeouts go too, and out
    /// again once it worked ([`Core::rewrite_short`]).
    fn restore_scope(&self) -> Vec<Option<Guid>> {
        match self.state.plan.filter(|_| self.applies()) {
            Some(PlanValues { active, .. }) => {
                let plans: BTreeSet<Guid> = self
                    .state
                    .journal
                    .iter()
                    .map(|e| e.plan)
                    .filter(|&p| p != active)
                    .collect();
                plans.into_iter().map(Some).collect()
            }
            None => vec![None],
        }
    }

    /// `Plan`: a new active id is a plan switch, handled first. A failed read releases nothing. The sleep route's short
    /// timeouts follow the switch as a hold's values do (product "User settings safety" 4).
    pub(super) fn take_plan(&mut self, read: Result<PlanValues, Failure>, out: &mut Vec<Command>) {
        let Ok(values) = read else {
            return;
        };
        let old = self.state.plan.replace(values).map(|p| p.active);
        if let Some(old) = old.filter(|&old| old != values.active) {
            // An inactive plan needs no apply, so its restore is all the old plan needs (section 5).
            if self.may_own_on(old) {
                out.push(Command::Restore {
                    plan: Some(old),
                    why: RestoreWhy::PlanSwitch,
                });
            }
            self.want_apply();
            if self.state.short.is_some() && self.can_write_short() {
                self.send_short(values.active, out);
            }
        } else if let Some(episode) = self.state.episode.as_mut()
            && episode.apply == ApplyStep::WaitPlan
        {
            episode.apply = ApplyStep::Wanted;
        }
    }

    /// `ActivePlan`: only a read, and only when the plan matters; the `Plan` answer drives the switch.
    pub(super) fn take_active_plan(&mut self, id: Guid, now: Now, out: &mut Vec<Command>) {
        let known = self.state.plan.map(|p| p.active);
        if known != Some(id) && (self.may_own() || self.state.episode.is_some()) {
            self.read_plan(now, out);
        }
    }

    /// `UserChanged`: the IO thread already dropped the entry. The key is released on every plan for the episode
    /// (power verify minor 9), and the user hears it once per key.
    pub(super) fn take_user_changed(&mut self, changed: OwnedKey, out: &mut Vec<Command>) {
        self.state
            .journal
            .retain(|e| (e.plan, e.key) != (changed.plan, changed.key));
        // The sleep route writes it again after input only while the user has not changed it (decision 26).
        let short_newly = self.state.countdown.is_some()
            && SettingKey::SHORT.contains(&changed.key)
            && self.state.short_released.insert(changed.key);
        let what = key_words(changed.key);
        let newly = self
            .state
            .episode
            .as_mut()
            .map(|e| e.released.insert(changed.key));
        let line = match newly {
            Some(false) => return,
            Some(true) => format!(
                "Released {what} for this episode: the user changed it on plan {}",
                changed.plan
            ),
            None => format!("The user changed {what} on plan {}", changed.plan),
        };
        out.push(Command::Log(line));
        self.notify(Notice::UserChanged(changed.key), out);
        // A work-end countdown has no episode, yet its release is the same (architecture, "The sleep route" 4).
        if newly == Some(true) || short_newly {
            self.take_back(changed.key, out);
        }
    }

    /// A key just released may still be ours somewhere: an apply on its way writes it (a plan switch's read posts
    /// `Plan` before its check's `UserChanged`), or another plan's entry holds it (`p2-core-props` finding 1). The next
    /// apply leaves the key out and restores this plan's entry for it (track note 3); with every key released no apply
    /// comes, so a restore takes them back, and with them the short timeouts, which go out again once it worked
    /// ([`Core::rewrite_short`]). The sleep route's apply races the same way (`p2-6-props-flake`): the switch's, or one
    /// sent while the user's change waited for its check, and it runs while no hold applies.
    fn take_back(&mut self, key: SettingKey, out: &mut Vec<Command>) {
        let ours =
            !self.state.applying.is_empty() || self.state.journal.iter().any(|e| e.key == key);
        if !ours {
            return;
        }
        if self.applies() {
            self.want_apply();
        } else if self.state.short.is_some() && SettingKey::SHORT.contains(&key) {
            self.short_again(out);
        } else if self
            .state
            .episode
            .as_ref()
            .is_some_and(every_hold_key_released)
        {
            out.push(Command::Log(
                "Every key is the user's for this episode: restore what Wakeshot may still hold"
                    .to_owned(),
            ));
            self.restore_all(RestoreWhy::WorkEnded, out);
        }
    }

    /// A corrupt journal was set aside: lost at once (product "User settings safety" 7), and a new corrupt file may get
    /// its own restore.
    pub(super) fn take_journal_corrupt(&mut self, now: Now, out: &mut Vec<Command>) {
        self.state.corrupt_restore_sent = false;
        self.send_reset(true, now, out);
    }

    /// `ResetLost` answered. Entries mean the file read after all: the copy takes them, and the retry goes on as
    /// before. Otherwise the journal is known and empty, whatever the result: a reset that failed leaves values only
    /// the user can tell apart, and the notice says so; with the lid closed since the window it also locks
    /// ([`Core::lost_reset_failed`]). One notice when anything was reset or a call failed; nothing reset is the log's
    /// only. A running hold and the countdown's short timeouts apply again, since the reset may have taken their values
    /// too. A read follows, since the plan copy still shows the values from before.
    ///
    /// Entries after a damaged file went aside are this tray's own apply, when this tray owns the settings: nothing
    /// else writes entries, and only the owner applies. It ran between the set-aside and this reset (the IO queue had
    /// it already), so it took the values in force as its originals, Wakeshot's own among them: a hold that starts
    /// while the 1-minute timeouts are in force journals battery sleep with 60 s as the user's (review major 2, case
    /// B). So its entries go back, the reset runs again on the file that is then gone, and the hold and the short
    /// timeouts apply again over the defaults, all in this order on the IO queue. Once only: a second find keeps the
    /// entries.
    pub(super) fn done_reset(
        &mut self,
        reset: Vec<SettingKey>,
        result: Result<(), Failure>,
        journal: Vec<JournalEntry>,
        now: Now,
        out: &mut Vec<Command>,
    ) {
        let damaged = self.state.resetting.take() == Some(true);
        let rerun = std::mem::take(&mut self.state.reset_rerun);
        if !journal.is_empty() && damaged && self.state.owner && !rerun {
            out.push(Command::Log(format!(
                "Journal reset after a damaged file found {} entries of this tray's own apply, taken over values from \
                 before: they go back, the reset runs again, then Wakeshot applies again",
                journal.len()
            )));
            self.state.journal = journal;
            self.state.reset_rerun = true;
            self.restore_all(RestoreWhy::WorkEnded, out);
            self.send_reset(true, now, out);
            self.want_apply();
            if self.state.short.is_some() && self.can_write_short() {
                let plan = self.state.plan.map(|p| p.active);
                if let Some(plan) = plan {
                    self.send_short(plan, out);
                }
            }
            return;
        }
        if journal.is_empty() {
            self.state.journal = journal;
            self.state.retry = None;
            self.state.retry_warned = false;
            self.state.corrupt_restore_sent = false;
            let what: Vec<&str> = reset.iter().map(|&k| key_words(k)).collect();
            let mut line = match what.is_empty() {
                true => "Journal reset: nothing held Wakeshot's values".to_owned(),
                false => format!("Journal reset: Windows defaults for {}", what.join(", ")),
            };
            if let Err(Failure { call, code }) = result {
                line.push_str(&format!("; failed ({call}, code {code})"));
            }
            out.push(Command::Log(line));
            if result.is_err() || !reset.is_empty() {
                let notice = Notice::JournalLost {
                    reset,
                    failed: result.is_err(),
                };
                self.keep_for_return(notice.clone());
                self.notify(notice, out);
            }
            if result.is_err() {
                self.lost_reset_failed(now, out);
            }
            // Its own load may fail right after a running hold's apply wrote, and then it puts the hold's values to the
            // defaults too (`p2-6-reset-gaps`, gap A): what a hold or the countdown still wants applies again. An
            // apply finds nothing to write where nothing was reset; settle drops the hold's when the core does not
            // apply. One still out was sent after this reset, since answers come in order, so it writes already.
            if !self.state.applying.iter().any(|a| !a.short) {
                self.want_apply();
            }
            self.rewrite_short(out);
        } else {
            out.push(Command::Log(format!(
                "Journal reset not needed: journal.json reads now, with {} entries",
                journal.len()
            )));
            self.state.journal = journal;
        }
        self.read_plan(now, out);
    }

    /// "Restore my power settings": restore now, release every key for the episode, keep the requests (decision 26).
    pub(super) fn menu_restore(&mut self, out: &mut Vec<Command>) {
        out.push(Command::Restore {
            plan: None,
            why: RestoreWhy::UserAsked,
        });
        self.state.restore_request = true;
        let line = match self.state.episode.as_mut() {
            Some(episode) => {
                episode.released.extend(KEYS);
                "Released every key for this episode: Restore my power settings"
            }
            None => "Restore my power settings: nothing holds",
        };
        out.push(Command::Log(line.to_owned()));
        self.notify(Notice::RestoredByUser, out);
    }

    /// Quit: requests off, every entry back, Keep awake ends with the tray, then exit. Nothing follows.
    pub(super) fn menu_quit(&mut self, out: &mut Vec<Command>) {
        self.state.quit = true;
        self.state.next = None;
        self.state.requests = crate::policy::requests::Requests::NONE;
        out.push(Command::Requests {
            system: false,
            display: false,
        });
        out.push(Command::Restore {
            plan: None,
            why: RestoreWhy::Quit,
        });
        out.extend(self.keep_awake_files().into_iter().map(Command::DeleteHold));
        out.push(Command::Quit);
    }

    pub(super) fn take_session(&mut self, change: SessionChange, now: Now, out: &mut Vec<Command>) {
        match change {
            SessionChange::Locked => self.take_locked(out),
            SessionChange::Unlocked => self.take_unlocked(now, out),
            // The last chance before Windows ends the process, so it goes out whatever the copy says.
            SessionChange::Ending => {
                self.state.session_ending = true;
                out.push(Command::Restore {
                    plan: None,
                    why: RestoreWhy::SessionEnd,
                });
            }
            SessionChange::EndCancelled => {
                self.state.session_ending = false;
                self.want_apply();
            }
            // `Ending` restored already; an `Ended` without it still must not leave an override.
            SessionChange::Ended if self.state.session_ending => {}
            SessionChange::Ended => {
                self.state.session_ending = true;
                out.push(Command::Restore {
                    plan: None,
                    why: RestoreWhy::SessionEnd,
                });
            }
            // Plans are machine-wide: the other user would meet our Do nothing (Question 10, Haik's answer). Settle
            // drops the display request.
            SessionChange::ConsoleLeft => {
                self.state.on_console = false;
                out.push(Command::Log(
                    "Another session has the PC's screen: requests only until it comes back"
                        .to_owned(),
                ));
                self.restore_all(RestoreWhy::ConsoleLeft, out);
            }
            SessionChange::ConsoleBack => {
                self.state.on_console = true;
                self.want_apply();
                self.send_held(out);
            }
        }
    }

    pub(super) fn done_applied(
        &mut self,
        plan: Guid,
        result: Result<(), ApplyError>,
        journal: Vec<JournalEntry>,
        now: Now,
        out: &mut Vec<Command>,
    ) {
        let short = self
            .state
            .applying
            .iter()
            .position(|a| a.plan == plan)
            .is_some_and(|i| self.state.applying.remove(i).short);
        let read_nothing = matches!(result, Err(ApplyError::SafetyNet(_) | ApplyError::Write(_)))
            && journal.is_empty();
        if !read_nothing {
            self.state.journal = journal;
        }
        if short {
            self.done_short_applied(plan, result, now, out);
            return;
        }
        match result {
            Ok(()) => self.state.requests_only_warned = false,
            // Step 1 wrote nothing; step 6 left the writes journaled on a plan now inactive. A switch already seen
            // sent that plan's restore behind this apply; otherwise a fresh read decides.
            Err(ApplyError::PlanChanged) => {
                if self.state.plan.map(|p| p.active) == Some(plan) && self.applies() {
                    if let Some(episode) = self.state.episode.as_mut() {
                        episode.apply = ApplyStep::WaitPlan;
                    }
                    self.read_plan(now, out);
                }
            }
            Err(ApplyError::SafetyNet(failure)) => self.apply_failed(failure, false, out),
            Err(ApplyError::Write(failure)) => self.apply_failed(failure, true, out),
        }
    }

    /// Requests only (decision 29): for the run on access denied from a settings call, else for the episode, with one
    /// notice per failure streak. What this episode owns goes back at once, since no apply will take it over. A failed write
    /// restores whatever the copy holds: the apply saved and maybe wrote entries, and its own restore may have failed
    /// to load the journal, which reports none (power safety review finding 2); a restore of a missing journal costs
    /// nothing. A safety net failure writes nothing.
    fn apply_failed(&mut self, failure: Failure, wrote: bool, out: &mut Vec<Command>) {
        let Failure { call, code } = failure;
        let holding = self.state.episode.is_some();
        let line = if ACCESS_DENIED.contains(&code) && SETTINGS_CALLS.contains(&call) {
            self.state.requests_only_run = true;
            format!("Settings access denied ({call}, code {code}): requests only for this run")
        } else if let Some(episode) = self.state.episode.as_mut() {
            episode.requests_only = true;
            episode.apply = ApplyStep::Idle;
            format!("Apply failed ({call}, code {code}): requests only for this episode")
        } else {
            format!("Apply failed ({call}, code {code}) after the hold ended")
        };
        out.push(Command::Log(line));
        if !holding {
            // The hold end's restore is queued behind this apply.
            return;
        }
        if !std::mem::replace(&mut self.state.requests_only_warned, true) {
            self.notify(Notice::RequestsOnly, out);
        }
        // A corrupt file fails every apply until a restore sets it aside (orchestrator, 2026-11-03 20:39).
        let corrupt = failure == JOURNAL_CORRUPT;
        let first_corrupt =
            corrupt && !std::mem::replace(&mut self.state.corrupt_restore_sent, true);
        if first_corrupt || (!corrupt && (wrote || !self.state.journal.is_empty())) {
            out.push(Command::Restore {
                plan: None,
                why: RestoreWhy::WorkEnded,
            });
        }
    }

    pub(super) fn done_restored(
        &mut self,
        plan: Option<Guid>,
        why: RestoreWhy,
        result: Result<(), Failure>,
        journal: Vec<JournalEntry>,
        out: &mut Vec<Command>,
    ) {
        // A failed load reports `journal: []` since nothing is known: never "nothing owned" (`p2-ownership-restore`).
        let read_nothing = result.is_err() && journal.is_empty();
        if !read_nothing {
            self.state.journal = journal;
        }
        match result {
            // On `Ok` the copy is exact, so it can tell whether anything is left for a retry.
            Ok(()) if plan.is_none() || self.restore_scope().is_empty() => {
                self.state.retry = None;
                self.state.retry_warned = false;
                self.state.corrupt_restore_sent = false;
            }
            Ok(()) => {}
            Err(_) => {
                self.state.retry = Some(why);
                // A journal the copy does not know ends in a good read or in the reset, which has its own notice, so
                // the user gets one word for it, not two (product "User settings safety" 7).
                if !self.journal_unknown() && !std::mem::replace(&mut self.state.retry_warned, true)
                {
                    self.notify(Notice::RestoreFailed, out);
                }
            }
        }
        if why == RestoreWhy::UserAsked && std::mem::take(&mut self.state.restore_request) {
            self.state.finish_restore_request = true;
        }
        // A retry, or the take back of a released hold key, takes every plan's entries, the short timeouts' too
        // (`p2-6-reset-gaps`, gap B). A plan switch's restore of one plan goes out before their apply to the new plan,
        // which is then still out. The user's own restore wins.
        if result.is_ok() && why != RestoreWhy::UserAsked {
            self.rewrite_short(out);
        }
    }
}

/// The user took back every key a hold writes. A short key the user changed in a stepped-back episode is released too
/// (`core::lid`), but it is not one of these, so it never stops the hold's apply (review minor 4).
fn every_hold_key_released(episode: &Episode) -> bool {
    KEYS.iter().all(|key| episode.released.contains(key))
}

/// Log words, not tray words.
fn key_words(key: SettingKey) -> &'static str {
    match key {
        SettingKey::LidAc => "the lid action on AC",
        SettingKey::LidDc => "the lid action on battery",
        SettingKey::SleepDc => "sleep on battery",
        SettingKey::SleepAc => "sleep on AC",
        SettingKey::VideoIdleAc => "screen off on AC",
        SettingKey::VideoIdleDc => "screen off on battery",
    }
}
