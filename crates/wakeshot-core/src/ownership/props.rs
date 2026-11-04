//! The journal property test: no sequence of apply, user change, plan switch, failure or stop, then a restore, drops
//! an entry whose value Wakeshot still holds (test only).
//!
//! Owner: `p2-ownership-props`.
//!
//! Written from architecture sections 5 and 6, not from the routines, as a second check on them. A case is a random
//! start (three plans; maybe a crashed tray's entries or a newer version's entry), up to 40 operations, then one
//! restore with no fault. Each routine call may carry one scripted fault (a failed call or a stop) and one user action
//! between two of its calls. The invariants run after every single call, not only after each operation: the fake's
//! stop at call n leaves the state as it was after call n - 1, so every stop point is covered without sampling it.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Write as _};
use std::panic::{self, AssertUnwindSafe};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use proptest::collection::vec;
use proptest::prelude::*;
use proptest::sample::{select, subsequence};
use proptest::test_runner::{Config as ProptestConfig, TestCaseError};

use crate::command::{ApplyError, Failure};
use crate::files::{JournalFile, JournalFileEntry, JournalKey};
use crate::ownership::apply::{Applied, apply};
use crate::ownership::check::{Checked, check};
use crate::ownership::fake::{
    Action, Call, FakeGuard, FakeJournal, FakePlan, FakePlans, FakeState, SetBy,
};
use crate::ownership::io::{JournalRead, PlanIo, SafetyNet};
use crate::ownership::journal::JournalParts;
use crate::ownership::restore::{Found, Restored, restore};
use crate::process::FileTime;
use crate::settings::{Guid, JournalEntry, JournalOwner, OwnedKey, PlanValues, SettingKey};

const PLANS: [Guid; 3] = [
    Guid(0x381b4222_f694_41f0_9685_ff5bb260df2e),
    Guid(0x8c5e7fda_e8bf_4a96_9a85_a6e23a8c635c),
    Guid(0xa1841308_3541_4fab_bc81_f71556f20b4a),
];
const KEYS: [SettingKey; 3] = [SettingKey::LidAc, SettingKey::LidDc, SettingKey::SleepDc];
const TARGET: u32 = 0;

/// Fault and interleaving points reach past the longest routine (an apply whose failure runs the restore routine,
/// about 30 calls), so some never fire, as a fault scripted for a call the routine does not make.
const CALLS: usize = 32;

/// Most routines make fewer calls than `CALLS`, so most points fall here, or a mutant that needs a fault on one call
/// survives 256 cases (mutants 1 and 3a of the card Result).
const BUSY: usize = 18;

fn at() -> impl Strategy<Value = usize> {
    prop_oneof![4 => 0..BUSY, 1 => BUSY..CALLS]
}

/// The user's values per key, each set holding the target; the usual value first, so a shrunk case starts from it.
/// These cases write only the hold keys at 0 (`KEYS`); the sleep route's 1-minute writes have their own stop-point
/// test in `ownership::apply`.
fn choices(key: SettingKey) -> [u32; 3] {
    match key {
        SettingKey::LidAc | SettingKey::LidDc => [1, TARGET, 2],
        SettingKey::SleepDc | SettingKey::SleepAc => [600, TARGET, 10_800],
        SettingKey::VideoIdleAc | SettingKey::VideoIdleDc => [300, TARGET, 600],
    }
}

fn index(key: SettingKey) -> usize {
    match key {
        SettingKey::LidAc => 0,
        SettingKey::LidDc => 1,
        SettingKey::SleepDc => 2,
        SettingKey::SleepAc => 3,
        SettingKey::VideoIdleAc => 4,
        SettingKey::VideoIdleDc => 5,
    }
}

fn key_name(key: SettingKey) -> &'static str {
    match key {
        SettingKey::LidAc => "lidAc",
        SettingKey::LidDc => "lidDc",
        SettingKey::SleepDc => "sleepDc",
        SettingKey::SleepAc => "sleepAc",
        SettingKey::VideoIdleAc => "videoIdleAc",
        SettingKey::VideoIdleDc => "videoIdleDc",
    }
}

fn plan_name(plan: Guid) -> String {
    match PLANS.iter().position(|&p| p == plan) {
        Some(i) => format!("P{i}"),
        None => plan.to_string(),
    }
}

/// Tray 0 left the start's entries; the case runs in tray 1, and each stop starts the next tray.
fn owner(tray: u32) -> JournalOwner {
    JournalOwner {
        pid: 4000 + tray,
        proc_start: FileTime(1_000 + u64::from(tray)),
        session: 1,
    }
}

/// The owner a guard started for a tray that never owned the file expects.
const STRANGER: JournalOwner = JournalOwner {
    pid: 1,
    proc_start: FileTime(1),
    session: 1,
};

fn tray_name(who: JournalOwner) -> String {
    match who.pid.checked_sub(4000) {
        Some(tray) if who == owner(tray) => format!("tray {tray}"),
        _ => "a stranger".into(),
    }
}

/// An entry a newer version wrote, for a key this version does not know (track note 2).
fn newer_entry() -> JournalFileEntry {
    JournalFileEntry {
        plan: PLANS[0],
        key: JournalKey::Other("hibernateAc".into()),
        original: 10_800,
        written: 0,
        restored: false,
    }
}

// The case.

/// A crashed tray's entry at the start, in each state a stop can leave (section 5, the stop-point paragraph).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Left {
    /// Live equals `written`.
    Ours,
    /// A stop between Apply step 3's save and its write: live equals `original`, not marked.
    Unwritten,
    /// A stop between restore step 4's mark and its write.
    MarkedWritten,
    /// A restore whose `set_active` failed: `original` stored, ours in force on the active plan (E1 of
    /// `p2-ownership-apply`, which the orchestrator asked to keep alone on its plan in some starts).
    MarkedOriginal,
}

#[derive(Clone)]
struct World {
    /// Per plan and key, an index into `choices`. P0 is active.
    values: [[usize; 3]; 3],
    /// At most one per plan and key, never for a key whose value is the target: an apply skips those.
    left: Vec<(usize, SettingKey, Left)>,
    newer: bool,
}

impl World {
    fn value(&self, plan: usize, key: SettingKey) -> u32 {
        choices(key)[self.values[plan][index(key)]]
    }
}

