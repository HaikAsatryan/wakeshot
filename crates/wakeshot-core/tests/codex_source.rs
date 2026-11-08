//! `CodexSource` fed the way the watch thread feeds it: lock file names as `Present` and `Gone`, rollout bytes as
//! `Appended` from the start of the file, whole and in pieces. Every rollout is a fixture in `fixtures/codex/`, built
//! from the schema (its README), or in `fixtures/codex/live/`, redacted from live Codex CLI 0.167.0 sessions (its
//! README). The source does no I/O, so the home folder is a made-up path.

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use wakeshot_core::sources::codex::{self, CodexSource, NO_TURN_EVENT, NOT_JSON};
use wakeshot_core::sources::{
    Activity, AgentSource, Fed, Feed, FileId, FormatHealth, Liveness, Note, SessionFacts,
    SessionKey, SourceId, WantedFile,
};

const HOME: &str = r"X:\home\.codex";
const THREAD: &str = "0199aaaa-0000-7000-8000-000000000001";
const OTHER: &str = "0199aaaa-0000-7000-8000-000000000002";

fn fixture(name: &str) -> Vec<u8> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/codex");
    fs::read(dir.join(name)).unwrap()
}

fn locks_dir() -> PathBuf {
    PathBuf::from(HOME).join("thread-writer-locks")
}

fn sessions_dir() -> PathBuf {
    PathBuf::from(HOME).join("sessions")
}

/// `sessions\2026\11\<day>`, one level per `join`: `\` is a separator on Windows only, so a joined `2026\11\06`
/// would be one folder on Linux and every rollout would sit at the wrong depth.
fn day_dir(day: &str) -> PathBuf {
    date_dir(sessions_dir(), day)
}

fn date_dir(root: PathBuf, day: &str) -> PathBuf {
    root.join("2026").join("11").join(day)
}

fn lock(id: &str) -> FileId {
    FileId(locks_dir().join(format!("{id}.lock")))
}

fn rollout(id: &str) -> FileId {
    FileId(day_dir("06").join(format!("rollout-2026-11-06T12-00-00-{id}.jsonl")))
}

fn facts(id: &str, activity: Activity, format: FormatHealth) -> SessionFacts {
    SessionFacts {
        key: SessionKey {
            source: SourceId::Codex,
            id: id.to_owned(),
        },
        liveness: Liveness::AnyProcess {
            image: "codex.exe",
            lock_file: lock(id).0,
        },
        activity,
        timers: Vec::new(),
        open_tasks: 0,
        label: "Codex",
        format,
    }
}

fn idle(id: &str) -> SessionFacts {
    facts(id, Activity::Idle, FormatHealth::Ok)
}

fn working(id: &str) -> SessionFacts {
    facts(id, Activity::Working, FormatHealth::Ok)
}

/// A source with `THREAD`'s lock file listed.
fn with_thread() -> CodexSource {
    let mut source = CodexSource::new(PathBuf::from(HOME));
    assert_eq!(source.feed(&lock(THREAD), Feed::Present), Fed::Taken);
    source
}

fn append(source: &mut CodexSource, file: &FileId, bytes: &[u8]) {
    assert_eq!(
        source.feed(file, Feed::Appended(bytes.to_vec())),
        Fed::Taken
    );
}

/// Every fixture, with what its last turn event says.
fn expected() -> Vec<(&'static str, Activity, FormatHealth)> {
    let unknown = |kind: &str| FormatHealth::UnknownValue("rollout", kind.to_owned());
    vec![
        ("running.jsonl", Activity::Working, FormatHealth::Ok),
        ("complete.jsonl", Activity::Idle, FormatHealth::Ok),
        ("aborted.jsonl", Activity::Idle, FormatHealth::Ok),
        ("aborted-no-turn-id.jsonl", Activity::Idle, FormatHealth::Ok),
        ("aliases-running.jsonl", Activity::Working, FormatHealth::Ok),
        ("aliases-complete.jsonl", Activity::Idle, FormatHealth::Ok),
        ("before-turn.jsonl", Activity::Idle, FormatHealth::Ok),
        ("junk.jsonl", Activity::Working, FormatHealth::Ok),
        ("replaced.jsonl", Activity::Working, FormatHealth::Ok),
        ("fork-mid-turn.jsonl", Activity::Idle, FormatHealth::Ok),
        ("odd-no-type.jsonl", Activity::Working, unknown("event_msg")),
        (
            "odd-no-turn-id.jsonl",
            Activity::Idle,
            unknown("task_complete"),
        ),
    ]
}

