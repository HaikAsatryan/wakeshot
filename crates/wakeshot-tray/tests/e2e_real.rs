//! The real tray, not a dry run, with real holds on two throwaway plan copies made active in turn: every path that
//! writes or restores power settings (`docs/tasks/p6-int-real-e2e.md`; `docs/phases/06-integration.md` notes 3, 6
//! and 7; rows 4, 6 and 9 of the manual checklist in architecture section 9).
//!
//! Owner: `p6-int-real-e2e`. Every test is ignored and named `real_e2e_*`: they write power settings and the real
//! RunOnce value and start real guards, so they run only in the global lane, alone on the laptop, after the CLI is
//! built into the same target folder, with `WAKESHOT_REAL_E2E_BEFORE` naming the BEFORE record (below).
//! `real_e2e_cleanup` runs first and last, and it is what to run after a run that was cut off:
//!
//! ```text
//! cargo build --locked -p wakeshot-cli --target x86_64-pc-windows-msvc
//! cargo test --locked -p wakeshot-tray --target x86_64-pc-windows-msvc --test e2e_real -- --ignored --exact real_e2e_cleanup
//! cargo test --locked -p wakeshot-tray --target x86_64-pc-windows-msvc --test e2e_real -- --ignored --test-threads=1
//! cargo test --locked -p wakeshot-tray --target x86_64-pc-windows-msvc --test e2e_real -- --ignored --exact real_e2e_cleanup
//! ```
//!
//! The rules every test keeps, so the user's own plan is never touched:
//!
//! - The user's plan (the record's active plan) is only read: never written, and never active while a tray of this
//!   build runs, because a holding tray applies to whatever plan is active. Each test makes `Wakeshot int A` (and
//!   `Wakeshot int B`) at Medium with `powercfg /duplicatescheme`, makes A active, and only then starts the tray. At
//!   the end every tray and guard of this build has ended before the user's plan is active again and the copies go.
//! - The tray watches an empty Claude folder, so the agent sessions on this PC never hold it.
//! - A test starts only on a clean PC: the user's plan active with the recorded values, the recorded plan list and
//!   RunOnce values, no process of this build and no real tray in this session. Otherwise it fails at once and says
//!   to run `real_e2e_cleanup`.
//!
//! The BEFORE record is a text file of `key=value` lines that only these tests read: `active` (the active plan's id),
//! `lid_ac`, `lid_dc`, `sleep_ac`, `sleep_dc`, `saver_ac` and `saver_dc` (its values by `powercfg /qh`, decimal), one
//! `plan` line per plan `powercfg /l` lists, and one `run_once` line per value of the user's RunOnce key, as `reg
//! query` prints it with the spaces squeezed to one. `#` starts a comment.
//!
//! Log lines are matched only where their text belongs to the tray, the core, the guard or the CLI; the journal's own
//! lines (`power/journal.rs`) are printed, never matched, so a wording change there cannot fail a test.

mod e2e;

use std::ffi::c_void;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::str::FromStr;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use std::{env, fs, thread};

use e2e::{
    Claude, EXIT_WITHIN, FOLLOW_WITHIN, LogMark, REAL_PREFIX, SAFETY_TICK, TRAY_EXE, TempData,
    Tray, assert_medium, exit_code_within, full_path, loop_window, skip, terminate,
    trays_of_this_build, wait_for, wait_for_result, wait_until,
};
use wakeshot_core::files::{JournalFile, JournalKey, StateFile};
use wakeshot_core::process::{Alive, ProcessRef};
use wakeshot_core::settings::{Guid, SettingKey};
use wakeshot_core::text;
use wakeshot_core::view::KeepAwake;
use wakeshot_windows::app::window::ask_quit;
use wakeshot_windows::power::journal::RUN_ONCE_NAME;
use wakeshot_windows::process::{self as winproc, Liveness, ProcessHandle, SystemLiveness};
use wakeshot_windows::session;
use wakeshot_windows::store::{AutoStart, StartKey};
use wakeshot_windows::tray::icon::IconKind;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::System::Power::{
    CallNtPowerInformation, POWER_INFORMATION_LEVEL, SYSTEM_POWER_POLICY, SystemPowerPolicyAc,
    SystemPowerPolicyDc,
};
use windows::Win32::UI::WindowsAndMessaging::{
    ENDSESSION_LOGOFF, SMTO_ABORTIFHUNG, SendMessageTimeoutW, WM_ENDSESSION, WM_QUERYENDSESSION,
};

const BEFORE_ENV: &str = "WAKESHOT_REAL_E2E_BEFORE";
const RUN_ONCE_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\RunOnce";
const NOTIFY_KEY: &str = r"HKCU\Control Panel\NotifyIconSettings";

/// The two copies this card may make, delete and switch to (phase file note 7). `Wakeshot int haik` belongs to
/// `p6-int-haik-checks` and `Wakeshot test <pid>` to the power harness: neither is ever touched here.
const NAME_A: &str = "Wakeshot int A";
const NAME_B: &str = "Wakeshot int B";
/// In each data folder: the ids handed to `powercfg /duplicatescheme`, written before it runs, so a copy that never
/// got its name is still known to be this card's.
const MADE_FILE: &str = "copy-ids.txt";

/// `powercfg` subgroup and setting ids: lid action (a hidden setting, so `/qh`), sleep after, energy saver threshold.
const SUB_BUTTONS: &str = "4f971e89-eebd-4455-a8de-9e59040e7347";
const LID: &str = "5ca83367-6e45-459f-a27b-476b1d01c936";
const SUB_SLEEP: &str = "238c9fa8-0aad-41ed-83f4-97be242c8f20";
const SLEEP: &str = "29f6c1db-86da-48c5-9fdb-f2b67b1f44da";
const SUB_SAVER: &str = "de830923-a562-41af-a086-e3a2c6bad2da";
const SAVER: &str = "e69653ca-cf7f-4f05-aa73-cb833fa90ad4";

/// The user's change in the user-change test: Hibernate, neither the user's Sleep nor Wakeshot's Do nothing, so a
/// restore that wrongly wrote the original back would show. Only stored, on a copy, with no lid closed.
const USER_LID_DC: u32 = 2;
/// `SYSTEM_POWER_POLICY.IdleTimeout` while sleep after is Never (research, "Power track real checks").
const NEVER_IDLE: u32 = 0x7fff_ffff;

/// The card's limits: an apply after Keep awake on; a guard gone, or its restore after the tray died; the ownership
/// check seeing a user change (it runs every 10 s); the end-of-session send.
const APPLY_WITHIN: Duration = Duration::from_secs(10);
const GUARD_WITHIN: Duration = Duration::from_secs(15);
const USER_CHANGE_WITHIN: Duration = Duration::from_secs(20);
const SESSION_END_WITHIN: Duration = Duration::from_secs(5);
/// A plan switch is read at once from `GUID_ACTIVE_POWERSCHEME`, at the latest by the next 10 s plan read.
const PLAN_SWITCH_WITHIN: Duration = Duration::from_secs(15);
/// A plan read comes at every safety tick at the latest, so a write that was still to come would come by then.
const NO_WRITE_FOR: Duration = Duration::from_secs(SAFETY_TICK.as_secs() + 5);
/// Explorer and `cmd` take about a second before a `.cmd` runs.
const MEDIUM_CMD_WITHIN: Duration = Duration::from_secs(20);

