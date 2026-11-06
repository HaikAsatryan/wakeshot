//! A live harness for the watch thread (card `p4-live-claude`): the real `WatchThread` with the real agent sources,
//! `SystemLiveness` and the PC's zone, printing each event it posts as one redacted line, so a live test can compare
//! what the thread posts with what an agent session does. A later source is one more entry in `sources`.
//!
//! Owner: `p4-live-claude`.
//!
//! Build: `cargo build --locked -p wakeshot-windows --example agent-watch`. Then `agent-watch --help`. Ctrl+C or
//! Ctrl+Break stops it.
//!
//! No line holds a prompt or any file content: sessions show their key, activity, timers as local times, open tasks,
//! liveness and format health; holds show id, kind, liveness and expiry, never the reason. `--raw` adds the kinds of
//! the lines each tail fed (line type, subtype, tool names, number and yes/no fields, notification statuses) and the
//! status of each status file read, never a text field.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::Value;
use wakeshot_core::event::Event;
use wakeshot_core::files::Config;
use wakeshot_core::holds::HoldFacts;
use wakeshot_core::process::Alive;
use wakeshot_core::sources::claude::{ClaudeSource, claude_folder};
use wakeshot_core::sources::cron::LocalOffset;
use wakeshot_core::sources::{
    Activity, AgentSource, Fed, Feed, FileId, FormatHealth, Liveness as Seen, SessionFacts,
    SourceId, WantedFile,
};
use wakeshot_windows::WinError;
use wakeshot_windows::clock;
use wakeshot_windows::localtime::SystemZone;
use wakeshot_windows::log::{Level, Log};
use wakeshot_windows::process::{Liveness, SystemLiveness};
use wakeshot_windows::sink::EventSink;
use wakeshot_windows::store::DataDir;
use wakeshot_windows::watch::thread::WatchThread;
use windows::Win32::System::Console::{CTRL_BREAK_EVENT, CTRL_C_EVENT, SetConsoleCtrlHandler};
use windows::core::BOOL;

const HELP: &str = "\
agent-watch: runs the real watch thread and prints what it posts.

  agent-watch [--data <folder>] [--raw <file>]

  --data <folder>  the data folder to watch (holds, config); default: the dry-run folder
                   (%LocalAppData%\\Wakeshot\\dry-run). The real data folder is refused.
  --raw <file>     also write, per fed file, the kinds of the lines it fed (never their text)

Claude Code is read where it writes: %CLAUDE_CONFIG_DIR% when set, else %USERPROFILE%\\.claude.
Ctrl+C stops it.";

/// The tray's safety tick (architecture section 4); the thread has no timer of its own.
const TICK: Duration = Duration::from_secs(30);

const MS_PER_DAY: i64 = 86_400_000;

/// Set by the console handler; the main loop polls it, since nothing else may run inside the handler.
static STOP: AtomicBool = AtomicBool::new(false);

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(text) => {
            eprintln!("agent-watch: {text}");
            ExitCode::FAILURE
        }
    }
}

struct Args {
    data: Option<PathBuf>,
    raw: Option<PathBuf>,
}

fn parse_args() -> Result<Option<Args>, String> {
    let mut args = Args {
        data: None,
        raw: None,
    };
    let mut it = std::env::args_os().skip(1);
    while let Some(arg) = it.next() {
        match arg.to_str() {
            Some("--help" | "-h") => return Ok(None),
            Some("--data") => args.data = Some(it.next().ok_or("--data needs a folder")?.into()),
            Some("--raw") => args.raw = Some(it.next().ok_or("--raw needs a file")?.into()),
            _ => return Err(format!("unknown argument {arg:?}; see --help")),
        }
    }
    Ok(Some(args))
}

