//! Antigravity CLI sessions, from the files Wakeshot's own Antigravity hook writes in `agents\antigravity\`, and from
//! `agy`'s own log for the one end no hook reports, Esc and Ctrl+C (architecture section 7; `docs/agent-hooks.md`,
//! "Antigravity"; signals in `docs/research/antigravity-signals.md`). The table is what the hook and the source know
//! of Antigravity's hooks; the shared rules are in `sources::hooked`.
//!
//! Not started by the v1 tray, and its hook folder is not added, per "No guessing" in `docs/product.md`: Esc and
//! Ctrl+C fire no hook; only an undocumented log shows them, and `agy` releases almost daily, so it would need a
//! version lock. Kept, with its tests, for a later version.
//!
//! The exact rule: a chat works from a `PreInvocation` to its `Stop`, to a cancel line for it in that `agy.exe`'s log
//! at or after the hook's start, or to that process's end. Background commands are not held: no hook or line marks
//! their end exactly in every case (`fullyIdle` is ignored).
//! A log that does not look as tested (lines 1 and 2, the version, a cancel line's shape) or that no hook could name
//! holds nothing and shows "update Wakeshot": the source never guesses. Only the CLI is held; the IDE's hub runs the
//! same hooks, and they write nothing for it.
//!
//! Privacy: the log holds every prompt in plain text. Its bytes pass through memory once; the source keeps only the
//! pid of line 1, whether line 2 names a tested version, whether a line broke the tested shape, and per cancel line
//! the chat id and the line's time. No other byte is kept, and no line text leaves this module.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Path, PathBuf};

use crate::sources::cron::LocalOffset;
use crate::sources::hooked::{
    Action, Agent, Base, EventRule, HookedSource, ImageRule, InputFields, State, StateFile, Under,
    chat_id,
};
use crate::sources::{
    Activity, AgentSource, Fed, Feed, FileId, FormatHealth, SessionFacts, SourceId, WantedFile,
};

/// What the tooltip names a session of the IDE (its hub) or of any `agy.exe` outside the CLI's folder, and
/// Antigravity's plain name.
pub const LABEL: &str = "Antigravity";

/// What the tooltip names a session of the Antigravity CLI.
pub const CLI_LABEL: &str = "Antigravity CLI";

/// The CLI's image rule key, the one image Wakeshot holds for in v1.
const CLI_KEY: &str = "cli";

/// The CLI's install folder, `%LocalAppData%\agy\bin\` (its install script, `research/antigravity-signals.md`). The
/// IDE runs its own copy from `%USERPROFILE%\.gemini\bin\`.
const CLI_FOLDER: Under = Under {
    base: Base::LocalAppData,
    parts: &["agy", "bin"],
};

/// Antigravity's table (`docs/agent-hooks.md`, "Antigravity"). `agy` 1.5.2 runs each hook as `cmd /c <command>` with
/// the input on stdin, and the input has no event name, turn id or version (`research.md`, "Antigravity hook payload
/// form").
pub static AGENT: Agent = Agent {
    source: SourceId::Antigravity,
    name: "antigravity",
    label: LABEL,
    fields: InputFields {
        chat: "conversationId",
        turn: None,
        event: None,
        version: None,
        // `fullyIdle` says nothing exact about background commands, so it is not read.
        idle: None,
    },
    events: &[
        // Before every model call, so one turn runs it many times; each run notes when `agy` started it, so a cancel
        // line older than the latest call never ends the turn. A file a late end deleted comes back at the next call.
        EventRule {
            name: "PreInvocation",
            action: Action::Write(State::Working),
            prune: true,
        },
        // The loop ended. Background commands are not held, whatever `fullyIdle` says.
        EventRule {
            name: "Stop",
            action: Action::Delete,
            prune: false,
        },
    ],
    images: &[
        // First, so the CLI's own folder names its label before the rule for any `agy.exe`.
        ImageRule {
            image: "agy.exe",
            under: Some(CLI_FOLDER),
            key: CLI_KEY,
            label: CLI_LABEL,
        },
        // The IDE's hub and any other install: found, so the hook knows it ran there and writes nothing, rather than
        // walking on to a farther process.
        ImageRule {
            image: "agy.exe",
            under: None,
            key: "app",
            label: LABEL,
        },
    ],
    payload_key: "conversationId",
    input_problem: "Antigravity hook input",
    blocked_problem: "Antigravity hook command line",
};

/// Whether Wakeshot holds for the `agy.exe` an image rule found: only the CLI, until the IDE's own check passes.
pub fn holds(image: &ImageRule) -> bool {
    image.key == CLI_KEY
}