#[test]
#[ignore = "real power settings, RunOnce and guard: global lane only, alone on the laptop"]
fn real_e2e_cleanup() {
    if skip() {
        return;
    }
    let before = Before::load();
    let mut notes = Vec::new();
    end_this_build(&mut notes);
    let leftovers = temp_folders();
    let mut ours: Vec<Guid> = list_plans()
        .into_iter()
        .filter(|p| p.name == NAME_A || p.name == NAME_B)
        .map(|p| p.id)
        .collect();
    for folder in &leftovers {
        ours.extend(made_ids(folder));
    }
    ours.sort();
    ours.dedup();
    let active = active_plan();
    if active != before.active {
        assert!(
            ours.contains(&active),
            "the active plan {active} is neither the recorded {} nor a copy of these tests: left alone",
            before.active
        );
        powercfg(&["/setactive", &before.active.to_string()]);
        notes.push(format!(
            "the recorded plan {} is active again (was {active})",
            before.active
        ));
    }
    let listed: Vec<Guid> = list_plans().iter().map(|p| p.id).collect();
    for id in ours.iter().filter(|id| listed.contains(id)) {
        powercfg(&["/delete", &id.to_string()]);
        notes.push(format!("copy {id} deleted"));
    }
    if let Some(value) = wakeshot_restore().expect("read the RunOnce value")
        && points_into_target(&value)
    {
        let removed = AutoStart::real().remove(StartKey::RunOnce, RUN_ONCE_NAME);
        notes.push(format!("RunOnce value {value} removed: {removed:?}"));
    }
    for folder in leftovers {
        let removed = fs::remove_dir_all(&folder);
        notes.push(format!("{} removed: {removed:?}", folder.display()));
    }
    if notes.is_empty() {
        println!("clean-up: nothing to do");
    }
    for note in &notes {
        println!("clean-up: {note}");
    }

    let left: Vec<u32> = trays_of_this_build()
        .iter()
        .map(|h| h.process().pid)
        .collect();
    assert!(
        left.is_empty(),
        "processes of this build still run: {left:?}"
    );
    let copies: Vec<PlanInfo> = list_plans()
        .into_iter()
        .filter(|p| p.name.starts_with("Wakeshot int") || p.name.starts_with("Wakeshot test"))
        .collect();
    assert!(copies.is_empty(), "plan copies left: {copies:?}");
    let differences = before.differences();
    assert!(
        differences.is_empty(),
        "AFTER differs from BEFORE:\n{}",
        differences.join("\n")
    );
    println!("AFTER equals BEFORE: {before:?}");
}

#[test]
#[ignore = "real power settings, RunOnce and guard: global lane only, alone on the laptop"]
fn real_e2e_hold_start_applies_and_hold_end_restores() {
    if skip() {
        return;
    }
    let bench = Bench::new("hold", &[NAME_A]);
    let a = bench.copy(0);
    let mut tray = bench.start_tray();
    let guard = bench.hold(&tray, a);
    assert!(
        system_request_of_this_build().expect("powercfg /requests"),
        "no SYSTEM request of this build while Keep awake holds"
    );
    bench.release(a, a.original, &guard);
    assert_eq!(tray.quit(), 0);
    bench.finish();
}

#[test]
#[ignore = "real power settings, RunOnce and guard: global lane only, alone on the laptop"]
fn real_e2e_user_change_drops_the_entry_and_stays() {
    if skip() {
        return;
    }
    let bench = Bench::new("user-change", &[NAME_A]);
    let a = bench.copy(0);
    assert_ne!(a.original.lid_dc, USER_LID_DC, "pick another user value");
    let mut tray = bench.start_tray();
    let guard = bench.hold(&tray, a);

    let mark = bench.data.log_mark();
    let changed = Instant::now();
    set_lid_dc(a, USER_LID_DC);
    let left = [SettingKey::LidAc, SettingKey::SleepDc];
    wait_for(USER_CHANGE_WITHIN, "the journal drops lid DC", || {
        let journal = bench.data.journal()?.ok_or("no journal")?;
        check_journal(&journal, &tray, a, &left)
    });
    println!(
        "lid DC entry dropped {} ms after the change",
        changed.elapsed().as_millis()
    );
    let line = bench.data.wait_log(
        mark,
        &format!("for this episode: the user changed it on plan {}", a.id),
        FOLLOW_WITHIN,
    );
    println!("{line}");
    // The core keeps notices while the session is locked and shows them at the unlock (`Core::notify`).
    match session::locked_now() {
        Some(false) => {
            let notice = bench.data.wait_log(mark, "UserChanged", FOLLOW_WITHIN);
            println!("{notice}");
        }
        locked => {
            println!("session locked: {locked:?}, so the UserChanged notice waits for the unlock")
        }
    }
    bench
        .data
        .wait_state(FOLLOW_WITHIN, "lid DC no longer overridden", |s| {
            s.overridden.len() == 2 && s.overridden.iter().all(|k| k.key != SettingKey::LidDc)
        });

    let mut want = a.original;
    want.lid_dc = USER_LID_DC;
    bench.release(a, want, &guard);

    // The test's own change goes back, so A ends as it was made.
    set_lid_dc(a, a.original.lid_dc);
    assert_eq!(read_values(a.id), a.original);
    assert_eq!(tray.quit(), 0);
    bench.finish();
}

#[test]
#[ignore = "real power settings, RunOnce and guard: global lane only, alone on the laptop"]
fn real_e2e_plan_switch_moves_the_writes_to_the_new_plan() {
    if skip() {
        return;
    }
    let bench = Bench::new("plan-switch", &[NAME_A, NAME_B]);
    let (a, b) = (bench.copy(0), bench.copy(1));
    let mut tray = bench.start_tray();
    let guard = bench.hold(&tray, a);

    let mark = bench.data.log_mark();
    let switched = Instant::now();
    powercfg(&["/setactive", &b.id.to_string()]);
    let all = [SettingKey::LidAc, SettingKey::LidDc, SettingKey::SleepDc];
    wait_for(PLAN_SWITCH_WITHIN, "A restored and B applied", || {
        plan_has(a, a.original)?;
        plan_has(b, b.original.applied())?;
        let journal = bench.data.journal()?.ok_or("no journal")?;
        check_journal(&journal, &tray, b, &all)
    });
    println!(
        "A restored and B applied {} ms after the switch",
        switched.elapsed().as_millis()
    );
    for line in bench.data.log_since(mark) {
        println!("after the switch: {line}");
    }
    // The restore of A may empty the journal, which stops the guard, and the apply to B then starts a new one.
    let old = guard.process().pid;
    let guard = bench.one_guard(&tray);
    println!(
        "guard after the switch: pid {} (before: {old})",
        guard.process().pid
    );

    bench.release(b, b.original, &guard);
    assert_eq!(read_values(a.id), a.original, "A changed at the hold end");
    assert_eq!(tray.quit(), 0);
    bench.finish();
}

