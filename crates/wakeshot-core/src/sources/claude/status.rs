//! Claude Code's `sessions\<pid>.json` status files.
//!
//! Owner: `p4-claude-status`; fixes by `p4-live-claude`.
//!
//! Rules 1 to 4 of `docs/research/claude-code-signals.md`. Claude Code writes these files with a plain truncate and
//! write, so a read can catch an empty or half-written file, and leftover bytes after a whole object have been seen
//! in the wild (issue anthropics/claude-code#96438): only the first JSON value counts. Only the keys of `Raw` are
//! read; serde skips every other key (`cwd`, `name`, the pipe name) without keeping it.

use serde::Deserialize;
use serde_json::Value;

use crate::process::FileTime;
use crate::sources::Activity;

/// The most characters of an unknown `status` value passed on, so a strange file cannot put much text in the log.
pub const UNKNOWN_VALUE_CHARS: usize = 32;

/// The longest `sessionId` kept. Claude's are 36-character UUIDs.
const SESSION_ID_MAX: usize = 64;

/// The pid a status file name gives: `<digits>.json` only, so `<pid>.<hash>.key` files (a peer token), the
/// `.fleetview-heartbeat` file and temp names never pass and are never opened.
pub fn status_file_pid(name: &str) -> Option<u32> {
    let stem = name.strip_suffix(".json")?;
    // Claude names the file `${process.pid}.json`. Refusing a leading zero keeps one name per pid, and keeps out pid
    // 0, the System Idle Process, which never ends and so would hold forever.
    if stem.is_empty() || stem.starts_with('0') || !stem.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    stem.parse().ok()
}

/// The `accept` filter of the sessions folder (`WantedFile::Folder`), run on bare names before any open.
pub fn is_status_file_name(name: &str) -> bool {
    status_file_pid(name).is_some()
}

/// Which client started the session. Only terminal sessions write `shell`; the others show a background shell as
/// `idle`, so their transcript must show it (rule 3, `p4-claude-transcript`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Entrypoint {
    /// `cli`: the terminal.
    Cli,
    /// Any other value: `sdk-cli`, `claude-vscode`, `claude-desktop`, and ones not seen yet.
    Other,
}

/// `kind`, from `CLAUDE_CODE_SESSION_KIND`. No fact depends on it yet; it is read for later rules on `bg` and
/// daemon sessions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Interactive,
    Background,
    Daemon,
    DaemonWorker,
    Other,
}

/// What Wakeshot keeps of one status file. Nothing else is kept: `waitingFor`, `statusUpdatedAt` and
/// `bridgeSessionId` are read only to make `activity`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatusFile {
    pub pid: u32,
    /// `sessionId`, the transcript's file name, kept only when it is a plain id (ASCII letters, digits, `-`, `_`; at
    /// most 64), so it can never turn a transcript search into a path elsewhere.
    pub session_id: Option<String>,
    /// A `sessionId` that is there but not a plain id, as `<string N>` (N bytes) or its JSON type in angle brackets:
    /// its transcript cannot be found, so its timers and tasks are not known (review m5).
    pub session_id_problem: Option<String>,
    /// `procStartFt`, else `procStart`, as Claude's own readers take it.
    pub proc_start: FileTime,
    pub activity: Activity,
    /// A `status` value Wakeshot does not know, cut to [`UNKNOWN_VALUE_CHARS`] ASCII characters; for a value that is
    /// not text, its JSON type in angle brackets. The session then counts as working while its process lives.
    pub unknown_status: Option<String>,
    pub entrypoint: Option<Entrypoint>,
    pub kind: Option<Kind>,
}

/// Why a status file gives no facts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Problem {
    /// Not one whole JSON object: empty, half written, or not JSON. A read 200 ms later may find it whole.
    NotParsed,
    /// `pid` is missing, not a pid, or not the pid in the file name.
    Pid,
    /// No usable `procStart`, so the file cannot be tied to its process.
    ProcStart,
}