fn run() -> Result<(), String> {
    let Some(args) = parse_args()? else {
        println!("{HELP}");
        return Ok(());
    };
    let real = DataDir::real().map_err(|e| win("the data folder", e))?;
    let dir = match args.data {
        Some(path) => DataDir::at(path),
        None => DataDir::dry_run().map_err(|e| win("the dry-run folder", e))?,
    };
    // The real folder is the tray's: a live test must never feed it holds or read its restore request.
    if same_folder(dir.root(), real.root()) {
        return Err("refusing the real data folder; give a test folder or leave --data out".into());
    }
    let home = std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .ok_or("USERPROFILE is not set")?;
    // The folder Claude Code itself uses, so a `CLAUDE_CONFIG_DIR` session is seen too.
    let claude = claude_folder(&home, std::env::var_os("CLAUDE_CONFIG_DIR").as_deref());

    let out = Arc::new(Out::new());
    let raw = match &args.raw {
        Some(path) => Some(Arc::new(RawLog {
            out: Arc::clone(&out),
            file: Mutex::new(File::create(path).map_err(|e| format!("--raw: {e}"))?),
        })),
        None => None,
    };
    let sources: Vec<Box<dyn AgentSource + Send>> = vec![tap(
        Box::new(ClaudeSource::in_folder(
            claude.clone(),
            Box::new(SystemZone::current()),
        )),
        raw.as_ref(),
    )];

    // SAFETY: the handler is a plain function that lives as long as the process.
    unsafe { SetConsoleCtrlHandler(Some(on_console_event), true) }
        .map_err(|e| format!("SetConsoleCtrlHandler failed: {e}"))?;

    out.line(&format!(
        "start data={} claude={} tick={}s",
        dir.root().display(),
        claude.display(),
        TICK.as_secs()
    ));
    let liveness: Arc<dyn Liveness> = Arc::new(SystemLiveness);
    let sink: Arc<dyn EventSink> = Arc::new(PrintSink(Arc::clone(&out)));
    let log: Arc<dyn Log> = Arc::new(PrintLog(Arc::clone(&out)));
    let handle = WatchThread::start(dir, Config::default(), sources, liveness, sink, log)
        .map_err(|e| win("the watch thread", e))?;

    let mut last_tick = Instant::now();
    while !STOP.load(Ordering::SeqCst) {
        std::thread::sleep(Duration::from_millis(100));
        if last_tick.elapsed() >= TICK {
            out.line("tick");
            handle.tick();
            last_tick = Instant::now();
        }
    }
    out.line("stopping");
    let stopped = handle.stop();
    out.line(&format!("stopped clean={stopped}"));
    if stopped {
        Ok(())
    } else {
        Err("the watch thread did not stop cleanly".into())
    }
}

fn win(what: &str, e: WinError) -> String {
    format!("{what}: {} failed with {}", e.call, e.code)
}

/// Case and a trailing separator do not make another folder on Windows; the folders may not exist yet.
fn same_folder(a: &Path, b: &Path) -> bool {
    let norm = |p: &Path| {
        p.to_string_lossy()
            .trim_end_matches(['\\', '/'])
            .to_lowercase()
    };
    norm(a) == norm(b)
}

/// Ctrl+C and Ctrl+Break stop the harness; closing the console goes on to the default handler.
unsafe extern "system" fn on_console_event(kind: u32) -> BOOL {
    let ours = kind == CTRL_C_EVENT || kind == CTRL_BREAK_EVENT;
    if ours {
        STOP.store(true, Ordering::SeqCst);
    }
    BOOL::from(ours)
}

/// One line per event on stdout, each with the local time it was printed.
struct Out {
    zone: SystemZone,
    lock: Mutex<()>,
}

impl Out {
    fn new() -> Self {
        Self {
            zone: SystemZone::current(),
            lock: Mutex::new(()),
        }
    }

    fn line(&self, text: &str) {
        let now = self.clock(clock::now().wall_ms);
        let _held = self
            .lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        println!("{now} {text}");
    }

    /// Local `HH:MM:SS.mmm`.
    fn clock(&self, wall_ms: i64) -> String {
        let (_, ms) = self.local(wall_ms);
        format!(
            "{:02}:{:02}:{:02}.{:03}",
            ms / 3_600_000,
            ms / 60_000 % 60,
            ms / 1_000 % 60,
            ms % 1_000
        )
    }

    /// Local `HH:MM:SS`, plus the days ahead of today when it is not today.
    fn when(&self, wall_ms: i64) -> String {
        let (day, ms) = self.local(wall_ms);
        let (today, _) = self.local(clock::now().wall_ms);
        let time = format!(
            "{:02}:{:02}:{:02}",
            ms / 3_600_000,
            ms / 60_000 % 60,
            ms / 1_000 % 60
        );
        match day - today {
            0 => time,
            ahead => format!("{time}({ahead:+}d)"),
        }
    }