impl fmt::Debug for World {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("start:")?;
        for plan in 0..3 {
            write!(f, " P{plan}{}", if plan == 0 { " (active)" } else { "" })?;
            for key in KEYS {
                write!(f, " {} {}", key_name(key), self.value(plan, key))?;
            }
            f.write_str(";")?;
        }
        for &(plan, key, left) in &self.left {
            let what = match left {
                Left::Ours => "ours",
                Left::Unwritten => "never written",
                Left::MarkedWritten => "marked, ours stored",
                Left::MarkedOriginal => "marked, original stored",
            };
            write!(f, " tray 0 left P{plan} {} {what};", key_name(key))?;
        }
        if self.newer {
            f.write_str(" a newer version's entry;")?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
enum Fault {
    None,
    Fail { at: usize, code: u32 },
    Stop { at: usize },
}

impl fmt::Display for Fault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::None => Ok(()),
            Self::Fail { at, code } => write!(f, ", call {at} fails ({code})"),
            Self::Stop { at } => write!(f, ", stop at call {at}"),
        }
    }
}

/// A user action at the start of call `at`, before the call does anything.
#[derive(Clone, Copy, Debug)]
enum Stir {
    None,
    User {
        at: usize,
        plan: usize,
        key: SettingKey,
        value: usize,
    },
    Switch {
        at: usize,
        plan: usize,
    },
    Delete {
        at: usize,
        plan: usize,
    },
}

impl fmt::Display for Stir {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::None => Ok(()),
            Self::User {
                at,
                plan,
                key,
                value,
            } => write!(
                f,
                ", user sets P{plan} {} = {} at call {at}",
                key_name(key),
                choices(key)[value]
            ),
            Self::Switch { at, plan } => write!(f, ", user switches to P{plan} at call {at}"),
            Self::Delete { at, plan } => write!(f, ", user deletes P{plan} at call {at}"),
        }
    }
}

#[derive(Clone)]
enum Op {
    User {
        plan: usize,
        key: SettingKey,
        value: usize,
    },
    Switch(usize),
    Delete(usize),
    Check(Fault, Stir),
    /// `guard`: run as the guard of the tray before this one, so `expect_owner` names that tray.
    Restore {
        scope: Option<usize>,
        guard: bool,
        fault: Fault,
        stir: Stir,
    },
    /// `to`: `None` is the active plan; a plan index may be stale.
    Apply {
        to: Option<usize>,
        keys: Vec<SettingKey>,
        fault: Fault,
        stir: Stir,
    },
}

fn keys_text(keys: &[SettingKey]) -> String {
    keys.iter()
        .map(|&k| key_name(k))
        .collect::<Vec<_>>()
        .join(" ")
}

impl fmt::Debug for Op {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::User { plan, key, value } => write!(
                f,
                "user sets P{plan} {} = {}",
                key_name(*key),
                choices(*key)[*value]
            ),
            Self::Switch(plan) => write!(f, "user switches to P{plan}"),
            Self::Delete(plan) => write!(f, "user deletes P{plan}"),
            Self::Check(fault, stir) => write!(f, "check{fault}{stir}"),
            Self::Restore {
                scope,
                guard,
                fault,
                stir,
            } => {
                match scope {
                    None => f.write_str("restore all")?,
                    Some(plan) => write!(f, "restore P{plan}")?,
                }
                if *guard {
                    f.write_str(" as the guard of the tray before")?;
                }
                write!(f, "{fault}{stir}")
            }
            Self::Apply {
                to,
                keys,
                fault,
                stir,
            } => {
                match to {
                    None => f.write_str("apply to the active plan")?,
                    Some(plan) => write!(f, "apply to P{plan}")?,
                }
                write!(f, " [{}]{fault}{stir}", keys_text(keys))
            }
        }
    }
}

/// The fault-free restore that ends a case. `Guard`: as a guard retry (restore step 7), first by a guard started for
/// another tray, which must leave the file alone, then by the guard of the file's owner.
#[derive(Clone, Copy, Debug)]
enum Ending {
    Tray,
    Guard,
}

#[derive(Clone)]
struct Case {
    world: World,
    ops: Vec<Op>,
    ending: Ending,
}

impl fmt::Debug for Case {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{:?}", self.world)?;
        for (i, op) in self.ops.iter().enumerate() {
            writeln!(f, "{:>2}. {op:?}", i + 1)?;
        }
        write!(f, "end: {:?}", self.ending)
    }
}

// Strategies.

fn key() -> impl Strategy<Value = SettingKey> {
    select(KEYS.to_vec())
}

fn fault() -> impl Strategy<Value = Fault> {
    prop_oneof![
        4 => Just(Fault::None),
        3 => (at(), select(vec![5_u32, 87, 1168])).prop_map(|(at, code)| Fault::Fail { at, code }),
        2 => at().prop_map(|at| Fault::Stop { at }),
    ]
}

fn stir() -> impl Strategy<Value = Stir> {
    prop_oneof![
        6 => Just(Stir::None),
        1 => (at(), 0..3_usize, key(), 0..3_usize)
            .prop_map(|(at, plan, key, value)| Stir::User { at, plan, key, value }),
        1 => (at(), 0..3_usize).prop_map(|(at, plan)| Stir::Switch { at, plan }),
        1 => (at(), 0..3_usize).prop_map(|(at, plan)| Stir::Delete { at, plan }),
    ]
}

fn op() -> impl Strategy<Value = Op> {
    // The core never sends an empty `writes`, but the routine takes one, so it is kept rare.
    let keys = prop_oneof![
        9 => subsequence(KEYS.to_vec(), 1..=3).prop_shuffle(),
        1 => Just(Vec::new()),
    ];
    prop_oneof![
        3 => (0..3_usize, key(), 0..3_usize).prop_map(|(plan, key, value)| Op::User { plan, key, value }),
        2 => (0..3_usize).prop_map(Op::Switch),
        1 => (0..3_usize).prop_map(Op::Delete),
        3 => (fault(), stir()).prop_map(|(fault, stir)| Op::Check(fault, stir)),
        3 => (
            proptest::option::of(0..3_usize),
            proptest::bool::weighted(0.25),
            fault(),
            stir(),
        )
            .prop_map(|(scope, guard, fault, stir)| Op::Restore { scope, guard, fault, stir }),
        6 => (proptest::option::weighted(0.25, 0..3_usize), keys, fault(), stir())
            .prop_map(|(to, keys, fault, stir)| Op::Apply { to, keys, fault, stir }),
    ]
}

