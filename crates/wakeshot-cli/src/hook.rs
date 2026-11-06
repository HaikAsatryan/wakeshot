//! `wakeshot hook <agent> <event> [--checked <value>...]`: the one command every agent's hook runs
//! (`docs/agent-hooks.md`, "The hook command"). It keeps two ids of the agent's input, names the agent process by
//! walking up its parents, writes or deletes the chat's file in `agents\<agent>\`, and always prints `{}` and exits 0,
//! since any other answer could block or change the agent's prompt.
//!
//! The steps run behind `Env`, so the tests run them in-process with fakes; `main` gives the real one.
//!
//! Antigravity adds two steps (`docs/agent-hooks.md`, "Antigravity"): its IDE's hub runs the same hooks and writes
//! nothing, since only the CLI is held in v1; and a model call's file names the `agy.exe`'s own log and when `agy`
//! started the hook, so the source can end the turn at a cancel line no hook reports.
//!
//! Owner: `p4-cursor-source`; the Antigravity steps by `p4-antigravity-exact-end`.

use std::ffi::OsString;
use std::fs::File;
use std::io::{self, IsTerminal, Read, Write};
use std::os::windows::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, SystemTime};

use wakeshot_core::process::{Alive, FileTime, ProcessRef};
use wakeshot_core::sources::hooked::{
    self, Agent, Bases, Checked, EventRule, Found, InputProblem, Listed, LogMark, Op, Problem,
    State, Walk,
};
use wakeshot_core::sources::{antigravity, cursor};
use wakeshot_windows::WinError;
use wakeshot_windows::agent_hooks;
use wakeshot_windows::clock;
use wakeshot_windows::log::{Level, Log, Proc};
use wakeshot_windows::process::{self, Liveness as _, ProcessEntry, SystemLiveness};
use wakeshot_windows::recovery;
use wakeshot_windows::store::{self, DataDir, NamedMutex};

use crate::command::Global;
use crate::status::error_text;

/// Cursor merges hook answers field by field, so `{}` changes nothing, and a non-empty answer keeps it from counting
/// the hook as failed (`docs/agent-hooks.md`, step 6).
const ANSWER: &[u8] = b"{}";

const AREA: &str = "agent-hook";

/// Wakeshot's own form for an elevated uninstall (`docs/agent-hooks.md`, "Removing").
const REMOVE: &str = "remove";

const HOOK: &str = "hook";
const CHECKED: &str = "--checked";
const DRY_RUN: &str = "--dry-run";

/// One mutex for every agent's folder (step 5).
const MUTEX: &str = r"Local\Wakeshot.AgentHooks";
const MUTEX_WAIT_MS: u32 = 1_000;

/// How long an elevated hook waits for its Medium copy, and how late that copy may still write "working".
const HAND_OFF_MS: u32 = 2_000;

/// A temp file of `write_atomic` older than this is a crash's leftover, never a write in progress.
const TEMP_AGE: Duration = Duration::from_secs(60);

/// Every agent the command knows; any other word does nothing, so a hook left by any version never fails.
static AGENTS: &[&Agent] = &[&cursor::AGENT, &antigravity::AGENT];

/// Set once `{}` went out, so the panic hook never prints it twice.
static ANSWERED: AtomicBool = AtomicBool::new(false);

/// Read by debug builds only: a test makes the hook panic after the parse, to see the answer and the exit code hold.
const PANIC_ENV: &str = "WAKESHOT_TEST_HOOK_PANIC";

pub fn main(global: &Global, agent: &str, event: &str, values: &[String]) -> ExitCode {
    // Before anything loads a DLL: a `stop` hook runs with the user's repo, which may not be trusted, as its folder.
    let dll = process::harden_dll_search().err();
    // The release build aborts on a panic, so only a hook can still answer and exit 0. It logs nothing, so no input
    // can leak through a panic message.
    std::panic::set_hook(Box::new(|_| {
        answer();
        std::process::exit(0);
    }));
    let env = RealEnv::new(*global);
    run(&env, agent, event, values, dll);
    answer();
    ExitCode::SUCCESS
}

fn answer() {
    if ANSWERED.swap(true, Ordering::SeqCst) {
        return;
    }
    let mut out = io::stdout().lock();
    // A write error is ignored: the exit code must stay 0 whatever happens to the pipe.
    let _ = out.write_all(ANSWER).and_then(|()| out.flush());
}

/// What the hook needs from the system.
trait Env {
    fn global(&self) -> Global;
    fn data(&self) -> Option<&DataDir>;
    fn log(&self) -> &dyn Log;
    /// The agent's input, at most [`hooked::INPUT_LIMIT`] of it.
    fn input(&self) -> Box<dyn Read + '_>;
    /// Reads the input to its end, so the agent's pipe never breaks.
    fn drain_input(&self);
    fn list(&self) -> Result<Vec<ProcessEntry>, WinError>;
    fn me(&self) -> Result<ProcessRef, WinError>;
    fn image_path(&self, process: ProcessRef) -> Option<PathBuf>;
    fn command_line_has(&self, process: ProcessRef, needle: &str) -> Result<bool, WinError>;
    fn above_medium(&self) -> Result<bool, WinError>;
    /// Starts this exe with `args` at Medium (Explorer's token) and waits for it at most [`HAND_OFF_MS`].
    fn hand_off(&self, args: &[OsString]);
    fn alive(&self, process: ProcessRef) -> Alive;
    fn now_wall_ms(&self) -> i64;
    fn bases(&self) -> Bases;
    /// Removes the agent's plugin folder by its fixed names (`agent_hooks`), which checks the level itself.
    fn remove_hooks(&self, agent: &str, log: &dyn Log);
    /// Debug builds: whether a test asked for a panic.
    fn test_panic(&self) -> bool;
    /// The bare name of the log `agy.exe` `agent` writes ([`find_log`]); `None` when none names it.
    fn agy_log(&self, agent: ProcessRef) -> Option<String>;
}

struct RealEnv {
    global: Global,
    data: Option<DataDir>,
    log: OnceLock<Arc<dyn Log>>,
}

impl RealEnv {
    fn new(global: Global) -> Self {
        let data = match global.dry_run {
            true => DataDir::dry_run(),
            false => DataDir::real(),
        };
        Self {
            global,
            data: data.ok(),
            log: OnceLock::new(),
        }
    }
}

impl Env for RealEnv {
    fn global(&self) -> Global {
        self.global
    }

    fn data(&self) -> Option<&DataDir> {
        self.data.as_ref()
    }

    /// Opened at the first call: `Lines` calls it only for a line, so a hook with nothing to say never touches the log
    /// folder.
    fn log(&self) -> &dyn Log {
        self.log
            .get_or_init(|| match &self.data {
                Some(dir) => recovery::open_log(dir, Proc::Cli),
                None => Arc::new(Quiet),
            })
            .as_ref()
    }

    fn input(&self) -> Box<dyn Read + '_> {
        let stdin = io::stdin();
        // A person who types the command gives no input; a read would wait for them.
        if stdin.is_terminal() {
            return Box::new(io::empty());
        }
        Box::new(stdin.lock().take(hooked::INPUT_LIMIT))
    }

    fn drain_input(&self) {
        let stdin = io::stdin();
        if !stdin.is_terminal() {
            let _ = io::copy(&mut stdin.lock(), &mut io::sink());
        }
    }

    fn list(&self) -> Result<Vec<ProcessEntry>, WinError> {
        SystemLiveness.list()
    }

    fn me(&self) -> Result<ProcessRef, WinError> {
        process::current()
    }

    fn image_path(&self, process: ProcessRef) -> Option<PathBuf> {
        process::image_path(process).ok().flatten()
    }

    fn command_line_has(&self, process: ProcessRef, needle: &str) -> Result<bool, WinError> {
        process::command_line_contains(process, needle)
    }

    fn above_medium(&self) -> Result<bool, WinError> {
        process::current_above_medium()
    }

    fn hand_off(&self, args: &[OsString]) {
        let Ok(exe) = std::env::current_exe() else {
            return;
        };
        if let Ok(spawned) = process::spawn_outside_tree(&exe, args) {
            let _ = spawned.handle.wait(HAND_OFF_MS);
        }
    }

    fn alive(&self, process: ProcessRef) -> Alive {
        process::liveness(process)
    }

    fn now_wall_ms(&self) -> i64 {
        clock::now().wall_ms
    }

    fn bases(&self) -> Bases {
        Bases {
            local_app_data: store::local_app_data().ok(),
            // No table matches under the profile: Antigravity's IDE copy in `.gemini\bin` is told from the CLI by not
            // being in the CLI's folder.
            profile: None,
        }
    }

    fn remove_hooks(&self, agent: &str, log: &dyn Log) {
        agent_hooks::remove_command(agent, log);
    }

    fn test_panic(&self) -> bool {
        cfg!(debug_assertions) && std::env::var_os(PANIC_ENV).is_some()
    }

    /// Under the profile folder as Go's `os.UserHomeDir` finds it (`USERPROFILE`), the one `agy` writes under; the
    /// tray's source reads the same folder.
    fn agy_log(&self, agent: ProcessRef) -> Option<String> {
        find_log(&antigravity::log_dir(&std::env::home_dir()?), agent)
    }
}

/// The log `agy.exe` `agent` writes: a plain file of the log's name shape, made at or after that process started (so
/// an older process that had the same pid never counts), whose first line names its pid; the newest if two do. Only
/// the first [`antigravity::HEAD_BYTES`] of a file are read, and only its pid is kept: the rest of the log holds
/// prompts.
fn find_log(dir: &Path, agent: ProcessRef) -> Option<String> {
    let mut found: Option<(u64, String)> = None;
    for entry in std::fs::read_dir(dir).ok()?.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str().filter(|name| antigravity::is_log_name(name)) else {
            continue;
        };
        // A link is no file here, so a planted one is never followed.
        if !entry.file_type().is_ok_and(|kind| kind.is_file()) {
            continue;
        }
        let Ok(made) = entry.metadata().map(|meta| meta.creation_time()) else {
            continue;
        };
        if made < agent.start.0 || found.as_ref().is_some_and(|(newest, _)| *newest >= made) {
            continue;
        }
        let mut head = [0u8; antigravity::HEAD_BYTES];
        let Ok(file) = File::open(entry.path()) else {
            continue;
        };
        let got = read_head(file, &mut head);
        if antigravity::head_pid(&head[..got]) == Some(agent.pid) {
            found = Some((made, name.to_owned()));
        }
    }
    found.map(|(_, name)| name)
}

