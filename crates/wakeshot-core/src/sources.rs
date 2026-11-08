//! Agent sources: bytes in, facts out, no I/O (architecture sections 2 and 7). The watch thread feeds them; the core
//! turns their facts into holds, so parsers never decide policy.
//!
//! Callers may rely on:
//! - `wanted` changes only after a `feed`, and `sessions` is a full snapshot (posted as `Event::Sources`).
//! - A [`SessionKey`] is stable for the life of its session and never holds prompt text.
//! - [`FormatHealth::Unreadable`] means "no signal for this session" (the watch thread already read it again once,
//!   200 ms later); [`FormatHealth::UnknownValue`] means "counts as working while the process lives". Their texts are
//!   field names and values, never file content.
//! - Every name filter (`accept`, `matches`) runs on bare names before any open, so a source can keep files it must
//!   never read (Claude's `.key` files) from ever being opened.
//! - [`Note`]s, like `sessions`, are a full set as of the last `feed`. They feed the log only: no hold, notice or
//!   [`FormatHealth`] depends on them.

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

/// Which agent a source reads. Sources are compiled in (decision 14), so a closed list is enough. `VsCode` is the
/// chat panel's own sessions (Copilot and any other chat participant in it).
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

/// A session's identity across snapshots, so the core can keep per-session state (first seen waiting, timers). `id`
/// is the source's own id; for Claude, the pid as text, since `sessionId` changes at `/clear`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SessionKey {
    pub source: SourceId,
    pub id: String,
}

/// How the watch thread checks that a session still runs. The parser only describes it, so it stays free of I/O.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Liveness {
    Process {
        pid: u32,
        start: FileTime,
    },
    /// A process that holds `lock_file` open (Restart Manager) and runs `image`, or whose pid the lock file's name
    /// gives (a whole-number part, Copilot's `inuse.<pid>.hold`) whatever its image (VS Code's
    /// `copilot-runtime.exe`). If Restart Manager fails: the process the name gives while it runs; a name with no pid
    /// (Codex) counts any live process with `image` while `lock_file` exists, which can outlive its session
    /// (architecture section 7).
    AnyProcess {
        image: &'static str,
        lock_file: PathBuf,
    },
    /// Alive while some process running one of `images` (exe names, ASCII case ignored) started at or before
    /// `since_wall_ms` plus [`IMAGE_SINCE_SLACK_MS`]; `None` means any start. For files that hold no pid, only a time
    /// the agent wrote: an OpenCode row's time, or a VS Code chat request's `timestamp` (rule 7 of
    /// `research/copilot-vscode-signals.md`: a request is a crash leftover once every `Code.exe` started after it).
    /// The watch thread checks it in the system process list, which needs no rights, and waits on each match's exit.
    ImageSince {
        images: &'static [&'static str],
        since_wall_ms: Option<i64>,
    },
}

/// How much later than `since_wall_ms` a process of `Liveness::ImageSince` may have started and still count: room for
/// a time the agent rounded or wrote just before its process start was stamped.
pub const IMAGE_SINCE_SLACK_MS: i64 = 2_000;

