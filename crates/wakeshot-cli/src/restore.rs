//! `wakeshot restore`: ask the tray to restore, or restore here when its owner is dead.
//!
//! Owner: sources track (`p4-cli-restore`); fixes by `p6-int-dry-run-e2e`.
//!
//! The paths of architecture section 4 ("How they talk"): no journal, nothing to do; the journal's owner (pid and
//! start, not `state.json`) runs, so the tray restores through `restore-request.json`; the owner is gone, so this
//! process runs the restore routine itself, through `restore_locked` like every other caller. An owner whose
//! liveness is unknown starts nothing (section 5, "Liveness").

use std::path::Path;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use wakeshot_core::files::{JournalFile, JournalKey};
use wakeshot_core::ownership::io::{JournalRead, PlanIo, SafetyNet};
use wakeshot_core::ownership::restore::{Found, Restored};
use wakeshot_core::process::{Alive, ProcessRef};
use wakeshot_core::settings::JournalOwner;
use wakeshot_windows::log::{Level, Log};
use wakeshot_windows::power::journal::{FileNet, restore_locked};
use wakeshot_windows::power::plan::{DryRunPower, Win32Power};
use wakeshot_windows::process::{Liveness, SystemLiveness};
use wakeshot_windows::store::{self, AutoStart, DataDir, DataFile, Names};

use crate::command::Global;
use crate::status::{self, AREA, Answer, Tray};

/// How long the tray gets to answer, and how often the file is looked at (architecture section 4).
const ANSWER_WAIT: Duration = Duration::from_secs(5);
const ANSWER_POLL: Duration = Duration::from_millis(100);

const NOTHING: &str = "Nothing to restore.";
const RESTORED: &str = "Your power settings are restored.";
const NO_ANSWER: &str = "Wakeshot did not answer within 5 seconds. It still restores your power settings when it \
                         gets to the request.";
const STILL_CHANGED: &str = "Wakeshot could not put back every power setting yet. It keeps trying.";
const CORRUPT: &str = "Wakeshot's record of your old power settings was damaged, so it cannot put them back. It is \
                       kept as journal.json.corrupt. Set the lid and sleep settings again by hand; the log has the old \
                       values.";
const OTHER_OWNER: &str =
    "Another Wakeshot took over your power settings just now. Run 'wakeshot restore' again.";
const UNCLEAR: &str =
    "Wakeshot took the request, but what it did cannot be read. Run 'wakeshot status'.";

pub fn main(global: &Global) -> ExitCode {
    let (dir, log) = match status::open(global) {
        Ok(opened) => opened,
        Err(answer) => return answer.print(),
    };
    let answer = match global.dry_run {
        // The dry-run folder, mutex names and plan writes kept in memory, as `wakeshot-tray --dry-run` uses them: no
        // RunOnce value, no real write.
        true => run(
            &dir,
            &mut Parts {
                plans: DryRunPower::new(Win32Power::new(log.clone()), log.clone()),
                net: FileNet::dry_run(dir.clone(), log.clone()),
                names: &Names::dry_run(),
                log: &*log,
                liveness: &SystemLiveness,
            },
        ),
        // RunOnce is cleared by the clean-up and never set; no guard: the CLI has no tray for one to wait on.
        false => run(
            &dir,
            &mut Parts {
                plans: Win32Power::new(log.clone()),
                net: FileNet::restore_only(dir.clone(), AutoStart::real(), log.clone()),
                names: &Names::real(),
                log: &*log,
                liveness: &SystemLiveness,
            },
        ),
    };
    answer.print()
}

/// What a run works on: the real parts in `main`, fakes and a temp folder in the tests.
pub(crate) struct Parts<'a, P, N> {
    pub plans: P,
    pub net: N,
    pub names: &'a Names,
    pub log: &'a dyn Log,
    pub liveness: &'a dyn Liveness,
}

pub(crate) fn run<P: PlanIo, N: SafetyNet>(dir: &DataDir, parts: &mut Parts<'_, P, N>) -> Answer {
    let (answer, path) = decide(dir, parts);
    let level = match answer.code {
        0 => Level::Info,
        _ => Level::Warn,
    };
    parts.log.line(level, AREA, &format!("restore: {path}"));
    answer
}

