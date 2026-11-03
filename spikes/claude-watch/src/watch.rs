use crate::names::StatusFile;
use crate::procs;
use crate::summary::{self, Summarizer};
use crate::tail::Tail;
use chrono::Local;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Liveness, task files and schedule files are checked at this pace; status files and transcripts every round.
const SLOW_EVERY: Duration = Duration::from_secs(1);
/// A task output file that keeps growing is logged at most this often.
const GROWTH_LOG_EVERY: Duration = Duration::from_secs(10);
/// Claude Code writes status files with truncate-then-write, so a read can land between the two.
const REREAD_AFTER: Duration = Duration::from_millis(50);

pub struct Options {
    /// The user profile folder; `.claude` sits under it.
    pub home: PathBuf,
    /// The temp folder Claude Code uses (`%TEMP%`); task output files sit under `claude\` in it.
    pub temp: PathBuf,
    pub kinds: bool,
    /// Read transcripts of sessions that already run at start from the beginning, not from the end.
    pub backlog: bool,
}

pub struct Logger {
    file: Option<File>,
    echo: bool,
    pub captured: Option<Vec<String>>,
}

impl Logger {
    pub fn new(path: Option<&Path>) -> io::Result<Self> {
        let file = match path {
            Some(p) => Some(OpenOptions::new().create(true).append(true).open(p)?),
            None => None,
        };
        Ok(Self {
            file,
            echo: true,
            captured: None,
        })
    }

    #[cfg(test)]
    pub fn capture() -> Self {
        Self {
            file: None,
            echo: false,
            captured: Some(Vec::new()),
        }
    }

    pub fn line(&mut self, text: &str) {
        let line = format!("{} {text}", Local::now().format("%H:%M:%S%.3f"));
        if self.echo {
            println!("{line}");
        }
        if let Some(f) = &mut self.file {
            let _ = writeln!(f, "{line}");
        }
        if let Some(c) = &mut self.captured {
            c.push(line);
        }
    }
}

/// Raw bytes of a status file and its fields, or `None` fields when it did not parse.
type StatusRead = (Vec<u8>, Option<Map<String, Value>>);

struct Session {
    bytes: Vec<u8>,
    fields: Map<String, Value>,
    verdict: String,
}

struct Transcript {
    pid: u32,
    tail: Option<Tail>,
    from_start: bool,
    summarizer: Summarizer,
    last_search: Option<Instant>,
}

struct TaskFile {
    size: u64,
    logged_size: u64,
    logged_at: Instant,
}

pub struct Watcher {
    opts: Options,
    pub log: Logger,
    sessions: BTreeMap<u32, Session>,
    transcripts: BTreeMap<String, Transcript>,
    tasks: HashMap<PathBuf, TaskFile>,
    side_files: HashMap<PathBuf, Option<Vec<u8>>>,
    first_round: bool,
    last_slow: Option<Instant>,
}

fn field_str<'a>(fields: &'a Map<String, Value>, key: &str) -> Option<&'a str> {
    fields.get(key).and_then(Value::as_str)
}

fn proc_start(fields: &Map<String, Value>) -> Option<&str> {
    field_str(fields, "procStartFt").or_else(|| field_str(fields, "procStart"))
}

fn short(sid: &str) -> &str {
    sid.get(..8).unwrap_or(sid)
}

/// Takes the first JSON value only: issue anthropics/claude-code#96438 shows files with stray bytes after it.
pub fn first_object(bytes: &[u8]) -> Option<Map<String, Value>> {
    let mut values = serde_json::Deserializer::from_slice(bytes).into_iter::<Value>();
    match values.next()? {
        Ok(Value::Object(map)) => Some(map),
        _ => None,
    }
}

/// `projects\<folder>\<sessionId>.jsonl` under `<home>\.claude`, whichever folder holds it.
pub fn find_transcript(home: &Path, sid: &str) -> Option<PathBuf> {
    let projects = home.join(".claude").join("projects");
    let file = format!("{sid}.jsonl");
    fs::read_dir(projects)
        .ok()?
        .flatten()
        .map(|e| e.path().join(&file))
        .find(|p| p.is_file())
}

