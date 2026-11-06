//! Codex CLI sessions (architecture section 7): one per thread lock file, working while the thread's rollout says a
//! turn is open. Built from Codex 0.167.0's own source (`docs/tasks/p4-codex-source.md`, Log); not tested live until
//! `p4-live-codex`.
//!
//! What it reads, under Codex's home folder ([`codex_home`]):
//!
//! - `thread-writer-locks\<thread id>.lock`: a Codex process holds one open, with an exclusive lock, for each thread it
//!   has loaded, and deletes it when done. Listed by name only, never opened: Codex reads a second holder as "this
//!   thread already has an active writer" and fails to open the thread. The `.coordination.lock` next to them is not
//!   a thread.
//! - `sessions\YYYY\MM\DD\rollout-<local time>-<thread id>.jsonl`, the thread's rollout: one JSON object per line,
//!   made at the thread's first persisted item, so a thread that has not run a turn yet has none. The last turn
//!   event decides: `task_started` (or `turn_started`) is a turn running, `task_complete` (or `turn_complete`) and
//!   `turn_aborted` end it. Approval prompts are not written, so a turn that waits for the user reads as working
//!   (Haik, 2026-11-03, sign-off item 1: it holds with no limit).
//!
//! Not followed yet: a thread reverted in the TUI (editing an earlier prompt of a paginated thread, the TUI's default
//! in 0.167.0) goes on in `rollout-<time>-<thread id>_<rollout id>.jsonl`, which `FindTail`'s name suffix cannot
//! name; its later turns are not seen (card Result, open item). Compressed `.jsonl.zst` rollouts are only ever old
//! ones without a live writer, so they are never asked for.
//!
//! Owner: `p4-codex-source`; fixes by `p4-live-codex`.

use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::sources::{
    Activity, AgentSource, Fed, Feed, FileId, FormatHealth, Liveness, SessionFacts, SessionKey,
    SourceId, WantedFile,
};

/// What the tooltip names a Codex session.
pub const LABEL: &str = "Codex";

/// The process that holds a thread's lock file: the npm package's `codex.js` starts the native `codex.exe` and
/// waits, and the shared app-server daemon is the same exe.
pub const IMAGE: &str = "codex.exe";

/// What `FormatHealth::UnknownValue` names: a rollout line whose turn event this version cannot read.
pub const ROLLOUT: &str = "rollout";

const LOCKS_DIR: &str = "thread-writer-locks";
const SESSIONS_DIR: &str = "sessions";

/// `sessions\YYYY\MM\DD`.
const DATE_DEPTH: u8 = 3;

/// A thread id as Codex writes it: a UUID, lowercase, with hyphens.
const ID_LEN: usize = 36;

/// Every turn event is an `event_msg` line, so the others are never parsed.
const MARKER: &[u8] = b"event_msg";

/// A longer line is skipped: no turn event comes near it, and the buffer must stay bounded.
const MAX_LINE: usize = 32 << 20;

/// Codex's home folder: `codex_home_env` (the value of `CODEX_HOME`) when set, else `<profile>\.codex`. Codex 0.167.0
/// treats an empty value as unset (`utils/home-dir/src/lib.rs`). The caller reads both values, so the core stays free
/// of environment reads.
pub fn codex_home(profile: &Path, codex_home_env: Option<&OsStr>) -> PathBuf {
    match codex_home_env {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => profile.join(".codex"),
    }
}

/// A thread lock file name: `<thread id>.lock`. The name filter the watch thread runs on the lock folder.
pub fn is_lock_name(name: &str) -> bool {
    name.strip_suffix(".lock").is_some_and(is_thread_id)
}

fn is_thread_id(text: &str) -> bool {
    text.len() == ID_LEN
        && text.bytes().enumerate().all(|(i, b)| match i {
            8 | 13 | 18 | 23 => b == b'-',
            _ => b.is_ascii_hexdigit(),
        })
}

/// Codex CLI's sessions, one per thread lock file, keyed by thread id.
///
/// The watch thread is expected to feed the lock folder's names as `Present` and `Gone`, and each `FindTail` from
/// the start of the file it finds, as it does for Claude transcripts.
#[derive(Debug)]
pub struct CodexSource {
    locks_dir: PathBuf,
    sessions_dir: PathBuf,
    threads: BTreeMap<String, Thread>,
}

#[derive(Debug)]
struct Thread {
    lock_file: PathBuf,
    /// The rollout first fed; another file's lines are not this thread's until that one is gone.
    rollout: Option<PathBuf>,
    turns: Turns,
}

/// What the rollout's lines said so far.
#[derive(Debug, Default)]
struct Turns {
    open: bool,
    /// The kind of the last turn event that could not be read; a readable one clears it.
    problem: Option<&'static str>,
    partial: Vec<u8>,
    skipping: bool,
}

impl CodexSource {
    /// `home` is Codex's home folder ([`codex_home`]).
    pub fn new(home: PathBuf) -> Self {
        Self {
            locks_dir: home.join(LOCKS_DIR),
            sessions_dir: home.join(SESSIONS_DIR),
            threads: BTreeMap::new(),
        }
    }

    /// The thread id of a lock file in the lock folder.
    fn lock_id(&self, file: &FileId) -> Option<String> {
        if file.0.parent() != Some(self.locks_dir.as_path()) {
            return None;
        }
        let name = file.0.file_name()?.to_str()?;
        is_lock_name(name).then(|| name[..ID_LEN].to_ascii_lowercase())
    }

