//! `OpenCodeSource` fed the way the watch thread feeds it: each query's answer whole, under `FileId::query(db, name)`,
//! and `Gone` once the database closed. The answers are fixtures in `fixtures/opencode/` (its README); the queries
//! themselves run on a real database in `wakeshot-windows`' `sqlite` tests. The source does no I/O, so the database
//! path is made up.

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use wakeshot_core::command::Command;
use wakeshot_core::core::Core;
use wakeshot_core::event::Event;
use wakeshot_core::files::Config;
use wakeshot_core::process::{Alive, FileTime, ProcessRef};
use wakeshot_core::sources::opencode::{
    self, ALL_SQL, IMAGES, OpenCodeSource, PROBLEM_KEY, SCHEDULES, Sql, TABLES, UNREADABLE, WORKING,
};
use wakeshot_core::sources::{
    Activity, AgentSource, Fed, Feed, FileId, FormatHealth, Liveness, SessionFacts, SessionKey,
    SourceId, WantedFile,
};
use wakeshot_core::time::Now;

const DB: &str = r"X:\home\.local\share\opencode\opencode.db";

const BUSY_A: &str = "ses_1a2b3c4d5e6fAbCdEfGhIjKlMn";
const BUSY_B: &str = "ses_0f0e0d0c0b0aZyXwVuTsRqPoNm";
const IDLE: &str = "ses_9999999999ffIdleIdleIdleId";

fn fixture(name: &str) -> Vec<u8> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/opencode");
    fs::read(dir.join(name)).unwrap()
}

fn db() -> PathBuf {
    PathBuf::from(DB)
}

fn id(sql: Sql) -> FileId {
    FileId::query(&db(), sql.name)
}

fn feed(source: &mut OpenCodeSource, sql: Sql, name: &str) {
    assert_eq!(
        source.feed(&id(sql), Feed::Whole(fixture(name))),
        Fed::Taken
    );
}

fn gone(source: &mut OpenCodeSource, sql: Sql) {
    assert_eq!(source.feed(&id(sql), Feed::Gone), Fed::Taken);
}

/// The names of the queries the source asks for, after checking each is one of its fixed queries on `DB`.
fn wanted(source: &OpenCodeSource) -> Vec<&'static str> {
    source
        .wanted()
        .into_iter()
        .map(|want| {
            let WantedFile::Query { db: at, name, sql } = want else {
                panic!("{want:?}");
            };
            assert_eq!(at, db());
            assert!(ALL_SQL.contains(&Sql { name, sql }), "{name}");
            name
        })
        .collect()
}

fn facts(id: &str, activity: Activity, since: Option<i64>, timers: &[i64]) -> SessionFacts {
    SessionFacts {
        key: SessionKey {
            source: SourceId::OpenCode,
            id: id.to_owned(),
        },
        liveness: Liveness::ImageSince {
            images: &["opencode.exe", "opencode-cli.exe"],
            since_wall_ms: since,
        },
        activity,
        timers: timers.to_vec(),
        open_tasks: 0,
        label: "OpenCode",
        format: FormatHealth::Ok,
    }
}

fn working(id: &str, since: i64, timers: &[i64]) -> SessionFacts {
    facts(id, Activity::Working, Some(since), timers)
}

fn idle(id: &str, timers: &[i64]) -> SessionFacts {
    facts(id, Activity::Idle, None, timers)
}

fn problem() -> SessionFacts {
    SessionFacts {
        format: FormatHealth::Unreadable("opencode.db"),
        ..idle("opencode.db", &[])
    }
}

/// A source whose probe found both tables.
fn with_tables() -> OpenCodeSource {
    let mut source = OpenCodeSource::new(db());
    feed(&mut source, TABLES, "tables-both.json");
    source
}

