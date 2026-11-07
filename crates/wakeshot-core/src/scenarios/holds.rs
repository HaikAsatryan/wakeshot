//! Hold scenarios over `Core`.
//!
//! Owner: `p2-core-holds`.
//!
//! Every expected word comes from `present` or `text`, never a literal, so `p5-text` can change the wording freely.

use proptest::collection::vec;
use proptest::prelude::*;
use proptest::test_runner::Config as ProptestConfig;

use crate::command::{Command, Failure, FileKind, Outcome};
use crate::event::Event;
use crate::files::{Config, HoldFile};
use crate::holds::file::format_expires;
use crate::holds::{HoldFacts, HoldId, HoldKind};
use crate::policy::requests::Requests;
use crate::present::{self, Holder, Snapshot};
use crate::process::{Alive, FileTime};
use crate::settings::{Guid, Journal, JournalEntry, JournalOwner, OwnedKey, SettingKey};
use crate::sim::{self, KEEP_AWAKE, SAFETY_TICK_MS, START, Sim, Start, TRAY, claude, hold};
use crate::sources::{Activity, FormatHealth, SessionFacts, SessionKey, SourceId};
use crate::view::{IconState, KeepAwake, MenuCommand, MenuItem, Notice, TrayView};

const SECOND: u64 = 1_000;
const MINUTE: u64 = 60 * SECOND;
const HOUR: u64 = 60 * MINUTE;

/// Not a multiple of the 30 s safety tick, so an end checked to the millisecond is the core's deadline at work.
const OFF_THE_TICK: u64 = 7 * SECOND;

const OFF: Command = Command::Requests {
    system: false,
    display: false,
};
const SYSTEM: Command = Command::Requests {
    system: true,
    display: false,
};
const SCREEN: Command = Command::Requests {
    system: true,
    display: true,
};

const FAILED: Failure = Failure {
    call: "PowerSetRequest",
    code: 5,
};

fn ms(n: u64) -> i64 {
    i64::try_from(n).unwrap()
}

fn started() -> Sim {
    let mut sim = Sim::new();
    sim.start();
    sim
}

/// Started, then 7 s later, so the times a test checks lie between safety ticks: only the core's own deadline can
/// make a change land on them exactly.
fn started_off_the_tick() -> Sim {
    let mut sim = started();
    sim.run(OFF_THE_TICK);
    sim
}

fn working(pid: u32) -> Event {
    sim::sources(vec![(claude(pid, Activity::Working), Alive::Yes)])
}

fn idle(pid: u32) -> Event {
    sim::sources(vec![(claude(pid, Activity::Idle), Alive::Yes)])
}

fn keep_awake_id() -> HoldId {
    HoldId(KEEP_AWAKE.to_owned())
}

fn reason(name: &str, reason: &str) -> HoldFacts {
    HoldFacts {
        reason: Some(reason.to_owned()),
        ..hold(name, Alive::Yes)
    }
}

fn count(commands: &[Command], wanted: &Command) -> usize {
    commands.iter().filter(|c| *c == wanted).count()
}

fn logs(commands: &[Command]) -> usize {
    commands
        .iter()
        .filter(|c| matches!(c, Command::Log(_)))
        .count()
}

fn view(sim: &Sim) -> &TrayView {
    sim.view()
        .unwrap_or_else(|| panic!("no view yet\n{}", sim.trace()))
}

/// The Keep awake rows the menu checks.
fn checked_keep_awake(view: &TrayView) -> Vec<KeepAwake> {
    view.menu
        .items
        .iter()
        .filter_map(|item| match item {
            MenuItem::Submenu { items, .. } => Some(items),
            _ => None,
        })
        .flatten()
        .filter_map(|item| match item {
            MenuItem::Action {
                command: MenuCommand::KeepAwake(choice),
                checked: true,
                ..
            } => Some(*choice),
            _ => None,
        })
        .collect()
}

/// The view `present` builds for these holds, with the rest as a plain run with Automatic on.
fn expected(sim: &Sim, holds: Vec<Holder>, keep_awake: KeepAwake) -> TrayView {
    let snapshot = Snapshot {
        automatic: true,
        requests_on: !holds.is_empty(),
        holds,
        keep_awake,
        screen_on: false,
        requests_only: false,
        stepped_back: false,
        overridden: Vec::new(),
        tray: TRAY,
    };
    present::build(&snapshot, sim.now()).0
}

fn agent() -> Holder {
    Holder {
        kind: HoldKind::Agent,
        label: Some("Claude Code".to_owned()),
        expires_wall_ms: None,
    }
}

#[test]
fn starts_without_a_config_file() {
    let mut sim = Sim::new();
    let out = sim.send(
        Start {
            first_run: true,
            ..Start::default()
        }
        .event(),
    );
    let trace = sim.trace();
    assert!(out.contains(&Command::Notify(Notice::FirstRun)), "{trace}");
    assert!(
        out.contains(&Command::WriteConfig(Config::default())),
        "{trace}"
    );
    assert_eq!(count(&out, &OFF), 0, "nothing to release at start\n{trace}");
    assert_eq!(view(&sim), &expected(&sim, vec![], KeepAwake::Off));
    let state = sim.state_file().expect("state.json at start");
    assert_eq!((state.pid, state.proc_start), (TRAY.pid, TRAY.start));
    assert!(state.automatic && state.holding.is_empty() && !state.stepped_back);
}

#[test]
fn starts_with_a_config_file_and_its_config_wins() {
    let mut sim = Sim::with(Config::new(true), START);
    let out = sim.send(
        Start {
            config: Config::new(false),
            ..Start::default()
        }
        .event(),
    );
    let trace = sim.trace();
    assert!(!out.contains(&Command::Notify(Notice::FirstRun)), "{trace}");
    assert!(
        !out.iter().any(|c| matches!(c, Command::WriteConfig(_))),
        "{trace}"
    );
    assert_eq!(view(&sim).icon, IconState::Off, "{trace}");
    assert!(!sim.state_file().unwrap().automatic);
}