fn read_head(mut file: File, head: &mut [u8]) -> usize {
    let mut got = 0;
    while got < head.len() {
        match file.read(&mut head[got..]) {
            Ok(0) => break,
            Ok(n) => got += n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(_) => break,
        }
    }
    got
}

/// Antigravity's hooks are the ones with steps of their own (module doc).
fn is_antigravity(agent: &Agent) -> bool {
    std::ptr::eq(agent, &antigravity::AGENT)
}

/// When `agy` started this hook: the start of the process it ran the hook in (`cmd /c`), or the hook's own start if
/// it ran the hook itself. A cancel line logged at or after it ends this turn.
fn turn_at(walk: &Walk, found: &Found, me: ProcessRef) -> FileTime {
    walk.parent
        .filter(|shell| *shell != found.process)
        .unwrap_or(me)
        .start
}

/// The log of a hook that must not write: an elevated one, whose log folder a Medium process could swap for a link.
struct Quiet;

impl Log for Quiet {
    fn line(&self, _: Level, _: &'static str, _: &str) {}
}

/// Where the hook's lines go: nowhere above Medium, else the log, opened at the first line.
struct Lines<'a> {
    env: &'a dyn Env,
    medium: bool,
}

impl Lines<'_> {
    fn line(&self, level: Level, text: &str) {
        self.sink().line(level, AREA, text);
    }

    fn sink(&self) -> &dyn Log {
        if self.medium { self.env.log() } else { &Quiet }
    }
}

fn run(env: &dyn Env, agent: &str, event: &str, values: &[String], dll: Option<WinError>) {
    // File work only at Medium or lower (step 4): the folders inherit the profile's rights, so any Medium process of
    // the user could swap them for a junction, and an elevated write or delete there would lend it rights it lacks.
    // A token that does not read counts as elevated.
    let medium = matches!(env.above_medium(), Ok(false));
    let log = Lines { env, medium };
    // An elevated uninstall's Medium copy (`agent_hooks::on_uninstall`). It has no input, and `agent_hooks` knows its
    // own agents, so the word goes to it as given.
    if event == REMOVE {
        env.remove_hooks(agent, log.sink());
        // With the hooks gone no `stop` comes for a chat mid-turn (review m5). Also the elevated hook's own hand-off
        // when it found its input on a command line but no agent process to name.
        if medium && let Some(known) = AGENTS.iter().copied().find(|known| known.name == agent) {
            end_chats(env, &log, &format!("{} {REMOVE}", known.name), known);
        }
        return;
    }
    let Some(agent) = AGENTS.iter().copied().find(|known| known.name == agent) else {
        env.drain_input();
        return;
    };
    let Some(rule) = agent.event(event) else {
        env.drain_input();
        return;
    };
    let name = format!("{} {}", agent.name, rule.name);
    if let Some(e) = dll {
        log.line(
            Level::Warn,
            &format!("{name}: DLL search not limited: {}", error_text(e)),
        );
    }
    match values {
        [] => hook(env, &log, &name, agent, rule, medium),
        values => checked_copy(env, &log, &name, agent, rule, values, medium),
    }
}

/// The agent's own run: steps 1 to 6.
fn hook(
    env: &dyn Env,
    log: &Lines<'_>,
    name: &str,
    agent: &'static Agent,
    rule: &EventRule,
    medium: bool,
) {
    let parsed = hooked::parse_input(agent, rule.name, env.input());
    env.drain_input();
    if env.test_panic() {
        panic!("a test panic, after the input was read");
    }
    let (list, me) = match (env.list(), env.me()) {
        (Ok(list), Ok(me)) => (list, me),
        // Without the list the hook's parent has no name, so its command line cannot be read either: `process` has
        // no call for this process's parent alone. Nothing is written, and the next hook run checks again.
        (Err(e), _) | (_, Err(e)) => {
            log.line(
                Level::Warn,
                &format!(
                    "{name}: cannot list processes, so neither the agent nor the command line is checked: {}",
                    error_text(e)
                ),
            );
            return;
        }
    };
    let listed: Vec<Listed<'_>> = list
        .iter()
        .map(|entry| Listed {
            pid: entry.pid,
            parent: entry.parent,
            start: entry.start,
            image: &entry.image,
        })
        .collect();
    let walk = hooked::walk(agent, &listed, me, &env.bases(), |p| env.image_path(p));
    // Before the agent gate (review M1): an agent version that moves its input to a command line may also run its hooks
    // from a process the walk does not know, and this check is the only backstop then.
    let on_command_line = input_on_command_line(env, log, name, agent, walk.parent);
    let Some(found) = walk.agent else {
        if on_command_line {
            log.line(Level::Warn, &format!(
                    "{name}: {} put its input on a command line, where audit logs keep it; no {} process above the hook, so nothing kept or written, the hook folder removed",
                    agent.label, agent.label
                ),
            );
            if medium {
                env.remove_hooks(agent.name, log.sink());
                end_chats(env, log, name, agent);
            } else {
                env.hand_off(&remove_args(env.global(), agent));
            }
        } else {
            log.line(
                Level::Info,
                &format!(
                    "{name}: no {} process above the hook; nothing written",
                    agent.label
                ),
            );
        }
        return;
    };
    // The IDE is not in v1 (Haik, 2026-11-06): its hub loads the same plugin, and its turns write nothing, quietly. A
    // command line with the input still blocks above and below, for the IDE too.
    if !on_command_line && is_antigravity(agent) && !antigravity::holds(found.image) {
        return;
    }
    let input = parsed.input.as_ref();
    let op = if on_command_line {
        log.line(Level::Warn, &format!(
                "{name}: {} put its input on a command line, where audit logs keep it; nothing kept, {} written, the hook folder removed",
                agent.label,
                hooked::BLOCKED_FILE
            ),
        );
        Op::Problem(Problem::CommandLine)
    } else {
        match input
            .map_err(|p| *p)
            .and_then(|input| hooked::resolve(rule, input))
        {
            Ok(op) => op,
            Err(problem) => {
                log_problem(log, name, agent, problem, parsed.version.as_deref());
                Op::Problem(Problem::Input)
            }
        }
    };
    let (chat, turn) = match (op, input) {
        (Op::Problem(_), _) | (_, Err(_)) => (String::new(), String::new()),
        (_, Ok(input)) => (input.chat.clone(), input.turn.clone()),
    };
    // Found here, before a hand-off, since the Medium copy starts later than the moment the mark names. None found:
    // the file names no log, and the source holds nothing for it and shows "update Wakeshot".
    let mark = match op {
        Op::Write(State::Working) if is_antigravity(agent) => {
            env.agy_log(found.process).map(|name| LogMark {
                name,
                turn_at: turn_at(&walk, &found, me),
            })
        }
        _ => None,
    };
    let checked = Checked {
        chat,
        turn,
        process: found.process,
        image: found.image,
        op,
        deadline_wall_ms: env.now_wall_ms() + i64::from(HAND_OFF_MS),
        log: mark,
    };
    if medium {
        apply(env, log, name, agent, rule, &checked, None);
    } else {
        env.hand_off(&hand_off_args(env.global(), agent, rule, &checked));
    }
}

/// The runtime transport check ("Privacy"): the hook's parent is the shell the agent started, so its command line
/// holds the input when the agent put it there. The text is searched and dropped inside `process`.
fn input_on_command_line(
    env: &dyn Env,
    log: &Lines<'_>,
    name: &str,
    agent: &Agent,
    parent: Option<ProcessRef>,
) -> bool {
    let Some(parent) = parent else {
        return false;
    };
    match env.command_line_has(parent, agent.payload_key) {
        Ok(found) => found,
        // The static check of each Cursor version stays the main gate.
        Err(e) => {
            log.line(
                Level::Warn,
                &format!(
                    "{name}: cannot read the parent's command line: {}",
                    error_text(e)
                ),
            );
            false
        }
    }
}

fn log_problem(
    log: &Lines<'_>,
    name: &str,
    agent: &Agent,
    problem: InputProblem,
    version: Option<&str>,
) {
    log.line(
        Level::Warn,
        &format!(
            "{name}: {problem}; {} {}; {} written",
            agent.label,
            version.unwrap_or("of an unknown version"),
            hooked::PROBLEM_FILE
        ),
    );
}

fn hand_off_args(
    global: Global,
    agent: &Agent,
    rule: &EventRule,
    checked: &Checked,
) -> Vec<OsString> {
    let mut args = Vec::new();
    if global.dry_run {
        args.push(OsString::from(DRY_RUN));
    }
    args.extend([HOOK, agent.name, rule.name, CHECKED].map(OsString::from));
    args.extend(checked.values().into_iter().map(OsString::from));
    args
}

/// `hook <agent> remove` for the Medium copy: it needs no checked values, since it reads no input.
fn remove_args(global: Global, agent: &Agent) -> Vec<OsString> {
    let mut args = Vec::new();
    if global.dry_run {
        args.push(OsString::from(DRY_RUN));
    }
    args.extend([HOOK, agent.name, REMOVE].map(OsString::from));
    args
}

/// The Medium copy of an elevated hook. Any process of the user can run this form with any values, so each is checked
/// again, and so is the token: without Explorer, `spawn_outside_tree` starts a plain child, still elevated.
fn checked_copy(
    env: &dyn Env,
    log: &Lines<'_>,
    name: &str,
    agent: &'static Agent,
    rule: &EventRule,
    values: &[String],
    medium: bool,
) {
    if !medium {
        return;
    }
    let Some(checked) = Checked::parse(agent, rule, values) else {
        log.line(
            Level::Warn,
            &format!("{name}: the checked values do not pass; nothing written"),
        );
        return;
    };
    // Only an older elevated hook or a hand-made call names the IDE's hub here: the IDE is not held.
    let blocked = checked.op == Op::Problem(Problem::CommandLine);
    if !blocked && is_antigravity(agent) && !antigravity::holds(checked.image) {
        return;
    }
    apply(
        env,
        log,
        name,
        agent,
        rule,
        &checked,
        Some(checked.deadline_wall_ms),
    );
}

