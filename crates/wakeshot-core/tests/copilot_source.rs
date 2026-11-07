//! `CopilotSource` fed the way the watch thread feeds it: lock names from a `Names` listing, then each session's
//! `events.jsonl` as appended bytes. Every fixture in `fixtures/copilot/` is read whole and in pieces. The source does
//! no I/O, so the home folder is a made-up path and no real `~/.copilot` is ever read.

use std::fs;
use std::path::{Path, PathBuf};

use proptest::prelude::*;
use wakeshot_core::sources::copilot::{
    CopilotSource, EVENT, IMAGE, LABEL, UNREADABLE, is_lock_file_name,
};
use wakeshot_core::sources::cron::FixedOffset;
use wakeshot_core::sources::{
    Activity, AgentSource, Fed, Feed, FileId, FormatHealth, Liveness, SessionFacts, SessionKey,
    SourceId, WantedFile,
};

const HOME: &str = r"X:\home\.copilot";
const ID: &str = "0199c0de-0000-4000-8000-000000000001";
const OTHER: &str = "0199c0de-0000-4000-8000-000000000002";
/// UTC+4, the zone the tests give the PC (Yerevan).
const ZONE: i32 = 240;

// Times in the fixtures, from the script that made them (`fixtures/copilot/README.md`).
const T_04_200: i64 = 1_793_959_204_200;
const T_10_20_00_750: i64 = 1_793_960_400_750;
const T_10_20_00_500: i64 = 1_793_960_400_500;
const T_10_30_00_500: i64 = 1_793_961_000_500;
const T_18_00: i64 = 1_793_988_000_000;
const T_11_30: i64 = 1_793_964_600_000;
/// Monday 2026-11-09 05:00 UTC: 09:00 in UTC+4. In New York (the cron's `tz`) it would be 14:00 UTC on the Friday.
const MON_05_00: i64 = 1_794_200_400_000;

const FIXTURES: [&str; 19] = [
    "working.jsonl",
    "waiting.jsonl",
    "idle.jsonl",
    "aborted.jsonl",
    "subagent-open.jsonl",
    "subagent-done.jsonl",
    "ask-user.jsonl",
    "resumed.jsonl",
    "schedule-at.jsonl",
    "schedule-every.jsonl",
    "schedule-every-later.jsonl",
    "schedule-after.jsonl",
    "schedule-cron.jsonl",
    "schedule-self-paced.jsonl",
    "schedule-cancelled.jsonl",
    "shutdown.jsonl",
    "junk-lines.jsonl",
    "rotation-before.jsonl",
    "rotation-head.jsonl",
];

fn state_dir() -> PathBuf {
    PathBuf::from(HOME).join("session-state")
}

fn lock(id: &str) -> FileId {
    FileId(state_dir().join(id).join("inuse.4242.lock"))
}

fn events(id: &str) -> FileId {
    FileId(state_dir().join(id).join("events.jsonl"))
}

fn source() -> CopilotSource {
    CopilotSource::new(PathBuf::from(HOME), Box::new(FixedOffset(ZONE)))
}

fn read_fixture(name: &str) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("copilot")
        .join(name);
    fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// A source with one locked session, fed `bytes` in pieces of the given sizes (the rest at the end).
fn fed_in(bytes: &[u8], pieces: &[usize]) -> CopilotSource {
    let mut s = source();
    assert_eq!(s.feed(&lock(ID), Feed::Present), Fed::Taken);
    let mut rest = bytes;
    for &size in pieces {
        let (piece, tail) = rest.split_at(size.min(rest.len()));
        assert_eq!(
            s.feed(&events(ID), Feed::Appended(piece.to_vec())),
            Fed::Taken
        );
        rest = tail;
    }
    if !rest.is_empty() {
        assert_eq!(
            s.feed(&events(ID), Feed::Appended(rest.to_vec())),
            Fed::Taken
        );
    }
    s
}

fn only(s: &CopilotSource) -> SessionFacts {
    let mut all = s.sessions();
    assert_eq!(all.len(), 1, "{all:?}");
    all.remove(0)
}

fn fed(name: &str) -> SessionFacts {
    only(&fed_in(&read_fixture(name), &[]))
}

fn facts(activity: Activity, timers: Vec<i64>, format: FormatHealth) -> SessionFacts {
    SessionFacts {
        key: SessionKey {
            source: SourceId::Copilot,
            id: ID.to_owned(),
        },
        liveness: Liveness::AnyProcess {
            image: "copilot.exe",
            lock_file: lock(ID).0,
        },
        activity,
        timers,
        open_tasks: 0,
        label: "Copilot",
        format,
    }
}