#[test]
fn a_journal_at_start_shows_as_overridden() {
    let plan = Guid(0x381b4222_f694_41f0_9685_ff5bb260df2e);
    let journal = Journal {
        owner: JournalOwner {
            pid: 4100,
            proc_start: FileTime(7),
            session: 1,
        },
        entries: vec![JournalEntry {
            plan,
            key: SettingKey::LidDc,
            original: 1,
            written: 0,
            restored: false,
        }],
    };
    let mut sim = Sim::new();
    sim.send(
        Start {
            journal: Some(journal),
            ..Start::default()
        }
        .event(),
    );
    let state = sim.state_file().expect("state.json at start");
    assert_eq!(
        state.overridden,
        [OwnedKey {
            plan,
            key: SettingKey::LidDc
        }]
    );
}

#[test]
fn events_before_started_are_dropped() {
    let mut sim = Sim::new();
    assert!(sim.send(working(1)).is_empty());
    assert!(sim.send(Event::Tick).is_empty());
    assert_eq!(sim.deadline(), None);
    sim.start();
    assert_eq!(sim.requests(), Requests::NONE, "{}", sim.trace());
}

#[test]
fn a_second_started_changes_nothing() {
    let mut sim = started();
    sim.send(working(1));
    let out = sim.send(
        Start {
            config: Config::new(false),
            first_run: true,
            ..Start::default()
        }
        .event(),
    );
    assert!(out.is_empty(), "{}", sim.trace());
}

#[test]
fn busy_session_holds_and_ends() {
    let mut sim = started();
    let out = sim.send(working(20636));
    let trace = sim.trace();
    assert_eq!(count(&out, &SYSTEM), 1, "{trace}");
    assert_eq!(
        logs(&out),
        3,
        "the session count, the session's line and the hold start\n{trace}"
    );
    assert_eq!(view(&sim), &expected(&sim, vec![agent()], KeepAwake::Off));

    let out = sim.run(HOUR);
    assert!(
        !out.iter().any(|c| matches!(c, Command::Requests { .. })),
        "{}",
        sim.trace()
    );

    let out = sim.send(idle(20636));
    let trace = sim.trace();
    assert_eq!(count(&out, &OFF), 1, "{trace}");
    assert_eq!(
        logs(&out),
        2,
        "the session's line and the hold end\n{trace}"
    );
    assert_eq!(view(&sim).icon, IconState::Idle, "{trace}");
}