/// The `agy` versions whose log this rule was tested on. Any other shows "update Wakeshot" and holds nothing: a later
/// `agy` may drop or reword the cancel line, and a missing line cannot be seen.
const TESTED_VERSIONS: &[&str] = &["1.5.2"];

/// The `FormatHealth::Unreadable` text of a chat whose log is not as tested or not named.
pub const LOG_PROBLEM: &str = "Antigravity CLI log";

/// The most of a log the hook reads to find its pid: line 1 is about 100 bytes.
pub const HEAD_BYTES: usize = 512;

/// Where each `agy.exe` writes its log, below the profile folder (`os.UserHomeDir`, which reads `USERPROFILE`).
pub fn log_dir(home: &Path) -> PathBuf {
    home.join(".gemini").join("antigravity-cli").join("log")
}

/// `cli-<yyyymmdd>_<hhmmss>.log`, the one name shape `agy` 1.5.2 gives its logs. `cli.log` one folder up is a hard link
/// to the newest process's file, so it is never read.
pub fn is_log_name(name: &str) -> bool {
    let b = name.as_bytes();
    b.len() == 23
        && name.starts_with("cli-")
        && name.ends_with(".log")
        && b[12] == b'_'
        && b[4..12].iter().chain(&b[13..19]).all(u8::is_ascii_digit)
}

/// The pid line 1 of a log names, from its first `HEAD_BYTES` or fewer; `None` for any other first line.
pub fn head_pid(head: &[u8]) -> Option<u32> {
    let end = head.iter().position(|&b| b == b'\n')?;
    glog(&head[..=end]).and_then(|line| pid_of(line.msg))
}

const PID_LINE: &[u8] = b"Starting language server process with pid ";
const VERSION_LINE: &[u8] = b"Language server version: ";
const CANCEL_LINE: &[u8] = b"Cancelling conversation ";
/// What follows the chat id in the cancel lines seen; nothing at all is accepted too.
const CANCEL_ENDS: [&[u8]; 3] = [
    b"",
    b" (killBackgroundTasks=false)",
    b" (killBackgroundTasks=true)",
];

fn pid_of(msg: &[u8]) -> Option<u32> {
    let digits = msg.strip_prefix(PID_LINE)?;
    let plain = (1..=10).contains(&digits.len()) && digits.iter().all(u8::is_ascii_digit);
    plain
        .then(|| std::str::from_utf8(digits).ok()?.parse().ok())
        .flatten()
}

/// One glog line of `agy`'s log, `Lmmdd hh:mm:ss.uuuuuu <thread> <file>:<line>] <msg>`: its level, local date and
/// time, and its message. `None` for any other line, such as the second line of a prompt.
struct Glog<'a> {
    level: u8,
    stamp: Stamp,
    msg: &'a [u8],
}

/// A glog time: local, with no year and no zone, as `agy` wrote it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Stamp {
    month: u8,
    day: u8,
    micros_of_day: i64,
}

fn number(digits: &[u8]) -> Option<i64> {
    digits.iter().try_fold(0i64, |n, &b| {
        b.is_ascii_digit().then(|| n * 10 + i64::from(b - b'0'))
    })
}

fn glog(line: &[u8]) -> Option<Glog<'_>> {
    let line = line.strip_suffix(b"\n").unwrap_or(line);
    let line = line.strip_suffix(b"\r").unwrap_or(line);
    let head = line.get(..21)?;
    let level = head[0];
    let fixed = matches!(level, b'I' | b'W' | b'E' | b'F')
        && head[5] == b' '
        && head[8] == b':'
        && head[11] == b':'
        && head[14] == b'.';
    if !fixed {
        return None;
    }
    let month = number(&head[1..3])?;
    let day = number(&head[3..5])?;
    let (hour, minute, second) = (
        number(&head[6..8])?,
        number(&head[9..11])?,
        number(&head[12..14])?,
    );
    let micros = number(&head[15..21])?;
    let in_range = (1..=12).contains(&month)
        && (1..=31).contains(&day)
        && hour < 24
        && minute < 60
        && second < 60;
    if !in_range {
        return None;
    }
    // Then ` +<thread> <file>:<line>] <msg>`.
    let rest = &line[21..];
    let thread_at = rest.iter().position(|&b| b != b' ').filter(|&at| at > 0)?;
    let rest = &rest[thread_at..];
    let space = rest.iter().position(|&b| b == b' ')?;
    if space == 0 || !rest[..space].iter().all(u8::is_ascii_digit) {
        return None;
    }
    let rest = &rest[space + 1..];
    let close = rest.windows(2).position(|w| w == b"] ")?;
    let place = &rest[..close];
    let colon = place.iter().rposition(|&b| b == b':')?;
    let place_ok = colon > 0
        && colon + 1 < place.len()
        && !place.contains(&b' ')
        && place[colon + 1..].iter().all(u8::is_ascii_digit);
    if !place_ok {
        return None;
    }
    Some(Glog {
        level,
        stamp: Stamp {
            month: month as u8,
            day: day as u8,
            micros_of_day: ((hour * 60 + minute) * 60 + second) * 1_000_000 + micros,
        },
        msg: &rest[close + 2..],
    })
}

