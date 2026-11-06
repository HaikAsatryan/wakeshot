//! `VsCodeSource` fed the way the watch thread feeds a `Recent` want: each chat session log whole, again on each
//! change, `Gone` when deleted. The fixtures in `fixtures/vscode/` are built from the schema, those in its `live/`
//! are redacted real files. The source does no I/O, so the user folder is a made-up path and no real VS Code folder
//! is ever read.

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use proptest::prelude::*;
use wakeshot_core::sources::vscode::{
    CHAT_SESSIONS, INSIDERS, Install, KIND, LABEL, MODEL_STATE, STABLE, UNREADABLE, VsCodeSource,
    installs, is_session_file_name,
};
use wakeshot_core::sources::{
    Activity, AgentSource, Fed, Feed, FileId, FormatHealth, Liveness, SessionFacts, SessionKey,
    SourceId, WantedFile,
};

const PROFILE: &str = r"X:\Users\me";
const ROAMING: &str = r"X:\Users\me\AppData\Roaming";
const WORKSPACE: &str = "55992610aaaabbbbccccddddeeeeffff";
const ID: &str = "0b1c2d3e-0000-4000-8000-000000000001";
const OTHER: &str = "0b1c2d3e-0000-4000-8000-000000000002";

// Times in the fixtures (`fixtures/vscode/README.md`): request 1 sent at 10:00:05, request 3 at 10:10.
const T_R1: i64 = 1_793_959_205_000;
const T_R3: i64 = 1_793_959_800_000;

const WAITING: Activity = Activity::Waiting {
    since_wall_ms: None,
    remote: false,
};

const FIXTURES: [&str; 11] = [
    "working.jsonl",
    "waiting.jsonl",
    "working-again.jsonl",
    "idle.jsonl",
    "compacted.jsonl",
    "several-requests.jsonl",
    "reopened.jsonl",
    "unknown-kind.jsonl",
    "unknown-state.jsonl",
    "empty-window.jsonl",
    "partial-last-line.jsonl",
];

/// `<ROAMING>\Code\User`, one level per `join` as `installs` builds it: `\` is a separator on Windows only, so one
/// literal with backslashes would be another path than the source's on Linux.
fn user() -> PathBuf {
    Path::new(ROAMING).join("Code").join("User")
}

fn insiders_user() -> PathBuf {
    Path::new(ROAMING).join("Code - Insiders").join("User")
}

fn source() -> VsCodeSource {
    VsCodeSource::new(vec![Install {
        user_dir: user(),
        product: STABLE,
    }])
}

fn both_builds() -> VsCodeSource {
    VsCodeSource::new(installs(
        Path::new(PROFILE),
        Some(OsStr::new(ROAMING)),
        None,
    ))
}

fn in_workspace(user: &Path, id: &str) -> FileId {
    FileId(
        user.join("workspaceStorage")
            .join(WORKSPACE)
            .join(CHAT_SESSIONS)
            .join(format!("{id}.jsonl")),
    )
}

fn in_empty_window(user: &Path, id: &str) -> FileId {
    FileId(
        user.join("globalStorage")
            .join("emptyWindowChatSessions")
            .join(format!("{id}.jsonl")),
    )
}

fn chat(id: &str) -> FileId {
    in_workspace(&user(), id)
}

fn read_fixture(name: &str) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("vscode")
        .join(name);
    fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn only(s: &VsCodeSource) -> SessionFacts {
    let mut all = s.sessions();
    assert_eq!(all.len(), 1, "{all:?}");
    all.remove(0)
}

fn fed_bytes(bytes: &[u8]) -> SessionFacts {
    let mut s = source();
    assert_eq!(s.feed(&chat(ID), Feed::Whole(bytes.to_vec())), Fed::Taken);
    only(&s)
}

fn fed(name: &str) -> SessionFacts {
    fed_bytes(&read_fixture(name))
}

fn text(lines: &[&str]) -> Vec<u8> {
    lines
        .iter()
        .map(|line| format!("{line}\n"))
        .collect::<String>()
        .into_bytes()
}

fn fed_lines(lines: &[&str]) -> SessionFacts {
    fed_bytes(&text(lines))
}

/// Feeds `bytes` twice, as the watch thread does after a `RetryOnce`, and checks that it asked for the retry.
fn fed_unreadable(bytes: &[u8]) -> SessionFacts {
    let mut s = source();
    assert_eq!(
        s.feed(&chat(ID), Feed::Whole(bytes.to_vec())),
        Fed::RetryOnce
    );
    assert_eq!(s.feed(&chat(ID), Feed::Whole(bytes.to_vec())), Fed::Taken);
    only(&s)
}

fn facts(activity: Activity, since: Option<i64>, format: FormatHealth) -> SessionFacts {
    SessionFacts {
        key: SessionKey {
            source: SourceId::VsCode,
            id: ID.to_owned(),
        },
        liveness: Liveness::ImageSince {
            images: &["Code.exe"],
            since_wall_ms: since,
        },
        activity,
        timers: Vec::new(),
        open_tasks: 0,
        label: "Copilot in VS Code",
        format,
    }
}

fn since(facts: &SessionFacts) -> Option<i64> {
    match facts.liveness {
        Liveness::ImageSince { since_wall_ms, .. } => since_wall_ms,
        ref other => panic!("{other:?}"),
    }
}

fn initial(requests: &str) -> String {
    format!(
        r#"{{"kind":0,"v":{{"version":3,"sessionId":"{ID}","requests":[{requests}],"pendingRequests":[]}}}}"#
    )
}