#[test]
fn the_database_is_in_the_xdg_data_folder() {
    // One level per `join`: `\` is a separator on Windows only.
    let profile = Path::new(r"C:\Users\x");
    let default = profile
        .join(".local")
        .join("share")
        .join("opencode")
        .join("opencode.db");
    assert_eq!(opencode::opencode_db(profile, None), default);
    assert_eq!(
        opencode::opencode_db(profile, Some(OsStr::new(""))),
        default,
        "empty is unset, as `||` reads it"
    );
    assert_eq!(
        opencode::opencode_db(profile, Some(OsStr::new(r"D:\data"))),
        Path::new(r"D:\data").join("opencode").join("opencode.db")
    );
}

#[test]
fn the_constants_are_what_the_watch_thread_and_the_tooltip_use() {
    assert_eq!(IMAGES, ["opencode.exe", "opencode-cli.exe"]);
    assert_eq!(
        (opencode::LABEL, UNREADABLE, PROBLEM_KEY),
        ("OpenCode", "opencode.db", "opencode.db")
    );
    assert_eq!(OpenCodeSource::new(db()).id(), SourceId::OpenCode);
}

#[test]
fn it_asks_for_the_probe_first_and_then_only_for_the_tables_found() {
    let mut source = OpenCodeSource::new(db());
    assert_eq!(wanted(&source), ["tables"]);
    assert!(source.sessions().is_empty());

    feed(&mut source, TABLES, "tables-both.json");
    assert_eq!(wanted(&source), ["tables", "working", "schedules"]);
    feed(&mut source, TABLES, "tables-sessions.json");
    assert_eq!(wanted(&source), ["tables", "working"]);
    feed(&mut source, TABLES, "tables-none.json");
    assert_eq!(wanted(&source), ["tables"], "a file only 1.x wrote");
    assert!(source.sessions().is_empty());
}

#[test]
fn a_running_turn_is_working_from_its_busy_time() {
    let mut source = with_tables();
    feed(&mut source, WORKING, "working.json");
    assert_eq!(
        source.sessions(),
        [
            working(BUSY_B, 1_793_750_460_000, &[]),
            working(BUSY_A, 1_793_750_400_000, &[]),
        ]
    );
    feed(&mut source, WORKING, "working-none.json");
    assert!(source.sessions().is_empty(), "an answer replaces the last");
}

#[test]
fn schedules_give_timers_to_their_session_or_to_an_entry_of_their_own() {
    let mut source = with_tables();
    feed(&mut source, WORKING, "working.json");
    feed(&mut source, SCHEDULES, "schedules.json");
    assert_eq!(
        source.sessions(),
        [
            idle(
                "schedule:sch_0000000000a5AAAAAAAAAAAAAA",
                &[1_793_764_800_000]
            ),
            working(BUSY_B, 1_793_750_460_000, &[]),
            working(
                BUSY_A,
                1_793_750_400_000,
                &[1_793_754_000_000, 1_793_757_600_000]
            ),
            idle(IDLE, &[1_793_761_200_000]),
        ],
        "timers sorted, each once; a schedule alone holds through the core's timer rule only"
    );
}

#[test]
fn gone_drops_what_its_query_said_and_a_gone_probe_drops_everything() {
    let mut source = with_tables();
    feed(&mut source, WORKING, "working.json");
    feed(&mut source, SCHEDULES, "schedules.json");
    gone(&mut source, WORKING);
    assert!(
        source
            .sessions()
            .iter()
            .all(|s| s.activity == Activity::Idle),
        "schedules stay"
    );
    assert_eq!(source.sessions().len(), 3);

    feed(&mut source, WORKING, "error-prepare.json");
    gone(&mut source, TABLES);
    assert!(source.sessions().is_empty(), "rows and the problem");
    assert_eq!(wanted(&source), ["tables"]);
}