fn log_lines(commands: &[Command]) -> Vec<&str> {
    commands
        .iter()
        .filter_map(|c| match c {
            Command::Log(text) => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

/// A Claude Code session with the facts of another agent whose id is no pid, as Codex's thread ids are.
fn other_agent(id: &str, activity: Activity) -> SessionFacts {
    let mut facts = claude(9, activity);
    facts.key.id = id.to_owned();
    facts.label = "Codex";
    facts
}

/// One line per session at each start and end of its hold, with a short id and the reason (architecture section 8),
/// before the lines that sum up every hold; a second session of the same agent gets its own lines too.
#[test]
fn each_session_gets_its_own_start_and_end_line() {
    let mut sim = started();
    let both = |a: Activity, b: Activity, alive_b: Alive| {
        sim::sources(vec![
            (claude(4100, a), Alive::Yes),
            (claude(4200, b), alive_b),
        ])
    };
    let out = sim.send(both(Activity::Working, Activity::Idle, Alive::Yes));
    assert_eq!(
        log_lines(&out),
        [
            "Claude Code: 2 sessions in its files",
            "Claude Code session 4100 holds: working",
            "Holding starts: Claude Code"
        ],
        "{}",
        sim.trace()
    );
    let out = sim.send(both(Activity::Working, Activity::Working, Alive::Yes));
    assert_eq!(
        log_lines(&out),
        ["Claude Code session 4200 holds: working"],
        "the summed-up holds did not change\n{}",
        sim.trace()
    );
    let waiting = Activity::Waiting {
        since_wall_ms: None,
        remote: false,
    };
    let out = sim.send(both(waiting, Activity::Working, Alive::Yes));
    assert_eq!(
        log_lines(&out),
        ["Claude Code session 4100 stops holding: waiting for the user"],
        "{}",
        sim.trace()
    );
    let out = sim.send(both(waiting, Activity::Working, Alive::Gone));
    assert_eq!(
        log_lines(&out),
        [
            "Claude Code session 4200 stops holding: its process ended",
            "Holding ends; held by Claude Code"
        ],
        "{}",
        sim.trace()
    );

    // An id that is not a pid shows as a hash: never the id, which can be a path with the user's name.
    let id = r"C:\Users\someone\.codex\sessions\2026\11\07\rollout-019a.jsonl";
    let out = sim.send(sim::sources(vec![(
        other_agent(id, Activity::Working),
        Alive::Yes,
    )]));
    let lines = log_lines(&out);
    assert_eq!(
        lines.first().copied(),
        Some("Codex: 1 session in its files")
    );
    let first = lines.get(1).copied().unwrap_or_default();
    let hash = first
        .strip_prefix("Codex session ")
        .and_then(|rest| rest.strip_suffix(" holds: working"))
        .unwrap_or_else(|| panic!("{lines:?}"));
    assert!(
        hash.len() == 8 && hash.bytes().all(|b| b.is_ascii_hexdigit()),
        "{first}"
    );
    assert!(
        !lines
            .iter()
            .any(|l| l.contains("someone") || l.contains("Users")),
        "{lines:?}"
    );
    // The session leaves the snapshot while it holds: its end line, with the same short id.
    let out = sim.send(sim::sources(vec![]));
    let lines = log_lines(&out);
    let gone = format!("Codex session {hash} stops holding: it is gone from the agent's files");
    assert_eq!(
        lines[..2],
        ["Codex: 0 sessions in its files", gone.as_str()],
        "{lines:?}"
    );

    // Hold files: the bare file name and why.
    let out = sim.send(Event::Holds(vec![hold("run-4242.json", Alive::Yes)]));
    assert_eq!(
        log_lines(&out),
        [
            "Hold file run-4242.json counts: its process runs",
            "Holding starts: wakeshot run"
        ],
        "{}",
        sim.trace()
    );
    let out = sim.send(Event::Holds(vec![hold("run-4242.json", Alive::Gone)]));
    assert_eq!(
        log_lines(&out)[..1],
        ["Hold file run-4242.json stops counting: its process ended"],
        "{}",
        sim.trace()
    );

    // Automatic off ends an agent session's hold with that reason.
    sim.send(working(4300));
    let out = sim.send(Event::Menu(MenuCommand::Automatic(false)));
    assert!(
        log_lines(&out).contains(&"Claude Code session 4300 stops holding: Automatic is off"),
        "{}",
        sim.trace()
    );
}

#[test]
fn a_session_that_leaves_the_snapshot_ends_its_hold() {
    let mut sim = started();
    sim.send(working(1));
    let out = sim.send(sim::sources(vec![]));
    assert_eq!(count(&out, &OFF), 1, "{}", sim.trace());
}

#[test]
fn timer_window_opens_and_closes_on_time_across_a_suspend_and_a_wall_jump() {
    let mut sim = started();
    let due = START.wall_ms + ms(2 * HOUR + OFF_THE_TICK);
    let session = SessionFacts {
        timers: vec![due],
        ..claude(7, Activity::Idle)
    };
    let out = sim.send(sim::sources(vec![(session, Alive::Yes)]));
    assert_eq!(count(&out, &SYSTEM), 0, "{}", sim.trace());

    sim.run(HOUR + OFF_THE_TICK);
    let opened: Vec<i64> = sim.sent_at(&SYSTEM).iter().map(|t| t.wall_ms).collect();
    assert_eq!(opened, [due - ms(HOUR)], "{}", sim.trace());

    // Asleep 30 minutes inside the window: still holding, and the requests are taken again.
    let out = sim.suspend(30 * MINUTE);
    assert!(out.contains(&Command::Retake), "{}", sim.trace());
    assert_eq!(count(&out, &OFF), 0, "{}", sim.trace());

    // Someone sets the clock 40 minutes ahead, past the window's end (due plus 2 minutes): the next tick ends it,
    // although the loop's timer still waits for the old end in run time.
    sim.set_wall(sim.now().wall_ms + ms(40 * MINUTE));
    let out = sim.run(SAFETY_TICK_MS);
    assert_eq!(count(&out, &OFF), 1, "{}", sim.trace());
}

#[test]
fn timer_window_ends_two_minutes_after_due() {
    let mut sim = started();
    let due = START.wall_ms + ms(10 * MINUTE + OFF_THE_TICK);
    let session = SessionFacts {
        timers: vec![due],
        ..claude(7, Activity::Idle)
    };
    let out = sim.send(sim::sources(vec![(session, Alive::Yes)]));
    assert_eq!(count(&out, &SYSTEM), 1, "inside the hour before due");
    sim.run(13 * MINUTE);
    let ended: Vec<i64> = sim.sent_at(&OFF).iter().map(|t| t.wall_ms).collect();
    assert_eq!(ended, [due + ms(2 * MINUTE)], "{}", sim.trace());
}

#[test]
fn a_sleep_past_the_window_ends_the_hold_at_resume() {
    let mut sim = started();
    let due = START.wall_ms + ms(10 * MINUTE);
    let session = SessionFacts {
        timers: vec![due],
        ..claude(7, Activity::Idle)
    };
    sim.send(sim::sources(vec![(session, Alive::Yes)]));
    let out = sim.suspend(HOUR);
    assert_eq!(count(&out, &OFF), 1, "{}", sim.trace());
    assert!(
        !out.contains(&Command::Retake),
        "no request is left to take again\n{}",
        sim.trace()
    );
}

#[test]
fn waiting_session_with_remote_control_ends_after_one_hour() {
    let mut sim = started_off_the_tick();
    let since = sim.now().wall_ms;
    let waiting = Activity::Waiting {
        since_wall_ms: Some(since),
        remote: true,
    };
    let out = sim.send(sim::sources(vec![(claude(3, waiting), Alive::Yes)]));
    assert_eq!(count(&out, &SYSTEM), 1, "{}", sim.trace());
    let out = sim.run(HOUR - SECOND);
    assert_eq!(count(&out, &OFF), 0, "{}", sim.trace());
    sim.run(SECOND);
    let ended: Vec<i64> = sim.sent_at(&OFF).iter().map(|t| t.wall_ms).collect();
    assert_eq!(ended, [since + ms(HOUR)], "{}", sim.trace());
}

#[test]
fn waiting_without_a_time_counts_from_first_sight() {
    let mut sim = started();
    let waiting = sim::sources(vec![(
        claude(
            3,
            Activity::Waiting {
                since_wall_ms: None,
                remote: true,
            },
        ),
        Alive::Yes,
    )]);
    sim.run(10 * MINUTE + OFF_THE_TICK);
    let first_sight = sim.now().wall_ms;
    sim.send(waiting.clone());
    // Each new snapshot of the same wait keeps the first sight, so the hour does not start again.
    for _ in 0..5 {
        sim.run(10 * MINUTE);
        sim.send(waiting.clone());
    }
    sim.run(10 * MINUTE);
    let ended: Vec<i64> = sim.sent_at(&OFF).iter().map(|t| t.wall_ms).collect();
    assert_eq!(ended, [first_sight + ms(HOUR)], "{}", sim.trace());
}

#[test]
fn waiting_without_remote_control_does_not_hold() {
    let mut sim = started();
    let waiting = Activity::Waiting {
        since_wall_ms: None,
        remote: false,
    };
    let out = sim.send(sim::sources(vec![(claude(3, waiting), Alive::Yes)]));
    assert_eq!(count(&out, &SYSTEM), 0, "{}", sim.trace());
}

#[test]
fn unknown_liveness_keeps_a_hold() {
    let mut sim = started();
    sim.send(working(5));
    let unknown = sim::sources(vec![(claude(5, Activity::Working), Alive::Unknown)]);
    let mut out = sim.send(unknown.clone());
    out.extend(sim.run(5 * MINUTE));
    out.extend(sim.send(unknown));
    assert_eq!(count(&out, &OFF), 0, "{}", sim.trace());
    assert_eq!(
        sim.requests(),
        Requests {
            system: true,
            display: false
        }
    );

    let out = sim.send(sim::sources(vec![(
        claude(5, Activity::Working),
        Alive::Gone,
    )]));
    assert_eq!(count(&out, &OFF), 1, "{}", sim.trace());
}

#[test]
fn gone_hold_file_is_deleted() {
    let mut sim = started();
    let id = HoldId("run-4242.json".to_owned());
    let delete = Command::DeleteHold(id.clone());

    let out = sim.put_hold(hold("run-4242.json", Alive::Gone));
    assert_eq!(count(&out, &delete), 1, "{}", sim.trace());
    assert_eq!(sim.requests(), Requests::NONE, "a Gone file never holds");

    // A snapshot that comes before the delete is answered asks no second time.
    let out = sim.send(Event::Holds(vec![hold("run-4242.json", Alive::Gone)]));
    assert_eq!(count(&out, &delete), 0, "{}", sim.trace());

    // A delete that failed is asked again at the next snapshot that still shows the file.
    sim.send(Event::Done(Outcome::Written {
        file: FileKind::Hold(id),
        result: Err(Failure {
            call: "DeleteFileW",
            code: 32,
        }),
    }));
    let out = sim.send(Event::Holds(vec![hold("run-4242.json", Alive::Gone)]));
    assert_eq!(count(&out, &delete), 1, "{}", sim.trace());

    sim.watch();
    assert_eq!(sim.sent_at(&delete).len(), 2, "{}", sim.trace());
}

#[test]
fn unknown_liveness_never_deletes_a_hold_file() {
    let mut sim = started();
    let out = sim.put_hold(hold("backup.json", Alive::Unknown));
    assert!(
        !out.iter().any(|c| matches!(c, Command::DeleteHold(_))),
        "{}",
        sim.trace()
    );
    assert_eq!(
        count(&out, &SYSTEM),
        0,
        "Unknown at first sight holds nothing"
    );
}

#[test]
fn a_hold_file_holds_until_its_process_is_gone() {
    let mut sim = started();
    let out = sim.put_hold(reason("run-4242.json", "cargo"));
    assert_eq!(count(&out, &SYSTEM), 1, "{}", sim.trace());
    let out = sim.put_hold(hold("run-4242.json", Alive::Gone));
    let trace = sim.trace();
    assert_eq!(count(&out, &OFF), 1, "{trace}");
    assert_eq!(
        count(&out, &Command::DeleteHold(HoldId("run-4242.json".into()))),
        1,
        "{trace}"
    );
}

#[test]
fn a_hold_file_stops_at_expires() {
    let mut sim = started();
    let expires = START.wall_ms + ms(90 * SECOND + OFF_THE_TICK);
    sim.put_hold(HoldFacts {
        expires_wall_ms: Some(expires),
        ..reason("backup.json", "nightly backup")
    });
    sim.run(2 * MINUTE);
    let ended: Vec<i64> = sim.sent_at(&OFF).iter().map(|t| t.wall_ms).collect();
    assert_eq!(ended, [expires], "{}", sim.trace());
}

#[test]
fn automatic_off_keeps_only_keep_awake() {
    let mut sim = started();
    sim.send(working(1));
    sim.put_hold(reason("backup.json", "nightly backup"));
    sim.send(Event::Menu(MenuCommand::KeepAwake(KeepAwake::UntilOff)));
    sim.watch();
    sim.run(SECOND);
    assert_eq!(
        sim.state_file().unwrap().holding.len(),
        3,
        "{}",
        sim.trace()
    );

    let out = sim.send(Event::Menu(MenuCommand::Automatic(false)));
    let trace = sim.trace();
    assert!(
        out.contains(&Command::WriteConfig(Config::new(false))),
        "{trace}"
    );
    assert_eq!(count(&out, &OFF), 0, "Keep awake still holds\n{trace}");
    let keep_awake = Holder {
        kind: HoldKind::KeepAwake,
        label: None,
        expires_wall_ms: None,
    };
    let want = present::build(
        &Snapshot {
            automatic: false,
            holds: vec![keep_awake],
            keep_awake: KeepAwake::UntilOff,
            screen_on: false,
            requests_on: true,
            requests_only: false,
            stepped_back: false,
            overridden: Vec::new(),
            tray: TRAY,
        },
        sim.now(),
    )
    .0;
    assert_eq!(view(&sim), &want, "{trace}");

    let mut out = sim.send(Event::Menu(MenuCommand::KeepAwake(KeepAwake::Off)));
    assert!(out.contains(&Command::DeleteHold(keep_awake_id())));
    out.extend(sim.watch());
    assert_eq!(count(&out, &OFF), 1, "{}", sim.trace());
    assert_eq!(view(&sim).icon, IconState::Off, "{}", sim.trace());

    // Turned on again, the agent and the script count again.
    let out = sim.send(Event::Menu(MenuCommand::Automatic(true)));
    assert_eq!(count(&out, &SYSTEM), 1, "{}", sim.trace());
}

#[test]
fn automatic_from_the_config_file_writes_no_config() {
    let mut sim = started();
    sim.send(working(1));
    let out = sim.send(Event::ConfigChanged(Config::new(false)));
    let trace = sim.trace();
    assert!(
        !out.iter().any(|c| matches!(c, Command::WriteConfig(_))),
        "{trace}"
    );
    assert_eq!(count(&out, &OFF), 1, "{trace}");
    assert_eq!(view(&sim).icon, IconState::Off, "{trace}");
}

#[test]
fn keep_awake_30_minutes_ends_on_time() {
    let mut sim = started_off_the_tick();
    let end = sim.now().wall_ms + ms(30 * MINUTE);
    let out = sim.send(Event::Menu(MenuCommand::KeepAwake(KeepAwake::Minutes(30))));
    let written = Command::WriteHold {
        id: keep_awake_id(),
        hold: HoldFile {
            expires: Some(format_expires(end)),
            ..HoldFile::new(TRAY.pid, Some(TRAY.start))
        },
    };
    assert_eq!(count(&out, &written), 1, "{}", sim.trace());

    let out = sim.watch();
    assert_eq!(count(&out, &SYSTEM), 1, "{}", sim.trace());
    assert_eq!(checked_keep_awake(view(&sim)), [KeepAwake::Minutes(30)]);

    // The menu keeps checking the length picked while the minutes left run down.
    let out = sim.run(29 * MINUTE);
    assert_eq!(count(&out, &OFF), 0, "{}", sim.trace());
    assert_eq!(checked_keep_awake(view(&sim)), [KeepAwake::Minutes(30)]);
    let keep_awake = Holder {
        kind: HoldKind::KeepAwake,
        label: None,
        expires_wall_ms: Some(end),
    };
    assert_eq!(
        view(&sim),
        &expected(&sim, vec![keep_awake], KeepAwake::Minutes(30)),
        "the tooltip shows the minute left\n{}",
        sim.trace()
    );

    sim.run(MINUTE);
    let ended: Vec<i64> = sim.sent_at(&OFF).iter().map(|t| t.wall_ms).collect();
    assert_eq!(ended, [end], "{}", sim.trace());
    assert_eq!(checked_keep_awake(view(&sim)), [KeepAwake::Off]);
}

#[test]
fn keep_awake_picked_again_starts_its_time_again() {
    let mut sim = started();
    sim.send(Event::Menu(MenuCommand::KeepAwake(KeepAwake::Minutes(30))));
    sim.watch();
    sim.run(20 * MINUTE);
    sim.send(Event::Menu(MenuCommand::KeepAwake(KeepAwake::Minutes(60))));
    sim.watch();
    assert_eq!(checked_keep_awake(view(&sim)), [KeepAwake::Minutes(60)]);
    let out = sim.run(59 * MINUTE);
    assert_eq!(count(&out, &OFF), 0, "{}", sim.trace());
    assert_eq!(checked_keep_awake(view(&sim)), [KeepAwake::Minutes(60)]);
    let out = sim.run(MINUTE);
    assert_eq!(count(&out, &OFF), 1, "{}", sim.trace());
}

#[test]
fn keep_awake_from_the_cli_checks_the_length_that_covers_it() {
    let mut sim = started();
    // `wakeshot keep-awake 30m` wrote it a few minutes ago, naming this tray.
    let hold = HoldFile {
        expires: Some(format_expires(START.wall_ms + ms(25 * MINUTE))),
        ..HoldFile::new(TRAY.pid, Some(TRAY.start))
    };
    sim.put_hold(sim::watched(&keep_awake_id(), &hold, Alive::Yes));
    assert_eq!(checked_keep_awake(view(&sim)), [KeepAwake::Minutes(30)]);
    sim.run(20 * MINUTE);
    assert_eq!(checked_keep_awake(view(&sim)), [KeepAwake::Minutes(30)]);
}

#[test]
fn keep_screen_on_asks_for_the_display() {
    let mut sim = started();
    sim.send(Event::Menu(MenuCommand::KeepAwake(KeepAwake::Minutes(60))));
    let out = sim.watch();
    assert_eq!(count(&out, &SYSTEM), 1, "{}", sim.trace());

    let out = sim.send(Event::Menu(MenuCommand::KeepScreenOn(true)));
    let written = Command::WriteHold {
        id: keep_awake_id(),
        hold: HoldFile {
            expires: Some(format_expires(START.wall_ms + ms(HOUR))),
            screen: true,
            ..HoldFile::new(TRAY.pid, Some(TRAY.start))
        },
    };
    assert_eq!(
        count(&out, &written),
        1,
        "the same end, now with the screen\n{}",
        sim.trace()
    );
    let out = sim.watch();
    assert_eq!(count(&out, &SCREEN), 1, "{}", sim.trace());
    assert_eq!(checked_keep_awake(view(&sim)), [KeepAwake::Minutes(60)]);

    // A new length keeps the screen on.
    let out = sim.send(Event::Menu(MenuCommand::KeepAwake(KeepAwake::UntilOff)));
    assert!(
        out.iter().any(|c| matches!(c, Command::WriteHold { hold, .. } if hold.screen && hold.expires.is_none())),
        "{}",
        sim.trace()
    );
    sim.watch();
    assert_eq!(
        sim.requests(),
        Requests {
            system: true,
            display: true
        }
    );

    let out = sim.send(Event::Menu(MenuCommand::KeepScreenOn(false)));
    assert!(
        out.iter()
            .any(|c| matches!(c, Command::WriteHold { hold, .. } if !hold.screen))
    );
    let out = sim.watch();
    assert_eq!(count(&out, &SYSTEM), 1, "{}", sim.trace());

    // Keep awake Off clears it, and Keep screen on alone does nothing.
    sim.send(Event::Menu(MenuCommand::KeepAwake(KeepAwake::Off)));
    sim.watch();
    let out = sim.send(Event::Menu(MenuCommand::KeepScreenOn(true)));
    assert!(
        !out.iter().any(|c| matches!(c, Command::WriteHold { .. })),
        "{}",
        sim.trace()
    );
    assert_eq!(sim.requests(), Requests::NONE);
}

#[test]
fn keep_awake_off_deletes_the_file_by_the_name_it_has() {
    let mut sim = started();
    let upper = HoldId("KEEP-AWAKE.JSON".to_owned());
    let hold = HoldFile::new(TRAY.pid, Some(TRAY.start));
    sim.put_hold(sim::watched(&upper, &hold, Alive::Yes));
    assert_eq!(
        sim.requests(),
        Requests {
            system: true,
            display: false
        }
    );
    let out = sim.send(Event::Menu(MenuCommand::KeepAwake(KeepAwake::Off)));
    assert_eq!(
        out.iter()
            .filter(|c| matches!(c, Command::DeleteHold(_)))
            .collect::<Vec<_>>(),
        [&Command::DeleteHold(upper)],
        "{}",
        sim.trace()
    );
}

#[test]
fn off_the_console_keep_screen_on_takes_no_display() {
    let mut sim = Sim::new();
    sim.send(
        Start {
            on_console: false,
            ..Start::default()
        }
        .event(),
    );
    sim.send(Event::Menu(MenuCommand::KeepAwake(KeepAwake::UntilOff)));
    sim.watch();
    sim.send(Event::Menu(MenuCommand::KeepScreenOn(true)));
    sim.watch();
    assert_eq!(
        sim.requests(),
        Requests {
            system: true,
            display: false
        },
        "{}",
        sim.trace()
    );
}

#[test]
fn a_keep_awake_file_seen_gone_after_this_tray_wrote_it_is_not_deleted() {
    let mut sim = started();
    sim.send(Event::Menu(MenuCommand::KeepAwake(KeepAwake::UntilOff)));
    // A snapshot taken before the write still shows the dead tray's file.
    let out = sim.send(Event::Holds(vec![hold(KEEP_AWAKE, Alive::Gone)]));
    assert!(
        !out.iter().any(|c| matches!(c, Command::DeleteHold(_))),
        "{}",
        sim.trace()
    );
}

#[test]
fn a_dead_trays_keep_awake_file_is_deleted() {
    let mut sim = started();
    let out = sim.put_hold(hold(KEEP_AWAKE, Alive::Gone));
    assert_eq!(
        count(&out, &Command::DeleteHold(keep_awake_id())),
        1,
        "{}",
        sim.trace()
    );
}

#[test]
fn update_wakeshot_once_per_agent_per_start() {
    let unreadable = SessionFacts {
        format: FormatHealth::Unreadable("status file"),
        ..claude(1, Activity::Idle)
    };
    let claude_notice = Notice::UpdateWakeshot {
        agent: "Claude Code",
    };
    let mut sim = started();
    sim.send(sim::sources(vec![(unreadable.clone(), Alive::Yes)]));
    sim.run(5 * MINUTE);
    sim.send(sim::sources(vec![
        (unreadable.clone(), Alive::Yes),
        (
            SessionFacts {
                format: FormatHealth::UnknownValue("status", "teleporting".into()),
                ..claude(2, Activity::Idle)
            },
            Alive::Yes,
        ),
    ]));
    let codex = SessionFacts {
        key: SessionKey {
            source: SourceId::Codex,
            id: "thread-1".into(),
        },
        label: "Codex",
        format: FormatHealth::UnknownValue("state", "new".into()),
        ..claude(9, Activity::Idle)
    };
    sim.send(Event::Sources {
        source: SourceId::Codex,
        sessions: vec![(codex, Alive::Yes)],
    });
    assert_eq!(
        sim.notices(),
        [&claude_notice, &Notice::UpdateWakeshot { agent: "Codex" }],
        "{}",
        sim.trace()
    );

    // A new start shows it again.
    let mut sim = started();
    sim.send(sim::sources(vec![(unreadable, Alive::Yes)]));
    assert_eq!(sim.notices(), [&claude_notice]);
}

#[test]
fn update_wakeshot_waits_while_automatic_is_off() {
    let mut sim = Sim::new();
    sim.send(
        Start {
            config: Config::new(false),
            ..Start::default()
        }
        .event(),
    );
    sim.send(sim::sources(vec![(
        SessionFacts {
            format: FormatHealth::Unreadable("status file"),
            ..claude(1, Activity::Idle)
        },
        Alive::Yes,
    )]));
    assert!(sim.notices().is_empty(), "{}", sim.trace());
    sim.send(Event::Menu(MenuCommand::Automatic(true)));
    assert_eq!(
        sim.notices(),
        [&Notice::UpdateWakeshot {
            agent: "Claude Code"
        }],
        "{}",
        sim.trace()
    );
}

#[test]
fn unknown_status_counts_as_working_with_the_notice() {
    let mut sim = started();
    let unknown = SessionFacts {
        format: FormatHealth::UnknownValue("status", "teleporting".into()),
        ..claude(1, Activity::Working)
    };
    let out = sim.send(sim::sources(vec![(unknown, Alive::Yes)]));
    assert_eq!(count(&out, &SYSTEM), 1, "{}", sim.trace());
    assert_eq!(
        sim.notices(),
        [&Notice::UpdateWakeshot {
            agent: "Claude Code"
        }]
    );
}

#[test]
fn format_problem_logs_at_most_once_a_minute_per_source() {
    let mut sim = started();
    let unreadable = sim::sources(vec![(
        SessionFacts {
            format: FormatHealth::Unreadable("status file"),
            ..claude(1, Activity::Idle)
        },
        Alive::Yes,
    )]);
    let mut out = Vec::new();
    for _ in 0..12 {
        out.extend(sim.send(unreadable.clone()));
        sim.run(10 * SECOND);
    }
    let problems = log_lines(&out)
        .into_iter()
        .filter(|l| l.contains("format problem"))
        .count();
    assert_eq!(problems, 2, "at 0 s and 60 s\n{}", sim.trace());
    // Besides those, only the one line that counts the session at its first sight.
    assert_eq!(logs(&out), 3, "{}", sim.trace());
}

#[test]
fn retake_after_resume() {
    let mut sim = started();
    sim.send(working(1));
    let out = sim.suspend(HOUR);
    assert_eq!(count(&out, &Command::Retake), 1, "{}", sim.trace());

    sim.send(idle(1));
    let out = sim.suspend(HOUR);
    assert_eq!(count(&out, &Command::Retake), 0, "{}", sim.trace());
}

#[test]
fn failed_request_shows_cannot_hold_once() {
    let mut sim = started();
    sim.send(working(1));
    let out = sim.send(Event::Done(Outcome::Requests(Err(FAILED))));
    assert!(
        out.contains(&Command::Notify(Notice::CannotHold)),
        "{}",
        sim.trace()
    );
    assert_eq!(view(&sim).icon, IconState::Idle, "nothing holds the PC");

    // The same episode: a second failure, and a failed retake after a resume, show nothing more.
    let mut out = sim.send(Event::Done(Outcome::Requests(Err(FAILED))));
    out.extend(sim.suspend(MINUTE));
    out.extend(sim.send(Event::Done(Outcome::Requests(Err(FAILED)))));
    assert!(
        !out.contains(&Command::Notify(Notice::CannotHold)),
        "{}",
        sim.trace()
    );

    // A retake that works shows the hold again.
    sim.suspend(MINUTE);
    sim.send(Event::Done(Outcome::Requests(Ok(()))));
    assert_eq!(view(&sim).icon, IconState::Holding, "{}", sim.trace());

    // A new episode may warn again.
    sim.send(idle(1));
    sim.send(working(1));
    let out = sim.send(Event::Done(Outcome::Requests(Err(FAILED))));
    assert!(
        out.contains(&Command::Notify(Notice::CannotHold)),
        "{}",
        sim.trace()
    );
}

#[test]
fn a_failed_release_shows_no_notice() {
    let mut sim = started();
    sim.send(working(1));
    sim.send(idle(1));
    let out = sim.send(Event::Done(Outcome::Requests(Err(FAILED))));
    assert!(sim.notices().is_empty(), "{}", sim.trace());
    assert!(out.iter().all(|c| !matches!(c, Command::Notify(_))));
    // No notice, but the log shows the request that may still be set.
    assert_eq!(
        log_lines(&out),
        ["Power request change failed (PowerSetRequest, code 5)"]
    );
}

#[test]
fn tooltip_names_what_holds() {
    let mut sim = started();
    sim.send(sim::sources(vec![
        (claude(1, Activity::Working), Alive::Yes),
        (claude(2, Activity::Working), Alive::Yes),
    ]));
    sim.put_hold(reason("run-4242.json", "nightly backup"));
    sim.send(Event::Menu(MenuCommand::KeepAwake(KeepAwake::Minutes(60))));
    sim.watch();
    let holds = vec![
        agent(),
        agent(),
        Holder {
            kind: HoldKind::Run,
            label: Some("nightly backup".to_owned()),
            expires_wall_ms: None,
        },
        Holder {
            kind: HoldKind::KeepAwake,
            label: None,
            expires_wall_ms: Some(START.wall_ms + ms(HOUR)),
        },
    ];
    let snapshot = Snapshot {
        automatic: true,
        holds,
        keep_awake: KeepAwake::Minutes(60),
        screen_on: false,
        requests_on: true,
        requests_only: false,
        stepped_back: false,
        overridden: Vec::new(),
        tray: TRAY,
    };
    let (want_view, want_state) = present::build(&snapshot, sim.now());
    assert_eq!(view(&sim), &want_view, "{}", sim.trace());
    sim.run(SECOND);
    assert_eq!(sim.state_file(), Some(&want_state), "{}", sim.trace());
}

#[test]
fn state_json_writes_at_most_once_a_second() {
    let mut sim = started();
    for i in 0..20 {
        sim.run(100);
        sim.put_hold(reason(&format!("job-{i}.json"), &format!("job {i}")));
    }
    let last_change = sim.now();
    sim.run(5 * SECOND);
    let writes = sim.state_writes();
    let trace = sim.trace();
    assert!(writes.len() >= 2, "{trace}");
    for pair in writes.windows(2) {
        let gap = pair[1].0.mono_ms - pair[0].0.mono_ms;
        assert!(gap >= SECOND, "writes {gap} ms apart\n{trace}");
    }
    let (at, last) = writes.last().unwrap();
    assert_eq!(
        last.holding.len(),
        20,
        "the last write has every change\n{trace}"
    );
    assert!(
        at.mono_ms - last_change.mono_ms <= SECOND,
        "the last change is written within a second\n{trace}"
    );
}

#[test]
fn a_failed_state_write_is_sent_again() {
    let mut sim = started();
    sim.send(Event::Done(Outcome::Written {
        file: FileKind::State,
        result: Err(Failure {
            call: "MoveFileExW",
            code: 5,
        }),
    }));
    sim.run(2 * SECOND);
    assert_eq!(sim.state_writes().len(), 2, "{}", sim.trace());
}

#[test]
fn open_log_opens_the_log() {
    let mut sim = started();
    let out = sim.send(Event::Menu(MenuCommand::OpenLog));
    assert_eq!(out, [Command::OpenLog]);
}

// Random events of this card's kinds, in any order, with time moving in between.

#[derive(Clone, Debug)]
enum Op {
    Send(Event),
    Run(u64),
    Suspend(u64),
    Wall(i64),
}

fn wall() -> impl Strategy<Value = i64> {
    (-ms(3 * HOUR)..ms(3 * HOUR)).prop_map(|d| START.wall_ms + d)
}

fn alive() -> impl Strategy<Value = Alive> {
    prop_oneof![Just(Alive::Yes), Just(Alive::Gone), Just(Alive::Unknown)]
}

fn activity() -> impl Strategy<Value = Activity> {
    prop_oneof![
        Just(Activity::Working),
        Just(Activity::Idle),
        (proptest::option::of(wall()), any::<bool>()).prop_map(|(since_wall_ms, remote)| {
            Activity::Waiting {
                since_wall_ms,
                remote,
            }
        }),
    ]
}

fn format() -> impl Strategy<Value = FormatHealth> {
    prop_oneof![
        4 => Just(FormatHealth::Ok),
        1 => Just(FormatHealth::Unreadable("status file")),
        1 => Just(FormatHealth::UnknownValue("status", "teleporting".into())),
    ]
}

fn session() -> impl Strategy<Value = (SessionFacts, Alive)> {
    (
        0u32..4,
        activity(),
        vec(wall(), 0..3),
        0u32..2,
        format(),
        alive(),
    )
        .prop_map(|(pid, activity, timers, open_tasks, format, alive)| {
            let facts = SessionFacts {
                timers,
                open_tasks,
                format,
                ..claude(pid, activity)
            };
            (facts, alive)
        })
}

fn hold_file() -> impl Strategy<Value = HoldFacts> {
    let names = prop_oneof![
        Just(KEEP_AWAKE),
        Just("run-4242.json"),
        Just("backup.json"),
        Just("KEEP-AWAKE.JSON"),
    ];
    (
        names,
        alive(),
        proptest::option::of(wall()),
        proptest::option::of(Just("nightly backup".to_owned())),
        any::<bool>(),
    )
        .prop_map(|(name, alive, expires_wall_ms, reason, screen)| {
            let base = hold(name, alive);
            HoldFacts {
                expires_wall_ms,
                reason,
                screen: screen && base.kind == HoldKind::KeepAwake,
                ..base
            }
        })
}

fn keep_awake_choice() -> impl Strategy<Value = KeepAwake> {
    prop_oneof![
        Just(KeepAwake::Off),
        (0u32..300).prop_map(KeepAwake::Minutes),
        Just(KeepAwake::UntilOff),
    ]
}

fn written() -> impl Strategy<Value = Outcome> {
    let file = prop_oneof![
        Just(FileKind::State),
        Just(FileKind::Config),
        Just(FileKind::Hold(HoldId(KEEP_AWAKE.to_owned()))),
        Just(FileKind::Hold(HoldId("run-4242.json".to_owned()))),
    ];
    let result = prop_oneof![Just(Ok(())), Just(Err(FAILED))];
    (file, result).prop_map(|(file, result)| Outcome::Written { file, result })
}

fn event() -> impl Strategy<Value = Event> {
    prop_oneof![
        1 => (any::<bool>(), any::<bool>(), any::<bool>()).prop_map(|(automatic, on_console, first_run)| {
            Start {
                config: Config::new(automatic),
                on_console,
                first_run,
                ..Start::default()
            }
            .event()
        }),
        4 => Just(Event::Tick),
        4 => vec(session(), 0..4).prop_map(sim::sources),
        4 => vec(hold_file(), 0..4).prop_map(Event::Holds),
        1 => any::<bool>().prop_map(|a| Event::ConfigChanged(Config::new(a))),
        1 => any::<bool>().prop_map(|a| Event::Menu(MenuCommand::Automatic(a))),
        2 => keep_awake_choice().prop_map(|c| Event::Menu(MenuCommand::KeepAwake(c))),
        1 => any::<bool>().prop_map(|s| Event::Menu(MenuCommand::KeepScreenOn(s))),
        1 => Just(Event::Menu(MenuCommand::OpenLog)),
        1 => Just(Event::Suspending),
        1 => Just(Event::Resumed),
        1 => prop_oneof![Just(Ok(())), Just(Err(FAILED))]
            .prop_map(|r| Event::Done(Outcome::Requests(r))),
        1 => written().prop_map(Event::Done),
    ]
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        6 => event().prop_map(Op::Send),
        2 => (0..10 * MINUTE).prop_map(Op::Run),
        1 => (0..3 * HOUR).prop_map(Op::Suspend),
        1 => (-ms(2 * HOUR)..ms(2 * HOUR)).prop_map(Op::Wall),
    ]
}

proptest! {
    // A regression file would be a file this card does not own; a shrunk case becomes a named test instead.
    #![proptest_config(ProptestConfig { failure_persistence: None, ..ProptestConfig::default() })]

    /// `handle` never panics on any order of these events, `Sim` finds no deadline behind a `Tick`, and
    /// `state.json` is never written twice within a second.
    #[test]
    fn handle_never_panics_on_any_order(start_first in any::<bool>(), ops in vec(op(), 0..40)) {
        let mut sim = Sim::new();
        if start_first {
            sim.start();
        }
        for op in ops {
            match op {
                Op::Send(event) => {
                    sim.send(event);
                }
                Op::Run(ms) => {
                    sim.run(ms);
                }
                Op::Suspend(ms) => {
                    sim.suspend(ms);
                }
                Op::Wall(delta) => sim.set_wall(sim.now().wall_ms + delta),
            }
        }
        let writes = sim.state_writes();
        for pair in writes.windows(2) {
            prop_assert!(pair[1].0.mono_ms - pair[0].0.mono_ms >= SECOND, "{}", sim.trace());
        }
    }
}