/// The answer, and the path taken with its result for the one log line.
fn decide<P: PlanIo, N: SafetyNet>(dir: &DataDir, parts: &mut Parts<'_, P, N>) -> (Answer, String) {
    // A look without the lock, to learn the owner, as restore mode does: every journal write is atomic, so the file
    // read is whole, and the routine reads it again under the lock and checks the owner once more.
    let file = match parts.net.load_journal() {
        JournalRead::Missing => return (Answer::ok(NOTHING), "no journal, nothing to do".into()),
        JournalRead::Failed(f) => {
            let why = format!("{} failed with {}", f.call, f.code);
            let text = format!("Could not read Wakeshot's record of your power settings: {why}.");
            return (Answer::failed(text), format!("journal read failed, {why}"));
        }
        // No owner can be read, so nobody else would set it aside; the routine does, as restore mode does.
        JournalRead::Corrupt => return restore_here(parts, None, 0, "journal corrupt"),
        JournalRead::Read(file) => file,
    };
    let owner = file.owner;
    let process = ProcessRef {
        pid: owner.pid,
        start: owner.proc_start,
    };
    match parts.liveness.check(process) {
        Alive::Yes => ask_tray(dir, parts, owner),
        Alive::Gone => {
            let path = format!("owner pid {} gone", owner.pid);
            restore_here(parts, Some(owner), newer_entries(&file), &path)
        }
        Alive::Unknown => (
            Answer::failed(status::UNKNOWN),
            format!("owner pid {} liveness unknown, nothing done", owner.pid),
        ),
    }
}

/// Entries whose key only a newer Wakeshot knows: the routine leaves them in the file for that version.
fn newer_entries(file: &JournalFile) -> usize {
    file.entries
        .iter()
        .filter(|e| matches!(e.key, JournalKey::Other(_)))
        .count()
}

fn restore_here<P: PlanIo, N: SafetyNet>(
    parts: &mut Parts<'_, P, N>,
    expect_owner: Option<JournalOwner>,
    newer: usize,
    path: &str,
) -> (Answer, String) {
    let Restored { found, result, .. } = restore_locked(
        &mut parts.plans,
        &mut parts.net,
        None,
        expect_owner,
        parts.names,
        parts.log,
    );
    let (answer, result) = match (found, result) {
        (Found::OtherOwner(other), _) => (
            Answer::failed(OTHER_OWNER),
            format!("another tray (pid {}) took the journal over", other.pid),
        ),
        (_, Err(f)) => {
            let why = format!("{} failed with {}", f.call, f.code);
            let text = format!(
                "Could not restore your power settings: {why}. Wakeshot tries again when it next starts."
            );
            (Answer::failed(text), format!("restore failed, {why}"))
        }
        (Found::Nothing, Ok(())) => (Answer::ok(NOTHING), "nothing to restore".to_owned()),
        (Found::Entries, Ok(())) => (Answer::ok(RESTORED), "restored here".to_owned()),
        (Found::SetAside, Ok(())) => (
            Answer::failed(CORRUPT),
            "set aside as journal.json.corrupt".to_owned(),
        ),
    };
    if newer == 0 || answer.code != 0 {
        return (answer, format!("{path}, {result}"));
    }
    let (count, them) = match newer {
        1 => ("1 setting is".to_owned(), "it"),
        n => (format!("{n} settings are"), "them"),
    };
    let text = format!(
        "{}\n{count} kept as a newer Wakeshot set {them}. Update Wakeshot to put {them} back.",
        answer.text
    );
    let log = format!("{path}, {result}; {newer} entries of a newer version kept");
    (Answer::failed(text), log)
}

/// The tray runs the same path as its menu item, writes `state.json`, then deletes the request (review m4), so once
/// the file is gone `state.json` says what the restore left.
fn ask_tray<P: PlanIo, N: SafetyNet>(
    dir: &DataDir,
    parts: &mut Parts<'_, P, N>,
    owner: JournalOwner,
) -> (Answer, String) {
    let request = dir.file(DataFile::RestoreRequest);
    let path = format!("owner pid {} alive, asked the tray", owner.pid);
    if let Err(e) = store::write_atomic(&request, br#"{"version":1}"#) {
        let why = status::error_text(e);
        let text = format!("Could not ask Wakeshot to restore: {why}.");
        return (
            Answer::failed(text),
            format!("{path}: request not written, {why}"),
        );
    }
    if !gone_within(&request, ANSWER_WAIT) {
        // The request stays: the tray still acts on it when it gets to it.
        return (
            Answer::failed(NO_ANSWER),
            format!("{path}: no answer within 5 s"),
        );
    }
    match status::tray(dir, parts.liveness) {
        Tray::Running(state) if state.overridden.is_empty() => {
            (Answer::ok(RESTORED), format!("{path}: restored"))
        }
        Tray::Running(state) => {
            let mut lines = vec![STILL_CHANGED.to_owned()];
            lines.extend(status::overridden(&state.overridden));
            let log = format!("{path}: {} keys still overridden", state.overridden.len());
            (Answer::failed(lines.join("\n")), log)
        }
        other => (
            Answer::failed(UNCLEAR),
            format!("{path}: answered, then {}", other.describe()),
        ),
    }
}