#[test]
fn an_error_answer_is_one_unreadable_entry_until_a_good_answer() {
    for bad in ["error-prepare.json", "not-json.txt", "rows-not-a-list.json"] {
        let mut source = with_tables();
        feed(&mut source, WORKING, "working.json");
        feed(&mut source, SCHEDULES, "schedules.json");
        feed(&mut source, WORKING, bad);
        let sessions = source.sessions();
        assert_eq!(sessions.last(), Some(&problem()), "{bad}");
        assert!(
            sessions.iter().all(|s| s.activity == Activity::Idle),
            "{bad}: no signal for the turns"
        );
        assert_eq!(sessions.len(), 4, "{bad}: the schedules stay");

        feed(&mut source, WORKING, "working.json");
        assert!(!source.sessions().contains(&problem()), "{bad}: mended");
    }
}

#[test]
fn a_probe_that_fails_asks_for_nothing_else_and_reports_it() {
    let mut source = with_tables();
    feed(&mut source, WORKING, "working.json");
    feed(&mut source, TABLES, "error-prepare.json");
    assert_eq!(wanted(&source), ["tables"]);
    assert_eq!(source.sessions(), [problem()]);
    feed(&mut source, TABLES, "tables-both.json");
    assert!(source.sessions().is_empty());
    assert_eq!(wanted(&source), ["tables", "working", "schedules"]);
}

#[test]
fn a_windows_without_sqlite_gives_no_signal_and_no_notice() {
    let mut source = with_tables();
    feed(&mut source, WORKING, "working.json");
    feed(&mut source, SCHEDULES, "error-prepare.json");
    feed(&mut source, WORKING, "error-no-sqlite.json");
    assert_eq!(
        source.sessions(),
        [problem()],
        "the schedules' own problem only"
    );
    feed(&mut source, SCHEDULES, "error-no-sqlite.json");
    assert!(
        source.sessions().is_empty(),
        "updating Wakeshot cannot mend it"
    );
    feed(&mut source, TABLES, "error-no-sqlite.json");
    assert_eq!(wanted(&source), ["tables"]);
    assert!(source.sessions().is_empty());
}

#[test]
fn a_row_of_the_wrong_shape_is_left_out_and_reported() {
    let mut source = with_tables();
    feed(&mut source, WORKING, "working-bad-rows.json");
    assert_eq!(
        source.sessions(),
        [working(BUSY_A, 1_793_750_400_000, &[]), problem()]
    );

    let mut source = with_tables();
    feed(&mut source, SCHEDULES, "schedules-bad-rows.json");
    assert_eq!(
        source.sessions(),
        [
            idle(
                "schedule:sch_0000000000a5AAAAAAAAAAAAAA",
                &[1_793_764_800_000]
            ),
            problem(),
        ]
    );
}

#[test]
fn a_table_that_goes_takes_its_rows_and_its_problem_with_it() {
    let mut source = with_tables();
    feed(&mut source, WORKING, "working.json");
    feed(&mut source, SCHEDULES, "error-prepare.json");
    assert_eq!(source.sessions().last(), Some(&problem()));
    feed(&mut source, TABLES, "tables-sessions.json");
    assert_eq!(
        source.sessions(),
        [
            working(BUSY_B, 1_793_750_460_000, &[]),
            working(BUSY_A, 1_793_750_400_000, &[]),
        ]
    );
    feed(&mut source, TABLES, "tables-none.json");
    assert!(source.sessions().is_empty());
}

#[test]
fn feeds_for_anything_else_change_nothing() {
    let mut source = with_tables();
    feed(&mut source, WORKING, "working.json");
    let before = source.sessions();
    let others = [
        FileId(db()),
        FileId::query(&db(), "message"),
        FileId::query(Path::new(r"X:\other\opencode.db"), WORKING.name),
    ];
    for other in &others {
        assert_eq!(
            source.feed(other, Feed::Whole(fixture("error-prepare.json"))),
            Fed::Taken
        );
        assert_eq!(source.feed(other, Feed::Gone), Fed::Taken);
    }
    for data in [Feed::Present, Feed::Appended(fixture("working-none.json"))] {
        assert_eq!(source.feed(&id(WORKING), data), Fed::Taken);
    }
    assert_eq!(source.sessions(), before);
    assert_eq!(wanted(&source), ["tables", "working", "schedules"]);
}