/// Field changes between two writes of one status file. `updatedAt` changes on every write, so it is left out.
pub fn diff(old: &Map<String, Value>, new: &Map<String, Value>) -> Vec<String> {
    let keys: BTreeSet<&String> = old.keys().chain(new.keys()).collect();
    keys.into_iter()
        .filter(|k| k.as_str() != "updatedAt")
        .filter_map(|k| match (old.get(k), new.get(k)) {
            (Some(a), Some(b)) if a == b => None,
            (Some(a), Some(b)) => Some(format!(
                "{k}: {} -> {}",
                summary::show(k, a),
                summary::show(k, b)
            )),
            (None, Some(b)) => Some(format!("{k}: (none) -> {}", summary::show(k, b))),
            (Some(a), None) => Some(format!("{k}: {} -> (gone)", summary::show(k, a))),
            (None, None) => None,
        })
        .collect()
}

impl Watcher {
    pub fn new(opts: Options, log: Logger) -> Self {
        Self {
            opts,
            log,
            sessions: BTreeMap::new(),
            transcripts: BTreeMap::new(),
            tasks: HashMap::new(),
            side_files: HashMap::new(),
            first_round: true,
            last_slow: None,
        }
    }

    fn sessions_dir(&self) -> PathBuf {
        self.opts.home.join(".claude").join("sessions")
    }

    pub fn round(&mut self) {
        self.scan_sessions();
        self.scan_transcripts();
        if self.last_slow.is_none_or(|t| t.elapsed() >= SLOW_EVERY) {
            self.check_liveness();
            self.scan_task_files();
            self.scan_side_files();
            self.last_slow = Some(Instant::now());
        }
        self.first_round = false;
    }

    fn read_status(&mut self, file: &StatusFile) -> Option<StatusRead> {
        let pid = file.pid();
        let mut bytes = match file.read() {
            Ok(b) => b,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return None,
            Err(e) => {
                self.log.line(&format!("[{pid}] READ ERROR {e}"));
                return None;
            }
        };
        if self.sessions.get(&pid).is_some_and(|s| s.bytes == bytes) {
            return None;
        }
        let mut fields = first_object(&bytes);
        if fields.is_none() {
            std::thread::sleep(REREAD_AFTER);
            bytes = file.read().ok()?;
            fields = first_object(&bytes);
        }
        Some((bytes, fields))
    }

    fn scan_sessions(&mut self) {
        let dir = self.sessions_dir();
        let Ok(entries) = fs::read_dir(&dir) else {
            return;
        };
        let mut seen = BTreeSet::new();
        for entry in entries.flatten() {
            let name = entry.file_name();
            let Some(file) = name.to_str().and_then(|n| StatusFile::from_name(&dir, n)) else {
                continue;
            };
            let pid = file.pid();
            seen.insert(pid);
            let Some((bytes, fields)) = self.read_status(&file) else {
                continue;
            };
            let Some(fields) = fields else {
                self.log.line(&format!(
                    "[{pid}] UNPARSED {} bytes, kept the last good copy",
                    bytes.len()
                ));
                continue;
            };
            self.on_status(pid, bytes, fields);
        }
        let gone: Vec<u32> = self
            .sessions
            .keys()
            .filter(|p| !seen.contains(p))
            .copied()
            .collect();
        for pid in gone {
            if let Some(s) = self.sessions.remove(&pid) {
                let status = field_str(&s.fields, "status").unwrap_or("(none)");
                let verdict = procs::verdict(pid, proc_start(&s.fields));
                self.log.line(&format!(
                    "[{pid}] GONE last status={status}; process: {verdict}"
                ));
            }
        }
    }