/// Step 5, at Medium: the event's work on `agents\<agent>\`, under the mutex, then the prune.
fn apply(
    env: &dyn Env,
    log: &Lines<'_>,
    name: &str,
    agent: &Agent,
    rule: &EventRule,
    checked: &Checked,
    deadline: Option<i64>,
) {
    let Some(data) = env.data() else {
        log.line(
            Level::Warn,
            &format!("{name}: no data folder; nothing written"),
        );
        return;
    };
    let dir = hooked::agent_folder(data.root(), agent);
    // A stuck holder costs a second, never the agent's turn.
    let mutex = NamedMutex::open(MUTEX).ok();
    let _held = mutex
        .as_ref()
        .and_then(|mutex| mutex.take(MUTEX_WAIT_MS).ok().flatten());
    if let Err(e) = std::fs::create_dir_all(&dir) {
        log.line(
            Level::Warn,
            &format!("{name}: cannot make the agent's folder: {}", e.kind()),
        );
        return;
    }
    let chat = || dir.join(hooked::chat_file(&checked.chat));
    let done = match checked.op {
        Op::Write(state) => write_chat(env, log, name, &chat(), checked, state, deadline),
        Op::DeleteIfTurn => match store::read_whole(&chat()) {
            Ok(Some(bytes)) if hooked::names_turn(&bytes, &checked.turn) => {
                store::remove(&chat()).map(drop)
            }
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        },
        Op::Delete => store::remove(&chat()).map(drop),
        Op::Problem(problem) => store::write_atomic(
            &dir.join(problem.file_name()),
            &hooked::problem_text(checked.process, problem),
        ),
    };
    if let Err(e) = done {
        log.line(
            Level::Warn,
            &format!("{name}: the chat's file did not change: {}", error_text(e)),
        );
    }
    // So the leak ends with this one prompt: the tray then finds the folder gone and never adds it again ("Privacy").
    if checked.op == Op::Problem(Problem::CommandLine) {
        env.remove_hooks(agent.name, log.sink());
        end_chats_in(log, name, &dir);
    }
    if rule.prune {
        prune(env, log, name, &dir);
    }
}

/// The agent's hooks are gone, so no `stop` or session end comes for a chat of another tab that is mid-turn, and its
/// file would hold while the process it names lives (review m5): each chat's file goes now, under the mutex.
fn end_chats(env: &dyn Env, log: &Lines<'_>, name: &str, agent: &Agent) {
    let Some(data) = env.data() else {
        return;
    };
    let mutex = NamedMutex::open(MUTEX).ok();
    let _held = mutex
        .as_ref()
        .and_then(|mutex| mutex.take(MUTEX_WAIT_MS).ok().flatten());
    end_chats_in(log, name, &hooked::agent_folder(data.root(), agent));
}

/// By the id rule only: a plain file named `<chat id>.json`. `problem.json` and `blocked.json` stay for the notice.
fn end_chats_in(log: &Lines<'_>, name: &str, dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut ended = 0;
    for entry in entries.flatten() {
        if !entry.file_type().is_ok_and(|kind| kind.is_file()) {
            continue;
        }
        let file_name = entry.file_name();
        let chat = file_name.to_str().and_then(|n| n.strip_suffix(".json"));
        let is_chat = chat.is_some_and(|chat| hooked::chat_id(chat).as_deref() == Some(chat));
        if is_chat && store::remove(&entry.path()).unwrap_or(false) {
            ended += 1;
        }
    }
    if ended > 0 {
        log.line(
            Level::Info,
            &format!("{name}: removed {ended} files of chats the removed hooks would have ended"),
        );
    }
}

fn write_chat(
    env: &dyn Env,
    log: &Lines<'_>,
    name: &str,
    path: &Path,
    checked: &Checked,
    state: State,
    deadline: Option<i64>,
) -> Result<(), WinError> {
    let text = hooked::state_text_marked(
        checked.process,
        &checked.turn,
        checked.image.label,
        state,
        checked.log.as_ref(),
    );
    if store::read_whole(path).ok().flatten().as_deref() == Some(&text[..]) {
        return Ok(());
    }
    // The turn may have ended while this copy started; its `stop` has run then, and a write now would hold for good.
    if deadline.is_some_and(|deadline| env.now_wall_ms() > deadline) {
        log.line(
            Level::Warn,
            &format!("{name}: the Medium copy came past its deadline; not written"),
        );
        return Ok(());
    }
    store::write_atomic(path, &text)
}