#[test]
#[ignore = "real power settings, RunOnce and guard: global lane only, alone on the laptop"]
fn real_e2e_restore_my_power_settings_keeps_requests_only() {
    if skip() {
        return;
    }
    let bench = Bench::new("restore-mine", &[NAME_A]);
    let a = bench.copy(0);
    let mut tray = bench.start_tray();
    let guard = bench.hold(&tray, a);

    let mark = bench.data.log_mark();
    let started = Instant::now();
    let said = bench.data.cli("restore");
    println!("wakeshot restore took {} ms", started.elapsed().as_millis());
    assert_eq!(
        (said.code, said.out.as_str()),
        (0, "Your power settings are restored.\n"),
        "{said:?}"
    );
    wait_for(
        FOLLOW_WITHIN,
        "A restored by Restore my power settings",
        || bench.restored(a, a.original),
    );
    let code = wait_gone(&guard, GUARD_WITHIN);
    println!("guard ended with {code:?} after the restore");
    let line = bench.data.wait_log(
        mark,
        "Released every key for this episode: Restore my power settings",
        Duration::ZERO,
    );
    println!("{line}");
    let line = bench
        .data
        .wait_log(mark, "restore: owner pid", Duration::ZERO);
    assert!(
        line.ends_with(&format!(
            "owner pid {} alive, asked the tray: restored",
            tray.pid()
        )),
        "{line}"
    );
    // The hold goes on with power requests only.
    let state = bench.data.state().expect("state.json");
    assert!(
        keep_awake_only(&state) && state.overridden.is_empty(),
        "{state:?}"
    );
    assert!(
        system_request_of_this_build().expect("powercfg /requests"),
        "the SYSTEM request went with the restore"
    );

    // No write until the hold ends: one safety tick and more, with the plan read it brings.
    let quiet = Instant::now();
    while quiet.elapsed() < NO_WRITE_FOR {
        if let Err(why) = bench.restored(a, a.original) {
            panic!(
                "a write {} ms after the restore: {why}",
                quiet.elapsed().as_millis()
            );
        }
        thread::sleep(Duration::from_secs(1));
    }
    let lines = bench.data.log_since(mark);
    let guards_started = lines
        .iter()
        .filter(|l| l.contains("started for tray pid"))
        .count();
    assert_eq!(
        guards_started, 0,
        "a guard started after the restore: {lines:#?}"
    );

    let said = bench.data.cli("keep-awake off");
    assert_eq!(said.code, 0, "{said:?}");
    bench
        .data
        .wait_state(FOLLOW_WITHIN, "idle after off", |s| s.holding.is_empty());
    bench.restored(a, a.original).expect("A after the hold end");

    // The next hold is a new episode: it applies again.
    let guard = bench.hold(&tray, a);
    bench.release(a, a.original, &guard);
    assert_eq!(tray.quit(), 0);
    bench.finish();
}

#[test]
#[ignore = "real power settings, RunOnce and guard: global lane only, alone on the laptop"]
fn real_e2e_quit_during_a_hold_restores_and_exits() {
    if skip() {
        return;
    }
    let bench = Bench::new("quit", &[NAME_A]);
    let a = bench.copy(0);
    let mut tray = bench.start_tray();
    let guard = bench.hold(&tray, a);
    assert!(bench.data.keep_awake().exists());

    let mark = bench.data.log_mark();
    assert_eq!(tray.quit(), 0);
    // The quit restores before the tray exits.
    bench.restored(a, a.original).expect("A after the quit");
    assert!(
        !bench.data.keep_awake().exists(),
        "the Keep awake hold file is left"
    );
    let code = wait_gone(&guard, GUARD_WITHIN);
    println!("guard ended with {code:?} after the quit");
    bench
        .data
        .wait_log(mark, "Wakeshot ends with code 0", Duration::ZERO);
    bench.finish();
}

#[test]
#[ignore = "real power settings, RunOnce and guard: global lane only, alone on the laptop"]
fn real_e2e_session_end_restores_before_the_send_returns() {
    if skip() {
        return;
    }
    let bench = Bench::new("session-end", &[NAME_A]);
    let a = bench.copy(0);
    let tray = bench.start_tray();
    let guard = bench.hold(&tray, a);

    let (hwnd, pid) = loop_window(false).expect("the real loop window");
    assert_eq!(pid, tray.pid(), "the real loop window is not this tray's");
    let lparam = LPARAM(ENDSESSION_LOGOFF as isize);
    let (sent, answer) = send(hwnd, WM_QUERYENDSESSION, WPARAM(0), lparam);
    assert!(sent, "WM_QUERYENDSESSION was not delivered");
    assert_eq!(answer, 1, "WM_QUERYENDSESSION answers TRUE");
    let started = Instant::now();
    let (sent, _) = send(hwnd, WM_ENDSESSION, WPARAM(1), lparam);
    let took = started.elapsed();
    println!(
        "the WM_ENDSESSION send returned after {} ms",
        took.as_millis()
    );
    assert!(sent, "WM_ENDSESSION was not delivered");
    assert!(took < SESSION_END_WITHIN, "{took:?}");
    bench
        .restored(a, a.original)
        .expect("A when the send returned");
    let log = bench.data.log();
    println!(
        "{}",
        log.lines()
            .find(|l| l.contains("Session end: the restore"))
            .unwrap_or("no session end line yet")
    );

    // Windows ends the process once its windows answered.
    tray.kill();
    let code = wait_gone(&guard, GUARD_WITHIN);
    println!("guard ended with {code:?} after the end by pid");
    bench
        .restored(a, a.original)
        .expect("A after the end by pid");
    bench.finish();
}

#[test]
#[ignore = "real power settings, RunOnce and guard: global lane only, alone on the laptop"]
fn real_e2e_killed_tray_guard_restores() {
    if skip() {
        return;
    }
    let bench = Bench::new("killed", &[NAME_A]);
    let a = bench.copy(0);
    let tray = bench.start_tray();
    let guard = bench.hold(&tray, a);
    let mark = bench.data.log_mark();

    let killed = Instant::now();
    tray.kill();
    wait_for(GUARD_WITHIN, "the guard restores A", || {
        bench.restored(a, a.original)
    });
    println!(
        "the guard restored A {} ms after the end by pid",
        killed.elapsed().as_millis()
    );
    let code = wait_gone(&guard, GUARD_WITHIN.saturating_sub(killed.elapsed()));
    println!("guard ended with {code:?}");
    let line = bench.data.wait_log(
        mark,
        &format!("guard for tray pid {}:", tray.pid()),
        Duration::ZERO,
    );
    println!("{line}");
    bench.finish();
}