fn world() -> impl Strategy<Value = World> {
    let left = (
        0..3_usize,
        key(),
        select(vec![
            Left::Ours,
            Left::MarkedOriginal,
            Left::MarkedWritten,
            Left::Unwritten,
        ]),
    );
    (
        proptest::array::uniform3(proptest::array::uniform3(0..3_usize)),
        // E1 alone on the active plan, often: beside another entry of its plan, the restore's `set_active` for that
        // one brings E1's `original` into force too and hides a dropped E1 (orchestrator, from `p2-ownership-apply`).
        prop_oneof![
            3 => Just(Vec::new()),
            2 => key().prop_map(|key| vec![(0, key, Left::MarkedOriginal)]),
            3 => vec(left, 1..=3),
        ],
        proptest::bool::weighted(0.2),
    )
        .prop_map(|(values, left, newer)| {
            let mut world = World {
                values,
                left: Vec::new(),
                newer,
            };
            let mut taken = BTreeSet::new();
            for (plan, key, state) in left {
                if world.value(plan, key) != TARGET && taken.insert((plan, key)) {
                    world.left.push((plan, key, state));
                }
            }
            world
        })
}

fn case() -> impl Strategy<Value = Case> {
    (
        world(),
        vec(op(), 0..40),
        prop_oneof![Just(Ending::Tray), Just(Ending::Guard)],
    )
        .prop_map(|(world, ops, ending)| Case { world, ops, ending })
}

/// E1 alone on the active plan, then an apply that takes it over with a fault on one of its calls. The general case
/// reaches this only in about a third of its runs, and a mutant that clears E1's mark on takeover needs exactly it.
fn e1_case() -> impl Strategy<Value = Case> {
    let fault = prop_oneof![
        3 => (0..BUSY, select(vec![5_u32, 87, 1168])).prop_map(|(at, code)| Fault::Fail { at, code }),
        2 => (0..BUSY).prop_map(|at| Fault::Stop { at }),
    ];
    (
        world(),
        key(),
        subsequence(KEYS.to_vec(), 0..=2).prop_shuffle(),
        fault,
        vec(op(), 0..20),
        prop_oneof![Just(Ending::Tray), Just(Ending::Guard)],
    )
        .prop_map(|(mut world, key, others, fault, mut ops, ending)| {
            // The user's value, not the target: an apply skips a key already at it.
            if world.value(0, key) == TARGET {
                world.values[0][index(key)] = 0;
            }
            world.left = vec![(0, key, Left::MarkedOriginal)];
            let mut keys = vec![key];
            keys.extend(others.into_iter().filter(|&k| k != key));
            let first = Op::Apply {
                to: None,
                keys,
                fault,
                stir: Stir::None,
            };
            ops.insert(0, first);
            Case { world, ops, ending }
        })
}

// What a run found.

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Prop {
    NeverDrops,
    PutsBack,
    RunOnce,
    SafetyNetWritesNothing,
    ActivePlanOnly,
    NoFalseUserChange,
    OneEntryPerKey,
    ReportsHeld,
    KeepsUserChange,
}

/// What every call and every user action of one case shares. Behind a mutex because the fake's interleaving
/// closures must be `Send`.
#[derive(Default)]
struct Seen {
    /// The first breach of each property, with the state at that moment.
    broken: BTreeMap<Prop, String>,
    /// Keys the user changed since Wakeshot last wrote them.
    since_ours: BTreeSet<(Guid, SettingKey)>,
    /// Keys the user changed at any time.
    ever: BTreeSet<(Guid, SettingKey)>,
    /// The user's last value per key, or `None` once a later write of the same routine overwrote it: section 5
    /// accepts that for a change between Apply steps 2 and 5 (and the restore has the same window, finding 3).
    last: BTreeMap<(Guid, SettingKey), Option<u32>>,
    /// Keys the user changed inside the routine that runs now.
    inside: BTreeSet<(Guid, SettingKey)>,
    /// Keys whose entry an older version saved unmarked and never wrote, left by the start's crash: the check reads
    /// them as the user's. Until Wakeshot writes the key. Since power safety review finding 9 an apply saves new
    /// entries marked, so no run of this version leaves one.
    unwritten: BTreeSet<(Guid, SettingKey)>,
    step: String,
}

impl Seen {
    fn user(&mut self, plan: Guid, key: SettingKey, value: u32, inside: bool) {
        self.since_ours.insert((plan, key));
        self.ever.insert((plan, key));
        self.last.insert((plan, key), Some(value));
        if inside {
            self.inside.insert((plan, key));
        }
    }

    fn breach(&mut self, prop: Prop, what: impl FnOnce() -> String) {
        if !self.broken.contains_key(&prop) {
            let text = format!("{}: {}", self.step, what());
            self.broken.insert(prop, text);
        }
    }
}

type Shared = Arc<Mutex<Seen>>;

fn lock(seen: &Shared) -> MutexGuard<'_, Seen> {
    seen.lock().unwrap_or_else(PoisonError::into_inner)
}

fn by_text(by: Option<SetBy>) -> &'static str {
    match by {
        Some(SetBy::Override) => " ours",
        Some(SetBy::Restore) => " put back",
        _ => "",
    }
}

fn entries_text(file: &JournalFile) -> String {
    let parts = JournalParts::split(file.clone());
    let mut items: Vec<String> = parts
        .known
        .entries
        .iter()
        .map(|e| {
            format!(
                "{} {} {}->{}{}",
                plan_name(e.plan),
                key_name(e.key),
                e.original,
                e.written,
                if e.restored { " marked" } else { "" }
            )
        })
        .collect();
    items.extend(parts.other.iter().map(|_| "a newer version's entry".into()));
    match items.is_empty() {
        true => "no entries".into(),
        false => items.join(", "),
    }
}