/// `true`: the file is gone. A look that fails counts as still there.
fn gone_within(file: &Path, wait: Duration) -> bool {
    let start = Instant::now();
    loop {
        if matches!(file.try_exists(), Ok(false)) {
            return true;
        }
        if start.elapsed() >= wait {
            return false;
        }
        std::thread::sleep(ANSWER_POLL);
    }
}

#[cfg(test)]
mod tests {
    use std::thread;

    use wakeshot_core::ownership::fake::{Action, FakePlans};
    use wakeshot_core::process::FileTime;
    use wakeshot_core::settings::{Guid, PlanValues, SettingKey};
    use wakeshot_windows::log::MemoryLog;
    use wakeshot_windows::process::FakeLiveness;

    use super::*;
    use crate::status::fixture::{TempData, dead, me, state_json};

    const BALANCED: Guid = Guid(0x381b4222_f694_41f0_9685_ff5bb260df2e);
    const DO_NOTHING: u32 = 0;
    const SLEEP: u32 = 1;

    /// Haik's values with our Do nothing on battery: what a hold leaves.
    fn plans() -> FakePlans {
        FakePlans::new(PlanValues {
            active: BALANCED,
            lid_ac: SLEEP,
            lid_dc: DO_NOTHING,
            sleep_ac: 10_800,
            sleep_dc: 14_400,
            saver_threshold_dc: 20,
        })
    }

    /// One entry: we wrote Do nothing over Sleep for the lid on battery.
    fn journal(owner: ProcessRef) -> String {
        journal_with(owner, 1, "")
    }

    fn journal_with(owner: ProcessRef, version: u32, more: &str) -> String {
        format!(
            r#"{{"version":{version},"owner":{{"pid":{},"procStart":"{}","session":1}},"entries":[{{"plan":"{BALANCED}","key":"lidDc","original":{SLEEP},"written":{DO_NOTHING},"restored":false}}{more}]}}"#,
            owner.pid, owner.start.0
        )
    }

    struct Run {
        answer: Answer,
        log: Vec<String>,
        plans: FakePlans,
    }

    /// The real `FileNet` form of `main` (`restore_only`) over the temp folder, with a RunOnce key that does not
    /// exist, so the clean-up's remove finds nothing and the registry is not written.
    fn restore(data: &TempData, plans: FakePlans, liveness: &dyn Liveness) -> Run {
        let log = MemoryLog::default();
        let base = format!(r"Software\Wakeshot.Test.CliRestore.{}", std::process::id());
        let names = Names::test("cli-restore");
        let net = {
            let lines: std::sync::Arc<dyn Log> = std::sync::Arc::new(MemoryLog::default());
            FileNet::restore_only(data.dir.clone(), AutoStart::at(&base), lines)
        };
        let answer = run(
            &data.dir,
            &mut Parts {
                plans: plans.clone(),
                net,
                names: &names,
                log: &log,
                liveness,
            },
        );
        // Only this crate's lines: `restore_locked` logs its own, in its own area.
        let lines = log.lines.into_inner().unwrap();
        Run {
            answer,
            log: lines
                .into_iter()
                .filter(|(_, area, _)| *area == AREA)
                .map(|(_, _, text)| text)
                .collect(),
            plans,
        }
    }

    #[test]
    fn no_journal_is_nothing_to_restore() {
        let data = TempData::new("restore-none");
        let run = restore(&data, plans(), &SystemLiveness);
        assert_eq!(run.answer, Answer::ok(NOTHING));
        assert_eq!(run.log, ["restore: no journal, nothing to do"]);
        assert!(run.plans.state().calls.is_empty());
    }

    #[test]
    fn a_dead_owner_is_restored_here() {
        let data = TempData::new("restore-dead");
        let owner = dead();
        data.write(DataFile::Journal, &journal(owner));
        let run = restore(&data, plans(), &SystemLiveness);
        assert_eq!(run.answer, Answer::ok(RESTORED));
        assert_eq!(
            run.log,
            [format!(
                "restore: owner pid {} gone, restored here",
                owner.pid
            )]
        );
        assert_eq!(
            run.plans.state().stored(BALANCED, SettingKey::LidDc),
            Some(SLEEP)
        );
        assert_eq!(data.read(DataFile::Journal), None);
        assert_eq!(data.read(DataFile::RestoreRequest), None);
    }