#[test]
#[ignore = "real power settings, RunOnce and guard: global lane only, alone on the laptop"]
fn real_e2e_cli_restore_with_a_dead_owner() {
    if skip() {
        return;
    }
    let bench = Bench::new("cli-restore", &[NAME_A]);
    let a = bench.copy(0);
    // What a crashed tray's apply leaves on lid DC: Do nothing, stored and in force.
    set_lid_dc(a, 0);
    let mut held = a.original;
    held.lid_dc = 0;
    plan_has(a, held).expect("lid DC 0 on A");

    // The owner: a process this test started and ended.
    let mut child = Command::new("ping")
        .args(["-n", "30", "127.0.0.1"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .spawn()
        .expect("start ping");
    let owner = ProcessRef {
        pid: child.id(),
        start: winproc::start_of(&child).expect("ping's start time"),
    };
    child.kill().expect("end ping");
    let _ = child.wait();
    assert_eq!(SystemLiveness.check(owner), Alive::Gone);
    let session = winproc::current_session().expect("this session");
    let journal = format!(
        "{{\n  \"version\": 1,\n  \"owner\": {{ \"pid\": {}, \"procStart\": \"{}\", \"session\": {session} }},\n  \
         \"entries\": [\n    {{ \"plan\": \"{}\", \"key\": \"lidDc\", \"original\": {}, \"written\": 0, \
         \"restored\": false }}\n  ]\n}}\n",
        owner.pid, owner.start, a.id, a.original.lid_dc
    );
    fs::write(bench.data.file("journal.json"), journal).expect("write journal.json");
    let read = bench
        .data
        .journal()
        .expect("read journal.json")
        .expect("journal.json");
    assert_eq!(
        (read.owner.pid, read.owner.proc_start, read.entries.len()),
        (owner.pid, owner.start, 1)
    );
    assert!(trays_of_this_build().is_empty() && loop_window(false).is_none());

    let said = bench.data.cli("restore");
    assert_eq!(
        (said.code, said.out.as_str()),
        (0, "Your power settings are restored.\n"),
        "{said:?}"
    );
    bench
        .restored(a, a.original)
        .expect("A after wakeshot restore");
    let line = bench
        .data
        .wait_log(LogMark::START, "restore: owner pid", Duration::ZERO);
    assert!(
        line.ends_with(&format!("owner pid {} gone, restored here", owner.pid)),
        "{line}"
    );
    bench.finish();
}

#[test]
#[ignore = "real power settings, RunOnce and guard: global lane only, alone on the laptop"]
fn real_e2e_icon_settings_entry_names_the_product_guid() {
    if skip() {
        return;
    }
    let bench = Bench::new("icon", &[NAME_A]);
    let earlier: Vec<String> = notify_entries().into_iter().map(|e| e.id).collect();
    let mut tray = bench.start_tray();
    let look = || {
        let entries = notify_entries();
        let found: Vec<NotifyEntry> = entries
            .iter()
            .filter(|e| e.names_product() || (!earlier.contains(&e.id) && e.names_this_build()))
            .cloned()
            .collect();
        if found.is_empty() {
            Err(format!(
                "no entry with the product GUID among {} entries",
                entries.len()
            ))
        } else {
            Ok(found)
        }
    };
    let found = match wait_for_result(FOLLOW_WITHIN, look) {
        Ok(found) => found,
        // Windows may write the entry only when the icon goes.
        Err(_) => {
            assert_eq!(tray.quit(), 0);
            wait_for(FOLLOW_WITHIN, "the product GUID's entry", look)
        }
    };
    for entry in &found {
        println!(
            "NotifyIconSettings\\{}: new {}, ExecutablePath {:?}, values {:?}",
            entry.id,
            !earlier.contains(&entry.id),
            entry.value("ExecutablePath"),
            entry.values.iter().map(|v| &v.0).collect::<Vec<_>>()
        );
    }
    assert!(found.iter().any(NotifyEntry::names_product), "{found:#?}");
    if tray.exit_within(Duration::ZERO).is_none() {
        assert_eq!(tray.quit(), 0);
    }
    bench.finish();
}

/// The user's values and the PC's state before the card, from the record `WAKESHOT_REAL_E2E_BEFORE` names.
#[derive(Debug)]
struct Before {
    active: Guid,
    values: Values,
    plans: Vec<Guid>,
    run_once: Vec<String>,
}

impl Before {
    fn load() -> Self {
        let path = env::var_os(BEFORE_ENV).unwrap_or_else(|| {
            panic!("{BEFORE_ENV} is not set: it names the BEFORE record (see the module doc)")
        });
        let path = PathBuf::from(path);
        let text = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read the BEFORE record {}: {e}", path.display()));
        let mut active = None;
        let mut numbers = [None; 6];
        let mut plans = Vec::new();
        let mut run_once = Vec::new();
        for line in text.lines().map(str::trim) {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (key, value) = line
                .split_once('=')
                .unwrap_or_else(|| panic!("BEFORE line {line:?} is not key=value"));
            let value = value.trim();
            match key.trim() {
                "active" => active = Some(guid(value)),
                "plan" => plans.push(guid(value)),
                "run_once" => run_once.push(value.to_owned()),
                key => {
                    let at = VALUE_KEYS
                        .iter()
                        .position(|k| *k == key)
                        .unwrap_or_else(|| panic!("BEFORE key {key:?} is not known"));
                    numbers[at] = Some(
                        value
                            .parse::<u32>()
                            .unwrap_or_else(|_| panic!("BEFORE {key}={value} is not a number")),
                    );
                }
            }
        }
        let number =
            |at: usize| numbers[at].unwrap_or_else(|| panic!("BEFORE has no {}", VALUE_KEYS[at]));
        plans.sort();
        run_once.sort();
        Self {
            active: active.expect("BEFORE has no active plan"),
            values: Values {
                lid_ac: number(0),
                lid_dc: number(1),
                sleep_ac: number(2),
                sleep_dc: number(3),
                saver_ac: number(4),
                saver_dc: number(5),
            },
            plans,
            run_once,
        }
    }

    /// What differs from the record now; never panics, so a clean-up in a drop can call it.
    fn differences(&self) -> Vec<String> {
        let mut found = Vec::new();
        match try_active_plan() {
            Ok(active) if active == self.active => {}
            Ok(active) => found.push(format!("active plan {active}, recorded {}", self.active)),
            Err(e) => found.push(e),
        }
        match try_read_values(self.active) {
            Ok(values) if values == self.values => {}
            Ok(values) => found.push(format!(
                "values of {}: {values:?}, recorded {:?}",
                self.active, self.values
            )),
            Err(e) => found.push(e),
        }
        match try_list_plans() {
            Ok(plans) => {
                let mut ids: Vec<Guid> = plans.iter().map(|p| p.id).collect();
                ids.sort();
                if ids != self.plans {
                    found.push(format!("plans {plans:?}, recorded {:?}", self.plans));
                }
            }
            Err(e) => found.push(e),
        }
        match try_run_once_lines() {
            Ok(lines) if lines == self.run_once => {}
            Ok(lines) => found.push(format!(
                "RunOnce values {lines:?}, recorded {:?}",
                self.run_once
            )),
            Err(e) => found.push(e),
        }
        found
    }

    /// `differences` and anything of this build or a real tray, which a test must not start beside.
    fn unclean(&self) -> Vec<String> {
        let mut found = self.differences();
        let ours: Vec<u32> = trays_of_this_build()
            .iter()
            .map(|h| h.process().pid)
            .collect();
        if !ours.is_empty() {
            found.push(format!("processes of this build run: {ours:?}"));
        }
        if let Some((_, pid)) = loop_window(false) {
            found.push(format!("a real tray (pid {pid}) runs in this session"));
        }
        match wakeshot_restore() {
            Ok(None) => {}
            Ok(Some(value)) => found.push(format!("RunOnce {RUN_ONCE_NAME} is set: {value}")),
            Err(e) => found.push(e),
        }
        found
    }
}

const VALUE_KEYS: [&str; 6] = [
    "lid_ac", "lid_dc", "sleep_ac", "sleep_dc", "saver_ac", "saver_dc",
];

fn guid(text: &str) -> Guid {
    Guid::from_str(text).unwrap_or_else(|_| panic!("{text:?} is not a GUID"))
}

/// One plan's stored values, read with `powercfg /qh`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Values {
    lid_ac: u32,
    lid_dc: u32,
    sleep_ac: u32,
    sleep_dc: u32,
    saver_ac: u32,
    saver_dc: u32,
}

impl Values {
    /// What a finished apply leaves: Wakeshot's three targets (architecture section 5), the rest untouched.
    fn applied(self) -> Self {
        Self {
            lid_ac: 0,
            lid_dc: 0,
            sleep_dc: 0,
            ..self
        }
    }

    fn of(&self, key: SettingKey) -> u32 {
        match key {
            SettingKey::LidAc => self.lid_ac,
            SettingKey::LidDc => self.lid_dc,
            SettingKey::SleepDc => self.sleep_dc,
            SettingKey::SleepAc | SettingKey::VideoIdleAc | SettingKey::VideoIdleDc => {
                unreachable!("these runs check only the hold keys")
            }
        }
    }
}

/// The active plan's policy in force, from `CallNtPowerInformation` (research, "Power track real checks"): stored
/// values count only once `PowerSetActiveScheme` ran.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct InForce {
    lid_ac: i32,
    lid_dc: i32,
    idle_dc: u32,
}

