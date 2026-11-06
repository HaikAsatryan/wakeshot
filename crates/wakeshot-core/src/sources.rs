//! Agent sources: bytes in, facts out, no I/O (architecture sections 2 and 7). The watch thread feeds them; the core
//! turns their facts into holds, so parsers never decide policy.
//!
//! Owner: contracts card. Each type's own doc names the track that owns its meaning.

pub mod antigravity;
pub mod claude;
pub mod codex;
pub mod copilot;
pub mod cron;
pub mod cursor;
pub mod hooked;
pub mod opencode;
pub mod vscode;

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::process::FileTime;

/// Which agent a source reads. Sources are compiled in (decision 14), so a closed list is enough.
///
/// Owner: sources track (Phase 4) adds a variant with each source. `VsCode` is the chat panel's own sessions
/// (Copilot and any other chat participant in it).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SourceId {
    ClaudeCode,
    Codex,
    Copilot,
    OpenCode,
    VsCode,
    Cursor,
    Antigravity,
}

/// A session's identity across snapshots, so the core can keep per-session state (first seen waiting, timers).
///
/// Owner: sources track (Phase 4). `id` is the source's own id; for Claude, the pid as text, since `sessionId`
/// changes at `/clear`. The core may rely on it being stable for the life of the session and never holding prompt
/// text.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SessionKey {
    pub source: SourceId,
    pub id: String,
}

/// How the adapter can check that a session is still running. The parser only describes it; the watch thread checks
/// it, so a parser stays free of I/O.
///
/// Owner: sources track (Phase 4). `AnyProcess`: a process running `image` that holds `lock_file` open (Restart
/// Manager), else any live process with `image` while `lock_file` exists; that fallback can outlive its session
/// (architecture section 7).
///
/// `ImageSince`: alive while some process running one of `images` (exe names, ASCII case ignored) started at or
/// before `since_wall_ms` plus [`IMAGE_SINCE_SLACK_MS`]; `None` means any start. For files that hold no pid, only a
/// time the agent wrote: an OpenCode row's time, or a VS Code chat request's `timestamp` (rule 7 of
/// `research/copilot-vscode-signals.md`: a request is a crash leftover once every `Code.exe` started after it). The
/// watch thread checks it in the system process list, which needs no rights, and waits on each match's exit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Liveness {
    Process {
        pid: u32,
        start: FileTime,
    },
    AnyProcess {
        image: &'static str,
        lock_file: PathBuf,
    },
    ImageSince {
        images: &'static [&'static str],
        since_wall_ms: Option<i64>,
    },
}

/// How much later than `since_wall_ms` a process of `Liveness::ImageSince` may have started and still count: room for
/// a time the agent rounded or wrote just before its process start was stamped.
pub const IMAGE_SINCE_SLACK_MS: i64 = 2_000;

/// What a session is doing, as the agent's own files say.
///
/// Owner: sources track (Phase 4) reports it; the core track (Phase 2) applies the waiting hour and the Remote
/// Control rule. `since_wall_ms` is `None` when the file has no time for it: the parser has no clock, so the core
/// uses the time it first saw the session waiting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Activity {
    Working,
    Waiting {
        since_wall_ms: Option<i64>,
        remote: bool,
    },
    Idle,
}

