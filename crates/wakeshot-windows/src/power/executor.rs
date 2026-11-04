//! Runs `ReadPlan`, `Apply` and `Restore` on the IO thread, and the restore at tray startup (architecture section 4,
//! "Tray threads" and "Tray startup" step 6).
//!
//! Owner: `p3-power-io`. The signatures are contract: the IO thread (`p6-int-io-thread`) calls `run`, the tray's
//! startup (`p6-int-startup`) calls `startup`, and the harness (`p3-harness`) drives both. The ownership routines are
//! called only through `power::journal`'s locked entry points.

use std::sync::Arc;

use wakeshot_core::command::{Command, Failure, Outcome, RestoreWhy};
use wakeshot_core::event::Event;
use wakeshot_core::files::JournalKey;
use wakeshot_core::ownership::io::{JournalRead, PlanIo, SafetyNet};
use wakeshot_core::ownership::journal::JournalParts;
use wakeshot_core::ownership::restore::{Found, Restored};
use wakeshot_core::process::{Alive, ProcessRef};
use wakeshot_core::settings::{Guid, Journal, JournalEntry, JournalOwner, SettingKey};
use windows::Win32::Foundation::ERROR_BUSY;

use crate::log::{Level, Log};
use crate::power::journal::{
    FileNet, NoGuard, apply_locked, check_locked, mark_corrupt_seen, reset_locked, restore_locked,
};
use crate::power::plan::{DryRunPower, Win32Power};
use crate::process::Liveness;
use crate::store::{DataDir, Names};

const AREA: &str = "power";

/// A `Restore { why: Leftover }` not run: the journal's owner is alive, or another process took the journal over
/// between the owner check and the lock. 170 is `ERROR_BUSY` ("the requested resource is in use"), our choice.
pub const OWNER_ALIVE: Failure = Failure {
    call: "journal owner (alive)",
    code: ERROR_BUSY.0,
};

/// A `Restore { why: Leftover }` not run: the owner's liveness is Unknown, which never starts a restore (architecture
/// section 5, "Liveness").
pub const OWNER_UNKNOWN: Failure = Failure {
    call: "journal owner (liveness unknown)",
    code: ERROR_BUSY.0,
};

/// What the startup restore leaves for `Event::Started` and the events right after it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Startup {
    /// `Event::Started`'s `journal`: the known entries the journal holds after the startup restore, with its owner;
    /// `None` when there are none. The core retries what is left as `Restore { why: Leftover }`. `None` also when
    /// `journal.json` did not read at all (logged): nothing can be known then.
    pub journal: Option<Journal>,
    /// `Event::Started`'s `journal_unread`: `journal.json` did not read (`JournalRead::Failed`), so nothing was
    /// restored and what it holds is not known. The core then retries it as `Restore { why: Leftover }` on each tick,
    /// whose run reads the file again, checks its owner and restores. `false` for a missing, corrupt or read file.
    pub unread: bool,
    /// A corrupt journal was found: set aside by this restore, or left as `journal.json.corrupt` by restore mode, the
    /// guard or a hook. It is renamed `journal.json.corrupt.seen` already, so post `Event::JournalCorrupt` once, after
    /// `Started` (and after `Session(Locked)` when that comes).
    pub corrupt: bool,
}

/// Runs the IO thread's power commands through the locked ownership routines and turns each into the events the core
/// expects (architecture section 3). One thread owns it at a time: the tray builds it, runs `startup` on its main
/// thread, then moves it to the IO thread.
///
/// Callers may rely on:
///
/// - `run` answers `ReadPlan`, `Apply`, `Restore` and `ResetLost`; every other command gives no event and makes no
///   call;
/// - before each of those three, a journal that holds a known entry this tray owns gets a guard (`ensure_guard`), so a
///   guard that ended is replaced (architecture section 5, "Guard");
/// - `Restore { why: Leftover }` runs only when the journal's owner is Gone or this tray; otherwise it answers a failed
///   `Restored` (`OWNER_ALIVE`, `OWNER_UNKNOWN`) with the file's known entries, so the core retries on the tick;
/// - a restore that sets a corrupt journal aside answers `Done(Restored)`, then `JournalCorrupt`, and the file is
///   renamed `journal.json.corrupt.seen` (track note 7).
pub struct PowerExecutor<P, N> {
    plans: P,
    net: N,
    me: JournalOwner,
    names: Names,
    dir: DataDir,
    liveness: Arc<dyn Liveness>,
    log: Arc<dyn Log>,
}

// The tray builds the executor for the startup restore and then moves it to the IO thread, so the forms it ships must
// be `Send`. `FileNet<TrayGuard>` is `p3-recovery`'s to keep `Send`.
const _: () = {
    const fn is_send<T: Send>() {}
    is_send::<PowerExecutor<Win32Power, FileNet<NoGuard>>>();
    is_send::<PowerExecutor<DryRunPower<Win32Power>, FileNet<NoGuard>>>();
};

impl<P: PlanIo, N: SafetyNet> PowerExecutor<P, N> {
    /// `me`: this tray, which every apply writes as the journal's owner (pid, start time and session). `names`: the
    /// mutex names of this process, real or dry run. `dir`: the folder of `net`'s journal, where a set-aside journal is
    /// marked seen.
    pub fn new(
        plans: P,
        net: N,
        me: JournalOwner,
        names: Names,
        dir: DataDir,
        liveness: Arc<dyn Liveness>,
        log: Arc<dyn Log>,
    ) -> Self {
        Self {
            plans,
            net,
            me,
            names,
            dir,
            liveness,
            log,
        }
    }

    pub fn plans(&self) -> &P {
        &self.plans
    }

    pub fn net(&self) -> &N {
        &self.net
    }

    /// One command from the IO queue, and the events to post for it, in order.
    ///
    /// - `ReadPlan`: `Plan` with the active plan's values (`Err` when that read fails), then one `UserChanged` per entry
    ///   the ownership check dropped. An entry whose read fails is kept and logged, and `Plan` is still `Ok` (track
    ///   note 10).
    /// - `Apply`: `Done(Applied)`.
    /// - `Restore`: `Done(Restored)`, then `JournalCorrupt` when this run set a corrupt journal aside.
    /// - `ResetLost`: `Done(ResetLost)`.
    pub fn run(&mut self, command: &Command) -> Vec<Event> {
        match command {
            Command::ReadPlan => {
                self.keep_guard("plan read");
                self.read_plan()
            }
            Command::Apply { plan, writes } => {
                self.keep_guard("apply");
                vec![self.apply(*plan, writes)]
            }
            Command::Restore { plan, why } => {
                let before = self.keep_guard("restore");
                self.restore(*plan, *why, &before)
            }
            // No guard: a lost journal names no owner a guard could wait on.
            Command::ResetLost { damaged } => vec![self.reset(*damaged)],
            _ => Vec::new(),
        }
    }

