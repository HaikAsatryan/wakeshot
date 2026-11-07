//! `ClaudeSource` on transcripts. The live Claude Code 2.1.317 sessions in `fixtures/claude/live/` are replayed with
//! their status writes in the order they happened, fed the way the watch thread feeds them, and each due time is
//! checked against the fire the session logged next and against Claude Code's own `CronList` answers. The hand-made
//! cases in `fixtures/claude/transcript/` cover what the live ones do not. The source does no I/O, so the home folder
//! is a made-up path and no real `~/.claude` is ever read.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use proptest::prelude::*;
use serde_json::{Value, json};
use wakeshot_core::holds::file::parse_expires;
use wakeshot_core::sources::claude::ClaudeSource;
use wakeshot_core::sources::cron::FixedOffset;
use wakeshot_core::sources::{
    Activity, AgentSource, Fed, Feed, FileId, FormatHealth, SessionFacts, WantedFile,
};

const HOME: &str = r"X:\home";
/// The test laptop at the live tests: UTC+4, no daylight saving.
const ZONE: i32 = 240;
const SECOND: i64 = 1000;

fn fixtures(dir: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("claude")
        .join(dir)
}

fn source() -> ClaudeSource {
    ClaudeSource::new(PathBuf::from(HOME), Box::new(FixedOffset(ZONE)))
}

fn projects() -> PathBuf {
    PathBuf::from(HOME).join(".claude").join("projects")
}

fn status_file(pid: u32) -> FileId {
    FileId(
        PathBuf::from(HOME)
            .join(".claude")
            .join("sessions")
            .join(format!("{pid}.json")),
    )
}

fn transcript_file(folder: &str, session_id: &str) -> FileId {
    FileId(projects().join(folder).join(format!("{session_id}.jsonl")))
}

/// `procStart` as wall ms, the resume divider.
fn wall_ms(proc_start: &str) -> i64 {
    let ticks: i128 = proc_start.parse().unwrap();
    i64::try_from((ticks - 116_444_736_000_000_000) / 10_000).unwrap()
}

fn time(value: &Value) -> Option<i64> {
    parse_expires(value["timestamp"].as_str()?)
}

/// One transcript file of a case.
struct Transcript {
    session_id: String,
    file: FileId,
    lines: Vec<Line>,
}

struct Line {
    /// When the line was surely on disk: the latest `timestamp` up to it, since lines are written in order.
    written: i64,
    bytes: Vec<u8>,
    value: Value,
}

#[derive(Clone)]
enum Step {
    Status { pid: u32, content: Option<Value> },
    Line { file: usize, line: usize },
}

/// A live case: its status writes and transcript lines in the order they happened.
struct Live {
    transcripts: Vec<Transcript>,
    steps: Vec<(i64, Step)>,
}

fn read_jsonl(path: &Path) -> Vec<(Vec<u8>, Value)> {
    fs::read_to_string(path)
        .unwrap()
        .lines()
        .map(|line| {
            let mut bytes = line.as_bytes().to_vec();
            bytes.push(b'\n');
            (bytes, serde_json::from_str(line).unwrap())
        })
        .collect()
}

fn transcript(path: &Path, folder: &str) -> Transcript {
    let lines = read_jsonl(path);
    let session_id = lines
        .iter()
        .find_map(|(_, value)| value["sessionId"].as_str())
        .unwrap()
        .to_owned();
    let first = lines.iter().find_map(|(_, value)| time(value)).unwrap();
    let mut written = first;
    let lines = lines
        .into_iter()
        .map(|(bytes, value)| {
            written = written.max(time(&value).unwrap_or(written));
            Line {
                written,
                bytes,
                value,
            }
        })
        .collect();
    Transcript {
        file: transcript_file(folder, &session_id),
        session_id,
        lines,
    }
}

fn live(name: &str) -> Live {
    let dir = fixtures("live");
    let mut steps = Vec::new();
    for (_, value) in read_jsonl(&dir.join(format!("{name}.status.jsonl"))) {
        let Some(file) = value["file"].as_str().unwrap().strip_prefix("sessions/") else {
            continue;
        };
        let pid = file.strip_suffix(".json").unwrap().parse().unwrap();
        let content = (value["gone"] != true).then(|| value["content"].clone());
        steps.push((value["t"].as_i64().unwrap(), Step::Status { pid, content }));
    }
    let folder = format!("C--Users-USER-wakeshot-test-{name}");
    let mut transcripts = Vec::new();
    for suffix in ["", ".1", ".2"] {
        let path = dir.join(format!("{name}{suffix}.transcript.jsonl"));
        if path.exists() {
            transcripts.push(transcript(&path, &folder));
        }
    }
    for (file, transcript) in transcripts.iter().enumerate() {
        for (line, at) in transcript.lines.iter().enumerate() {
            steps.push((at.written, Step::Line { file, line }));
        }
    }
    steps.sort_by_key(|(at, _)| *at);
    Live { transcripts, steps }
}

const LIVE: [&str; 13] = [
    "01-bg-shell",
    "02-monitor",
    "03-loop-1m",
    "04-dynamic-loop",
    "05-reminder",
    "06-clear-during-loop",
    "07-subagent",
    "08-sdk-bg-shell",
    "09-resume-clean",
    "10-resume-kill",
    "11-resume-one-shot",
    "12-sdk-close-during-shell",
    "13-sdk-resume-kill",
];