// Answers a real OpenCode 2.2.5 gave (`fixtures/opencode/live/README.md`, card `p4-live-opencode`).

const LIVE: [&str; 8] = [
    "schedules-after-fire.json",
    "schedules-two.json",
    "tables.json",
    "working-crash-and-newer.json",
    "working-none.json",
    "working-resumed.json",
    "working-schedule-fired.json",
    "working-tui-turn.json",
];

fn live_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("opencode")
        .join("live")
}

fn feed_live(source: &mut OpenCodeSource, sql: Sql, name: &str) {
    let bytes = fs::read(live_dir().join(name)).unwrap();
    assert_eq!(source.feed(&id(sql), Feed::Whole(bytes)), Fed::Taken);
}

#[test]
fn every_live_fixture_is_listed_once() {
    let mut on_disk: Vec<String> = fs::read_dir(live_dir())
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .filter(|name| name.ends_with(".json"))
        .collect();
    on_disk.sort();
    assert_eq!(on_disk, LIVE);
}

/// The live test's steps in order, each answer as the reader gave it: every one reads with no format problem.
#[test]
fn the_live_answers_give_the_facts_the_live_test_saw() {
    const TUI: &str = "ses_live0000000000000000000001";
    const SESSION: &str = "ses_live0000000000000000000004";
    const NEWER: &str = "ses_live0000000000000000000013";
    const IN_3_MIN: i64 = 1_793_970_487_652;
    const IN_2_H: i64 = 1_793_977_470_699;
    let mut source = OpenCodeSource::new(db());
    feed_live(&mut source, TABLES, "tables.json");
    assert_eq!(wanted(&source), ["tables", "working", "schedules"]);

    feed_live(&mut source, WORKING, "working-tui-turn.json");
    assert_eq!(source.sessions(), [working(TUI, 1_793_968_233_535, &[])]);
    feed_live(&mut source, WORKING, "working-none.json");
    assert!(
        source.sessions().is_empty(),
        "an idle session is not listed"
    );

    feed_live(&mut source, SCHEDULES, "schedules-two.json");
    assert_eq!(
        source.sessions(),
        [idle(SESSION, &[IN_3_MIN, IN_2_H])],
        "both timers on their session, which waits for any OpenCode"
    );

    feed_live(&mut source, WORKING, "working-schedule-fired.json");
    feed_live(&mut source, SCHEDULES, "schedules-after-fire.json");
    assert_eq!(
        source.sessions(),
        [working(SESSION, 1_793_970_487_664, &[IN_2_H])],
        "the fired one is gone, its turn runs"
    );

    feed_live(&mut source, WORKING, "working-crash-and-newer.json");
    assert_eq!(
        source.sessions(),
        [
            working(SESSION, 1_793_972_637_404, &[IN_2_H]),
            working(NEWER, 1_793_972_673_569, &[])
        ],
        "the crash left its row; each row keeps its own busy time, so the newer process counts only for its own"
    );

    feed_live(&mut source, WORKING, "working-resumed.json");
    assert_eq!(
        source.sessions(),
        [working(SESSION, 1_793_972_717_888, &[IN_2_H])],
        "the managed service resumed the claimed turn"
    );
}

// The table test (Haik, 2026-11-06): the file also holds OpenCode's login tokens, so no query may name a table but
// these.

const ALLOWED: [&str; 3] = ["sqlite_master", "session_v2", "schedule"];

#[derive(Clone, Debug, PartialEq, Eq)]
enum Token {
    /// A bare word, or a quoted name (`"x"`, `` `x` ``, `[x]`) with `true`.
    Word(String, bool),
    /// A `'...'` literal, which SQLite also takes as a name where only a name fits (`FROM 'account'`).
    Literal(String),
    Punct(char),
}