    #[test]
    fn a_dead_owner_whose_restore_fails_keeps_the_journal() {
        let data = TempData::new("restore-fails");
        let owner = dead();
        let text = journal(owner);
        data.write(DataFile::Journal, &text);
        let plans = plans();
        // The first plan call, the active plan read: the run fails before any entry is dropped or written.
        plans.script(0, Action::Fail(5));
        let run = restore(&data, plans, &SystemLiveness);
        assert_eq!(
            run.answer,
            Answer::failed(
                "Could not restore your power settings: FakePlans::active_plan failed with 5. Wakeshot tries \
                 again when it next starts."
            )
        );
        assert_eq!(
            run.plans.state().stored(BALANCED, SettingKey::LidDc),
            Some(DO_NOTHING)
        );
        assert_eq!(data.read(DataFile::Journal).unwrap(), text.as_bytes());
        assert_eq!(
            run.log,
            [format!(
                "restore: owner pid {} gone, restore failed, FakePlans::active_plan failed with 5",
                owner.pid
            )]
        );
    }

    #[test]
    fn unknown_owner_liveness_starts_nothing() {
        let data = TempData::new("restore-unknown");
        let owner = me();
        let text = journal(owner);
        data.write(DataFile::Journal, &text);
        let liveness = FakeLiveness::default();
        liveness.alive.lock().unwrap().push((owner, Alive::Unknown));
        let run = restore(&data, plans(), &liveness);
        assert_eq!(run.answer, Answer::failed(status::UNKNOWN));
        assert!(run.plans.state().calls.is_empty());
        assert_eq!(data.read(DataFile::Journal).unwrap(), text.as_bytes());
        assert_eq!(data.read(DataFile::RestoreRequest), None);
    }

    /// A fake tray: after `after`, it writes `state.json` (this process as the tray), then deletes the request, in
    /// that order, as `FinishRestoreRequest` does.
    fn fake_tray(
        data: &TempData,
        after: Duration,
        overridden: &'static [(&'static str, &'static str)],
    ) -> thread::JoinHandle<bool> {
        let dir = data.dir.clone();
        thread::spawn(move || {
            thread::sleep(after);
            let request = dir.file(DataFile::RestoreRequest);
            if !request.exists() {
                return false;
            }
            let state = state_json(me(), true, &[], overridden, false);
            std::fs::write(dir.file(DataFile::State), state).unwrap();
            std::fs::remove_file(request).unwrap();
            true
        })
    }

    #[test]
    fn a_live_owner_is_asked_and_its_answer_read() {
        let data = TempData::new("restore-ask");
        let owner = me();
        data.write(DataFile::Journal, &journal(owner));
        let tray = fake_tray(&data, Duration::from_secs(1), &[]);
        let started = Instant::now();
        let run = restore(&data, plans(), &SystemLiveness);
        let took = started.elapsed();
        assert!(tray.join().unwrap(), "the request file was not there");
        assert_eq!(run.answer, Answer::ok(RESTORED));
        assert!(
            took >= Duration::from_secs(1) && took < Duration::from_secs(3),
            "{took:?}"
        );
        assert_eq!(
            run.log,
            [format!(
                "restore: owner pid {} alive, asked the tray: restored",
                owner.pid
            )]
        );
        // The tray restores; this process touched no plan and left the journal to it.
        assert!(run.plans.state().calls.is_empty());
        assert!(data.read(DataFile::Journal).is_some());
    }

    #[test]
    fn a_tray_that_could_not_restore_says_what_is_still_changed() {
        let data = TempData::new("restore-still");
        data.write(DataFile::Journal, &journal(me()));
        let tray = fake_tray(
            &data,
            Duration::from_millis(300),
            &[("381b4222-f694-41f0-9685-ff5bb260df2e", "lidDc")],
        );
        let run = restore(&data, plans(), &SystemLiveness);
        assert!(tray.join().unwrap());
        assert_eq!(
            run.answer,
            Answer::failed(format!(
                "{STILL_CHANGED}\nChanged by Wakeshot: what closing the lid does on battery"
            ))
        );
    }

