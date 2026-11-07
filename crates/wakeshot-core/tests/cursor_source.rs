//! Cursor's table and source on the hook inputs in `fixtures/cursor/` (`docs/agent-hooks.md`, "Cursor"): what the hook
//! keeps of each event, the walk to the agent process, the files and the sessions they give, and the privacy rule that
//! no input value but the two ids ever reaches a file or a log text.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use wakeshot_core::process::{FileTime, ProcessRef};
use wakeshot_core::sources::cursor::{self, AGENT, CLI_LABEL, LABEL};
use wakeshot_core::sources::hooked::{
    self, Bases, FileRead, HookInput, InputProblem, Listed, Op, Parsed, Problem, State,
};
use wakeshot_core::sources::{
    Activity, AgentSource, Fed, Feed, FileId, FormatHealth, Liveness, SourceId, WantedFile,
};

const EDITOR_CHAT: &str = "a3f1c2d4-5b6e-4f70-8a9b-0c1d2e3f4a5b";
const EDITOR_TURN: &str = "7d2e9c41-0b8a-4e55-9f13-6a2c4b7d8e90";

/// The fixtures with the event each one is for.
const FIXTURES: [(&str, &str); 5] = [
    ("editor-before-submit-prompt.json", "beforeSubmitPrompt"),
    ("editor-stop-completed.json", "stop"),
    ("editor-stop-aborted.json", "stop"),
    ("editor-session-end.json", "sessionEnd"),
    ("cli-before-submit-prompt.json", "beforeSubmitPrompt"),
];

/// A word with a space, so it is neither a chat id, a turn id nor a version: none of the kept fields may keep it.
const MARKER: &str = "QXV7 marker";
const MARKER_NUMBER: u64 = 77_317_731;

fn fixture(name: &str) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("cursor")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn parse(event: &str, bytes: &[u8]) -> Parsed {
    hooked::parse_input(&AGENT, event, bytes)
}

#[test]
fn each_fixture_keeps_its_ids_lowercased_and_its_version() {
    for (name, event) in FIXTURES {
        let parsed = parse(event, &fixture(name));
        let input = parsed.input.unwrap_or_else(|p| panic!("{name}: {p}"));
        let rule = AGENT.event(event).unwrap();
        let op = hooked::resolve(rule, &input).unwrap();
        match name {
            "cli-before-submit-prompt.json" => {
                assert_eq!(input.chat, "e4b7a1c9-2d3f-4a6b-8c9d-1e2f3a4b5c6d");
                assert_eq!(parsed.version.as_deref(), Some("2026.11.05-86047df"));
            }
            "editor-session-end.json" => {
                assert_eq!(input.chat, EDITOR_CHAT);
                assert_eq!(input.turn, "", "a session end names no turn");
            }
            _ => {
                assert_eq!(input.chat, EDITOR_CHAT, "{name}");
                assert_eq!(input.turn, EDITOR_TURN, "{name}");
                assert_eq!(parsed.version.as_deref(), Some("3.27.12"));
            }
        }
        let want = match event {
            "beforeSubmitPrompt" => Op::Write(State::Working),
            "stop" => Op::DeleteIfTurn,
            _ => Op::Delete,
        };
        assert_eq!(op, want, "{name}");
        assert_eq!(input.idle, None);
    }
}

#[test]
fn an_input_for_another_event_is_unreadable() {
    let bytes = fixture("editor-stop-completed.json");
    for event in ["beforeSubmitPrompt", "sessionEnd"] {
        assert_eq!(parse(event, &bytes).input, Err(InputProblem::OtherEvent));
    }
}