fn tokens(sql: &str) -> Vec<Token> {
    let mut out = Vec::new();
    let mut chars = sql.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            c if c.is_whitespace() => {}
            '-' if chars.next_if_eq(&'-').is_some() => {
                // A line comment.
                for c in chars.by_ref() {
                    if c == '\n' {
                        break;
                    }
                }
            }
            '/' if chars.next_if_eq(&'*').is_some() => {
                // A block comment.
                let mut star = false;
                for c in chars.by_ref() {
                    if star && c == '/' {
                        break;
                    }
                    star = c == '*';
                }
            }
            '\'' => {
                // `''` inside a literal is a quote.
                let mut text = String::new();
                while let Some(c) = chars.next() {
                    if c == '\'' && chars.next_if_eq(&'\'').is_none() {
                        break;
                    }
                    text.push(c);
                }
                out.push(Token::Literal(text));
            }
            '"' | '`' | '[' => {
                let close = if c == '[' { ']' } else { c };
                let name = chars.by_ref().take_while(|&n| n != close).collect();
                out.push(Token::Word(name, true));
            }
            c if c.is_alphanumeric() || c == '_' => {
                let mut word = String::from(c);
                while let Some(n) = chars.next_if(|n| n.is_alphanumeric() || *n == '_' || *n == '$')
                {
                    word.push(n);
                }
                out.push(Token::Word(word, false));
            }
            c => out.push(Token::Punct(c)),
        }
    }
    out
}

/// Words that may follow a table and are not its alias.
const NOT_ALIAS: &str = "WHERE JOIN INNER LEFT RIGHT FULL OUTER CROSS NATURAL ON USING GROUP ORDER LIMIT UNION \
                         EXCEPT INTERSECT WINDOW HAVING INDEXED NOT AS RETURNING";

/// Words that open a subquery where a table could stand.
const SUBQUERY: &str = "SELECT WITH VALUES";

fn is_word(token: Option<&Token>, word: &str) -> bool {
    matches!(token, Some(Token::Word(w, false)) if w.eq_ignore_ascii_case(word))
}

/// A token that can name a table: a word, a quoted name or a literal.
fn name_of(token: Option<&Token>) -> Option<(String, bool)> {
    match token {
        Some(Token::Word(name, quoted)) => Some((name.clone(), *quoted)),
        Some(Token::Literal(name)) => Some((name.clone(), true)),
        _ => None,
    }
}

/// Every table a query reads: each name after `FROM` or `JOIN` (also in parentheses, `FROM (x)`), after a comma that
/// follows one, and after `IN` with no parenthesis (`x IN account`), in subqueries too, comments skipped. A schema
/// prefix (`main.x`) is dropped; a table-valued function counts by its name.
fn tables_named(sql: &str) -> Vec<String> {
    let t = tokens(sql);
    let mut tables = Vec::new();
    let mut i = 0;
    while i < t.len() {
        let into_in = is_word(t.get(i), "IN") && t.get(i + 1) != Some(&Token::Punct('('));
        if is_word(t.get(i), "FROM") || is_word(t.get(i), "JOIN") || into_in {
            loop {
                i += 1;
                while t.get(i) == Some(&Token::Punct('(')) {
                    i += 1;
                }
                let Some((mut name, quoted)) = name_of(t.get(i)) else {
                    break;
                };
                if !quoted
                    && SUBQUERY
                        .split_whitespace()
                        .any(|k| name.eq_ignore_ascii_case(k))
                {
                    // A subquery: its own `FROM` comes next in the scan.
                    break;
                }
                if t.get(i + 1) == Some(&Token::Punct('.'))
                    && let Some((table, _)) = name_of(t.get(i + 2))
                {
                    name = table;
                    i += 2;
                }
                tables.push(name.to_ascii_lowercase());
                if is_word(t.get(i + 1), "AS") {
                    i += 2;
                } else if let Some(Token::Word(alias, quoted)) = t.get(i + 1)
                    && (*quoted
                        || !NOT_ALIAS
                            .split_whitespace()
                            .any(|k| alias.eq_ignore_ascii_case(k)))
                {
                    i += 1;
                }
                if t.get(i + 1) == Some(&Token::Punct(',')) {
                    i += 1;
                } else {
                    break;
                }
            }
        }
        i += 1;
    }
    tables
}