/// What one step saw: the facts of the session that wrote the line, or whose status it was, before and after it.
struct Seen<'a> {
    step: &'a Step,
    before: Option<SessionFacts>,
    after: Option<SessionFacts>,
    all: Vec<SessionFacts>,
}

/// The watch thread, played by hand: status writes are fed whole; each wanted `FindTail` gets the lines on disk it has
/// not had, from the start when it first enters `wanted()`.
struct Run<'a> {
    live: &'a Live,
    source: ClaudeSource,
    fed: BTreeMap<String, usize>,
    written: Vec<usize>,
    pid_of: BTreeMap<String, u32>,
    as_sdk: bool,
}

impl<'a> Run<'a> {
    fn new(live: &'a Live) -> Self {
        Self {
            live,
            source: source(),
            fed: BTreeMap::new(),
            written: vec![0; live.transcripts.len()],
            pid_of: BTreeMap::new(),
            as_sdk: false,
        }
    }

    fn session(&self, pid: Option<u32>) -> Option<SessionFacts> {
        let id = pid?.to_string();
        self.source.sessions().into_iter().find(|s| s.key.id == id)
    }

    fn step(&mut self, step: &Step) -> Option<u32> {
        match step {
            Step::Status { pid, content } => {
                let data = match content {
                    Some(content) => {
                        let mut content = content.clone();
                        if self.as_sdk {
                            content["entrypoint"] = json!("sdk-cli");
                        }
                        if let Some(id) = content["sessionId"].as_str() {
                            self.pid_of.insert(id.to_owned(), *pid);
                        }
                        Feed::Whole(serde_json::to_vec(&content).unwrap())
                    }
                    None => Feed::Gone,
                };
                assert_eq!(self.source.feed(&status_file(*pid), data), Fed::Taken);
                Some(*pid)
            }
            Step::Line { file, .. } => {
                self.written[*file] += 1;
                self.pid_of
                    .get(&self.live.transcripts[*file].session_id)
                    .copied()
            }
        }
    }

    fn sync(&mut self) {
        let mut wanted = BTreeSet::new();
        for want in self.source.wanted() {
            if let WantedFile::FindTail {
                root,
                depth,
                name_suffix,
            } = want
            {
                assert_eq!((root, depth), (projects(), 1));
                wanted.insert(name_suffix);
            }
        }
        self.fed.retain(|suffix, _| wanted.contains(suffix));
        for suffix in wanted {
            let Some(index) = self
                .live
                .transcripts
                .iter()
                .position(|t| format!("{}.jsonl", t.session_id) == suffix)
            else {
                continue;
            };
            let transcript = &self.live.transcripts[index];
            let offset = self.fed.entry(suffix).or_insert(0);
            let upto = self.written[index];
            if upto > *offset {
                let bytes: Vec<u8> = transcript.lines[*offset..upto]
                    .iter()
                    .flat_map(|line| line.bytes.iter().copied())
                    .collect();
                assert_eq!(
                    self.source.feed(&transcript.file, Feed::Appended(bytes)),
                    Fed::Taken
                );
                *offset = upto;
            }
        }
    }

    fn run(mut self, steps: &[(i64, Step)], mut seen: impl FnMut(Seen)) {
        for (_, step) in steps {
            let pid = match step {
                Step::Status { pid, .. } => Some(*pid),
                Step::Line { file, .. } => self
                    .pid_of
                    .get(&self.live.transcripts[*file].session_id)
                    .copied(),
            };
            let before = self.session(pid);
            let pid = self.step(step).or(pid);
            self.sync();
            seen(Seen {
                step,
                before,
                after: self.session(pid),
                all: self.source.sessions(),
            });
        }
    }
}

fn line_of<'a>(live: &'a Live, step: &Step) -> Option<(usize, usize, &'a Value)> {
    match step {
        Step::Line { file, line } => Some((
            *file,
            *line + 1,
            &live.transcripts[*file].lines[*line].value,
        )),
        Step::Status { .. } => None,
    }
}

/// The first fire of a job in a resumed process: Claude Code counts it from its restore, which comes about 1 s after
/// the process starts (2.1.317, `zi`); Wakeshot counts from the start.
const AFTER_RESUME: [(&str, usize); 2] = [("09-resume-clean", 50), ("10-resume-kill", 40)];

/// The `*/1` loop is due within 1 s before each logged fire, and so is every other fire but the first one after a
/// resume, within 2 s. No live line gives a format problem.
#[test]
fn every_live_fire_was_due_and_every_live_line_reads() {
    let mut fires = 0;
    for name in LIVE {
        let live = live(name);
        Run::new(&live).run(&live.steps, |seen| {
            for session in &seen.all {
                assert_eq!(
                    session.format,
                    FormatHealth::Ok,
                    "{name}: {}",
                    session.key.id
                );
            }
            let Some((_, number, value)) = line_of(&live, seen.step) else {
                return;
            };
            if value["subtype"] != "scheduled_task_fire" {
                return;
            }
            let fired = time(value).unwrap();
            let late = if AFTER_RESUME.contains(&(name, number)) {
                2 * SECOND
            } else {
                SECOND
            };
            let timers = seen.before.expect("a live session").timers;
            assert!(
                timers
                    .iter()
                    .any(|&due| (0..=late).contains(&(fired - due))),
                "{name} line {number}: fired {fired}, due {timers:?}"
            );
            fires += 1;
        });
    }
    assert_eq!(fires, 11);
}