impl InForce {
    fn read() -> Result<Self, String> {
        let ac = policy(SystemPowerPolicyAc)?;
        let dc = policy(SystemPowerPolicyDc)?;
        Ok(Self {
            lid_ac: ac.LidClose.Action.0,
            lid_dc: dc.LidClose.Action.0,
            idle_dc: dc.IdleTimeout,
        })
    }

    /// The keys whose policy form this laptop's reads cover; a lid value other than Do nothing or Sleep is not
    /// checked.
    fn matches(&self, values: &Values) -> bool {
        lid_action(values.lid_ac).is_none_or(|a| a == self.lid_ac)
            && lid_action(values.lid_dc).is_none_or(|a| a == self.lid_dc)
            && self.idle_dc == idle_timeout(values.sleep_dc)
    }
}

/// `LidClose.Action` for a lid action index, as read here: Do nothing is `PowerActionNone` (0), Sleep is
/// `PowerActionSleep` (2).
fn lid_action(index: u32) -> Option<i32> {
    match index {
        0 => Some(0),
        1 => Some(2),
        _ => None,
    }
}

fn idle_timeout(seconds: u32) -> u32 {
    if seconds == 0 { NEVER_IDLE } else { seconds }
}

fn policy(level: POWER_INFORMATION_LEVEL) -> Result<SYSTEM_POWER_POLICY, String> {
    // SAFETY: plain data of integers and structs of integers, for which all zeros is a valid value.
    let mut policy: SYSTEM_POWER_POLICY = unsafe { std::mem::zeroed() };
    let size = u32::try_from(size_of::<SYSTEM_POWER_POLICY>()).expect("a small struct");
    // SAFETY: the out buffer is a live local of exactly `size` bytes; no input buffer.
    let status = unsafe {
        CallNtPowerInformation(
            level,
            None,
            0,
            Some((&raw mut policy).cast::<c_void>()),
            size,
        )
    };
    if status.is_ok() {
        Ok(policy)
    } else {
        Err(format!(
            "CallNtPowerInformation({}) failed with {:#x}",
            level.0, status.0
        ))
    }
}

/// A plan copy this test made, with its values as made (the user's plan's values).
#[derive(Clone, Copy, Debug)]
struct PlanCopy {
    id: Guid,
    name: &'static str,
    original: Values,
}

#[derive(Clone, Debug)]
struct PlanInfo {
    id: Guid,
    name: String,
}

fn powercfg_try(args: &[&str]) -> Result<String, String> {
    let output = Command::new("powercfg")
        .args(args)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("powercfg {} did not start: {e}", args.join(" ")))?;
    let out = String::from_utf8_lossy(&output.stdout).into_owned();
    if output.status.success() {
        return Ok(out);
    }
    Err(format!(
        "powercfg {} failed with {:?}: {} {}",
        args.join(" "),
        output.status.code(),
        out.trim(),
        String::from_utf8_lossy(&output.stderr).trim()
    ))
}

fn powercfg(args: &[&str]) -> String {
    powercfg_try(args).unwrap_or_else(|e| panic!("{e}"))
}

/// `Power Scheme GUID: <id>  (<name>)`, with ` *` after it for the active plan in `/l`.
fn scheme_line(line: &str) -> Option<PlanInfo> {
    let rest = line.trim().strip_prefix("Power Scheme GUID:")?.trim_start();
    let (id, rest) = rest.split_at_checked(36)?;
    let name = rest
        .trim()
        .trim_end_matches('*')
        .trim_end()
        .strip_prefix('(')?
        .strip_suffix(')')?;
    Some(PlanInfo {
        id: Guid::from_str(id).ok()?,
        name: name.to_owned(),
    })
}

fn try_active_plan() -> Result<Guid, String> {
    let text = powercfg_try(&["/getactivescheme"])?;
    text.lines()
        .find_map(scheme_line)
        .map(|p| p.id)
        .ok_or_else(|| format!("no plan in powercfg /getactivescheme: {text}"))
}

fn active_plan() -> Guid {
    try_active_plan().unwrap_or_else(|e| panic!("{e}"))
}

fn try_list_plans() -> Result<Vec<PlanInfo>, String> {
    Ok(powercfg_try(&["/l"])?
        .lines()
        .filter_map(scheme_line)
        .collect())
}

fn list_plans() -> Vec<PlanInfo> {
    try_list_plans().unwrap_or_else(|e| panic!("{e}"))
}

fn try_read_values(plan: Guid) -> Result<Values, String> {
    let (lid_ac, lid_dc) = setting(plan, SUB_BUTTONS, LID)?;
    let (sleep_ac, sleep_dc) = setting(plan, SUB_SLEEP, SLEEP)?;
    let (saver_ac, saver_dc) = setting(plan, SUB_SAVER, SAVER)?;
    Ok(Values {
        lid_ac,
        lid_dc,
        sleep_ac,
        sleep_dc,
        saver_ac,
        saver_dc,
    })
}

fn read_values(plan: Guid) -> Values {
    try_read_values(plan).unwrap_or_else(|e| panic!("{e}"))
}