/// The tables a query names outside [`ALLOWED`].
fn foreign_tables(sql: &str) -> Vec<String> {
    tables_named(sql)
        .into_iter()
        .filter(|t| !ALLOWED.contains(&t.as_str()))
        .collect()
}

#[test]
fn every_query_names_only_the_session_and_schedule_tables() {
    // Every SQL string the source can emit: its wants in each state the probe can leave it in.
    let mut emitted = Vec::new();
    for tables in [
        None,
        Some("tables-none.json"),
        Some("tables-sessions.json"),
        Some("tables-both.json"),
    ] {
        let mut source = OpenCodeSource::new(db());
        if let Some(answer) = tables {
            feed(&mut source, TABLES, answer);
        }
        for want in source.wanted() {
            let WantedFile::Query { sql, .. } = want else {
                panic!("{want:?}");
            };
            if !emitted.contains(&sql) {
                emitted.push(sql);
            }
        }
    }
    let all: Vec<&str> = ALL_SQL.iter().map(|q| q.sql).collect();
    assert_eq!(emitted, all, "ALL_SQL is every query, each one emitted");

    for sql in emitted {
        assert!(sql.starts_with("SELECT "), "{sql}");
        assert!(!sql.contains(';'), "one statement: {sql}");
        assert!(
            !tables_named(sql).is_empty(),
            "the scan found its table: {sql}"
        );
        assert_eq!(foreign_tables(sql), Vec::<String>::new(), "{sql}");
    }
}

#[test]
fn the_table_scan_finds_every_table_a_query_names() {
    let cases: &[(&str, &[&str])] = &[
        ("SELECT a FROM schedule", &["schedule"]),
        (
            "SELECT * FROM session_v2 s JOIN message m ON m.session_id = s.id",
            &["session_v2", "message"],
        ),
        ("SELECT * FROM schedule, account", &["schedule", "account"]),
        (
            "SELECT * FROM schedule AS x, `credential` y",
            &["schedule", "credential"],
        ),
        ("SELECT * FROM main.account", &["account"]),
        (
            "SELECT (SELECT value FROM credential) FROM schedule",
            &["credential", "schedule"],
        ),
        (
            "SELECT * FROM (SELECT * FROM session) WHERE 1",
            &["session"],
        ),
        (
            "SELECT 'x FROM account', 'it''s' FROM schedule",
            &["schedule"],
        ),
        ("SELECT * FROM json_each('[]')", &["json_each"]),
        (
            "select * from schedule left join \"control_account\" on 1",
            &["schedule", "control_account"],
        ),
        ("SELECT * FROM [account]", &["account"]),
        // The review's holes (`docs/reviews/p4-opencode-db-read.md`, finding 3), and a literal as a name.
        ("SELECT * FROM (account)", &["account"]),
        (
            "SELECT id FROM session_v2 JOIN (account) ON 1",
            &["session_v2", "account"],
        ),
        ("SELECT * FROM/**/account", &["account"]),
        ("SELECT * FROM -- c\naccount", &["account"]),
        (
            "SELECT id FROM session_v2 WHERE id IN account",
            &["session_v2", "account"],
        ),
        ("SELECT * FROM 'account'", &["account"]),
        (
            "SELECT id FROM schedule WHERE id IN (SELECT id FROM credential)",
            &["schedule", "credential"],
        ),
        (
            "SELECT id FROM schedule WHERE id IN ('a', 'b')",
            &["schedule"],
        ),
        ("SELECT * FROM ((SELECT * FROM schedule))", &["schedule"]),
        ("SELECT 1 /* FROM account */ FROM schedule", &["schedule"]),
    ];
    for (sql, tables) in cases {
        assert_eq!(tables_named(sql), *tables, "{sql}");
    }
    assert_eq!(
        foreign_tables("SELECT id FROM session_v2 JOIN message ON 1"),
        ["message"]
    );
}