/// Whether the source could read the agent's format, so a format change shows "update Wakeshot" instead of a wrong
/// answer.
///
/// Owner: sources track (Phase 4). The core may rely on `Unreadable` meaning "no signal for this session" (already
/// retried once 200 ms later by the watch thread) and `UnknownValue` meaning "counts as working while the process
/// lives" (`CLAUDE.md`). The texts are field names and values, never file content.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FormatHealth {
    Ok,
    Unreadable(&'static str),
    UnknownValue(&'static str, String),
}

/// One session as the source sees it, a full snapshot entry.
///
/// Owner: sources track (Phase 4); the core track (Phase 2) turns it into holds and deadlines. `timers` are due
/// times in wall ms without any margin (the core adds it). `label` is what the tooltip names, such as "Claude Code".
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionFacts {
    pub key: SessionKey,
    pub liveness: Liveness,
    pub activity: Activity,
    pub timers: Vec<i64>,
    pub open_tasks: u32,
    pub label: &'static str,
    pub format: FormatHealth,
}

/// A file the watch thread feeds to a source, named by its full path.
///
/// Owner: sources track (Phase 4).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FileId(pub PathBuf);

impl FileId {
    /// The id the answers of a `WantedFile::Query` come under: `<db>?<name>`. `?` cannot be in a Windows file name, so
    /// it never names a real file.
    pub fn query(db: &Path, name: &str) -> Self {
        let mut id = OsString::from(db.as_os_str());
        id.push("?");
        id.push(name);
        Self(PathBuf::from(id))
    }
}

/// What a source asks the watch thread to feed it.
///
/// Owner: sources track (Phase 4), both sides. Every name filter (`accept`, `matches`) runs on bare names before any
/// open, so a source can keep files it must never read (Claude's `.key` files) from ever being opened. A tail
/// (`Tail`, `FindTail`, `FindNewest`) is read on each change in a folder of the same source and on the safety tick,
/// not watched (architecture section 7).
///
/// - `Names`: files are listed, never opened (Codex and Copilot lock files), and fed as `Present` or `Gone`. `depth`
///   0 is the files in `dir`, 1 the files in its direct subfolders.
/// - `FindTail`: the watch thread finds `root\<depth folder levels>\*<name_suffix>` and tails it like `Tail`, for a
///   file whose folder is not known (Claude transcripts); feeds carry the found path. The file found first stays the
///   one tailed while it is still there.
/// - `FindNewest`: the files `root\<depth folder levels>\<name>` with `matches(name, key)`, and of these the newest by
///   modified time is tailed. When another becomes the newest, the old one is fed `Gone`, then the new one from its
///   start. For a session that moves to a new file (a reverted Codex thread). The folders are listed again on each
///   read, at most once per call for all wants under one root.
/// - `Recent`: the files `root\<depth folder levels>\<sub>\<name>` (no `sub` level when `None`) whose name `accept`
///   passes, fed `Whole` when they changed since the watch thread started or within the last 24 hours, again on each
///   change, and `Gone` when deleted. An older file is never opened. For files rewritten in place (VS Code chat
///   sessions, which compaction rewrites). A file a change notice names is read: the change is new.
/// - `Query`: `sql` run read only on the SQLite database `db`, fed as `Whole` under [`FileId::query`] with JSON
///   `{"rows":[[...],...]}` (each cell an integer, text or null) or `{"error":"<kind>"}` (a fixed word such as `open`,
///   `prepare` or `no_sqlite`, never SQLite's own text). It runs only while `<db>-wal` exists (the agent has the
///   database open), on a change of the db's files in its folder (at most once a second for each want) and on the
///   safety tick. `Gone` once `<db>` or `<db>-wal` is missing after an answer; a busy database gives no feed, so the
///   last answer stands. For OpenCode's `opencode.db`.
#[derive(Clone, Debug)]
pub enum WantedFile {
    Folder {
        dir: PathBuf,
        accept: fn(&str) -> bool,
    },
    Names {
        dir: PathBuf,
        depth: u8,
        accept: fn(&str) -> bool,
    },
    Tail(PathBuf),
    FindTail {
        root: PathBuf,
        depth: u8,
        name_suffix: String,
    },
    FindNewest {
        root: PathBuf,
        depth: u8,
        key: String,
        matches: fn(&str, &str) -> bool,
    },
    Recent {
        root: PathBuf,
        depth: u8,
        sub: Option<&'static str>,
        accept: fn(&str) -> bool,
    },
    Query {
        db: PathBuf,
        name: &'static str,
        sql: &'static str,
    },
}

/// One read of a wanted file. `Whole` for folder files, recent files and query answers, `Appended` for tails (only the
/// bytes since the last feed), `Present` for a listed name (`WantedFile::Names`), `Gone` when the file was deleted (for
/// a query: the database or its `-wal` went), so a source can drop the session.
///
/// Owner: sources track (Phase 4), both sides.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Feed {
    Whole(Vec<u8>),
    Appended(Vec<u8>),
    Present,
    Gone,
}

/// A source's answer to one feed. `RetryOnce`: the file did not parse; the watch thread reads it again 200 ms later,
/// once, before the source reports it `Unreadable` (architecture section 7).
///
/// Owner: sources track (Phase 4), both sides.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fed {
    Taken,
    RetryOnce,
}

/// One agent's reader. Bytes in, facts out, no I/O, so every source is tested with fixture files and a fake clock.
///
/// Owner: sources track (Phase 4) implements it; the watch thread (also Phase 4) drives it and owns the sources as
/// `Box<dyn AgentSource + Send>`. The watch thread may rely on `wanted` changing only after a `feed`, and on
/// `sessions` being a full snapshot it posts as `Event::Sources`.
pub trait AgentSource {
    fn id(&self) -> SourceId;
    fn wanted(&self) -> Vec<WantedFile>;
    fn feed(&mut self, file: &FileId, data: Feed) -> Fed;
    fn sessions(&self) -> Vec<SessionFacts>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_query_id_names_its_db_and_query_and_no_real_file() {
        let db = Path::new(r"C:\Users\x\.local\share\opencode\opencode.db");
        let id = FileId::query(db, "tables");
        assert_eq!(
            id.0,
            PathBuf::from(r"C:\Users\x\.local\share\opencode\opencode.db?tables")
        );
        assert_eq!(id.0.parent(), db.parent(), "the db's folder");
        assert_ne!(id, FileId::query(db, "v2"));
        assert_ne!(id, FileId(db.to_path_buf()));
    }
}