/// Claude Code's own `CronList` answer names each job's next fire; the source's due times agree within 1 s.
#[test]
fn every_live_cron_list_agrees() {
    let mut lists = 0;
    for name in LIVE {
        let live = live(name);
        Run::new(&live).run(&live.steps, |seen| {
            let Some((_, number, value)) = line_of(&live, seen.step) else {
                return;
            };
            let Some(jobs) = value["toolUseResult"]["jobs"].as_array() else {
                return;
            };
            let timers = seen.after.expect("a live session").timers;
            for job in jobs {
                let next = job["nextFireAt"].as_f64().unwrap().ceil() as i64;
                assert!(
                    timers.iter().any(|&due| (due - next).abs() <= SECOND),
                    "{name} line {number}: CronList {next}, due {timers:?}"
                );
            }
            lists += 1;
        });
    }
    assert_eq!(lists, 6);
}

/// After `stop: true`, `CronDelete` or a fired one-shot no timer is left; a resumed process drops the dead wakeup.
#[test]
fn the_live_timers_after_the_lines_that_change_them() {
    let d = |start: &str| wall_ms(start);
    let cases: Vec<(&str, usize, usize, Vec<i64>)> = vec![
        ("03-loop-1m", 0, 6, vec![1_793_715_622_220]),
        ("03-loop-1m", 0, 20, vec![]),
        ("04-dynamic-loop", 0, 6, vec![1_793_715_660_000]),
        ("04-dynamic-loop", 0, 10, vec![]),
        ("04-dynamic-loop", 0, 16, vec![]),
        ("05-reminder", 0, 11, vec![1_793_715_780_000]),
        ("05-reminder", 0, 19, vec![]),
        ("06-clear-during-loop", 0, 6, vec![1_793_715_688_955]),
        ("06-clear-during-loop", 1, 16, vec![]),
        (
            "09-resume-clean",
            0,
            35,
            vec![1_793_716_080_000, 1_793_716_091_763],
        ),
        // The resumed process, before its first fire: the `*/5` job counts from its start, the wakeup is gone. Its
        // second time is the latest any flag value allows: with a lead above 0, before 5 minutes after the start.
        (
            "09-resume-clean",
            0,
            49,
            vec![
                d("134381894952491900") + 285_000,
                d("134381894952491900") + 300_000,
            ],
        ),
        (
            "10-resume-kill",
            0,
            35,
            vec![1_793_716_080_000, 1_793_716_095_164],
        ),
        (
            "10-resume-kill",
            0,
            39,
            vec![
                d("134381894969847429") + 285_000,
                d("134381894969847429") + 300_000,
            ],
        ),
        (
            "11-resume-one-shot",
            0,
            38,
            vec![1_793_716_440_000, 1_793_770_200_000],
        ),
        (
            "11-resume-one-shot",
            0,
            59,
            vec![1_793_716_440_000, 1_793_770_200_000],
        ),
        ("11-resume-one-shot", 0, 63, vec![1_793_770_200_000]),
    ];
    let mut checked = BTreeSet::new();
    for name in LIVE {
        let live = live(name);
        Run::new(&live).run(&live.steps, |seen| {
            let Some((file, number, _)) = line_of(&live, seen.step) else {
                return;
            };
            for (case, case_file, line, want) in &cases {
                if (*case, *case_file, *line) == (name, file, number) {
                    let timers = seen.after.as_ref().expect("a live session").timers.clone();
                    assert_eq!(&timers, want, "{name} file {file} line {number}");
                    checked.insert((*case, *case_file, *line));
                }
            }
        });
    }
    assert_eq!(checked.len(), cases.len());
}