fn call_text(call: &Call) -> String {
    match call {
        Call::ActivePlan => "active".into(),
        Call::Plans => "plans".into(),
        Call::CanWrite(k) => format!("can_write {}", key_name(*k)),
        Call::Read(p, k) => format!("read {} {}", plan_name(*p), key_name(*k)),
        Call::Write(p, k, v) => format!("write {} {} {v}", plan_name(*p), key_name(*k)),
        Call::SetActive(p) => format!("set_active {}", plan_name(*p)),
        Call::Values => "values".into(),
        Call::ReadDefault(p, k) => format!("read default {} {}", plan_name(*p), key_name(*k)),
        Call::LoadJournal => "load".into(),
        Call::SaveJournal(file) => format!("save [{}]", entries_text(file)),
        Call::DeleteJournal => "delete".into(),
        Call::SetAsideJournal => "set aside".into(),
        Call::SetRunOnce => "set RunOnce".into(),
        Call::ClearRunOnce => "clear RunOnce".into(),
        Call::EnsureGuard => "ensure guard".into(),
        Call::StopGuard => "stop guard".into(),
    }
}

fn owned_text(keys: &[OwnedKey]) -> String {
    keys.iter()
        .map(|k| format!("{} {}", plan_name(k.plan), key_name(k.key)))
        .collect::<Vec<_>>()
        .join(", ")
}

fn fail_text(failure: &Failure) -> String {
    format!(
        "{} {}",
        failure.call.trim_start_matches("FakePlans::"),
        failure.code
    )
}

/// The state, and the calls of the routine since call `base`, for a breach message.
fn describe(state: &FakeState, base: usize) -> String {
    let mut out = String::new();
    for plan in PLANS {
        let active = if plan == state.active { "*" } else { " " };
        let _ = write!(out, "\n  {}{active}", plan_name(plan));
        if !state.plans.contains_key(&plan) {
            out.push_str(" deleted");
            continue;
        }
        out.push_str(" stored");
        for key in KEYS {
            let value = state.stored(plan, key).unwrap_or_default();
            let by = by_text(state.stored_by(plan, key));
            let _ = write!(out, " {} {value}{by}", key_name(key));
        }
        out.push_str(" | in force");
        for key in KEYS {
            let value = state.in_force(plan, key).unwrap_or_default();
            let by = by_text(state.in_force_by(plan, key));
            let _ = write!(out, " {} {value}{by}", key_name(key));
        }
    }
    match &state.journal {
        FakeJournal::File(file) => {
            let owner = tray_name(file.owner);
            let _ = write!(out, "\n  journal of {owner}: {}", entries_text(file));
        }
        other => {
            let _ = write!(out, "\n  journal: {other:?}");
        }
    }
    let run_once = if state.run_once { "set" } else { "clear" };
    let _ = write!(out, "\n  RunOnce {run_once}, guard {:?}", state.guard);
    let calls: Vec<String> = state
        .calls
        .get(base..)
        .unwrap_or_default()
        .iter()
        .map(call_text)
        .collect();
    let _ = write!(out, "\n  calls of this routine: {}", calls.join(", "));
    out
}

fn twice(entries: &[JournalEntry]) -> bool {
    let mut keys = BTreeSet::new();
    entries.iter().any(|e| !keys.insert((e.plan, e.key)))
}

// The seams under test.

/// Both seams over one fake, checking the call-level properties after each call.
struct Probe {
    fake: FakePlans,
    seen: Shared,
    /// The first call of the routine, for the messages.
    base: usize,
}

impl Probe {
    fn after<T>(&self, result: T) -> T {
        self.verify();
        result
    }

    fn verify(&self) {
        let state = self.fake.state();
        let lost = state.held_without_entry();
        let entries = state.known_entries();
        let alone = !entries.is_empty() && !state.run_once;
        let twice = twice(&entries);
        if lost.is_empty() && !alone && !twice {
            return;
        }
        let text = describe(&state, self.base);
        drop(state);
        let mut seen = lock(&self.seen);
        if !lost.is_empty() {
            seen.breach(Prop::NeverDrops, || {
                format!("held with no journal entry: {}{text}", owned_text(&lost))
            });
        }
        if alone {
            seen.breach(Prop::RunOnce, || {
                format!("the journal holds a known entry, RunOnce is clear{text}")
            });
        }
        if twice {
            seen.breach(Prop::OneEntryPerKey, || {
                format!("two entries for one plan and key{text}")
            });
        }
    }
}

impl PlanIo for Probe {
    fn active_plan(&mut self) -> Result<Guid, Failure> {
        let result = self.fake.active_plan();
        self.after(result)
    }

    fn plans(&mut self) -> Result<Vec<Guid>, Failure> {
        let result = self.fake.plans();
        self.after(result)
    }

    fn can_write(&mut self, key: SettingKey) -> Result<(), Failure> {
        let result = self.fake.can_write(key);
        self.after(result)
    }

    fn read(&mut self, plan: Guid, key: SettingKey) -> Result<u32, Failure> {
        let result = self.fake.read(plan, key);
        self.after(result)
    }

    fn write(&mut self, plan: Guid, key: SettingKey, value: u32) -> Result<(), Failure> {
        let result = self.fake.write(plan, key, value);
        if result.is_ok() {
            let mut seen = lock(&self.seen);
            seen.since_ours.remove(&(plan, key));
            seen.unwritten.remove(&(plan, key));
            if seen.inside.remove(&(plan, key)) {
                seen.last.insert((plan, key), None);
            }
        }
        self.after(result)
    }

    fn set_active(&mut self, plan: Guid) -> Result<(), Failure> {
        let active = self.fake.state().active;
        if plan != active {
            let text = describe(&self.fake.state(), self.base);
            lock(&self.seen).breach(Prop::ActivePlanOnly, || {
                let (plan, active) = (plan_name(plan), plan_name(active));
                format!("set_active {plan} while {active} is active{text}")
            });
        }
        let result = self.fake.set_active(plan);
        self.after(result)
    }

    fn values(&mut self) -> Result<PlanValues, Failure> {
        let result = self.fake.values();
        self.after(result)
    }

    fn read_default(&mut self, plan: Guid, key: SettingKey) -> Result<u32, Failure> {
        let result = self.fake.read_default(plan, key);
        self.after(result)
    }
}

