//! The apply (architecture section 5, "Apply", steps 1 to 6). `wakeshot-windows` calls it through `apply_locked`.

use crate::command::{ApplyError, Failure};
use crate::ownership::io::{JournalRead, PlanIo, SafetyNet};
use crate::ownership::journal::JournalParts;
use crate::ownership::restore::{restore, save_or_clean_up};
use crate::settings::{Guid, Journal, JournalEntry, JournalOwner, SettingKey};

/// What the apply and the ownership check report for a `journal.json` that is there but does not parse. The code is
/// `ERROR_FILE_CORRUPT`. Neither sets the file aside: only the restore routine's outcome can tell the core so
/// (`Found::SetAside`), and the core shows the notice from that.
pub const JOURNAL_CORRUPT: Failure = Failure {
    call: "journal.json (does not parse)",
    code: 1392,
};

/// What the apply reports when the read-back after a write returns another value. Windows gives no code for it, so
/// the code is `ERROR_INVALID_DATA`.
pub const READ_BACK_DIFFERS: Failure = Failure {
    call: "read-back (another value than written)",
    code: 13,
};

/// A live value that already does what a write of `target` would. A timeout target other than Never is a ceiling
/// (the sleep route's 1 minute): a value at or under it, and not Never, sleeps the PC as soon or sooner, so it is the
/// user's own and stays. Any other target is met only by itself.
pub fn meets(key: SettingKey, live: u32, target: u32) -> bool {
    if key.is_timeout() && target != 0 {
        live != 0 && live <= target
    } else {
        live == target
    }
}

/// How one apply went, for `Outcome::Applied`. `journal`: the known entries left after it.
///
/// - `written`: the keys whose target this run wrote (the write call returned `Ok`), in call order, whatever `result`
///   says. A key skipped as the user's own value or as the user's change since our write, or already at its target,
///   is not in it.
/// - `journal`: the known entries the file holds after the run, in every plan; empty when the file was missing,
///   corrupt or unreadable. After a failed save that this run tolerates or reports, it is the most the file can hold.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Applied {
    pub written: Vec<SettingKey>,
    pub result: Result<(), ApplyError>,
    pub journal: Vec<JournalEntry>,
}

