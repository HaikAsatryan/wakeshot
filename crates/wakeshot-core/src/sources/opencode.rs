//! OpenCode 2.x sessions (architecture section 7): rows of `opencode.db` read through `WantedFile::Query`, alive
//! through `Liveness::ImageSince`. Built from `@opencode/cli` 2.2.5's own source (`docs/research/opencode-signals.md`)
//! and tested on a live 2.2.5.
//!
//! What it reads, in `<data>\opencode\opencode.db` ([`opencode_db`]), and nothing else in that file:
//!
//! - `sqlite_master`: which of `session_v2` and `schedule` exist ([`TABLES`]), so a file that only OpenCode 1.x wrote
//!   runs no query that would fail.
//! - `session_v2`: a session works while `time_busy` is newer than `time_idle` ([`WORKING`]). `time_suspended` is not
//!   read: 2.2.5 sets it while a turn runs. A turn cut by a shutdown keeps `time_busy` newer, so such a row counts
//!   only while an OpenCode process that started before it lives.
//! - `schedule`: active rows with their next due time ([`SCHEDULES`]); only a running OpenCode fires them.
//!
//! The same file holds OpenCode's login tokens (tables `account`, `control_account`, `credential`). No query names
//! those tables, and `tests/opencode_source.rs` fails when one does. 1.x is not read: its state is in `message` rows,
//! which hold prompt text.
//!
//! A permission prompt lives in OpenCode's memory only, so a session waiting on one reads as working. `job` rows are
//! not read: one can stay `running` after its job ended.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::sources::{
    Activity, AgentSource, Fed, Feed, FileId, FormatHealth, Liveness, Note, SessionFacts,
    SessionKey, SourceId, WantedFile,
};

/// What the tooltip names an OpenCode session.
pub const LABEL: &str = "OpenCode";

/// The processes that run sessions: the npm package links the native `opencode.exe` (no `node.exe` in between), and
/// the desktop app's sidecar is `opencode-cli.exe`.
pub const IMAGES: &[&str] = &["opencode.exe", "opencode-cli.exe"];

/// What `FormatHealth::Unreadable` names: a fixed word, so nothing from the file reaches a log.
pub const UNREADABLE: &str = "opencode.db";

/// The key of the one entry that carries a read problem, as no session can be named then. Session ids start with
/// `ses` and schedule keys with `schedule:`, so it never meets a real key.
pub const PROBLEM_KEY: &str = "opencode.db";

/// The [`Note`] of a database whose tables answer read but named no `session_v2`: an OpenCode 1.x file, or a 2.x one
/// whose table got a new name. No notice: an update of Wakeshot mends neither for sure.
pub const NO_SESSION_TABLE: &str = "its database has no session_v2 table, so no session is read";

/// The longest id taken from a row; OpenCode makes ids of 30 characters.
const MAX_ID: usize = 128;

/// One fixed query: its name (the `FileId::query` suffix) and its SQL.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sql {
    pub name: &'static str,
    pub sql: &'static str,
}

/// Which of the two tables exist. Rows: `[name]`.
pub const TABLES: Sql = Sql {
    name: "tables",
    sql: "SELECT name FROM sqlite_master WHERE type = 'table' AND name IN ('session_v2', 'schedule')",
};

/// The sessions whose turn runs. Rows: `[id, time_busy]`, `time_busy` in wall ms.
pub const WORKING: Sql = Sql {
    name: "working",
    sql: "SELECT id, time_busy FROM session_v2 WHERE time_busy IS NOT NULL \
          AND (time_idle IS NULL OR time_busy > time_idle)",
};

/// The active schedules. Rows: `[id, session_id or null, due]`, due in wall ms. A row fires once `next_due_at` and a
/// set `retry_at` have both passed (`core/src/schedule.ts:478-486`), so the later of the two is its due time.
pub const SCHEDULES: Sql = Sql {
    name: "schedules",
    sql: "SELECT id, session_id, max(next_due_at, coalesce(retry_at, next_due_at)) FROM schedule \
          WHERE status = 'active' AND next_due_at IS NOT NULL",
};

/// Every query the source can ask for.
pub const ALL_SQL: [Sql; 3] = [TABLES, WORKING, SCHEDULES];