/// One setting's AC and DC index from `powercfg /qh <plan> <subgroup> <setting>`.
fn setting(plan: Guid, subgroup: &str, id: &str) -> Result<(u32, u32), String> {
    let plan = plan.to_string();
    let text = powercfg_try(&["/qh", &plan, subgroup, id])?;
    let index = |source: &str| {
        let prefix = format!("Current {source} Power Setting Index:");
        let found = text
            .lines()
            .find_map(|l| l.trim().strip_prefix(prefix.as_str()))
            .ok_or_else(|| format!("powercfg /qh {plan} {id}: no {source} index in {text}"))?;
        let hex = found.trim();
        u32::from_str_radix(hex.strip_prefix("0x").unwrap_or(hex), 16)
            .map_err(|_| format!("powercfg /qh {plan} {id}: {hex:?} is not a hex index"))
    };
    Ok((index("AC")?, index("DC")?))
}

/// The user's change, as `powercfg` makes it: stored, then put in force by activating the plan again.
fn set_lid_dc(plan: PlanCopy, value: u32) {
    let id = plan.id.to_string();
    let value = value.to_string();
    powercfg(&["/setdcvalueindex", &id, SUB_BUTTONS, LID, &value]);
    powercfg(&["/setactive", &id]);
}

/// `want` on `plan`, stored, and in force too while it is the active plan.
fn plan_has(plan: PlanCopy, want: Values) -> Result<(), String> {
    let stored = try_read_values(plan.id)?;
    if stored != want {
        return Err(format!("{} stored {stored:?}, want {want:?}", plan.name));
    }
    if try_active_plan()? == plan.id {
        let in_force = InForce::read()?;
        if !in_force.matches(&want) {
            return Err(format!(
                "{} in force {in_force:?}, want the policy of {want:?}",
                plan.name
            ));
        }
    }
    Ok(())
}

/// A journal owned by `tray`, with one entry per key in `keys`, each on `plan` with its original and our 0 in force
/// (not marked).
fn check_journal(
    journal: &JournalFile,
    tray: &Tray,
    plan: PlanCopy,
    keys: &[SettingKey],
) -> Result<(), String> {
    let owner = journal.owner;
    let me = tray.process();
    let session = winproc::current_session().map_err(|e| format!("{e:?}"))?;
    if (owner.pid, owner.proc_start, owner.session) != (me.pid, me.start, session) {
        return Err(format!(
            "journal owner {owner:?}, tray {me:?} in session {session}"
        ));
    }
    let mut found = Vec::new();
    for entry in &journal.entries {
        let JournalKey::Known(key) = &entry.key else {
            return Err(format!("an entry with an unknown key: {entry:?}"));
        };
        let right = entry.plan == plan.id
            && entry.original == plan.original.of(*key)
            && entry.written == 0
            && !entry.restored;
        if !right {
            return Err(format!("entry {entry:?} for {}", plan.name));
        }
        found.push(*key);
    }
    found.sort();
    if found != keys {
        return Err(format!("journal keys {found:?}, want {keys:?}"));
    }
    Ok(())
}

fn try_run_once_lines() -> Result<Vec<String>, String> {
    let output = Command::new("reg")
        .args(["query", RUN_ONCE_KEY])
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("reg query did not start: {e}"))?;
    // A missing key gives exit code 1, and it holds no value either.
    let text = String::from_utf8_lossy(&output.stdout);
    let mut lines: Vec<String> = text
        .lines()
        .filter(|l| l.starts_with("    ") && l.contains("REG_"))
        .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect();
    lines.sort();
    Ok(lines)
}

fn wakeshot_restore() -> Result<Option<String>, String> {
    AutoStart::real()
        .get(StartKey::RunOnce, RUN_ONCE_NAME)
        .map_err(|e| format!("RunOnce {RUN_ONCE_NAME} not read: {e:?}"))
}

/// The exe in `"<exe>" --restore`, the RunOnce value's form (`FileNet::tray`).
fn run_once_exe(value: &str) -> Option<&str> {
    value
        .strip_suffix(" --restore")?
        .strip_prefix('"')?
        .strip_suffix('"')
}

fn names_this_build(value: &str) -> bool {
    run_once_exe(value)
        .is_some_and(|exe| full_path(Path::new(exe)) == full_path(Path::new(TRAY_EXE)))
}

/// This card's whole target folder: `CARGO_TARGET_TMPDIR` is its `tmp` subfolder.
fn points_into_target(value: &str) -> bool {
    let Some(target) = Path::new(env!("CARGO_TARGET_TMPDIR")).parent() else {
        return false;
    };
    let folder = format!("{}\\", full_path(target));
    run_once_exe(value).is_some_and(|exe| full_path(Path::new(exe)).starts_with(&folder))
}

/// `powercfg /requests` (admin) lists a SYSTEM request of this build's tray. It names the exe by its device path,
/// so the path after the drive is compared.
fn system_request_of_this_build() -> Result<bool, String> {
    let text = powercfg_try(&["/requests"])?;
    let exe = full_path(Path::new(TRAY_EXE));
    let tail = exe.split_once(':').map_or(exe.as_str(), |(_, tail)| tail);
    let mut section = "";
    for line in text.lines().map(str::trim) {
        if line.ends_with(':') && !line.contains(' ') {
            section = line;
        } else if section == "SYSTEM:" && line.to_lowercase().ends_with(tail) {
            return Ok(true);
        }
    }
    Ok(false)
}

/// `SendMessageTimeoutW` as Windows sends the end-of-session messages: it waits for the answer. Gives whether it was
/// delivered and the answer.
fn send(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> (bool, usize) {
    let mut answer = 0usize;
    // SAFETY: Windows checks the window handle; the answer goes to a live local.
    let sent = unsafe {
        SendMessageTimeoutW(
            hwnd,
            msg,
            wparam,
            lparam,
            SMTO_ABORTIFHUNG,
            10_000,
            Some(&raw mut answer),
        )
    };
    (sent.0 != 0, answer)
}

/// Waits for a guard to end by itself and gives its exit code.
fn wait_gone(guard: &ProcessHandle, limit: Duration) -> Option<u32> {
    let started = Instant::now();
    let code = exit_code_within(guard, limit);
    assert!(
        code.is_some(),
        "guard pid {} still runs {limit:?} later",
        guard.process().pid
    );
    println!(
        "guard pid {} gone after {} ms",
        guard.process().pid,
        started.elapsed().as_millis()
    );
    code
}

/// Every process of this build that is not `tray`: its guards.
fn guards(tray: u32) -> Vec<ProcessHandle> {
    trays_of_this_build()
        .into_iter()
        .filter(|h| h.process().pid != tray)
        .collect()
}

/// Ends every process of this build: a tray quits first, so it restores and stops its guard; a guard then ends by
/// itself once its tray is gone and its restore ran; whatever still runs after that is ended by pid. Never panics.
fn end_this_build(notes: &mut Vec<String>) {
    if let Some((_, pid)) = loop_window(false)
        && let Some(tray) = trays_of_this_build()
            .into_iter()
            .find(|h| h.process().pid == pid)
    {
        let _ = ask_quit(false);
        match exit_code_within(&tray, EXIT_WITHIN) {
            Some(code) => notes.push(format!("tray pid {pid} quit with {code}")),
            None => {
                terminate(&tray);
                notes.push(format!("tray pid {pid} did not quit; ended by pid"));
            }
        }
    }
    let started = Instant::now();
    loop {
        let left = trays_of_this_build();
        if left.is_empty() {
            return;
        }
        if started.elapsed() > GUARD_WITHIN + Duration::from_secs(5) {
            for process in &left {
                terminate(process);
                notes.push(format!(
                    "pid {} of this build still ran; ended by pid",
                    process.process().pid
                ));
            }
            return;
        }
        thread::sleep(Duration::from_millis(200));
    }
}

/// A fresh plan id: random enough that no plan has it, so a copy made with it is this card's.
fn fresh_guid() -> Guid {
    static N: AtomicU32 = AtomicU32::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let n = u128::from(N.fetch_add(1, Ordering::Relaxed));
    let seed = nanos ^ (u128::from(std::process::id()) << 64) ^ (n << 100);
    let folded = seed as u64 ^ (seed >> 64) as u64;
    let high = u128::from(mix(folded));
    let low = u128::from(mix(folded ^ 0x5851_f42d_4c95_7f2d));
    let mut value = (high << 64) | low;
    // Version 4, variant 10, as a random GUID has.
    value = (value & !(0xf << 76)) | (0x4 << 76);
    value = (value & !(0x3 << 62)) | (0x2 << 62);
    Guid(value)
}

/// SplitMix64's finalizer.
fn mix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9e37_79b9_7f4a_7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// The real-run data folders under `%TEMP%`; a run that was cut off leaves its own.
fn temp_folders() -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(env::temp_dir()) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with(REAL_PREFIX))
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect()
}