/// Deletes a version 1 file whose process is gone and `write_atomic` temp files older than a minute; nothing else.
/// Only plain files: a link or a folder in the agent's folder is never followed or deleted.
fn prune(env: &dyn Env, log: &Lines<'_>, name: &str, dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let now = SystemTime::now();
    let mut pruned = 0;
    for entry in entries.flatten() {
        if !entry.file_type().is_ok_and(|kind| kind.is_file()) {
            continue;
        }
        let file_name = entry.file_name();
        let Some(file_name) = file_name.to_str() else {
            continue;
        };
        let path = entry.path();
        let ended = if file_name.ends_with(".json") {
            store::read_whole(&path)
                .ok()
                .flatten()
                .and_then(|bytes| hooked::named_process(&bytes))
                .is_some_and(|process| env.alive(process) == Alive::Gone)
        } else {
            store::temp_target(file_name).is_some()
                && entry
                    .metadata()
                    .and_then(|meta| meta.modified())
                    .ok()
                    .and_then(|written| now.duration_since(written).ok())
                    .is_some_and(|age| age > TEMP_AGE)
        };
        if ended && store::remove(&path).unwrap_or(false) {
            pruned += 1;
        }
    }
    if pruned > 0 {
        log.line(
            Level::Info,
            &format!("{name}: removed {pruned} files of ended chats"),
        );
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::collections::BTreeSet;
    use std::fs;

    use wakeshot_windows::log::MemoryLog;

    use super::*;
    use crate::status::fixture::TempData;

    const CHAT: &str = "a3f1c2d4-5b6e-4f70-8a9b-0c1d2e3f4a5b";
    const TURN: &str = "7d2e9c41-0b8a-4e55-9f13-6a2c4b7d8e90";
    /// A word with a space, so it is neither a chat id, a turn id nor a version: none of the kept fields may keep it.
    const MARKER: &str = r#""QXV7 marker""#;
    const MARKER_NUMBER: &str = "77317731";
    const MARKER_OBJECT: &str = r#"{"QXV7 marker":["QXV7 marker",77317731]}"#;
    const MARKER_WORDS: [&str; 3] = ["QXV7", "marker", "77317731"];

    /// The fixtures in `wakeshot-core/tests/fixtures/cursor/` as key and raw JSON value, so a test can swap one value:
    /// the CLI has no JSON crate, and one more would change `Cargo.lock`.
    const EDITOR_PROMPT: &[(&str, &str)] = &[
        (
            "conversation_id",
            r#""A3F1C2D4-5B6E-4F70-8A9B-0C1D2E3F4A5B""#,
        ),
        ("generation_id", r#""7d2e9c41-0b8a-4e55-9f13-6a2c4b7d8e90""#),
        ("model", r#""default""#),
        ("prompt", r#""Refactor the parser and add tests""#),
        (
            "attachments",
            r#"[{"type":"file","file_path":"c:\\Users\\dev\\project\\src\\parser.rs"}]"#,
        ),
        ("session_id", r#""A3F1C2D4-5B6E-4F70-8A9B-0C1D2E3F4A5B""#),
        ("hook_event_name", r#""beforeSubmitPrompt""#),
        ("cursor_version", r#""3.27.12""#),
        ("workspace_roots", r#"["/c:/Users/dev/project"]"#),
        ("user_email", r#""dev@example.com""#),
        (
            "transcript_path",
            r#""c:\\Users\\dev\\.cursor\\projects\\c-Users-dev-project\\agent-transcripts\\a3f1c2d4-5b6e-4f70-8a9b-0c1d2e3f4a5b.jsonl""#,
        ),
    ];
    const EDITOR_STOP: &[(&str, &str)] = &[
        (
            "conversation_id",
            r#""A3F1C2D4-5B6E-4F70-8A9B-0C1D2E3F4A5B""#,
        ),
        ("generation_id", r#""7d2e9c41-0b8a-4e55-9f13-6a2c4b7d8e90""#),
        ("model", r#""default""#),
        ("status", r#""completed""#),
        ("loop_count", "0"),
        ("input_tokens", "18234"),
        ("session_id", r#""A3F1C2D4-5B6E-4F70-8A9B-0C1D2E3F4A5B""#),
        ("hook_event_name", r#""stop""#),
        ("cursor_version", r#""3.27.12""#),
        ("workspace_roots", r#"["/c:/Users/dev/project"]"#),
        ("user_email", r#""dev@example.com""#),
        ("transcript_path", "null"),
    ];
    const EDITOR_END: &[(&str, &str)] = &[
        (
            "conversation_id",
            r#""a3f1c2d4-5b6e-4f70-8a9b-0c1d2e3f4a5b""#,
        ),
        ("session_id", r#""a3f1c2d4-5b6e-4f70-8a9b-0c1d2e3f4a5b""#),
        ("reason", r#""user_close""#),
        ("duration_ms", "482113"),
        ("error_message", r#""""#),
        ("hook_event_name", r#""sessionEnd""#),
        ("cursor_version", r#""3.27.12""#),
        ("workspace_roots", r#"["/c:/Users/dev/project"]"#),
        ("user_email", r#""dev@example.com""#),
        ("transcript_path", "null"),
    ];
    const CLI_PROMPT: &[(&str, &str)] = &[
        (
            "conversation_id",
            r#""e4b7a1c9-2d3f-4a6b-8c9d-1e2f3a4b5c6d""#,
        ),
        ("generation_id", r#""0c1d2e3f-4a5b-4c6d-8e9f-a0b1c2d3e4f5""#),
        ("model", r#""auto""#),
        ("prompt", r#""Run the test suite and fix what fails""#),
        ("attachments", "[]"),
        ("hook_event_name", r#""beforeSubmitPrompt""#),
        ("cursor_version", r#""2026.11.05-86047df""#),
        ("workspace_roots", r#"["C:\\Users\\dev\\project"]"#),
        ("user_email", r#""dev@example.com""#),
        (
            "transcript_path",
            r#""C:\\Users\\dev\\.cursor\\projects\\C-Users-dev-project\\agent-transcripts\\e4b7a1c9-2d3f-4a6b-8c9d-1e2f3a4b5c6d.jsonl""#,
        ),
    ];

    /// A flat JSON object, each value as raw JSON text, with the values of `swaps` in place of the fields' own. A swap
    /// for a key the fields lack adds it; its key is then raw JSON text too.
    fn object(fields: &[(&str, &str)], swaps: &[(&str, &str)]) -> Vec<u8> {
        let swapped = |key: &str| swaps.iter().find(|&&(k, _)| k == key).map(|&(_, v)| v);
        let mut parts: Vec<String> = fields
            .iter()
            .map(|&(key, value)| format!(r#""{key}":{}"#, swapped(key).unwrap_or(value)))
            .collect();
        for &(key, value) in swaps {
            if !fields.iter().any(|&(k, _)| k == key) {
                parts.push(format!("{key}:{value}"));
            }
        }
        format!("{{{}}}", parts.join(",")).into_bytes()
    }

    /// A JSON string of `text`, which holds no quote or control character.
    fn string(text: &str) -> String {
        format!(r#""{}""#, text.replace('\\', r"\\"))
    }

    const ME: ProcessRef = ProcessRef {
        pid: 900,
        start: FileTime(9_000),
    };
    const SHELL: ProcessRef = ProcessRef {
        pid: 800,
        start: FileTime(8_000),
    };
    const EDITOR: ProcessRef = ProcessRef {
        pid: 700,
        start: FileTime(7_000),
    };

    fn fixture(name: &str) -> Vec<u8> {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("wakeshot-core")
            .join("tests")
            .join("fixtures")
            .join("cursor")
            .join(name);
        fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
    }

    fn entry(process: ProcessRef, parent: u32, image: &str) -> ProcessEntry {
        ProcessEntry {
            pid: process.pid,
            parent,
            start: process.start,
            image: image.to_owned(),
        }
    }

    /// The hook, its PowerShell, a Cursor extension host and the window above it.
    fn editor_chain() -> Vec<ProcessEntry> {
        vec![
            entry(ME, SHELL.pid, "wakeshot.exe"),
            entry(SHELL, EDITOR.pid, "powershell.exe"),
            entry(EDITOR, 600, "Cursor.exe"),
            entry(
                ProcessRef {
                    pid: 600,
                    start: FileTime(6_000),
                },
                1,
                "Cursor.exe",
            ),
        ]
    }

    struct Fake {
        temp: TempData,
        global: Global,
        input: RefCell<Vec<u8>>,
        drained: Cell<u32>,
        list: Vec<ProcessEntry>,
        paths: Vec<(u32, PathBuf)>,
        command_line: Result<bool, WinError>,
        above: Result<bool, WinError>,
        handed: RefCell<Vec<Vec<OsString>>>,
        gone: Vec<ProcessRef>,
        now: Cell<i64>,
        log: MemoryLog,
        removed: RefCell<Vec<String>>,
        /// A profile folder of its own, outside the data folder, for `agy`'s logs.
        home: TempData,
    }

    impl Fake {
        fn new(tag: &str) -> Self {
            Self {
                home: TempData::new(&format!("hook-{tag}-home")),
                temp: TempData::new(&format!("hook-{tag}")),
                global: Global { dry_run: false },
                input: RefCell::new(Vec::new()),
                drained: Cell::new(0),
                list: editor_chain(),
                paths: Vec::new(),
                command_line: Ok(false),
                above: Ok(false),
                handed: RefCell::new(Vec::new()),
                gone: Vec::new(),
                now: Cell::new(1_762_400_000_000),
                log: MemoryLog::default(),
                removed: RefCell::new(Vec::new()),
            }
        }

        fn run(&self, event: &str, input: &[u8]) {
            *self.input.borrow_mut() = input.to_vec();
            run(self, "cursor", event, &[], None);
        }

        fn run_checked(&self, event: &str, values: &[String]) {
            run(self, "cursor", event, values, None);
        }

        fn folder(&self) -> PathBuf {
            hooked::agent_folder(self.temp.dir.root(), &cursor::AGENT)
        }

        fn chat_path(&self) -> PathBuf {
            self.folder().join(hooked::chat_file(CHAT))
        }

        /// Every file under the data folder, by its path below it.
        fn files(&self) -> BTreeSet<String> {
            fn walk(dir: &Path, root: &Path, out: &mut BTreeSet<String>) {
                for entry in fs::read_dir(dir).into_iter().flatten().flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        walk(&path, root, out);
                    } else {
                        out.insert(path.strip_prefix(root).unwrap().display().to_string());
                    }
                }
            }
            let mut out = BTreeSet::new();
            walk(self.temp.dir.root(), self.temp.dir.root(), &mut out);
            out
        }

        fn chat_file_names(&self) -> BTreeSet<String> {
            fs::read_dir(self.folder())
                .into_iter()
                .flatten()
                .flatten()
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        }

        fn lines(&self) -> Vec<String> {
            self.log
                .lines
                .lock()
                .unwrap()
                .iter()
                .map(|(_, _, text)| text.clone())
                .collect()
        }
    }

    impl Env for Fake {
        fn global(&self) -> Global {
            self.global
        }

        fn data(&self) -> Option<&DataDir> {
            Some(&self.temp.dir)
        }

        fn log(&self) -> &dyn Log {
            &self.log
        }

        fn input(&self) -> Box<dyn Read + '_> {
            Box::new(io::Cursor::new(self.input.borrow().clone()))
        }

        fn drain_input(&self) {
            self.drained.set(self.drained.get() + 1);
        }

        fn list(&self) -> Result<Vec<ProcessEntry>, WinError> {
            Ok(self.list.clone())
        }

        fn me(&self) -> Result<ProcessRef, WinError> {
            Ok(ME)
        }

        fn image_path(&self, process: ProcessRef) -> Option<PathBuf> {
            self.paths
                .iter()
                .find(|(pid, _)| *pid == process.pid)
                .map(|(_, path)| path.clone())
        }

        fn command_line_has(&self, process: ProcessRef, needle: &str) -> Result<bool, WinError> {
            // By pid: a test may start each of `agy`'s `cmd /c` anew.
            assert_eq!(process.pid, SHELL.pid, "the hook's own parent");
            assert!(
                AGENTS.iter().any(|agent| agent.payload_key == needle),
                "{needle}"
            );
            self.command_line
        }

        fn above_medium(&self) -> Result<bool, WinError> {
            self.above
        }

        fn hand_off(&self, args: &[OsString]) {
            self.handed.borrow_mut().push(args.to_vec());
        }

        fn alive(&self, process: ProcessRef) -> Alive {
            if self.gone.contains(&process) {
                Alive::Gone
            } else {
                Alive::Yes
            }
        }

        fn now_wall_ms(&self) -> i64 {
            self.now.get()
        }

        fn bases(&self) -> Bases {
            Bases {
                local_app_data: Some(local()),
                profile: None,
            }
        }

        /// Records the call, so no test touches the real profile; the log says which log it got.
        fn remove_hooks(&self, agent: &str, log: &dyn Log) {
            self.removed.borrow_mut().push(agent.to_owned());
            log.line(Level::Info, AREA, &format!("removed {agent}"));
        }

        fn test_panic(&self) -> bool {
            false
        }

        fn agy_log(&self, agent: ProcessRef) -> Option<String> {
            find_log(&antigravity::log_dir(self.home.dir.root()), agent)
        }
    }

    fn local() -> PathBuf {
        PathBuf::from(r"C:\Users\dev\AppData\Local")
    }

    fn prompt() -> Vec<u8> {
        fixture("editor-before-submit-prompt.json")
    }

    fn state(fake: &Fake) -> hooked::StateFile {
        match hooked::read_state(&fs::read(fake.chat_path()).unwrap()) {
            hooked::FileRead::V1(file) => file,
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_prompt_writes_the_chats_file_and_only_its_own_stop_deletes_it() {
        let fake = Fake::new("turn");
        fake.run("beforeSubmitPrompt", &prompt());
        let file = state(&fake);
        assert_eq!(
            (
                file.pid,
                file.proc_start,
                file.generation.as_str(),
                file.label.as_str(),
                file.state
            ),
            (EDITOR.pid, EDITOR.start, TURN, "Cursor", State::Working),
            "the nearest Cursor.exe, never the hook or its shell"
        );
        assert_eq!(fake.drained.get(), 1, "the pipe is read to its end");

        // A late stop of an older turn keeps the file.
        let late = object(
            EDITOR_STOP,
            &[("generation_id", r#""11111111-2222-4333-8444-555555555555""#)],
        );
        fake.run("stop", &late);
        assert!(fake.chat_path().exists());
        fake.run("stop", &fixture("editor-stop-aborted.json"));
        assert!(!fake.chat_path().exists());

        fake.run("beforeSubmitPrompt", &prompt());
        fake.run("sessionEnd", &fixture("editor-session-end.json"));
        assert!(!fake.chat_path().exists());
        assert_eq!(fake.chat_file_names(), BTreeSet::new());
        assert!(fake.handed.borrow().is_empty());
    }

    /// What the editor 3.27.12 really sent (`wakeshot-core/tests/fixtures/cursor/live/`, `p4-live-cursor`).
    fn live(name: &str) -> Vec<u8> {
        fixture(&format!(r"live\{name}"))
    }

    #[test]
    fn a_running_chats_closed_tab_keeps_it_working_until_its_own_stop() {
        let fake = Fake::new("live-tab");
        let chat = fake
            .folder()
            .join(hooked::chat_file("7f2e1a4c-9d63-4b5a-9caf-3e8d4a6b9c12"));
        fake.run(
            "beforeSubmitPrompt",
            &live("r3-3-before-submit-prompt-b.json"),
        );
        let written = fs::read(&chat).unwrap();
        // Ctrl+F4 on the running chat: Cursor says `generating`, and the turn went on for 50 s in the live test.
        let closed = live("r3-4-session-end-running-b.json");
        fake.run("sessionEnd", &closed);
        assert_eq!(fs::read(&chat).unwrap(), written);
        fake.run("stop", &live("r3-6-stop-completed-b.json"));
        assert!(!chat.exists(), "the turn's own stop ends it");

        // A tab closed after its turn ended, or a status Wakeshot does not know, ends the chat at once.
        let ended = String::from_utf8(closed.clone())
            .unwrap()
            .replace(r#""final_status":"generating""#, r#""final_status":"none""#);
        assert_ne!(ended.as_bytes(), &closed[..]);
        fake.run(
            "beforeSubmitPrompt",
            &live("r3-3-before-submit-prompt-b.json"),
        );
        fake.run("sessionEnd", ended.as_bytes());
        assert!(!chat.exists());
        assert!(fake.handed.borrow().is_empty());

        // Elevated, the Medium copy gets the same op.
        let mut up = Fake::new("live-tab-up");
        up.above = Ok(true);
        up.run("sessionEnd", &closed);
        let handed = up.handed.borrow();
        assert_eq!(handed.len(), 1);
        assert!(handed[0].iter().any(|arg| arg == "working"), "{handed:?}");
    }

    #[test]
    fn the_cli_is_named_by_its_node_in_the_clis_folder() {
        let mut fake = Fake::new("cli");
        let node = ProcessRef {
            pid: 750,
            start: FileTime(7_500),
        };
        fake.list = vec![
            entry(ME, SHELL.pid, "wakeshot.exe"),
            entry(SHELL, node.pid, "powershell.exe"),
            entry(node, 1, "node.exe"),
        ];
        fake.paths = vec![(
            node.pid,
            local().join(r"cursor-agent\versions\2026.11.05-86047df\node.exe"),
        )];
        let input = fixture("cli-before-submit-prompt.json");
        fake.run("beforeSubmitPrompt", &input);
        let path = fake
            .folder()
            .join("e4b7a1c9-2d3f-4a6b-8c9d-1e2f3a4b5c6d.json");
        let hooked::FileRead::V1(file) = hooked::read_state(&fs::read(path).unwrap()) else {
            panic!("written");
        };
        assert_eq!((file.pid, file.label.as_str()), (node.pid, "Cursor CLI"));

        // Another Node is not the CLI.
        let mut fake = Fake::new("other-node");
        fake.list = vec![
            entry(ME, SHELL.pid, "wakeshot.exe"),
            entry(SHELL, node.pid, "powershell.exe"),
            entry(node, 1, "node.exe"),
        ];
        fake.paths = vec![(node.pid, PathBuf::from(r"C:\Program Files\nodejs\node.exe"))];
        fake.run("beforeSubmitPrompt", &input);
        assert_eq!(fake.files(), BTreeSet::new());
        assert_eq!(
            fake.lines(),
            ["cursor beforeSubmitPrompt: no Cursor process above the hook; nothing written"]
        );
    }

    #[test]
    fn without_a_cursor_process_above_it_the_hook_writes_nothing() {
        let mut fake = Fake::new("none");
        fake.list = vec![
            entry(ME, SHELL.pid, "wakeshot.exe"),
            entry(SHELL, 1, "cmd.exe"),
        ];
        for event in ["beforeSubmitPrompt", "stop", "sessionEnd"] {
            fake.run(event, &prompt());
        }
        assert_eq!(fake.files(), BTreeSet::new());
        assert_eq!(fake.lines().len(), 3);
    }

    #[test]
    fn unreadable_input_writes_problem_json_and_never_a_name_from_the_input() {
        let mut cases: Vec<Vec<u8>> = [
            r"..\..\..\Windows\x",
            "../../x",
            "CON",
            "COM1",
            "nul",
            "a3f1c2d4-5b6e-4f70-8a9b-0c1d2e3f4a5b.json",
            r"a3f1c2d4-5b6e-4f70-8a9b-0c1d2e3f4a5b\..\x",
            &"a".repeat(5_000),
            "",
        ]
        .iter()
        .map(|chat| object(EDITOR_PROMPT, &[("conversation_id", &string(chat))]))
        .collect();
        cases.extend([b"".to_vec(), b"not json".to_vec(), b"[]".to_vec()]);
        cases.push(object(EDITOR_PROMPT, &[("hook_event_name", r#""stop""#)]));
        for (at, input) in cases.iter().enumerate() {
            let fake = Fake::new(&format!("crafted-{at}"));
            fake.run("beforeSubmitPrompt", input);
            assert_eq!(
                fake.files(),
                BTreeSet::from([r"agents\cursor\problem.json".to_owned()]),
                "case {at}"
            );
            let lines = fake.lines();
            assert_eq!(lines.len(), 1, "case {at}: {lines:?}");
            assert!(
                lines[0].starts_with("cursor beforeSubmitPrompt: "),
                "{lines:?}"
            );
            assert!(lines[0].ends_with("; problem.json written"), "{lines:?}");
        }
    }

    #[test]
    fn a_problem_line_names_the_class_and_place_and_a_version_only_when_it_is_one() {
        let fake = Fake::new("problem-line");
        fake.run(
            "beforeSubmitPrompt",
            &object(EDITOR_PROMPT, &[("conversation_id", "7")]),
        );
        fake.run(
            "beforeSubmitPrompt",
            &object(EDITOR_PROMPT, &[("conversation_id", r#""CON""#)]),
        );
        let swaps = [
            ("conversation_id", r#""CON""#),
            ("cursor_version", r#""beta build""#),
        ];
        fake.run("beforeSubmitPrompt", &object(EDITOR_PROMPT, &swaps));
        let lines = fake.lines();
        assert!(
            lines[0].starts_with(
                "cursor beforeSubmitPrompt: the input does not parse (data at line 1, column "
            ),
            "{lines:?}"
        );
        assert!(lines[0].ends_with("; Cursor of an unknown version; problem.json written"));
        assert_eq!(
            lines[1],
            "cursor beforeSubmitPrompt: the chat id is not a UUID; Cursor 3.27.12; problem.json written"
        );
        assert_eq!(
            lines[2],
            "cursor beforeSubmitPrompt: the chat id is not a UUID; Cursor of an unknown version; problem.json written"
        );
    }

    #[test]
    fn the_input_on_the_parents_command_line_blocks_every_write() {
        let mut fake = Fake::new("blocked");
        fake.command_line = Ok(true);
        fake.run("beforeSubmitPrompt", &prompt());
        fake.run("stop", &fixture("editor-stop-completed.json"));
        assert_eq!(
            fake.files(),
            BTreeSet::from([r"agents\cursor\blocked.json".to_owned()])
        );
        let hooked::FileRead::V1(file) =
            hooked::read_problem(&fs::read(fake.folder().join("blocked.json")).unwrap())
        else {
            panic!("written");
        };
        assert_eq!((file.pid, file.problem), (EDITOR.pid, Problem::CommandLine));
        assert_eq!(
            *fake.removed.borrow(),
            ["cursor", "cursor"],
            "the plugin goes too"
        );
        assert_eq!(
            fake.lines()[..2],
            [
                "cursor beforeSubmitPrompt: Cursor put its input on a command line, where audit logs keep it; nothing kept, blocked.json written, the hook folder removed",
                "removed cursor",
            ]
        );
        // A command line that cannot be read does not stop the hook.
        let mut fake = Fake::new("unread-line");
        fake.command_line = Err(WinError {
            call: "NtQueryInformationProcess",
            code: 5,
        });
        fake.run("beforeSubmitPrompt", &prompt());
        assert!(fake.chat_path().exists());
        assert_eq!(
            fake.lines(),
            [
                "cursor beforeSubmitPrompt: cannot read the parent's command line: NtQueryInformationProcess failed with 5"
            ]
        );
    }

    const OTHER_CHAT: &str = "11111111-2222-4333-8444-555555555555";

    /// Another tab's chat, mid-turn, and a file of a name the id rule does not give.
    fn other_files(fake: &Fake) {
        fs::create_dir_all(fake.folder()).unwrap();
        let text = hooked::state_text(EDITOR, TURN, "Cursor", State::Working);
        fs::write(fake.folder().join(format!("{OTHER_CHAT}.json")), text).unwrap();
        fs::write(fake.folder().join("notes.json"), "{}").unwrap();
    }

    #[test]
    fn the_command_line_check_runs_without_an_agent_process_too() {
        let mut fake = Fake::new("line-no-agent");
        fake.list = vec![
            entry(ME, SHELL.pid, "wakeshot.exe"),
            entry(SHELL, 1, "powershell.exe"),
        ];
        fake.command_line = Ok(true);
        other_files(&fake);
        fake.run("beforeSubmitPrompt", &prompt());
        assert_eq!(*fake.removed.borrow(), ["cursor"], "the plugin goes");
        assert_eq!(
            fake.files(),
            BTreeSet::from([r"agents\cursor\notes.json".to_owned()]),
            "no process to name, so no blocked.json; the other chat's file ends with the hooks"
        );
        assert_eq!(
            fake.lines(),
            [
                "cursor beforeSubmitPrompt: Cursor put its input on a command line, where audit logs keep it; no Cursor process above the hook, so nothing kept or written, the hook folder removed",
                "removed cursor",
                "cursor beforeSubmitPrompt: removed 1 files of chats the removed hooks would have ended",
            ]
        );

        // Elevated: nothing here, the removal goes to the Medium copy's `remove` form.
        let mut fake = Fake::new("line-no-agent-high");
        fake.list = vec![
            entry(ME, SHELL.pid, "wakeshot.exe"),
            entry(SHELL, 1, "powershell.exe"),
        ];
        fake.command_line = Ok(true);
        fake.above = Ok(true);
        other_files(&fake);
        fake.run("stop", &fixture("editor-stop-completed.json"));
        assert!(fake.removed.borrow().is_empty());
        assert!(fake.lines().is_empty());
        assert_eq!(fake.files().len(), 2, "nothing touched");
        assert_eq!(*fake.handed.borrow(), [["hook", "cursor", "remove"]]);
    }

    #[test]
    fn the_blocked_path_and_the_remove_form_end_every_chat_of_the_agent() {
        let mut fake = Fake::new("blocked-ends");
        other_files(&fake);
        fake.command_line = Ok(true);
        fake.run("beforeSubmitPrompt", &prompt());
        assert_eq!(
            fake.files(),
            BTreeSet::from([
                r"agents\cursor\blocked.json".to_owned(),
                r"agents\cursor\notes.json".to_owned(),
            ])
        );

        // The Medium copy of an elevated uninstall or hook: `hook cursor remove`.
        let fake = Fake::new("remove-ends");
        other_files(&fake);
        fake.run_checked("remove", &[]);
        assert_eq!(*fake.removed.borrow(), ["cursor"]);
        assert_eq!(
            fake.files(),
            BTreeSet::from([r"agents\cursor\notes.json".to_owned()])
        );
        // Above Medium the form removes nothing itself and ends no chat.
        let mut fake = Fake::new("remove-high");
        fake.above = Ok(true);
        other_files(&fake);
        fake.run_checked("remove", &[]);
        assert_eq!(fake.files().len(), 2);
    }

    #[test]
    fn an_elevated_hook_touches_no_file_and_hands_its_checked_values_to_a_medium_copy() {
        for (tag, above, dry_run) in [
            ("high", Ok(true), false),
            ("high-dry", Ok(true), true),
            (
                "unknown",
                Err(WinError {
                    call: "GetTokenInformation",
                    code: 5,
                }),
                false,
            ),
        ] {
            let mut fake = Fake::new(tag);
            fake.above = above;
            fake.global = Global { dry_run };
            fake.run("beforeSubmitPrompt", &prompt());
            fake.command_line = Ok(true);
            fake.run("stop", &fixture("editor-stop-completed.json"));
            assert_eq!(fake.files(), BTreeSet::new(), "{tag}: not even a log line");
            assert!(fake.lines().is_empty(), "{tag}");
            assert!(
                fake.removed.borrow().is_empty(),
                "{tag}: the Medium copy removes"
            );
            let handed = fake.handed.borrow();
            let deadline = (fake.now.get() + 2_000).to_string();
            let mut prompt_args = vec![
                "hook",
                "cursor",
                "beforeSubmitPrompt",
                "--checked",
                CHAT,
                TURN,
                "700",
                "7000",
                "editor",
                "working",
                &deadline,
            ];
            let mut stop_args = vec![
                "hook",
                "cursor",
                "stop",
                "--checked",
                "",
                "",
                "700",
                "7000",
                "editor",
                "command-line",
                &deadline,
            ];
            if dry_run {
                prompt_args.insert(0, "--dry-run");
                stop_args.insert(0, "--dry-run");
            }
            assert_eq!(handed[0], prompt_args, "{tag}");
            assert_eq!(handed[1], stop_args, "{tag}");
        }
    }

    fn values(op: &str, deadline: i64) -> Vec<String> {
        [
            CHAT,
            TURN,
            "700",
            "7000",
            "editor",
            op,
            &deadline.to_string(),
        ]
        .map(str::to_owned)
        .to_vec()
    }

    #[test]
    fn the_medium_copy_rechecks_its_values_and_its_token() {
        let fake = Fake::new("copy");
        let deadline = fake.now.get() + 2_000;
        fake.run_checked("beforeSubmitPrompt", &values("working", deadline));
        let file = state(&fake);
        assert_eq!((file.pid, file.generation.as_str()), (700, TURN));
        fake.run_checked("stop", &values("none", deadline));
        assert!(!fake.chat_path().exists());
        let blocked = [
            "",
            "",
            "700",
            "7000",
            "editor",
            "command-line",
            &deadline.to_string(),
        ];
        fake.run_checked("stop", &blocked.map(str::to_owned));
        assert!(fake.folder().join("blocked.json").exists());
        assert_eq!(*fake.removed.borrow(), ["cursor"]);

        // A copy that is not Medium (no Explorer, so a plain child) writes nothing.
        let mut fake = Fake::new("copy-high");
        fake.above = Ok(true);
        fake.run_checked("beforeSubmitPrompt", &values("working", deadline));
        assert_eq!(fake.files(), BTreeSet::new());

        // Crafted values: nothing written, one line each.
        let fake = Fake::new("copy-crafted");
        let mut traversal = values("working", deadline);
        traversal[0] = r"..\..\x".to_owned();
        let mut wrong_op = values("none", deadline);
        wrong_op[0] = CHAT.to_owned();
        for crafted in [traversal, wrong_op, vec!["x".to_owned()]] {
            fake.run_checked("beforeSubmitPrompt", &crafted);
        }
        assert_eq!(fake.files(), BTreeSet::new());
        assert_eq!(fake.lines().len(), 3);
        assert!(
            fake.lines().iter().all(|l| l
                == "cursor beforeSubmitPrompt: the checked values do not pass; nothing written")
        );
    }

    #[test]
    fn a_copy_past_its_deadline_drops_a_write_but_still_deletes() {
        let fake = Fake::new("late");
        let deadline = fake.now.get() - 1;
        fake.run_checked("beforeSubmitPrompt", &values("working", deadline));
        assert!(!fake.chat_path().exists());
        assert_eq!(
            fake.lines(),
            ["cursor beforeSubmitPrompt: the Medium copy came past its deadline; not written"]
        );
        fake.run("beforeSubmitPrompt", &prompt());
        assert!(fake.chat_path().exists());
        fake.run_checked("stop", &values("none", deadline));
        assert!(!fake.chat_path().exists());
        fake.run_checked(
            "stop",
            &[
                "",
                "",
                "700",
                "7000",
                "editor",
                "input",
                &deadline.to_string(),
            ]
            .map(str::to_owned),
        );
        assert!(fake.folder().join("problem.json").exists());
    }

    #[test]
    fn the_prune_deletes_only_version_1_files_of_ended_processes_and_old_temp_files() {
        let mut fake = Fake::new("prune");
        let ended = ProcessRef {
            pid: 55,
            start: FileTime(5),
        };
        fake.gone = vec![ended];
        let dir = fake.folder();
        fs::create_dir_all(&dir).unwrap();
        let write = |name: &str, bytes: &[u8]| fs::write(dir.join(name), bytes).unwrap();
        let other_chat = "1b9e6b0e-3c1a-4f7e-9a51-2d64c1f0a7b3";
        let third_chat = "2b9e6b0e-3c1a-4f7e-9a51-2d64c1f0a7b3";
        write(
            &hooked::chat_file(other_chat),
            &hooked::state_text(ended, "t", "Cursor", State::Working),
        );
        write(
            &hooked::chat_file(third_chat),
            &hooked::state_text(EDITOR, "t", "Cursor", State::Working),
        );
        write("problem.json", &hooked::problem_text(ended, Problem::Input));
        write("newer.json", br#"{"version":2,"pid":55,"procStart":"5"}"#);
        write("junk.json", b"{");
        write("notes.txt", b"the user's");
        let old_temp = format!(
            "{}.55.0.0123456789abcdef.tmp",
            hooked::chat_file(other_chat)
        );
        let new_temp = format!(
            "{}.55.1.0123456789abcdef.tmp",
            hooked::chat_file(other_chat)
        );
        write(&old_temp, b"{");
        write(&new_temp, b"{");
        write("x.55.0.0123456789ABCDEF.tmp", b"{");
        let old = SystemTime::now() - Duration::from_secs(120);
        for name in [old_temp.as_str(), "x.55.0.0123456789ABCDEF.tmp"] {
            fs::File::options()
                .write(true)
                .open(dir.join(name))
                .unwrap()
                .set_modified(old)
                .unwrap();
        }
        // A folder with a state file's name is never entered or deleted.
        fs::create_dir_all(dir.join(hooked::chat_file(&"3".repeat(8).to_string()))).unwrap();
        let folder_named = dir.join("4b9e6b0e-3c1a-4f7e-9a51-2d64c1f0a7b3.json");
        fs::create_dir_all(&folder_named).unwrap();
        fs::write(
            folder_named.join("inside.json"),
            hooked::state_text(ended, "", "Cursor", State::Working),
        )
        .unwrap();

        fake.run("stop", &fixture("editor-stop-completed.json"));
        assert_eq!(fake.chat_file_names().len(), 11, "a stop does not prune");
        fake.run("beforeSubmitPrompt", &prompt());
        let left = fake.chat_file_names();
        let want: BTreeSet<String> = [
            hooked::chat_file(CHAT),
            hooked::chat_file(third_chat),
            "newer.json".to_owned(),
            "junk.json".to_owned(),
            "notes.txt".to_owned(),
            new_temp,
            "x.55.0.0123456789ABCDEF.tmp".to_owned(),
            "33333333.json".to_owned(),
            "4b9e6b0e-3c1a-4f7e-9a51-2d64c1f0a7b3.json".to_owned(),
        ]
        .into();
        assert_eq!(left, want);
        assert!(folder_named.join("inside.json").exists());
        assert_eq!(
            fake.lines(),
            ["cursor beforeSubmitPrompt: removed 3 files of ended chats"]
        );
    }

    #[test]
    fn unknown_agents_and_events_and_the_remove_form_do_nothing() {
        let fake = Fake::new("unknown");
        *fake.input.borrow_mut() = prompt();
        for (agent, event) in [
            ("Cursor", "beforeSubmitPrompt"),
            // An Antigravity event Wakeshot does not hook.
            ("antigravity", "SessionStart"),
            ("", ""),
            ("cursor", "BeforeSubmitPrompt"),
            ("cursor", "afterAgentResponse"),
            ("cursor", ""),
        ] {
            run(&fake, agent, event, &[], None);
        }
        assert_eq!(
            fake.drained.get(),
            6,
            "each still reads the pipe to its end"
        );
        assert_eq!(fake.files(), BTreeSet::new());
        assert!(fake.lines().is_empty());
        assert!(fake.removed.borrow().is_empty());
    }

    #[test]
    fn the_remove_form_hands_its_agent_word_to_agent_hooks_and_reads_no_input() {
        let fake = Fake::new("remove");
        for agent in ["cursor", "antigravity", "nobody"] {
            run(&fake, agent, "remove", &[], None);
        }
        // `--checked` values change nothing for it.
        run(&fake, "cursor", "remove", &["x".to_owned()], None);
        assert_eq!(
            *fake.removed.borrow(),
            ["cursor", "antigravity", "nobody", "cursor"],
            "agent_hooks knows its own agents"
        );
        assert_eq!(fake.drained.get(), 0, "the uninstall's copy has no input");
        assert_eq!(fake.files(), BTreeSet::new());
        // Elevated, its lines go nowhere: `agent_hooks` refuses above Medium itself.
        let mut fake = Fake::new("remove-high");
        fake.above = Ok(true);
        run(&fake, "cursor", "remove", &[], None);
        assert_eq!(*fake.removed.borrow(), ["cursor"]);
        assert!(fake.lines().is_empty());
    }

    #[test]
    fn a_failed_dll_limit_is_one_line_at_medium() {
        let fake = Fake::new("dll");
        *fake.input.borrow_mut() = prompt();
        let e = WinError {
            call: "SetDefaultDllDirectories",
            code: 87,
        };
        run(&fake, "cursor", "beforeSubmitPrompt", &[], Some(e));
        assert_eq!(
            fake.lines(),
            [
                "cursor beforeSubmitPrompt: DLL search not limited: SetDefaultDllDirectories failed with 87"
            ]
        );
        assert!(fake.chat_path().exists(), "the hook still works");
    }

    /// The privacy rule: the marker as the value of any field (as a string, a number or an object), or as a key, never
    /// reaches a file name, a file or a log line.
    #[test]
    fn the_marker_in_any_field_never_reaches_a_file_or_a_log_line() {
        let mut written = 0;
        for (fields, event) in [
            (EDITOR_PROMPT, "beforeSubmitPrompt"),
            (EDITOR_STOP, "stop"),
            (EDITOR_END, "sessionEnd"),
            (CLI_PROMPT, "beforeSubmitPrompt"),
        ] {
            let mut inputs: Vec<Vec<u8>> = fields
                .iter()
                .flat_map(|&(key, _)| {
                    [MARKER, MARKER_NUMBER, MARKER_OBJECT]
                        .map(|value| object(fields, &[(key, value)]))
                })
                .collect();
            inputs.push(object(fields, &[(MARKER, MARKER)]));
            let fake = Fake::new("marker");
            for input in inputs {
                let _ = fs::remove_dir_all(fake.folder());
                // A stop and an end need the file a prompt wrote to work on.
                if event != "beforeSubmitPrompt" {
                    fake.run("beforeSubmitPrompt", &prompt());
                }
                fake.run(event, &input);
                for name in fake.files() {
                    let text = fs::read_to_string(fake.temp.dir.root().join(&name)).unwrap();
                    for word in MARKER_WORDS {
                        assert!(
                            !name.contains(word) && !text.contains(word),
                            "{name}: {text}"
                        );
                    }
                }
                let chat_files = fake.chat_file_names();
                if event == "beforeSubmitPrompt" && chat_files.iter().any(|n| n != "problem.json") {
                    written += 1;
                }
            }
            for line in fake.lines() {
                for word in MARKER_WORDS {
                    assert!(!line.contains(word), "{line}");
                }
            }
        }
        assert!(
            written >= 40,
            "the check saw state files of marked inputs: {written}"
        );
    }

    #[test]
    fn the_field_lists_are_the_fixtures() {
        for (fields, name, event) in [
            (
                EDITOR_PROMPT,
                "editor-before-submit-prompt.json",
                "beforeSubmitPrompt",
            ),
            (EDITOR_STOP, "editor-stop-completed.json", "stop"),
            (EDITOR_END, "editor-session-end.json", "sessionEnd"),
            (
                CLI_PROMPT,
                "cli-before-submit-prompt.json",
                "beforeSubmitPrompt",
            ),
        ] {
            let from_list = hooked::parse_input(&cursor::AGENT, event, &object(fields, &[])[..]);
            let from_file = hooked::parse_input(&cursor::AGENT, event, &fixture(name)[..]);
            assert!(from_list.input.is_ok(), "{name}");
            assert_eq!(from_list, from_file, "{name}");
        }
    }

    const AGY_CHAT: &str = "ec33ebf9-0cba-4100-8142-c61503f6c587";
    /// The `agy.exe` that ran the hook's `cmd /c` (`SHELL` here).
    const AGY: ProcessRef = ProcessRef {
        pid: 650,
        start: FileTime(6_500),
    };

    /// `cli-pre-invocation.json` and `cli-stop-background.json` of `wakeshot-core/tests/fixtures/antigravity/`.
    const AGY_CALL: &[(&str, &str)] = &[
        (
            "conversationId",
            r#""ec33ebf9-0cba-4100-8142-c61503f6c587""#,
        ),
        ("workspacePaths", r#"["C:\\Users\\dev\\project"]"#),
        (
            "transcriptPath",
            r#""C:\\Users\\dev\\.gemini\\antigravity-cli\\brain\\ec33ebf9-0cba-4100-8142-c61503f6c587\\.system_generated\\logs\\transcript.jsonl""#,
        ),
        (
            "artifactDirectoryPath",
            r#""C:\\Users\\dev\\.gemini\\antigravity-cli\\brain\\ec33ebf9-0cba-4100-8142-c61503f6c587""#,
        ),
        ("modelName", r#""gemini-3.8-flash""#),
        ("lastUserInput", r#""Fix the failing unit test""#),
        ("invocationNum", "0"),
        ("initialNumSteps", "1"),
    ];
    const AGY_STOP: &[(&str, &str)] = &[
        (
            "conversationId",
            r#""ec33ebf9-0cba-4100-8142-c61503f6c587""#,
        ),
        ("workspacePaths", r#"["C:\\Users\\dev\\project"]"#),
        (
            "transcriptPath",
            r#""C:\\Users\\dev\\.gemini\\antigravity-cli\\brain\\ec33ebf9-0cba-4100-8142-c61503f6c587\\.system_generated\\logs\\transcript.jsonl""#,
        ),
        (
            "artifactDirectoryPath",
            r#""C:\\Users\\dev\\.gemini\\antigravity-cli\\brain\\ec33ebf9-0cba-4100-8142-c61503f6c587""#,
        ),
        ("modelName", r#""gemini-3.8-flash""#),
        (
            "lastUserInput",
            r#""Start the dev server and keep it running""#,
        ),
        ("executionNum", "2"),
        ("terminationReason", r#""NO_TOOL_CALL""#),
        ("error", r#""""#),
        ("fullyIdle", "false"),
    ];

    fn agy_fixture(name: &str) -> Vec<u8> {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("wakeshot-core")
            .join("tests")
            .join("fixtures")
            .join("antigravity")
            .join(name);
        fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
    }

    /// The hook, the `cmd /c` that `agy` runs it with, `agy.exe` at `path`, the user's terminal.
    fn agy_fake(tag: &str, path: &str) -> Fake {
        let mut fake = Fake::new(tag);
        fake.list = vec![
            entry(ME, SHELL.pid, "wakeshot.exe"),
            entry(SHELL, AGY.pid, "cmd.exe"),
            entry(AGY, 1, "agy.exe"),
        ];
        fake.paths = vec![(AGY.pid, PathBuf::from(path))];
        fake
    }

    fn cli_agy() -> String {
        local().join(r"agy\bin\agy.exe").display().to_string()
    }

    fn run_agy(fake: &Fake, event: &str, input: &[u8]) {
        *fake.input.borrow_mut() = input.to_vec();
        run(fake, "antigravity", event, &[], None);
    }

    fn agy_folder(fake: &Fake) -> PathBuf {
        hooked::agent_folder(fake.temp.dir.root(), &antigravity::AGENT)
    }

    fn agy_state(fake: &Fake, chat: &str) -> Option<hooked::StateFile> {
        let bytes = fs::read(agy_folder(fake).join(hooked::chat_file(chat))).ok()?;
        match hooked::read_state(&bytes) {
            hooked::FileRead::V1(file) => Some(file),
            other => panic!("{other:?}"),
        }
    }

    const AGY_LOG: &str = "cli-20261106_185448.log";

    /// `agy`'s log for `pid` in the fake's profile: lines 1 and 2 as 1.5.2 writes them, then a prompt line with the
    /// marker, which no file or log line of the hook may hold.
    fn agy_log_file(fake: &Fake, name: &str, pid: u32) -> PathBuf {
        let dir = antigravity::log_dir(fake.home.dir.root());
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        fs::write(
            &path,
            format!(
                "I1106 18:54:48.718882      51 server.go:2275] Starting language server process with pid {pid}\n\
                 I1106 18:54:48.727658      51 server.go:2333] Language server version: 1.5.2\n\
                 I1106 18:55:41.100000     176 input_loop.go:301] HandleUserInput called with text: {MARKER}\n"
            ),
        )
        .unwrap();
        path
    }

    /// A model call run from a new `cmd /c`, as `agy` runs each one, started at `start` (before the hook's own start).
    fn new_shell(fake: &mut Fake, start: u64) {
        fake.list[1].start = FileTime(start);
    }

    #[test]
    fn antigravity_a_model_call_names_agys_log_and_when_agy_started_the_hook() {
        let mut fake = agy_fake("agy-turn", &cli_agy());
        agy_log_file(&fake, "cli-20261106_185447.log", AGY.pid + 1);
        agy_log_file(&fake, AGY_LOG, AGY.pid);
        fs::write(
            antigravity::log_dir(fake.home.dir.root()).join("cli.log"),
            b"I1106 18:54:48.718882 51 server.go:2275] Starting language server process with pid 650\n",
        )
        .unwrap();
        run_agy(
            &fake,
            "PreInvocation",
            &agy_fixture("cli-pre-invocation.json"),
        );
        let file = agy_state(&fake, AGY_CHAT).expect("written");
        assert_eq!(
            (
                file.pid,
                file.proc_start,
                file.generation.as_str(),
                file.label.as_str(),
                file.state,
                file.log.as_deref(),
                file.turn_at,
            ),
            (
                AGY.pid,
                AGY.start,
                "",
                "Antigravity CLI",
                State::Working,
                Some(AGY_LOG),
                Some(SHELL.start)
            ),
            "the agy.exe above cmd, its own log, and cmd's start"
        );

        // Each later model call has a cmd of its own, so the file moves to its start.
        for (call, start) in [("1", 8_100), ("2", 8_200)] {
            new_shell(&mut fake, start);
            run_agy(
                &fake,
                "PreInvocation",
                &object(AGY_CALL, &[("invocationNum", call)]),
            );
            let file = agy_state(&fake, AGY_CHAT).unwrap();
            assert_eq!(file.turn_at, Some(FileTime(start)), "call {call}");
            assert_eq!(file.log.as_deref(), Some(AGY_LOG));
        }

        // `fullyIdle` false or missing: the chat ends all the same, since background commands are not held.
        run_agy(&fake, "Stop", &agy_fixture("cli-stop-background.json"));
        assert_eq!(agy_state(&fake, AGY_CHAT), None);
        run_agy(&fake, "PreInvocation", &object(AGY_CALL, &[]));
        let no_flag: Vec<(&str, &str)> = AGY_STOP
            .iter()
            .copied()
            .filter(|&(key, _)| key != "fullyIdle")
            .collect();
        run_agy(&fake, "Stop", &object(&no_flag, &[]));
        assert_eq!(agy_state(&fake, AGY_CHAT), None);
        run_agy(&fake, "Stop", &agy_fixture("cli-stop-idle.json"));
        assert_eq!(fake.files(), BTreeSet::new());
        assert!(fake.lines().is_empty(), "{:?}", fake.lines());
        assert!(fake.handed.borrow().is_empty());
        assert_eq!(fake.drained.get(), 7, "every run reads the pipe to its end");
    }

    #[test]
    fn antigravity_with_no_log_naming_agy_the_file_names_none() {
        let fake = agy_fake("agy-no-log", &cli_agy());
        // No log folder at all.
        run_agy(&fake, "PreInvocation", &object(AGY_CALL, &[]));
        let file = agy_state(&fake, AGY_CHAT).expect("written");
        assert_eq!(
            (file.log, file.turn_at, file.state),
            (None, None, State::Working)
        );
        // Logs of other pids, or a first line of another shape: none.
        agy_log_file(&fake, "cli-20261106_185447.log", AGY.pid + 1);
        let odd = antigravity::log_dir(fake.home.dir.root()).join(AGY_LOG);
        fs::write(
            &odd,
            format!("Starting language server process with pid {}\n", AGY.pid),
        )
        .unwrap();
        run_agy(&fake, "PreInvocation", &object(AGY_CALL, &[]));
        assert_eq!(agy_state(&fake, AGY_CHAT).unwrap().log, None);
        assert!(fake.lines().is_empty(), "{:?}", fake.lines());
    }

    #[test]
    fn find_log_takes_only_a_file_made_since_the_process_started_that_names_its_pid() {
        let fake = Fake::new("find-log");
        let dir = antigravity::log_dir(fake.home.dir.root());
        let path = agy_log_file(&fake, AGY_LOG, 4_242);
        let made = fs::metadata(&path).unwrap().creation_time();
        let at = |pid: u32, start: u64| {
            find_log(
                &dir,
                ProcessRef {
                    pid,
                    start: FileTime(start),
                },
            )
        };
        assert_eq!(at(4_242, made).as_deref(), Some(AGY_LOG));
        assert_eq!(at(4_242, made - 1).as_deref(), Some(AGY_LOG));
        assert_eq!(at(4_242, made + 1), None, "an older process with the pid");
        assert_eq!(at(4_243, made), None);
        // A folder of the name is no log; neither is any other name.
        fs::remove_file(&path).unwrap();
        fs::create_dir(&path).unwrap();
        let head = b"I1106 18:54:48.718882 51 server.go:2275] Starting language server process with pid 4242
";
        fs::write(dir.join("cli-20261106_185449.txt"), head).unwrap();
        fs::write(dir.join("cli.log"), head).unwrap();
        assert_eq!(at(4_242, 0), None);
        assert_eq!(
            find_log(
                &dir.join("missing"),
                ProcessRef {
                    pid: 1,
                    start: FileTime(0)
                }
            ),
            None
        );
    }

    #[test]
    fn antigravity_the_ides_hub_writes_nothing_and_logs_nothing() {
        let fake = agy_fake("agy-ide", r"C:\Users\dev\.gemini\bin\agy.exe");
        agy_log_file(&fake, AGY_LOG, AGY.pid);
        run_agy(
            &fake,
            "PreInvocation",
            &agy_fixture("ide-pre-invocation.json"),
        );
        assert_eq!(fake.files(), BTreeSet::new());
        run_agy(&fake, "Stop", &agy_fixture("ide-stop-canceled.json"));
        assert_eq!(fake.files(), BTreeSet::new());
        assert!(fake.lines().is_empty(), "{:?}", fake.lines());
        assert!(fake.handed.borrow().is_empty());

        // Its input on a command line still blocks, as for the CLI.
        let mut fake = agy_fake("agy-ide-blocked", r"C:\Users\dev\.gemini\bin\agy.exe");
        fake.command_line = Ok(true);
        run_agy(&fake, "PreInvocation", &object(AGY_CALL, &[]));
        assert_eq!(
            fake.files(),
            BTreeSet::from([r"agents\antigravity\blocked.json".to_owned()])
        );
        assert_eq!(*fake.removed.borrow(), ["antigravity"]);

        // No agy.exe above the hook: nothing written, one line.
        let mut fake = agy_fake("agy-none", &cli_agy());
        fake.list[1] = entry(SHELL, 1, "cmd.exe");
        run_agy(
            &fake,
            "PreInvocation",
            &agy_fixture("cli-pre-invocation.json"),
        );
        assert_eq!(fake.files(), BTreeSet::new());
        assert_eq!(
            fake.lines(),
            ["antigravity PreInvocation: no Antigravity process above the hook; nothing written"]
        );
    }

    #[test]
    fn antigravity_input_on_cmds_command_line_blocks_and_removes_its_plugin() {
        let mut fake = agy_fake("agy-blocked", &cli_agy());
        fake.command_line = Ok(true);
        run_agy(&fake, "PreInvocation", &object(AGY_CALL, &[]));
        assert_eq!(
            fake.files(),
            BTreeSet::from([r"agents\antigravity\blocked.json".to_owned()])
        );
        assert_eq!(*fake.removed.borrow(), ["antigravity"]);
        assert_eq!(
            fake.lines()[0],
            "antigravity PreInvocation: Antigravity put its input on a command line, where audit logs keep it; nothing kept, blocked.json written, the hook folder removed"
        );
    }

    #[test]
    fn antigravity_an_elevated_hook_hands_its_log_and_its_stop_to_the_medium_copy() {
        let mut fake = agy_fake("agy-high", &cli_agy());
        agy_log_file(&fake, AGY_LOG, AGY.pid);
        fake.above = Ok(true);
        run_agy(&fake, "PreInvocation", &object(AGY_CALL, &[]));
        run_agy(&fake, "Stop", &object(AGY_STOP, &[]));
        assert_eq!(fake.files(), BTreeSet::new());
        let deadline = (fake.now.get() + 2_000).to_string();
        let args = |event: &str, op: &str, log: &[&str]| {
            let mut args = vec![
                "hook",
                "antigravity",
                event,
                "--checked",
                AGY_CHAT,
                "",
                "650",
                "6500",
                "cli",
                op,
                &deadline,
            ];
            args.extend(log);
            args.into_iter().map(OsString::from).collect::<Vec<_>>()
        };
        assert_eq!(
            *fake.handed.borrow(),
            [
                args("PreInvocation", "working", &[AGY_LOG, "8000"]),
                args("Stop", "none", &[])
            ]
        );

        // The copy, at Medium, does what was handed.
        let fake = agy_fake("agy-copy", &cli_agy());
        let values = |args: Vec<OsString>| {
            args[4..]
                .iter()
                .map(|v| v.to_string_lossy().into_owned())
                .collect::<Vec<_>>()
        };
        let call = values(args("PreInvocation", "working", &[AGY_LOG, "8000"]));
        run(&fake, "antigravity", "PreInvocation", &call, None);
        assert_eq!(
            agy_state(&fake, AGY_CHAT).map(|f| (f.state, f.label, f.log, f.turn_at)),
            Some((
                State::Working,
                "Antigravity CLI".to_owned(),
                Some(AGY_LOG.to_owned()),
                Some(FileTime(8_000))
            ))
        );
        run(
            &fake,
            "antigravity",
            "Stop",
            &values(args("Stop", "none", &[])),
            None,
        );
        assert_eq!(agy_state(&fake, AGY_CHAT), None);
        // What a Stop could never do, a log on a delete, or the IDE's hub: nothing written.
        for (event, handed) in [
            ("Stop", args("Stop", "working", &[])),
            ("Stop", args("Stop", "none", &[AGY_LOG, "8000"])),
            (
                "PreInvocation",
                args("PreInvocation", "working", &[r"..\x.log", "8000"]),
            ),
        ] {
            run(&fake, "antigravity", event, &values(handed), None);
        }
        let mut ide = call.clone();
        ide[4] = "app".to_owned();
        run(&fake, "antigravity", "PreInvocation", &ide, None);
        assert_eq!(fake.files(), BTreeSet::new());
        assert_eq!(
            fake.lines(),
            [
                "antigravity Stop: the checked values do not pass; nothing written",
                "antigravity Stop: the checked values do not pass; nothing written",
                "antigravity PreInvocation: the checked values do not pass; nothing written",
            ]
        );
    }

    /// The privacy rule for Antigravity: the marker as the value of any field (the prompt in `lastUserInput`, the
    /// paths, the model) or as a key, or in a prompt line of `agy`'s log, never reaches a file name, a file or a log
    /// line.
    #[test]
    fn antigravity_the_marker_in_any_field_never_reaches_a_file_or_a_log_line() {
        let mut written = 0;
        for (fields, event) in [(AGY_CALL, "PreInvocation"), (AGY_STOP, "Stop")] {
            let mut inputs: Vec<Vec<u8>> = fields
                .iter()
                .flat_map(|&(key, _)| {
                    [MARKER, MARKER_NUMBER, MARKER_OBJECT]
                        .map(|value| object(fields, &[(key, value)]))
                })
                .collect();
            inputs.push(object(fields, &[(MARKER, MARKER)]));
            let fake = agy_fake("agy-marker", &cli_agy());
            agy_log_file(&fake, AGY_LOG, AGY.pid);
            for input in inputs {
                let _ = fs::remove_dir_all(agy_folder(&fake));
                if event == "Stop" {
                    run_agy(&fake, "PreInvocation", &object(AGY_CALL, &[]));
                }
                run_agy(&fake, event, &input);
                for name in fake.files() {
                    let text = fs::read_to_string(fake.temp.dir.root().join(&name)).unwrap();
                    for word in MARKER_WORDS {
                        assert!(
                            !name.contains(word) && !text.contains(word),
                            "{name}: {text}"
                        );
                    }
                }
                let state = agy_state(&fake, AGY_CHAT);
                let wrote = match event {
                    "Stop" => state.is_none(),
                    _ => state.is_some_and(|f| f.log.as_deref() == Some(AGY_LOG)),
                };
                written += usize::from(wrote);
            }
            for line in fake.lines() {
                for word in MARKER_WORDS {
                    assert!(!line.contains(word), "{line}");
                }
            }
        }
        assert!(
            written >= 40,
            "the check saw state files of marked inputs: {written}"
        );
    }

    #[test]
    fn antigravity_the_field_lists_are_the_fixtures() {
        for (fields, name, event) in [
            (AGY_CALL, "cli-pre-invocation.json", "PreInvocation"),
            (AGY_STOP, "cli-stop-background.json", "Stop"),
        ] {
            let agent = &antigravity::AGENT;
            let from_list = hooked::parse_input(agent, event, &object(fields, &[])[..]);
            let from_file = hooked::parse_input(agent, event, &agy_fixture(name)[..]);
            assert!(from_list.input.is_ok(), "{name}");
            assert_eq!(from_list, from_file, "{name}");
        }
    }
}
