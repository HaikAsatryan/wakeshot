//! Claude Code: status files and transcripts (architecture section 7).
//!
//! Owner: `p4-claude-status`, then `p4-claude-transcript`; fixes by `p4-live-claude`.

pub mod status;
pub mod transcript;

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::process::FileTime;
use crate::sources::{
    Activity, AgentSource, Fed, Feed, FileId, FormatHealth, Liveness, SessionFacts, SessionKey,
    SourceId, WantedFile,
};

use status::{Problem, StatusFile};

/// What the tooltip names a Claude Code session.
pub const LABEL: &str = "Claude Code";

/// No process starts at the FILETIME epoch (1601), so a session with no known start never counts as alive. It is
/// listed anyway, so its format problem shows "update Wakeshot".
const NO_START: FileTime = FileTime(0);

/// Claude Code's sessions, one per `<home>\.claude\sessions\<pid>.json`, keyed by pid (track note 3).
#[derive(Debug)]
pub struct ClaudeSource {
    sessions_dir: PathBuf,
    files: BTreeMap<u32, Slot>,
}

/// One status file. `last` survives a bad read, so a retry or an unreadable file keeps the process it named.
#[derive(Debug, Default)]
struct Slot {
    last: Option<StatusFile>,
    unreadable: Option<&'static str>,
    retry_pending: bool,
}

impl ClaudeSource {
    /// `home` is the user's profile folder, the one that holds `.claude`.
    pub fn new(home: PathBuf) -> Self {
        Self {
            sessions_dir: home.join(".claude").join("sessions"),
            files: BTreeMap::new(),
        }
    }

    /// The pid of a status file this source asked for; `None` for any other file, which is then never parsed.
    fn status_pid(&self, file: &FileId) -> Option<u32> {
        if file.0.parent() != Some(self.sessions_dir.as_path()) {
            return None;
        }
        status::status_file_pid(file.0.file_name()?.to_str()?)
    }

    fn take_status(&mut self, pid: u32, bytes: &[u8]) -> Fed {
        let slot = self.files.entry(pid).or_default();
        match status::parse(pid, bytes) {
            Ok(file) => {
                *slot = Slot {
                    last: Some(file),
                    ..Slot::default()
                };
                Fed::Taken
            }
            // Claude truncates, then writes, so a read 200 ms later usually finds the file whole.
            Err(Problem::NotParsed) if !slot.retry_pending => {
                slot.retry_pending = true;
                Fed::RetryOnce
            }
            Err(problem) => {
                slot.retry_pending = false;
                slot.unreadable = Some(problem.what());
                Fed::Taken
            }
        }
    }
}

impl Slot {
    fn facts(&self, pid: u32) -> Option<SessionFacts> {
        let (start, activity, format) = match (self.unreadable, &self.last) {
            (Some(what), last) => (
                last.as_ref().map_or(NO_START, |file| file.proc_start),
                Activity::Idle,
                FormatHealth::Unreadable(what),
            ),
            (None, Some(file)) => (
                file.proc_start,
                file.activity,
                match &file.unknown_status {
                    Some(value) => FormatHealth::UnknownValue("status", value.clone()),
                    None => FormatHealth::Ok,
                },
            ),
            // The first read failed and its retry is still to come: nothing to say yet.
            (None, None) => return None,
        };
        Some(SessionFacts {
            key: SessionKey {
                source: SourceId::ClaudeCode,
                id: pid.to_string(),
            },
            liveness: Liveness::Process { pid, start },
            activity,
            timers: Vec::new(),
            open_tasks: 0,
            label: LABEL,
            format,
        })
    }
}

impl AgentSource for ClaudeSource {
    fn id(&self) -> SourceId {
        SourceId::ClaudeCode
    }

    fn wanted(&self) -> Vec<WantedFile> {
        vec![WantedFile::Folder {
            dir: self.sessions_dir.clone(),
            accept: status::is_status_file_name,
        }]
    }

    fn feed(&mut self, file: &FileId, data: Feed) -> Fed {
        let Some(pid) = self.status_pid(file) else {
            return Fed::Taken;
        };
        match data {
            Feed::Whole(bytes) => self.take_status(pid, &bytes),
            Feed::Gone => {
                self.files.remove(&pid);
                Fed::Taken
            }
            // A folder file comes whole or gone; a tail or a listing of it says nothing.
            Feed::Appended(_) | Feed::Present => Fed::Taken,
        }
    }

    fn sessions(&self) -> Vec<SessionFacts> {
        self.files
            .iter()
            .filter_map(|(&pid, slot)| slot.facts(pid))
            .collect()
    }
}