#[test]
fn the_stop_of_the_turn_that_wrote_the_file_deletes_it_and_no_other_does() {
    let process = ProcessRef {
        pid: 23_816,
        start: FileTime(134_381_211_391_959_336),
    };
    let written = hooked::state_text(process, EDITOR_TURN, LABEL, State::Working);
    let stop = parse("stop", &fixture("editor-stop-completed.json"))
        .input
        .unwrap();
    assert!(hooked::names_turn(&written, &stop.turn));
    let newer = hooked::state_text(
        process,
        "11111111-2222-4333-8444-555555555555",
        LABEL,
        State::Working,
    );
    assert!(!hooked::names_turn(&newer, &stop.turn));
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
    // As a key, too.
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
/// holds no input value but the checked ids.
#[test]
fn the_marker_in_any_field_never_reaches_a_file_or_a_log_text() {
    let process = ProcessRef {
        pid: 4_242,
        start: FileTime(134_000_000_000_000_001),
    };
    let mut written = 0;
    for (name, event) in FIXTURES {
        let base: Value = serde_json::from_slice(&fixture(name)).unwrap();
        let rule = AGENT.event(event).unwrap();
        for (field, input) in marked(&base) {
            let case = format!("{name}, {field} = {}", input[&field]);
            let parsed = parse(event, input.to_string().as_bytes());
            assert_clean(&format!("{parsed:?}"), &case);
            let mut texts = Vec::new();
            match &parsed.input {
                Ok(input) => {
                    let op = hooked::resolve(rule, input).unwrap();
                    if let Op::Write(state) = op {
                        texts.push(hooked::state_text(process, &input.turn, LABEL, state));
                        written += 1;
                    }
                }
                Err(problem) => {
                    texts.push(problem.to_string().into_bytes());
                    texts.push(hooked::problem_text(process, Problem::Input));
                }
            }
            if let Some(version) = &parsed.version {
                texts.push(version.clone().into_bytes());
            }
            for text in texts {
                assert_clean(&String::from_utf8(text).unwrap(), &case);
            }
            // The kept fields, as a string, are each refused rather than kept.
            if input[&field] == json!(MARKER) {
                let refused = match field.as_str() {
                    "conversation_id" => Some(InputProblem::ChatShape),
                    "generation_id" => Some(InputProblem::TurnShape),
                    "hook_event_name" => Some(InputProblem::OtherEvent),
                    _ => None,
                };
                if let Some(problem) = refused {
                    assert_eq!(parsed.input, Err(problem), "{case}");
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

fn local() -> PathBuf {
    PathBuf::from("Users")
        .join("dev")
        .join("AppData")
        .join("Local")
}

fn bases() -> Bases {
    Bases {
        local_app_data: Some(local()),
        profile: None,
    }
}

#[test]
fn the_editors_hook_names_the_nearest_cursor_process() {
    // Hook, its PowerShell, the extension host, the main window process.
    let list = [
        Listed {
            pid: 900,
            parent: 800,
            start: FileTime(9_000),
            image: "wakeshot.exe",
        },
        Listed {
            pid: 800,
            parent: 700,
            start: FileTime(8_000),
            image: "powershell.exe",
        },
        Listed {
            pid: 700,
            parent: 600,
            start: FileTime(7_000),
            image: "Cursor.exe",
        },
        Listed {
            pid: 600,
            parent: 1,
            start: FileTime(6_000),
            image: "Cursor.exe",
        },
    ];
    let walk = hooked::walk(&AGENT, &list, me(), &bases(), |_| None);
    assert_eq!(walk.parent.map(|p| p.pid), Some(800));
    let found = walk.agent.unwrap();
    assert_eq!(found.process.pid, 700);
    assert_eq!((found.image.key, found.image.label), ("editor", LABEL));
}

#[test]
fn the_clis_hook_names_its_node_and_any_other_node_is_passed_by() {
    let cli_node = local()
        .join("cursor-agent")
        .join("versions")
        .join("2026.11.05-86047df")
        .join("node.exe");
    let other_node = PathBuf::from("Program Files")
        .join("nodejs")
        .join("node.exe");
    let list = [
        Listed {
            pid: 900,
            parent: 800,
            start: FileTime(9_000),
            image: "wakeshot.exe",
        },
        Listed {
            pid: 800,
            parent: 750,
            start: FileTime(8_000),
            image: "powershell.exe",
        },
        Listed {
            pid: 750,
            parent: 700,
            start: FileTime(7_500),
            image: "node.exe",
        },
        Listed {
            pid: 700,
            parent: 1,
            start: FileTime(7_000),
            image: "node.exe",
        },
    ];
    let walk = hooked::walk(&AGENT, &list, me(), &bases(), |p| {
        Some(if p.pid == 750 {
            other_node.clone()
        } else {
            cli_node.clone()
        })
    });
    let found = walk.agent.unwrap();
    assert_eq!((found.process.pid, found.image.label), (700, CLI_LABEL));
}

fn data_root() -> PathBuf {
    PathBuf::from("data")
}

fn chat_id(chat: &str) -> FileId {
    FileId(
        data_root()
            .join("agents")
            .join("cursor")
            .join(hooked::chat_file(chat)),
    )
}

#[test]
fn the_source_reads_the_hooks_files_into_cursor_sessions() {
    let mut source = cursor::source(&data_root());
    assert_eq!(source.id(), SourceId::Cursor);
    let wanted = source.wanted();
    let [WantedFile::Folder { dir, .. }] = &wanted[..] else {
        panic!("one folder");
    };
    assert_eq!(dir, &data_root().join("agents").join("cursor"));

    let process = ProcessRef {
        pid: 23_816,
        start: FileTime(134_381_211_391_959_336),
    };
    let cli_chat = "e4b7a1c9-2d3f-4a6b-8c9d-1e2f3a4b5c6d";
    let forged = "0c1d2e3f-4a5b-4c6d-8e9f-a0b1c2d3e4f5";
    for (chat, label) in [
        (EDITOR_CHAT, LABEL),
        (cli_chat, CLI_LABEL),
        (forged, "Cursor; rm -rf"),
    ] {
        let text = hooked::state_text(process, "", label, State::Working);
        assert_eq!(source.feed(&chat_id(chat), Feed::Whole(text)), Fed::Taken);
    }
    let problem = data_root()
        .join("agents")
        .join("cursor")
        .join(hooked::PROBLEM_FILE);
    source.feed(
        &FileId(problem),
        Feed::Whole(hooked::problem_text(process, Problem::Input)),
    );
    let sessions = source.sessions();
    let seen: Vec<_> = sessions
        .iter()
        .map(|s| (s.key.id.as_str(), s.label, s.activity, s.format.clone()))
        .collect();
    assert_eq!(
        seen,
        [
            (forged, LABEL, Activity::Working, FormatHealth::Ok),
            (EDITOR_CHAT, LABEL, Activity::Working, FormatHealth::Ok),
            (cli_chat, CLI_LABEL, Activity::Working, FormatHealth::Ok),
            (
                "problem",
                LABEL,
                Activity::Idle,
                FormatHealth::Unreadable("Cursor hook input")
            ),
        ]
    );
    assert!(sessions.iter().all(|s| s.key.source == SourceId::Cursor
        && s.liveness
            == Liveness::Process {
                pid: 23_816,
                start: FileTime(134_381_211_391_959_336),
            }
        && s.timers.is_empty()
        && s.open_tasks == 0));
}

#[test]
fn a_hook_input_is_never_a_state_file() {
    // A misplaced input file in the folder is no session: it has no version.
    for (name, _) in FIXTURES {
        assert_eq!(hooked::read_state(&fixture(name)), FileRead::Bad, "{name}");
    }
    let input = HookInput {
        chat: EDITOR_CHAT.to_owned(),
        turn: String::new(),
        idle: None,
    };
    assert_eq!(
        hooked::resolve(AGENT.event("sessionEnd").unwrap(), &input),
        Ok(Op::Delete)
    );
}
