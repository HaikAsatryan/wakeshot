//! Antigravity's table and source on the hook inputs in `fixtures/antigravity/` (`docs/agent-hooks.md`,
//! "Antigravity"): what the hook keeps of each event, the walk to `agy.exe`, the files and the sessions they give, and
//! the privacy rule that no input value but the chat id ever reaches a file or a log text.
//!
//! Owner: `p4-antigravity-source`.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use wakeshot_core::process::{FileTime, ProcessRef};
use wakeshot_core::sources::antigravity::{self, AGENT, CLI_LABEL, LABEL};
use wakeshot_core::sources::hooked::{
    self, Bases, Checked, FileRead, HookInput, InputProblem, Listed, Op, Parsed, Problem, State,
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
        let want = match name {
            "cli-stop-background.json" => Op::Write(State::Background),
            _ if event == "Stop" => Op::Delete,
            _ => Op::Write(State::Working),
        };
        assert_eq!(op(event, &fixture(name)), Ok(want), "{name}");
        let idle = match event {
            "Stop" => Some(name != "cli-stop-background.json"),
            _ => None,
        };
        assert_eq!(input.idle, idle, "{name}");
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

#[test]
fn a_stop_without_a_plain_idle_flag_is_unreadable_and_a_model_call_needs_none() {
    let stop: Value = serde_json::from_slice(&fixture("cli-stop-idle.json")).unwrap();
    let mut no_flag = stop.clone();
    no_flag.as_object_mut().unwrap().remove("fullyIdle");
    let mut null_flag = stop.clone();
    null_flag["fullyIdle"] = Value::Null;
    for case in [&no_flag, &null_flag] {
        assert_eq!(
            op("Stop", case.to_string().as_bytes()),
            Err(InputProblem::NoIdle)
        );
    }
    let mut word = stop.clone();
    word["fullyIdle"] = json!("true");
    assert!(matches!(
        parse("Stop", word.to_string().as_bytes()).input,
        Err(InputProblem::Parse { class: "data", .. })
    ));
    // The model call's input has no flag, and one there changes nothing.
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
                    texts.push(hooked::state_text(process, &input.turn, CLI_LABEL, state));
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
            // The kept fields, as the marker, are each refused rather than kept.
            if input[&field] == json!(MARKER) {
                match field.as_str() {
                    "conversationId" => {
                        assert_eq!(parsed.input, Err(InputProblem::ChatShape), "{case}");
                    }
                    "fullyIdle" => assert!(
                        matches!(parsed.input, Err(InputProblem::Parse { .. })),
                        "{case}"
                    ),
                    _ => {}
                }
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
        walk.parent.map(|p| p.pid),
        Some(800),
        "the transport check reads cmd's command line"
    );
    let found = walk.agent.unwrap();
    assert_eq!(found.process.pid, 700);
    assert_eq!((found.image.key, found.image.label), ("cli", CLI_LABEL));
}

#[test]
fn the_ides_hook_names_its_hub_and_never_a_farther_agy() {
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

    let nested = [
        listed(900, 800, "wakeshot.exe"),
        listed(800, 750, "cmd.exe"),
        listed(750, 700, "AGY.EXE"),
        listed(700, 1, "agy.exe"),
    ];
    let walk = hooked::walk(&AGENT, &nested, me(), &bases(), |_| Some(cli_agy()));
    assert_eq!(walk.agent.unwrap().process.pid, 750, "the nearest one");

    // An `agy.exe` whose path does not read is still Antigravity, under its plain name.
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
fn the_checked_values_carry_the_stops_two_outcomes_and_nothing_else() {
    let process = ProcessRef {
        pid: 700,
        start: FileTime(7_000),
    };
    let stop = AGENT.event("Stop").unwrap();
    let call = AGENT.event("PreInvocation").unwrap();
    let values = |op: &str, key: &str| {
        [CLI_CHAT, "", "700", "7000", key, op, "1762400002000"]
            .map(str::to_owned)
            .to_vec()
    };
    let background = Checked::parse(&AGENT, stop, &values("background", "cli")).unwrap();
    assert_eq!(background.op, Op::Write(State::Background));
    assert_eq!(background.process, process);
    assert_eq!(background.image.label, CLI_LABEL);
    assert_eq!(background.values(), values("background", "cli"));
    let deleted = Checked::parse(&AGENT, stop, &values("none", "app")).unwrap();
    assert_eq!((deleted.op, deleted.image.label), (Op::Delete, LABEL));
    let working = Checked::parse(&AGENT, call, &values("working", "cli")).unwrap();
    assert_eq!(working.op, Op::Write(State::Working));
    // What the event itself cannot do, and a key no rule has.
    for (rule, op, key) in [
        (stop, "working", "cli"),
        (call, "background", "cli"),
        (call, "none", "cli"),
        (stop, "none", "editor"),
    ] {
        assert_eq!(
            Checked::parse(&AGENT, rule, &values(op, key)),
            None,
            "{op} {key}"
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

/// The liveness pairing: every session holds only while the `agy.exe` the hook found runs, with that pid and start,
/// since Antigravity sends no session end.
#[test]
fn the_source_reads_the_hooks_files_into_sessions_of_the_agy_that_runs_them() {
    let mut source = antigravity::source(&data_root());
    assert_eq!(source.id(), SourceId::Antigravity);
    let wanted = source.wanted();
    let [WantedFile::Folder { dir, accept }] = &wanted[..] else {
        panic!("one folder");
    };
    assert_eq!(dir, &folder());
    assert!(accept(&format!("{CLI_CHAT}.json")));
    assert!(!accept(&format!(
        "{CLI_CHAT}.json.900.0.0123456789abcdef.tmp"
    )));

    let cli = ProcessRef {
        pid: 31_204,
        start: FileTime(134_381_211_391_959_336),
    };
    let hub = ProcessRef {
        pid: 18_008,
        start: FileTime(134_381_200_000_000_000),
    };
    let forged = "0c1d2e3f-4a5b-4c6d-8e9f-a0b1c2d3e4f5";
    for (chat, process, label, state) in [
        (CLI_CHAT, cli, CLI_LABEL, State::Working),
        (IDE_CHAT, hub, LABEL, State::Background),
        (forged, cli, "Antigravity\" & calc", State::Working),
    ] {
        let text = hooked::state_text(process, "", label, state);
        assert_eq!(source.feed(&chat_id(chat), Feed::Whole(text)), Fed::Taken);
    }
    for (name, problem) in [
        (hooked::PROBLEM_FILE, Problem::Input),
        (hooked::BLOCKED_FILE, Problem::CommandLine),
    ] {
        let text = hooked::problem_text(hub, problem);
        let fed = source.feed(&FileId(folder().join(name)), Feed::Whole(text));
        assert_eq!(fed, Fed::Taken);
    }
    let sessions = source.sessions();
    let seen: Vec<_> = sessions
        .iter()
        .map(|s| {
            (
                s.key.id.as_str(),
                s.label,
                s.activity,
                s.open_tasks,
                s.liveness.clone(),
                s.format.clone(),
            )
        })
        .collect();
    let alive = |p: ProcessRef| Liveness::Process {
        pid: p.pid,
        start: p.start,
    };
    assert_eq!(
        seen,
        [
            (
                forged,
                LABEL,
                Activity::Working,
                0,
                alive(cli),
                FormatHealth::Ok
            ),
            (
                IDE_CHAT,
                LABEL,
                Activity::Idle,
                1,
                alive(hub),
                FormatHealth::Ok
            ),
            (
                "blocked",
                LABEL,
                Activity::Idle,
                0,
                alive(hub),
                FormatHealth::Unreadable("Antigravity hook command line")
            ),
            (
                CLI_CHAT,
                CLI_LABEL,
                Activity::Working,
                0,
                alive(cli),
                FormatHealth::Ok
            ),
            (
                "problem",
                LABEL,
                Activity::Idle,
                0,
                alive(hub),
                FormatHealth::Unreadable("Antigravity hook input")
            ),
        ]
    );
    assert!(
        sessions
            .iter()
            .all(|s| s.key.source == SourceId::Antigravity && s.timers.is_empty())
    );

    // The hook's delete is the session's end.
    source.feed(&chat_id(CLI_CHAT), Feed::Gone);
    assert!(source.sessions().iter().all(|s| s.key.id != CLI_CHAT));
}

#[test]
fn a_file_of_another_agent_or_name_is_never_read() {
    let mut source = antigravity::source(&data_root());
    let process = ProcessRef {
        pid: 1,
        start: FileTime(1),
    };
    let text = hooked::state_text(process, "", LABEL, State::Working);
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
}

#[test]
fn a_hook_input_is_never_a_state_file() {
    for (name, _) in FIXTURES {
        assert_eq!(hooked::read_state(&fixture(name)), FileRead::Bad, "{name}");
    }
    let input = HookInput {
        chat: CLI_CHAT.to_owned(),
        turn: String::new(),
        idle: Some(true),
    };
    assert_eq!(
        hooked::resolve(AGENT.event("Stop").unwrap(), &input),
        Ok(Op::Delete)
    );
}

/// One step of the live turn: the fixture, its event, what it does to the chat's file, and the sessions the source then
/// gives, as `activity` and `open_tasks`.
type LiveStep = (&'static str, &'static str, Op, &'static [(Activity, u32)]);

/// The live turn in `fixtures/antigravity/live/`, in order (`p4-live-antigravity`, 2026-11-06).
const LIVE: [LiveStep; 5] = [
    (
        "turn-1-pre-invocation.json",
        "PreInvocation",
        Op::Write(State::Working),
        &[(Activity::Working, 0)],
    ),
    (
        "turn-2-pre-invocation.json",
        "PreInvocation",
        Op::Write(State::Working),
        &[(Activity::Working, 0)],
    ),
    (
        "turn-3-stop-background.json",
        "Stop",
        Op::Write(State::Background),
        &[(Activity::Idle, 1)],
    ),
    (
        "turn-4-pre-invocation-woken.json",
        "PreInvocation",
        Op::Write(State::Working),
        &[(Activity::Working, 0)],
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

/// A real turn, replayed as the hook and the source handle it: the second model call writes nothing, the loop's end
/// with a command still running holds as one open task, the command's end wakes the agent into a new loop, and that
/// loop's end deletes the file. What the parse keeps holds none of the input's other values.
#[test]
fn the_live_turn_replays_into_the_sessions_the_live_test_saw() {
    let process = ProcessRef {
        pid: 40_292,
        start: FileTime(134_384_452_138_799_879),
    };
    let mut source = antigravity::source(&data_root());
    let mut on_disk: Option<Vec<u8>> = None;
    let mut writes = 0;
    for (name, event, want, sessions) in LIVE {
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
            // The hook skips a write whose bytes the file already holds.
            Op::Write(state) => {
                let text = hooked::state_text(process, &input.turn, CLI_LABEL, state);
                if on_disk.as_deref() != Some(text.as_slice()) {
                    writes += 1;
                    on_disk = Some(text.clone());
                    let fed = source.feed(&chat_id(LIVE_CHAT), Feed::Whole(text));
                    assert_eq!(fed, Fed::Taken, "{name}");
                }
            }
            Op::Delete => {
                on_disk = None;
                source.feed(&chat_id(LIVE_CHAT), Feed::Gone);
            }
            other => panic!("{name}: {other:?}"),
        }
        let seen: Vec<_> = source
            .sessions()
            .iter()
            .map(|s| {
                assert_eq!(
                    (s.key.id.as_str(), s.label, s.format.clone()),
                    (LIVE_CHAT, CLI_LABEL, FormatHealth::Ok)
                );
                assert_eq!(
                    s.liveness,
                    Liveness::Process {
                        pid: process.pid,
                        start: process.start
                    }
                );
                (s.activity, s.open_tasks)
            })
            .collect();
        assert_eq!(seen, sessions, "{name}");
    }
    assert_eq!(
        writes, 3,
        "the second model call of the loop writes nothing"
    );
    assert_eq!(on_disk, None);
}

#[test]
fn every_model_call_of_one_process_writes_the_same_bytes() {
    // So the hook's compare skips every write after the first of a turn (`Action::Write`).
    let process = ProcessRef {
        pid: 31_204,
        start: FileTime(134_381_211_391_959_336),
    };
    let calls: Vec<_> = (0..3)
        .map(|_| {
            let input = parse("PreInvocation", &fixture("cli-pre-invocation.json"))
                .input
                .unwrap();
            hooked::state_text(process, &input.turn, CLI_LABEL, State::Working)
        })
        .collect();
    assert!(calls.windows(2).all(|pair| pair[0] == pair[1]));
    let FileRead::V1(file) = hooked::read_state(&calls[0]) else {
        panic!("a state file");
    };
    assert_eq!(
        (file.pid, file.generation.as_str(), file.label.as_str()),
        (31_204, "", CLI_LABEL)
    );
}