    fn local(&self, wall_ms: i64) -> (i64, i64) {
        let local = wall_ms + i64::from(self.zone.offset_minutes(wall_ms)) * 60_000;
        (local.div_euclid(MS_PER_DAY), local.rem_euclid(MS_PER_DAY))
    }
}

struct PrintSink(Arc<Out>);

impl EventSink for PrintSink {
    fn post(&self, event: Event) -> Result<(), WinError> {
        let text = match &event {
            Event::Sources { source, sessions } => self.sources(*source, sessions),
            Event::Holds(holds) => self.holds(holds),
            Event::ConfigChanged(config) => format!("config automatic={}", config.automatic),
            Event::Menu(command) => format!("menu {command:?}"),
            _ => "other event".to_owned(),
        };
        self.0.line(&text);
        Ok(())
    }
}

impl PrintSink {
    fn sources(&self, source: SourceId, sessions: &[(SessionFacts, Alive)]) -> String {
        let list: Vec<String> = sessions
            .iter()
            .map(|(facts, alive)| self.session(facts, *alive))
            .collect();
        format!(
            "sources {source:?} n={}: {}",
            sessions.len(),
            list.join(" | ")
        )
    }

    fn session(&self, facts: &SessionFacts, alive: Alive) -> String {
        let activity = match facts.activity {
            Activity::Working => "working".to_owned(),
            Activity::Idle => "idle".to_owned(),
            Activity::Waiting {
                since_wall_ms,
                remote,
            } => format!(
                "waiting(remote={remote}, since={})",
                since_wall_ms.map_or_else(|| "-".to_owned(), |ms| self.0.when(ms))
            ),
        };
        let timers: Vec<String> = facts.timers.iter().map(|&ms| self.0.when(ms)).collect();
        let liveness = match &facts.liveness {
            Seen::Process { pid, .. } => format!("pid {pid}"),
            Seen::AnyProcess { image, .. } => format!("any {image}"),
            Seen::ImageSince { images, .. } => format!("since {}", images.join("|")),
        };
        format!(
            "{} {activity} timers=[{}] tasks={} {liveness} alive={alive:?} format={}",
            facts.key.id,
            timers.join(","),
            facts.open_tasks,
            format_health(&facts.format)
        )
    }

    fn holds(&self, holds: &[HoldFacts]) -> String {
        let list: Vec<String> = holds
            .iter()
            .map(|hold| {
                format!(
                    "{} {:?} alive={:?} expires={} screen={}",
                    hold.id.0,
                    hold.kind,
                    hold.alive,
                    hold.expires_wall_ms
                        .map_or_else(|| "-".to_owned(), |ms| self.0.when(ms)),
                    hold.screen
                )
            })
            .collect();
        format!("holds n={}: {}", holds.len(), list.join(" | "))
    }
}

fn format_health(format: &FormatHealth) -> String {
    match format {
        FormatHealth::Ok => "ok".to_owned(),
        FormatHealth::Unreadable(what) => format!("unreadable({what})"),
        // The value is a field value such as a status word; cut, in case a format change puts text there.
        FormatHealth::UnknownValue(field, value) => {
            format!(
                "unknown({field}={})",
                value.chars().take(40).collect::<String>()
            )
        }
    }
}

struct PrintLog(Arc<Out>);

impl Log for PrintLog {
    fn line(&self, level: Level, area: &'static str, text: &str) {
        self.0.line(&format!("log {level:?} {area}: {text}"));
    }
}

/// The `--raw` file.
struct RawLog {
    out: Arc<Out>,
    file: Mutex<File>,
}

impl RawLog {
    fn line(&self, text: &str) {
        let now = self.out.clock(clock::now().wall_ms);
        let mut file = self
            .file
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        // A lost raw line only weakens the record of a live test, so it never stops the harness.
        let _ = writeln!(file, "{now} {text}");
        let _ = file.flush();
    }
}

fn tap(
    source: Box<dyn AgentSource + Send>,
    raw: Option<&Arc<RawLog>>,
) -> Box<dyn AgentSource + Send> {
    match raw {
        Some(raw) => Box::new(RawTap {
            inner: source,
            raw: Arc::clone(raw),
            carry: BTreeMap::new(),
        }),
        None => source,
    }
}

/// Passes every call through and writes what each feed held, as kinds only.
struct RawTap {
    inner: Box<dyn AgentSource + Send>,
    raw: Arc<RawLog>,
    /// A tail can end inside a line; its start waits here for the rest.
    carry: BTreeMap<PathBuf, Vec<u8>>,
}