/// Rule 3 on the live data: an SDK session's background shells and Monitors are open tasks until a notification with a
/// status; a terminal session's are not counted (its status says `shell`). 02 and 07 also run as if from the SDK.
#[test]
fn the_live_open_tasks() {
    /// A case, whether its status says SDK, and the open tasks after some of its lines.
    type Case = (&'static str, bool, Vec<(usize, u32)>);
    let cases: Vec<Case> = vec![
        ("01-bg-shell", false, vec![(3, 0), (7, 0)]),
        ("02-monitor", false, vec![(5, 0)]),
        // Its Monitors are not persistent: timers at their timeouts (`the_live_monitors_hold_until_their_timeout`).
        (
            "02-monitor",
            true,
            vec![
                (5, 0),
                (9, 0),
                (11, 0),
                (15, 0),
                (19, 0),
                (23, 0),
                (27, 0),
                (31, 0),
            ],
        ),
        // A background subagent keeps the status `busy` itself; it opens no task.
        ("07-subagent", true, vec![(3, 0), (13, 0), (18, 0), (28, 0)]),
        // The notification's `queue-operation` line (6) ends the shell before its user line (8).
        (
            "08-sdk-bg-shell",
            false,
            vec![(4, 0), (5, 1), (6, 0), (8, 0)],
        ),
        (
            "12-sdk-close-during-shell",
            false,
            vec![(5, 1), (6, 0), (8, 0)],
        ),
        // The killed process's shell was open; the resumed process does not count it, before or after its notice.
        ("13-sdk-resume-kill", false, vec![(11, 1), (16, 0), (18, 0)]),
    ];
    for (name, as_sdk, want) in cases {
        let live = live(name);
        let mut run = Run::new(&live);
        run.as_sdk = as_sdk;
        let mut checked = 0;
        run.run(&live.steps, |seen| {
            let Some((_, number, _)) = line_of(&live, seen.step) else {
                return;
            };
            if let Some((_, tasks)) = want.iter().find(|(line, _)| *line == number) {
                let session = seen.after.as_ref().expect("a live session");
                assert_eq!(session.open_tasks, *tasks, "{name} line {number}");
                checked += 1;
            }
        });
        assert_eq!(checked, want.len(), "{name}");
    }
}

/// Live fixture 02: each Monitor (120 s, not persistent) holds as a timer at its result's time plus its timeout, until
/// a notification with a `<status>` ends it, from its `queue-operation` line on; events change nothing. A terminal
/// session's status says `shell` for it, so there it adds no timer.
#[test]
fn the_live_monitors_hold_until_their_timeout() {
    let want: [(usize, &[i64]); 13] = [
        (5, &[1_793_715_683_407]),
        (6, &[]),
        (9, &[]),
        (11, &[1_793_715_689_001]),
        (12, &[]),
        (15, &[]),
        (19, &[1_793_715_695_984]),
        (21, &[1_793_715_695_984]),
        (23, &[1_793_715_695_984]),
        (25, &[1_793_715_695_984]),
        (27, &[1_793_715_695_984]),
        (29, &[]),
        (31, &[]),
    ];
    let live = live("02-monitor");
    for as_sdk in [true, false] {
        let mut run = Run::new(&live);
        run.as_sdk = as_sdk;
        let mut checked = 0;
        run.run(&live.steps, |seen| {
            let Some((_, number, _)) = line_of(&live, seen.step) else {
                return;
            };
            if let Some((_, timers)) = want.iter().find(|(line, _)| *line == number) {
                let session = seen.after.as_ref().expect("a live session");
                let timers = if as_sdk { *timers } else { &[] };
                assert_eq!(session.timers, timers, "line {number}, SDK {as_sdk}");
                assert_eq!(session.open_tasks, 0, "line {number}, SDK {as_sdk}");
                checked += 1;
            }
        });
        assert_eq!(checked, want.len());
    }
}

/// Not one line of the live sessions is a format problem, read by a session that would tell it.
#[test]
fn no_live_transcript_line_is_a_problem() {
    let mut read = 0;
    for (name, bytes) in every_transcript() {
        if !name.ends_with(".transcript.jsonl") {
            continue;
        }
        let [session] =
            <[SessionFacts; 1]>::try_from(whole_facts(&bytes, &session_of(&bytes))).unwrap();
        assert_eq!(session.format, FormatHealth::Ok, "{name}");
        read += 1;
    }
    assert_eq!(read, 14);
}

/// A resume can take over a transcript while the killed process's leftover status file is still listed (fixture 13:
/// both changes in one millisecond). Either feed order gives the resumed process the same facts.
#[test]
fn a_resume_gives_the_same_facts_in_either_feed_order() {
    let live = live("13-sdk-resume-kill");
    let at = live
        .steps
        .iter()
        .position(|(_, step)| {
            matches!(
                step,
                Step::Status {
                    pid: 30812,
                    content: None
                }
            )
        })
        .unwrap();
    assert!(matches!(
        live.steps[at + 1].1,
        Step::Status {
            pid: 31708,
            content: Some(_)
        }
    ));
    let mut swapped = live.steps.clone();
    swapped.swap(at, at + 1);
    let facts = |steps: &[(i64, Step)]| {
        let mut all = Vec::new();
        Run::new(&live).run(steps, |seen| all.push(seen.all));
        all
    };
    let (gone_first, new_first) = (facts(&live.steps), facts(&swapped));
    assert_eq!(gone_first[at + 1..], new_first[at + 1..]);
    assert_eq!(gone_first.last().unwrap().len(), 0);
}

/// The watch thread goes on with a tail that stays wanted, so a resumed process listed before the leftover of the
/// process it took over is gone starts from that leftover's replay.
#[test]
fn a_resume_while_the_leftover_is_listed_goes_on_from_its_replay() {
    let (session_id, bytes) = hand("resume.jsonl");
    let lines = lines_of(&bytes);
    let file = transcript_file("C--Users-USER-wakeshot-test-hand", &session_id);
    let mut source = source();
    let status_of = |pid, start| Feed::Whole(status(pid, start, &session_id, "idle", "sdk-cli"));
    source.feed(&status_file(1), status_of(1, EARLY));
    source.feed(&file, Feed::Appended(lines[..11].concat()));
    source.feed(&status_file(2), status_of(2, "134381895000000000"));
    source.feed(&status_file(1), Feed::Gone);
    assert_eq!(
        only(&source).timers,
        [1_793_716_185_000, 1_793_716_440_000, LATEST_5]
    );
    source.feed(&file, Feed::Appended(lines[11..].concat()));
    assert_eq!(
        only(&source).timers,
        [1_793_716_440_000, 1_793_716_471_100, LATEST_5]
    );
}

/// The latest fire any value of Claude Code's remote flag allows for the `*/5` job of `resume.jsonl` (`f9471e8f`)
/// counted from 14:25:00.000 or from its fire at 14:29:46.100 UTC: both give the 14:30 run plus the whole share of its
/// 5 minute gap, `0xf9471e8f / 2^32 * 300 s` = 292.123 s.
const LATEST_5: i64 = 1_793_716_492_123;

/// Wakeshot started after the `/clear` reads only the new transcript: the carry line alone gives the job's next fire.
#[test]
fn wakeshot_started_after_a_clear_finds_the_next_fire() {
    let live = live("06-clear-during-loop");
    let cleared = live
        .steps
        .iter()
        .position(|(_, step)| {
            matches!(step, Step::Status { content: Some(content), .. }
                if content["sessionId"] == "5e550019-0000-4000-8000-000000000019")
        })
        .unwrap();
    let steps: Vec<(i64, Step)> = live.steps[cleared..]
        .iter()
        .filter(|(_, step)| !matches!(step, Step::Line { file: 0, .. }))
        .cloned()
        .collect();
    let mut skipped_lines = 0;
    let mut checked = false;
    let mut run = Run::new(&live);
    // The lines of the new transcript written before Wakeshot started are on disk already.
    for (_, step) in &live.steps[..cleared] {
        if let Step::Line { file: 1, .. } = step {
            run.written[1] += 1;
            skipped_lines += 1;
        }
    }
    assert_eq!(skipped_lines, 4);
    run.run(&steps, |seen| {
        let Some((1, 7, value)) = line_of(&live, seen.step) else {
            return;
        };
        let fired = time(value).unwrap();
        let timers = seen.before.expect("a live session").timers;
        // The second time is the latest any flag value allows: the whole share of the gap, not half.
        assert_eq!(timers, [1_793_715_748_955, 1_793_715_777_910]);
        assert!((0..=SECOND).contains(&(fired - timers[0])));
        checked = true;
    });
    assert!(checked);
}

fn status(pid: u32, start: &str, session_id: &str, status: &str, entrypoint: &str) -> Vec<u8> {
    json!({"pid": pid, "sessionId": session_id, "cwd": "C:\\Users\\USER\\wakeshot-test\\x",
        "procStart": start, "startedAt": 0, "version": "2.1.317", "kind": "interactive",
        "entrypoint": entrypoint, "status": status, "updatedAt": 1})
    .to_string()
    .into_bytes()
}

/// A FILETIME long before every fixture line, so no line is an earlier process's.
const EARLY: &str = "134000000000000000";
const PID: u32 = 4242;

fn hand(name: &str) -> (String, Vec<u8>) {
    let bytes = fs::read(fixtures("transcript").join(name)).unwrap();
    let first: Value =
        serde_json::from_slice(bytes.split(|&b| b == b'\n').next().unwrap()).unwrap();
    (first["sessionId"].as_str().unwrap().to_owned(), bytes)
}

/// A source with one session at `start` that reads the whole fixture `name`.
fn fed(name: &str, start: &str, status_text: &str, entrypoint: &str) -> ClaudeSource {
    let (session_id, bytes) = hand(name);
    let mut source = source();
    source.feed(
        &status_file(PID),
        Feed::Whole(status(PID, start, &session_id, status_text, entrypoint)),
    );
    source.feed(
        &transcript_file("C--Users-USER-wakeshot-test-hand", &session_id),
        Feed::Appended(bytes),
    );
    source
}

fn only(source: &ClaudeSource) -> SessionFacts {
    let [session] = <[SessionFacts; 1]>::try_from(source.sessions()).unwrap();
    session
}

const HAND: [&str; 12] = [
    "broken-line.jsonl",
    "carry-after-clear.jsonl",
    "deleted.jsonl",
    "header-notice.jsonl",
    "joined-notices.jsonl",
    "missing-field.jsonl",
    "monitor-timeout.jsonl",
    "resume.jsonl",
    "seven-days.jsonl",
    "task-stop.jsonl",
    "ui-stop.jsonl",
    "unknown-kind.jsonl",
];

#[test]
fn every_hand_fixture_has_a_case() {
    let on_disk: BTreeSet<String> = fs::read_dir(fixtures("transcript"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .filter(|name| name != "README.md")
        .collect();
    let listed: BTreeSet<String> = HAND.iter().map(|name| (*name).to_owned()).collect();
    assert_eq!(on_disk, listed);
}

/// A transcript problem never makes a session hold. It is named only while the status file or an open task holds the
/// session anyway, so "update Wakeshot" shows at its next turn.
#[test]
fn a_broken_line_is_named_only_while_the_session_holds_anyway() {
    let idle = only(&fed("broken-line.jsonl", EARLY, "idle", "cli"));
    assert_eq!(idle.format, FormatHealth::Ok);
    assert_eq!(idle.activity, Activity::Idle);
    // The first fire was lost with the broken line; the second one moved the job on. The second time is the latest
    // any value of Claude Code's remote flag allows: the whole share of the minute, not half.
    assert_eq!(idle.timers, [1_793_715_742_220, 1_793_715_764_440]);
    let busy = only(&fed("broken-line.jsonl", EARLY, "busy", "cli"));
    assert_eq!(
        busy.format,
        FormatHealth::UnknownValue("transcript", "line".into())
    );
    // A waiting session holds for an hour at most: it is not told there.
    let waiting = only(&fed("broken-line.jsonl", EARLY, "waiting", "cli"));
    assert_eq!(waiting.format, FormatHealth::Ok);
    // A status file problem comes first.
    let unknown = only(&fed("broken-line.jsonl", EARLY, "pondering", "cli"));
    assert_eq!(
        unknown.format,
        FormatHealth::UnknownValue("status", "pondering".into())
    );
}

#[test]
fn unknown_kinds_are_ignored_silently() {
    let session = only(&fed("unknown-kind.jsonl", EARLY, "busy", "cli"));
    assert_eq!(session.format, FormatHealth::Ok);
    assert_eq!(session.timers, [1_793_715_622_220]);
}

#[test]
fn a_known_kind_with_a_missing_field_is_skipped_and_named() {
    let session = only(&fed("missing-field.jsonl", EARLY, "busy", "cli"));
    assert_eq!(
        session.format,
        FormatHealth::UnknownValue("transcript", "CronCreate result".into())
    );
    assert_eq!(session.timers, Vec::<i64>::new());
}

/// The job was made 2026-11-03 14:19:24.194 UTC: the fire one week later less 2 s moves it on, the next ends it.
#[test]
fn a_recurring_job_ends_after_its_fire_at_7_days() {
    let (session_id, bytes) = hand("seven-days.jsonl");
    let lines = lines_of(&bytes);
    let mut one = source();
    one.feed(
        &status_file(PID),
        Feed::Whole(status(PID, EARLY, &session_id, "idle", "cli")),
    );
    let file = transcript_file("C--Users-USER-wakeshot-test-hand", &session_id);
    let mut timers = Vec::new();
    for line in lines {
        one.feed(&file, Feed::Appended(line.to_vec()));
        timers.push(only(&one).timers);
    }
    let day = parse_expires("2026-11-10T00:00:00Z").unwrap();
    let at = |text: &str| day + parse_expires(&format!("1970-01-01T{text}Z")).unwrap();
    assert_eq!(
        timers,
        [
            vec![],
            vec![1_793_715_622_220],
            vec![at("14:19:22.220"), at("14:19:44.440")],
            vec![at("14:20:22.220"), at("14:20:44.440")],
            vec![],
        ]
    );
    // A process resumed 7 days after the job was made, before its last fire, does not take it back.
    let mut resumed = source();
    let start = filetime(at("14:19:30.000"));
    resumed.feed(
        &status_file(PID),
        Feed::Whole(status(PID, &start, &session_id, "idle", "cli")),
    );
    resumed.feed(&file, Feed::Appended(lines_of(&bytes)[..4].concat()));
    assert_eq!(only(&resumed).timers, Vec::<i64>::new());
}

/// Each fixture line by line, as an SDK session reads it: its timers and open tasks after each line.
fn sdk_steps(name: &str) -> Vec<(Vec<i64>, u32)> {
    let (session_id, bytes) = hand(name);
    let mut source = source();
    source.feed(
        &status_file(PID),
        Feed::Whole(status(PID, EARLY, &session_id, "idle", "sdk-cli")),
    );
    let file = transcript_file("C--Users-USER-wakeshot-test-hand", &session_id);
    lines_of(&bytes)
        .into_iter()
        .map(|line| {
            source.feed(&file, Feed::Appended(line.to_vec()));
            let session = only(&source);
            assert_eq!(session.format, FormatHealth::Ok, "{name}");
            (session.timers, session.open_tasks)
        })
        .collect()
}

/// The shell stopped with `TaskStop` is closed by the stop's result; Claude Code writes no notification.
#[test]
fn a_task_stop_ends_the_shell_it_names() {
    assert_eq!(
        sdk_steps("task-stop.jsonl"),
        [(vec![], 0), (vec![], 1), (vec![], 1), (vec![], 0)]
    );
}

/// A shell stopped from the UI ends at its `killed` notice's `queue-operation` line, written when Claude Code queues
/// the notice; its user line may come only at the next prompt. The shape is from the 2.1.317 code (`tUr`), not
/// checked live.
#[test]
fn a_stop_from_the_ui_ends_the_shell_at_its_queued_notice() {
    assert_eq!(
        sdk_steps("ui-stop.jsonl"),
        [(vec![], 0), (vec![], 1), (vec![], 0)]
    );
}

/// A 120 s Monitor holds as a timer at 14:21:35.984 UTC; its timeout's event, which has no `<status>`, changes nothing,
/// and the core's window ends 2 minutes after the time.
#[test]
fn a_monitor_that_times_out_holds_only_until_its_timeout() {
    let due = 1_793_715_695_984;
    assert_eq!(
        sdk_steps("monitor-timeout.jsonl"),
        [(vec![], 0), (vec![due], 0), (vec![due], 0), (vec![due], 0)]
    );
}

/// A notification after Claude Code's own header closes its shell, and so does each of two joined ones.
#[test]
fn notifications_after_the_header_close_their_shells() {
    assert_eq!(
        sdk_steps("header-notice.jsonl"),
        [(vec![], 0), (vec![], 1), (vec![], 0)]
    );
    assert_eq!(
        sdk_steps("joined-notices.jsonl"),
        [
            (vec![], 0),
            (vec![], 1),
            (vec![], 1),
            (vec![], 2),
            (vec![], 0)
        ]
    );
}

fn filetime(wall_ms: i64) -> String {
    (i128::from(wall_ms) * 10_000 + 116_444_736_000_000_000).to_string()
}

fn lines_of(bytes: &[u8]) -> Vec<&[u8]> {
    bytes.split_inclusive(|&b| b == b'\n').collect()
}

#[test]
fn a_deleted_job_is_gone_and_a_failed_delete_keeps_it() {
    let (session_id, bytes) = hand("deleted.jsonl");
    let lines = lines_of(&bytes);
    let mut source = source();
    source.feed(
        &status_file(PID),
        Feed::Whole(status(PID, EARLY, &session_id, "idle", "cli")),
    );
    let file = transcript_file("C--Users-USER-wakeshot-test-hand", &session_id);
    let mut timers = Vec::new();
    for line in lines {
        source.feed(&file, Feed::Appended(line.to_vec()));
        timers.push(only(&source).timers);
    }
    assert_eq!(timers[6..11], vec![vec![1_793_715_622_220]; 5]);
    assert_eq!(timers[12], Vec::<i64>::new());
    assert_eq!(only(&source).format, FormatHealth::Ok);
}

/// Wakeshot started after a `/clear`: a fuller carry line than the live one, with a one-shot in `tasks`, a daily job,
/// a pending wakeup and an ended job.
#[test]
fn a_carry_line_after_clear_is_the_armed_set() {
    let session = only(&fed("carry-after-clear.jsonl", EARLY, "idle", "cli"));
    // The `*/1` job moved on from its fire (line 7) to the time live fixture 06's `CronList` gave; the daily job is
    // at 09:00 local plus the 30 minute cap, the one-shot at 18:45 local.
    assert_eq!(
        session.timers,
        [
            1_793_715_808_955,
            1_793_715_837_910,
            1_793_716_080_000,
            1_793_717_100_000,
            1_793_770_200_000
        ]
    );
    assert_eq!(session.format, FormatHealth::Ok);
}

/// Two processes on one transcript (a resume): the second starts at 14:25:00.000 UTC.
#[test]
fn a_resumed_process_takes_back_only_what_claude_code_restores() {
    let second = "134381895000000000";
    assert_eq!(wall_ms(second), 1_793_715_900_000);
    // Read as one process, every line counts: the one-shot past its time, the wakeup, the `*/5` job moved on by its
    // fire, and the shell closed by the stopped notice.
    let one = only(&fed("resume.jsonl", EARLY, "idle", "sdk-cli"));
    assert_eq!(
        one.timers,
        [
            1_793_715_780_000,
            1_793_716_080_000,
            1_793_716_440_000,
            1_793_716_471_100,
            LATEST_5
        ]
    );
    assert_eq!(one.open_tasks, 0);

    let (session_id, bytes) = hand("resume.jsonl");
    let lines = lines_of(&bytes);
    let mut source = source();
    source.feed(
        &status_file(PID),
        Feed::Whole(status(PID, second, &session_id, "idle", "sdk-cli")),
    );
    let file = transcript_file("C--Users-USER-wakeshot-test-hand", &session_id);
    source.feed(&file, Feed::Appended(lines[..11].concat()));
    let resumed = only(&source);
    // The missed one-shot, the wakeup and the dead process's shell are gone; the `*/5` job counts from the start.
    assert_eq!(
        resumed.timers,
        [1_793_716_185_000, 1_793_716_440_000, LATEST_5]
    );
    assert_eq!(resumed.open_tasks, 0);
    source.feed(&file, Feed::Appended(lines[11..].concat()));
    assert_eq!(
        only(&source).timers,
        [1_793_716_440_000, 1_793_716_471_100, LATEST_5]
    );
}

/// Rule 5: one `FindTail` per transcript a live session names, by name only.
#[test]
fn wanted_tails_each_session_transcript_by_name() {
    let mut source = source();
    let find = |source: &ClaudeSource| -> Vec<String> {
        source
            .wanted()
            .into_iter()
            .filter_map(|want| match want {
                WantedFile::FindTail {
                    root,
                    depth,
                    name_suffix,
                } => {
                    assert_eq!((root, depth), (projects(), 1));
                    Some(name_suffix)
                }
                _ => None,
            })
            .collect()
    };
    let a = "5e550004-0000-4000-8000-000000000004";
    let b = "5e550019-0000-4000-8000-000000000019";
    source.feed(
        &status_file(1),
        Feed::Whole(status(1, EARLY, a, "idle", "cli")),
    );
    source.feed(
        &status_file(2),
        Feed::Whole(status(2, EARLY, a, "idle", "cli")),
    );
    assert_eq!(find(&source), [format!("{a}.jsonl")]);
    source.feed(
        &status_file(1),
        Feed::Whole(status(1, EARLY, b, "idle", "cli")),
    );
    assert_eq!(find(&source), [format!("{a}.jsonl"), format!("{b}.jsonl")]);
    source.feed(&status_file(2), Feed::Gone);
    assert_eq!(find(&source), [format!("{b}.jsonl")]);
    // A status file without a usable `sessionId` names no transcript.
    source.feed(
        &status_file(3),
        Feed::Whole(status(3, EARLY, "a/b", "idle", "cli")),
    );
    assert_eq!(find(&source), [format!("{b}.jsonl")]);
}

/// A `sessionId` of another shape hides the transcript, so it is named like a transcript problem: only while the
/// session holds anyway, by its length, never its text.
#[test]
fn a_session_id_of_another_shape_is_named_while_the_session_holds() {
    let facts = |status_text: &str| {
        let mut source = source();
        source.feed(
            &status_file(PID),
            Feed::Whole(status(PID, EARLY, "a/b", status_text, "sdk-cli")),
        );
        only(&source).format
    };
    assert_eq!(
        facts("busy"),
        FormatHealth::UnknownValue("sessionId", "<string 3>".into())
    );
    assert_eq!(facts("idle"), FormatHealth::Ok);
}

/// Only the transcript a session names, in a project folder, as appended bytes, reaches its replay.
#[test]
fn a_transcript_it_did_not_ask_for_is_ignored() {
    let (session_id, bytes) = hand("carry-after-clear.jsonl");
    let mut source = source();
    source.feed(
        &status_file(PID),
        Feed::Whole(status(PID, EARLY, &session_id, "idle", "cli")),
    );
    let name = format!("{session_id}.jsonl");
    for (path, data) in [
        (
            projects().join("a").join("b").join(&name),
            Feed::Appended(bytes.clone()),
        ),
        (
            PathBuf::from(r"X:\elsewhere\a").join(&name),
            Feed::Appended(bytes.clone()),
        ),
        (projects().join(&name), Feed::Appended(bytes.clone())),
        (
            projects()
                .join("a")
                .join("5e550099-0000-4000-8000-000000000099.jsonl"),
            Feed::Appended(bytes.clone()),
        ),
        (projects().join("a").join(&name), Feed::Whole(bytes.clone())),
        (projects().join("a").join(&name), Feed::Present),
        (projects().join("a").join(&name), Feed::Gone),
    ] {
        assert_eq!(source.feed(&FileId(path), data), Fed::Taken);
    }
    assert_eq!(only(&source).timers, Vec::<i64>::new());
    // The first folder fed is the session's; the same name in another folder is not.
    source.feed(
        &FileId(projects().join("first").join(&name)),
        Feed::Appended(bytes.clone()),
    );
    let timers = only(&source).timers;
    assert_eq!(timers.len(), 5);
    source.feed(
        &FileId(projects().join("second").join(&name)),
        Feed::Appended(hand("resume.jsonl").1),
    );
    assert_eq!(only(&source).timers, timers);
}

/// Every fixture transcript, live and hand-made, read whole by a session that counts every line.
fn whole_facts(bytes: &[u8], session_id: &str) -> Vec<SessionFacts> {
    pieces_facts(bytes, session_id, &[])
}

fn pieces_facts(bytes: &[u8], session_id: &str, cuts: &[usize]) -> Vec<SessionFacts> {
    let mut source = source();
    source.feed(
        &status_file(PID),
        Feed::Whole(status(PID, EARLY, session_id, "busy", "sdk-cli")),
    );
    let file = transcript_file("C--Users-USER-wakeshot-test-any", session_id);
    let mut start = 0;
    for &end in cuts.iter().chain([&bytes.len()]) {
        source.feed(&file, Feed::Appended(bytes[start..end].to_vec()));
        start = end;
    }
    source.sessions()
}

fn every_transcript() -> Vec<(String, Vec<u8>)> {
    let mut all = Vec::new();
    for dir in ["live", "transcript"] {
        for entry in fs::read_dir(fixtures(dir)).unwrap() {
            let path = entry.unwrap().path();
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            if name.ends_with(".transcript.jsonl")
                || (dir == "transcript" && name.ends_with(".jsonl"))
            {
                all.push((name, fs::read(&path).unwrap()));
            }
        }
    }
    all
}

fn session_of(bytes: &[u8]) -> String {
    bytes
        .split(|&b| b == b'\n')
        .filter_map(|line| serde_json::from_slice::<Value>(line).ok())
        .find_map(|value| value["sessionId"].as_str().map(str::to_owned))
        .unwrap()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    /// The tail may deliver a transcript in any pieces; the facts are those of the whole file.
    #[test]
    fn every_fixture_transcript_in_any_pieces_gives_the_same_facts(
        cuts in proptest::collection::vec(any::<prop::sample::Index>(), 0..16),
    ) {
        for (name, bytes) in every_transcript() {
            let session_id = session_of(&bytes);
            let mut at: Vec<usize> = cuts.iter().map(|cut| cut.index(bytes.len() + 1)).collect();
            at.sort_unstable();
            prop_assert_eq!(
                pieces_facts(&bytes, &session_id, &at),
                whole_facts(&bytes, &session_id),
                "{}", name
            );
        }
    }
}