/// What a session is doing, as the agent's own files say. The core applies the waiting hour and the Remote Control
/// rule. `since_wall_ms` is `None` when the file has no time for it: the parser has no clock, so the core uses the
/// time it first saw the session waiting.
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
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FormatHealth {
    Ok,
    Unreadable(&'static str),
    UnknownValue(&'static str, String),
}

/// One session as the source sees it, a full snapshot entry. `timers` are due times in wall ms without any margin
/// (the core adds it). `label` is what the tooltip names, such as "Claude Code".
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

/// Why a source, or one of its sessions, may not hold or is not read, when no notice says so: a state Wakeshot cannot
/// tell from a normal one, where "update Wakeshot" would be wrong as often as right, or a format problem told only
/// while the session works. The watch logs each once while it lasts (architecture section 8). Every part is a fixed
/// text or an id, so no file content can reach the log.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Note {
    /// The session's own id, as in its [`SessionKey`]; `None` for the whole source. The log shows only a short form,
    /// since an id can be a file name.
    pub session: Option<String>,
    pub text: &'static str,
    /// The field the text is about, such as `cron`, shown after it.
    pub field: Option<&'static str>,
}

/// The [`Note`] text of a format problem a source tells as `UnknownValue` only while the session works, since a
/// session that does not work must not hold for it (the core counts `UnknownValue` as working). Noted while the
/// problem stands, working or not, so the log names it once per session, not at each turn.
pub const KEPT_BACK: &str = "an unknown value is ignored while the session is not working";

/// A file the watch thread feeds to a source, named by its full path.
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

/// What a source asks the watch thread to feed it. A tail (`Tail`, `FindTail`, `FindNewest`) is read on each change in
/// a folder of the same source and on the safety tick, not watched (architecture section 7).
#[derive(Clone, Debug)]
pub enum WantedFile {
    Folder {
        dir: PathBuf,
        accept: fn(&str) -> bool,
    },
    /// Files listed, never opened (Codex and Copilot lock files), fed as `Present` or `Gone`. `depth` 0 is the files in
    /// `dir`, 1 the files in its direct subfolders.
    Names {
        dir: PathBuf,
        depth: u8,
        accept: fn(&str) -> bool,
    },
    Tail(PathBuf),
    /// `root\<depth folder levels>\*<name_suffix>`, tailed like `Tail`, for a file whose folder is not known (Claude
    /// transcripts); feeds carry the found path. The file found first stays the one tailed while it is still there.
    FindTail {
        root: PathBuf,
        depth: u8,
        name_suffix: String,
    },
    /// Of the files `root\<depth folder levels>\<name>` with `matches(name, key)`, the newest by modified time is
    /// tailed. When another becomes the newest, the old one is fed `Gone`, then the new one from its start: for a
    /// session that moves to a new file (a reverted Codex thread). The folders are listed again on each read, at most
    /// once per call for all wants under one root.
    FindNewest {
        root: PathBuf,
        depth: u8,
        key: String,
        matches: fn(&str, &str) -> bool,
    },
    /// The files `root\<depth folder levels>\<sub>\<name>` (no `sub` level when `None`) whose name `accept` passes,
    /// fed `Whole` when they changed since the watch thread started or within the last 24 hours, again on each change,
    /// and `Gone` when deleted. An older file is never opened, but one a change notice names is read: the change is
    /// new. For files rewritten in place (VS Code chat sessions, which compaction rewrites).
    Recent {
        root: PathBuf,
        depth: u8,
        sub: Option<&'static str>,
        accept: fn(&str) -> bool,
    },
    /// `sql` run read only on the SQLite database `db` (OpenCode's `opencode.db`), fed as `Whole` under
    /// [`FileId::query`] with JSON `{"rows":[[...],...]}` (each cell an integer, text or null) or
    /// `{"error":"<kind>"}` (a fixed word such as `open`, `prepare` or `no_sqlite`, never SQLite's own text).
    ///
    /// It runs only while `<db>-wal` exists: on a change of the db or its `-wal`, and about every 2 s while `<db>`
    /// exists and on the safety tick when the first 48 bytes of `<db>-shm` (the WAL-index header, which every commit
    /// changes) differ from those after its last read; at most once a second for each want. The `-shm` look is there
    /// because a commit gives no change notice (OpenCode keeps its `-wal` open at one size) and the agent leaves its
    /// files after it stops (live test 2026-11-06). `Gone` once `<db>` or `<db>-wal` is missing after an answer; a
    /// busy database gives no feed, so the last answer stands.
    Query {
        db: PathBuf,
        name: &'static str,
        sql: &'static str,
    },
}

/// One read of a wanted file. `Whole` for folder files, recent files and query answers, `Appended` for tails (only the
/// bytes since the last feed), `Present` for a listed name (`WantedFile::Names`), `Gone` when the file was deleted (for
/// a query: the database or its `-wal` went), so a source can drop the session.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Feed {
    Whole(Vec<u8>),
    Appended(Vec<u8>),
    Present,
    Gone,
}

/// A source's answer to one feed. `RetryOnce`: the file did not parse; the watch thread reads it again 200 ms later,
/// once, before the source reports it `Unreadable` (architecture section 7).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fed {
    Taken,
    RetryOnce,
}

/// One agent's reader, so every source is tested with fixture files and a fake clock. The watch thread drives it and
/// owns the sources as `Box<dyn AgentSource + Send>`.
pub trait AgentSource {
    fn id(&self) -> SourceId;
    fn wanted(&self) -> Vec<WantedFile>;
    fn feed(&mut self, file: &FileId, data: Feed) -> Fed;
    fn sessions(&self) -> Vec<SessionFacts>;
    /// Most sources have nothing to say that their sessions do not.
    fn notes(&self) -> Vec<Note> {
        Vec::new()
    }
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
