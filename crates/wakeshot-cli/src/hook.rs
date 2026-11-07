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

/// Every agent the command knows; any other word does nothing, so a hook left by any version never fails. Both are out
/// of v1 by "No guessing" (`docs/product.md`): a deleted Cursor chat sends no `stop`, and Antigravity's Esc fires no
/// hook. Kept, so the `remove` form works and a folder an earlier build left still runs.
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
        // With the hooks gone no `stop` comes for a chat mid-turn. Also the elevated hook's own hand-off when it
        // found its input on a command line but no agent process to name.
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
    // Before the agent gate: an agent version that moves its input to a command line may also run its hooks from a
    // process the walk does not know, and this check is the only backstop then.
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
    // The IDE is not in v1: its hub loads the same plugin, and its turns write nothing, quietly. A command line with
    // the input still blocks above and below, for the IDE too.
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
/// file would hold while the process it names lives: each chat's file goes now, under the mutex.
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
mod tests;