impl SafetyNet for Probe {
    fn load_journal(&mut self) -> JournalRead {
        let result = self.fake.load_journal();
        self.after(result)
    }

    fn save_journal(&mut self, journal: &JournalFile) -> Result<(), Failure> {
        let result = self.fake.save_journal(journal);
        self.after(result)
    }

    fn delete_journal(&mut self) -> Result<(), Failure> {
        let result = self.fake.delete_journal();
        self.after(result)
    }

    fn set_aside_journal(&mut self) -> Result<(), Failure> {
        let result = self.fake.set_aside_journal();
        self.after(result)
    }

    fn set_run_once(&mut self) -> Result<bool, Failure> {
        let result = self.fake.set_run_once();
        self.after(result)
    }

    fn clear_run_once(&mut self) -> Result<(), Failure> {
        let result = self.fake.clear_run_once();
        self.after(result)
    }

    fn ensure_guard(&mut self) -> Result<bool, Failure> {
        let result = self.fake.ensure_guard();
        self.after(result)
    }

    fn stop_guard(&mut self) -> Result<(), Failure> {
        let result = self.fake.stop_guard();
        self.after(result)
    }
}

/// The user changes a value in Settings; `inside`: while a routine runs.
fn user_write(
    state: &mut FakeState,
    seen: &Shared,
    (plan, key, value): (Guid, SettingKey, u32),
    inside: bool,
) {
    if state.plans.contains_key(&plan) {
        state.user_write(plan, key, value);
        lock(seen).user(plan, key, value, inside);
    }
}

/// Windows refuses to delete the active plan (`PowerDeleteScheme`; not verified), so the world never does.
fn delete(state: &mut FakeState, plan: Guid) {
    if state.active != plan {
        state.delete_plan(plan);
    }
}

type Values = (Guid, BTreeMap<Guid, FakePlan>);

type Change = Box<dyn FnMut(&mut FakeState) + Send>;

fn values(state: &FakeState) -> Values {
    (state.active, state.plans.clone())
}

// One case.

struct Ran<T> {
    /// `None`: a scripted stop ended the routine.
    out: Option<T>,
    base: usize,
    calls: Vec<Call>,
}

struct Run {
    fake: FakePlans,
    seen: Shared,
    tray: u32,
    /// A stop or a failed clean-up call: a RunOnce value or a guard may be left with no journal, which then finds
    /// nothing to do (walk R6 in `p1-design-verify-fixes`).
    loose: bool,
    /// No stop yet, so each user change `check` reports must be real, but for the keys in `Seen::unwritten`.
    sure: bool,
    original: BTreeMap<(Guid, SettingKey), u32>,
    trace: String,
}

impl Run {
    fn new(world: &World) -> Self {
        let values = |p: usize| PlanValues {
            active: PLANS[p],
            lid_ac: world.value(p, SettingKey::LidAc),
            lid_dc: world.value(p, SettingKey::LidDc),
            sleep_ac: 10_800,
            sleep_dc: world.value(p, SettingKey::SleepDc),
            saver_threshold_dc: 20,
        };
        let fake = FakePlans::new(values(0));
        fake.add_plan(values(1));
        fake.add_plan(values(2));
        let mut unwritten = BTreeSet::new();
        {
            let mut state = fake.state();
            let mut entries = Vec::new();
            for &(p, key, left) in &world.left {
                let (plan, original) = (PLANS[p], world.value(p, key));
                let ours = (TARGET, SetBy::Override);
                let (stored, in_force, restored) = match left {
                    Left::Ours => (ours, ours, false),
                    Left::Unwritten => ((original, SetBy::User), (original, SetBy::User), false),
                    Left::MarkedWritten => (ours, ours, true),
                    Left::MarkedOriginal => ((original, SetBy::Restore), ours, true),
                };
                if left == Left::Unwritten {
                    unwritten.insert((plan, key));
                }
                state.set_stored(plan, key, stored.0, stored.1);
                state.set_in_force(plan, key, in_force.0, in_force.1);
                entries.push(JournalFileEntry {
                    plan,
                    key: JournalKey::Known(key),
                    original,
                    written: TARGET,
                    restored,
                });
            }
            if world.newer {
                entries.push(newer_entry());
            }
            if !entries.is_empty() {
                let version = JournalFile::VERSION + u32::from(world.newer);
                state.journal = FakeJournal::File(JournalFile {
                    version,
                    owner: owner(0),
                    entries,
                });
                state.run_once = true;
            }
        }
        let original = (0..3)
            .flat_map(|p| KEYS.map(|k| ((PLANS[p], k), world.value(p, k))))
            .collect();
        let seen = Seen {
            unwritten,
            ..Seen::default()
        };
        Self {
            fake,
            seen: Arc::new(Mutex::new(seen)),
            tray: 1,
            loose: false,
            sure: true,
            original,
            trace: format!("{world:?}\n"),
        }
    }

    fn probe(&self) -> Probe {
        Probe {
            fake: self.fake.clone(),
            seen: Arc::clone(&self.seen),
            base: self.fake.state().calls.len(),
        }
    }

    fn stir(&self, stir: Stir) -> Option<(usize, Action)> {
        let seen = Arc::clone(&self.seen);
        let (at, change): (usize, Change) = match stir {
            Stir::None => return None,
            Stir::User {
                at,
                plan,
                key,
                value,
            } => {
                let (plan, value) = (PLANS[plan], choices(key)[value]);
                (
                    at,
                    Box::new(move |s: &mut FakeState| {
                        user_write(s, &seen, (plan, key, value), true);
                    }),
                )
            }
            Stir::Switch { at, plan } => (
                at,
                Box::new(move |s: &mut FakeState| {
                    // Section 5 reads the id right before `set_active`; a switch between the two is a race no
                    // routine can close, so it is not a case here.
                    if !matches!(s.calls.last(), Some(Call::SetActive(_))) {
                        s.switch_plan(PLANS[plan]);
                    }
                }),
            ),
            Stir::Delete { at, plan } => (
                at,
                Box::new(move |s: &mut FakeState| delete(s, PLANS[plan])),
            ),
        };
        Some((at, Action::Run(change)))
    }