impl Problem {
    /// The text for `FormatHealth::Unreadable`: fixed, so file content never reaches the log.
    pub fn what(self) -> &'static str {
        match self {
            Problem::NotParsed => "status file",
            Problem::Pid => "status file pid",
            Problem::ProcStart => "status file procStart",
        }
    }
}

/// The only keys read. A JSON `null` reads as missing, as for `bridgeSessionId` after Remote Control disconnects.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Raw {
    pid: Option<Value>,
    session_id: Option<Value>,
    proc_start: Option<Value>,
    proc_start_ft: Option<Value>,
    status: Option<Value>,
    waiting_for: Option<Value>,
    status_updated_at: Option<Value>,
    bridge_session_id: Option<Value>,
    entrypoint: Option<Value>,
    kind: Option<Value>,
}

/// Reads one status file whose name gave `name_pid`.
pub fn parse(name_pid: u32, bytes: &[u8]) -> Result<StatusFile, Problem> {
    let raw = first_object(bytes).ok_or(Problem::NotParsed)?;
    let pid = raw
        .pid
        .as_ref()
        .and_then(Value::as_u64)
        .and_then(|pid| u32::try_from(pid).ok());
    if pid != Some(name_pid) {
        return Err(Problem::Pid);
    }
    // No process starts at the FILETIME epoch (1601); a zero would match only pids that never end.
    let proc_start = raw
        .proc_start_ft
        .or(raw.proc_start)
        .and_then(|value| FileTime::deserialize(value).ok())
        .filter(|start| start.0 != 0)
        .ok_or(Problem::ProcStart)?;
    let (activity, unknown_status) = activity(
        raw.status,
        raw.waiting_for.as_ref(),
        raw.status_updated_at.as_ref(),
        raw.bridge_session_id.as_ref(),
    );
    let (session_id, session_id_problem) = match raw.session_id.map(session_id) {
        None => (None, None),
        Some(Ok(id)) => (Some(id), None),
        Some(Err(problem)) => (None, Some(problem)),
    };
    Ok(StatusFile {
        pid: name_pid,
        session_id,
        session_id_problem,
        proc_start,
        activity,
        unknown_status,
        entrypoint: raw.entrypoint.as_ref().and_then(Value::as_str).map(|e| {
            if e == "cli" {
                Entrypoint::Cli
            } else {
                Entrypoint::Other
            }
        }),
        kind: raw
            .kind
            .as_ref()
            .and_then(Value::as_str)
            .map(|kind| match kind {
                "interactive" => Kind::Interactive,
                "bg" => Kind::Background,
                "daemon" => Kind::Daemon,
                "daemon-worker" => Kind::DaemonWorker,
                _ => Kind::Other,
            }),
    })
}

fn first_object(bytes: &[u8]) -> Option<Raw> {
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    // serde also fills a struct from a JSON array, so the first value must open with a brace to be an object.
    let start = bytes
        .iter()
        .position(|b| !matches!(b, b' ' | b'\t' | b'\n' | b'\r'))?;
    if bytes[start] != b'{' {
        return None;
    }
    serde_json::Deserializer::from_slice(&bytes[start..])
        .into_iter::<Raw>()
        .next()?
        .ok()
}

/// Rules 3 and 4: `busy` and `shell` work; `waiting` waits, except on a slash command dialog; `idle` and no status
/// yet are idle; any other value works and is named, so "update Wakeshot" shows.
fn activity(
    status: Option<Value>,
    waiting_for: Option<&Value>,
    status_updated_at: Option<&Value>,
    bridge_session_id: Option<&Value>,
) -> (Activity, Option<String>) {
    let text = match status {
        None => return (Activity::Idle, None),
        Some(Value::String(text)) => text,
        Some(other) => return (Activity::Working, Some(type_name(&other).to_owned())),
    };
    match text.as_str() {
        "busy" | "shell" => (Activity::Working, None),
        "idle" => (Activity::Idle, None),
        "waiting" if waiting_for.and_then(Value::as_str) == Some("dialog open") => {
            (Activity::Idle, None)
        }
        // A `waitingFor` not seen yet still waits: the core caps a wait at one hour, while an unknown value would
        // count as working for the life of the process.
        "waiting" => (
            Activity::Waiting {
                since_wall_ms: status_updated_at.and_then(wall_ms),
                remote: matches!(bridge_session_id, Some(Value::String(_))),
            },
            None,
        ),
        other => (Activity::Working, Some(cut(other))),
    }
}