fn waiting(since: i64) -> Activity {
    Activity::Waiting {
        since_wall_ms: Some(since),
        remote: false,
    }
}

fn line(kind: &str, time: &str, data: &str) -> String {
    format!(
        r#"{{"id":"00000000-0000-4000-8000-0000000000ff","timestamp":"2026-11-06T{time}Z","parentId":null,"type":"{kind}","data":{data}}}"#
    ) + "\n"
}

fn feed_text(s: &mut CopilotSource, text: &str) {
    s.feed(&events(ID), Feed::Appended(text.as_bytes().to_vec()));
}

// Wanted files and session discovery.

#[test]
fn wanted_lists_lock_names_then_tails_each_locked_session() {
    let mut s = source();
    let names = |w: &[WantedFile]| match &w[0] {
        WantedFile::Names { dir, depth, accept } => {
            assert_eq!((dir, *depth), (&state_dir(), 1));
            assert!(accept("inuse.4242.lock") && !accept("events.jsonl"));
        }
        other => panic!("{other:?}"),
    };
    let first = s.wanted();
    assert_eq!(first.len(), 1);
    names(&first);
    s.feed(&lock(OTHER), Feed::Present);
    s.feed(&lock(ID), Feed::Present);
    let wanted = s.wanted();
    names(&wanted);
    let tails: Vec<&PathBuf> = wanted[1..]
        .iter()
        .map(|w| match w {
            WantedFile::Tail(path) => path,
            other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(tails, [&events(ID).0, &events(OTHER).0]);
}

#[test]
fn lock_names_are_filtered_before_any_open() {
    for name in [
        "inuse.4242.lock",
        "inuse.4242.hold",
        "INUSE.1.LOCK",
        "Inuse.1.HOLD",
        "inuse.abc.lock",
        "inuse.1.2.lock",
    ] {
        assert!(is_lock_file_name(name), "{name}");
    }
    for name in [
        "inuse..lock",
        "inuse..hold",
        "inuse.lock",
        "inuse.hold",
        "inuse.4242.lock.tmp",
        "inuse.4242.holder",
        "events.jsonl",
        "workspace.yaml",
        ".workspace-fork.lock",
        "handshake.4242",
        "x.inuse.4242.lock",
        "",
    ] {
        assert!(!is_lock_file_name(name), "{name}");
    }
}

/// The in-use file Copilot keeps open (seen live: `inuse.<pid>.hold`, made just before its `.lock`).
fn hold(id: &str) -> FileId {
    FileId(state_dir().join(id).join("inuse.4242.hold"))
}

#[test]
fn liveness_names_the_held_file_while_one_is_listed() {
    let held = |s: &CopilotSource| match only(s).liveness {
        Liveness::AnyProcess { lock_file, .. } => lock_file,
        other => panic!("{other:?}"),
    };
    // In the order Copilot makes them, and the other way.
    for order in [[hold(ID), lock(ID)], [lock(ID), hold(ID)]] {
        let mut s = source();
        for file in &order {
            s.feed(file, Feed::Present);
        }
        assert_eq!(held(&s), hold(ID).0, "{order:?}");
    }
    // A `.hold` alone is a session too, and a `.lock` stands in once the `.hold` is gone.
    let mut s = source();
    s.feed(&hold(ID), Feed::Present);
    assert_eq!(held(&s), hold(ID).0);
    s.feed(&lock(ID), Feed::Present);
    s.feed(&hold(ID), Feed::Gone);
    assert_eq!(held(&s), lock(ID).0);
    // A clean exit deletes both.
    s.feed(&lock(ID), Feed::Gone);
    assert!(s.sessions().is_empty());
}

#[test]
fn only_session_folders_with_a_lock_count() {
    let mut s = source();
    // Copilot's own dot folder, a lock file at the top, and a folder name that is not an id are not sessions.
    for path in [
        state_dir()
            .join(".session-operation-locks")
            .join("inuse.1.lock"),
        state_dir().join("inuse.1.lock"),
        state_dir().join("a b").join("inuse.1.lock"),
        PathBuf::from(r"X:\elsewhere").join(ID).join("inuse.1.lock"),
        state_dir().join(ID).join("workspace.yaml"),
    ] {
        s.feed(&FileId(path), Feed::Present);
    }
    // A log is ignored until its session has a lock.
    s.feed(&events(ID), Feed::Appended(read_fixture("working.jsonl")));
    assert!(s.sessions().is_empty());
    assert_eq!(s.wanted().len(), 1);
}

#[test]
fn a_lock_alone_is_an_idle_session() {
    let mut s = source();
    s.feed(&lock(ID), Feed::Present);
    assert_eq!(only(&s), facts(Activity::Idle, vec![], FormatHealth::Ok));
}

#[test]
fn lock_gone_ends_the_session_and_its_log() {
    let mut s = fed_in(&read_fixture("working.jsonl"), &[]);
    let second = FileId(state_dir().join(ID).join("inuse.5151.lock"));
    s.feed(&second, Feed::Present);
    s.feed(&lock(ID), Feed::Gone);
    let one_left = only(&s);
    assert_eq!(one_left.activity, Activity::Working);
    assert_eq!(
        one_left.liveness,
        Liveness::AnyProcess {
            image: IMAGE,
            lock_file: second.0.clone()
        }
    );
    s.feed(&second, Feed::Gone);
    assert!(s.sessions().is_empty());
    assert_eq!(s.wanted().len(), 1);
    // Back again: the watch thread replays the log from byte 0, so nothing of the old replay may stay.
    s.feed(&lock(ID), Feed::Present);
    assert_eq!(only(&s), facts(Activity::Idle, vec![], FormatHealth::Ok));
}

#[test]
fn lock_bytes_and_other_files_are_never_parsed() {
    let mut s = source();
    s.feed(&lock(ID), Feed::Present);
    // A source that is fed lock bytes anyway must not read them as a log.
    s.feed(&lock(ID), Feed::Whole(read_fixture("working.jsonl")));
    s.feed(&lock(ID), Feed::Appended(read_fixture("working.jsonl")));
    s.feed(
        &FileId(state_dir().join(ID).join("events.1.jsonl")),
        Feed::Appended(read_fixture("working.jsonl")),
    );
    s.feed(&events(ID), Feed::Whole(read_fixture("working.jsonl")));
    assert_eq!(only(&s), facts(Activity::Idle, vec![], FormatHealth::Ok));
}

#[test]
fn two_sessions_are_kept_apart() {
    let mut s = source();
    s.feed(&lock(ID), Feed::Present);
    s.feed(&lock(OTHER), Feed::Present);
    s.feed(&events(ID), Feed::Appended(read_fixture("working.jsonl")));
    s.feed(
        &events(OTHER),
        Feed::Appended(read_fixture("schedule-at.jsonl")),
    );
    let all = s.sessions();
    assert_eq!(all.len(), 2);
    assert_eq!(all[0], facts(Activity::Working, vec![], FormatHealth::Ok));
    assert_eq!(all[1].key.id, OTHER);
    assert_eq!(
        (all[1].activity, all[1].timers.as_slice()),
        (Activity::Idle, [T_18_00].as_slice())
    );
}

#[test]
fn source_and_labels_name_copilot() {
    assert_eq!(source().id(), SourceId::Copilot);
    assert_eq!((LABEL, IMAGE), ("Copilot", "copilot.exe"));
}

// Activity.

#[test]
fn a_turn_that_asked_for_a_tool_is_working() {
    assert_eq!(
        fed("working.jsonl"),
        facts(Activity::Working, vec![], FormatHealth::Ok)
    );
}

#[test]
fn an_approval_prompt_waits_since_its_time_and_is_not_remote() {
    assert_eq!(
        fed("waiting.jsonl"),
        facts(waiting(T_04_200), vec![], FormatHealth::Ok)
    );
}

#[test]
fn a_turn_that_ends_with_no_tool_is_idle() {
    assert_eq!(
        fed("idle.jsonl"),
        facts(Activity::Idle, vec![], FormatHealth::Ok)
    );
}

#[test]
fn an_abort_is_idle() {
    assert_eq!(
        fed("aborted.jsonl"),
        facts(Activity::Idle, vec![], FormatHealth::Ok)
    );
}

#[test]
fn an_open_subagent_keeps_working_after_the_main_turn_ends() {
    assert_eq!(
        fed("subagent-open.jsonl"),
        facts(Activity::Working, vec![], FormatHealth::Ok)
    );
    assert_eq!(
        fed("subagent-done.jsonl"),
        facts(Activity::Idle, vec![], FormatHealth::Ok)
    );
}

#[test]
fn a_subagents_own_turn_end_or_abort_does_not_end_the_main_turn() {
    let mut s = fed_in(&read_fixture("working.jsonl"), &[]);
    // The main agent's next model call has asked for no tool yet, so only its own turn end may make it idle.
    feed_text(
        &mut s,
        &line("assistant.turn_start", "10:00:04.900", r#"{"turnId":"1"}"#),
    );
    let sub = |kind: &str, data: &str| {
        line(kind, "10:00:05.000", data).replacen(
            r#""parentId":null,"#,
            r#""parentId":null,"agentId":"agent-1","#,
            1,
        )
    };
    feed_text(
        &mut s,
        &sub("assistant.message", r#"{"messageId":"x","content":"done"}"#),
    );
    feed_text(&mut s, &sub("assistant.turn_end", r#"{"turnId":"3"}"#));
    feed_text(&mut s, &sub("abort", r#"{"reason":"user_initiated"}"#));
    feed_text(
        &mut s,
        &sub("session.error", r#"{"errorType":"x","message":"x"}"#),
    );
    assert_eq!(only(&s).activity, Activity::Working);
    // The main agent's own end does.
    feed_text(
        &mut s,
        &line(
            "tool.execution_complete",
            "10:00:06.000",
            r#"{"toolCallId":"call_1","success":true}"#,
        ),
    );
    feed_text(
        &mut s,
        &line(
            "assistant.message",
            "10:00:07.000",
            r#"{"messageId":"y","content":"done"}"#,
        ),
    );
    feed_text(
        &mut s,
        &line("assistant.turn_end", "10:00:07.100", r#"{"turnId":"1"}"#),
    );
    assert_eq!(only(&s).activity, Activity::Idle);
}

#[test]
fn a_subagent_waiting_for_approval_waits() {
    let mut s = fed_in(&read_fixture("subagent-open.jsonl"), &[]);
    feed_text(
        &mut s,
        &line(
            "permission.requested",
            "10:00:09.000",
            r#"{"requestId":"r2","permissionRequest":{"kind":"read"}}"#,
        )
        .replacen(
            r#""parentId":null,"#,
            r#""parentId":null,"agentId":"agent-1","#,
            1,
        ),
    );
    assert_eq!(only(&s).activity, waiting(T_04_200 + 4_800));
    feed_text(
        &mut s,
        &line(
            "permission.carriedForward",
            "10:00:09.500",
            r#"{"requestId":"r2","toolCallId":"c","recordId":"x","decisionSource":"authorization_carry_forward"}"#,
        ),
    );
    assert_eq!(only(&s).activity, Activity::Working);
}

#[test]
fn a_question_to_the_user_waits_like_an_approval() {
    assert_eq!(
        fed("ask-user.jsonl"),
        facts(waiting(T_04_200), vec![], FormatHealth::Ok)
    );
}

#[test]
fn a_resume_after_a_crash_starts_at_the_prompt() {
    assert_eq!(
        fed("resumed.jsonl"),
        facts(Activity::Idle, vec![], FormatHealth::Ok)
    );
}

#[test]
fn a_session_error_is_idle_and_the_next_prompt_works() {
    let mut s = fed_in(&read_fixture("working.jsonl"), &[]);
    feed_text(
        &mut s,
        &line(
            "session.error",
            "10:00:05.000",
            r#"{"errorType":"model","message":"rate limited"}"#,
        ),
    );
    assert_eq!(only(&s).activity, Activity::Idle);
    feed_text(
        &mut s,
        &line("user.message", "10:01:00.000", r#"{"content":"try again"}"#),
    );
    assert_eq!(only(&s).activity, Activity::Working);
}

// Schedules.

#[test]
fn a_fixed_time_schedule_is_exact() {
    assert_eq!(
        fed("schedule-at.jsonl"),
        facts(Activity::Idle, vec![T_18_00], FormatHealth::Ok)
    );
}

#[test]
fn an_interval_counts_from_its_last_fire() {
    assert_eq!(
        fed("schedule-every.jsonl"),
        facts(Activity::Idle, vec![T_10_20_00_750], FormatHealth::Ok)
    );
}

#[test]
fn an_interval_with_no_fire_seen_moves_past_the_newest_event() {
    assert_eq!(
        fed("schedule-every-later.jsonl"),
        facts(Activity::Idle, vec![T_10_30_00_500], FormatHealth::Ok)
    );
}

#[test]
fn a_fire_that_names_no_schedule_moves_the_intervals_that_were_due() {
    let mut s = source();
    s.feed(&lock(ID), Feed::Present);
    // Two intervals from 10:00:00.500: 10 and 30 minutes. A fire with an unknown source tail at 10:10:00.750 is the
    // 10-minute one's; the 30-minute one was not due and keeps its phase.
    feed_text(
        &mut s,
        &line(
            "session.schedule_created",
            "10:00:00.500",
            r#"{"id":1,"intervalMs":600000,"prompt":"p","recurring":true}"#,
        ),
    );
    feed_text(
        &mut s,
        &line(
            "session.schedule_created",
            "10:00:00.500",
            r#"{"id":2,"intervalMs":1800000,"prompt":"p","recurring":true}"#,
        ),
    );
    feed_text(
        &mut s,
        &line(
            "user.message",
            "10:10:00.750",
            r#"{"content":"p","source":"schedule-delivery"}"#,
        ),
    );
    assert_eq!(only(&s).timers, [T_10_20_00_750, T_10_30_00_500]);
}

#[test]
fn a_one_shot_interval_fires_once() {
    assert_eq!(
        fed("schedule-after.jsonl"),
        facts(Activity::Idle, vec![T_10_30_00_500], FormatHealth::Ok)
    );
    // Its named fire ends it.
    let mut s = fed_in(&read_fixture("schedule-after.jsonl"), &[]);
    feed_text(
        &mut s,
        &line(
            "user.message",
            "10:30:00.600",
            r#"{"content":"p","source":"schedule-1"}"#,
        ),
    );
    assert!(only(&s).timers.is_empty());
}

#[test]
fn a_cron_runs_in_the_pc_zone_whatever_tz_says() {
    assert_eq!(
        fed("schedule-cron.jsonl"),
        facts(Activity::Idle, vec![MON_05_00], FormatHealth::Ok)
    );
}

#[test]
fn a_self_paced_schedule_takes_its_last_rearm() {
    assert_eq!(
        fed("schedule-self-paced.jsonl"),
        facts(Activity::Idle, vec![T_11_30], FormatHealth::Ok)
    );
}

#[test]
fn a_cancelled_schedule_holds_nothing() {
    assert_eq!(
        fed("schedule-cancelled.jsonl"),
        facts(Activity::Idle, vec![T_18_00], FormatHealth::Ok)
    );
}

#[test]
fn a_shutdown_drops_timers_until_the_session_resumes() {
    let mut s = fed_in(&read_fixture("shutdown.jsonl"), &[]);
    assert_eq!(only(&s), facts(Activity::Idle, vec![], FormatHealth::Ok));
    feed_text(
        &mut s,
        &line(
            "session.resume",
            "10:20:00.000",
            r#"{"resumeTime":"2026-11-06T10:20:00.000Z","eventCount":3}"#,
        ),
    );
    assert_eq!(
        only(&s),
        facts(Activity::Idle, vec![T_18_00], FormatHealth::Ok)
    );
}

#[test]
fn spent_one_shots_are_dropped() {
    let mut s = source();
    s.feed(&lock(ID), Feed::Present);
    feed_text(
        &mut s,
        &line(
            "session.schedule_created",
            "10:00:00.500",
            r#"{"id":1,"intervalMs":1800000,"prompt":"p","recurring":false}"#,
        ),
    );
    // More than an hour after its 10:30:00.500 run, the next schedule made clears it.
    feed_text(
        &mut s,
        &line(
            "session.schedule_created",
            "11:30:01.000",
            r#"{"id":2,"at":1793988000000,"prompt":"p"}"#,
        ),
    );
    assert_eq!(only(&s).timers, [T_18_00]);
}

#[test]
fn a_bad_schedule_is_named_only_while_working_and_holds_nothing() {
    let mut s = source();
    s.feed(&lock(ID), Feed::Present);
    feed_text(&mut s, &line("session.start", "10:00:00.000", "{}"));
    feed_text(
        &mut s,
        &line(
            "session.schedule_created",
            "10:00:00.500",
            r#"{"id":1,"cron":"0 9 * * MON","prompt":"p"}"#,
        ),
    );
    feed_text(
        &mut s,
        &line(
            "session.schedule_created",
            "10:00:00.600",
            r#"{"id":2,"intervalMs":0,"prompt":"p"}"#,
        ),
    );
    feed_text(
        &mut s,
        &line(
            "session.schedule_created",
            "10:00:00.700",
            r#"{"id":3,"prompt":"p"}"#,
        ),
    );
    assert_eq!(only(&s), facts(Activity::Idle, vec![], FormatHealth::Ok));
    feed_text(
        &mut s,
        &line("assistant.turn_start", "10:00:01.000", r#"{"turnId":"0"}"#),
    );
    assert_eq!(
        only(&s),
        facts(
            Activity::Working,
            vec![],
            FormatHealth::UnknownValue("cron", "0 9 * * MON".to_owned())
        )
    );
}

// Format problems.

#[test]
fn junk_lines_are_skipped_and_named_only_while_working() {
    let mut s = fed_in(&read_fixture("junk-lines.jsonl"), &[]);
    assert_eq!(
        only(&s),
        facts(
            Activity::Working,
            vec![],
            FormatHealth::UnknownValue(EVENT, "line".to_owned())
        )
    );
    feed_text(
        &mut s,
        &line(
            "assistant.message",
            "10:00:03.000",
            r#"{"messageId":"m","content":"done"}"#,
        ),
    );
    feed_text(
        &mut s,
        &line("assistant.turn_end", "10:00:03.100", r#"{"turnId":"0"}"#),
    );
    assert_eq!(only(&s), facts(Activity::Idle, vec![], FormatHealth::Ok));
}

#[test]
fn a_known_kind_with_a_missing_field_is_named_by_its_type() {
    let mut s = fed_in(&read_fixture("working.jsonl"), &[]);
    feed_text(
        &mut s,
        &line(
            "tool.execution_start",
            "10:00:05.000",
            r#"{"toolCallId":"c"}"#,
        ),
    );
    assert_eq!(
        only(&s).format,
        FormatHealth::UnknownValue(EVENT, "tool.execution_start".to_owned())
    );
}

#[test]
fn a_log_with_no_readable_line_is_unreadable() {
    let mut s = source();
    s.feed(&lock(ID), Feed::Present);
    feed_text(
        &mut s,
        "not json\n{\"type\":\"user.message\",\"timestamp\":\"yesterday\",\"data\":{}}\n",
    );
    assert_eq!(
        only(&s),
        facts(Activity::Idle, vec![], FormatHealth::Unreadable(UNREADABLE))
    );
    feed_text(
        &mut s,
        &line("user.message", "10:00:01.000", r#"{"content":"hi"}"#),
    );
    assert_eq!(
        only(&s).format,
        FormatHealth::UnknownValue(EVENT, "line".to_owned())
    );
}

#[test]
fn ephemeral_lines_are_ignored() {
    let mut s = fed_in(&read_fixture("idle.jsonl"), &[]);
    let ephemeral = line("assistant.turn_start", "10:01:00.000", r#"{"turnId":"9"}"#).replacen(
        r#""parentId":null,"#,
        r#""parentId":null,"ephemeral":true,"#,
        1,
    );
    feed_text(&mut s, &ephemeral);
    assert_eq!(only(&s), facts(Activity::Idle, vec![], FormatHealth::Ok));
}

#[test]
fn a_last_line_without_its_newline_waits_for_the_rest() {
    let bytes = read_fixture("idle.jsonl");
    let cut = bytes.len() - 30;
    let mut s = fed_in(&bytes[..cut], &[]);
    // The turn end is not complete yet, so the last message's turn is still running.
    assert_eq!(only(&s).activity, Activity::Working);
    s.feed(&events(ID), Feed::Appended(bytes[cut..].to_vec()));
    assert_eq!(only(&s), fed("idle.jsonl"));
}

#[test]
fn crlf_lines_read_the_same() {
    let text = String::from_utf8(read_fixture("waiting.jsonl")).unwrap();
    let crlf = text.replace('\n', "\r\n");
    assert_eq!(only(&fed_in(crlf.as_bytes(), &[])), fed("waiting.jsonl"));
}

// Rotation and restarts.

#[test]
fn a_rotation_keeps_what_the_old_file_said() {
    let mut s = fed_in(&read_fixture("rotation-before.jsonl"), &[]);
    let before = only(&s);
    assert_eq!(
        before,
        facts(Activity::Working, vec![T_10_20_00_750], FormatHealth::Ok)
    );
    s.feed(&events(ID), Feed::Gone);
    s.feed(
        &events(ID),
        Feed::Appended(read_fixture("rotation-head.jsonl")),
    );
    assert_eq!(only(&s), before);
}

#[test]
fn a_rotated_head_read_alone_takes_its_checkpoint_schedules_only() {
    assert_eq!(
        fed("rotation-head.jsonl"),
        facts(Activity::Idle, vec![T_10_20_00_500], FormatHealth::Ok)
    );
}

#[test]
fn a_file_that_starts_over_without_a_link_is_replayed_from_scratch() {
    let mut s = fed_in(&read_fixture("rotation-before.jsonl"), &[]);
    s.feed(&events(ID), Feed::Gone);
    s.feed(&events(ID), Feed::Appended(read_fixture("idle.jsonl")));
    assert_eq!(only(&s), fed("idle.jsonl"));
    // A partial line before the restart is not glued to the new file's first line.
    let mut s = fed_in(&read_fixture("working.jsonl"), &[]);
    feed_text(
        &mut s,
        r#"{"id":"x","timestamp":"2026-11-06T10:00:05.000Z","type":"abo"#,
    );
    s.feed(&events(ID), Feed::Gone);
    s.feed(&events(ID), Feed::Appended(read_fixture("waiting.jsonl")));
    assert_eq!(only(&s), fed("waiting.jsonl"));
}

// Live logs (`fixtures/copilot/live/`, 2026-11-06), redacted.

const LIVE: [&str; 3] = [
    "tui-a.jsonl",
    "tui-crash-mid-turn.jsonl",
    "oneshot-npm.jsonl",
];

fn read_live(name: &str) -> Vec<u8> {
    read_fixture(&Path::new("live").join(name).to_string_lossy())
}

/// Each point where a line by line replay changes the facts: the 1-based line, the activity and the timers after it.
fn changes(bytes: &[u8]) -> Vec<(usize, Activity, Vec<i64>)> {
    let mut s = source();
    s.feed(&lock(ID), Feed::Present);
    let mut out = Vec::new();
    let mut last = (Activity::Idle, Vec::new());
    for (i, line) in bytes.split_inclusive(|&b| b == b'\n').enumerate() {
        s.feed(&events(ID), Feed::Appended(line.to_vec()));
        let now = only(&s);
        assert_eq!(now.format, FormatHealth::Ok, "line {}", i + 1);
        let state = (now.activity, now.timers);
        if state != last {
            out.push((i + 1, state.0, state.1.clone()));
            last = state;
        }
    }
    out
}

#[test]
fn every_live_fixture_is_listed_once() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/copilot/live");
    let mut names: Vec<String> = fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| n != "README.md")
        .collect();
    names.sort();
    let mut listed = LIVE.map(str::to_owned).to_vec();
    listed.sort();
    assert_eq!(names, listed);
}

/// TUI A: a 60 s command behind an approval left 60 s, an `ask_user` question left 2 min 40 s, a command stopped by
/// Ctrl+C, `/every 2m` with one fire, its cancel, `/exit`.
#[test]
fn the_live_tui_session_moves_at_its_turn_events() {
    const APPROVAL_1: i64 = 1_793_965_985_272; // 11:53:05.272 UTC
    const ASK: i64 = 1_793_966_160_706; // 11:56:00.706
    const APPROVAL_3: i64 = 1_793_966_444_665; // 12:00:44.665
    const FIRST_FIRE: i64 = 1_793_966_621_381; // made 12:01:41.381, every 2 min
    const SECOND_FIRE: i64 = 1_793_966_741_499; // fired 12:03:41.499 as `schedule-1`
    let working = |line| (line, Activity::Working, vec![]);
    let idle = |line| (line, Activity::Idle, vec![]);
    assert_eq!(
        changes(&read_live("tui-a.jsonl")),
        [
            working(5),
            (19, waiting(APPROVAL_1), vec![]),
            working(20),
            idle(26),
            working(28),
            (40, waiting(ASK), vec![]),
            working(42),
            idle(47),
            working(49),
            (62, waiting(APPROVAL_3), vec![]),
            working(63),
            idle(65),
            (67, Activity::Idle, vec![FIRST_FIRE]),
            (68, Activity::Working, vec![SECOND_FIRE]),
            (72, Activity::Idle, vec![SECOND_FIRE]),
            idle(74),
        ]
    );
}

/// TUI B: a 60 s command, allowed up front, then its `copilot.exe` killed by pid while it ran. The log ends inside the
/// turn, so the source says working; only the watch's liveness (the `.hold` lost its holder) ends the session.
#[test]
fn the_live_crash_ends_inside_its_turn() {
    assert_eq!(
        changes(&read_live("tui-crash-mid-turn.jsonl")),
        [(5, Activity::Working, vec![])]
    );
}

/// `copilot -p` through npm's own launcher: a 30 s command allowed up front, two turns that ask for tools, a last one
/// that asks for none. A dry-run tray held from the prompt to that last turn's end.
#[test]
fn the_live_one_shot_works_until_its_last_turn_ends() {
    assert_eq!(
        changes(&read_live("oneshot-npm.jsonl")),
        [(5, Activity::Working, vec![]), (22, Activity::Idle, vec![])]
    );
}

#[test]
fn live_logs_give_the_same_facts_in_any_pieces() {
    for name in LIVE {
        let bytes = read_live(name);
        let whole = only(&fed_in(&bytes, &[]));
        assert_eq!(
            only(&fed_in(&bytes, &vec![1; bytes.len()])),
            whole,
            "{name}"
        );
        assert_eq!(
            only(&fed_in(&bytes, &vec![4093; bytes.len() / 4093 + 1])),
            whole,
            "{name}"
        );
    }
}

// VS Code's Copilot harness (`fixtures/vscode/live/harness/`, 2026-11-06): the same event log, written by VS Code's
// `copilot-runtime.exe`, with kinds the CLI's logs did not have (hooks, `model.*`).

const HARNESS: [&str; 1] = ["stop-approve-deny-close.jsonl"];

fn harness_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/vscode/live/harness")
}

fn read_harness(name: &str) -> Vec<u8> {
    let path = harness_dir().join(name);
    fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

#[test]
fn every_harness_fixture_is_listed_once() {
    let mut names: Vec<String> = fs::read_dir(harness_dir())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    let mut listed = HARNESS.map(str::to_owned).to_vec();
    listed.sort();
    assert_eq!(names, listed);
}

/// One harness chat: three Stops while `sleep 60` ran, an approval left 61 s, a Skip after 60 s, then a turn whose
/// window was closed while it ran. VS Code approves `sleep` itself 12 ms after asking, so each command start passes
/// through a waiting that short. Every Stop ends the turn at its `abort`, and the closed window's turn at its own end.
#[test]
fn the_live_harness_chat_ends_at_each_stop_and_at_the_real_end() {
    let working = |line| (line, Activity::Working, vec![]);
    let idle = |line| (line, Activity::Idle, vec![]);
    let asked = |line, since| (line, waiting(since), vec![]);
    assert_eq!(
        changes(&read_harness("stop-approve-deny-close.jsonl")),
        [
            working(9),
            asked(17, 1_793_978_935_501), // 15:28:55.501 UTC
            working(18),
            idle(19), // Stop 1: `abort`
            working(27),
            asked(42, 1_793_978_980_857),
            working(43),
            idle(44), // Stop 2
            working(51),
            asked(77, 1_793_979_023_284),
            working(78),
            idle(79), // Stop 3
            working(86),
            asked(95, 1_793_979_101_072), // `ping`, left 61 s
            working(102),
            asked(113, 1_793_979_193_283), // the chat's rename, approved by VS Code
            working(114),
            idle(124),
            working(133),
            asked(148, 1_793_979_227_097), // `ping`, then Skip
            working(149),
            idle(155),
            working(163),
            asked(178, 1_793_979_335_963),
            working(179),
            idle(187), // the window was closed while this command ran; the turn ran on to its end
        ]
    );
}

#[test]
fn the_live_harness_log_gives_the_same_facts_in_any_pieces() {
    for name in HARNESS {
        let bytes = read_harness(name);
        let whole = only(&fed_in(&bytes, &[]));
        assert_eq!(
            only(&fed_in(&bytes, &vec![1; bytes.len()])),
            whole,
            "{name}"
        );
        assert_eq!(
            only(&fed_in(&bytes, &vec![4093; bytes.len() / 4093 + 1])),
            whole,
            "{name}"
        );
    }
}

// Whole and in pieces.

#[test]
fn every_fixture_gives_the_same_facts_byte_by_byte() {
    for name in FIXTURES {
        let bytes = read_fixture(name);
        let whole = only(&fed_in(&bytes, &[]));
        let ones = vec![1; bytes.len()];
        assert_eq!(only(&fed_in(&bytes, &ones)), whole, "{name}");
        let sevens = vec![7; bytes.len() / 7 + 1];
        assert_eq!(only(&fed_in(&bytes, &sevens)), whole, "{name}");
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn every_fixture_gives_the_same_facts_in_any_pieces(
        cuts in prop::collection::vec(0usize..4096, 0..24),
        pick in 0usize..FIXTURES.len(),
    ) {
        let bytes = read_fixture(FIXTURES[pick]);
        let whole = only(&fed_in(&bytes, &[]));
        prop_assert_eq!(only(&fed_in(&bytes, &cuts)), whole);
    }

    #[test]
    fn any_bytes_never_panic_and_keep_the_bounds(bytes in prop::collection::vec(any::<u8>(), 0..2048)) {
        let s = fed_in(&bytes, &[]);
        let facts = only(&s);
        prop_assert!(facts.timers.len() <= 256);
    }
}

// Privacy.

#[test]
fn prompt_text_never_reaches_facts_or_debug() {
    let mut s = source();
    for name in FIXTURES {
        let one = fed_in(&read_fixture(name), &[]);
        let shown = format!("{one:?} {:?}", one.sessions());
        for text in [
            "fix the build",
            "check the deploy",
            "map the code",
            "Which database",
            "cargo build",
            "The build passes",
        ] {
            assert!(!shown.contains(text), "{name}: {text} in {shown}");
        }
    }
    // A partial line is shown by its length only.
    s.feed(&lock(ID), Feed::Present);
    feed_text(
        &mut s,
        r#"{"type":"user.message","data":{"content":"fix the build"#,
    );
    assert!(!format!("{s:?}").contains("fix the build"));
}