    /// A file the reset set aside is marked seen at once: its own notice tells of it, so the next start must not
    /// report it as a damaged journal again (which would reset again, then over a value the user set since).
    fn reset(&mut self, damaged: bool) -> Event {
        let done = reset_locked(
            &mut self.plans,
            &mut self.net,
            damaged,
            &self.names,
            self.log.as_ref(),
        );
        if done.set_aside == Some(Ok(())) {
            self.mark_seen();
        }
        Event::Done(Outcome::ResetLost {
            reset: done.reset_keys(),
            result: done.result,
            journal: done.journal,
        })
    }

    /// Tray startup step 6, on the tray's main thread before any other thread starts. A journal whose owner is Gone is
    /// restored at once; one whose owner is alive or Unknown, or one that does not read (`Startup::unread`), is left
    /// for the core's `Leftover` retry on the tick. A corrupt `journal.json` is set aside, as any restore would: no
    /// owner can be read from it, and setting it aside writes no setting. A `journal.json.corrupt` left by restore
    /// mode, the guard or a hook is reported once.
    pub fn startup(&mut self) -> Startup {
        let read = self.net.load_journal();
        let unread = matches!(read, JournalRead::Failed(_));
        let (journal, set_aside) = match read {
            JournalRead::Missing => (None, false),
            JournalRead::Failed(f) => {
                let text = format!(
                    "startup: journal.json does not read: {}; nothing restored at start, the core retries it on \
                     each tick",
                    error_text(f)
                );
                self.note(Level::Warn, &text);
                (None, false)
            }
            JournalRead::Corrupt => {
                let done = self.restore_now(None, None);
                (None, was_set_aside(&done))
            }
            JournalRead::Read(file) => self.leftover_at_start(JournalParts::split(file).known),
        };
        let left_by_others = self.mark_seen();
        Startup {
            journal: journal.filter(|j| !j.entries.is_empty()),
            corrupt: set_aside || left_by_others,
            unread,
        }
    }

    fn leftover_at_start(&mut self, known: Journal) -> (Option<Journal>, bool) {
        let owner = known.owner;
        let how = match self.liveness.check(process_of(owner)) {
            Alive::Gone => None,
            Alive::Yes => Some("alive"),
            Alive::Unknown => Some("of unknown liveness"),
        };
        if let Some(how) = how {
            let text = format!(
                "startup: journal of pid {} start {} left for the retry on the tick: its owner is {how}",
                owner.pid, owner.proc_start
            );
            self.note(Level::Info, &text);
            return (Some(known), false);
        }
        let done = self.restore_now(None, Some(owner));
        let set_aside = was_set_aside(&done);
        let journal = match done.found {
            // The routine could not read the file, so it changed nothing and the entries read above are still there.
            // `Started` carries no result, so an empty journal would read as "nothing owned" and never be retried.
            Found::Nothing if done.result.is_err() => known,
            Found::OtherOwner(other) => Journal {
                owner: other,
                entries: done.journal,
            },
            _ => Journal {
                owner,
                entries: done.journal,
            },
        };
        (Some(journal), set_aside)
    }

    /// A guard waits on this tray and restores only a journal that names this tray as owner (`recovery`), so a
    /// leftover of another owner gets none: it would never act. A failed start is logged and the command still runs:
    /// RunOnce covers the next sign-in, and the next command tries again. Returns the journal as read, for the
    /// `Leftover` check.
    fn keep_guard(&mut self, what: &str) -> JournalRead {
        let read = self.net.load_journal();
        let owned = match &read {
            JournalRead::Read(file) => {
                file.owner == self.me
                    && file
                        .entries
                        .iter()
                        .any(|e| matches!(e.key, JournalKey::Known(_)))
            }
            _ => false,
        };
        if owned && let Err(f) = self.net.ensure_guard() {
            let text = format!(
                "guard not started before the {what}: {}; it runs without one",
                error_text(f)
            );
            self.note(Level::Warn, &text);
        }
        read
    }

    /// The values are read before the check, so a user change the values show was also there for the check, and its
    /// `UserChanged` follows. The check runs also when the values do not read: an entry it drops must reach the core,
    /// or the next apply overrides the user's change.
    fn read_plan(&mut self) -> Vec<Event> {
        let values = self.plans.values();
        if let Err(f) = values {
            self.note(Level::Warn, &format!("plan read failed: {}", error_text(f)));
        }
        let checked = check_locked(
            &mut self.plans,
            &mut self.net,
            &self.names,
            self.log.as_ref(),
        );
        let mut events = vec![Event::Plan(values)];
        events.extend(checked.user_changed.into_iter().map(Event::UserChanged));
        events
    }

    fn apply(&mut self, plan: Guid, writes: &[(SettingKey, u32)]) -> Event {
        let done = apply_locked(
            &mut self.plans,
            &mut self.net,
            self.me,
            plan,
            writes,
            &self.names,
            self.log.as_ref(),
        );
        Event::Done(Outcome::Applied {
            plan,
            written: done.written,
            result: done.result,
            journal: done.journal,
        })
    }

    fn restore(&mut self, plan: Option<Guid>, why: RestoreWhy, before: &JournalRead) -> Vec<Event> {
        let expect_owner = match why {
            RestoreWhy::Leftover => match self.leftover(before) {
                Leftover::Run(owner) => owner,
                Leftover::Answer(result, journal) => {
                    return vec![restored(plan, why, result, journal)];
                }
            },
            _ => None,
        };
        let done = self.restore_now(plan, expect_owner);
        let set_aside = was_set_aside(&done);
        let result = match done.found {
            // Another process wrote the journal between the owner check and the lock: alive a moment ago.
            Found::OtherOwner(_) => Err(OWNER_ALIVE),
            _ => done.result,
        };
        let mut events = vec![restored(plan, why, result, done.journal)];
        if set_aside {
            events.push(Event::JournalCorrupt);
            self.mark_seen();
        }
        events
    }

    /// The owner goes to the routine as `expect_owner`, so a journal another process took over between this read and
    /// the lock is left alone.
    fn leftover(&self, before: &JournalRead) -> Leftover {
        let file = match before {
            // What the routine would answer, without taking the lock: a journal written in between belongs to an owner
            // nobody checked.
            JournalRead::Missing => return Leftover::Answer(Ok(()), Vec::new()),
            // No owner can be read; the routine only sets the file aside, which writes no setting.
            JournalRead::Corrupt => return Leftover::Run(None),
            JournalRead::Failed(f) => {
                let text = format!(
                    "leftover restore not run: journal.json does not read: {}",
                    error_text(*f)
                );
                self.note(Level::Warn, &text);
                return Leftover::Answer(Err(*f), Vec::new());
            }
            JournalRead::Read(file) => file,
        };
        let owner = file.owner;
        if owner == self.me {
            return Leftover::Run(Some(owner));
        }
        let (failure, level, how) = match self.liveness.check(process_of(owner)) {
            Alive::Gone => return Leftover::Run(Some(owner)),
            Alive::Yes => (OWNER_ALIVE, Level::Info, "alive"),
            Alive::Unknown => (OWNER_UNKNOWN, Level::Warn, "of unknown liveness"),
        };
        let text = format!(
            "leftover restore not run: the journal's owner (pid {} start {}) is {how}; retried on the tick",
            owner.pid, owner.proc_start
        );
        self.note(level, &text);
        let known = JournalParts::split(file.clone()).known.entries;
        Leftover::Answer(Err(failure), known)
    }