    fn on_status(&mut self, pid: u32, bytes: Vec<u8>, fields: Map<String, Value>) {
        let write = fields.get("updatedAt").and_then(Value::as_i64);
        let lag = write.map(|w| Local::now().timestamp_millis() - w);
        let when = match (write, lag) {
            (Some(w), Some(l)) => format!("write {} seen +{l} ms", summary::local_ms(w)),
            _ => "no updatedAt".to_string(),
        };
        match self.sessions.get_mut(&pid) {
            None => {
                let verdict = procs::verdict(pid, proc_start(&fields));
                let all = Value::Object(fields.clone());
                self.log.line(&format!("[{pid}] NEW ({when}) {all}"));
                self.log.line(&format!("[{pid}] process: {verdict}"));
                self.sessions.insert(
                    pid,
                    Session {
                        bytes,
                        fields: fields.clone(),
                        verdict,
                    },
                );
            }
            Some(s) => {
                let changes = diff(&s.fields, &fields);
                if changes.is_empty() {
                    self.log
                        .line(&format!("[{pid}] TOUCH ({when}) only updatedAt changed"));
                } else {
                    self.log
                        .line(&format!("[{pid}] CHANGE ({when}) {}", changes.join(", ")));
                }
                s.bytes = bytes;
                s.fields = fields.clone();
            }
        }
        if let Some(sid) = field_str(&fields, "sessionId")
            && !self.transcripts.contains_key(sid)
        {
            let from_start = !self.first_round || self.opts.backlog;
            self.transcripts.insert(
                sid.to_string(),
                Transcript {
                    pid,
                    tail: None,
                    from_start,
                    summarizer: Summarizer::default(),
                    last_search: None,
                },
            );
        }
    }

    fn scan_transcripts(&mut self) {
        let sids: Vec<String> = self.transcripts.keys().cloned().collect();
        for sid in sids {
            let needs_search = self.transcripts.get(&sid).is_some_and(|t| {
                t.tail.is_none() && t.last_search.is_none_or(|s| s.elapsed() >= SLOW_EVERY)
            });
            if needs_search {
                let found = find_transcript(&self.opts.home, &sid);
                let Some(t) = self.transcripts.get_mut(&sid) else {
                    continue;
                };
                t.last_search = Some(Instant::now());
                if let Some(path) = found {
                    let pid = t.pid;
                    let from = if t.from_start { "start" } else { "end" };
                    let shown = path.display().to_string();
                    t.tail = Some(Tail::new(path, t.from_start));
                    self.log.line(&format!(
                        "[{pid} {}] TRANSCRIPT {shown} (reading from {from})",
                        short(&sid)
                    ));
                }
            }
            let kinds = self.opts.kinds;
            let mut out = Vec::new();
            if let Some(t) = self.transcripts.get_mut(&sid)
                && let Some(tail) = t.tail.as_mut()
            {
                let lines = match tail.poll() {
                    Ok(l) => l,
                    Err(e) => {
                        out.push(format!("READ ERROR {}: {e}", tail.path().display()));
                        Vec::new()
                    }
                };
                for raw in lines {
                    let Ok(v) = serde_json::from_slice::<Value>(&raw) else {
                        out.push(format!("  (unparsed line, {} bytes)", raw.len()));
                        continue;
                    };
                    if let Some(text) = t.summarizer.line(&v, kinds) {
                        let at = v
                            .get("timestamp")
                            .and_then(Value::as_str)
                            .map(summary::local_iso);
                        out.push(format!("{} {text}", at.unwrap_or_else(|| "-".to_string())));
                    }
                }
                for line in &mut out {
                    *line = format!("[{} {}] T {line}", t.pid, short(&sid));
                }
            }
            for line in out {
                self.log.line(&line);
            }
        }
    }

    fn check_liveness(&mut self) {
        let mut changed = Vec::new();
        for (pid, s) in self.sessions.iter_mut() {
            let verdict = procs::verdict(*pid, proc_start(&s.fields));
            if verdict != s.verdict {
                changed.push(format!("[{pid}] process: {} -> {verdict}", s.verdict));
                s.verdict = verdict;
            }
        }
        for line in changed {
            self.log.line(&line);
        }
    }