/// One apply to `plan` with `writes`, owned by `me`. Entries on `plan` whose key is not in `writes` are restored in
/// the same run (`Command::Apply` doc). `writes` names each key at most once.
///
/// Call order: `load_journal`; `active_plan`; `can_write` per key, then `read` per key, then `read` per entry on
/// `plan` for another key; with nothing to own or restore, at most the save of the drops; else `set_run_once`,
/// `ensure_guard`, `save_journal`; per entry in journal order a `write` (and for our target its read-back);
/// `active_plan`; `set_active`; one save (or the clean-up) only when a mark or a released entry is left to clear.
pub fn apply(
    plans: &mut impl PlanIo,
    net: &mut impl SafetyNet,
    me: JournalOwner,
    plan: Guid,
    writes: &[(SettingKey, u32)],
) -> Applied {
    let refused = |error, journal| Applied {
        written: Vec::new(),
        result: Err(error),
        journal,
    };
    // Loaded before step 1 so that every way out reports the entries the file holds: the core's mirror is a copy of
    // `journal`, and an empty one would read as nothing owned.
    let (parts, absent) = match net.load_journal() {
        JournalRead::Missing => {
            let empty = JournalParts {
                known: Journal {
                    owner: me,
                    entries: Vec::new(),
                },
                other: Vec::new(),
            };
            (empty, true)
        }
        JournalRead::Read(file) => {
            let parts = JournalParts::split(file);
            // Only a newer version's entries: this version's RunOnce value and guard do nothing for them.
            let absent = parts.known.entries.is_empty();
            (parts, absent)
        }
        // With the entries unknown, the skip rule and the takeover cannot be applied, so nothing may be written.
        JournalRead::Corrupt => return refused(ApplyError::SafetyNet(JOURNAL_CORRUPT), Vec::new()),
        JournalRead::Failed(failure) => return refused(ApplyError::SafetyNet(failure), Vec::new()),
    };
    let JournalParts {
        known: Journal {
            owner: found_owner,
            entries: loaded,
        },
        other,
    } = parts;

    // Step 1.
    match plans.active_plan() {
        Err(failure) => return refused(ApplyError::SafetyNet(failure), loaded),
        Ok(active) if active != plan => return refused(ApplyError::PlanChanged, loaded),
        Ok(_) => {}
    }

    // Step 2.
    let slots = match step_2(plans, &loaded, plan, writes) {
        Ok(slots) => slots,
        Err(failure) => return refused(ApplyError::SafetyNet(failure), loaded),
    };
    let saved = entries(&slots);
    if slots.iter().all(|s| s.role == Role::Keep) {
        if saved == loaded {
            return Applied {
                written: Vec::new(),
                result: Ok(()),
                journal: loaded,
            };
        }
        // Nothing to own or restore, but released entries the user changed (or never written) leave the journal, as
        // the restore routine's step 3 drops them; with nothing left that is the clean-up.
        let parts = JournalParts {
            known: Journal {
                owner: found_owner,
                entries: saved,
            },
            other,
        };
        return match save_or_clean_up(net, &parts) {
            Ok(()) => Applied {
                written: Vec::new(),
                result: Ok(()),
                journal: parts.known.entries,
            },
            Err(failure) => refused(ApplyError::SafetyNet(failure), loaded),
        };
    }

    // Step 3: the journal last, so a failure here never leaves entries for keys never written, which the next check
    // would take for the user's changes.
    let added_run_once = match net.set_run_once() {
        Ok(added) => added,
        Err(failure) => return refused(ApplyError::SafetyNet(failure), loaded),
    };
    let added_guard = match net.ensure_guard() {
        Ok(added) => added,
        Err(failure) => {
            take_back(net, absent && added_run_once, false);
            return refused(ApplyError::SafetyNet(failure), loaded);
        }
    };
    let mut parts = JournalParts {
        known: Journal {
            owner: me,
            entries: saved,
        },
        other,
    };
    if let Err(failure) = net.save_journal(&parts.merge()) {
        // Step 4. Entries already in the file need the RunOnce value and the guard, whoever added them.
        take_back(net, absent && added_run_once, absent && added_guard);
        return refused(ApplyError::SafetyNet(failure), loaded);
    }

    // Step 5.
    let mut written = Vec::new();
    let wrote = slots
        .iter()
        .try_for_each(|slot| step_5(plans, plan, slot, &mut written));

    // Step 6.
    let active = match wrote.and_then(|()| plans.active_plan()) {
        Ok(active) => active,
        Err(failure) => return undo(plans, net, plan, written, failure),
    };
    if active != plan {
        // Written to a plan now inactive and journaled: the plan switch path restores it (section 5).
        return Applied {
            written,
            result: Err(ApplyError::PlanChanged),
            journal: parts.known.entries,
        };
    }
    if let Err(failure) = plans.set_active(active) {
        return undo(plans, net, plan, written, failure);
    }
    let on_disk = std::mem::take(&mut parts.known.entries);
    parts.known.entries = slots
        .iter()
        .filter_map(|s| match s.role {
            Role::Keep => Some(s.entry),
            // Our value is in force again, so the mark of a taken-over entry or a new entry has done its job.
            Role::Own { .. } => Some(JournalEntry {
                restored: false,
                ..s.entry
            }),
            Role::Release { .. } => None,
        })
        .collect();
    if parts.known.entries == on_disk {
        return Applied {
            written,
            result: Ok(()),
            journal: on_disk,
        };
    }
    let journal = match save_or_clean_up(net, &parts) {
        Ok(()) => parts.known.entries,
        // Tolerated (Apply step 6): a mark left only hides from the check a user change back to exactly `original`,
        // until the next restore, which handles a marked entry right; a released entry left is dropped by that restore.
        Err(_) => on_disk,
    };
    Applied {
        written,
        result: Ok(()),
        journal,
    }
}

/// What this run does with one known entry of the journal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Role {
    /// Saved back as it was: an entry on another plan, or one on this plan for a key in `writes` that the user changed
    /// since our write (left for the ownership check to report).
    Keep,
    /// A key in `writes`, ours from step 3 on. `write`: its live value is not the target yet.
    Own { write: bool },
    /// This plan's entry for a key not in `writes`, marked for its restore in the same run. `write`: its `original` is
    /// not stored yet.
    Release { write: bool },
}

#[derive(Clone, Copy, Debug)]
struct Slot {
    entry: JournalEntry,
    role: Role,
}

fn entries(slots: &[Slot]) -> Vec<JournalEntry> {
    slots.iter().map(|s| s.entry).collect()
}

/// Step 2: each key's access check, then its live value, then the live value of this plan's entries for other keys.
/// Gives the journal's entries in their order, each with what this run does with it, and the new entries last.
fn step_2(
    plans: &mut impl PlanIo,
    loaded: &[JournalEntry],
    plan: Guid,
    writes: &[(SettingKey, u32)],
) -> Result<Vec<Slot>, Failure> {
    for &(key, _) in writes {
        plans.can_write(key)?;
    }
    let mut live = Vec::with_capacity(writes.len());
    for &(key, _) in writes {
        live.push(plans.read(plan, key)?);
    }
    let mut slots = Vec::with_capacity(loaded.len() + writes.len());
    let mut has_entry = vec![false; writes.len()];
    for &e in loaded {
        if e.plan != plan {
            slots.push(Slot {
                entry: e,
                role: Role::Keep,
            });
            continue;
        }
        let slot = match writes.iter().position(|&(k, _)| k == e.key) {
            Some(i) => {
                has_entry[i] = true;
                own(Some(e), plan, writes[i], live[i])
            }
            None => release(e, plans.read(plan, e.key)?),
        };
        slots.extend(slot);
    }
    for (i, &w) in writes.iter().enumerate() {
        if !has_entry[i] {
            slots.extend(own(None, plan, w, live[i]));
        }
    }
    Ok(slots)
}