    fn restore_now(&mut self, plan: Option<Guid>, expect_owner: Option<JournalOwner>) -> Restored {
        restore_locked(
            &mut self.plans,
            &mut self.net,
            plan,
            expect_owner,
            &self.names,
            self.log.as_ref(),
        )
    }

    /// The notice shows once per corrupt file, so the file is renamed once it is reported (track note 7). `true`: a
    /// `journal.json.corrupt` was there. A rename that fails counts as there and is logged: the next start reports the
    /// same file again, a repeated notice rather than none.
    fn mark_seen(&self) -> bool {
        match mark_corrupt_seen(&self.dir) {
            Ok(found) => found,
            Err(e) => {
                let text = format!(
                    "journal.json.corrupt not renamed to .seen: {} failed with {}; the next start reports it again",
                    e.call, e.code
                );
                self.note(Level::Warn, &text);
                true
            }
        }
    }

    fn note(&self, level: Level, text: &str) {
        self.log.line(level, AREA, text);
    }
}

/// What a `Restore { why: Leftover }` does after the owner check.
enum Leftover {
    /// Run the routine with this `expect_owner`.
    Run(Option<JournalOwner>),
    /// Answer this without running it.
    Answer(Result<(), Failure>, Vec<JournalEntry>),
}

/// `Found::SetAside` with `Err` means the move failed and the file is still in place, so no notice yet.
fn was_set_aside(done: &Restored) -> bool {
    done.found == Found::SetAside && done.result.is_ok()
}

fn restored(
    plan: Option<Guid>,
    why: RestoreWhy,
    result: Result<(), Failure>,
    journal: Vec<JournalEntry>,
) -> Event {
    Event::Done(Outcome::Restored {
        plan,
        why,
        result,
        journal,
    })
}

fn process_of(owner: JournalOwner) -> ProcessRef {
    ProcessRef {
        pid: owner.pid,
        start: owner.proc_start,
    }
}