    /// One routine call with its fault and interleaving; a stop is caught as the crash of this tray.
    fn routine<T>(
        &mut self,
        fault: Fault,
        stir: Stir,
        go: impl FnOnce(&mut Probe, &mut Probe) -> T,
    ) -> Ran<T> {
        let (mut plans, mut net) = (self.probe(), self.probe());
        let base = plans.base;
        let fault_at = match fault {
            Fault::None => None,
            Fault::Fail { at, code } => {
                self.fake.script(base + at, Action::Fail(code));
                Some(at)
            }
            Fault::Stop { at } => {
                self.fake.script(base + at, Action::Stop);
                Some(at)
            }
        };
        if let Some((at, action)) = self.stir(stir)
            && fault_at != Some(at)
        {
            self.fake.script(base + at, action);
        }
        let out = panic::catch_unwind(AssertUnwindSafe(|| go(&mut plans, &mut net)));
        let calls = {
            let mut state = self.fake.state();
            // A fault past the routine's last call must not hit the next routine.
            state.script.clear();
            state.calls[base..].to_vec()
        };
        lock(&self.seen).inside.clear();
        if let Fault::Fail { at, .. } = fault
            && matches!(calls.get(at), Some(Call::ClearRunOnce | Call::StopGuard))
        {
            self.loose = true;
        }
        let out = match out {
            Ok(value) => Some(value),
            Err(payload) => {
                let stop = payload
                    .downcast_ref::<String>()
                    .is_some_and(|m| m.starts_with("FakePlans: stopped"));
                if !stop {
                    panic::resume_unwind(payload);
                }
                self.tray += 1;
                self.loose = true;
                self.sure = false;
                None
            }
        };
        Ran { out, base, calls }
    }

    /// The core's mirror is a copy of each routine's `journal`, so a held value missing from it is one the core no
    /// longer knows to restore. A failed journal load reports nothing by contract (`p2-ownership-apply`, Found
    /// outside scope 3), so it is not a breach here.
    fn reported<T>(&self, ran: &Ran<T>, fault: Fault, journal: &[JournalEntry]) {
        if let Fault::Fail { at, .. } = fault
            && ran.calls.get(at) == Some(&Call::LoadJournal)
        {
            return;
        }
        let state = self.fake.state();
        let mut missing = state.held();
        missing.retain(|k| !journal.iter().any(|e| e.plan == k.plan && e.key == k.key));
        if !missing.is_empty() {
            let text = describe(&state, ran.base);
            drop(state);
            lock(&self.seen).breach(Prop::ReportsHeld, || {
                format!(
                    "the reported journal lacks held {}{text}",
                    owned_text(&missing)
                )
            });
        }
    }

    fn stopped<T>(&self, ran: &Ran<T>) -> String {
        let at = ran.calls.len().saturating_sub(1);
        format!(" -> stopped at call {at}; tray {} runs next", self.tray)
    }

    fn step(&mut self, number: usize, op: &Op) {
        lock(&self.seen).step = format!("op {number} ({op:?})");
        let outcome = match op {
            Op::User { plan, key, value } => {
                let change = (PLANS[*plan], *key, choices(*key)[*value]);
                user_write(&mut self.fake.state(), &self.seen, change, false);
                String::new()
            }
            Op::Switch(plan) => {
                self.fake.state().switch_plan(PLANS[*plan]);
                String::new()
            }
            Op::Delete(plan) => {
                delete(&mut self.fake.state(), PLANS[*plan]);
                String::new()
            }
            Op::Check(fault, stir) => self.check(*fault, *stir),
            Op::Restore {
                scope,
                guard,
                fault,
                stir,
            } => self.restore(*scope, *guard, *fault, *stir),
            Op::Apply {
                to,
                keys,
                fault,
                stir,
            } => self.apply(*to, keys, *fault, *stir),
        };
        let _ = writeln!(self.trace, "{number:>2}. {op:?}{outcome}");
        self.probe().verify();
    }

    fn apply(
        &mut self,
        to: Option<usize>,
        keys: &[SettingKey],
        fault: Fault,
        stir: Stir,
    ) -> String {
        let me = owner(self.tray);
        let (plan, before) = {
            let state = self.fake.state();
            (to.map_or(state.active, |p| PLANS[p]), values(&state))
        };
        let writes: Vec<_> = keys.iter().map(|&k| (k, TARGET)).collect();
        let ran = self.routine(fault, stir, |plans, net| {
            apply(plans, net, me, plan, &writes)
        });
        let Some(applied) = &ran.out else {
            return self.stopped(&ran);
        };
        if let Err(ApplyError::SafetyNet(_)) = applied.result {
            let made: Vec<String> = ran
                .calls
                .iter()
                .filter(|c| matches!(c, Call::Write(..) | Call::SetActive(_)))
                .map(call_text)
                .collect();
            let state = self.fake.state();
            // A user action in the routine changes values too; the calls still show what the apply did.
            let changed = matches!(stir, Stir::None) && values(&state) != before;
            if !made.is_empty() || changed {
                let text = describe(&state, ran.base);
                drop(state);
                lock(&self.seen).breach(Prop::SafetyNetWritesNothing, || {
                    format!("reported SafetyNet after {made:?}, values changed: {changed}{text}")
                });
            }
        }
        self.reported(&ran, fault, &applied.journal);
        format!(" -> {}", applied_text(applied))
    }

    fn check(&mut self, fault: Fault, stir: Stir) -> String {
        let sure = self.sure;
        let ran = self.routine(fault, stir, check);
        let Some(checked) = &ran.out else {
            return self.stopped(&ran);
        };
        if sure {
            let seen = lock(&self.seen);
            let wrong: Vec<OwnedKey> = checked
                .user_changed
                .iter()
                .filter(|k| !seen.since_ours.contains(&(k.plan, k.key)))
                .filter(|k| !seen.unwritten.contains(&(k.plan, k.key)))
                .copied()
                .collect();
            drop(seen);
            if !wrong.is_empty() {
                let text = describe(&self.fake.state(), ran.base);
                lock(&self.seen).breach(Prop::NoFalseUserChange, || {
                    let wrong = owned_text(&wrong);
                    format!("reported {wrong} as the user's, unchanged since Wakeshot's last write{text}")
                });
            }
        }
        self.reported(&ran, fault, &checked.journal);
        format!(" -> {}", checked_text(checked))
    }