/// Step 2's decision for a key in `writes`, with its entry on this plan, if any.
fn own(
    entry: Option<JournalEntry>,
    plan: Guid,
    (key, target): (SettingKey, u32),
    live: u32,
) -> Option<Slot> {
    let slot = |entry: JournalEntry| Slot {
        entry,
        role: Role::Own {
            write: live != target,
        },
    };
    let ours = |e: JournalEntry| live == e.written || (e.restored && live == e.original);
    match entry {
        // Still ours, but the user's own value meets the ceiling (a hold's battery sleep Never left by a failed
        // restore, over the user's own 30 s): that value goes back, as for a key not in `writes`.
        Some(e) if ours(e) && target != 0 && meets(key, e.original, target) => release(e, live),
        // Still ours (restore step 3's two "ours" cases): taken over with its `original` and its mark (Apply step 3),
        // and `written` becomes this run's target: a hold takes over the sleep route's 1 minute, and the other way.
        Some(e) if ours(e) => Some(slot(JournalEntry {
            written: target,
            ..e
        })),
        // Neither ours nor `original`: the user changed it since our write, and no ownership check has seen it yet
        // (one runs every 10 s). Writing over it would break "the latest change wins at once" (decision 26), so the
        // key is skipped and the entry kept as it is: the next check reports it and the core releases the key.
        Some(e) if live != e.original => Some(Slot {
            entry: e,
            role: Role::Keep,
        }),
        // No entry: the live value is the user's own. At the target, or under a ceiling, it is skipped.
        _ if meets(key, live, target) => None,
        // No entry, or one never written (not marked, live equals `original`): a new entry, marked until our value is
        // in force, so a run that never writes it leaves nothing the check reads as the user's change. Keeping an
        // older `original` could put back a value older than the user's latest.
        _ => Some(slot(JournalEntry {
            plan,
            key,
            original: live,
            written: target,
            restored: true,
        })),
    }
}

/// Step 2's decision for this plan's entry for a key not in `writes`: restore step 3's rules.
fn release(e: JournalEntry, live: u32) -> Option<Slot> {
    let role = match live {
        // Ours stored, marked or not: marked in step 3's save, `original` written in step 5.
        _ if live == e.written => Role::Release { write: true },
        // Marked with `original` stored: only the call is left.
        _ if e.restored && live == e.original => Role::Release { write: false },
        // The user's, or never written.
        _ => return None,
    };
    Some(Slot {
        entry: JournalEntry {
            restored: true,
            ..e
        },
        role,
    })
}

/// Step 5 for one entry: our target with its read-back, or a released entry's `original`.
fn step_5(
    plans: &mut impl PlanIo,
    plan: Guid,
    slot: &Slot,
    written: &mut Vec<SettingKey>,
) -> Result<(), Failure> {
    let e = slot.entry;
    match slot.role {
        Role::Own { write: true } => {
            plans.write(plan, e.key, e.written)?;
            written.push(e.key);
            match plans.read(plan, e.key)? {
                back if back == e.written => Ok(()),
                _ => Err(READ_BACK_DIFFERS),
            }
        }
        Role::Release { write: true } => plans.write(plan, e.key, e.original),
        _ => Ok(()),
    }
}

/// Step 4's take-back of what this apply added. A failure here leaves a RunOnce value or a guard with no journal,
/// and both then find nothing to do, so it is not reported over the failure that led here.
fn take_back(net: &mut impl SafetyNet, run_once: bool, guard: bool) {
    if run_once {
        let _ = net.clear_run_once();
    }
    if guard {
        let _ = net.stop_guard();
    }
}

/// Step 6's failure path: the restore routine on this plan's entries, so each leaves the journal only on the
/// routine's own rules, never while our value is still stored or in force.
fn undo(
    plans: &mut impl PlanIo,
    net: &mut impl SafetyNet,
    plan: Guid,
    written: Vec<SettingKey>,
    failure: Failure,
) -> Applied {
    let restored = restore(plans, net, Some(plan), None);
    Applied {
        written,
        result: Err(ApplyError::Write(failure)),
        journal: restored.journal,
    }
}

#[cfg(test)]
mod tests;