/// What a cancel message says: the chat it names, or `Err` for a message that starts as one but has another shape.
fn cancelled_chat(msg: &[u8]) -> Option<Result<String, ()>> {
    let rest = msg.strip_prefix(CANCEL_LINE)?;
    let chat = rest
        .get(..36)
        .and_then(|id| std::str::from_utf8(id).ok())
        .and_then(chat_id);
    Some(match chat {
        Some(chat) if CANCEL_ENDS.contains(&&rest[36..]) => Ok(chat),
        _ => Err(()),
    })
}

/// What the source keeps of one log: the facts of its first two lines and the last cancel line per chat. Never any
/// other byte of a line.
#[derive(Debug, Default)]
struct LogRead {
    /// Lines 1 and 2 are the head; later lines are only looked at for a cancel.
    lines: u8,
    pid: Option<u32>,
    tested: bool,
    /// A line broke the tested shape, so the log is no longer the one this rule was built on.
    odd: bool,
    cancels: BTreeMap<String, Stamp>,
}

impl LogRead {
    fn read(&mut self, bytes: &[u8]) {
        for line in bytes.split_inclusive(|&b| b == b'\n') {
            self.line(line);
        }
    }

    fn line(&mut self, line: &[u8]) {
        let parsed = glog(line);
        match self.lines {
            0 => {
                self.lines = 1;
                self.pid = parsed.and_then(|g| pid_of(g.msg));
                self.odd |= self.pid.is_none();
            }
            1 => {
                self.lines = 2;
                let version = parsed.and_then(|g| g.msg.strip_prefix(VERSION_LINE));
                self.tested =
                    version.is_some_and(|v| TESTED_VERSIONS.iter().any(|t| t.as_bytes() == v));
                self.odd |= version.is_none();
            }
            _ => {
                let Some(line) = parsed else {
                    return;
                };
                match cancelled_chat(line.msg) {
                    Some(Ok(chat)) if line.level == b'I' => {
                        self.cancels.insert(chat, line.stamp);
                    }
                    Some(_) => self.odd = true,
                    None => {}
                }
            }
        }
    }
}

/// What a working chat file means once its log is read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Turn {
    Working,
    /// A cancel line for the chat at or after the hook's start: Esc or Ctrl+C ended the turn.
    Cancelled,
    /// No log named, or one not as tested: no signal, so no hold.
    Unknown,
}

/// The Antigravity CLI's source: the hooked files, plus the log each working chat's file names.
pub struct AntigravitySource {
    chats: HookedSource,
    /// `None` when the profile folder is unknown: then no log is read, and no chat holds.
    log_dir: Option<PathBuf>,
    zone: Box<dyn LocalOffset + Send>,
    logs: BTreeMap<String, LogRead>,
}

impl fmt::Debug for AntigravitySource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AntigravitySource")
            .field("chats", &self.chats)
            .field("log_dir", &self.log_dir)
            .field("logs", &self.logs)
            .finish_non_exhaustive()
    }
}

/// Antigravity's source: `data_root` is Wakeshot's data folder, the one that holds `agents`; `home` the profile
/// folder `agy` writes its log under; `zone` turns the hook's start into the local time the log's lines carry.
pub fn source(
    data_root: &Path,
    home: Option<&Path>,
    zone: Box<dyn LocalOffset + Send>,
) -> AntigravitySource {
    AntigravitySource {
        chats: HookedSource::new(&AGENT, data_root),
        log_dir: home.map(log_dir),
        zone,
        logs: BTreeMap::new(),
    }
}

/// Microseconds from 1601-01-01 (the FILETIME start) to 1970-01-01.
const FILETIME_TO_UNIX_US: i64 = 11_644_473_600_000_000;
const DAY_US: i64 = 86_400_000_000;

