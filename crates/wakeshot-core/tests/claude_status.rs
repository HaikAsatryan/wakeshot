//! `ClaudeSource` on status files, fed the way the watch thread feeds them: every fixture in
//! `fixtures/claude/status/`, and every status file write in `fixtures/claude/live/`. The source does no I/O, so the
//! home folder is a made-up path and no real `~/.claude` is ever read.

use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;
use wakeshot_core::process::FileTime;
use wakeshot_core::sources::claude::{ClaudeSource, claude_folder};
use wakeshot_core::sources::cron::FixedOffset;
use wakeshot_core::sources::{
    Activity, AgentSource, Fed, Feed, FileId, FormatHealth, Liveness, SessionFacts, SessionKey,
    SourceId, WantedFile,
};

const HOME: &str = r"X:\home";

fn fixture_dir(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("claude")
        .join(name)
}

fn sessions_dir() -> PathBuf {
    PathBuf::from(HOME).join(".claude").join("sessions")
}

fn status_file(pid: u32) -> FileId {
    FileId(sessions_dir().join(format!("{pid}.json")))
}

fn source() -> ClaudeSource {
    ClaudeSource::new(PathBuf::from(HOME), Box::new(FixedOffset(240)))
}

fn read_fixture(name: &str) -> Vec<u8> {
    fs::read(fixture_dir("status").join(name)).unwrap()
}

fn facts(pid: u32, start: u64, activity: Activity, format: FormatHealth) -> SessionFacts {
    SessionFacts {
        key: SessionKey {
            source: SourceId::ClaudeCode,
            id: pid.to_string(),
        },
        liveness: Liveness::Process {
            pid,
            start: FileTime(start),
        },
        activity,
        timers: Vec::new(),
        open_tasks: 0,
        label: "Claude Code",
        format,
    }
}

const CLI: (u32, u64) = (33448, 134_381_285_374_949_759);
const CLI_WAIT: (u32, u64) = (8516, 134_381_292_823_120_614);
const SDK: (u32, u64) = (30436, 134_381_296_680_519_065);

/// One fixture: the pid in the name it is fed under, the answer to each feed, and the facts after the last one.
struct Case {
    file: &'static str,
    pid: u32,
    fed: &'static [Fed],
    facts: Vec<SessionFacts>,
}

fn case(file: &'static str, (pid, start): (u32, u64), activity: Activity) -> Case {
    Case {
        file,
        pid,
        fed: &[Fed::Taken],
        facts: vec![facts(pid, start, activity, FormatHealth::Ok)],
    }
}

fn waiting(since_wall_ms: i64, remote: bool) -> Activity {
    Activity::Waiting {
        since_wall_ms: Some(since_wall_ms),
        remote,
    }
}

fn cases() -> Vec<Case> {
    let unreadable = |file, pid, fed, what| Case {
        file,
        pid,
        fed,
        facts: vec![facts(
            pid,
            0,
            Activity::Idle,
            FormatHealth::Unreadable(what),
        )],
    };
    vec![
        case("before-status.json", CLI, Activity::Idle),
        case("idle-remote.json", CLI, Activity::Idle),
        case(
            "waiting-permission.json",
            CLI_WAIT,
            waiting(1_793_655_699_331, true),
        ),
        case(
            "waiting-input.json",
            CLI_WAIT,
            waiting(1_793_655_720_596, true),
        ),
        case("waiting-dialog.json", CLI_WAIT, Activity::Idle),
        case("remote-off.json", CLI_WAIT, Activity::Idle),
        case("sdk-idle.json", SDK, Activity::Idle),
        case(
            "sdk-waiting-permission.json",
            SDK,
            waiting(1_793_656_093_506, false),
        ),
        case("trailing-bytes.json", CLI_WAIT, Activity::Idle),
        Case {
            file: "unknown-status.json",
            pid: CLI.0,
            fed: &[Fed::Taken],
            facts: vec![facts(
                CLI.0,
                CLI.1,
                Activity::Working,
                FormatHealth::UnknownValue("status", "compacting".into()),
            )],
        },
        unreadable(
            "empty.json",
            CLI.0,
            &[Fed::RetryOnce, Fed::Taken],
            "status file",
        ),
        unreadable(
            "half-written.json",
            CLI.0,
            &[Fed::RetryOnce, Fed::Taken],
            "status file",
        ),
        unreadable("pid-mismatch.json", CLI.0, &[Fed::Taken], "status file pid"),
        unreadable(
            "no-proc-start.json",
            CLI.0,
            &[Fed::Taken],
            "status file procStart",
        ),
    ]
}