fn made_ids(folder: &Path) -> Vec<Guid> {
    fs::read_to_string(folder.join(MADE_FILE))
        .unwrap_or_default()
        .lines()
        .filter_map(|l| Guid::from_str(l.trim()).ok())
        .collect()
}

fn keep_awake_only(state: &StateFile) -> bool {
    state.holding == [text::keep_awake_group(KeepAwake::UntilOff)]
}

/// One test's world: the BEFORE record, a real temp data folder, and the plan copies it made, the first one active.
/// The test ends it with `finish`, which asserts the PC is as recorded; a failed test's drop does the same clean-up
/// and prints what it found. The data folder goes last, with the log printed (`TempData`'s drop).
struct Bench {
    before: Before,
    copies: Vec<PlanCopy>,
    /// Every id handed to `powercfg /duplicatescheme`, named or not.
    made: Vec<Guid>,
    finished: bool,
    data: TempData,
}

impl Bench {
    fn new(tag: &str, names: &[&'static str]) -> Self {
        let before = Before::load();
        let unclean = before.unclean();
        assert!(
            unclean.is_empty(),
            "the PC is not as recorded; run real_e2e_cleanup first:\n{}",
            unclean.join("\n")
        );
        let mut bench = Self {
            before,
            copies: Vec::new(),
            made: Vec::new(),
            finished: false,
            data: TempData::real(tag),
        };
        bench.make_copies(names);
        let first = bench.copy(0);
        powercfg(&["/setactive", &first.id.to_string()]);
        assert_eq!(active_plan(), first.id, "{} is not active", first.name);
        println!(
            "{} active ({}), data folder {}",
            first.name,
            first.id,
            bench.data.root().display()
        );
        bench
    }

    /// The copies of the user's plan, made at Medium as a user could (research, "powercfg as a normal user").
    fn make_copies(&mut self, names: &[&'static str]) {
        let user = self.before.active;
        let ids: Vec<Guid> = names.iter().map(|_| fresh_guid()).collect();
        self.made.clone_from(&ids);
        let root = self.data.root();
        let list: String = ids.iter().map(|id| format!("{id}\n")).collect();
        fs::write(root.join(MADE_FILE), list).expect("write the copy ids");
        let out = root.join("copies-out.txt");
        let temp = root.join("copies-out.tmp");
        let mut tail = Vec::new();
        for (id, name) in ids.iter().zip(names) {
            for command in [
                format!("powercfg /duplicatescheme {user} {id}"),
                format!("powercfg /changename {id} \"{name}\""),
            ] {
                tail.push(format!("{command} >> \"{}\" 2>&1", temp.display()));
                tail.push(format!(
                    "echo [exit %ERRORLEVEL%] >> \"{}\"",
                    temp.display()
                ));
            }
        }
        // Written under a temp name and moved, so the test never reads half of it.
        tail.push(format!(
            "move /y \"{}\" \"{}\" > nul",
            temp.display(),
            out.display()
        ));
        let groups = self.data.launch(Claude::Empty, &tail, "copies");
        let text = wait_until(MEDIUM_CMD_WITHIN, "the .cmd that makes the copies", || {
            fs::read(&out)
                .ok()
                .map(|b| String::from_utf8_lossy(&b).into_owned())
        });
        assert_medium(&groups);
        // `powercfg /duplicatescheme` ends its line without a newline, so a marker may share a line.
        let codes: Vec<&str> = text
            .split("[exit ")
            .skip(1)
            .filter_map(|rest| rest.split_once(']').map(|(code, _)| code))
            .collect();
        assert_eq!(codes, vec!["0"; 2 * names.len()], "{text}");
        let plans = list_plans();
        for (id, name) in ids.iter().zip(names) {
            assert!(
                plans.iter().any(|p| p.id == *id && p.name == *name),
                "{name} ({id}) is not listed: {plans:?}"
            );
            let original = read_values(*id);
            assert_eq!(
                original, self.before.values,
                "{name} is not a copy of the user's values"
            );
            self.copies.push(PlanCopy {
                id: *id,
                name,
                original,
            });
        }
        let a = self.before.values;
        assert!(
            a.lid_ac != 0 && a.lid_dc != 0 && a.sleep_dc != 0,
            "the user's values already are Wakeshot's targets, so a write would not show: {a:?}"
        );
    }

    fn copy(&self, at: usize) -> PlanCopy {
        self.copies[at]
    }

    /// The real tray at Medium on this bench's folder, on an empty Claude folder. Refuses while the active plan is
    /// not one of this bench's copies, since a holding tray applies to the active plan.
    fn start_tray(&self) -> Tray {
        let active = active_plan();
        assert!(
            self.copies.iter().any(|c| c.id == active),
            "the active plan {active} is not a copy of this test"
        );
        let tray = Tray::start(&self.data, Claude::Empty);
        let line = self
            .data
            .wait_log(LogMark::START, "starts: tray", FOLLOW_WITHIN);
        println!("{line}");
        tray
    }

    /// `want` on `plan`, no journal and no RunOnce value: what every restore leaves.
    fn restored(&self, plan: PlanCopy, want: Values) -> Result<(), String> {
        plan_has(plan, want)?;
        if let Some(journal) = self.data.journal()? {
            return Err(format!("journal left: {journal:?}"));
        }
        if let Some(value) = wakeshot_restore()? {
            return Err(format!("RunOnce {RUN_ONCE_NAME} left: {value}"));
        }
        Ok(())
    }

    /// Keep awake until off through the CLI. Within 10 s: Wakeshot's three values on `plan`, stored and in force; a
    /// journal of three entries owned by `tray`; the RunOnce value naming this build; one guard, the one the log
    /// names. Gives the guard.
    fn hold(&self, tray: &Tray, plan: PlanCopy) -> ProcessHandle {
        let started = Instant::now();
        let said = self.data.cli("keep-awake on");
        assert_eq!(said.code, 0, "{said:?}");
        self.data
            .wait_state(APPLY_WITHIN, "Keep awake until off", keep_awake_only);
        let all = [SettingKey::LidAc, SettingKey::LidDc, SettingKey::SleepDc];
        let left = APPLY_WITHIN.saturating_sub(started.elapsed());
        wait_for(left, &format!("Wakeshot's values on {}", plan.name), || {
            plan_has(plan, plan.original.applied())?;
            let journal = self.data.journal()?.ok_or("no journal")?;
            check_journal(&journal, tray, plan, &all)
        });
        println!(
            "applied to {} after {} ms",
            plan.name,
            started.elapsed().as_millis()
        );
        let run_once = wakeshot_restore().expect("read the RunOnce value");
        assert!(
            run_once.as_deref().is_some_and(names_this_build),
            "RunOnce {RUN_ONCE_NAME}: {run_once:?}"
        );
        self.one_guard(tray)
    }

    /// The one guard of `tray`, the one the log names. A guard told to stop may still be on its way out, so this
    /// waits until only one runs.
    fn one_guard(&self, tray: &Tray) -> ProcessHandle {
        let guard = wait_until(FOLLOW_WITHIN, "one guard", || {
            let mut list = guards(tray.pid());
            (list.len() == 1).then(|| list.remove(0))
        });
        let line = format!(
            "guard pid {} started for tray pid {}",
            guard.process().pid,
            tray.pid()
        );
        assert!(self.data.log().contains(&line), "no log line {line:?}");
        guard
    }

    /// Keep awake off through the CLI: `want` on `plan` (stored and in force), no journal and no RunOnce value within
    /// 10 s, and the guard gone within 15 s.
    fn release(&self, plan: PlanCopy, want: Values, guard: &ProcessHandle) {
        let started = Instant::now();
        let said = self.data.cli("keep-awake off");
        assert_eq!(said.code, 0, "{said:?}");
        self.data
            .wait_state(FOLLOW_WITHIN, "idle after off", |s| s.holding.is_empty());
        wait_for(FOLLOW_WITHIN, &format!("{} restored", plan.name), || {
            self.restored(plan, want)
        });
        println!(
            "{} restored after {} ms",
            plan.name,
            started.elapsed().as_millis()
        );
        let code = wait_gone(guard, GUARD_WITHIN.saturating_sub(started.elapsed()));
        println!("guard ended with {code:?}");
    }

    /// The test's end: the clean-up, then the PC must be as recorded.
    fn finish(mut self) {
        self.finished = true;
        let problems = self.teardown();
        assert!(
            problems.is_empty(),
            "after the test:\n{}",
            problems.join("\n")
        );
    }

    /// Every process of this build ends before the user's plan is active again; then the copies and a RunOnce value
    /// of this build go. Gives what was not as it should be. Never panics: a drop calls it.
    fn teardown(&mut self) -> Vec<String> {
        let mut problems = Vec::new();
        end_this_build(&mut problems);
        match try_active_plan() {
            Ok(active) if active == self.before.active => {}
            Ok(active) if self.made.contains(&active) => {
                if let Err(e) = powercfg_try(&["/setactive", &self.before.active.to_string()]) {
                    problems.push(e);
                }
            }
            Ok(active) => problems.push(format!(
                "the active plan {active} is not this test's copy: left alone"
            )),
            Err(e) => problems.push(e),
        }
        let listed: Vec<Guid> = try_list_plans()
            .unwrap_or_default()
            .iter()
            .map(|p| p.id)
            .collect();
        for id in self.made.iter().filter(|id| listed.contains(id)) {
            if let Err(e) = powercfg_try(&["/delete", &id.to_string()]) {
                problems.push(e);
            }
        }
        match wakeshot_restore() {
            Ok(Some(value)) if points_into_target(&value) => {
                let removed = AutoStart::real().remove(StartKey::RunOnce, RUN_ONCE_NAME);
                problems.push(format!(
                    "RunOnce value {value} was left; removed: {removed:?}"
                ));
            }
            Ok(_) => {}
            Err(e) => problems.push(e),
        }
        problems.extend(self.before.differences());
        problems
    }
}

impl Drop for Bench {
    fn drop(&mut self) {
        if self.finished {
            return;
        }
        for problem in self.teardown() {
            eprintln!("clean-up after a failed test: {problem}");
        }
    }
}

/// One `HKCU\Control Panel\NotifyIconSettings\<id>` key as `reg query /s` prints it.
#[derive(Clone, Debug)]
struct NotifyEntry {
    id: String,
    /// Name and data; binary data as hex.
    values: Vec<(String, String)>,
}

impl NotifyEntry {
    fn value(&self, name: &str) -> Option<&str> {
        self.values
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, d)| d.as_str())
    }

