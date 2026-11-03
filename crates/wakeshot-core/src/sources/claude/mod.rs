//! Claude Code: status files and transcripts (architecture section 7).
//!
//! Owner: `p4-claude-status`, then `p4-claude-transcript`; fixes by `p4-live-claude`.

pub mod status;
pub mod transcript;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::PathBuf;

use crate::process::FileTime;
use crate::sources::cron::LocalOffset;
use crate::sources::{
    Activity, AgentSource, Fed, Feed, FileId, FormatHealth, Liveness, SessionFacts, SessionKey,
    SourceId, WantedFile,
};

use status::{Entrypoint, Problem, StatusFile};
use transcript::Replay;

/// What the tooltip names a Claude Code session.
pub const LABEL: &str = "Claude Code";

/// No process starts at the FILETIME epoch (1601), so a session with no known start never counts as alive. It is
/// listed anyway, so its format problem shows "update Wakeshot".
const NO_START: FileTime = FileTime(0);

/// What `FormatHealth::UnknownValue` names for a `sessionId` that is not a plain id.
pub const SESSION_ID: &str = "sessionId";

/// FILETIME of the Unix epoch, in 100 ns steps from 1601.
const UNIX_EPOCH_FT: i128 = 116_444_736_000_000_000;

/// Claude Code's sessions, one per `<home>\.claude\sessions\<pid>.json`, keyed by pid (track note 3), each with the
/// transcript `<home>\.claude\projects\<folder>\<sessionId>.jsonl` its status file names.
///
/// The watch thread is expected to read a `FindTail` from its start when it enters `wanted()`, keep its place while it
/// stays, and forget it when it leaves; a resumed process that takes over a transcript still listed goes on from
/// the replay that tail has fed.
pub struct ClaudeSource {
    sessions_dir: PathBuf,
    projects_dir: PathBuf,
    /// The PC's zone, for cron times (track note 2).
    zone: Box<dyn LocalOffset + Send>,
    files: BTreeMap<u32, Slot>,
}

/// One status file. `last` survives a bad read, so a retry or an unreadable file keeps the process it named.
#[derive(Debug, Default)]
struct Slot {
    last: Option<StatusFile>,
    unreadable: Option<&'static str>,
    retry_pending: bool,
    transcript: Option<Transcript>,
}

/// The transcript one session process writes. Its replay stays with the process across `/clear`, when `session_id`
/// changes (track note 3, rule 5).
#[derive(Clone, Debug)]
struct Transcript {
    proc_start: FileTime,
    session_id: String,
    /// The file first fed for `session_id`; the same name in another project folder is not this session's.
    path: Option<PathBuf>,
    replay: Replay,
}

impl ClaudeSource {
    /// `home` is the user's profile folder, the one that holds `.claude`; `zone` gives the PC's local time.
    pub fn new(home: PathBuf, zone: Box<dyn LocalOffset + Send>) -> Self {
        let claude = home.join(".claude");
        Self {
            sessions_dir: claude.join("sessions"),
            projects_dir: claude.join("projects"),
            zone,
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

    /// The session id of a transcript in `projects\<folder>\`; whether a session wants it is checked by the caller.
    fn transcript_session<'a>(&self, file: &'a FileId) -> Option<&'a str> {
        if file.0.parent()?.parent()? != self.projects_dir.as_path() {
            return None;
        }
        file.0.file_name()?.to_str()?.strip_suffix(".jsonl")
    }

    fn take_status(&mut self, pid: u32, bytes: &[u8]) -> Fed {
        match status::parse(pid, bytes) {
            Ok(file) => {
                let transcript = self.transcript_for(pid, &file);
                self.files.insert(
                    pid,
                    Slot {
                        last: Some(file),
                        transcript,
                        ..Slot::default()
                    },
                );
                Fed::Taken
            }
            Err(problem) => {
                let slot = self.files.entry(pid).or_default();
                // Claude truncates, then writes, so a read 200 ms later usually finds the file whole.
                if problem == Problem::NotParsed && !slot.retry_pending {
                    slot.retry_pending = true;
                    return Fed::RetryOnce;
                }
                slot.retry_pending = false;
                slot.unreadable = Some(problem.what());
                Fed::Taken
            }
        }
    }

    /// The transcript after a good status read: the same process keeps its replay, also when `/clear` moved it to a
    /// new `sessionId`; a new process starts one.
    fn transcript_for(&mut self, pid: u32, file: &StatusFile) -> Option<Transcript> {
        let kept = self
            .files
            .get_mut(&pid)
            .and_then(|slot| slot.transcript.take())
            .filter(|transcript| transcript.proc_start == file.proc_start);
        let Some(session_id) = &file.session_id else {
            return kept;
        };
        if let Some(mut transcript) = kept {
            if &transcript.session_id != session_id {
                transcript.session_id.clone_from(session_id);
                transcript.path = None;
                transcript.replay.start_file();
            }
            return Some(transcript);
        }
        // A resume takes over the transcript of a process that just ended; when that crash leftover is still listed,
        // its tail goes on unbroken, so its replay is the start.
        let shared = self
            .files
            .values()
            .filter_map(|slot| slot.transcript.as_ref())
            .find(|transcript| &transcript.session_id == session_id);
        Some(match shared {
            Some(shared) => Transcript {
                proc_start: file.proc_start,
                ..shared.clone()
            },
            None => Transcript {
                proc_start: file.proc_start,
                session_id: session_id.clone(),
                path: None,
                replay: Replay::default(),
            },
        })
    }