impl AgentSource for RawTap {
    fn id(&self) -> SourceId {
        self.inner.id()
    }

    fn wanted(&self) -> Vec<WantedFile> {
        self.inner.wanted()
    }

    fn feed(&mut self, file: &FileId, data: Feed) -> Fed {
        self.note(file, &data);
        let fed = self.inner.feed(file, data);
        if fed == Fed::RetryOnce {
            self.raw
                .line(&format!("{} retry once", short_name(&file.0)));
        }
        fed
    }

    fn sessions(&self) -> Vec<SessionFacts> {
        self.inner.sessions()
    }
}

impl RawTap {
    fn note(&mut self, file: &FileId, data: &Feed) {
        let name = short_name(&file.0);
        match data {
            Feed::Whole(bytes) => self
                .raw
                .line(&format!("{name} whole {}", status_kind(bytes))),
            Feed::Appended(bytes) => {
                let carry = self.carry.entry(file.0.clone()).or_default();
                carry.extend_from_slice(bytes);
                let Some(end) = carry.iter().rposition(|&b| b == b'\n') else {
                    self.raw
                        .line(&format!("{name} +{} bytes, no full line", bytes.len()));
                    return;
                };
                let lines: Vec<u8> = carry.drain(..=end).collect();
                let kinds: Vec<String> = lines
                    .split(|&b| b == b'\n')
                    .filter(|line| !line.iter().all(u8::is_ascii_whitespace))
                    .map(line_kind)
                    .collect();
                self.raw.line(&format!(
                    "{name} +{} bytes, {} lines",
                    bytes.len(),
                    kinds.len()
                ));
                for kind in kinds {
                    self.raw.line(&format!("  {name} {kind}"));
                }
            }
            Feed::Present => self.raw.line(&format!("{name} present")),
            Feed::Gone => {
                self.carry.remove(&file.0);
                self.raw.line(&format!("{name} gone"));
            }
        }
    }
}

/// A status file by its name (the pid); a transcript by the first 8 characters of its session id, since its folder
/// name is built from a path.
fn short_name(path: &Path) -> String {
    let name = path
        .file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
    match name.strip_suffix(".jsonl") {
        Some(id) => format!("t:{}", id.chars().take(8).collect::<String>()),
        None => name,
    }
}

/// The fields of a status file that the rules read, never its paths or names.
fn status_kind(bytes: &[u8]) -> String {
    let Some(Ok(v)) = serde_json::Deserializer::from_slice(bytes)
        .into_iter::<Value>()
        .next()
    else {
        return format!("not parsed ({} bytes)", bytes.len());
    };
    let word = |key: &str| v.get(key).and_then(Value::as_str).map_or("-", short_word);
    let number = |key: &str| v.get(key).and_then(Value::as_i64);
    format!(
        "status={} waitingFor={} entrypoint={} kind={} session={} statusUpdatedAt={} bridge={}",
        word("status"),
        v.get("waitingFor")
            .and_then(Value::as_str)
            .map_or("-", |s| if s.len() <= 20 { s } else { "(long)" }),
        word("entrypoint"),
        word("kind"),
        v.get("sessionId")
            .and_then(Value::as_str)
            .map_or("-", |s| s.get(..8).unwrap_or(s)),
        number("statusUpdatedAt").map_or_else(|| "-".to_owned(), |ms| ms.to_string()),
        match v.get("bridgeSessionId") {
            None => "absent",
            Some(Value::Null) => "null",
            Some(_) => "set",
        }
    )
}

/// A value Claude Code picks from a fixed set (a status, an entrypoint); anything longer may be text.
fn short_word(s: &str) -> &str {
    if s.len() <= 24
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        s
    } else {
        "(other)"
    }
}