    fn scan_task_files(&mut self) {
        let root = self.opts.temp.join("claude");
        let Ok(folders) = fs::read_dir(&root) else {
            return;
        };
        let folders: Vec<PathBuf> = folders.flatten().map(|e| e.path()).collect();
        let mut lines = Vec::new();
        for (sid, t) in &self.transcripts {
            for folder in &folders {
                let Ok(files) = fs::read_dir(folder.join(sid).join("tasks")) else {
                    continue;
                };
                for f in files.flatten() {
                    let path = f.path();
                    let size = f.metadata().map(|m| m.len()).unwrap_or(0);
                    let name = f.file_name().to_string_lossy().to_string();
                    let tag = format!("[{} {}]", t.pid, short(sid));
                    match self.tasks.get_mut(&path) {
                        None => {
                            if !self.first_round || t.from_start {
                                lines.push(format!("{tag} TASK FILE new {name} {size} bytes"));
                            }
                            self.tasks.insert(
                                path,
                                TaskFile {
                                    size,
                                    logged_size: size,
                                    logged_at: Instant::now(),
                                },
                            );
                        }
                        Some(tf) => {
                            tf.size = size;
                            if tf.size != tf.logged_size
                                && tf.logged_at.elapsed() >= GROWTH_LOG_EVERY
                            {
                                lines.push(format!("{tag} TASK FILE {name} now {size} bytes"));
                                tf.logged_size = size;
                                tf.logged_at = Instant::now();
                            }
                        }
                    }
                }
            }
        }
        for line in lines {
            self.log.line(&line);
        }
    }