/// Days from 1970-01-01 to a date of the proleptic Gregorian calendar (Howard Hinnant's `days_from_civil`).
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let month_from_march = (month + 9) % 12;
    let day_of_year = (153 * month_from_march + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// The year of a day counted from 1970-01-01 (the inverse's year part).
fn year_of(days: i64) -> i64 {
    let days = days + 719_468;
    let era = days.div_euclid(146_097);
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_from_march = (5 * day_of_year + 2) / 153;
    year_of_era + era * 400 + i64::from(month_from_march >= 10)
}

impl AntigravitySource {
    fn log_name(&self, file: &FileId) -> Option<String> {
        let dir = self.log_dir.as_deref()?;
        if file.0.parent() != Some(dir) {
            return None;
        }
        let name = file.0.file_name()?.to_str()?;
        is_log_name(name).then(|| name.to_owned())
    }

    /// The logs the working CLI chats name; each is tailed while one does.
    fn wanted_logs(&self) -> BTreeSet<String> {
        self.chats
            .chats()
            .filter(|(_, file)| file.label == CLI_LABEL && file.state == State::Working)
            .filter_map(|(_, file)| file.log.clone())
            .filter(|name| is_log_name(name))
            .collect()
    }

    /// The hook's start as a local time in microseconds, the clock `agy`'s glog lines use.
    fn local_us(&self, turn_at: u64) -> i64 {
        let utc_us = i64::try_from(turn_at / 10).unwrap_or(i64::MAX) - FILETIME_TO_UNIX_US;
        let offset = self.zone.offset_minutes(utc_us.div_euclid(1_000));
        utc_us + i64::from(offset) * 60_000_000
    }

    fn turn(&self, chat: &str, file: &StateFile) -> Turn {
        let (Some(name), Some(turn_at), Some(_)) = (&file.log, file.turn_at, &self.log_dir) else {
            return Turn::Unknown;
        };
        let Some(log) = self.logs.get(name) else {
            // Not read yet: the watch polls a new tail in the same run as the feed that asked for it.
            return Turn::Working;
        };
        if log.lines < 2 || log.odd || !log.tested || log.pid != Some(file.pid) {
            return Turn::Unknown;
        }
        let Some(stamp) = log.cancels.get(chat) else {
            return Turn::Working;
        };
        let start = self.local_us(turn_at.0);
        // glog writes no year: the one that puts the line nearest the hook's start.
        let year = year_of(start.div_euclid(DAY_US));
        let cancelled = (year - 1..=year + 1)
            .map(|y| {
                days_from_civil(y, i64::from(stamp.month), i64::from(stamp.day)) * DAY_US
                    + stamp.micros_of_day
            })
            .min_by_key(|at| (at - start).abs())
            .unwrap_or(i64::MIN);
        if cancelled >= start {
            Turn::Cancelled
        } else {
            Turn::Working
        }
    }
}

impl AgentSource for AntigravitySource {
    fn id(&self) -> SourceId {
        SourceId::Antigravity
    }

    fn wanted(&self) -> Vec<WantedFile> {
        let mut wants = self.chats.wanted();
        if let Some(dir) = &self.log_dir {
            wants.extend(
                self.wanted_logs()
                    .into_iter()
                    .map(|name| WantedFile::Tail(dir.join(name))),
            );
        }
        wants
    }

    fn feed(&mut self, file: &FileId, data: Feed) -> Fed {
        if let Some(name) = self.log_name(file) {
            // Only a log a working chat names is read: no other log's facts are kept.
            if !self.wanted_logs().contains(&name) {
                return Fed::Taken;
            }
            match data {
                Feed::Appended(bytes) => self.logs.entry(name).or_default().read(&bytes),
                // Gone or restarted: a tail that comes back starts again from byte 0.
                Feed::Gone => {
                    self.logs.remove(&name);
                }
                Feed::Whole(_) | Feed::Present => {}
            }
            return Fed::Taken;
        }
        let fed = self.chats.feed(file, data);
        // A log no chat names is no longer tailed; if one names it again, its tail starts from byte 0.
        let wanted = self.wanted_logs();
        self.logs.retain(|name, _| wanted.contains(name));
        fed
    }

    fn sessions(&self) -> Vec<SessionFacts> {
        let mut sessions = self.chats.sessions();
        sessions.retain_mut(|facts| {
            let Some(file) = self.chats.chat(&facts.key.id) else {
                // `problem.json` and `blocked.json`: "update Wakeshot" while their process lives.
                return true;
            };
            if facts.label != CLI_LABEL {
                return false;
            }
            facts.open_tasks = 0;
            let turn = match file.state {
                State::Working => self.turn(&facts.key.id, file),
                // An earlier build's background file: background commands are not held.
                State::Background => Turn::Cancelled,
            };
            (facts.activity, facts.format) = match turn {
                Turn::Working => (Activity::Working, FormatHealth::Ok),
                Turn::Cancelled => (Activity::Idle, FormatHealth::Ok),
                Turn::Unknown => (Activity::Idle, FormatHealth::Unreadable(LOG_PROBLEM)),
            };
            true
        });
        sessions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_civil_day_count_matches_known_dates() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(days_from_civil(2026, 11, 6), 20_763);
        assert_eq!(days_from_civil(2000, 2, 29), 11_016);
        for days in [-1, 0, 59, 11_016, 20_398, 20_453, 20_454] {
            let year = year_of(days);
            assert!(days_from_civil(year, 1, 1) <= days && days < days_from_civil(year + 1, 1, 1));
        }
        assert_eq!(year_of(20_453), 2025, "2025-12-31");
        assert_eq!(year_of(20_454), 2026, "2026-01-01");
    }

    #[test]
    fn a_glog_line_gives_its_time_and_message_and_nothing_else_parses() {
        let line = b"I1106 18:54:48.718882      51 server.go:2275] Starting language server process with pid 51384\n";
        let g = glog(line).unwrap();
        assert_eq!(g.level, b'I');
        assert_eq!(
            g.stamp,
            Stamp {
                month: 11,
                day: 6,
                micros_of_day: (18 * 3600 + 54 * 60 + 48) * 1_000_000 + 718_882,
            }
        );
        assert_eq!(pid_of(g.msg), Some(51_384));
        assert_eq!(head_pid(line), Some(51_384));
        assert_eq!(head_pid(&line[..line.len() - 1]), None, "no whole line");
        for bad in [
            &b"HandleUserInput called with text: hi\n"[..],
            b"I1106 18:54:48.718882 51 server.go:2275 Starting\n",
            b"I1306 18:54:48.718882 51 server.go:2275] x\n",
            b"I1106 24:54:48.718882 51 server.go:2275] x\n",
            b"X1106 18:54:48.718882 51 server.go:2275] x\n",
            b"I1106 18:54:48.718882 x server.go:2275] x\n",
            b"I1106 18:54:48.718882 51 server.go] x\n",
            b"I1106 18:54:48.71888 51 server.go:1] x\n",
            b"I1106 18:54:48.718882 51 server.go:x1] x\n",
            b"I1106 18:54:48.718882 51  server.go:1] x\n",
        ] {
            assert!(glog(bad).is_none(), "{}", String::from_utf8_lossy(bad));
        }
        assert_eq!(
            head_pid(b"I1106 18:54:48.718882 51 server.go:2275] Starting language server process with pid 5138x\n"),
            None
        );
    }

    #[test]
    fn only_the_tested_cancel_shapes_name_a_chat() {
        let id = "1ad3ac30-4354-4e1e-a5d3-d338da829975";
        for tail in [
            "",
            " (killBackgroundTasks=false)",
            " (killBackgroundTasks=true)",
        ] {
            let msg = format!("Cancelling conversation {}{tail}", id.to_uppercase());
            assert_eq!(
                cancelled_chat(msg.as_bytes()),
                Some(Ok(id.to_owned())),
                "{tail}"
            );
        }
        for odd in [
            format!("Cancelling conversation {id} (killBackgroundTasks=maybe)"),
            format!("Cancelling conversation {id}x"),
            "Cancelling conversation 1ad3ac30".to_owned(),
            "Cancelling conversation ../../../x-4354-4e1e-a5d3-d338da829975".to_owned(),
        ] {
            assert_eq!(cancelled_chat(odd.as_bytes()), Some(Err(())), "{odd}");
        }
        for other in [
            format!("Cancelling in-progress response for conversation {id}"),
            format!("Cancelling steps [6] in conversation {id}"),
            format!("HandleUserInput called with text: Cancelling conversation {id}"),
        ] {
            assert_eq!(cancelled_chat(other.as_bytes()), None, "{other}");
        }
    }

    #[test]
    fn the_log_name_is_agys_own_shape() {
        assert!(is_log_name("cli-20261106_185448.log"));
        for bad in [
            "cli.log",
            "cli-20261106_185448.txt",
            "cli-2026110_1854481.log",
            "cli-20261106-185448.log",
            "CLI-20261106_185448.log",
            "cli-2026110x_185448.log",
            r"..\cli-20261106_185448.log",
            "cli-20261106_185448.log ",
        ] {
            assert!(!is_log_name(bad), "{bad}");
        }
    }
}