fn request(sent: i64, state: &str) -> String {
    format!(
        r#"{{"requestId":"request_{sent}","timestamp":{sent},"message":{{"text":"fix the build","parts":[]}},"response":[],"modelState":{state}}}"#
    )
}

const PENDING: &str = r#"{"value":0}"#;
const NEEDS_INPUT: &str = r#"{"value":4}"#;
const COMPLETE: &str = r#"{"value":1,"completedAt":1793959290000}"#;

// What the source asks for, and which files it takes.

/// Each want's root, levels and fixed folder; every want must be `Recent` with the session name filter.
fn shape(wants: &[WantedFile]) -> Vec<(PathBuf, u8, Option<&'static str>)> {
    wants
        .iter()
        .map(|want| match want {
            WantedFile::Recent {
                root,
                depth,
                sub,
                accept,
            } => {
                assert!(accept(&format!("{ID}.jsonl")));
                assert!(!accept("state.vscdb"));
                (root.clone(), *depth, *sub)
            }
            other => panic!("{other:?}"),
        })
        .collect()
}

#[test]
fn it_wants_both_chat_folders_of_each_build_whole_and_recent() {
    let mut s = both_builds();
    let got = shape(&s.wanted());
    let (user, insiders) = (user(), insiders_user());
    assert_eq!(
        got,
        [
            (user.join("workspaceStorage"), 1, Some("chatSessions")),
            (
                user.join("globalStorage").join("emptyWindowChatSessions"),
                0,
                None
            ),
            (insiders.join("workspaceStorage"), 1, Some("chatSessions")),
            (
                insiders
                    .join("globalStorage")
                    .join("emptyWindowChatSessions"),
                0,
                None
            ),
        ]
    );
    s.feed(&chat(ID), Feed::Whole(read_fixture("working.jsonl")));
    assert_eq!(shape(&s.wanted()), got, "the wants never change");
}

#[test]
fn only_session_logs_with_a_plain_id_pass_the_name_filter() {
    let long = "a".repeat(128);
    for name in [
        format!("{ID}.jsonl"),
        "ABC_def-1.JSONL".to_owned(),
        "a.Jsonl".to_owned(),
        format!("{long}.jsonl"),
    ] {
        assert!(is_session_file_name(&name), "{name}");
    }
    let too_long = format!("{long}a.jsonl");
    for name in [
        &format!("{ID}.json"),
        ".jsonl",
        "jsonl",
        "x.jsonl.bak",
        "a b.jsonl",
        "a.b.jsonl",
        "x.jsonl ",
        "\u{65e5}\u{672c}.jsonl",
        "x\u{e9}.json",
        &too_long,
        "state.vscdb",
        "state.vscdb-journal",
        "agent-host.db",
        "session.db",
        "index.json",
    ] {
        assert!(!is_session_file_name(name), "{name}");
    }
}

#[test]
fn installs_find_the_data_folder_the_way_vs_code_does() {
    let users =
        |appdata: Option<&str>, vscode: Option<&str>| -> Vec<(PathBuf, &'static [&'static str])> {
            installs(
                Path::new(PROFILE),
                appdata.map(OsStr::new),
                vscode.map(OsStr::new),
            )
            .into_iter()
            .map(|install| (install.user_dir, install.product.images))
            .collect()
        };
    let stable: &[&str] = &["Code.exe"];
    let insiders: &[&str] = &["Code - Insiders.exe"];
    let under = |base: &str, folder: &str| Path::new(base).join(folder).join("User");
    assert_eq!(
        users(Some(r"X:\Roaming"), None),
        [
            (under(r"X:\Roaming", "Code"), stable),
            (under(r"X:\Roaming", "Code - Insiders"), insiders),
        ]
    );
    // `VSCODE_APPDATA` wins over `APPDATA`, for both builds.
    assert_eq!(
        users(Some(r"X:\Roaming"), Some(r"D:\vsdata")),
        [
            (under(r"D:\vsdata", "Code"), stable),
            (under(r"D:\vsdata", "Code - Insiders"), insiders),
        ]
    );
    // An empty value is unset; with no `APPDATA` VS Code uses the profile's `AppData\Roaming`.
    let roaming = Path::new(PROFILE).join("AppData").join("Roaming");
    let fallback = [
        (roaming.join("Code").join("User"), stable),
        (roaming.join("Code - Insiders").join("User"), insiders),
    ];
    assert_eq!(users(None, None), fallback);
    assert_eq!(users(Some(""), Some("")), fallback);
    assert_eq!(STABLE.folder, "Code");
    assert_eq!(INSIDERS.folder, "Code - Insiders");
}

#[test]
fn files_outside_the_chat_folders_are_never_taken() {
    let mut s = source();
    let working = read_fixture("working.jsonl");
    let user = user();
    let ws = user.join("workspaceStorage").join(WORKSPACE);
    let name = format!("{ID}.jsonl");
    for path in [
        ws.join(&name),
        user.join("workspaceStorage").join(&name),
        user.join("workspaceStorage")
            .join(CHAT_SESSIONS)
            .join(&name),
        ws.join("chatEditingSessions").join(&name),
        ws.join("x").join(CHAT_SESSIONS).join(&name),
        ws.join(CHAT_SESSIONS).join(format!("{ID}.json")),
        ws.join(CHAT_SESSIONS).join("state.vscdb"),
        user.join("globalStorage").join(&name),
        user.join("globalStorage")
            .join("transferredChatSessions")
            .join(&name),
        user.join("globalStorage")
            .join("emptyWindowChatSessions")
            .join("x")
            .join(&name),
        PathBuf::from(r"X:\elsewhere")
            .join(WORKSPACE)
            .join(CHAT_SESSIONS)
            .join(&name),
        insiders_user()
            .join("workspaceStorage")
            .join(WORKSPACE)
            .join(CHAT_SESSIONS)
            .join(&name),
    ] {
        assert_eq!(
            s.feed(&FileId(path), Feed::Whole(working.clone())),
            Fed::Taken
        );
    }
    // A session log comes whole; a listing or a tail of one says nothing.
    s.feed(&chat(ID), Feed::Present);
    s.feed(&chat(ID), Feed::Appended(working));
    assert_eq!(s.sessions(), []);
}

