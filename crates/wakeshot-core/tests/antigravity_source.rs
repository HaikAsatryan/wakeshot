//! Antigravity's table and source on the hook inputs and the log in `fixtures/antigravity/` (`docs/agent-hooks.md`,
//! "Antigravity"): what the hook keeps of each event, the walk to `agy.exe`, the files and the sessions they give, the
//! exact end (a `Stop`, a cancel line in `agy`'s log at or after the hook's start, or the process's end), the guard for
//! a log not as tested, and the privacy rule that no input value but the chat id, and no log line, ever reaches a file,
//! a session or a log text.
//!
//! Owner: `p4-antigravity-source`; the log rule by `p4-antigravity-exact-end`.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use wakeshot_core::process::{FileTime, ProcessRef};
use wakeshot_core::sources::antigravity::{
    self, AGENT, AntigravitySource, CLI_LABEL, LABEL, LOG_PROBLEM,
};
use wakeshot_core::sources::cron::{FixedOffset, LocalOffset};
use wakeshot_core::sources::hooked::{
    self, Bases, Checked, FileRead, HookInput, InputProblem, Listed, LogMark, Op, Parsed, Problem,
    State,
};
use wakeshot_core::sources::{
    Activity, AgentSource, Fed, Feed, FileId, FormatHealth, Liveness, SourceId, WantedFile,
};

const CLI_CHAT: &str = "ec33ebf9-0cba-4100-8142-c61503f6c587";
const IDE_CHAT: &str = "5d0c7b2e-9a41-4f3b-b8e6-2c7f1a9d4e10";

/// The fixtures with the event each one is for.
const FIXTURES: [(&str, &str); 5] = [
    ("cli-pre-invocation.json", "PreInvocation"),
    ("cli-stop-idle.json", "Stop"),
    ("cli-stop-background.json", "Stop"),
    ("ide-pre-invocation.json", "PreInvocation"),
    ("ide-stop-canceled.json", "Stop"),
];

/// A word with a space, so it is not a chat id: no kept field may keep it.
const MARKER: &str = "QXV7 marker";
const MARKER_NUMBER: u64 = 77_317_731;

fn fixture(name: &str) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("antigravity")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn parse(event: &str, bytes: &[u8]) -> Parsed {
    hooked::parse_input(&AGENT, event, bytes)
}

fn op(event: &str, bytes: &[u8]) -> Result<Op, InputProblem> {
    let input = parse(event, bytes).input?;
    hooked::resolve(AGENT.event(event).unwrap(), &input)
}

#[test]
fn each_fixture_keeps_its_chat_lowercased_and_nothing_else() {
    for (name, event) in FIXTURES {
        let parsed = parse(event, &fixture(name));
        let input = parsed
            .input
            .clone()
            .unwrap_or_else(|p| panic!("{name}: {p}"));
        let chat = if name.starts_with("ide") {
            IDE_CHAT
        } else {
            CLI_CHAT
        };
        assert_eq!(input.chat, chat, "{name}");
        assert_eq!(input.turn, "", "{name}: Antigravity has no turn id");
        assert_eq!(parsed.version, None, "{name}: and no version field");
        let want = match event {
            "Stop" => Op::Delete,
            _ => Op::Write(State::Working),
        };
        assert_eq!(op(event, &fixture(name)), Ok(want), "{name}");
        assert_eq!(input.idle, None, "{name}: `fullyIdle` is not read");
    }
}

#[test]
fn the_table_hooks_only_the_model_call_and_the_stop() {
    let names: Vec<_> = AGENT.events.iter().map(|rule| rule.name).collect();
    assert_eq!(names, ["PreInvocation", "Stop"]);
    for event in [
        "SessionStart",
        "PostInvocation",
        "PreToolUse",
        "PostToolUse",
        "stop",
    ] {
        assert!(AGENT.event(event).is_none(), "{event}");
    }
    assert!(AGENT.event("PreInvocation").unwrap().prune);
    assert!(!AGENT.event("Stop").unwrap().prune);
    assert_eq!(AGENT.name, "antigravity");
    assert_eq!(AGENT.payload_key, "conversationId");
}

/// Background commands are not held (no exact end in every case), so whatever `fullyIdle` holds, or its absence, a
/// `Stop` ends the chat and is never a problem.
#[test]
fn fully_idle_is_never_read_so_every_stop_ends_the_chat() {
    let stop: Value = serde_json::from_slice(&fixture("cli-stop-idle.json")).unwrap();
    let mut no_flag = stop.clone();
    no_flag.as_object_mut().unwrap().remove("fullyIdle");
    let mut cases = vec![no_flag];
    for value in [
        json!(false),
        json!(true),
        Value::Null,
        json!("true"),
        json!(MARKER_NUMBER),
    ] {
        let mut case = stop.clone();
        case["fullyIdle"] = value;
        cases.push(case);
    }
    for case in cases {
        assert_eq!(
            op("Stop", case.to_string().as_bytes()),
            Ok(Op::Delete),
            "{case}"
        );
    }
    let mut flagged: Value = serde_json::from_slice(&fixture("cli-pre-invocation.json")).unwrap();
    flagged["fullyIdle"] = json!(false);
    assert_eq!(
        op("PreInvocation", flagged.to_string().as_bytes()),
        Ok(Op::Write(State::Working))
    );
}