/// OpenCode's database: `<data>\opencode\opencode.db`, where `<data>` is `xdg_data_home` (the value of
/// `XDG_DATA_HOME`) when set and not empty, else `<profile>\.local\share`, on Windows too (`xdg-basedir` 5.1.0).
/// `OPENCODE_DB` and the non-default channels' files are not followed. The caller reads both values, so the core
/// stays free of environment reads.
pub fn opencode_db(profile: &Path, xdg_data_home: Option<&OsStr>) -> PathBuf {
    let data = match xdg_data_home {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => profile.join(".local").join("share"),
    };
    data.join("opencode").join("opencode.db")
}

/// OpenCode's sessions and schedules, keyed by OpenCode's own ids.
///
/// The watch thread is expected to feed each query's answer whole, under `FileId::query(db, name)`, and `Gone` once
/// the database closed.
#[derive(Debug)]
pub struct OpenCodeSource {
    db: PathBuf,
    found: Found,
    /// The last tables answer read whole, so `found` is what the database holds, not a default.
    probed: bool,
    working: Vec<(String, i64)>,
    schedules: Vec<Schedule>,
    /// The queries whose last answer was an error or held a row of the wrong shape.
    unread: BTreeSet<Query>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Query {
    Tables,
    Working,
    Schedules,
}

impl Query {
    const ALL: [Self; 3] = [Self::Tables, Self::Working, Self::Schedules];

    fn sql(self) -> Sql {
        match self {
            Self::Tables => TABLES,
            Self::Working => WORKING,
            Self::Schedules => SCHEDULES,
        }
    }
}

/// The tables the last probe found; only their queries are asked for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Found {
    sessions: bool,
    schedules: bool,
}

#[derive(Debug)]
struct Schedule {
    id: String,
    session: Option<String>,
    due: i64,
}

#[derive(Debug, Default)]
struct Entry {
    busy_since: Option<i64>,
    timers: Vec<i64>,
}

impl OpenCodeSource {
    /// `db` is OpenCode's database ([`opencode_db`]).
    pub fn new(db: PathBuf) -> Self {
        Self {
            db,
            found: Found::default(),
            probed: false,
            working: Vec::new(),
            schedules: Vec::new(),
            unread: BTreeSet::new(),
        }
    }

    fn want(&self, query: Query) -> WantedFile {
        let sql = query.sql();
        WantedFile::Query {
            db: self.db.clone(),
            name: sql.name,
            sql: sql.sql,
        }
    }

    /// Which query an answer is for.
    fn query_of(&self, file: &FileId) -> Option<Query> {
        Query::ALL
            .into_iter()
            .find(|query| *file == FileId::query(&self.db, query.sql().name))
    }

    fn take(&mut self, query: Query, bytes: &[u8]) {
        let whole = match query {
            Query::Tables => {
                let (names, whole) = rows(bytes, table_row);
                self.set_found(Found {
                    sessions: names.contains(&"session_v2"),
                    schedules: names.contains(&"schedule"),
                });
                self.probed = whole;
                whole
            }
            Query::Working => {
                let (working, whole) = rows(bytes, working_row);
                self.working = working;
                whole
            }
            Query::Schedules => {
                let (schedules, whole) = rows(bytes, schedule_row);
                self.schedules = schedules;
                whole
            }
        };
        if whole || no_sqlite(bytes) {
            self.unread.remove(&query);
        } else {
            self.unread.insert(query);
        }
    }

    /// A table that went takes its query's rows and problem with it.
    fn set_found(&mut self, found: Found) {
        if !found.sessions {
            self.forget(Query::Working);
        }
        if !found.schedules {
            self.forget(Query::Schedules);
        }
        self.found = found;
    }

    fn forget(&mut self, query: Query) {
        match query {
            Query::Tables => {
                self.set_found(Found::default());
                self.probed = false;
            }
            Query::Working => self.working.clear(),
            Query::Schedules => self.schedules.clear(),
        }
        self.unread.remove(&query);
    }

    fn facts(id: String, entry: Entry) -> SessionFacts {
        let Entry {
            busy_since,
            mut timers,
        } = entry;
        timers.sort_unstable();
        timers.dedup();
        SessionFacts {
            key: SessionKey {
                source: SourceId::OpenCode,
                id,
            },
            liveness: Liveness::ImageSince {
                images: IMAGES,
                since_wall_ms: busy_since,
            },
            activity: if busy_since.is_some() {
                Activity::Working
            } else {
                Activity::Idle
            },
            timers,
            open_tasks: 0,
            label: LABEL,
            format: FormatHealth::Ok,
        }
    }
}