/// One transcript line as its kinds: type, subtype, tool names with their number and yes/no inputs, result field
/// names, and the statuses of task notifications. Text fields are never copied.
fn line_kind(line: &[u8]) -> String {
    let Ok(v) = serde_json::from_slice::<Value>(line) else {
        return format!("not json ({} bytes)", line.len());
    };
    let str_of = |v: &Value, key: &str| {
        v.get(key)
            .and_then(Value::as_str)
            .map(short_word)
            .unwrap_or("-")
            .to_owned()
    };
    let ty = str_of(&v, "type");
    let mut kind = ty.clone();
    if let Some(sub) = v.get("subtype").and_then(Value::as_str) {
        kind = format!("{kind}/{}", short_word(sub));
    }
    match ty.as_str() {
        "attachment" => {
            let a = v.get("attachment").unwrap_or(&Value::Null);
            kind.push(':');
            kind.push_str(&str_of(a, "type"));
            kind.push_str(&notification(a.get("prompt")));
        }
        "queue-operation" => {
            kind.push(':');
            kind.push_str(&str_of(&v, "operation"));
            kind.push_str(&notification(v.get("content")));
        }
        "assistant" => {
            for item in content_items(&v) {
                match item.get("type").and_then(Value::as_str) {
                    Some("tool_use") => {
                        kind.push_str(&format!(
                            " tool_use:{}{}",
                            str_of(item, "name"),
                            plain_fields(item.get("input"))
                        ));
                    }
                    Some(other) => kind.push_str(&format!(" {}", short_word(other))),
                    None => kind.push_str(" ?"),
                }
            }
        }
        "user" => {
            match v.get("message").and_then(|m| m.get("content")) {
                Some(Value::String(_)) => {
                    kind.push_str(&notification(
                        v.get("message").and_then(|m| m.get("content")),
                    ));
                }
                Some(Value::Array(items)) => {
                    for item in items {
                        match item.get("type").and_then(Value::as_str) {
                            Some("tool_result") => {
                                kind.push_str(" tool_result");
                                if item.get("is_error").and_then(Value::as_bool) == Some(true) {
                                    kind.push_str("(error)");
                                }
                            }
                            Some("text") => kind.push_str(&notification(item.get("text"))),
                            Some(other) => kind.push_str(&format!(" {}", short_word(other))),
                            None => kind.push_str(" ?"),
                        }
                    }
                }
                _ => {}
            }
            if let Some(result) = v.get("toolUseResult") {
                kind.push_str(&format!(" result{}", plain_fields(Some(result))));
            }
            if v.get("isMeta").and_then(Value::as_bool) == Some(true) {
                kind.push_str(" meta");
            }
        }
        _ => {}
    }
    if let Some(ts) = v
        .get("timestamp")
        .and_then(Value::as_str)
        .filter(|ts| ts.len() <= 30)
    {
        kind.push_str(&format!(" @{ts}"));
    }
    kind
}

fn content_items(v: &Value) -> &[Value] {
    v.get("message")
        .and_then(|m| m.get("content"))
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

/// An object's field names, with the value only for numbers and yes/no; text fields show as `name:"`.
fn plain_fields(v: Option<&Value>) -> String {
    let Some(Value::Object(map)) = v else {
        return String::new();
    };
    let fields: Vec<String> = map
        .iter()
        .map(|(key, value)| match value {
            Value::Number(n) => format!("{key}={n}"),
            Value::Bool(b) => format!("{key}={b}"),
            Value::Null => format!("{key}=null"),
            Value::String(_) => format!("{key}:\""),
            Value::Array(items) => format!("{key}:[{}]", items.len()),
            Value::Object(_) => format!("{key}:{{}}"),
        })
        .collect();
    format!("{{{}}}", fields.join(","))
}

/// ` task-notification(status=..)` when the text starts with one or more task notifications (after Claude Code's
/// header, if any); else ` text`. Only the `<status>` words are copied, and whether an `<event>` tag is present.
fn notification(text: Option<&Value>) -> String {
    let Some(text) = text.and_then(Value::as_str) else {
        return String::new();
    };
    let body = text
        .strip_prefix("[SYSTEM NOTIFICATION - NOT USER INPUT]")
        .map_or(text, |rest| {
            rest.split_once("\n\n").map_or(rest, |(_, body)| body)
        });
    if !body.trim_start().starts_with("<task-notification>") {
        return " text".to_owned();
    }
    let statuses: Vec<&str> = body
        .split("<status>")
        .skip(1)
        .filter_map(|rest| {
            rest.split_once("</status>")
                .map(|(word, _)| short_word(word))
        })
        .collect();
    let count = body.matches("<task-notification>").count();
    let event = if body.contains("<event>") {
        ",event"
    } else {
        ""
    };
    format!(
        " task-notification(n={count},status=[{}]{event})",
        statuses.join(",")
    )
}