// Nothing read from the database may reach a log (Haik, 2026-11-06). The core logs `FormatHealth` texts and labels;
// `wakeshot-windows`' `sqlite` tests check the watch thread's log end to end.

const MARK: &str = "MARK3R";

/// Answers with the mark in every cell the source keeps or drops, and in an error answer's text.
fn marked_answers() -> Vec<(Sql, Vec<u8>)> {
    let row_ids = format!(
        r#"{{"rows":[["ses_{MARK}",1793750412345],["{MARK} prompt",1793750412345],["ses_x",["{MARK}"]]]}}"#
    );
    let schedules = format!(
        r#"{{"rows":[["sch_{MARK}","ses_{MARK}",1793750467890],["sch_y","{MARK} text",1]]}}"#
    );
    vec![
        (
            TABLES,
            br#"{"rows":[["schedule"],["session_v2"]]}"#.to_vec(),
        ),
        (WORKING, row_ids.into_bytes()),
        (SCHEDULES, schedules.into_bytes()),
        (
            SCHEDULES,
            format!(r#"{{"error":"prepare: no such column: {MARK}","detail":"{MARK}"}}"#)
                .into_bytes(),
        ),
        (
            SCHEDULES,
            format!(r#"{{"rows":[["sch_{MARK}",null,1793750467890]]}}"#).into_bytes(),
        ),
    ]
}

#[test]
fn nothing_from_an_answer_reaches_a_format_problem_or_the_core_log() {
    let mut source = OpenCodeSource::new(db());
    let mut seen = Vec::new();
    for (sql, bytes) in marked_answers() {
        source.feed(&id(sql), Feed::Whole(bytes));
        seen.push(source.sessions());
    }
    let formats: Vec<FormatHealth> = seen.iter().flatten().map(|s| s.format.clone()).collect();

    let now = Now {
        wall_ms: 1_793_750_500_000,
        mono_ms: 3_600_000,
    };
    let mut core = Core::new(Config::default());
    let mut commands = core.handle(
        now,
        Event::Started {
            config: Config::default(),
            journal: None,
            journal_unread: false,
            owner: true,
            on_console: true,
            tray: ProcessRef {
                pid: 1,
                start: FileTime(1),
            },
            first_run: false,
        },
    );
    for (n, sessions) in seen.into_iter().enumerate() {
        let later = Now {
            wall_ms: now.wall_ms + 61_000 * n as i64,
            mono_ms: now.mono_ms + 61_000 * n as u64,
        };
        let sessions = sessions.into_iter().map(|s| (s, Alive::Yes)).collect();
        commands.extend(core.handle(
            later,
            Event::Sources {
                source: SourceId::OpenCode,
                sessions,
            },
        ));
        commands.extend(core.handle(later, Event::Tick));
    }
    let lines: Vec<String> = commands
        .into_iter()
        .filter_map(|c| match c {
            Command::Log(text) => Some(text),
            _ => None,
        })
        .collect();
    for line in &lines {
        for leak in [MARK, "prompt", "no such column", "12345", "67890"] {
            assert!(!line.contains(leak), "{leak:?} in {line:?}");
        }
    }
    assert!(
        lines.iter().any(|l| l.contains("cannot read opencode.db")),
        "{lines:?}"
    );
    // The only texts a format problem carries are fixed.
    for format in &formats {
        assert!(
            matches!(
                format,
                FormatHealth::Ok | FormatHealth::Unreadable(UNREADABLE)
            ),
            "{format:?}"
        );
    }
    assert!(
        formats.iter().any(|f| *f != FormatHealth::Ok),
        "a problem was seen"
    );
}