fn error_text(f: Failure) -> String {
    format!("{} failed with {}", f.call, f.code)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::thread;

    use wakeshot_core::command::ApplyError;
    use wakeshot_core::files::{JournalFile, JournalFileEntry};
    use wakeshot_core::ownership::apply::JOURNAL_CORRUPT;
    use wakeshot_core::ownership::fake::{
        Action, Call, FakeGuard, FakeJournal, FakePlans, FakeState, SetBy,
    };
    use wakeshot_core::process::FileTime;
    use wakeshot_core::settings::{OwnedKey, PlanValues};
    use wakeshot_core::view::Notice;

    use super::*;
    use crate::log::MemoryLog;
    use crate::process::FakeLiveness;
    use crate::store::{DataFile, NamedMutex};

    const BALANCED: Guid = Guid(0x381b4222_f694_41f0_9685_ff5bb260df2e);
    const OTHER: Guid = Guid(0xa000_0000_0000_0000_0000_0000_0000_0001);
    const ME: JournalOwner = JournalOwner {
        pid: 4242,
        proc_start: FileTime(134_381_300_000_000_000),
        session: 1,
    };
    const DEAD: JournalOwner = JournalOwner {
        pid: 9120,
        proc_start: FileTime(134_381_211_391_959_336),
        session: 1,
    };
    const KEYS: [SettingKey; 3] = [SettingKey::LidAc, SettingKey::LidDc, SettingKey::SleepDc];
    const ALL: [(Guid, SettingKey); 3] = [
        (BALANCED, SettingKey::LidAc),
        (BALANCED, SettingKey::LidDc),
        (BALANCED, SettingKey::SleepDc),
    ];

    /// The user's own values before any hold. Test values only, not read from any laptop.
    fn user_values(plan: Guid) -> PlanValues {
        PlanValues {
            active: plan,
            lid_ac: 1,
            lid_dc: 1,
            sleep_ac: 10_800,
            sleep_dc: 600,
            saver_threshold_dc: 30,
        }
    }

    fn user_value(key: SettingKey) -> u32 {
        match key {
            SettingKey::LidAc | SettingKey::LidDc => 1,
            SettingKey::SleepDc => 600,
            SettingKey::SleepAc => 10_800,
            SettingKey::VideoIdleAc => wakeshot_core::ownership::fake::FAKE_SCREEN_OFF[0],
            SettingKey::VideoIdleDc => wakeshot_core::ownership::fake::FAKE_SCREEN_OFF[1],
        }
    }

    /// A fresh folder under `%TEMP%`, removed with everything in it when the test ends.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(tag: &str) -> Self {
            static N: AtomicU64 = AtomicU64::new(0);
            let n = N.fetch_add(1, Ordering::Relaxed);
            let dir = std::env::temp_dir().join(format!(
                "wakeshot-power-io-{tag}-{}-{n}",
                std::process::id()
            ));
            fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }

        fn data(&self) -> DataDir {
            DataDir::at(self.0.clone())
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn names_in(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    /// The fakes, a temp folder and test mutex names, so each test builds executors over one shared state.
    struct Rig {
        fake: FakePlans,
        liveness: Arc<FakeLiveness>,
        log: Arc<MemoryLog>,
        names: Names,
        dir: TempDir,
    }

    impl Rig {
        fn new(tag: &str) -> Self {
            Self {
                fake: FakePlans::new(user_values(BALANCED)),
                liveness: Arc::default(),
                log: Arc::default(),
                names: Names::test(&format!("power-io-{tag}")),
                dir: TempDir::new(tag),
            }
        }

        fn executor(&self) -> PowerExecutor<FakePlans, FakePlans> {
            self.executor_over(self.fake.clone())
        }

        fn executor_over<N: SafetyNet>(&self, net: N) -> PowerExecutor<FakePlans, N> {
            PowerExecutor::new(
                self.fake.clone(),
                net,
                ME,
                self.names.clone(),
                self.dir.data(),
                self.liveness.clone(),
                self.log.clone(),
            )
        }

        fn calls(&self) -> Vec<Call> {
            self.fake.state().calls.clone()
        }

        /// Scripts the call `offset` calls from now.
        fn script_next(&self, offset: usize, action: Action) -> usize {
            let index = self.fake.state().calls.len() + offset;
            self.fake.script(index, action);
            index
        }

        fn set_alive(&self, owner: JournalOwner, alive: Alive) {
            self.liveness
                .alive
                .lock()
                .unwrap()
                .push((process_of(owner), alive));
        }

        fn warned(&self, part: &str) -> bool {
            self.log
                .lines
                .lock()
                .unwrap()
                .iter()
                .any(|(level, _, text)| *level == Level::Warn && text.contains(part))
        }
    }

    fn known(plan: Guid, key: SettingKey, restored: bool) -> JournalEntry {
        JournalEntry {
            plan,
            key,
            original: user_value(key),
            written: 0,
            restored,
        }
    }

    fn file_entry(e: JournalEntry) -> JournalFileEntry {
        JournalFileEntry {
            plan: e.plan,
            key: JournalKey::Known(e.key),
            original: e.original,
            written: e.written,
            restored: e.restored,
        }
    }

    /// An entry a newer version wrote (track note 5).
    fn unknown_entry() -> JournalFileEntry {
        JournalFileEntry {
            plan: BALANCED,
            key: JournalKey::Other("hibernateAc".into()),
            original: 10_800,
            written: 0,
            restored: false,
        }
    }

    fn journal_file(owner: JournalOwner, entries: Vec<JournalFileEntry>) -> JournalFile {
        JournalFile {
            version: JournalFile::VERSION,
            owner,
            entries,
        }
    }

    /// Our values (0) stored and in force for `keys`, a journal of `owner` naming each (plus `extra`), and RunOnce set:
    /// what an apply by `owner` left.
    fn overridden(
        fake: &FakePlans,
        owner: JournalOwner,
        keys: &[(Guid, SettingKey)],
        extra: Vec<JournalFileEntry>,
    ) {
        let mut s = fake.state();
        for &(plan, key) in keys {
            s.set_stored(plan, key, 0, SetBy::Override);
            s.set_in_force(plan, key, 0, SetBy::Override);
        }
        let mut entries: Vec<JournalFileEntry> = keys
            .iter()
            .map(|&(plan, key)| file_entry(known(plan, key, false)))
            .collect();
        entries.extend(extra);
        s.journal = FakeJournal::File(journal_file(owner, entries));
        s.run_once = true;
    }

    fn all_three() -> Command {
        Command::Apply {
            plan: BALANCED,
            writes: KEYS.iter().map(|&k| (k, 0)).collect(),
        }
    }

    fn restore(why: RestoreWhy) -> Command {
        Command::Restore { plan: None, why }
    }

    fn user_changed(plan: Guid, key: SettingKey) -> Event {
        Event::UserChanged(OwnedKey { plan, key })
    }

    #[test]
    fn read_plan_events_in_order() {
        let rig = Rig::new("read-plan");
        overridden(&rig.fake, ME, &ALL, vec![]);
        rig.fake.state().guard = FakeGuard::Running;
        {
            let mut s = rig.fake.state();
            s.user_write(BALANCED, SettingKey::LidDc, 2);
            s.user_write(BALANCED, SettingKey::SleepDc, 900);
        }
        let mut executor = rig.executor();

        let events = executor.run(&Command::ReadPlan);

        let values = PlanValues {
            lid_ac: 0,
            lid_dc: 2,
            sleep_dc: 900,
            ..user_values(BALANCED)
        };
        assert_eq!(
            events,
            [
                Event::Plan(Ok(values)),
                user_changed(BALANCED, SettingKey::LidDc),
                user_changed(BALANCED, SettingKey::SleepDc),
            ]
        );
        let calls = rig.calls();
        let read_values = calls.iter().position(|c| *c == Call::Values).unwrap();
        let check_load = calls.iter().rposition(|c| *c == Call::LoadJournal).unwrap();
        assert!(read_values < check_load, "{calls:?}");
        // `UserChanged` comes after the entry left the journal (`Event` doc).
        assert_eq!(
            rig.fake.state().known_entries(),
            [known(BALANCED, SettingKey::LidAc, false)]
        );

        // Nothing changed since: only the values.
        assert_eq!(executor.run(&Command::ReadPlan), [Event::Plan(Ok(values))]);
    }

    #[test]
    fn outcomes_carry_the_journal() {
        let rig = Rig::new("outcomes");
        let mut executor = rig.executor();

        let events = executor.run(&all_three());
        let journal = rig.fake.state().known_entries();
        assert_eq!(journal.len(), 3);
        assert_eq!(
            events,
            [Event::Done(Outcome::Applied {
                plan: BALANCED,
                written: KEYS.to_vec(),
                result: Ok(()),
                journal,
            })]
        );

        // Restore calls: load (the guard check), ensure_guard, then the routine: load, active plan, plan list, three
        // reads, the marks' save, three writes, active plan, set_active.
        let at = rig.script_next(13, Action::Fail(31));
        let events = executor.run(&restore(RestoreWhy::WorkEnded));
        assert_eq!(rig.calls()[at], Call::SetActive(BALANCED));
        let journal = rig.fake.state().known_entries();
        assert_eq!(journal.len(), 3);
        assert!(journal.iter().all(|e| e.restored));
        assert_eq!(
            events,
            [restored(
                None,
                RestoreWhy::WorkEnded,
                Err(Failure {
                    call: "FakePlans::set_active",
                    code: 31,
                }),
                journal,
            )]
        );

        assert_eq!(
            executor.run(&restore(RestoreWhy::WorkEnded)),
            [restored(None, RestoreWhy::WorkEnded, Ok(()), Vec::new())]
        );
        let s = rig.fake.state();
        assert_eq!(s.journal, FakeJournal::Missing);
        assert!(s.held().is_empty());
        assert!(!s.run_once);
    }

    #[test]
    fn failed_plan_read_is_plan_err() {
        let rig = Rig::new("plan-err");
        // Calls: load (the guard check), values, load (the check).
        let at = rig.script_next(1, Action::Fail(5));
        let events = rig.executor().run(&Command::ReadPlan);
        assert_eq!(rig.calls()[at], Call::Values);
        let failed = Failure {
            call: "FakePlans::values",
            code: 5,
        };
        assert_eq!(events, [Event::Plan(Err(failed))]);
        assert!(rig.warned("plan read failed: FakePlans::values failed with 5"));

        // The check still runs, and a change it finds still reaches the core.
        let rig = Rig::new("plan-err-check");
        overridden(&rig.fake, ME, &ALL, vec![]);
        rig.fake.state().user_write(BALANCED, SettingKey::LidAc, 3);
        // Calls: load, ensure_guard, values.
        rig.script_next(2, Action::Fail(5));
        assert_eq!(
            rig.executor().run(&Command::ReadPlan),
            [
                Event::Plan(Err(failed)),
                user_changed(BALANCED, SettingKey::LidAc)
            ]
        );
    }

    #[test]
    fn failed_entry_read_keeps_the_entry_and_plan_is_ok() {
        let rig = Rig::new("entry-err");
        overridden(&rig.fake, ME, &[(BALANCED, SettingKey::LidDc)], vec![]);
        // Calls: load, ensure_guard, values, load, active plan, read of the entry.
        let at = rig.script_next(5, Action::Fail(5));
        let events = rig.executor().run(&Command::ReadPlan);
        assert_eq!(rig.calls()[at], Call::Read(BALANCED, SettingKey::LidDc));
        let values = PlanValues {
            lid_dc: 0,
            ..user_values(BALANCED)
        };
        assert_eq!(events, [Event::Plan(Ok(values))]);
        assert_eq!(
            rig.fake.state().known_entries(),
            [known(BALANCED, SettingKey::LidDc, false)]
        );
        assert!(rig.warned("FakePlans::read failed with 5"));
    }

    #[test]
    fn corrupt_journal_gives_event_once() {
        let rig = Rig::new("corrupt");
        let journal_path = rig.dir.data().file(DataFile::Journal);
        fs::write(&journal_path, b"{ not a journal").unwrap();
        // The dry-run net renames real files and touches no registry key.
        let mut executor = rig.executor_over(FileNet::dry_run(rig.dir.data(), rig.log.clone()));

        // Only a restore sets the file aside: a plan read and an apply leave it.
        let values = Ok(user_values(BALANCED));
        assert_eq!(executor.run(&Command::ReadPlan), [Event::Plan(values)]);
        let events = executor.run(&all_three());
        assert!(matches!(
            &events[..],
            [Event::Done(Outcome::Applied {
                result: Err(ApplyError::SafetyNet(JOURNAL_CORRUPT)),
                ..
            })]
        ));
        assert_eq!(names_in(&rig.dir.0), ["journal.json"]);

        let ended = restore(RestoreWhy::WorkEnded);
        let done = restored(None, RestoreWhy::WorkEnded, Ok(()), Vec::new());
        assert_eq!(executor.run(&ended), [done.clone(), Event::JournalCorrupt]);
        assert_eq!(names_in(&rig.dir.0), ["journal.json.corrupt.seen"]);
        assert_eq!(executor.run(&ended), [done]);

        // A new corrupt file is a new notice, also for a leftover restore, which has no owner to check then.
        fs::write(&journal_path, b"[1, 2").unwrap();
        let leftover = restore(RestoreWhy::Leftover);
        let done = restored(None, RestoreWhy::Leftover, Ok(()), Vec::new());
        assert_eq!(
            executor.run(&leftover),
            [done.clone(), Event::JournalCorrupt]
        );
        assert_eq!(names_in(&rig.dir.0), ["journal.json.corrupt.seen"]);
        assert_eq!(
            fs::read(rig.dir.data().file(DataFile::JournalCorruptSeen)).unwrap(),
            b"[1, 2"
        );
        assert_eq!(executor.run(&leftover), [done]);
        assert!(rig.liveness.checked.lock().unwrap().is_empty());

        // A move that fails leaves the file in place: a failed restore, and no notice yet.
        let rig = Rig::new("corrupt-stuck");
        rig.fake.state().journal = FakeJournal::Corrupt;
        // Calls: load (the guard check), the routine's load, the set-aside.
        let at = rig.script_next(2, Action::Fail(32));
        let failed = Failure {
            call: "FakePlans::set_aside_journal",
            code: 32,
        };
        assert_eq!(
            rig.executor().run(&ended),
            [restored(
                None,
                RestoreWhy::WorkEnded,
                Err(failed),
                Vec::new()
            )]
        );
        assert_eq!(rig.calls()[at], Call::SetAsideJournal);
        assert_eq!(rig.fake.state().journal, FakeJournal::Corrupt);
    }

    #[test]
    fn a_lost_journal_reset_answers_its_keys_and_marks_the_file_seen() {
        use wakeshot_core::ownership::fake::{FAKE_DEFAULT_LID, FAKE_DEFAULT_SLEEP_DC};

        let rig = Rig::new("reset");
        let ours = |key: SettingKey| {
            let mut state = rig.fake.state();
            state.set_stored(BALANCED, key, 0, SetBy::Override);
            state.set_in_force(BALANCED, key, 0, SetBy::Override);
        };
        let answer = |reset: Vec<SettingKey>| {
            Event::Done(Outcome::ResetLost {
                reset,
                result: Ok(()),
                journal: Vec::new(),
            })
        };
        // A crashed tray's battery lid action, and a damaged journal that names it.
        ours(SettingKey::LidDc);
        fs::write(rig.dir.data().file(DataFile::Journal), b"{ not a journal").unwrap();
        // The dry-run net renames real files and touches no registry key.
        let mut executor = rig.executor_over(FileNet::dry_run(rig.dir.data(), rig.log.clone()));
        let lost = Command::ResetLost { damaged: false };
        assert_eq!(executor.run(&lost), [answer(vec![SettingKey::LidDc])]);
        let state = rig.fake.state();
        assert_eq!(
            state.in_force(BALANCED, SettingKey::LidDc),
            Some(FAKE_DEFAULT_LID)
        );
        assert!(state.held().is_empty());
        drop(state);
        // Set aside and marked seen, so the next start reports no damaged journal and resets nothing again.
        assert_eq!(names_in(&rig.dir.0), ["journal.json.corrupt.seen"]);
        assert!(rig.warned("journal.json moved to journal.json.corrupt"));

        // The file is gone now: a journal that did not read was restored elsewhere, so nothing is reset.
        ours(SettingKey::SleepDc);
        assert_eq!(executor.run(&lost), [answer(Vec::new())]);
        assert_eq!(
            rig.fake.state().in_force(BALANCED, SettingKey::SleepDc),
            Some(0)
        );
        // After a damaged one was set aside, its values are still in force: reset.
        let damaged = Command::ResetLost { damaged: true };
        assert_eq!(executor.run(&damaged), [answer(vec![SettingKey::SleepDc])]);
        assert_eq!(
            rig.fake.state().in_force(BALANCED, SettingKey::SleepDc),
            Some(FAKE_DEFAULT_SLEEP_DC)
        );
        assert!(!rig.calls().contains(&Call::EnsureGuard));
    }

    #[test]
    fn leftover_runs_only_for_gone_or_own_owner() {
        let leftover = restore(RestoreWhy::Leftover);
        for (owner, alive, failure) in [
            (DEAD, Some(Alive::Gone), None),
            (DEAD, Some(Alive::Yes), Some(OWNER_ALIVE)),
            (DEAD, Some(Alive::Unknown), Some(OWNER_UNKNOWN)),
            (ME, None, None),
        ] {
            let case = format!("{owner:?} {alive:?}");
            let rig = Rig::new("leftover");
            overridden(&rig.fake, owner, &ALL, vec![]);
            if let Some(alive) = alive {
                rig.set_alive(owner, alive);
            }
            let entries = rig.fake.state().known_entries();

            let events = rig.executor().run(&leftover);

            let s = rig.fake.state();
            match failure {
                None => {
                    assert_eq!(
                        events,
                        [restored(None, RestoreWhy::Leftover, Ok(()), Vec::new())],
                        "{case}"
                    );
                    assert!(s.held().is_empty(), "{case}");
                    assert_eq!(s.journal, FakeJournal::Missing, "{case}");
                }
                Some(failure) => {
                    assert_eq!(
                        events,
                        [restored(
                            None,
                            RestoreWhy::Leftover,
                            Err(failure),
                            entries.clone()
                        )],
                        "{case}"
                    );
                    assert_eq!(s.calls, [Call::LoadJournal], "{case}");
                    assert_eq!(s.known_entries(), entries, "{case}");
                    assert_eq!(s.held().len(), 3, "{case}");
                }
            }
            let checked = rig.liveness.checked.lock().unwrap().clone();
            let asked = match owner == ME {
                true => vec![],
                false => vec![process_of(owner)],
            };
            assert_eq!(checked, asked, "{case}");
        }

        // A journal another tray took over between the owner check and the lock is left alone.
        let rig = Rig::new("leftover-taken");
        overridden(&rig.fake, DEAD, &ALL, vec![]);
        let taker = JournalOwner { pid: 777, ..ME };
        // Calls: load (the owner check), then the routine's load.
        rig.script_next(
            1,
            Action::Run(Box::new(move |s: &mut FakeState| {
                if let FakeJournal::File(file) = &mut s.journal {
                    file.owner = taker;
                }
            })),
        );
        let entries = rig.fake.state().known_entries();
        assert_eq!(
            rig.executor().run(&leftover),
            [restored(
                None,
                RestoreWhy::Leftover,
                Err(OWNER_ALIVE),
                entries
            )]
        );
        assert_eq!(rig.calls(), [Call::LoadJournal, Call::LoadJournal]);
        assert_eq!(rig.fake.state().held().len(), 3);

        // No journal: answered without the routine, so no lock and no second read.
        let rig = Rig::new("leftover-missing");
        assert_eq!(
            rig.executor().run(&leftover),
            [restored(None, RestoreWhy::Leftover, Ok(()), Vec::new())]
        );
        assert_eq!(rig.calls(), [Call::LoadJournal]);

        // Any other reason runs whatever the owner: the core sends it for a restore it wants now.
        let rig = Rig::new("leftover-other-why");
        overridden(&rig.fake, DEAD, &ALL, vec![]);
        rig.set_alive(DEAD, Alive::Yes);
        assert_eq!(
            rig.executor().run(&restore(RestoreWhy::WorkEnded)),
            [restored(None, RestoreWhy::WorkEnded, Ok(()), Vec::new())]
        );
        assert!(rig.fake.state().held().is_empty());
        assert!(rig.liveness.checked.lock().unwrap().is_empty());
    }

    #[test]
    fn ensures_guard_while_owned() {
        let commands = [
            Command::ReadPlan,
            all_three(),
            restore(RestoreWhy::WorkEnded),
            restore(RestoreWhy::Leftover),
        ];
        for command in &commands {
            for (guard, started) in [
                (FakeGuard::None, 1),
                (FakeGuard::Stopping, 1),
                (FakeGuard::Running, 0),
            ] {
                let case = format!("{command:?} {guard:?}");
                let rig = Rig::new("guard");
                overridden(&rig.fake, ME, &ALL, vec![]);
                rig.fake.state().guard = guard;
                rig.executor().run(command);
                let s = rig.fake.state();
                assert_eq!(
                    s.calls[..2],
                    [Call::LoadJournal, Call::EnsureGuard],
                    "{case}"
                );
                assert_eq!(s.guards_started, started, "{case}");
            }
        }

        // A failed start is logged, and the command still runs.
        let rig = Rig::new("guard-fails");
        overridden(&rig.fake, ME, &ALL, vec![]);
        rig.script_next(1, Action::Fail(1450));
        let values = PlanValues {
            lid_ac: 0,
            lid_dc: 0,
            sleep_dc: 0,
            ..user_values(BALANCED)
        };
        assert_eq!(
            rig.executor().run(&Command::ReadPlan),
            [Event::Plan(Ok(values))]
        );
        assert!(rig.warned("guard not started before the plan read"));
        assert_eq!(rig.fake.state().guard, FakeGuard::None);
    }

    #[test]
    fn no_guard_when_nothing_owned() {
        type Start = (&'static str, fn(&FakePlans));
        let starts: [Start; 4] = [
            ("no journal", |_| {}),
            ("corrupt journal", |fake| {
                fake.state().journal = FakeJournal::Corrupt;
            }),
            ("only unknown entries", |fake| {
                fake.state().journal = FakeJournal::File(journal_file(ME, vec![unknown_entry()]));
            }),
            ("another owner's leftover", |fake| {
                overridden(fake, DEAD, &ALL, vec![]);
            }),
        ];
        let commands = [
            Command::ReadPlan,
            Command::Restore {
                plan: Some(OTHER),
                why: RestoreWhy::PlanSwitch,
            },
        ];
        for (name, start) in starts {
            for command in &commands {
                let rig = Rig::new("no-guard");
                start(&rig.fake);
                rig.set_alive(DEAD, Alive::Unknown);
                rig.executor().run(command);
                let s = rig.fake.state();
                assert!(
                    !s.calls.contains(&Call::EnsureGuard),
                    "{name} {command:?}: {:?}",
                    s.calls
                );
                assert_eq!(s.guards_started, 0, "{name} {command:?}");
            }
        }

        // A fresh apply starts its guard in its own safety net step, not before it.
        let rig = Rig::new("no-guard-apply");
        rig.executor().run(&all_three());
        let calls = rig.calls();
        assert_eq!(calls[..2], [Call::LoadJournal, Call::LoadJournal]);
        assert_eq!(calls.iter().filter(|c| **c == Call::EnsureGuard).count(), 1);
    }

    #[test]
    fn other_commands_give_no_events() {
        let rig = Rig::new("other");
        overridden(&rig.fake, ME, &ALL, vec![]);
        let mut executor = rig.executor();
        for command in [
            Command::Requests {
                system: true,
                display: false,
            },
            Command::Retake,
            Command::Lock,
            Command::Notify(Notice::RestoreFailed),
            Command::Log("hold started".into()),
            Command::OpenLog,
            Command::Quit,
        ] {
            assert!(executor.run(&command).is_empty(), "{command:?}");
        }
        assert!(rig.calls().is_empty());
    }

    #[test]
    fn startup_restores_gone_owner() {
        let rig = Rig::new("start-gone");
        overridden(&rig.fake, DEAD, &ALL, vec![]);

        let startup = rig.executor().startup();

        assert_eq!(
            startup,
            Startup {
                journal: None,
                corrupt: false,
                unread: false,
            }
        );
        let s = rig.fake.state();
        assert!(s.held().is_empty());
        for key in KEYS {
            assert_eq!(s.stored(BALANCED, key), Some(user_value(key)));
            assert_eq!(s.in_force(BALANCED, key), Some(user_value(key)));
        }
        assert_eq!(s.journal, FakeJournal::Missing);
        assert!(!s.run_once);
        assert_eq!(s.guards_started, 0);
        assert_eq!(*rig.liveness.checked.lock().unwrap(), [process_of(DEAD)]);
    }

    fn assert_left_alone(alive: Alive) {
        let rig = Rig::new("start-left");
        overridden(&rig.fake, DEAD, &ALL, vec![]);
        rig.set_alive(DEAD, alive);
        let entries = rig.fake.state().known_entries();

        let startup = rig.executor().startup();

        assert_eq!(
            startup,
            Startup {
                journal: Some(Journal {
                    owner: DEAD,
                    entries
                }),
                corrupt: false,
                unread: false,
            }
        );
        let s = rig.fake.state();
        assert_eq!(s.calls, [Call::LoadJournal]);
        assert_eq!(s.held().len(), 3);
        assert!(s.run_once);
    }

    #[test]
    fn startup_leaves_unknown_owner() {
        assert_left_alone(Alive::Unknown);
    }

    #[test]
    fn startup_leaves_live_owner() {
        assert_left_alone(Alive::Yes);
    }

    /// Power safety review finding 3: a journal that does not read is left as it is and said so, for the core's
    /// `Leftover` retry; every other answer of the read is not `unread`.
    #[test]
    fn startup_says_when_the_journal_does_not_read() {
        let rig = Rig::new("start-not-read");
        overridden(&rig.fake, DEAD, &ALL, vec![]);
        rig.script_next(0, Action::Fail(32));

        let startup = rig.executor().startup();

        assert_eq!(
            startup,
            Startup {
                journal: None,
                corrupt: false,
                unread: true,
            }
        );
        let s = rig.fake.state();
        assert_eq!(s.calls, [Call::LoadJournal]);
        assert_eq!(s.held().len(), 3);
        assert!(s.run_once);
        drop(s);
        assert!(rig.warned("startup: journal.json does not read: "));

        // The first `Leftover` retry reads it again and restores, its owner being Gone.
        let mut executor = rig.executor();
        let events = executor.run(&Command::Restore {
            plan: None,
            why: RestoreWhy::Leftover,
        });
        assert!(
            matches!(
                events.as_slice(),
                [Event::Done(Outcome::Restored { result: Ok(()), journal, .. })] if journal.is_empty()
            ),
            "{events:?}"
        );
        assert!(rig.fake.state().held().is_empty());

        for journal in [
            FakeJournal::Missing,
            FakeJournal::Corrupt,
            FakeJournal::File(journal_file(DEAD, vec![])),
        ] {
            let rig = Rig::new("start-read");
            rig.fake.state().journal = journal.clone();
            assert!(!rig.executor().startup().unread, "{journal:?}");
        }
    }

    #[test]
    fn startup_reports_corrupt_left_by_restore_mode() {
        let rig = Rig::new("start-corrupt-left");
        let corrupt = rig.dir.data().file(DataFile::JournalCorrupt);
        fs::write(&corrupt, b"{ set aside by restore mode").unwrap();
        let none_found = Startup {
            journal: None,
            corrupt: false,
            unread: false,
        };
        let found = Startup {
            journal: None,
            corrupt: true,
            unread: false,
        };

        assert_eq!(rig.executor().startup(), found);
        assert_eq!(names_in(&rig.dir.0), ["journal.json.corrupt.seen"]);
        assert_eq!(rig.executor().startup(), none_found);

        // A corrupt journal.json found at start is set aside and reported the same way.
        rig.fake.state().journal = FakeJournal::Corrupt;
        assert_eq!(rig.executor().startup(), found);
        assert_eq!(rig.fake.state().set_aside, 1);
        assert_eq!(rig.executor().startup(), none_found);

        // The same with real files: set aside, then marked seen, in one start.
        let rig = Rig::new("start-corrupt-file");
        fs::write(rig.dir.data().file(DataFile::Journal), b"not json").unwrap();
        let mut executor = rig.executor_over(FileNet::dry_run(rig.dir.data(), rig.log.clone()));
        assert_eq!(executor.startup(), found);
        assert_eq!(names_in(&rig.dir.0), ["journal.json.corrupt.seen"]);
        assert_eq!(executor.startup(), none_found);
    }

    #[test]
    fn startup_journal_holds_known_entries_only() {
        // An owner alive: the known entries, never the newer version's.
        let rig = Rig::new("start-known");
        overridden(&rig.fake, DEAD, &ALL, vec![unknown_entry()]);
        rig.set_alive(DEAD, Alive::Yes);
        let startup = rig.executor().startup();
        let entries: Vec<JournalEntry> = ALL
            .iter()
            .map(|&(plan, key)| known(plan, key, false))
            .collect();
        assert_eq!(
            startup.journal,
            Some(Journal {
                owner: DEAD,
                entries
            })
        );

        // Only a newer version's entries: none to report, also when the owner is alive.
        let rig = Rig::new("start-unknown-only");
        rig.fake.state().journal = FakeJournal::File(journal_file(DEAD, vec![unknown_entry()]));
        rig.set_alive(DEAD, Alive::Yes);
        assert_eq!(rig.executor().startup().journal, None);

        // A failed restore: what it left, known entries only; the newer version's entry stays in the file.
        let rig = Rig::new("start-failed");
        overridden(&rig.fake, DEAD, &ALL, vec![unknown_entry()]);
        // Calls: load, the routine's load, active plan, plan list, three reads, the marks' save, three writes.
        let at = rig.script_next(9, Action::Fail(5));
        let startup = rig.executor().startup();
        assert_eq!(rig.calls()[at], Call::Write(BALANCED, SettingKey::LidDc, 1));
        let left = vec![known(BALANCED, SettingKey::LidDc, true)];
        assert_eq!(rig.fake.state().known_entries(), left);
        assert_eq!(
            startup.journal,
            Some(Journal {
                owner: DEAD,
                entries: left
            })
        );
        let s = rig.fake.state();
        let FakeJournal::File(file) = &s.journal else {
            panic!("the journal is gone: {:?}", s.journal);
        };
        assert!(file.entries.contains(&unknown_entry()));
        drop(s);

        // The routine could not read the file: it changed nothing, so the entries read first are reported.
        let rig = Rig::new("start-unread");
        overridden(&rig.fake, DEAD, &ALL, vec![]);
        let entries = rig.fake.state().known_entries();
        rig.script_next(1, Action::Fail(32));
        assert_eq!(
            rig.executor().startup().journal,
            Some(Journal {
                owner: DEAD,
                entries
            })
        );
        assert_eq!(rig.fake.state().held().len(), 3);

        // Another tray took the journal over between the first read and the lock: left alone, reported as its.
        let rig = Rig::new("start-taken");
        overridden(&rig.fake, DEAD, &ALL, vec![]);
        let entries = rig.fake.state().known_entries();
        let taker = JournalOwner { pid: 777, ..ME };
        rig.script_next(
            1,
            Action::Run(Box::new(move |s: &mut FakeState| {
                if let FakeJournal::File(file) = &mut s.journal {
                    file.owner = taker;
                }
            })),
        );
        assert_eq!(
            rig.executor().startup().journal,
            Some(Journal {
                owner: taker,
                entries
            })
        );
        assert_eq!(rig.calls(), [Call::LoadJournal, Call::LoadJournal]);
        assert_eq!(rig.fake.state().held().len(), 3);
    }

    /// A starting state for the stop walk: what a dead tray can leave.
    struct WalkStart {
        name: &'static str,
        set: fn(&FakePlans),
        has_unknown: bool,
    }

    const WALK_STARTS: [WalkStart; 3] = [
        WalkStart {
            name: "three entries and a newer version's",
            set: |fake| overridden(fake, DEAD, &ALL, vec![unknown_entry()]),
            has_unknown: true,
        },
        WalkStart {
            // A restore that stopped after writing the originals, before its `set_active`: stored back, ours in force.
            name: "marked, original stored, ours in force",
            set: |fake| {
                let mut s = fake.state();
                let mut entries = Vec::new();
                for key in KEYS {
                    s.set_stored(BALANCED, key, user_value(key), SetBy::Restore);
                    s.set_in_force(BALANCED, key, 0, SetBy::Override);
                    entries.push(file_entry(known(BALANCED, key, true)));
                }
                s.journal = FakeJournal::File(journal_file(DEAD, entries));
                s.run_once = true;
            },
            has_unknown: false,
        },
        WalkStart {
            // One entry alone on the active plan, so a dropped one cannot hide behind another's `set_active`
            // (`p2-ownership-apply` Found outside scope 8); one more on an inactive plan.
            name: "one entry on the active plan, one on another",
            set: |fake| {
                fake.add_plan(user_values(OTHER));
                overridden(
                    fake,
                    DEAD,
                    &[(BALANCED, SettingKey::LidDc), (OTHER, SettingKey::SleepDc)],
                    vec![],
                );
            },
            has_unknown: false,
        },
    ];

    fn walk_rig(start: &WalkStart) -> Rig {
        let rig = Rig::new("walk");
        (start.set)(&rig.fake);
        rig
    }

    /// Track note 4, first point: every value we wrote that is still stored or in force has an entry with the user's
    /// original, and RunOnce is set while a known entry exists. A startup never starts a guard.
    fn assert_reachable(rig: &Rig, start: &WalkStart, case: &str) {
        let s = rig.fake.state();
        assert!(
            s.held_without_entry().is_empty(),
            "{case}: held without an entry: {:?}",
            s.held_without_entry()
        );
        for e in s.known_entries() {
            assert_eq!(e.original, user_value(e.key), "{case}: {e:?}");
        }
        if !s.known_entries().is_empty() {
            assert!(s.run_once, "{case}: entries left without RunOnce");
        }
        assert_eq!(s.guards_started, 0, "{case}: a guard was started");
        if start.has_unknown {
            let FakeJournal::File(file) = &s.journal else {
                panic!("{case}: the newer version's entry is gone: {:?}", s.journal);
            };
            assert!(file.entries.contains(&unknown_entry()), "{case}");
        }
    }

    /// Track note 4, second point: after a restore that worked, every key equals the user's value, stored on every plan
    /// and in force on the active one; no known entry is left; RunOnce is gone unless a stop came between the journal's
    /// delete and its clear (walk R6: the next start finds no journal, harmless).
    fn assert_restored(rig: &Rig, start: &WalkStart, startup: &Startup, case: &str) {
        assert_eq!(
            *startup,
            Startup {
                journal: None,
                corrupt: false,
                unread: false,
            },
            "{case}"
        );
        assert_reachable(rig, start, case);
        let s = rig.fake.state();
        assert!(s.held().is_empty(), "{case}: still held: {:?}", s.held());
        assert!(s.known_entries().is_empty(), "{case}");
        for (&plan, values) in &s.plans {
            for key in KEYS {
                assert_eq!(
                    s.stored(plan, key),
                    Some(user_value(key)),
                    "{case}: {plan} {key:?}"
                );
                if plan == s.active {
                    assert_eq!(
                        s.in_force(plan, key),
                        Some(user_value(key)),
                        "{case}: {plan} {key:?} {values:?}"
                    );
                }
            }
        }
        if s.run_once {
            assert_eq!(s.journal, FakeJournal::Missing, "{case}");
            assert!(s.calls.contains(&Call::DeleteJournal), "{case}");
        }
    }

    /// A fresh walk state, stopped at call `k` of a startup.
    fn stopped_at(start: &WalkStart, k: usize) -> Rig {
        let rig = walk_rig(start);
        rig.fake.script(k, Action::Stop);
        let mut executor = rig.executor();
        let ran = catch_unwind(AssertUnwindSafe(|| executor.startup()));
        assert!(ran.is_err(), "{}: no stop at call {k}", start.name);
        rig
    }

    #[test]
    fn startup_stop_points() {
        for start in &WALK_STARTS {
            let calls = {
                let rig = walk_rig(start);
                rig.executor().startup();
                rig.calls()
            };
            assert!(calls.contains(&Call::SetActive(BALANCED)), "{}", start.name);

            for (k, call) in calls.iter().enumerate() {
                let case = format!("{}: stop at call {k} ({call:?})", start.name);
                let rig = stopped_at(start, k);
                assert_reachable(&rig, start, &case);
                // The stop unwound through the journal lock and released it: another thread takes it at once.
                let names = rig.names.clone();
                let free = thread::spawn(move || {
                    NamedMutex::open(&names.journal)
                        .and_then(|m| m.take(0))
                        .map(|held| held.is_some())
                })
                .join()
                .unwrap();
                assert_eq!(free, Ok(true), "{case}");

                let base = rig.calls().len();
                let next = rig.executor().startup();
                assert_restored(&rig, start, &next, &format!("{case}, then a start"));
                let next_calls = rig.calls().len() - base;

                // The next start fails at each of its calls in turn: the first point still holds, its journal is what
                // the file keeps, and the start after it finishes.
                for j in 0..next_calls {
                    let case = format!("{case}, then a start failing at its call {j}");
                    let rig = stopped_at(start, k);
                    rig.script_next(j, Action::Fail(31));
                    let next = rig.executor().startup();
                    assert_reachable(&rig, start, &case);
                    let left = rig.fake.state().known_entries();
                    match (j, next.journal) {
                        // The first read failed: nothing is known, which the log says.
                        (0, journal) => {
                            assert_eq!(journal, None, "{case}");
                            assert!(rig.warned("startup: journal.json does not read"), "{case}");
                        }
                        (_, None) => assert!(left.is_empty(), "{case}: {left:?}"),
                        (_, Some(journal)) => {
                            assert_eq!(journal.owner, DEAD, "{case}");
                            assert_eq!(journal.entries, left, "{case}");
                        }
                    }
                    let last = rig.executor().startup();
                    assert_restored(&rig, start, &last, &format!("{case}, then a start"));
                }
            }
        }
    }
}