    /// The thread id of a rollout in `sessions\YYYY\MM\DD\`, from its name's end.
    fn rollout_id(&self, file: &FileId) -> Option<String> {
        let date_root = file.0.parent()?.parent()?.parent()?.parent()?;
        if date_root != self.sessions_dir {
            return None;
        }
        let name = file.0.file_name()?.to_str()?.to_ascii_lowercase();
        let stem = name.strip_suffix(".jsonl")?;
        let at = stem.len().checked_sub(ID_LEN + 1)?;
        let (dash, id) = stem.get(at..)?.split_at(1);
        (dash == "-" && is_thread_id(id)).then(|| id.to_owned())
    }
}

impl Thread {
    fn new(lock_file: PathBuf) -> Self {
        Self {
            lock_file,
            rollout: None,
            turns: Turns::default(),
        }
    }

    fn take(&mut self, path: &Path, bytes: &[u8]) {
        match &self.rollout {
            Some(known) if known != path => return,
            Some(_) => {}
            None => self.rollout = Some(path.to_path_buf()),
        }
        self.turns.feed(bytes);
    }

    /// The rollout went (or the watch thread starts it over): what it said goes with it.
    fn forget(&mut self, path: &Path) {
        if self.rollout.as_deref() == Some(path) {
            self.rollout = None;
            self.turns = Turns::default();
        }
    }

    fn facts(&self, id: &str) -> SessionFacts {
        SessionFacts {
            key: SessionKey {
                source: SourceId::Codex,
                id: id.to_owned(),
            },
            liveness: Liveness::AnyProcess {
                image: IMAGE,
                lock_file: self.lock_file.clone(),
            },
            activity: if self.turns.open {
                Activity::Working
            } else {
                Activity::Idle
            },
            timers: Vec::new(),
            open_tasks: 0,
            label: LABEL,
            format: self.turns.problem.map_or(FormatHealth::Ok, |kind| {
                FormatHealth::UnknownValue(ROLLOUT, kind.to_owned())
            }),
        }
    }
}

impl Turns {
    /// Takes whole lines; a piece after the last newline waits for the rest, so bytes may come split anywhere.
    fn feed(&mut self, bytes: &[u8]) {
        let mut rest = bytes;
        while let Some(end) = rest.iter().position(|&b| b == b'\n') {
            let line = &rest[..end];
            rest = &rest[end + 1..];
            if std::mem::take(&mut self.skipping) {
                continue;
            }
            if self.partial.is_empty() {
                self.take_line(line);
            } else {
                let mut whole = std::mem::take(&mut self.partial);
                whole.extend_from_slice(line);
                self.take_line(&whole);
            }
        }
        if self.skipping {
            return;
        }
        if self.partial.len() + rest.len() > MAX_LINE {
            self.partial = Vec::new();
            self.skipping = true;
        } else {
            self.partial.extend_from_slice(rest);
        }
    }

    fn take_line(&mut self, line: &[u8]) {
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        if !line.windows(MARKER.len()).any(|w| w == MARKER) {
            return;
        }
        // A line that is not JSON is skipped: one torn or foreign line must not end a turn.
        let Ok(value) = serde_json::from_slice::<Value>(line) else {
            return;
        };
        if value.get("type").and_then(Value::as_str) != Some("event_msg") {
            return;
        }
        let payload = value.get("payload");
        let Some(kind) = payload.and_then(|p| p.get("type")).and_then(Value::as_str) else {
            self.problem = Some("event_msg");
            return;
        };
        let turn_id = payload.and_then(|p| p.get("turn_id"));
        let named = turn_id.is_some_and(Value::is_string);
        let (open, kind, readable) = match kind {
            "task_started" => (true, "task_started", named),
            "turn_started" => (true, "turn_started", named),
            "task_complete" => (false, "task_complete", named),
            "turn_complete" => (false, "turn_complete", named),
            // Codex 0.167.0 writes `turn_id` as optional here (older records have none).
            "turn_aborted" => (
                false,
                "turn_aborted",
                turn_id.is_none_or(|id| id.is_string() || id.is_null()),
            ),
            _ => return,
        };
        self.open = open;
        self.problem = (!readable).then_some(kind);
    }
}

impl AgentSource for CodexSource {
    fn id(&self) -> SourceId {
        SourceId::Codex
    }

    fn wanted(&self) -> Vec<WantedFile> {
        let mut wanted = vec![WantedFile::Names {
            dir: self.locks_dir.clone(),
            depth: 0,
            accept: is_lock_name,
        }];
        wanted.extend(self.threads.keys().map(|id| WantedFile::FindTail {
            root: self.sessions_dir.clone(),
            depth: DATE_DEPTH,
            name_suffix: format!("-{id}.jsonl"),
        }));
        wanted
    }

    fn feed(&mut self, file: &FileId, data: Feed) -> Fed {
        if let Some(id) = self.lock_id(file) {
            match data {
                Feed::Present => {
                    self.threads
                        .entry(id)
                        .or_insert_with(|| Thread::new(file.0.clone()));
                }
                Feed::Gone => {
                    self.threads.remove(&id);
                }
                // Never asked for: a lock file is listed, not read.
                Feed::Whole(_) | Feed::Appended(_) => {}
            }
            return Fed::Taken;
        }
        if let Some(id) = self.rollout_id(file)
            && let Some(thread) = self.threads.get_mut(&id)
        {
            match data {
                Feed::Appended(bytes) => thread.take(&file.0, &bytes),
                Feed::Gone => thread.forget(&file.0),
                Feed::Whole(_) | Feed::Present => {}
            }
        }
        Fed::Taken
    }

    fn sessions(&self) -> Vec<SessionFacts> {
        self.threads
            .iter()
            .map(|(id, thread)| thread.facts(id))
            .collect()
    }
}