/// The rows `row` reads, and whether the answer was rows that all read. An error answer, or bytes that are not the
/// contract's JSON, give no rows.
fn rows<T>(bytes: &[u8], row: fn(&[Value]) -> Option<T>) -> (Vec<T>, bool) {
    let Ok(answer) = serde_json::from_slice::<Value>(bytes) else {
        return (Vec::new(), false);
    };
    let Some(list) = answer.get("rows").and_then(Value::as_array) else {
        return (Vec::new(), false);
    };
    let mut whole = true;
    let taken = list
        .iter()
        .filter_map(|cells| {
            let read = cells.as_array().and_then(|cells| row(cells));
            whole &= read.is_some();
            read
        })
        .collect();
    (taken, whole)
}

/// The answer of a Windows with no usable `winsqlite3.dll`. Updating Wakeshot cannot mend that, so it gives no signal
/// and no "update Wakeshot" notice.
fn no_sqlite(bytes: &[u8]) -> bool {
    serde_json::from_slice::<Value>(bytes)
        .is_ok_and(|answer| answer.get("error").and_then(Value::as_str) == Some("no_sqlite"))
}

fn table_row(cells: &[Value]) -> Option<&'static str> {
    match cells {
        [Value::String(name)] if name == "session_v2" => Some("session_v2"),
        [Value::String(name)] if name == "schedule" => Some("schedule"),
        _ => None,
    }
}

fn working_row(cells: &[Value]) -> Option<(String, i64)> {
    match cells {
        [Value::String(id), Value::Number(busy)] => Some((id_of(id, "ses")?, busy.as_i64()?)),
        _ => None,
    }
}

fn schedule_row(cells: &[Value]) -> Option<Schedule> {
    let [Value::String(id), session, Value::Number(due)] = cells else {
        return None;
    };
    let session = match session {
        Value::Null => None,
        Value::String(session) => Some(id_of(session, "ses")?),
        _ => return None,
    };
    Some(Schedule {
        id: id_of(id, "sch_")?,
        session,
        due: due.as_i64()?,
    })
}

/// An id as OpenCode makes it (`<prefix>` and 26 letters and digits), loosely: a key must never carry other text.
fn id_of(text: &str, prefix: &str) -> Option<String> {
    let ok = text.starts_with(prefix)
        && text.len() <= MAX_ID
        && text
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.' | b'+'));
    ok.then(|| text.to_owned())
}

impl AgentSource for OpenCodeSource {
    fn id(&self) -> SourceId {
        SourceId::OpenCode
    }

    fn wanted(&self) -> Vec<WantedFile> {
        let mut wanted = vec![self.want(Query::Tables)];
        if self.found.sessions {
            wanted.push(self.want(Query::Working));
        }
        if self.found.schedules {
            wanted.push(self.want(Query::Schedules));
        }
        wanted
    }

    fn feed(&mut self, file: &FileId, data: Feed) -> Fed {
        let Some(query) = self.query_of(file) else {
            return Fed::Taken;
        };
        match data {
            Feed::Whole(bytes) => self.take(query, &bytes),
            Feed::Gone => self.forget(query),
            // Never asked for: a query is answered whole.
            Feed::Appended(_) | Feed::Present => {}
        }
        // An answer is one read of a consistent snapshot, so reading it again 200 ms later cannot mend it.
        Fed::Taken
    }

    fn sessions(&self) -> Vec<SessionFacts> {
        let mut entries: BTreeMap<String, Entry> = BTreeMap::new();
        for (id, busy) in &self.working {
            let since = &mut entries.entry(id.clone()).or_default().busy_since;
            *since = Some(since.map_or(*busy, |s| s.max(*busy)));
        }
        for schedule in &self.schedules {
            let key = match &schedule.session {
                Some(session) => session.clone(),
                None => format!("schedule:{}", schedule.id),
            };
            entries.entry(key).or_default().timers.push(schedule.due);
        }
        let mut sessions: Vec<SessionFacts> = entries
            .into_iter()
            .map(|(id, entry)| Self::facts(id, entry))
            .collect();
        if !self.unread.is_empty() {
            sessions.push(SessionFacts {
                format: FormatHealth::Unreadable(UNREADABLE),
                ..Self::facts(PROBLEM_KEY.to_owned(), Entry::default())
            });
        }
        sessions
    }

    fn notes(&self) -> Vec<Note> {
        if self.probed && !self.found.sessions {
            vec![Note {
                session: None,
                text: NO_SESSION_TABLE,
                field: None,
            }]
        } else {
            Vec::new()
        }
    }
}