    fn restore(&mut self, scope: Option<usize>, guard: bool, fault: Fault, stir: Stir) -> String {
        let scope = scope.map(|p| PLANS[p]);
        let expect = guard.then(|| owner(self.tray - 1));
        let ran = self.routine(fault, stir, |plans, net| restore(plans, net, scope, expect));
        match &ran.out {
            None => self.stopped(&ran),
            Some(restored) => {
                self.reported(&ran, fault, &restored.journal);
                format!(" -> {}", restored_text(restored))
            }
        }
    }

    /// A guard started for a tray that never owned the file finds it taken over and leaves it (restore step 7).
    fn stranger_guard(&mut self, owner_now: JournalOwner) {
        let all = |s: &FakeState| (values(s), s.journal.clone(), s.run_once, s.guard);
        let before = all(&self.fake.state());
        let ran = self.routine(Fault::None, Stir::None, |plans, net| {
            restore(plans, net, None, Some(STRANGER))
        });
        let state = self.fake.state();
        let found = ran.out.as_ref().map(|r| (r.found, r.result));
        let left_alone = found == Some((Found::OtherOwner(owner_now), Ok(())))
            && ran.calls == [Call::LoadJournal]
            && all(&state) == before;
        if !left_alone {
            let text = describe(&state, ran.base);
            drop(state);
            lock(&self.seen).breach(Prop::PutsBack, || {
                format!("a guard for another tray did not leave the file alone: {found:?}{text}")
            });
        }
    }

    fn end(&mut self, ending: Ending, newer: bool) {
        lock(&self.seen).step = format!("the end ({ending:?})");
        let owner_now = match &self.fake.state().journal {
            FakeJournal::File(file) => Some(file.owner),
            _ => None,
        };
        let expect = match (ending, owner_now) {
            (Ending::Guard, Some(owner_now)) => {
                self.stranger_guard(owner_now);
                Some(owner_now)
            }
            _ => None,
        };
        let ran = self.routine(Fault::None, Stir::None, |plans, net| {
            restore(plans, net, None, expect)
        });
        let outcome = ran.out.as_ref().map(restored_text).unwrap_or_default();
        let _ = writeln!(self.trace, "end: {ending:?} -> {outcome}");

        let mut wrong = Vec::new();
        if !ran.out.as_ref().is_some_and(|r| r.result.is_ok()) {
            wrong.push(format!("the restore did not succeed ({outcome})"));
        }
        let state = self.fake.state();
        let held = state.held();
        if !held.is_empty() {
            wrong.push(format!("still held: {}", owned_text(&held)));
        }
        let ever = lock(&self.seen).ever.clone();
        for (&(plan, key), &original) in &self.original {
            if ever.contains(&(plan, key)) {
                continue;
            }
            let at = format!("{} {}", plan_name(plan), key_name(key));
            if let Some(stored) = state.stored(plan, key)
                && stored != original
            {
                wrong.push(format!("{at} stored {stored}, its original is {original}"));
            }
            if plan == state.active
                && let Some(in_force) = state.in_force(plan, key)
                && in_force != original
            {
                wrong.push(format!(
                    "{at} in force {in_force}, its original is {original}"
                ));
            }
        }
        let journal_gone = match &state.journal {
            FakeJournal::Missing => !newer,
            FakeJournal::File(file) => {
                let parts = JournalParts::split(file.clone());
                newer && parts.known.entries.is_empty() && parts.other == [newer_entry()]
            }
            FakeJournal::Corrupt | FakeJournal::Unreadable(_) => false,
        };
        if !journal_gone {
            wrong
                .push("the journal is not gone (with a newer version's entry: not only it)".into());
        }
        // A file at the end means this restore ran the clean-up, so nothing may be left over.
        if owner_now.is_some() || !self.loose {
            if state.run_once {
                wrong.push("RunOnce is set".into());
            }
            if state.guard == FakeGuard::Running {
                wrong.push("the guard's stop event is not set".into());
            }
        }
        if !wrong.is_empty() {
            let text = describe(&state, ran.base);
            lock(&self.seen).breach(Prop::PutsBack, || format!("{}{text}", wrong.join("; ")));
        }

        // Section 5 accepts that a user who sets exactly our value is not seen, so the restore puts back the
        // original over it.
        let last = lock(&self.seen).last.clone();
        let mut undone = Vec::new();
        for (&(plan, key), &value) in &last {
            let Some(value) = value.filter(|&v| v != TARGET) else {
                continue;
            };
            let at = format!("{} {}", plan_name(plan), key_name(key));
            if let Some(stored) = state.stored(plan, key)
                && stored != value
            {
                undone.push(format!(
                    "{at} stored {stored}, the user's last value is {value}"
                ));
            }
            if plan == state.active
                && let Some(in_force) = state.in_force(plan, key)
                && in_force != value
            {
                undone.push(format!(
                    "{at} in force {in_force}, the user's last value is {value}"
                ));
            }
        }
        if !undone.is_empty() {
            let text = describe(&state, ran.base);
            lock(&self.seen).breach(Prop::KeepsUserChange, || {
                format!("{}{text}", undone.join("; "))
            });
        }
    }
}

fn applied_text(applied: &Applied) -> String {
    let result = match &applied.result {
        Ok(()) => "Ok".into(),
        Err(ApplyError::PlanChanged) => "PlanChanged".into(),
        Err(ApplyError::SafetyNet(f)) => format!("SafetyNet({})", fail_text(f)),
        Err(ApplyError::Write(f)) => format!("Write({})", fail_text(f)),
    };
    let written = keys_text(&applied.written);
    format!(
        "{result}, wrote [{written}], {} entries",
        applied.journal.len()
    )
}

fn checked_text(checked: &Checked) -> String {
    let failures: Vec<String> = checked.failures.iter().map(fail_text).collect();
    format!(
        "user changed [{}], failures [{}], {} entries",
        owned_text(&checked.user_changed),
        failures.join(", "),
        checked.journal.len()
    )
}

fn restored_text(restored: &Restored) -> String {
    let found = match restored.found {
        Found::OtherOwner(owner) => format!("OtherOwner({})", tray_name(owner)),
        found => format!("{found:?}"),
    };
    let result = match &restored.result {
        Ok(()) => "Ok".into(),
        Err(f) => format!("Err({})", fail_text(f)),
    };
    format!("{found}, {result}, {} entries", restored.journal.len())
}