#[test]
fn every_fixture_is_listed_once() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/codex");
    let mut on_disk: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .filter(|name| name.ends_with(".jsonl"))
        .collect();
    on_disk.sort();
    let mut listed: Vec<String> = expected().iter().map(|e| e.0.to_owned()).collect();
    listed.sort();
    assert_eq!(on_disk, listed);
}

#[test]
fn wants_the_lock_folder_by_name_and_one_rollout_per_listed_thread() {
    let mut source = CodexSource::new(PathBuf::from(HOME));
    let wanted = source.wanted();
    let [
        WantedFile::Names {
            dir,
            depth: 0,
            accept,
        },
    ] = wanted.as_slice()
    else {
        panic!("only the lock folder before any lock: {wanted:?}");
    };
    assert_eq!(dir, &locks_dir());
    for name in [
        format!("{THREAD}.lock"),
        "0199AAAA-0000-7000-8000-00000000000F.lock".to_owned(),
    ] {
        assert!(accept(&name), "{name}");
    }
    for name in [
        ".coordination.lock",
        "x.lock",
        "0199aaaa-0000-7000-8000-000000000001",
        "0199aaaa-0000-7000-8000-000000000001.lock.tmp",
        "0199aaaa-0000-7000-8000-0000000000011.lock",
        "0199aaaa_0000_7000_8000_000000000001.lock",
        "0199aaaa-0000-7000-8000-00000000000g.lock",
        "{0199aaaa-0000-7000-8000-000000000001}.lock",
    ] {
        assert!(!accept(name), "{name}");
    }
    assert_eq!(source.id(), SourceId::Codex);
    assert_eq!(source.sessions(), []);

    source.feed(&lock(OTHER), Feed::Present);
    // An upper-case name is the same thread: the finder compares names without case.
    source.feed(
        &FileId(locks_dir().join(format!("{}.lock", THREAD.to_uppercase()))),
        Feed::Present,
    );
    let wanted = source.wanted();
    assert_eq!(wanted.len(), 4, "{wanted:?}");
    assert_sessions_watch(&wanted[1]);
    let tails: Vec<(PathBuf, u8, String)> = wanted[2..]
        .iter()
        .map(|w| match w {
            WantedFile::FindNewest {
                root,
                depth,
                key,
                matches,
            } => {
                let plain = format!("rollout-2026-11-06T12-00-00-{key}.jsonl");
                let reverted = format!("rollout-2026-11-06T12-00-00-{key}_{THREAD}.jsonl");
                assert!(matches(&plain, key) && matches(&reverted, key), "{key}");
                (root.clone(), *depth, key.clone())
            }
            other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(
        tails,
        [
            (sessions_dir(), 3, THREAD.to_owned()),
            (sessions_dir(), 3, OTHER.to_owned()),
        ]
    );
}

/// The `sessions` listing is there only so the watch thread watches the tree: a subtree watch (depth 1 or more), and
/// no name passes its filter, so it never feeds a file.
fn assert_sessions_watch(want: &WantedFile) {
    let WantedFile::Names { dir, depth, accept } = want else {
        panic!("not the sessions listing: {want:?}");
    };
    assert_eq!(dir, &sessions_dir());
    assert!(*depth >= 1, "a watch of the whole tree");
    for name in [
        format!("rollout-2026-11-06T12-00-00-{THREAD}.jsonl"),
        format!("rollout-2026-11-06T12-00-00-{THREAD}_{OTHER}.jsonl"),
        format!("{THREAD}.lock"),
        "11".to_owned(),
        "a.txt".to_owned(),
    ] {
        assert!(!accept(&name), "{name}");
    }
}

#[test]
fn the_sessions_tree_is_wanted_for_its_watch_only_while_a_thread_is_listed() {
    let mut source = CodexSource::new(PathBuf::from(HOME));
    assert_eq!(source.wanted().len(), 1, "the lock folder only");

    source.feed(&lock(THREAD), Feed::Present);
    let wanted = source.wanted();
    assert_eq!(wanted.len(), 3, "{wanted:?}");
    assert_sessions_watch(&wanted[1]);
    assert!(matches!(wanted[2], WantedFile::FindNewest { .. }));

    // A file the listing could ever name changes nothing: it takes no name, and the source ignores a stray feed.
    let year = FileId(sessions_dir().join("2026").join("x.jsonl"));
    assert_eq!(source.feed(&year, Feed::Present), Fed::Taken);
    assert_eq!(source.sessions(), [idle(THREAD)]);

    source.feed(&lock(THREAD), Feed::Gone);
    assert_eq!(source.wanted().len(), 1, "no thread, no rollout to follow");
}

#[test]
fn no_want_opens_a_lock_file() {
    let mut source = with_thread();
    append(&mut source, &rollout(THREAD), &fixture("running.jsonl"));
    for want in source.wanted() {
        match want {
            WantedFile::Names { ref dir, .. } if *dir == sessions_dir() => {
                assert_sessions_watch(&want);
            }
            WantedFile::Names { dir, .. } => assert_eq!(dir, locks_dir()),
            WantedFile::FindNewest {
                root, key, matches, ..
            } => {
                assert_eq!(root, sessions_dir());
                assert!(!matches(&format!("{key}.lock"), &key));
            }
            other @ (WantedFile::Folder { .. }
            | WantedFile::Tail(_)
            | WantedFile::FindTail { .. }
            | WantedFile::Recent { .. }
            | WantedFile::Query { .. }) => {
                panic!("a want that reads files by folder or path: {other:?}")
            }
        }
    }
}

#[test]
fn rollout_names_match_the_plain_and_the_reverted_form_only() {
    let date = "rollout-2026-11-06T12-00-00";
    for name in [
        format!("{date}-{THREAD}.jsonl"),
        format!("{date}-{THREAD}_{OTHER}.jsonl"),
        format!(
            "{date}-{}_{}.JSONL",
            THREAD.to_uppercase(),
            OTHER.to_uppercase()
        ),
        format!("{date}-{THREAD}_0199aaaa.jsonl"),
    ] {
        assert!(codex::is_rollout_of(&name, THREAD), "{name}");
    }
    for name in [
        format!("{date}-{OTHER}.jsonl"),
        format!("{date}-{OTHER}_{THREAD}.jsonl"),
        format!("{date}_{THREAD}.jsonl"),
        format!("{date}-{THREAD}_.jsonl"),
        format!("{date}-{THREAD}_a.b.jsonl"),
        format!("{date}-{THREAD}_a_b.jsonl"),
        format!("{date}-{THREAD}_{}.jsonl", "a".repeat(65)),
        format!("{date}-{THREAD}.jsonl.zst"),
        format!("{date}-{THREAD}x.jsonl"),
        format!("{THREAD}.lock"),
    ] {
        assert!(!codex::is_rollout_of(&name, THREAD), "{name}");
    }
}

/// The watch thread's switch to a reverted thread's new file: the old one `Gone`, then the new one from its start.
#[test]
fn a_reverted_thread_goes_on_in_its_new_rollout() {
    let mut source = with_thread();
    let reverted = FileId(day_dir("06").join(format!(
        "rollout-2026-11-06T12-30-00-{THREAD}_{OTHER}.jsonl"
    )));
    append(&mut source, &rollout(THREAD), &fixture("complete.jsonl"));
    assert_eq!(source.sessions(), [idle(THREAD)]);
    append(&mut source, &reverted, &fixture("running.jsonl"));
    assert_eq!(
        source.sessions(),
        [idle(THREAD)],
        "the old file is still the one it reads"
    );

    assert_eq!(source.feed(&rollout(THREAD), Feed::Gone), Fed::Taken);
    append(&mut source, &reverted, &fixture("running.jsonl"));
    assert_eq!(source.sessions(), [working(THREAD)]);
    append(
        &mut source,
        &reverted,
        concat!(
            r#"{"type":"event_msg","payload":{"type":"task_complete","turn_id":"t1"}}"#,
            "\n"
        )
        .as_bytes(),
    );
    assert_eq!(source.sessions(), [idle(THREAD)]);

    // A reverted file of another thread is not this one's.
    let other = FileId(day_dir("06").join(format!(
        "rollout-2026-11-06T12-31-00-{OTHER}_{THREAD}.jsonl"
    )));
    source.feed(&reverted, Feed::Gone);
    append(&mut source, &other, &fixture("running.jsonl"));
    assert_eq!(source.sessions(), [idle(THREAD)]);
}

#[test]
fn a_listed_lock_file_is_an_idle_session_until_its_rollout_says_more() {
    let mut source = with_thread();
    assert_eq!(source.sessions(), [idle(THREAD)]);

    // Listed again: still one session.
    source.feed(&lock(THREAD), Feed::Present);
    assert_eq!(source.sessions(), [idle(THREAD)]);

    append(&mut source, &rollout(THREAD), &fixture("running.jsonl"));
    assert_eq!(source.sessions(), [working(THREAD)]);
}

#[test]
fn a_lock_file_gone_ends_the_session_and_what_its_rollout_said() {
    let mut source = with_thread();
    source.feed(&lock(OTHER), Feed::Present);
    append(&mut source, &rollout(THREAD), &fixture("running.jsonl"));
    append(&mut source, &rollout(OTHER), &fixture("complete.jsonl"));
    assert_eq!(source.sessions(), [working(THREAD), idle(OTHER)]);

    assert_eq!(source.feed(&lock(THREAD), Feed::Gone), Fed::Taken);
    assert_eq!(source.sessions(), [idle(OTHER)]);
    assert_eq!(source.wanted().len(), 3, "its rollout is no longer wanted");

    // The same thread opened again starts from what its rollout says from the start.
    source.feed(&lock(THREAD), Feed::Present);
    assert_eq!(source.sessions(), [idle(THREAD), idle(OTHER)]);
    append(&mut source, &rollout(THREAD), &fixture("complete.jsonl"));
    assert_eq!(source.sessions(), [idle(THREAD), idle(OTHER)]);
}

#[test]
fn each_fixture_gives_the_activity_of_its_last_turn_event() {
    for (name, activity, format) in expected() {
        let mut source = with_thread();
        append(&mut source, &rollout(THREAD), &fixture(name));
        assert_eq!(
            source.sessions(),
            [facts(THREAD, activity, format)],
            "{name}"
        );
    }
}

#[test]
fn bytes_split_anywhere_give_the_same_answer() {
    for (name, activity, format) in expected() {
        let bytes = fixture(name);
        for size in [1, 2, 3, 7, 64, 333] {
            let mut source = with_thread();
            for piece in bytes.chunks(size) {
                append(&mut source, &rollout(THREAD), piece);
            }
            assert_eq!(
                source.sessions(),
                [facts(THREAD, activity, format.clone())],
                "{name} in pieces of {size}"
            );
        }
    }
}

#[test]
fn crlf_lines_read_the_same() {
    for (name, activity, format) in expected() {
        let crlf: Vec<u8> = fixture(name)
            .split_inclusive(|&b| b == b'\n')
            .flat_map(|line| {
                let body = line.strip_suffix(b"\n").unwrap_or(line);
                [body, b"\r\n"].concat()
            })
            .collect();
        let mut source = with_thread();
        append(&mut source, &rollout(THREAD), &crlf);
        assert_eq!(
            source.sessions(),
            [facts(THREAD, activity, format)],
            "{name}"
        );
    }
}

#[test]
fn line_by_line_each_turn_event_moves_the_activity() {
    let mut source = with_thread();
    let mut seen = Vec::new();
    for line in fixture("fork-mid-turn.jsonl").split_inclusive(|&b| b == b'\n') {
        append(&mut source, &rollout(THREAD), line);
        seen.push(source.sessions()[0].activity);
    }
    use Activity::{Idle, Working};
    assert_eq!(
        seen,
        [Idle, Working, Working, Idle, Idle, Working, Idle],
        "meta, the parent's start, an item, its boundary, settings, the child's start, its end"
    );
}

#[test]
fn a_line_without_its_newline_waits_for_the_rest() {
    let mut source = with_thread();
    append(&mut source, &rollout(THREAD), &fixture("running.jsonl"));
    let end = br#"{"timestamp":"2026-11-06T10:01:00.000Z","type":"event_msg","payload":{"type":"task_complete","turn_id":"t1","last_agent_message":null}}"#;
    append(&mut source, &rollout(THREAD), end);
    assert_eq!(source.sessions(), [working(THREAD)], "no newline yet");
    append(&mut source, &rollout(THREAD), b"\n");
    assert_eq!(source.sessions(), [idle(THREAD)]);
}

#[test]
fn a_readable_turn_event_clears_an_unreadable_one() {
    let mut source = with_thread();
    append(
        &mut source,
        &rollout(THREAD),
        &fixture("odd-no-turn-id.jsonl"),
    );
    assert_eq!(
        source.sessions()[0].format,
        FormatHealth::UnknownValue("rollout", "task_complete".to_owned())
    );
    let next = concat!(
        r#"{"timestamp":"2026-11-06T10:02:00.000Z","type":"event_msg","payload":{"type":"task_started","turn_id":"t2"}}"#,
        "\n",
        r#"{"timestamp":"2026-11-06T10:02:01.000Z","type":"event_msg","payload":{"type":"token_count","info":null}}"#,
        "\n",
    );
    append(&mut source, &rollout(THREAD), next.as_bytes());
    assert_eq!(source.sessions(), [working(THREAD)]);

    // A turn event whose `turn_id` is not text is unreadable too; an aborted turn's null `turn_id` is not.
    let odd = concat!(
        r#"{"type":"event_msg","payload":{"type":"turn_aborted","turn_id":7,"reason":"interrupted"}}"#,
        "\n"
    );
    append(&mut source, &rollout(THREAD), odd.as_bytes());
    assert_eq!(
        source.sessions(),
        [facts(
            THREAD,
            Activity::Idle,
            FormatHealth::UnknownValue("rollout", "turn_aborted".to_owned())
        )]
    );
    let null = concat!(
        r#"{"type":"event_msg","payload":{"type":"turn_aborted","turn_id":null,"reason":"interrupted"}}"#,
        "\n"
    );
    append(&mut source, &rollout(THREAD), null.as_bytes());
    assert_eq!(source.sessions(), [idle(THREAD)]);

    // A payload that is not an object has no type either.
    let bare = concat!(r#"{"type":"event_msg","payload":"task_started"}"#, "\n");
    append(&mut source, &rollout(THREAD), bare.as_bytes());
    assert_eq!(
        source.sessions(),
        [facts(
            THREAD,
            Activity::Idle,
            FormatHealth::UnknownValue("rollout", "event_msg".to_owned())
        )]
    );
}

#[test]
fn a_gone_rollout_forgets_its_turns_and_a_restart_reads_from_the_start() {
    let mut source = with_thread();
    append(&mut source, &rollout(THREAD), &fixture("running.jsonl"));
    assert_eq!(source.feed(&rollout(THREAD), Feed::Gone), Fed::Taken);
    assert_eq!(
        source.sessions(),
        [idle(THREAD)],
        "the lock file still says it is loaded"
    );

    // The watch thread feeds a file that got shorter or was replaced as gone, then its lines from the start.
    append(&mut source, &rollout(THREAD), &fixture("running.jsonl"));
    assert_eq!(source.sessions(), [working(THREAD)]);
    source.feed(&rollout(THREAD), Feed::Gone);
    append(&mut source, &rollout(THREAD), &fixture("complete.jsonl"));
    assert_eq!(source.sessions(), [idle(THREAD)]);
}

#[test]
fn a_second_rollout_of_the_same_thread_waits_until_the_first_is_gone() {
    let mut source = with_thread();
    let second = FileId(day_dir("07").join(format!("rollout-2026-11-07T08-00-00-{THREAD}.jsonl")));
    append(&mut source, &rollout(THREAD), &fixture("complete.jsonl"));
    append(&mut source, &second, &fixture("running.jsonl"));
    assert_eq!(source.sessions(), [idle(THREAD)]);

    source.feed(&second, Feed::Gone);
    assert_eq!(source.sessions(), [idle(THREAD)], "not the file it reads");
    source.feed(&rollout(THREAD), Feed::Gone);
    append(&mut source, &second, &fixture("running.jsonl"));
    assert_eq!(source.sessions(), [working(THREAD)]);
}

#[test]
fn files_it_did_not_ask_for_change_nothing() {
    let mut source = with_thread();
    let running = fixture("running.jsonl");
    let date = day_dir("06");
    let strangers = [
        // A thread with no lock file, plain and reverted.
        rollout(OTHER),
        FileId(date.join(format!(
            "rollout-2026-11-06T12-00-00-{OTHER}_{THREAD}.jsonl"
        ))),
        FileId(date.join(format!("rollout-2026-11-06T12-00-00-{THREAD}.jsonl.zst"))),
        FileId(date.join(format!("rollout-2026-11-06T12-00-00-{THREAD}_.jsonl"))),
        // The right name at the wrong depth, or under another root.
        FileId(sessions_dir().join(format!("rollout-2026-11-06T12-00-00-{THREAD}.jsonl"))),
        FileId(
            date_dir(PathBuf::from(HOME).join("archived_sessions"), "06")
                .join(format!("rollout-2026-11-06T12-00-00-{THREAD}.jsonl")),
        ),
        FileId(date.join(format!("rollout-2026-11-06T12-00-00_{THREAD}.jsonl"))),
        FileId(date.join(format!("{THREAD}.jsonl"))),
        FileId(date.join("a.jsonl")),
    ];
    for file in &strangers {
        append(&mut source, file, &running);
        assert_eq!(source.sessions(), [idle(THREAD)], "{file:?}");
    }

    // A lock file is listed, never read: bytes for it are no session.
    let mut fresh = CodexSource::new(PathBuf::from(HOME));
    assert_eq!(
        fresh.feed(&lock(THREAD), Feed::Whole(running.clone())),
        Fed::Taken
    );
    assert_eq!(
        fresh.feed(&lock(THREAD), Feed::Appended(running)),
        Fed::Taken
    );
    fresh.feed(
        &FileId(locks_dir().join(".coordination.lock")),
        Feed::Present,
    );
    fresh.feed(
        &FileId(PathBuf::from(HOME).join(format!("{THREAD}.lock"))),
        Feed::Present,
    );
    assert_eq!(fresh.sessions(), []);
}

#[test]
fn an_over_long_line_is_skipped_and_the_next_one_counts() {
    let mut source = with_thread();
    append(&mut source, &rollout(THREAD), &fixture("running.jsonl"));
    let mut long = br#"{"type":"event_msg","payload":{"type":"task_complete","turn_id":"t1","last_agent_message":""#.to_vec();
    long.resize(33 << 20, b'a');
    append(&mut source, &rollout(THREAD), &long);
    append(&mut source, &rollout(THREAD), b"\"}}\n");
    assert_eq!(
        source.sessions(),
        [working(THREAD)],
        "the long line is dropped"
    );

    let end = concat!(
        r#"{"type":"event_msg","payload":{"type":"task_complete","turn_id":"t1"}}"#,
        "\n"
    );
    append(&mut source, &rollout(THREAD), end.as_bytes());
    assert_eq!(source.sessions(), [idle(THREAD)]);
}

#[test]
fn the_home_folder_is_codex_home_when_set_else_dot_codex() {
    let profile = Path::new(r"C:\Users\x");
    assert_eq!(codex::codex_home(profile, None), profile.join(".codex"));
    assert_eq!(
        codex::codex_home(profile, Some(OsStr::new(""))),
        profile.join(".codex"),
        "an empty value is unset, as Codex reads it"
    );
    assert_eq!(
        codex::codex_home(profile, Some(OsStr::new(r"D:\codex"))),
        PathBuf::from(r"D:\codex")
    );
}

// Notes.

fn note(id: &str, text: &'static str) -> Note {
    Note {
        session: Some(id.to_owned()),
        text,
        field: None,
    }
}

fn event(kind: &str, turn_id: Option<&str>) -> Vec<u8> {
    let turn = turn_id.map_or(String::new(), |id| format!(r#","turn_id":"{id}""#));
    let mut line = format!(
        r#"{{"timestamp":"2026-11-06T10:00:05.000Z","type":"event_msg","payload":{{"type":"{kind}"{turn}}}}}"#
    )
    .into_bytes();
    line.push(b'\n');
    line
}

/// Of the fixtures, only `junk.jsonl` has an event line that is not JSON (two: a line of words and a torn
/// `task_complete`); every other one knows its turn events or names no turn.
#[test]
fn each_fixture_has_the_notes_of_its_shape() {
    for (name, _, _) in expected() {
        let mut source = with_thread();
        append(&mut source, &rollout(THREAD), &fixture(name));
        let want = if name == "junk.jsonl" {
            vec![note(THREAD, NOT_JSON)]
        } else {
            vec![]
        };
        assert_eq!(source.notes(), want, "{name}");
    }
}

/// Event lines that name a turn but no turn event this version knows (a rename) are a note while it lasts, never a
/// format problem: the session reads as idle and shows no notice. Events before a first turn name none.
#[test]
fn turn_lines_without_a_known_turn_event_are_a_note() {
    let turn = Some("019a0000-0000-7000-8000-000000000101");
    let mut source = with_thread();
    append(
        &mut source,
        &rollout(THREAD),
        &event("thread_settings_applied", None),
    );
    assert_eq!(source.notes(), [], "no turn named yet");
    append(&mut source, &rollout(THREAD), &event("turn_begun", turn));
    append(
        &mut source,
        &rollout(THREAD),
        &event("item_completed", turn),
    );
    assert_eq!(source.sessions(), [idle(THREAD)]);
    assert_eq!(source.notes(), [note(THREAD, NO_TURN_EVENT)]);

    // A turn event it knows, even one it cannot read, ends the note.
    append(&mut source, &rollout(THREAD), &event("task_complete", None));
    assert_eq!(
        source.sessions(),
        [facts(
            THREAD,
            Activity::Idle,
            FormatHealth::UnknownValue("rollout", "task_complete".to_owned())
        )]
    );
    assert_eq!(source.notes(), []);

    // A new rollout starts over, and a gone lock ends it with its session.
    source.feed(&rollout(THREAD), Feed::Gone);
    append(&mut source, &rollout(THREAD), &event("turn_begun", turn));
    assert_eq!(source.notes(), [note(THREAD, NO_TURN_EVENT)]);
    source.feed(&lock(THREAD), Feed::Gone);
    assert_eq!(source.notes(), []);
}

/// An event line that is not JSON is skipped, so a turn event in it is lost: a note for the rest of the rollout, so a
/// format that writes many such lines logs once.
#[test]
fn an_event_line_that_is_not_json_is_a_note_for_the_rest_of_its_rollout() {
    let mut source = with_thread();
    source.feed(&lock(OTHER), Feed::Present);
    append(
        &mut source,
        &rollout(THREAD),
        &event("task_started", Some("t")),
    );
    append(
        &mut source,
        &rollout(THREAD),
        b"{\"type\":\"event_msg\",\"payload\":{\"type\":\"task_complete\"\n",
    );
    assert_eq!(source.sessions()[0], working(THREAD), "the turn stays open");
    assert_eq!(source.notes(), [note(THREAD, NOT_JSON)]);
    append(
        &mut source,
        &rollout(THREAD),
        &event("task_complete", Some("t")),
    );
    assert_eq!(source.notes(), [note(THREAD, NOT_JSON)], "it stays");
    append(&mut source, &rollout(THREAD), b"not json, no marker\n");
    assert_eq!(source.notes(), [note(THREAD, NOT_JSON)]);

    // Only that thread's; its rollout gone, the note goes.
    append(&mut source, &rollout(OTHER), b"[1,\"event_msg\"]\n");
    assert_eq!(source.notes(), [note(THREAD, NOT_JSON)]);
    source.feed(&rollout(THREAD), Feed::Gone);
    assert_eq!(source.notes(), []);
}

// Live rollouts.

fn live(name: &str) -> Vec<u8> {
    fixture(&format!("live/{name}"))
}

/// A live rollout, the lines (counted from 1) after which the activity changes with the activity from there on, and
/// the activity at its end.
type LiveCase = (&'static str, Vec<(usize, Activity)>, Activity);

fn live_expected() -> Vec<LiveCase> {
    use Activity::{Idle, Working};
    vec![
        (
            "tui-four-turns.jsonl",
            vec![
                (2, Working),
                (31, Idle),
                (33, Working),
                (52, Idle),
                (54, Working),
                (71, Idle),
                (73, Working),
                (81, Idle),
            ],
            Idle,
        ),
        ("tui-reverted.jsonl", vec![(4, Working), (12, Idle)], Idle),
        ("tui-crash-mid-turn.jsonl", vec![(2, Working)], Working),
        ("exec-one-turn.jsonl", vec![(2, Working), (31, Idle)], Idle),
    ]
}

#[test]
fn every_live_fixture_is_listed_once() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/codex/live");
    let mut on_disk: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .filter(|name| name.ends_with(".jsonl"))
        .collect();
    on_disk.sort();
    let mut listed: Vec<String> = live_expected().iter().map(|e| e.0.to_owned()).collect();
    listed.sort();
    assert_eq!(on_disk, listed);
}

/// Line by line, as Codex wrote them: every real line reads (no format problem), only the turn events move the
/// activity, and an item that ends after an aborted turn does not open it again.
#[test]
fn live_rollouts_move_the_activity_only_at_their_turn_events() {
    for (name, changes, end) in live_expected() {
        let mut source = with_thread();
        let mut now = Activity::Idle;
        let bytes = live(name);
        for (i, line) in bytes.split_inclusive(|&b| b == b'\n').enumerate() {
            append(&mut source, &rollout(THREAD), line);
            if let Some((_, next)) = changes.iter().find(|(at, _)| *at == i + 1) {
                now = *next;
            }
            assert_eq!(
                source.sessions(),
                [facts(THREAD, now, FormatHealth::Ok)],
                "{name} line {}",
                i + 1
            );
            assert_eq!(source.notes(), [], "{name} line {}", i + 1);
        }
        assert_eq!(now, end, "{name}");

        for size in [1, 7, 333, 4096] {
            let mut source = with_thread();
            for piece in bytes.chunks(size) {
                append(&mut source, &rollout(THREAD), piece);
            }
            assert_eq!(
                source.sessions(),
                [facts(THREAD, end, FormatHealth::Ok)],
                "{name} in pieces of {size}"
            );
        }
    }
}

/// The feeds the watch thread gave during the live backtrack: Codex dropped and made the lock file twice; the first
/// time the new file did not exist yet, so the old one was read again from its start; the second time the new one
/// was the newest.
#[test]
fn the_live_backtrack_ends_on_the_reverted_rollout() {
    let reverted = FileId(day_dir("06").join(format!(
        "rollout-2026-11-06T14-35-34-{THREAD}_{OTHER}.jsonl"
    )));
    let mut source = with_thread();
    append(&mut source, &rollout(THREAD), &live("tui-four-turns.jsonl"));
    assert_eq!(source.sessions(), [idle(THREAD)]);

    source.feed(&lock(THREAD), Feed::Gone);
    assert_eq!(source.sessions(), []);
    source.feed(&lock(THREAD), Feed::Present);
    append(&mut source, &rollout(THREAD), &live("tui-four-turns.jsonl"));
    assert_eq!(source.sessions(), [idle(THREAD)]);

    source.feed(&lock(THREAD), Feed::Gone);
    source.feed(&lock(THREAD), Feed::Present);
    let mut seen = Vec::new();
    for line in live("tui-reverted.jsonl").split_inclusive(|&b| b == b'\n') {
        append(&mut source, &reverted, line);
        seen.push(source.sessions()[0].activity);
    }
    use Activity::{Idle, Working};
    assert_eq!(seen[..3], [Idle; 3], "meta and settings");
    assert_eq!(seen[3..11], [Working; 8], "the edited prompt's turn");
    assert_eq!(seen[11], Idle);
    assert_eq!(source.sessions(), [idle(THREAD)]);
}
