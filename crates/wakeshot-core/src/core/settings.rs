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

use std::collections::BTreeSet;

use super::Core;
use crate::command::{ApplyError, Command, Failure, RestoreWhy};
use crate::event::SessionChange;
use crate::ownership::apply::JOURNAL_CORRUPT;
use crate::settings::{Guid, JournalEntry, OwnedKey, PlanValues, SettingKey};
use crate::state::ApplyStep;
use crate::time::{Deadline, Now};
use crate::view::Notice;

/// The keys an apply writes, less the released ones (decision 17).
pub(super) const KEYS: [SettingKey; 3] =
    [SettingKey::LidAc, SettingKey::LidDc, SettingKey::SleepDc];

/// Do nothing for the lid and Never for sleep are both 0 (section 5, "Power settings").
pub(super) const TARGET: u32 = 0;

/// How often the plan is read while anything is owned (section 5, "Ownership"). Ticks read it no more often, so a
/// burst of short deadlines (the 1 s `state.json` wait) sends no burst of reads.
pub(super) const PLAN_READ_EVERY_MS: u64 = 10_000;

/// The access denied codes the design names (section 8): settings are blocked for the whole run.
const ACCESS_DENIED: [u32; 2] = [5, 1260];

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
            .is_some_and(|e| !e.requests_only && e.released.len() < KEYS.len())
            && state.owner
            && state.on_console
            && !self.holds_back()
            && !state.session_ending
            && !state.requests_only_run
    }

    /// Something may be overridden: the copy holds an entry, an apply is not answered yet, or a restore failed and
    /// the copy may not know what the journal holds.
    pub(super) fn may_own(&self) -> bool {
        !self.state.journal.is_empty()
            || !self.state.applying.is_empty()
            || self.state.retry.is_some()
    }

    fn may_own_on(&self, plan: Guid) -> bool {
        self.state.journal.iter().any(|e| e.plan == plan) || self.state.applying.contains(&plan)
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
                self.state.applying.push(plan);
            }
            // Track note 15: no plan values yet, so read first and apply at the answer.
            (ApplyStep::Wanted, true, None) => {
                episode.apply = ApplyStep::WaitPlan;
                self.read_plan(now, out);
            }
        }
    }

    /// At each tick, at most once per [`PLAN_READ_EVERY_MS`]: the restore retry, then a plan read, so the read's
    /// ownership check sees the journal the retry left.
    pub(super) fn tick(&mut self, now: Now, out: &mut Vec<Command>) {
        let due = self
            .state
            .plan_read_mono
            .is_none_or(|at| now.mono_ms.saturating_sub(at) >= PLAN_READ_EVERY_MS);
        if !due {
            return;
        }
        if let Some(why) = self.state.retry {
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
    /// failed apply it does not wait for an apply that never comes.
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

    /// `Plan`: a new active id is a plan switch, handled first. A failed read releases nothing.
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
        if newly == Some(true) {
            self.take_back(changed.key, out);
        }
    }

    /// A key just released may still be ours somewhere: an apply on its way writes it (a plan switch's read posts
    /// `Plan` before its check's `UserChanged`), or another plan's entry holds it (`p2-core-props` finding 1). The next
    /// apply leaves the key out and restores this plan's entry for it (track note 3); with every key released no apply
    /// comes, so a restore takes them back.
    fn take_back(&mut self, key: SettingKey, out: &mut Vec<Command>) {
        let ours =
            !self.state.applying.is_empty() || self.state.journal.iter().any(|e| e.key == key);
        if !ours {
            return;
        }
        if self.applies() {
            self.want_apply();
        } else if self
            .state
            .episode
            .as_ref()
            .is_some_and(|e| e.released.len() == KEYS.len())
        {
            out.push(Command::Log(
                "Every key is the user's for this episode: restore what Wakeshot may still hold"
                    .to_owned(),
            ));
            self.restore_all(RestoreWhy::WorkEnded, out);
        }
    }

    /// A corrupt journal was set aside: a new corrupt file may get its own restore.
    pub(super) fn take_journal_corrupt(&mut self, out: &mut Vec<Command>) {
        self.state.corrupt_restore_sent = false;
        self.notify(Notice::JournalCorrupt, out);
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
        if let Some(i) = self.state.applying.iter().position(|&p| p == plan) {
            self.state.applying.remove(i);
        }
        let read_nothing = matches!(result, Err(ApplyError::SafetyNet(_) | ApplyError::Write(_)))
            && journal.is_empty();
        if !read_nothing {
            self.state.journal = journal;
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

    /// Requests only (decision 29): for the run on access denied, else for the episode, with one notice per
    /// failure streak. What this episode owns goes back at once, since no apply will take it over. A failed write
    /// restores whatever the copy holds: the apply saved and maybe wrote entries, and its own restore may have failed
    /// to load the journal, which reports none (power safety review finding 2); a restore of a missing journal costs
    /// nothing. A safety net failure writes nothing.
    fn apply_failed(&mut self, failure: Failure, wrote: bool, out: &mut Vec<Command>) {
        let Failure { call, code } = failure;
        let holding = self.state.episode.is_some();
        let line = if ACCESS_DENIED.contains(&code) {
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
                if !std::mem::replace(&mut self.state.retry_warned, true) {
                    self.notify(Notice::RestoreFailed, out);
                }
            }
        }
        if why == RestoreWhy::UserAsked && std::mem::take(&mut self.state.restore_request) {
            self.state.finish_restore_request = true;
        }
    }
}

/// Log words, not tray words.
fn key_words(key: SettingKey) -> &'static str {
    match key {
        SettingKey::LidAc => "the lid action on AC",
        SettingKey::LidDc => "the lid action on battery",
        SettingKey::SleepDc => "sleep on battery",
    }
}