fn wall_ms(value: &Value) -> Option<i64> {
    // Claude writes `Date.now()`, an integer, but writes its timer times as floats; a float here is cut to whole ms
    // rather than lost.
    value.as_i64().or_else(|| {
        value
            .as_f64()
            .filter(|ms| ms.is_finite() && ms.abs() < 9.0e15)
            .map(|ms| ms.floor() as i64)
    })
}

fn cut(value: &str) -> String {
    value
        .chars()
        .take(UNKNOWN_VALUE_CHARS)
        .map(|c| {
            if c.is_ascii_graphic() || c == ' ' {
                c
            } else {
                '?'
            }
        })
        .collect()
}

fn type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "<null>",
        Value::Bool(_) => "<bool>",
        Value::Number(_) => "<number>",
        Value::String(_) => "<string>",
        Value::Array(_) => "<array>",
        Value::Object(_) => "<object>",
    }
}

/// The id, or how it is shown when it is not a plain one: never its text, which could be anything.
fn session_id(value: Value) -> Result<String, String> {
    match value {
        Value::String(id)
            if (1..=SESSION_ID_MAX).contains(&id.len())
                && id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_') =>
        {
            Ok(id)
        }
        Value::String(id) => Err(format!("<string {}>", id.len())),
        other => Err(type_name(&other).to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    const PID: u32 = 4242;
    const START: FileTime = FileTime(134_381_285_374_949_759);

    /// A status file of `PID` with `procStart` and the given extra fields.
    fn file(fields: &str) -> String {
        let comma = if fields.is_empty() { "" } else { "," };
        format!(r#"{{"pid":4242,"procStart":"134381285374949759"{comma}{fields}}}"#)
    }

    fn read(fields: &str) -> StatusFile {
        parse(PID, file(fields).as_bytes()).unwrap()
    }

    fn activity_of(fields: &str) -> (Activity, Option<String>) {
        let status = read(fields);
        (status.activity, status.unknown_status)
    }

    fn waiting(since_wall_ms: Option<i64>, remote: bool) -> (Activity, Option<String>) {
        (
            Activity::Waiting {
                since_wall_ms,
                remote,
            },
            None,
        )
    }

    #[test]
    fn busy_and_shell_are_working() {
        for status in ["busy", "shell"] {
            assert_eq!(
                activity_of(&format!(r#""status":"{status}""#)),
                (Activity::Working, None),
                "{status}"
            );
        }
    }

    /// Before the first status write there is none; `null` reads the same.
    #[test]
    fn idle_and_no_status_are_idle() {
        for fields in [r#""status":"idle""#, "", r#""status":null"#] {
            assert_eq!(activity_of(fields), (Activity::Idle, None), "{fields}");
        }
    }

    #[test]
    fn waiting_carries_its_time_and_remote_control() {
        let at = r#""status":"waiting","waitingFor":"permission prompt","statusUpdatedAt":1793655699331"#;
        assert_eq!(
            activity_of(&format!(r#"{at},"bridgeSessionId":"BRIDGE1""#)),
            waiting(Some(1_793_655_699_331), true)
        );
        // Remote Control off writes `null`; SDK sessions have no key at all.
        assert_eq!(
            activity_of(&format!(r#"{at},"bridgeSessionId":null"#)),
            waiting(Some(1_793_655_699_331), false)
        );
        assert_eq!(activity_of(at), waiting(Some(1_793_655_699_331), false));
        // Only a string is a Remote Control session id.
        for bridge in ["true", "1", "{}"] {
            assert_eq!(
                activity_of(&format!(r#"{at},"bridgeSessionId":{bridge}"#)),
                waiting(Some(1_793_655_699_331), false),
                "{bridge}"
            );
        }
    }

    /// The parser has no clock: no usable `statusUpdatedAt` leaves the time to the core.
    #[test]
    fn waiting_without_a_time_leaves_it_to_the_core() {
        let wait = r#""status":"waiting","waitingFor":"input needed""#;
        assert_eq!(activity_of(wait), waiting(None, false));
        for at in ["null", r#""1793655699331""#, "[]", "1e300"] {
            assert_eq!(
                activity_of(&format!(r#"{wait},"statusUpdatedAt":{at}"#)),
                waiting(None, false),
                "{at}"
            );
        }
        assert_eq!(
            activity_of(&format!(r#"{wait},"statusUpdatedAt":1793655699331.75"#)),
            waiting(Some(1_793_655_699_331), false)
        );
    }

    #[test]
    fn dialog_open_is_idle() {
        let fields = r#""status":"waiting","waitingFor":"dialog open","statusUpdatedAt":1,"bridgeSessionId":"s""#;
        assert_eq!(activity_of(fields), (Activity::Idle, None));
    }

    /// Every reason in the code, a missing one, and one not seen yet: all wait, none is a format problem.
    #[test]
    fn every_other_waiting_reason_waits() {
        for reason in [
            r#","waitingFor":"permission prompt""#,
            r#","waitingFor":"input needed""#,
            r#","waitingFor":"worker request""#,
            r#","waitingFor":"sandbox request""#,
            r#","waitingFor":"goal proposal""#,
            r#","waitingFor":"Dialog Open""#,
            r#","waitingFor":"a reason not seen yet""#,
            r#","waitingFor":7"#,
            "",
        ] {
            assert_eq!(
                activity_of(&format!(
                    r#""status":"waiting","statusUpdatedAt":5{reason}"#
                )),
                waiting(Some(5), false),
                "{reason}"
            );
        }
    }

    #[test]
    fn an_unknown_status_is_working_and_named() {
        for value in ["compacting", "Busy", "IDLE", "", " waiting"] {
            assert_eq!(
                activity_of(&format!(r#""status":"{value}""#)),
                (Activity::Working, Some(value.to_owned())),
                "{value:?}"
            );
        }
    }

    #[test]
    fn an_unknown_status_is_cut_to_32_ascii_characters() {
        let long = "a".repeat(40);
        assert_eq!(
            activity_of(&format!(r#""status":"{long}""#)).1,
            Some("a".repeat(32))
        );
        assert_eq!(
            activity_of(r#""status":"d\u00e9j\u00e0\tvu\n""#).1,
            Some("d?j??vu?".to_owned())
        );
        assert_eq!(
            activity_of(r#""status":"\u0531\u0532\u0533""#).1,
            Some("???".to_owned())
        );
    }

    #[test]
    fn a_status_that_is_not_text_is_working_and_named_by_its_type() {
        for (value, name) in [
            ("5", "<number>"),
            ("true", "<bool>"),
            (r#"["busy"]"#, "<array>"),
            (r#"{"busy":true}"#, "<object>"),
        ] {
            assert_eq!(
                activity_of(&format!(r#""status":{value}"#)),
                (Activity::Working, Some(name.to_owned())),
                "{value}"
            );
        }
    }

    #[test]
    fn pid_must_be_the_pid_in_the_name() {
        assert_eq!(read("").pid, PID);
        assert_eq!(parse(PID + 1, file("").as_bytes()), Err(Problem::Pid));
        for text in [
            r#"{"procStart":"134381285374949759","status":"busy"}"#,
            r#"{"pid":"4242","procStart":"134381285374949759"}"#,
            r#"{"pid":4242.0,"procStart":"134381285374949759"}"#,
            r#"{"pid":-4242,"procStart":"134381285374949759"}"#,
            r#"{"pid":null,"procStart":"134381285374949759"}"#,
        ] {
            assert_eq!(parse(PID, text.as_bytes()), Err(Problem::Pid), "{text}");
        }
        let big = r#"{"pid":4294971538,"procStart":"134381285374949759"}"#;
        assert_eq!(parse(PID, big.as_bytes()), Err(Problem::Pid));
    }

    #[test]
    fn proc_start_ties_the_file_to_its_process() {
        let with = |fields: &str| parse(PID, format!(r#"{{"pid":4242{fields}}}"#).as_bytes());
        assert_eq!(read("").proc_start, START);
        // `procStartFt ?? procStart`, as Claude's own readers do (`claude-code-signals.md`, "Status file fields").
        assert_eq!(
            with(r#","procStart":"1","procStartFt":"2""#)
                .unwrap()
                .proc_start,
            FileTime(2)
        );
        assert_eq!(
            with(r#","procStart":"1","procStartFt":null"#)
                .unwrap()
                .proc_start,
            FileTime(1)
        );
        assert_eq!(
            with(r#","procStartFt":"3""#).unwrap().proc_start,
            FileTime(3)
        );
        for fields in [
            "",
            r#","procStart":null"#,
            r#","procStart":134381285374949759"#,
            r#","procStart":"""#,
            r#","procStart":"+1""#,
            r#","procStart":"0""#,
            r#","procStart":"99999999999999999999""#,
            r#","procStart":"1","procStartFt":"x""#,
        ] {
            assert_eq!(with(fields), Err(Problem::ProcStart), "{fields}");
        }
    }

    #[test]
    fn only_the_first_json_value_counts() {
        let busy = file(r#""status":"busy""#);
        for text in [
            format!(r#"{busy}us","statusUpdatedAt":1}}"#),
            format!(r#"{busy}{{"pid":1,"status":"idle"}}"#),
            format!("{busy}\r\n"),
            format!(" \r\n\t{busy}"),
            format!("\u{feff}{busy}"),
        ] {
            assert_eq!(
                parse(PID, text.as_bytes()).map(|s| s.activity),
                Ok(Activity::Working),
                "{text}"
            );
        }
    }

    #[test]
    fn what_is_not_one_whole_object_is_not_parsed() {
        let whole = file(r#""status":"idle","bridgeSessionId":"BRIDGE1""#);
        let mut bad: Vec<Vec<u8>> = (0..whole.len())
            .map(|n| whole.as_bytes()[..n].to_vec())
            .collect();
        for text in [
            " \r\n",
            "not json",
            "[4242]",
            "[]",
            // One value per key of `Raw`, in order: serde would read this array as a busy session.
            r#"[4242,null,"134381285374949759",null,"busy",null,null,null,null,null]"#,
            r#""status""#,
            "4242",
            "null",
            r#"{"pid":4242,"pid":4242,"procStart":"134381285374949759"}"#,
            "\u{feff}\u{feff}{}",
        ] {
            bad.push(text.as_bytes().to_vec());
        }
        bad.push(vec![0xFF, b'{', b'}']);
        for bytes in bad {
            assert_eq!(
                parse(PID, &bytes),
                Err(Problem::NotParsed),
                "{:?}",
                String::from_utf8_lossy(&bytes)
            );
        }
    }

    #[test]
    fn session_id_is_kept_only_as_a_plain_id() {
        let id_of = |value: &str| read(&format!(r#""sessionId":{value}"#)).session_id;
        let uuid = "5e550002-0000-4000-8000-000000000002";
        assert_eq!(id_of(&format!(r#""{uuid}""#)).as_deref(), Some(uuid));
        assert_eq!(id_of(r#""session_01AB""#).as_deref(), Some("session_01AB"));
        assert_eq!(read("").session_id, None);
        let long = format!(r#""{}""#, "a".repeat(SESSION_ID_MAX + 1));
        for value in [
            r#""""#,
            r#""..\\..\\x""#,
            r#""a/b""#,
            r#""a*""#,
            r#""a.jsonl""#,
            r#""\u00e9""#,
            "42",
            "null",
            long.as_str(),
        ] {
            assert_eq!(id_of(value), None, "{value}");
        }
        assert_eq!(
            id_of(&format!(r#""{}""#, "a".repeat(SESSION_ID_MAX))),
            Some("a".repeat(SESSION_ID_MAX))
        );
    }

    /// Review m5: a `sessionId` of a new shape is named by its length or type, never its text; a missing one is not.
    #[test]
    fn a_session_id_of_another_shape_is_named() {
        let problem_of = |value: &str| read(&format!(r#""sessionId":{value}"#)).session_id_problem;
        assert_eq!(problem_of(r#""a/b""#).as_deref(), Some("<string 3>"));
        assert_eq!(problem_of(r#""""#).as_deref(), Some("<string 0>"));
        assert_eq!(problem_of("42").as_deref(), Some("<number>"));
        assert_eq!(problem_of(r#"{"id":"x"}"#).as_deref(), Some("<object>"));
        let long = "a".repeat(SESSION_ID_MAX + 1);
        assert_eq!(
            problem_of(&format!(r#""{long}""#)),
            Some(format!("<string {}>", SESSION_ID_MAX + 1))
        );
        for plain_or_missing in [r#""5e550002-0000-4000-8000-000000000002""#, "null"] {
            assert_eq!(problem_of(plain_or_missing), None, "{plain_or_missing}");
        }
        assert_eq!(read("").session_id_problem, None);
    }

    #[test]
    fn entrypoint_and_kind() {
        let entry = |fields: &str| read(fields).entrypoint;
        assert_eq!(entry(r#""entrypoint":"cli""#), Some(Entrypoint::Cli));
        for value in [
            "sdk-cli",
            "claude-vscode",
            "claude-desktop",
            "local-agent",
            "CLI",
        ] {
            assert_eq!(
                entry(&format!(r#""entrypoint":"{value}""#)),
                Some(Entrypoint::Other),
                "{value}"
            );
        }
        assert_eq!(entry(""), None);
        assert_eq!(entry(r#""entrypoint":1"#), None);

        let kind = |fields: &str| read(fields).kind;
        for (value, expected) in [
            ("interactive", Kind::Interactive),
            ("bg", Kind::Background),
            ("daemon", Kind::Daemon),
            ("daemon-worker", Kind::DaemonWorker),
            ("teammate", Kind::Other),
        ] {
            assert_eq!(
                kind(&format!(r#""kind":"{value}""#)),
                Some(expected),
                "{value}"
            );
        }
        assert_eq!(kind(""), None);
    }

    /// Acceptance check: the parser keeps no field beyond the card's list. A new field breaks this destructuring, so
    /// adding one is a visible choice.
    #[test]
    fn the_status_file_keeps_only_the_listed_fields() {
        let StatusFile {
            pid,
            session_id,
            session_id_problem,
            proc_start,
            activity,
            unknown_status,
            entrypoint,
            kind,
        } = read(
            r#""sessionId":"5e550002-0000-4000-8000-000000000002","status":"busy","entrypoint":"cli","kind":"interactive","cwd":"C:\\Users\\USER","name":"secret","messagingSocketPath":"\\\\.\\pipe\\x""#,
        );
        assert_eq!(
            (pid, proc_start, activity, unknown_status, entrypoint, kind),
            (
                PID,
                START,
                Activity::Working,
                None,
                Some(Entrypoint::Cli),
                Some(Kind::Interactive)
            )
        );
        assert_eq!(
            session_id.as_deref(),
            Some("5e550002-0000-4000-8000-000000000002")
        );
        assert_eq!(session_id_problem, None);
    }

    #[test]
    fn accepts_only_pid_json() {
        assert_eq!(status_file_pid("20636.json"), Some(20636));
        assert_eq!(status_file_pid("3032.json"), Some(3032));
        assert_eq!(status_file_pid("4294967295.json"), Some(u32::MAX));
        for name in [
            ".fleetview-heartbeat",
            ".json",
            "abc.json",
            "20636.JSON",
            "20636.Json",
            "20636.json.tmp",
            "20636.json~",
            ".20636.json.swp",
            "20636.tmp",
            "20 636.json",
            " 20636.json",
            "20636.json ",
            "20636.json.",
            "+20636.json",
            "-20636.json",
            "0.json",
            "020636.json",
            "4294967296.json",
            "99999999999.json",
            "\u{0662}\u{0660}.json",
            "20636.json:stream",
            r"sessions\20636.json",
            "sessions/20636.json",
            "",
        ] {
            assert_eq!(status_file_pid(name), None, "{name:?}");
            assert!(!is_status_file_name(name), "{name:?}");
        }
    }

    /// Acceptance check: every form a `.key` name takes is refused before any open.
    #[test]
    fn refuses_every_key_name_form() {
        for name in [
            "20636.685bf0d758162e180716a9b47a33761d744d0f5da53275e73110abe1a22f559b.key",
            "20636.key",
            "20636.json.key",
            "20636.JSON.KEY",
            "20636.Key",
            ".key",
            "key",
            "20636.key.json",
            "20636.685bf0d7.key.json",
        ] {
            assert!(!is_status_file_name(name), "{name}");
        }
    }

    proptest! {
        #[test]
        fn no_key_name_passes(stem in "\\PC*", ext in "[kK][eE][yY]") {
            let name = format!("{stem}.{ext}");
            prop_assert!(!is_status_file_name(&name), "{}", name);
        }

        /// A name passes only in the one form Claude writes, so two names never give one pid.
        #[test]
        fn a_name_that_passes_is_the_pid_written_out(stem in "[0-9+ -]{0,12}", ext in "json|JSON|json ") {
            let name = format!("{stem}.{ext}");
            if let Some(pid) = status_file_pid(&name) {
                prop_assert_eq!(format!("{pid}.json"), name);
                prop_assert_ne!(pid, 0);
            }
        }

        #[test]
        fn every_pid_name_passes(pid in 1u32..) {
            prop_assert_eq!(status_file_pid(&format!("{pid}.json")), Some(pid));
        }

        #[test]
        fn any_bytes_read_without_a_panic(bytes in proptest::collection::vec(any::<u8>(), 0..256)) {
            if let Ok(status) = parse(PID, &bytes) {
                prop_assert_eq!(status.pid, PID);
            }
        }

        /// Review scope: no file content holds forever by itself. A read that gives facts names the pid of its file
        /// name and a real start time, so the session ends with that process.
        #[test]
        fn facts_are_always_tied_to_the_named_process(
            pid in prop_oneof![Just(u64::from(PID)), any::<u64>()],
            start in prop_oneof![Just("0".to_owned()), "[0-9]{1,21}", "\\PC{0,8}"],
            start_ft in proptest::option::of(prop_oneof![Just("0".to_owned()), "[0-9]{1,21}"]),
            status in "\\PC{0,40}",
        ) {
            let text = |value: &str| serde_json::to_string(value).unwrap();
            let mut json = format!(r#"{{"pid":{pid},"procStart":{},"status":{}"#, text(&start), text(&status));
            if let Some(start_ft) = &start_ft {
                json += &format!(r#","procStartFt":{}"#, text(start_ft));
            }
            json.push('}');
            if let Ok(read) = parse(PID, json.as_bytes()) {
                prop_assert_eq!(pid, u64::from(PID));
                prop_assert_ne!(read.proc_start, FileTime(0));
                let chosen = start_ft.unwrap_or(start);
                prop_assert_eq!(read.proc_start.0.to_string(), chosen.trim_start_matches('0'));
            }
        }

        #[test]
        fn any_status_text_is_known_or_short_ascii(value in "\\PC{0,64}") {
            let fields = format!(r#""status":{}"#, serde_json::to_string(&value).unwrap());
            let status = read(&fields);
            match status.unknown_status {
                Some(shown) => {
                    prop_assert_eq!(status.activity, Activity::Working);
                    prop_assert!(shown.chars().count() <= UNKNOWN_VALUE_CHARS);
                    prop_assert!(shown.bytes().all(|b| b.is_ascii_graphic() || b == b' '));
                }
                None => prop_assert!(["busy", "shell", "idle", "waiting"].contains(&value.as_str())),
            }
        }
    }
}