    #[test]
    fn a_tray_that_does_not_answer_leaves_the_request() {
        let data = TempData::new("restore-timeout");
        let owner = me();
        data.write(DataFile::Journal, &journal(owner));
        let started = Instant::now();
        let run = restore(&data, plans(), &SystemLiveness);
        let took = started.elapsed();
        assert_eq!(run.answer, Answer::failed(NO_ANSWER));
        assert!(
            took >= ANSWER_WAIT && took < ANSWER_WAIT + Duration::from_secs(2),
            "{took:?}"
        );
        let request = data
            .read(DataFile::RestoreRequest)
            .expect("the request stays");
        assert_eq!(request, br#"{"version":1}"#);
        assert_eq!(
            run.log,
            [format!(
                "restore: owner pid {} alive, asked the tray: no answer within 5 s",
                owner.pid
            )]
        );
        assert!(run.plans.state().calls.is_empty());
    }

    #[test]
    fn a_corrupt_journal_is_set_aside_and_said_plainly() {
        let data = TempData::new("restore-corrupt");
        data.write(DataFile::Journal, "{ \"version\": 1, \"owner\": ");
        let run = restore(&data, plans(), &SystemLiveness);
        assert_eq!(run.answer, Answer::failed(CORRUPT));
        assert_eq!(
            run.log,
            ["restore: journal corrupt, set aside as journal.json.corrupt"]
        );
        assert_eq!(data.read(DataFile::Journal), None);
        assert!(data.read(DataFile::JournalCorrupt).is_some());
    }

    /// A newer journal: the keys this version knows are restored, the others stay for the version that wrote them.
    #[test]
    fn a_newer_journal_restores_what_this_version_knows() {
        let data = TempData::new("restore-newer");
        let owner = dead();
        let other = format!(
            r#",{{"plan":"{BALANCED}","key":"sleepAc","original":10800,"written":0,"restored":false}}"#
        );
        data.write(DataFile::Journal, &journal_with(owner, 2, &other));
        let run = restore(&data, plans(), &SystemLiveness);
        assert_eq!(
            run.answer,
            Answer::failed(format!(
                "{RESTORED}\n1 setting is kept as a newer Wakeshot set it. Update Wakeshot to put it back."
            ))
        );
        assert_eq!(
            run.plans.state().stored(BALANCED, SettingKey::LidDc),
            Some(SLEEP)
        );
        let kept = String::from_utf8(data.read(DataFile::Journal).expect("kept")).unwrap();
        assert!(
            kept.contains("sleepAc") && !kept.contains("lidDc"),
            "{kept}"
        );

        // Run again: nothing this version knows is left.
        let again = restore(&data, plans(), &SystemLiveness);
        assert_eq!(
            again.answer,
            Answer::failed(format!(
                "{NOTHING}\n1 setting is kept as a newer Wakeshot set it. Update Wakeshot to put it back."
            ))
        );
    }

    /// A journal whose owner is not the one read before the lock: another tray took it over; nothing is written.
    #[test]
    fn a_journal_taken_over_meanwhile_is_left_alone() {
        let data = TempData::new("restore-other");
        let first = dead();
        let taker = ProcessRef {
            pid: first.pid,
            start: FileTime(first.start.0 + 1),
        };
        data.write(DataFile::Journal, &journal(first));
        let mut parts = Parts {
            plans: plans(),
            net: FileNet::dry_run(data.dir.clone(), std::sync::Arc::new(MemoryLog::default())),
            names: &Names::test("cli-restore-other"),
            log: &MemoryLog::default(),
            liveness: &SystemLiveness,
        };
        // The look before the lock saw `first`; a tray then wrote the journal as its own.
        let owner = match parts.net.load_journal() {
            JournalRead::Read(file) => file.owner,
            other => panic!("{other:?}"),
        };
        data.write(DataFile::Journal, &journal(taker));
        let (answer, log) = restore_here(&mut parts, Some(owner), 0, "owner gone");
        assert_eq!(answer, Answer::failed(OTHER_OWNER));
        assert!(log.contains("took the journal over"), "{log}");
        assert_eq!(
            parts.plans.state().stored(BALANCED, SettingKey::LidDc),
            Some(DO_NOTHING)
        );
    }

    /// The card's `--dry-run` check through `main`, as the exe runs it once `args::parse` routes it (`p4-cli-args`).
    /// Uses the real dry-run folder, which must hold no journal: run by hand, never beside `p6-install-e2e`. With no
    /// journal nothing reads or writes a plan.
    #[test]
    #[ignore = "uses %LocalAppData%\\Wakeshot\\dry-run: run by hand"]
    fn dry_run_folder_check() {
        let dry = Global { dry_run: true };
        let dir = status::data_dir(&dry).unwrap();
        assert!(dir.root().ends_with(r"Wakeshot\dry-run"), "{dir:?}");
        assert!(!dir.file(DataFile::Journal).exists(), "a dry-run journal");
        assert_eq!(main(&dry), ExitCode::SUCCESS);
    }
}