/// Each fixture fed to a new source as the watch thread would: once, and once more 200 ms later after `RetryOnce`.
#[test]
fn every_fixture_gives_its_facts() {
    for case in cases() {
        let bytes = read_fixture(case.file);
        let mut source = source();
        let fed: Vec<Fed> = case
            .fed
            .iter()
            .map(|_| source.feed(&status_file(case.pid), Feed::Whole(bytes.clone())))
            .collect();
        assert_eq!(fed, case.fed, "{}", case.file);
        assert_eq!(source.sessions(), case.facts, "{}", case.file);
    }
}

#[test]
fn every_fixture_has_a_case() {
    let on_disk: BTreeSet<String> = fs::read_dir(fixture_dir("status"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .filter(|name| name != "README.md")
        .collect();
    let listed: BTreeSet<String> = cases().iter().map(|c| c.file.to_owned()).collect();
    assert_eq!(on_disk, listed);
}

/// The rules of card deliverable 1, written out again on the live data, so the replay below checks the source
/// against the card and not against itself.
fn expected_activity(content: &Value) -> Activity {
    match (content["status"].as_str(), content["waitingFor"].as_str()) {
        (None | Some("idle"), _) | (Some("waiting"), Some("dialog open")) => Activity::Idle,
        (Some("busy" | "shell"), _) => Activity::Working,
        (Some("waiting"), _) => Activity::Waiting {
            since_wall_ms: content["statusUpdatedAt"].as_i64(),
            remote: content["bridgeSessionId"].is_string(),
        },
        (Some(other), _) => panic!("the live fixtures hold no status {other:?}"),
    }
}

/// Every status file write of the 2.1.317 live sessions (`p4-claude-live-shapes`), in the order seen, one source
/// per case: each write reads, each delete drops the session, and no session is left at the end.
#[test]
fn every_live_status_write_reads() {
    let mut writes = 0;
    let mut seen = BTreeSet::new();
    for entry in fs::read_dir(fixture_dir("live")).unwrap() {
        let path = entry.unwrap().path();
        if !path.to_string_lossy().ends_with(".status.jsonl") {
            continue;
        }
        let mut source = source();
        for line in fs::read_to_string(&path).unwrap().lines() {
            let line: Value = serde_json::from_str(line).unwrap();
            let Some(name) = line["file"].as_str().unwrap().strip_prefix("sessions/") else {
                continue;
            };
            let file = FileId(sessions_dir().join(name));
            let pid: u32 = name.strip_suffix(".json").unwrap().parse().unwrap();
            if line["gone"] == true {
                assert_eq!(source.feed(&file, Feed::Gone), Fed::Taken);
                assert!(
                    source
                        .sessions()
                        .iter()
                        .all(|s| s.key.id != pid.to_string())
                );
                continue;
            }
            let content = &line["content"];
            let bytes = serde_json::to_vec(content).unwrap();
            assert_eq!(source.feed(&file, Feed::Whole(bytes)), Fed::Taken);
            let start: u64 = content["procStart"].as_str().unwrap().parse().unwrap();
            let activity = expected_activity(content);
            let session = source
                .sessions()
                .into_iter()
                .find(|s| s.key.id == pid.to_string())
                .unwrap();
            assert_eq!(
                session,
                facts(pid, start, activity, FormatHealth::Ok),
                "{}: {line}",
                path.display()
            );
            writes += 1;
            seen.insert(format!("{activity:?}"));
        }
        assert!(source.sessions().is_empty(), "{}", path.display());
    }
    assert!(writes > 50, "{writes} writes");
    assert!(
        seen.contains("Working") && seen.contains("Idle"),
        "{seen:?}"
    );
}

#[test]
fn wanted_is_the_sessions_folder_without_key_files() {
    let wanted = source().wanted();
    let [WantedFile::Folder { dir, accept }] = wanted.as_slice() else {
        panic!("{wanted:?}");
    };
    assert_eq!(dir, &sessions_dir());
    assert!(accept("33448.json"));
    for name in [
        "33448.685bf0d758162e180716a9b47a33761d744d0f5da53275e73110abe1a22f559b.key",
        "33448.key",
        "33448.json.key",
        ".fleetview-heartbeat",
        "33448.json.tmp",
    ] {
        assert!(!accept(name), "{name}");
    }
}

/// Live (`p4-live-claude`): a session started with `CLAUDE_CONFIG_DIR` set writes its status file under that folder
/// and nothing under `~\.claude`, so the source must read the folder Claude Code uses.
#[test]
fn claude_config_dir_names_the_folder_read() {
    let home = Path::new(HOME);
    let moved = PathBuf::from(r"Y:\cfg");
    assert_eq!(claude_folder(home, None), home.join(".claude"));
    assert_eq!(
        claude_folder(home, Some(OsStr::new(""))),
        home.join(".claude")
    );
    assert_eq!(claude_folder(home, Some(moved.as_os_str())), moved);

    let mut source = ClaudeSource::in_folder(moved.clone(), Box::new(FixedOffset(240)));
    let wanted = source.wanted();
    let [WantedFile::Folder { dir, .. }] = wanted.as_slice() else {
        panic!("{wanted:?}");
    };
    assert_eq!(dir, &moved.join("sessions"));

    let bytes = read_fixture("idle-remote.json");
    source.feed(&status_file(CLI.0), Feed::Whole(bytes.clone()));
    assert!(
        source.sessions().is_empty(),
        "the profile's folder is not read"
    );
    let file = FileId(moved.join("sessions").join(format!("{}.json", CLI.0)));
    assert_eq!(source.feed(&file, Feed::Whole(bytes)), Fed::Taken);
    assert_eq!(source.sessions().len(), 1);
}

/// A feed the source did not ask for is never parsed, so a `.key` file's bytes could not reach a session even if a
/// caller fed them.
#[test]
fn files_it_did_not_ask_for_are_ignored() {
    let bytes = read_fixture("idle-remote.json");
    let mut source = source();
    for path in [
        sessions_dir().join("33448.685bf0d758162e18.key"),
        sessions_dir().join("033448.json"),
        PathBuf::from(r"X:\elsewhere\33448.json"),
        sessions_dir().join("nested").join("33448.json"),
    ] {
        assert_eq!(
            source.feed(&FileId(path.clone()), Feed::Whole(bytes.clone())),
            Fed::Taken,
            "{}",
            path.display()
        );
    }
    assert!(source.sessions().is_empty());
}

/// A bad read keeps the last good facts until the retry; a good retry carries on as if nothing happened.
#[test]
fn a_good_retry_keeps_the_session() {
    let good = read_fixture("idle-remote.json");
    let busy = String::from_utf8(good.clone())
        .unwrap()
        .replace(r#""status":"idle""#, r#""status":"busy""#);
    let mut source = source();
    let file = status_file(CLI.0);
    assert_eq!(source.feed(&file, Feed::Whole(good)), Fed::Taken);
    let before = source.sessions();
    assert_eq!(
        source.feed(&file, Feed::Whole(read_fixture("half-written.json"))),
        Fed::RetryOnce
    );
    assert_eq!(source.sessions(), before);
    assert_eq!(source.feed(&file, Feed::Whole(busy.into())), Fed::Taken);
    assert_eq!(
        source.sessions(),
        [facts(CLI.0, CLI.1, Activity::Working, FormatHealth::Ok)]
    );
    // The retry flag went with the good read: the next bad read gets its own retry.
    assert_eq!(source.feed(&file, Feed::Whole(Vec::new())), Fed::RetryOnce);
}

/// Two bad reads in a row make the session unreadable, tied to the process the last good read named.
#[test]
fn two_bad_reads_keep_the_last_known_process() {
    let mut source = source();
    let file = status_file(CLI.0);
    source.feed(&file, Feed::Whole(read_fixture("idle-remote.json")));
    assert_eq!(source.feed(&file, Feed::Whole(Vec::new())), Fed::RetryOnce);
    assert_eq!(source.feed(&file, Feed::Whole(Vec::new())), Fed::Taken);
    let unreadable = facts(
        CLI.0,
        CLI.1,
        Activity::Idle,
        FormatHealth::Unreadable("status file"),
    );
    assert_eq!(source.sessions(), std::slice::from_ref(&unreadable));
    assert_eq!(
        source.feed(&file, Feed::Whole(read_fixture("pid-mismatch.json"))),
        Fed::Taken
    );
    assert_eq!(
        source.sessions(),
        [SessionFacts {
            format: FormatHealth::Unreadable("status file pid"),
            ..unreadable
        }]
    );
    // A good write ends it.
    source.feed(&file, Feed::Whole(read_fixture("unknown-status.json")));
    assert_eq!(
        source.sessions(),
        [facts(
            CLI.0,
            CLI.1,
            Activity::Working,
            FormatHealth::UnknownValue("status", "compacting".into())
        )]
    );
}

#[test]
fn gone_drops_the_session() {
    let mut source = source();
    let file = status_file(CLI.0);
    source.feed(&file, Feed::Whole(read_fixture("idle-remote.json")));
    source.feed(
        &status_file(SDK.0),
        Feed::Whole(read_fixture("sdk-idle.json")),
    );
    assert_eq!(source.feed(&file, Feed::Gone), Fed::Taken);
    assert_eq!(
        source.sessions(),
        [facts(SDK.0, SDK.1, Activity::Idle, FormatHealth::Ok)]
    );
    // Gone also ends a pending retry and an unreadable session.
    assert_eq!(source.feed(&file, Feed::Whole(Vec::new())), Fed::RetryOnce);
    source.feed(&file, Feed::Gone);
    assert_eq!(source.feed(&file, Feed::Whole(Vec::new())), Fed::RetryOnce);
    // A status file is never tailed or listed; such a feed changes nothing.
    for data in [Feed::Appended(b"{}".to_vec()), Feed::Present] {
        assert_eq!(source.feed(&status_file(SDK.0), data), Fed::Taken);
    }
    assert_eq!(
        source.sessions(),
        [facts(SDK.0, SDK.1, Activity::Idle, FormatHealth::Ok)]
    );
}

/// The watch thread posts a snapshot only when it changed (track note 14), so the order must not depend on the order
/// of the feeds.
#[test]
fn the_snapshot_order_does_not_depend_on_the_feed_order() {
    let files = [
        (CLI.0, "idle-remote.json"),
        (CLI_WAIT.0, "waiting-permission.json"),
        (SDK.0, "sdk-idle.json"),
    ];
    let mut forward = source();
    for (pid, file) in files {
        forward.feed(&status_file(pid), Feed::Whole(read_fixture(file)));
    }
    let mut backward = source();
    for (pid, file) in files.iter().rev() {
        backward.feed(&status_file(*pid), Feed::Whole(read_fixture(file)));
    }
    assert_eq!(forward.sessions().len(), 3);
    assert_eq!(forward.sessions(), backward.sessions());
}

#[test]
fn the_source_names_itself_and_can_move_to_the_watch_thread() {
    fn boxed(source: ClaudeSource) -> Box<dyn AgentSource + Send> {
        Box::new(source)
    }
    assert_eq!(boxed(source()).id(), SourceId::ClaudeCode);
}