struct Report {
    broken: BTreeMap<Prop, String>,
    trace: String,
}

impl Report {
    fn holds_all(&self) -> Result<(), TestCaseError> {
        match self.broken.keys().next() {
            None => Ok(()),
            Some(&prop) => self.holds(prop),
        }
    }

    fn holds(&self, prop: Prop) -> Result<(), TestCaseError> {
        match self.broken.get(&prop) {
            None => Ok(()),
            Some(text) => Err(TestCaseError::fail(format!(
                "{prop:?} broken at {text}\n\nthe case as it ran:\n{}",
                self.trace
            ))),
        }
    }
}

fn run(case: &Case) -> Report {
    let mut run = Run::new(&case.world);
    lock(&run.seen).step = "the start".into();
    run.probe().verify();
    for (i, op) in case.ops.iter().enumerate() {
        run.step(i + 1, op);
    }
    run.end(case.ending, case.world.newer);
    let broken = std::mem::take(&mut lock(&run.seen).broken);
    Report {
        broken,
        trace: run.trace,
    }
}

proptest! {
    // As in the other property tests: a regression file would be a file outside this card, so a shrunk case that
    // finds a bug becomes a named test.
    #![proptest_config(ProptestConfig { failure_persistence: None, ..ProptestConfig::default() })]

    /// This plan's acceptance check: after every call, every key whose value Wakeshot still holds on a plan that
    /// exists (stored by our override, or in force on the active plan) has a known entry in the journal file.
    #[test]
    fn never_drops_a_held_value(case in case()) {
        run(&case).holds(Prop::NeverDrops)?;
    }

    /// After the fault-free ending no value is held anywhere, every key the user did not change is back at its
    /// original, stored and in force, the journal file is gone, the RunOnce value is clear and the guard's stop
    /// event is set; a guard started for another tray leaves the file alone.
    #[test]
    fn the_final_restore_puts_everything_back(case in case()) {
        run(&case).holds(Prop::PutsBack)?;
    }

    /// After every call, a journal with a known entry has its RunOnce value (the clean-up order).
    #[test]
    fn a_known_entry_never_lives_without_run_once(case in case()) {
        run(&case).holds(Prop::RunOnce)?;
    }

    /// An apply that reports `SafetyNet` made no write and no `set_active`, and changed no stored or in-force value.
    #[test]
    fn safety_net_writes_nothing(case in case()) {
        run(&case).holds(Prop::SafetyNetWritesNothing)?;
    }

    /// Every `set_active` names the plan active at that moment (power review 6).
    #[test]
    fn set_active_only_on_the_active_plan(case in case()) {
        run(&case).holds(Prop::ActivePlanOnly)?;
    }

    /// Until a case's first stop, `check` reports a key as the user's only if the user changed it since Wakeshot's
    /// last write, or its entry was saved and never written (`Seen::unwritten`).
    #[test]
    fn no_false_user_change_without_a_stop(case in case()) {
        run(&case).holds(Prop::NoFalseUserChange)?;
    }

    /// After every call, the journal holds at most one known entry per plan and key, so ownership has one answer.
    #[test]
    fn one_entry_per_plan_and_key(case in case()) {
        run(&case).holds(Prop::OneEntryPerKey)?;
    }

    /// Each routine's reported `journal`, which the core's mirror copies, holds the entry of every value Wakeshot
    /// still holds, but after a failed journal load.
    #[test]
    fn the_report_never_drops_a_held_value(case in case()) {
        run(&case).holds(Prop::ReportsHeld)?;
    }

    /// After the ending, a key whose last user change no write of the same routine overwrote, to anything but the
    /// target, still has that value, stored and in force: no routine puts back a value older than the user's latest.
    #[test]
    fn a_users_change_is_never_undone(case in case()) {
        run(&case).holds(Prop::KeepsUserChange)?;
    }

    /// Every property holds when E1 is the only entry on the active plan and an apply takes it over under a fault:
    /// the start the orchestrator asked every stop-point test to keep (`p2-ownership-apply`, Found outside scope 8).
    #[test]
    fn e1_alone_survives_a_takeover_with_any_fault(case in e1_case()) {
        run(&case).holds_all()?;
    }
}

/// Finding 1 of `p2-ownership-props` (power safety review finding 9), the strict form of the user-change property: a
/// failed apply whose restore routine fails too must not leave an entry it never wrote for the check to report as the
/// user's. The apply saves new entries marked until its call, so the check reads lidDc as "marked, `original` stored".
#[test]
fn a_failed_apply_whose_restore_fails_reports_no_false_user_change() {
    let fake = FakePlans::new(PlanValues {
        active: PLANS[0],
        lid_ac: 1,
        lid_dc: 1,
        sleep_ac: 10_800,
        sleep_dc: 600,
        saver_threshold_dc: 20,
    });
    // The read-back of lidAc finds another value (the user, or a policy), so the apply runs the restore routine,
    // and that routine's journal delete fails, after the call for lidDc's mark.
    let (read_back, delete) = (10, 18);
    let user = |s: &mut FakeState| s.user_write(PLANS[0], SettingKey::LidAc, 2);
    fake.script(read_back, Action::Run(Box::new(user)));
    fake.script(delete, Action::Fail(5));
    let writes = [(SettingKey::LidAc, TARGET), (SettingKey::LidDc, TARGET)];
    let applied = apply(
        &mut fake.clone(),
        &mut fake.clone(),
        owner(1),
        PLANS[0],
        &writes,
    );
    {
        let state = fake.state();
        assert_eq!(
            state.calls[read_back],
            Call::Read(PLANS[0], SettingKey::LidAc)
        );
        assert_eq!(
            state.calls[delete],
            Call::DeleteJournal,
            "the apply's call order changed"
        );
    }
    assert!(matches!(applied.result, Err(ApplyError::Write(_))));
    let checked = check(&mut fake.clone(), &mut fake.clone());
    let lid_ac = OwnedKey {
        plan: PLANS[0],
        key: SettingKey::LidAc,
    };
    assert_eq!(checked.user_changed, [lid_ac], "lidDc was never written");
}