#[test]
fn an_input_with_no_chat_or_another_shape_is_unreadable() {
    let base: Value = serde_json::from_slice(&fixture("cli-pre-invocation.json")).unwrap();
    let mut none = base.clone();
    none.as_object_mut().unwrap().remove("conversationId");
    assert_eq!(
        parse("PreInvocation", none.to_string().as_bytes()).input,
        Err(InputProblem::NoChat)
    );
    // Cursor's spelling is another field to Antigravity's table.
    let mut snake = none.clone();
    snake["conversation_id"] = json!(CLI_CHAT);
    assert_eq!(
        parse("PreInvocation", snake.to_string().as_bytes()).input,
        Err(InputProblem::NoChat)
    );
    for chat in [r"..\..\x", "CON", "ec33ebf9-0cba-4100-8142-c61503f6c58", ""] {
        let mut crafted = base.clone();
        crafted["conversationId"] = json!(chat);
        assert_eq!(
            parse("PreInvocation", crafted.to_string().as_bytes()).input,
            Err(InputProblem::ChatShape),
            "{chat}"
        );
    }
    let mut twice = String::from_utf8(fixture("cli-pre-invocation.json")).unwrap();
    twice.insert_str(1, &format!(r#""conversationId":"{IDE_CHAT}","#));
    assert!(matches!(
        parse("PreInvocation", twice.as_bytes()).input,
        Err(InputProblem::Parse { .. })
    ));
}

/// Every top-level value of `base`, one at a time, swapped for the marker as a string, a number and an object.
fn marked(base: &Value) -> Vec<(String, Value)> {
    let Value::Object(fields) = base else {
        panic!("a fixture is an object");
    };
    let mut object = serde_json::Map::new();
    object.insert(MARKER.to_owned(), json!([MARKER, MARKER_NUMBER]));
    let mut out = Vec::new();
    for key in fields.keys() {
        for value in [
            json!(MARKER),
            json!(MARKER_NUMBER),
            Value::Object(object.clone()),
        ] {
            let mut changed = base.clone();
            changed[key] = value;
            out.push((key.clone(), changed));
        }
    }
    let mut changed = base.clone();
    changed[MARKER] = json!(MARKER);
    out.push((MARKER.to_owned(), changed));
    out
}

fn assert_clean(text: &str, case: &str) {
    for word in ["QXV7", "marker", &MARKER_NUMBER.to_string()] {
        assert!(!text.contains(word), "{case}: {word:?} in {text:?}");
    }
}

/// The privacy rule on the core's side: what the parse returns (and so every file text and log text made from it)
/// holds no input value but the checked chat id: not the prompt (`lastUserInput`), the model's answer, the paths or the
/// model name. On the made-up fixtures and on the live ones.
#[test]
fn the_marker_in_any_field_never_reaches_a_file_or_a_log_text() {
    let process = ProcessRef {
        pid: 4_242,
        start: FileTime(134_000_000_000_000_001),
    };
    let mut written = 0;
    let made_up = FIXTURES
        .iter()
        .map(|&(name, event)| (name, event, fixture(name)));
    let real = LIVE
        .iter()
        .map(|&(name, event, ..)| (name, event, live(name)));
    for (name, event, bytes) in made_up.chain(real) {
        let base: Value = serde_json::from_slice(&bytes).unwrap();
        let rule = AGENT.event(event).unwrap();
        for (field, input) in marked(&base) {
            let case = format!("{name}, {field} = {}", input[&field]);
            let parsed = parse(event, input.to_string().as_bytes());
            assert_clean(&format!("{parsed:?}"), &case);
            let mut texts = Vec::new();
            let resolved = parsed
                .input
                .as_ref()
                .map_err(|p| *p)
                .and_then(|input| hooked::resolve(rule, input));
            match (&parsed.input, resolved) {
                (Ok(input), Ok(Op::Write(state))) => {
                    texts.push(hooked::state_text_marked(
                        process,
                        &input.turn,
                        CLI_LABEL,
                        state,
                        Some(&mark(at(18, 56, 14, 950_000))),
                    ));
                    written += 1;
                }
                (_, Err(problem)) => {
                    texts.push(problem.to_string().into_bytes());
                    texts.push(hooked::problem_text(process, Problem::Input));
                }
                _ => {}
            }
            for text in texts {
                assert_clean(&String::from_utf8(text).unwrap(), &case);
            }
            // The chat id, as the marker, is refused rather than kept.
            if field == "conversationId" && input[&field] == json!(MARKER) {
                assert_eq!(parsed.input, Err(InputProblem::ChatShape), "{case}");
            }
        }
    }
    assert!(written >= 40, "the check saw real state files: {written}");
}

fn me() -> ProcessRef {
    ProcessRef {
        pid: 900,
        start: FileTime(9_000),
    }
}

fn profile() -> PathBuf {
    PathBuf::from("Users").join("dev")
}

fn local() -> PathBuf {
    profile().join("AppData").join("Local")
}

fn bases() -> Bases {
    Bases {
        local_app_data: Some(local()),
        profile: Some(profile()),
    }
}

fn cli_agy() -> PathBuf {
    local().join("agy").join("bin").join("agy.exe")
}

fn ide_agy() -> PathBuf {
    profile().join(".gemini").join("bin").join("agy.exe")
}

fn listed(pid: u32, parent: u32, image: &'static str) -> Listed<'static> {
    Listed {
        pid,
        parent,
        start: FileTime(u64::from(pid) * 10),
        image,
    }
}

#[test]
fn the_clis_hook_names_the_agy_that_ran_its_cmd() {
    // Hook, the `cmd /c` that `agy` starts, the CLI, the user's terminal.
    let list = [
        listed(900, 800, "wakeshot.exe"),
        listed(800, 700, "cmd.exe"),
        listed(700, 600, "agy.exe"),
        listed(600, 1, "pwsh.exe"),
    ];
    let walk = hooked::walk(&AGENT, &list, me(), &bases(), |p| {
        (p.pid == 700).then(cli_agy)
    });
    assert_eq!(
        walk.parent.map(|p| (p.pid, p.start)),
        Some((800, FileTime(8_000))),
        "the transport check reads cmd's command line, and cmd's start is the turn's mark"
    );
    let found = walk.agent.unwrap();
    assert_eq!(found.process.pid, 700);
    assert_eq!((found.image.key, found.image.label), ("cli", CLI_LABEL));
    assert!(antigravity::holds(found.image));
}

/// The walk still names the IDE's hub, so its hook stops there and writes nothing, rather than walking on.
#[test]
fn the_ides_hook_names_its_hub_and_never_a_farther_agy_and_is_not_held() {
    // Hook, cmd, the IDE's hub, the IDE's extension host; a Remote Control daemon farther up in the second case.
    let hub = [
        listed(900, 800, "wakeshot.exe"),
        listed(800, 700, "cmd.exe"),
        listed(700, 600, "agy.exe"),
        listed(600, 500, "Antigravity IDE.exe"),
    ];
    let walk = hooked::walk(&AGENT, &hub, me(), &bases(), |p| {
        (p.pid == 700).then(ide_agy)
    });
    let found = walk.agent.unwrap();
    assert_eq!(
        (found.process.pid, found.image.key, found.image.label),
        (700, "app", LABEL)
    );
    assert!(!antigravity::holds(found.image));

    let nested = [
        listed(900, 800, "wakeshot.exe"),
        listed(800, 750, "cmd.exe"),
        listed(750, 700, "AGY.EXE"),
        listed(700, 1, "agy.exe"),
    ];
    let walk = hooked::walk(&AGENT, &nested, me(), &bases(), |_| Some(cli_agy()));
    assert_eq!(walk.agent.unwrap().process.pid, 750, "the nearest one");

    // An `agy.exe` whose path does not read is still Antigravity, under its plain name, and not held.
    let walk = hooked::walk(&AGENT, &hub, me(), &bases(), |_| None);
    assert_eq!(walk.agent.unwrap().image.label, LABEL);
}

#[test]
fn without_agy_above_it_or_with_a_younger_parent_the_hook_finds_nothing() {
    let none = [
        listed(900, 800, "wakeshot.exe"),
        listed(800, 700, "cmd.exe"),
        listed(700, 1, "Cursor.exe"),
    ];
    let walk = hooked::walk(&AGENT, &none, me(), &bases(), |_| None);
    assert_eq!(walk.agent, None);
    assert_eq!(walk.parent.map(|p| p.pid), Some(800));

    // A pid reused by an `agy.exe` that started after the hook's cmd is not its parent.
    let reused = [
        listed(900, 800, "wakeshot.exe"),
        listed(800, 700, "cmd.exe"),
        Listed {
            pid: 700,
            parent: 1,
            start: FileTime(8_500),
            image: "agy.exe",
        },
    ];
    let walk = hooked::walk(&AGENT, &reused, me(), &bases(), |_| None);
    assert_eq!(walk.agent, None);
}

#[test]
fn the_checked_values_carry_a_model_calls_log_and_a_stops_delete_only() {
    let process = ProcessRef {
        pid: 700,
        start: FileTime(7_000),
    };
    let stop = AGENT.event("Stop").unwrap();
    let call = AGENT.event("PreInvocation").unwrap();
    let values = |op: &str, key: &str, log: &[&str]| {
        let mut values: Vec<String> = [CLI_CHAT, "", "700", "7000", key, op, "1762400002000"]
            .map(str::to_owned)
            .to_vec();
        values.extend(log.iter().map(|v| (*v).to_owned()));
        values
    };
    let deleted = Checked::parse(&AGENT, stop, &values("none", "cli", &[])).unwrap();
    assert_eq!((deleted.op, deleted.process), (Op::Delete, process));
    let working =
        Checked::parse(&AGENT, call, &values("working", "cli", &[LOG_NAME, "8000"])).unwrap();
    assert_eq!(working.op, Op::Write(State::Working));
    assert_eq!(
        working.log,
        Some(LogMark {
            name: LOG_NAME.to_owned(),
            turn_at: FileTime(8_000),
        })
    );
    assert_eq!(
        working.values(),
        values("working", "cli", &[LOG_NAME, "8000"])
    );
    // What the event itself cannot do (a `Stop` keeps no background state now), and a key no rule has.
    for (rule, op, key, log) in [
        (stop, "background", "cli", &[][..]),
        (stop, "working", "cli", &[]),
        (stop, "none", "cli", &[LOG_NAME, "8000"]),
        (call, "background", "cli", &[]),
        (call, "none", "cli", &[]),
        (stop, "none", "editor", &[]),
        (call, "working", "cli", &[r"..\..\x.log", "8000"]),
    ] {
        assert_eq!(
            Checked::parse(&AGENT, rule, &values(op, key, log)),
            None,
            "{op} {key} {log:?}"
        );
    }
}

fn data_root() -> PathBuf {
    PathBuf::from("data")
}

fn folder() -> PathBuf {
    data_root().join("agents").join("antigravity")
}

fn chat_id(chat: &str) -> FileId {
    FileId(folder().join(hooked::chat_file(chat)))
}

// The log rule (`p4-antigravity-exact-end`), on `fixtures/antigravity/log/`: a made-up log in the shape of `agy`
// 1.5.2's, with the pid, version and cancel lines as the live test saw them, and prompt lines that hold the marker.

const LOG_NAME: &str = "cli-20261106_185448.log";
/// The chat the log's cancel lines name, and a second chat cancelled once, later.
const LOG_CHAT: &str = "1ad3ac30-4354-4e1e-a5d3-d338da829975";
const OTHER_CHAT: &str = "43b8806d-6a1f-4c2e-9b7d-0e5f3a2c1d4b";
/// The `agy.exe` line 1 names.
const AGY: ProcessRef = ProcessRef {
    pid: 51_384,
    start: FileTime(134_384_504_887_188_820),
};

fn log_bytes() -> Vec<u8> {
    fixture(&format!("log/{LOG_NAME}"))
}

fn log_id() -> FileId {
    FileId(antigravity::log_dir(&profile()).join(LOG_NAME))
}

/// Days from 1970-01-01 (Howard Hinnant's `days_from_civil`), so a test names a local time plainly.
fn days(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let day_of_year = (153 * ((month + 9) % 12) + 2) / 5 + day - 1;
    era * 146_097 + year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year - 719_468
}

/// The FILETIME of a local time at `offset` minutes from UTC.
fn filetime(date: (i64, i64, i64), clock: (i64, i64, i64, i64), offset: i64) -> FileTime {
    let (hour, minute, second, micros) = clock;
    let local_us = (days(date.0, date.1, date.2) * 86_400 + hour * 3_600 + minute * 60 + second)
        * 1_000_000
        + micros;
    let utc_us = local_us - offset * 60_000_000;
    FileTime(u64::try_from(utc_us * 10 + 116_444_736_000_000_000).unwrap())
}

/// A local time of 2026-11-06 at +04:00 (Yerevan), the zone of the live tests.
fn at(hour: i64, minute: i64, second: i64, micros: i64) -> FileTime {
    filetime((2026, 11, 6), (hour, minute, second, micros), 240)
}

fn mark(turn_at: FileTime) -> LogMark {
    LogMark {
        name: LOG_NAME.to_owned(),
        turn_at,
    }
}

/// The file a model call's hook writes for `chat` in `AGY`, started at `turn_at`.
fn call_file(chat: &str, turn_at: FileTime) -> (FileId, Feed) {
    let text = hooked::state_text_marked(AGY, "", CLI_LABEL, State::Working, Some(&mark(turn_at)));
    (chat_id(chat), Feed::Whole(text))
}

fn source_in(zone: Box<dyn LocalOffset + Send>) -> AntigravitySource {
    antigravity::source(&data_root(), Some(&profile()), zone)
}

fn source() -> AntigravitySource {
    source_in(Box::new(FixedOffset(240)))
}

fn feed(source: &mut AntigravitySource, (file, data): (FileId, Feed)) {
    assert_eq!(source.feed(&file, data), Fed::Taken);
}

/// Each session as its id, activity and format.
fn turns(source: &AntigravitySource) -> Vec<(String, Activity, FormatHealth)> {
    source
        .sessions()
        .into_iter()
        .map(|s| (s.key.id, s.activity, s.format))
        .collect()
}

fn working(chat: &str) -> (String, Activity, FormatHealth) {
    (chat.to_owned(), Activity::Working, FormatHealth::Ok)
}

fn ended(chat: &str) -> (String, Activity, FormatHealth) {
    (chat.to_owned(), Activity::Idle, FormatHealth::Ok)
}

fn unknown(chat: &str) -> (String, Activity, FormatHealth) {
    (
        chat.to_owned(),
        Activity::Idle,
        FormatHealth::Unreadable(LOG_PROBLEM),
    )
}

#[test]
fn the_source_tails_the_log_each_working_cli_chat_names() {
    let mut source = source();
    assert_eq!(source.id(), SourceId::Antigravity);
    let folder_only = |source: &AntigravitySource| {
        let wanted = source.wanted();
        let [WantedFile::Folder { dir, accept }] = &wanted[..] else {
            panic!("one folder: {wanted:?}");
        };
        assert_eq!(dir, &folder());
        assert!(accept(&format!("{CLI_CHAT}.json")));
        assert!(!accept(&format!(
            "{CLI_CHAT}.json.900.0.0123456789abcdef.tmp"
        )));
    };
    folder_only(&source);
    feed(&mut source, call_file(LOG_CHAT, at(18, 56, 14, 950_000)));
    feed(&mut source, call_file(OTHER_CHAT, at(19, 18, 20, 0)));
    let wanted = source.wanted();
    assert!(matches!(&wanted[0], WantedFile::Folder { .. }));
    assert_eq!(
        wanted[1..]
            .iter()
            .map(|w| match w {
                WantedFile::Tail(path) => path.clone(),
                other => panic!("{other:?}"),
            })
            .collect::<Vec<_>>(),
        [log_id().0],
        "one tail for the one log both chats name"
    );
    // Gone, or a file that names no log or names one of another shape: no tail.
    feed(&mut source, (chat_id(LOG_CHAT), Feed::Gone));
    feed(&mut source, (chat_id(OTHER_CHAT), Feed::Gone));
    folder_only(&source);
    let plain = hooked::state_text(AGY, "", CLI_LABEL, State::Working);
    feed(&mut source, (chat_id(LOG_CHAT), Feed::Whole(plain)));
    let mut odd = mark(at(18, 0, 0, 0));
    for name in ["cli.log", r"..\cli-20261106_185448.log", "notes.log"] {
        odd.name = name.to_owned();
        let text = hooked::state_text_marked(AGY, "", CLI_LABEL, State::Working, Some(&odd));
        feed(&mut source, (chat_id(CLI_CHAT), Feed::Whole(text)));
        folder_only(&source);
    }
    // No profile folder: nothing to tail.
    let mut homeless = antigravity::source(&data_root(), None, Box::new(FixedOffset(0)));
    feed(&mut homeless, call_file(LOG_CHAT, at(18, 56, 14, 950_000)));
    assert_eq!(homeless.wanted().len(), 1);
    assert_eq!(turns(&homeless), [unknown(LOG_CHAT)]);
}

/// The exact rule: a chat works from its model call until a cancel line for it in its `agy.exe`'s log at or after
/// the moment `agy` started that call's hook; a later call works again; a cancel of another chat changes nothing.
#[test]
fn a_chat_works_until_a_cancel_line_at_or_after_its_hooks_start() {
    let mut source = source();
    let log = log_bytes();
    let split = log
        .windows(14)
        .position(|w| w == b"I1106 18:58:29")
        .unwrap();

    // Turn 2 of the log: Enter at 18:56:14.9, its hook at 14.95, Esc logged at 18:56:16.014209.
    feed(&mut source, call_file(LOG_CHAT, at(18, 56, 14, 950_000)));
    assert_eq!(turns(&source), [working(LOG_CHAT)], "until the log is read");
    feed(
        &mut source,
        (log_id(), Feed::Appended(log[..split].to_vec())),
    );
    assert_eq!(turns(&source), [ended(LOG_CHAT)]);

    // Turn 3: a new call after that Esc works, until its own Esc comes in the next lines.
    feed(&mut source, call_file(LOG_CHAT, at(18, 58, 29, 650_000)));
    assert_eq!(turns(&source), [working(LOG_CHAT)]);
    feed(
        &mut source,
        (log_id(), Feed::Appended(log[split..].to_vec())),
    );
    assert_eq!(turns(&source), [ended(LOG_CHAT)]);

    // The edge, to the microsecond: a cancel logged at the hook's start ends it, one logged before does not.
    for (turn_at, want) in [
        (at(18, 59, 1, 211_207), ended(LOG_CHAT)),
        (at(18, 59, 1, 211_208), working(LOG_CHAT)),
        (at(19, 20, 0, 0), working(LOG_CHAT)),
    ] {
        feed(&mut source, call_file(LOG_CHAT, turn_at));
        assert_eq!(turns(&source), [want], "{turn_at:?}");
    }

    // The other chat's cancel (19:18:27.9) ends only that chat. The "Cancelling steps" line after it ends nothing.
    feed(&mut source, call_file(OTHER_CHAT, at(19, 18, 20, 0)));
    assert_eq!(turns(&source), [working(LOG_CHAT), ended(OTHER_CHAT)]);

    // The session is the agy.exe line 1 names, with no task: a background command is not held.
    let sessions = source.sessions();
    assert!(sessions.iter().all(|s| s.liveness
        == Liveness::Process {
            pid: AGY.pid,
            start: AGY.start
        }
        && s.open_tasks == 0
        && s.label == CLI_LABEL
        && s.timers.is_empty()));

    // `Stop` deletes the file: the session ends, whatever the log says.
    feed(&mut source, (chat_id(LOG_CHAT), Feed::Gone));
    assert_eq!(turns(&source), [ended(OTHER_CHAT)]);
}

/// A tail that restarts or goes away is fed `Gone`, and its lines come again from byte 0.
#[test]
fn a_log_fed_again_from_its_start_gives_the_same_answer() {
    let mut source = source();
    feed(&mut source, call_file(LOG_CHAT, at(18, 58, 29, 650_000)));
    feed(&mut source, (log_id(), Feed::Appended(log_bytes())));
    assert_eq!(turns(&source), [ended(LOG_CHAT)]);
    feed(&mut source, (log_id(), Feed::Gone));
    assert_eq!(turns(&source), [working(LOG_CHAT)], "not read again yet");
    // Line by line, as the tail may hand them over.
    for line in log_bytes().split_inclusive(|&b| b == b'\n') {
        feed(&mut source, (log_id(), Feed::Appended(line.to_vec())));
    }
    assert_eq!(turns(&source), [ended(LOG_CHAT)]);
    // A log no chat names any more is dropped, so one named again is read from its start.
    feed(&mut source, (chat_id(LOG_CHAT), Feed::Gone));
    feed(&mut source, call_file(LOG_CHAT, at(18, 58, 29, 650_000)));
    assert_eq!(turns(&source), [working(LOG_CHAT)]);
}

/// The guard (Haik, 2026-11-06): a log that does not look as tested, or none named, holds nothing and shows "update
/// Wakeshot"; it never guesses either way.
#[test]
fn a_log_not_as_tested_holds_nothing_and_says_update_wakeshot() {
    let log = String::from_utf8(log_bytes()).unwrap();
    let line = |n: usize| log.lines().nth(n).unwrap().to_owned();
    let with = |n: usize, text: &str| {
        let mut lines: Vec<String> = log.lines().map(str::to_owned).collect();
        lines[n] = text.to_owned();
        lines.join("\n") + "\n"
    };
    let cancel = line(9);
    let cases = [
        (
            "an untested version",
            with(1, &line(1).replace("1.5.2", "1.5.3")),
        ),
        (
            "line 2 of another shape",
            with(1, &line(1).replace("version:", "version is")),
        ),
        (
            "line 1 of another shape",
            with(0, &line(0).replace("pid 51384", "id 51384")),
        ),
        (
            "line 1 of another process",
            with(0, &line(0).replace("51384", "51385")),
        ),
        (
            "a cancel line of another shape",
            with(9, &cancel.replace("=false)", "=maybe)")),
        ),
        (
            "a cancel line of another level",
            with(9, &cancel.replacen('I', "W", 1)),
        ),
        ("only line 1", line(0) + "\n"),
        ("a glog line 1 of no known message", with(0, &line(2))),
    ];
    for (case, text) in cases {
        let mut source = source();
        feed(&mut source, call_file(LOG_CHAT, at(19, 20, 0, 0)));
        feed(&mut source, (log_id(), Feed::Appended(text.into_bytes())));
        assert_eq!(turns(&source), [unknown(LOG_CHAT)], "{case}");
    }

    // A file the hook could name no log for: the same.
    let mut source = source();
    let plain = hooked::state_text(AGY, "", CLI_LABEL, State::Working);
    feed(&mut source, (chat_id(LOG_CHAT), Feed::Whole(plain)));
    assert_eq!(turns(&source), [unknown(LOG_CHAT)]);
    // The hook's problem files still say why while their process lives.
    let problem = hooked::problem_text(AGY, Problem::Input);
    feed(
        &mut source,
        (
            FileId(folder().join(hooked::PROBLEM_FILE)),
            Feed::Whole(problem),
        ),
    );
    assert_eq!(
        turns(&source)[1],
        (
            "problem".to_owned(),
            Activity::Idle,
            FormatHealth::Unreadable("Antigravity hook input")
        )
    );
}

/// Only the CLI is held in v1, and background commands are not held: an IDE file or a background file from an earlier
/// build holds nothing, and a forged label gives no tooltip text.
#[test]
fn an_ide_or_background_file_holds_nothing() {
    let mut source = source();
    let hub = ProcessRef {
        pid: 18_008,
        start: FileTime(134_381_200_000_000_000),
    };
    let ide =
        hooked::state_text_marked(hub, "", LABEL, State::Working, Some(&mark(at(19, 0, 0, 0))));
    feed(&mut source, (chat_id(IDE_CHAT), Feed::Whole(ide)));
    let forged = hooked::state_text_marked(
        AGY,
        "",
        "Antigravity CLI\" & calc",
        State::Working,
        Some(&mark(at(19, 0, 0, 0))),
    );
    feed(&mut source, (chat_id(OTHER_CHAT), Feed::Whole(forged)));
    let background = hooked::state_text(AGY, "", CLI_LABEL, State::Background);
    feed(&mut source, (chat_id(CLI_CHAT), Feed::Whole(background)));
    assert_eq!(turns(&source), [ended(CLI_CHAT)]);
    assert_eq!(source.sessions()[0].open_tasks, 0);
    assert_eq!(source.wanted().len(), 1, "no tail for any of them");
}

/// The log holds every prompt: the source keeps only the pid, the version check and the cancels' chat ids and times,
/// so no byte of another line is in what it holds or gives.
#[test]
fn nothing_of_a_log_line_but_a_cancels_chat_and_time_is_kept() {
    let mut source = source();
    feed(&mut source, call_file(LOG_CHAT, at(19, 20, 0, 0)));
    let mut log = log_bytes();
    for text in [
        format!(
            "I1106 19:21:00.000000 176 input_loop.go:301] HandleUserInput called with text: {MARKER}\n"
        ),
        format!("{MARKER} as a prompt's next line\n"),
        format!("I1106 19:21:01.000000 9 model.go:1] {MARKER} {MARKER_NUMBER}\n"),
        format!(
            "I1106 19:21:02.000000 9 server.go:2498] Cancelling conversation {MARKER} {MARKER_NUMBER}\n"
        ),
    ] {
        log.extend(text.into_bytes());
    }
    feed(&mut source, (log_id(), Feed::Appended(log)));
    let held = format!("{source:?} {:?}", source.sessions());
    assert_clean(&held, "the source");
    assert!(held.contains(LOG_CHAT), "the cancels are kept: {held}");
    assert!(!held.contains("HandleUserInput") && !held.contains("server.go"));
    // That near-miss cancel line broke the shape: the guard, not a guess.
    assert_eq!(turns(&source), [unknown(LOG_CHAT)]);
}

/// glog writes no year: a cancel just after New Year compares with a hook started just before it, and the zone's
/// offset at the hook's start is the one used.
#[test]
fn a_cancel_after_new_year_and_another_zone_compare_by_local_time() {
    let head = "I1231 23:50:00.000000 51 server.go:2275] Starting language server process with pid 51384\n\
                I1231 23:50:00.000001 51 server.go:2333] Language server version: 1.5.2\n";
    let cancel = format!(
        "I0101 00:00:00.500000 9 server.go:2498] Cancelling conversation {LOG_CHAT} (killBackgroundTasks=false)\n"
    );
    for (offset, turn_at, want) in [
        (
            -300,
            filetime((2026, 12, 31), (23, 59, 59, 0), -300),
            ended(LOG_CHAT),
        ),
        (
            -300,
            filetime((2027, 1, 1), (0, 0, 1, 0), -300),
            working(LOG_CHAT),
        ),
        (
            330,
            filetime((2026, 12, 31), (23, 59, 59, 999_999), 330),
            ended(LOG_CHAT),
        ),
    ] {
        let mut source = source_in(Box::new(FixedOffset(offset)));
        feed(&mut source, call_file(LOG_CHAT, turn_at));
        let log = format!("{head}{cancel}").into_bytes();
        feed(&mut source, (log_id(), Feed::Appended(log)));
        assert_eq!(turns(&source), [want], "{offset} {turn_at:?}");
    }
}

#[test]
fn a_file_of_another_agent_or_name_is_never_read() {
    let mut source = source();
    let (_, text) = call_file(CLI_CHAT, at(19, 0, 0, 0));
    let Feed::Whole(text) = text else {
        unreachable!()
    };
    let cursor_file = data_root()
        .join("agents")
        .join("cursor")
        .join(hooked::chat_file(CLI_CHAT));
    for file in [
        FileId(cursor_file),
        FileId(folder().join(format!("{}.json", CLI_CHAT.to_uppercase()))),
        FileId(folder().join("notes.json")),
    ] {
        assert_eq!(source.feed(&file, Feed::Whole(text.clone())), Fed::Taken);
    }
    assert!(source.sessions().is_empty());
    // A log of another name or folder is never kept either.
    feed(&mut source, call_file(LOG_CHAT, at(18, 56, 14, 950_000)));
    for file in [
        FileId(antigravity::log_dir(&profile()).join("cli.log")),
        FileId(profile().join(LOG_NAME)),
        FileId(antigravity::log_dir(&profile()).join("cli-20261106_185449.log")),
    ] {
        feed(&mut source, (file, Feed::Appended(log_bytes())));
    }
    assert_eq!(turns(&source), [working(LOG_CHAT)]);
}

#[test]
fn a_hook_input_is_never_a_state_file() {
    for (name, _) in FIXTURES {
        assert_eq!(hooked::read_state(&fixture(name)), FileRead::Bad, "{name}");
    }
    let input = HookInput {
        chat: CLI_CHAT.to_owned(),
        turn: String::new(),
        idle: Some(false),
    };
    assert_eq!(
        hooked::resolve(AGENT.event("Stop").unwrap(), &input),
        Ok(Op::Delete)
    );
}

/// One step of the live turn: the fixture, its event, what it does to the chat's file, and the sessions the source then
/// gives, as `activity`.
type LiveStep = (&'static str, &'static str, Op, &'static [Activity]);

/// The live turn in `fixtures/antigravity/live/`, in order (`p4-live-antigravity`, 2026-11-06).
const LIVE: [LiveStep; 5] = [
    (
        "turn-1-pre-invocation.json",
        "PreInvocation",
        Op::Write(State::Working),
        &[Activity::Working],
    ),
    (
        "turn-2-pre-invocation.json",
        "PreInvocation",
        Op::Write(State::Working),
        &[Activity::Working],
    ),
    ("turn-3-stop-background.json", "Stop", Op::Delete, &[]),
    (
        "turn-4-pre-invocation-woken.json",
        "PreInvocation",
        Op::Write(State::Working),
        &[Activity::Working],
    ),
    ("turn-5-stop-idle.json", "Stop", Op::Delete, &[]),
];

const LIVE_CHAT: &str = "7f3c9a21-5b6e-4d08-9c2a-e41b0d6f8a53";

fn live_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("antigravity")
        .join("live")
}

fn live(name: &str) -> Vec<u8> {
    let path = live_dir().join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

#[test]
fn every_live_fixture_is_listed_once() {
    let mut on_disk: Vec<String> = std::fs::read_dir(live_dir())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .filter(|name| name.ends_with(".json"))
        .collect();
    on_disk.sort();
    let mut listed: Vec<String> = LIVE.iter().map(|(name, ..)| (*name).to_owned()).collect();
    listed.sort();
    assert_eq!(on_disk, listed);
}

/// A real turn, replayed as the hook and the source handle it, with a log of that process's head and no cancel: each
/// model call writes its own start, the loop's end with a command still running ends the chat (background commands
/// are not held), the command's end wakes the agent into a new loop, and that loop's end deletes the file.
#[test]
fn the_live_turn_replays_into_the_sessions_the_rule_gives() {
    let process = ProcessRef {
        pid: 40_292,
        start: FileTime(134_384_452_138_799_879),
    };
    let mut source = source();
    let log = format!(
        "I1106 17:26:54.100000 51 server.go:2275] Starting language server process with pid {}\n\
         I1106 17:26:54.100001 51 server.go:2333] Language server version: 1.5.2\n",
        process.pid
    );
    let mut writes = 0;
    for (n, (name, event, want, sessions)) in LIVE.into_iter().enumerate() {
        let parsed = parse(event, &live(name));
        for word in [
            "app test",
            "C:/Users/dev",
            "gemini",
            "NO_TOOL_CALL",
            "brain",
        ] {
            assert!(!format!("{parsed:?}").contains(word), "{name}: {word}");
        }
        assert_eq!(parsed.version, None, "{name}");
        let input = parsed.input.unwrap_or_else(|p| panic!("{name}: {p}"));
        assert_eq!((input.chat.as_str(), input.turn.as_str()), (LIVE_CHAT, ""));
        let op = hooked::resolve(AGENT.event(event).unwrap(), &input).unwrap();
        assert_eq!(op, want, "{name}");
        match op {
            Op::Write(state) => {
                let turn_at = at(17, 28, i64::try_from(n).unwrap(), 0);
                let text = hooked::state_text_marked(
                    process,
                    &input.turn,
                    CLI_LABEL,
                    state,
                    Some(&mark(turn_at)),
                );
                writes += 1;
                feed(&mut source, (chat_id(LIVE_CHAT), Feed::Whole(text)));
                if writes == 1 {
                    feed(
                        &mut source,
                        (log_id(), Feed::Appended(log.clone().into_bytes())),
                    );
                }
            }
            Op::Delete => feed(&mut source, (chat_id(LIVE_CHAT), Feed::Gone)),
            other => panic!("{name}: {other:?}"),
        }
        let seen: Vec<_> = source
            .sessions()
            .iter()
            .map(|s| {
                assert_eq!(
                    (s.key.id.as_str(), s.label, s.format.clone(), s.open_tasks),
                    (LIVE_CHAT, CLI_LABEL, FormatHealth::Ok, 0)
                );
                assert_eq!(
                    s.liveness,
                    Liveness::Process {
                        pid: process.pid,
                        start: process.start
                    }
                );
                s.activity
            })
            .collect();
        assert_eq!(seen, sessions, "{name}");
    }
    assert_eq!(writes, 3, "each model call writes its own start");
}

#[test]
fn each_model_call_notes_its_own_start_and_nothing_else_differs() {
    let calls: Vec<_> = [at(18, 0, 0, 0), at(18, 0, 3, 0)]
        .into_iter()
        .map(|turn_at| {
            let input = parse("PreInvocation", &fixture("cli-pre-invocation.json"))
                .input
                .unwrap();
            hooked::state_text_marked(
                AGY,
                &input.turn,
                CLI_LABEL,
                State::Working,
                Some(&mark(turn_at)),
            )
        })
        .collect();
    let files: Vec<_> = calls
        .iter()
        .map(|text| match hooked::read_state(text) {
            FileRead::V1(file) => file,
            other => panic!("{other:?}"),
        })
        .collect();
    assert_ne!(files[0].turn_at, files[1].turn_at);
    let mut same = files[1].clone();
    same.turn_at = files[0].turn_at;
    assert_eq!(same, files[0]);
    assert_eq!(
        (
            files[0].pid,
            files[0].generation.as_str(),
            files[0].label.as_str(),
            files[0].log.as_deref()
        ),
        (AGY.pid, "", CLI_LABEL, Some(LOG_NAME))
    );
}