    fn scan_side_files(&mut self) {
        let mut paths = vec![self.opts.home.join(".claude").join("scheduled_tasks.json")];
        for s in self.sessions.values() {
            if let Some(cwd) = field_str(&s.fields, "cwd") {
                let dot = Path::new(cwd).join(".claude");
                paths.push(dot.join("scheduled_tasks.json"));
                paths.push(dot.join("scheduled_tasks.lock"));
            }
        }
        paths.sort();
        paths.dedup();
        for path in paths {
            let now = fs::read(&path).ok();
            let before = self.side_files.get(&path);
            if before == Some(&now) {
                continue;
            }
            let first = before.is_none();
            self.side_files.insert(path.clone(), now.clone());
            if first && now.is_none() {
                continue;
            }
            let shown = match &now {
                None => "removed".to_string(),
                Some(b) => match serde_json::from_slice::<Value>(b) {
                    Ok(v) => summary::redact(&v).to_string(),
                    Err(_) => format!("{} bytes, not JSON", b.len()),
                },
            };
            let what = if first { "present" } else { "changed" };
            self.log
                .line(&format!("SIDE FILE {what} {}: {shown}", path.display()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::names::opened;
    use crate::procs::Liveness;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT: AtomicUsize = AtomicUsize::new(0);

    struct TempTree(PathBuf);

    impl TempTree {
        fn new() -> Self {
            let n = NEXT.fetch_add(1, Ordering::SeqCst);
            let root =
                std::env::temp_dir().join(format!("claude-watch-test-{}-{n}", std::process::id()));
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(root.join(".claude").join("sessions")).unwrap();
            fs::create_dir_all(root.join(".claude").join("projects").join("C--x")).unwrap();
            Self(root)
        }
        fn sessions(&self) -> PathBuf {
            self.0.join(".claude").join("sessions")
        }
        fn watcher(&self) -> Watcher {
            let opts = Options {
                home: self.0.clone(),
                temp: self.0.join("temp"),
                kinds: false,
                backlog: false,
            };
            Watcher::new(opts, Logger::capture())
        }
    }

    impl Drop for TempTree {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn own_start() -> String {
        match procs::check(std::process::id()) {
            Liveness::Running(t) => t.to_string(),
            other => panic!("{other:?}"),
        }
    }

    fn lines(w: &mut Watcher) -> Vec<String> {
        std::mem::take(w.log.captured.as_mut().unwrap())
    }

    #[test]
    fn first_object_ignores_trailing_bytes() {
        let m = first_object(br#"{"pid":1,"status":"idle"}us"}"#).unwrap();
        assert_eq!(m["status"], "idle");
        assert!(first_object(b"").is_none());
        assert!(first_object(br#"{"pid":1,"sta"#).is_none());
    }

    #[test]
    fn diff_lists_changed_fields_but_not_updated_at() {
        let a = first_object(br#"{"status":"busy","updatedAt":1,"bridgeSessionId":"session_x"}"#)
            .unwrap();
        let b = first_object(br#"{"status":"idle","updatedAt":2,"bridgeSessionId":null}"#).unwrap();
        assert_eq!(
            diff(&a, &b),
            vec![
                "bridgeSessionId: \"session_x\" -> null".to_string(),
                "status: \"busy\" -> \"idle\"".to_string()
            ]
        );
    }

    #[test]
    fn only_pid_json_files_are_opened() {
        let tree = TempTree::new();
        let pid = std::process::id();
        let status = format!(
            r#"{{"pid":{pid},"sessionId":"11111111-2222-3333-4444-555555555555","procStart":"{}","status":"idle","updatedAt":1}}"#,
            own_start()
        );
        fs::write(tree.sessions().join(format!("{pid}.json")), &status).unwrap();
        // A .key file that looks like a status file: it must not be read, so it must not show up at all.
        fs::write(
            tree.sessions().join("777.abcdef.key"),
            r#"{"pid":777,"status":"busy"}"#,
        )
        .unwrap();
        fs::write(tree.sessions().join(".fleetview-heartbeat"), "").unwrap();
        let mut w = tree.watcher();
        opened::take();
        w.round();
        assert_eq!(opened::take(), vec![format!("{pid}.json")]);
        let out = lines(&mut w);
        assert!(out.iter().any(|l| l.contains(&format!("[{pid}] NEW"))));
        assert!(
            out.iter()
                .any(|l| l.contains("process: alive, start time matches"))
        );
        assert!(!out.iter().any(|l| l.contains("777")));
    }

    #[test]
    fn status_change_and_removal_are_logged() {
        let tree = TempTree::new();
        let file = tree.sessions().join("4242.json");
        fs::write(&file, r#"{"pid":4242,"status":"busy","updatedAt":1}"#).unwrap();
        let mut w = tree.watcher();
        w.round();
        fs::write(&file, r#"{"pid":4242,"status":"shell","updatedAt":2}"#).unwrap();
        w.round();
        fs::remove_file(&file).unwrap();
        w.round();
        let out = lines(&mut w);
        assert!(out.iter().any(|l| l.contains("[4242] CHANGE") && l.contains("status: \"busy\" -> \"shell\"")));
        assert!(
            out.iter()
                .any(|l| l.contains("[4242] GONE last status=shell"))
        );
    }

    #[test]
    fn new_session_transcript_is_read_from_the_start() {
        let tree = TempTree::new();
        let mut w = tree.watcher();
        w.round();
        let sid = "aaaaaaaa-2222-3333-4444-555555555555";
        let transcript = tree
            .0
            .join(".claude")
            .join("projects")
            .join("C--x")
            .join(format!("{sid}.jsonl"));
        let call = r#"{"type":"assistant","timestamp":"2026-11-03T10:00:00.000Z","message":{"content":[{"type":"tool_use","id":"toolu_9","name":"ScheduleWakeup","input":{"delaySeconds":90,"reason":"r","prompt":"p"}}]}}"#;
        let prompt = r#"{"type":"user","message":{"content":"use ScheduleWakeup please"}}"#;
        fs::write(&transcript, format!("{prompt}\n{call}\n")).unwrap();
        fs::write(
            tree.sessions().join("4343.json"),
            format!(r#"{{"pid":4343,"sessionId":"{sid}","status":"idle"}}"#),
        )
        .unwrap();
        w.round();
        let out = lines(&mut w);
        let hits: Vec<&String> = out
            .iter()
            .filter(|l| l.contains("ScheduleWakeup"))
            .collect();
        assert_eq!(hits.len(), 1, "{out:#?}");
        assert!(hits[0].contains("[4343 aaaaaaaa] T") && hits[0].contains("\"delaySeconds\":90"));
        assert!(!out.iter().any(|l| l.contains("please")));
    }
}