    fn take_transcript(&mut self, file: &FileId, session_id: &str, bytes: &[u8]) {
        let zone = &*self.zone;
        for transcript in self
            .files
            .values_mut()
            .filter_map(|slot| slot.transcript.as_mut())
            .filter(|transcript| transcript.session_id == session_id)
        {
            match &transcript.path {
                Some(path) if path != &file.0 => continue,
                Some(_) => {}
                None => transcript.path = Some(file.0.clone()),
            }
            transcript.replay.feed(bytes, zone);
        }
    }
}

impl fmt::Debug for ClaudeSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClaudeSource")
            .field("sessions_dir", &self.sessions_dir)
            .field("projects_dir", &self.projects_dir)
            .field("files", &self.files)
            .finish_non_exhaustive()
    }
}

impl Slot {
    fn facts(&self, pid: u32, zone: &dyn LocalOffset) -> Option<SessionFacts> {
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
        let (timers, open_tasks, problem) = match &self.transcript {
            Some(transcript) => {
                let since = wall_ms(transcript.proc_start);
                // Rule 3: only a terminal session shows a background shell as `shell`.
                let cli =
                    self.last.as_ref().and_then(|file| file.entrypoint) == Some(Entrypoint::Cli);
                let mut timers = transcript.replay.timers(since, zone);
                let open_tasks = if cli {
                    0
                } else {
                    timers.extend(transcript.replay.task_timers(since));
                    timers.sort_unstable();
                    timers.dedup();
                    transcript.replay.open_tasks(since)
                };
                (timers, open_tasks, transcript.replay.problem())
            }
            None => (Vec::new(), 0, None),
        };
        // A `sessionId` of another shape hides the transcript, so it is told the same way (review m5).
        let problem = self
            .last
            .as_ref()
            .and_then(|file| file.session_id_problem.as_deref())
            .map(|value| (SESSION_ID, value))
            .or(problem);
        // The core counts any `UnknownValue` as working while the process lives, so a transcript problem may not make
        // a session hold (orchestrator, 2026-11-03): it is told only while the session holds anyway, which still shows
        // "update Wakeshot" at its next turn.
        let format = match (format, problem) {
            (FormatHealth::Ok, Some((field, value)))
                if activity == Activity::Working || open_tasks > 0 =>
            {
                FormatHealth::UnknownValue(field, value.to_owned())
            }
            (format, _) => format,
        };
        Some(SessionFacts {
            key: SessionKey {
                source: SourceId::ClaudeCode,
                id: pid.to_string(),
            },
            liveness: Liveness::Process { pid, start },
            activity,
            timers,
            open_tasks,
            label: LABEL,
            format,
        })
    }
}

/// A process start as wall ms: transcript lines older than it were written by an earlier process.
fn wall_ms(start: FileTime) -> i64 {
    let ms = (i128::from(start.0) - UNIX_EPOCH_FT).div_euclid(10_000);
    i64::try_from(ms).unwrap_or(i64::MAX)
}

impl AgentSource for ClaudeSource {
    fn id(&self) -> SourceId {
        SourceId::ClaudeCode
    }

    fn wanted(&self) -> Vec<WantedFile> {
        let sessions: BTreeSet<&str> = self
            .files
            .values()
            .filter_map(|slot| slot.transcript.as_ref())
            .map(|transcript| transcript.session_id.as_str())
            .collect();
        let mut wanted = vec![WantedFile::Folder {
            dir: self.sessions_dir.clone(),
            accept: status::is_status_file_name,
        }];
        // Rule 5: found by name, since the folder name is built from the cwd in ways not worth copying.
        wanted.extend(sessions.into_iter().map(|id| WantedFile::FindTail {
            root: self.projects_dir.clone(),
            depth: 1,
            name_suffix: format!("{id}.jsonl"),
        }));
        wanted
    }

    fn feed(&mut self, file: &FileId, data: Feed) -> Fed {
        if let Some(pid) = self.status_pid(file) {
            match data {
                Feed::Whole(bytes) => return self.take_status(pid, &bytes),
                Feed::Gone => {
                    self.files.remove(&pid);
                }
                // A folder file comes whole or gone; a tail or a listing of it says nothing.
                Feed::Appended(_) | Feed::Present => {}
            }
            return Fed::Taken;
        }
        if let (Some(session_id), Feed::Appended(bytes)) = (self.transcript_session(file), &data) {
            // Only appended bytes say anything: jobs live in the process, so a deleted transcript keeps what it armed.
            self.take_transcript(file, session_id, bytes);
        }
        Fed::Taken
    }

    fn sessions(&self) -> Vec<SessionFacts> {
        self.files
            .iter()
            .filter_map(|(&pid, slot)| slot.facts(pid, &*self.zone))
            .collect()
    }
}