// Working, waiting and idle (rule 4).

#[test]
fn a_sent_request_works_from_its_send_time() {
    assert_eq!(
        fed("working.jsonl"),
        facts(Activity::Working, Some(T_R1), FormatHealth::Ok)
    );
}

#[test]
fn a_tool_approval_waits_with_no_time_and_is_not_remote() {
    assert_eq!(
        fed("waiting.jsonl"),
        facts(WAITING, Some(T_R1), FormatHealth::Ok)
    );
}

#[test]
fn an_approved_request_works_again() {
    assert_eq!(
        fed("working-again.jsonl"),
        facts(Activity::Working, Some(T_R1), FormatHealth::Ok)
    );
}

#[test]
fn a_done_request_is_idle_and_writes_elsewhere_change_nothing() {
    assert_eq!(
        fed("idle.jsonl"),
        facts(Activity::Idle, Some(T_R1), FormatHealth::Ok)
    );
}

#[test]
fn any_open_request_decides_and_working_comes_before_waiting() {
    let working_and_waiting =
        initial(&[request(T_R1, NEEDS_INPUT), request(T_R3, PENDING)].join(","));
    assert_eq!(
        fed_lines(&[&working_and_waiting]),
        facts(Activity::Working, Some(T_R3), FormatHealth::Ok)
    );
    // An older request left open still counts, and the newest open one sets the time.
    let two_open = initial(&[request(T_R3, PENDING), request(T_R1, COMPLETE)].join(","));
    assert_eq!(
        fed_lines(&[&two_open]),
        facts(Activity::Working, Some(T_R3), FormatHealth::Ok)
    );
    let cancelled_and_failed = initial(
        &[
            request(T_R1, r#"{"value":2,"completedAt":1}"#),
            request(T_R3, r#"{"value":3,"completedAt":1}"#),
        ]
        .join(","),
    );
    assert_eq!(
        fed_lines(&[&cancelled_and_failed]),
        facts(Activity::Idle, Some(T_R3), FormatHealth::Ok)
    );
}

#[test]
fn a_chat_with_no_request_or_none_with_a_state_is_idle() {
    assert_eq!(
        fed_lines(&[&initial("")]),
        facts(Activity::Idle, None, FormatHealth::Ok)
    );
    let old =
        format!(r#"{{"requestId":"r","timestamp":{T_R1},"message":{{"text":"fix the build"}}}}"#);
    assert_eq!(
        fed_lines(&[&initial(&old)]),
        facts(Activity::Idle, Some(T_R1), FormatHealth::Ok)
    );
    let null_state = request(T_R1, "null");
    assert_eq!(fed_lines(&[&initial(&null_state)]).activity, Activity::Idle);
}

// Compaction and several requests.

#[test]
fn a_compacted_file_is_read_from_its_one_line_also_when_rewritten_in_place() {
    let compacted = facts(Activity::Working, Some(T_R3), FormatHealth::Ok);
    assert_eq!(fed("compacted.jsonl"), compacted);
    let mut s = source();
    s.feed(&chat(ID), Feed::Whole(read_fixture("idle.jsonl")));
    assert_eq!(only(&s).activity, Activity::Idle);
    // Shorter than before: a tail would have broken here.
    assert_eq!(
        s.feed(&chat(ID), Feed::Whole(read_fixture("compacted.jsonl"))),
        Fed::Taken
    );
    assert_eq!(only(&s), compacted);
}

#[test]
fn several_requests_work_by_the_newest_open_one() {
    assert_eq!(
        fed("several-requests.jsonl"),
        facts(Activity::Working, Some(T_R3), FormatHealth::Ok)
    );
}

// A partial last line (rule 3).

#[test]
fn a_partial_last_line_is_read_again_once_then_unreadable() {
    let partial = read_fixture("partial-last-line.jsonl");
    let unreadable = FormatHealth::Unreadable(UNREADABLE);
    let mut s = source();
    assert_eq!(
        s.feed(&chat(ID), Feed::Whole(partial.clone())),
        Fed::RetryOnce
    );
    assert_eq!(s.sessions(), [], "nothing to say before the retry");
    assert_eq!(s.feed(&chat(ID), Feed::Whole(partial.clone())), Fed::Taken);
    assert_eq!(only(&s), facts(Activity::Idle, None, unreadable.clone()));
    // The write ends: the next read is whole.
    s.feed(&chat(ID), Feed::Whole(read_fixture("waiting.jsonl")));
    assert_eq!(only(&s), facts(WAITING, Some(T_R1), FormatHealth::Ok));
    // A cut write in a file read before: the last read stands until the retry.
    assert_eq!(
        s.feed(&chat(ID), Feed::Whole(partial.clone())),
        Fed::RetryOnce
    );
    assert_eq!(only(&s), facts(WAITING, Some(T_R1), FormatHealth::Ok));
    assert_eq!(s.feed(&chat(ID), Feed::Whole(partial.clone())), Fed::Taken);
    assert_eq!(only(&s), facts(Activity::Idle, Some(T_R1), unreadable));
    // The retry finds the write done.
    assert_eq!(
        s.feed(&chat(ID), Feed::Whole(partial.clone())),
        Fed::RetryOnce
    );
    assert_eq!(
        s.feed(&chat(ID), Feed::Whole(read_fixture("working.jsonl"))),
        Fed::Taken
    );
    assert_eq!(
        only(&s),
        facts(Activity::Working, Some(T_R1), FormatHealth::Ok)
    );
    // The file goes while its retry is due; when it comes back it gets its own retry.
    assert_eq!(
        s.feed(&chat(ID), Feed::Whole(partial.clone())),
        Fed::RetryOnce
    );
    s.feed(&chat(ID), Feed::Gone);
    assert_eq!(s.feed(&chat(ID), Feed::Whole(partial)), Fed::RetryOnce);
}

#[test]
fn a_file_with_no_initial_line_is_unreadable() {
    let unreadable = FormatHealth::Unreadable(UNREADABLE);
    let set = r#"{"kind":1,"k":["requests",0,"modelState"],"v":{"value":0}}"#;
    for bytes in [
        Vec::new(),
        b" \r\n\n".to_vec(),
        text(&[set]),
        text(&[r#"{"kind":9,"k":["requests"]}"#]),
    ] {
        assert_eq!(
            fed_unreadable(&bytes),
            facts(Activity::Idle, None, unreadable.clone()),
            "{bytes:?}"
        );
    }
    // A line before the Initial line has nothing to apply to; the Initial line then sets the state.
    assert_eq!(
        fed_lines(&[set, &initial("")]),
        facts(Activity::Idle, None, FormatHealth::Ok)
    );
}

#[test]
fn a_line_that_does_not_read_on_a_path_read_makes_the_file_unreadable() {
    let start = initial(&request(T_R1, PENDING));
    for line in [
        "not json",
        "[1,2]",
        r#"{"kind":1,"k":["requests",0,"modelState"],"v":"#,
        r#"{"kind":2,"k":["requests"],"v":["fix the build"]}"#,
        r#"{"kind":2,"k":["requests"],"v":{"value":0}}"#,
        r#"{"kind":2,"k":["requests"],"v":[],"i":-1}"#,
        r#"{"kind":2,"k":["requests"],"v":[],"i":"1"}"#,
        r#"{"kind":1,"k":["requests"],"v":7}"#,
        r#"{"kind":1,"k":["requests",0],"v":"fix the build"}"#,
        r#"{"kind":0,"v":{"requests":"fix the build"}}"#,
        r#"{"kind":0,"v":7}"#,
    ] {
        assert_eq!(
            fed_unreadable(&text(&[&start, line])),
            facts(Activity::Idle, None, FormatHealth::Unreadable(UNREADABLE)),
            "{line}"
        );
    }
}

// Unknown values (rule 5).

#[test]
fn an_unknown_kind_on_a_path_read_is_an_unknown_value() {
    assert_eq!(
        fed("unknown-kind.jsonl"),
        facts(
            Activity::Idle,
            Some(T_R1),
            FormatHealth::UnknownValue(KIND, "7".to_owned())
        )
    );
    let start = initial(&request(T_R1, COMPLETE));
    for (line, value) in [
        (r#"{"kind":4}"#, "4"),
        (r#"{"kind":4,"k":[]}"#, "4"),
        (r#"{"kind":4,"k":"requests"}"#, "4"),
        (r#"{"kind":4,"k":["requests"]}"#, "4"),
        (r#"{"kind":4,"k":["requests",0]}"#, "4"),
        (r#"{"kind":4,"k":["requests","x"]}"#, "4"),
        (r#"{"kind":4,"k":["requests",0,"modelState"]}"#, "4"),
        (r#"{"kind":4,"k":["requests",0,"modelState","value"]}"#, "4"),
        (r#"{"kind":4,"k":["requests",0,"timestamp"]}"#, "4"),
        (r#"{"kind":1.5,"k":["requests"]}"#, "1.5"),
        (r#"{"kind":-1,"k":["requests"]}"#, "-1"),
        (r#"{"kind":"set","k":["requests"]}"#, "text"),
        (r#"{"kind":null,"k":["requests"]}"#, "missing"),
        (r#"{"k":["requests"]}"#, "missing"),
    ] {
        assert_eq!(
            fed_lines(&[&start, line]).format,
            FormatHealth::UnknownValue(KIND, value.to_owned()),
            "{line}"
        );
    }
    // The first problem is the one named.
    assert_eq!(
        fed_lines(&[&start, r#"{"kind":4}"#, r#"{"kind":5}"#]).format,
        FormatHealth::UnknownValue(KIND, "4".to_owned())
    );
}

#[test]
fn an_unknown_kind_keeps_the_known_activity_and_any_request_may_hold() {
    // Two requests, the older one open: after an unknown kind, any request's time may be the one that counts.
    let start = initial(&[request(T_R3, PENDING), request(T_R1, COMPLETE)].join(","));
    assert_eq!(
        fed_lines(&[&start, r#"{"kind":4,"k":["requests",1]}"#]),
        facts(
            Activity::Working,
            Some(T_R3),
            FormatHealth::UnknownValue(KIND, "4".to_owned())
        )
    );
}

#[test]
fn a_line_of_any_kind_on_a_path_not_read_is_skipped() {
    let start = initial(&request(T_R1, PENDING));
    let working = facts(Activity::Working, Some(T_R1), FormatHealth::Ok);
    for line in [
        r#"{"kind":5,"k":["requests",0,"response",0,"value"],"v":"fix the build"}"#,
        r#"{"kind":9,"k":["pendingRequests"],"v":[]}"#,
        r#"{"kind":9,"k":["inputState","inputText"],"v":"fix the build"}"#,
        r#"{"kind":1,"k":["inputState","inputText"],"v":"fix the build"}"#,
        r#"{"kind":2,"k":["requests",0,"response"],"v":[{"value":"fix the build"}]}"#,
        r#"{"kind":3,"k":["requests",0,"response"]}"#,
        r#"{"kind":1,"k":["requests",0,"message"],"v":{"text":"fix the build"}}"#,
        r#"{"kind":1,"k":["requests",0,"modelState","completedAt"],"v":5}"#,
        r#"{"kind":1,"k":["pendingRequests",0,"request","modelState"],"v":{"value":9}}"#,
        r#"{"kind":1,"k":"requests","v":[]}"#,
        r#"{"kind":1,"k":[],"v":{"requests":[]}}"#,
        r#"{"kind":1,"k":["requests","0","modelState"],"v":{"value":1}}"#,
        r#"{"kind":2,"k":["requests",0,"modelState"],"v":[{"value":1}]}"#,
        r#"{"kind":2,"k":["requests",0],"v":[{"value":1}]}"#,
    ] {
        assert_eq!(fed_lines(&[&start, line]), working, "{line}");
    }
}

#[test]
fn an_unknown_state_on_the_newest_request_is_an_unknown_value() {
    assert_eq!(
        fed("unknown-state.jsonl"),
        facts(
            Activity::Idle,
            Some(T_R1),
            FormatHealth::UnknownValue(MODEL_STATE, "9".to_owned())
        )
    );
    for (state, value) in [
        (r#"{"value":5}"#, "5"),
        (r#"{"value":-1}"#, "-1"),
        (r#"{"value":1.5}"#, "1.5"),
        (r#"{"value":"fix the build"}"#, "text"),
        (r#"{"value":null}"#, "null"),
        (r#"{"value":[0]}"#, "list"),
        (r#"{"completedAt":1}"#, "missing"),
        (r#""pending""#, "text"),
        ("0", "0"),
        ("[0]", "list"),
        ("true", "bool"),
    ] {
        assert_eq!(
            fed_lines(&[&initial(&request(T_R1, state))]),
            facts(
                Activity::Idle,
                Some(T_R1),
                FormatHealth::UnknownValue(MODEL_STATE, value.to_owned())
            ),
            "{state}"
        );
    }
}

#[test]
fn an_unknown_state_on_an_older_request_is_history() {
    let start = initial(&[request(T_R1, r#"{"value":9}"#), request(T_R3, COMPLETE)].join(","));
    assert_eq!(
        fed_lines(&[&start]),
        facts(Activity::Idle, Some(T_R3), FormatHealth::Ok)
    );
    // With a newer request open, the newest unknown one also sets the time.
    let open = initial(&[request(T_R1, PENDING), request(T_R3, r#"{"value":9}"#)].join(","));
    assert_eq!(
        fed_lines(&[&open]),
        facts(
            Activity::Working,
            Some(T_R3),
            FormatHealth::UnknownValue(MODEL_STATE, "9".to_owned())
        )
    );
}

// Liveness (rule 7).

#[test]
fn a_reopened_request_counts_while_any_vs_code_runs() {
    assert_eq!(
        fed("reopened.jsonl"),
        facts(Activity::Working, None, FormatHealth::Ok)
    );
    // Waiting after a reopen too; a request replaced whole starts fresh.
    let start = initial(&request(T_R1, COMPLETE));
    let waits = r#"{"kind":1,"k":["requests",0,"modelState"],"v":{"value":4}}"#;
    assert_eq!(
        fed_lines(&[&start, waits]),
        facts(WAITING, None, FormatHealth::Ok)
    );
    let replaced = format!(
        r#"{{"kind":2,"k":["requests"],"v":[{}],"i":0}}"#,
        request(T_R3, PENDING)
    );
    assert_eq!(
        fed_lines(&[&start, waits, &replaced]),
        facts(Activity::Working, Some(T_R3), FormatHealth::Ok)
    );
}

#[test]
fn a_send_time_that_is_not_ms_since_2020_counts_any_start() {
    for sent in [
        "1793959205",
        r#""2026-11-06T10:00:05Z""#,
        "null",
        "-5",
        "1577836799999",
        "1e300",
        "[1793959205000]",
    ] {
        let line = initial(&format!(
            r#"{{"requestId":"r","timestamp":{sent},"modelState":{{"value":0}}}}"#
        ));
        assert_eq!(
            fed_lines(&[&line]),
            facts(Activity::Working, None, FormatHealth::Ok),
            "{sent}"
        );
    }
    let no_time = initial(r#"{"requestId":"r","modelState":{"value":0}}"#);
    assert_eq!(since(&fed_lines(&[&no_time])), None);
    for (sent, ms) in [
        ("1577836800000", 1_577_836_800_000),
        ("1793959205000.7", T_R1),
    ] {
        let line = initial(&format!(
            r#"{{"requestId":"r","timestamp":{sent},"modelState":{{"value":0}}}}"#
        ));
        assert_eq!(since(&fed_lines(&[&line])), Some(ms), "{sent}");
    }
    // One open request with no time: any start counts for the session.
    let mixed = initial(
        &[
            request(T_R1, PENDING),
            r#"{"requestId":"r","modelState":{"value":0}}"#.to_owned(),
        ]
        .join(","),
    );
    assert_eq!(since(&fed_lines(&[&mixed])), None);
}

#[test]
fn an_insiders_chat_lives_by_the_insiders_image() {
    let mut s = both_builds();
    s.feed(
        &in_workspace(&insiders_user(), ID),
        Feed::Whole(read_fixture("working.jsonl")),
    );
    assert_eq!(
        only(&s).liveness,
        Liveness::ImageSince {
            images: &["Code - Insiders.exe"],
            since_wall_ms: Some(T_R1),
        }
    );
    s.feed(
        &in_empty_window(&user(), OTHER),
        Feed::Whole(read_fixture("working.jsonl")),
    );
    let images: Vec<Liveness> = s.sessions().into_iter().map(|f| f.liveness).collect();
    assert_eq!(
        images,
        [
            Liveness::ImageSince {
                images: &["Code - Insiders.exe"],
                since_wall_ms: Some(T_R1),
            },
            Liveness::ImageSince {
                images: &["Code.exe"],
                since_wall_ms: Some(T_R1),
            },
        ]
    );
}

// Files that come and go.

#[test]
fn a_deleted_file_ends_its_session() {
    let mut s = source();
    s.feed(&chat(ID), Feed::Whole(read_fixture("working.jsonl")));
    s.feed(&chat(OTHER), Feed::Whole(read_fixture("idle.jsonl")));
    assert_eq!(s.sessions().len(), 2);
    assert_eq!(s.feed(&chat(ID), Feed::Gone), Fed::Taken);
    let left = only(&s);
    assert_eq!(left.key.id, OTHER);
    // Gone again, or for a file never fed: nothing changes.
    s.feed(&chat(ID), Feed::Gone);
    s.feed(&chat("0b1c2d3e-0000-4000-8000-000000000003"), Feed::Gone);
    assert_eq!(s.sessions(), [left]);
}

#[test]
fn an_empty_window_chat_is_read_from_its_own_folder() {
    let mut s = source();
    let file = in_empty_window(&user(), ID);
    assert_eq!(
        s.feed(&file, Feed::Whole(read_fixture("empty-window.jsonl"))),
        Fed::Taken
    );
    assert_eq!(only(&s), facts(Activity::Idle, None, FormatHealth::Ok));
    s.feed(&file, Feed::Whole(read_fixture("working.jsonl")));
    assert_eq!(
        only(&s),
        facts(Activity::Working, Some(T_R1), FormatHealth::Ok)
    );
}

#[test]
fn one_chat_in_two_folders_is_one_session_and_the_file_that_says_most_stands() {
    let mut s = source();
    s.feed(
        &in_empty_window(&user(), ID),
        Feed::Whole(read_fixture("idle.jsonl")),
    );
    s.feed(&chat(ID), Feed::Whole(read_fixture("working.jsonl")));
    assert_eq!(
        s.sessions(),
        [facts(Activity::Working, Some(T_R1), FormatHealth::Ok)]
    );
    s.feed(&chat(ID), Feed::Whole(read_fixture("waiting.jsonl")));
    assert_eq!(s.sessions(), [facts(WAITING, Some(T_R1), FormatHealth::Ok)]);
    // An unreadable copy never hides a readable one.
    let partial = read_fixture("partial-last-line.jsonl");
    s.feed(&chat(ID), Feed::Whole(partial.clone()));
    s.feed(&chat(ID), Feed::Whole(partial));
    assert_eq!(
        s.sessions(),
        [facts(Activity::Idle, Some(T_R1), FormatHealth::Ok)]
    );
    s.feed(
        &in_empty_window(&user(), ID),
        Feed::Whole(read_fixture("unknown-state.jsonl")),
    );
    assert_eq!(
        s.sessions(),
        [facts(
            Activity::Idle,
            Some(T_R1),
            FormatHealth::UnknownValue(MODEL_STATE, "9".to_owned())
        )]
    );
}

// VS Code's write rules (`objectMutationLog.ts`).

#[test]
fn writes_follow_vs_codes_own_rules() {
    let done = initial(&request(T_R1, COMPLETE));
    let set = |k: &str, v: &str| format!(r#"{{"kind":1,"k":{k},"v":{v}}}"#);
    let delete = |k: &str| format!(r#"{{"kind":3,"k":{k}}}"#);
    let pending_r3 = request(T_R3, PENDING);
    let working_r3 = facts(Activity::Working, Some(T_R3), FormatHealth::Ok);
    let idle_none = facts(Activity::Idle, None, FormatHealth::Ok);

    // The whole list set, set to undefined (no `v`), deleted, then pushed onto again.
    assert_eq!(
        fed_lines(&[&done, &set(r#"["requests"]"#, &format!("[{pending_r3}]"))]),
        working_r3
    );
    assert_eq!(
        fed_lines(&[&done, r#"{"kind":1,"k":["requests"]}"#]),
        idle_none
    );
    let push_r3 = format!(r#"{{"kind":2,"k":["requests"],"v":[{pending_r3}]}}"#);
    assert_eq!(
        fed_lines(&[&done, &delete(r#"["requests"]"#), &push_r3]),
        working_r3
    );
    // A Set past the end leaves holes; a hole or a null item is no request.
    assert_eq!(
        fed_lines(&[&done, &set(r#"["requests",3]"#, &pending_r3)]),
        working_r3
    );
    assert_eq!(
        fed_lines(&[&initial(&format!("null,{pending_r3}"))]),
        working_r3
    );
    let open = initial(&request(T_R1, PENDING));
    for gone in [
        delete(r#"["requests",0]"#),
        set(r#"["requests",0]"#, "null"),
        r#"{"kind":2,"k":["requests"],"i":0}"#.to_owned(),
    ] {
        assert_eq!(fed_lines(&[&open, &gone]), idle_none, "{gone}");
    }
    // A Push cuts at `i`, also past the end.
    let push_at = |i: u64| format!(r#"{{"kind":2,"k":["requests"],"v":[{pending_r3}],"i":{i}}}"#);
    assert_eq!(fed_lines(&[&done, &push_at(0)]), working_r3);
    assert_eq!(fed_lines(&[&initial(""), &push_at(2)]), working_r3);
    // `modelState` deleted is old data; its `value` alone is set too; a write into a state that is not there is
    // skipped, as VS Code itself could not apply it.
    assert_eq!(
        fed_lines(&[&open, &delete(r#"["requests",0,"modelState"]"#)]),
        facts(Activity::Idle, Some(T_R1), FormatHealth::Ok)
    );
    assert_eq!(
        fed_lines(&[&open, &set(r#"["requests",0,"modelState","value"]"#, "4")]),
        facts(WAITING, Some(T_R1), FormatHealth::Ok)
    );
    assert_eq!(
        fed_lines(&[&open, &delete(r#"["requests",0,"modelState","value"]"#)]),
        facts(
            Activity::Idle,
            Some(T_R1),
            FormatHealth::UnknownValue(MODEL_STATE, "missing".to_owned())
        )
    );
    let unset = initial(&format!(r#"{{"requestId":"r","timestamp":{T_R1}}}"#));
    assert_eq!(
        fed_lines(&[&unset, &set(r#"["requests",0,"modelState","value"]"#, "0")]),
        facts(Activity::Idle, Some(T_R1), FormatHealth::Ok)
    );
    // The send time set and deleted.
    assert_eq!(
        since(&fed_lines(&[
            &open,
            &set(r#"["requests",0,"timestamp"]"#, &T_R3.to_string())
        ])),
        Some(T_R3)
    );
    assert_eq!(
        since(&fed_lines(&[
            &open,
            &delete(r#"["requests",0,"timestamp"]"#)
        ])),
        None
    );
    // Writes into a request that is not there, or past the index bound, are skipped.
    for line in [
        set(r#"["requests",5,"modelState"]"#, PENDING),
        set(r#"["requests",65536]"#, &pending_r3),
        format!(r#"{{"kind":2,"k":["requests"],"v":[{pending_r3}],"i":65536}}"#),
        set(r#"["requests",1e3]"#, &pending_r3),
    ] {
        assert_eq!(
            fed_lines(&[&done, &line]),
            facts(Activity::Idle, Some(T_R1), FormatHealth::Ok),
            "{line}"
        );
    }
    let no_list = initial("").replace(r#""requests":[],"#, "");
    assert_eq!(
        fed_lines(&[&no_list, &set(r#"["requests",0]"#, &pending_r3)]),
        idle_none
    );
    // A later Initial line replaces the state, as VS Code's reader does.
    assert_eq!(fed_lines(&[&done, &initial(&pending_r3)]), working_r3);
    // Line ends with `\r` and blank lines read.
    let crlf = format!(
        "{open}\r\n\r\n{}\r\n",
        set(r#"["requests",0,"modelState"]"#, NEEDS_INPUT)
    );
    assert_eq!(
        fed_bytes(crlf.as_bytes()),
        facts(WAITING, Some(T_R1), FormatHealth::Ok)
    );
}

// Live chat logs (`p4-live-vscode`), redacted copies of real VS Code 1.145.1 files (`fixtures/vscode/live/README.md`).

fn live_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("vscode")
        .join("live")
}

/// A live chat log, the lines (counted from 1) after which the activity changes with the activity from there on, and
/// the send time its liveness names at the end.
type LiveCase = (&'static str, Vec<(usize, Activity)>, Option<i64>);

fn live_expected() -> Vec<LiveCase> {
    vec![
        // Sent 16:02:46.076 local (UTC+4); VS Code's first save, 44 s later, already says NeedsInput.
        (
            "terminal-call-waiting.jsonl",
            vec![(1, WAITING)],
            Some(1_793_966_566_076),
        ),
        // Sent 16:07:53.027, done 16:07:55.941; the first save came after the end, so it never says Pending.
        (
            "ask-turn-done.jsonl",
            vec![(1, Activity::Idle)],
            Some(1_793_966_873_027),
        ),
    ]
}

#[test]
fn every_live_fixture_is_listed_once() {
    let mut on_disk: Vec<String> = fs::read_dir(live_dir())
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .filter(|name| name.ends_with(".jsonl"))
        .collect();
    on_disk.sort();
    let mut listed: Vec<String> = live_expected().iter().map(|e| e.0.to_owned()).collect();
    listed.sort();
    assert_eq!(on_disk, listed);
}

/// Each part that ends at a line end is a file VS Code could have written at one save: each reads with no format
/// problem, the activity moves only where the table says, and the liveness names the chat's send time.
#[test]
fn live_chat_logs_read_at_every_save_with_no_format_problem() {
    for (name, changes, since) in live_expected() {
        let path = live_dir().join(name);
        let bytes = fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let mut now = None;
        let mut end = 0;
        for (i, line) in bytes.split_inclusive(|&b| b == b'\n').enumerate() {
            end += line.len();
            if let Some((_, next)) = changes.iter().find(|(at, _)| *at == i + 1) {
                now = Some(*next);
            }
            let got = fed_bytes(&bytes[..end]);
            assert_eq!(
                (Some(got.activity), got.format),
                (now, FormatHealth::Ok),
                "{name} line {}",
                i + 1
            );
        }
        assert!(bytes.ends_with(b"\n"), "{name}: VS Code ends every entry");
        assert_eq!(
            fed_bytes(&bytes).liveness,
            Liveness::ImageSince {
                images: &["Code.exe"],
                since_wall_ms: since,
            },
            "{name}"
        );
    }
}

// Privacy.

#[test]
fn no_prompt_text_reaches_facts_or_debug() {
    let mut s = source();
    for (i, name) in FIXTURES.iter().enumerate() {
        let file = chat(&format!("0b1c2d3e-0000-4000-9000-{i:012}"));
        let bytes = read_fixture(name);
        if s.feed(&file, Feed::Whole(bytes.clone())) == Fed::RetryOnce {
            s.feed(&file, Feed::Whole(bytes));
        }
    }
    let text_state = initial(&request(T_R1, r#"{"value":"fix the build"}"#));
    s.feed(&chat(OTHER), Feed::Whole(text(&[&text_state])));
    let sessions = s.sessions();
    assert_eq!(sessions.len(), FIXTURES.len() + 1);
    let shown = format!("{s:?} {sessions:?}");
    for secret in [
        "fix the build",
        "check the deploy",
        "map the code",
        "cargo build",
        "The build passes",
        "never sent",
        "and now the tests",
        "which database",
        "Reading the crates",
        "GitHub Copilot",
        "editsAgent",
        "copilot/auto",
        "summary",
        "request_1",
    ] {
        assert!(!shown.contains(secret), "{secret} in {shown}");
    }
    // What the core logs comes from `format` only: a field name and a number or a type word.
    for facts in &sessions {
        match &facts.format {
            FormatHealth::Ok => {}
            FormatHealth::Unreadable(what) => assert_eq!(*what, UNREADABLE),
            FormatHealth::UnknownValue(field, value) => {
                assert!([KIND, MODEL_STATE].contains(field), "{field}");
                assert!(
                    value.parse::<f64>().is_ok()
                        || ["missing", "null", "bool", "text", "list", "object"]
                            .contains(&value.as_str()),
                    "{value}"
                );
            }
        }
        assert_eq!(facts.label, LABEL);
    }
}

// Any input.

/// One line of a log with paths, indexes and values near every rule's edge.
fn entry() -> impl Strategy<Value = String> {
    let index = prop_oneof![0u64..4, Just(65_535u64), Just(65_536u64), Just(u64::MAX)];
    let field = prop_oneof![
        Just(r#","modelState""#),
        Just(r#","timestamp""#),
        Just(r#","response""#),
        Just(r#","modelState","value""#),
        Just(""),
    ];
    let path = prop_oneof![
        Just("[]".to_owned()),
        Just(r#"["requests"]"#.to_owned()),
        Just(r#"["inputState"]"#.to_owned()),
        (index, field).prop_map(|(n, f)| format!(r#"["requests",{n}{f}]"#)),
    ];
    let value = prop_oneof![
        Just("null"),
        Just(r#"{"value":0}"#),
        Just(r#"{"value":4}"#),
        Just(r#"{"value":2,"completedAt":1}"#),
        Just("4"),
        Just("1793959205000"),
        Just(r#"[{"timestamp":1793959205000,"modelState":{"value":0}},null]"#),
        Just(r#"{"timestamp":1793959800000,"modelState":{"value":4}}"#),
        Just("[]"),
        Just(r#""text""#),
    ];
    let cut = prop::option::of(prop_oneof![0u64..4, Just(65_535u64), Just(65_536u64)]);
    (0u8..6, path, value, cut).prop_map(|(kind, k, v, i)| {
        if kind == 0 {
            initial(&request(T_R1, PENDING))
        } else {
            let i = i.map(|i| format!(r#","i":{i}"#)).unwrap_or_default();
            format!(r#"{{"kind":{kind},"k":{k},"v":{v}{i}}}"#)
        }
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn a_fixture_cut_anywhere_reads_up_to_a_line_end_or_asks_for_a_retry(
        pick in 0usize..FIXTURES.len(),
        cut in 0usize..1600,
    ) {
        let bytes = read_fixture(FIXTURES[pick]);
        let cut = cut.min(bytes.len());
        let head = &bytes[..cut];
        let mut s = source();
        let fed = s.feed(&chat(ID), Feed::Whole(head.to_vec()));
        let at_line_end = cut > 0 && (head.ends_with(b"\n") || bytes.get(cut) == Some(&b'\n'));
        prop_assert_eq!(fed, if at_line_end { Fed::Taken } else { Fed::RetryOnce });
    }

    #[test]
    fn any_bytes_never_panic(bytes in prop::collection::vec(any::<u8>(), 0..2048)) {
        let mut s = source();
        s.feed(&chat(ID), Feed::Whole(bytes.clone()));
        s.feed(&chat(ID), Feed::Whole(bytes));
        prop_assert!(s.sessions().len() <= 1);
    }

    #[test]
    fn any_lines_never_panic_and_keep_one_session(lines in prop::collection::vec(entry(), 0..24)) {
        let mut all = vec![initial("")];
        all.extend(lines);
        let refs: Vec<&str> = all.iter().map(String::as_str).collect();
        let bytes = text(&refs);
        let mut s = source();
        // A value of another shape on a path read makes the file unreadable, after its one retry.
        if s.feed(&chat(ID), Feed::Whole(bytes.clone())) == Fed::RetryOnce {
            prop_assert_eq!(s.feed(&chat(ID), Feed::Whole(bytes)), Fed::Taken);
        }
        prop_assert_eq!(only(&s).key.id, ID);
    }
}