    /// `IconGuid` holds the product GUID, as the `GUID` struct's bytes or as text.
    fn names_product(&self) -> bool {
        let Some(data) = self.value("IconGuid") else {
            return false;
        };
        let value = IconKind::Product.guid();
        let mut bytes = String::new();
        let data1 = ((value >> 96) as u32).to_le_bytes();
        let data2 = ((value >> 80) as u16).to_le_bytes();
        let data3 = ((value >> 64) as u16).to_le_bytes();
        let data4 = (value as u64).to_be_bytes();
        for b in data1.iter().chain(&data2).chain(&data3).chain(&data4) {
            let _ = write!(bytes, "{b:02X}");
        }
        let data = data.to_uppercase();
        data == bytes || data.contains(&Guid(value).to_string().to_uppercase())
    }

    fn names_this_build(&self) -> bool {
        self.value("ExecutablePath")
            .is_some_and(|p| full_path(Path::new(p)) == full_path(Path::new(TRAY_EXE)))
    }
}

fn notify_entries() -> Vec<NotifyEntry> {
    let output = Command::new("reg")
        .args(["query", NOTIFY_KEY, "/s"])
        .stdin(Stdio::null())
        .output()
        .expect("run reg query");
    let text = String::from_utf8_lossy(&output.stdout);
    let prefix = r"HKEY_CURRENT_USER\Control Panel\NotifyIconSettings\";
    let mut entries: Vec<NotifyEntry> = Vec::new();
    for line in text.lines() {
        if let Some(id) = line.trim().strip_prefix(prefix) {
            entries.push(NotifyEntry {
                id: id.to_owned(),
                values: Vec::new(),
            });
            continue;
        }
        // `    <name>    <REG_ type>    <data>`, the data empty for an empty value.
        let parts: Vec<&str> = line
            .split("    ")
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .collect();
        if let (Some(entry), [name, kind, data @ ..]) = (entries.last_mut(), parts.as_slice())
            && kind.starts_with("REG_")
        {
            entry.values.push(((*name).to_owned(), data.join(" ")));
        }
    }
    entries
}
