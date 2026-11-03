//! A harness that plays the tray's power side by hand, so the agent and Haik can run the manual checks of the power
//! track before integration (architecture section 9; `docs/tasks/p3-harness.md`). It drives the real executor,
//! journal, power requests and notifications on a plan copy; the guard and restore mode it starts are the real tray
//! exe's.
//!
//! Owner: power track (`p3-harness`).
//!
//! Build: `cargo build --locked -p wakeshot-windows --features test-support --example power_check`, and
//! `cargo build --locked -p wakeshot-tray` (same target and profile) for the guard. Then `power_check --help`.

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::ffi::c_void;
use std::fs::OpenOptions;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::mpsc::{self, Sender};
use std::time::Instant;

use wakeshot_core::command::{ApplyError, Command, Failure, Outcome, RestoreWhy};
use wakeshot_core::event::Event;
use wakeshot_core::files::JournalFile;
use wakeshot_core::ownership::io::{JournalRead, PlanIo, SafetyNet};
use wakeshot_core::process::{Alive, FileTime, Lookup, ProcessRef};
use wakeshot_core::settings::{Guid, JournalEntry, JournalOwner, PlanValues, SettingKey};
use wakeshot_core::time::Now;
use wakeshot_windows::WinError;
use wakeshot_windows::clock;
use wakeshot_windows::log::{Level, Log, LogWriter, Proc};
use wakeshot_windows::power::executor::PowerExecutor;
use wakeshot_windows::power::journal::{self, FileNet, RUN_ONCE_NAME};
use wakeshot_windows::power::plan::{CopyAsActive, PlanSetting, TestPlan, Win32Power};
use wakeshot_windows::power::requests::{PowerRequests, RequestApi};
use wakeshot_windows::power::system::Subscriptions;
use wakeshot_windows::process::{self, Liveness, SystemLiveness};
use wakeshot_windows::recovery::TrayGuard;
use wakeshot_windows::session;
use wakeshot_windows::sink::EventSink;
use wakeshot_windows::store::{self, AutoStart, DataDir, DataFile, NamedMutex, Names, StartKey};
use windows::Win32::Foundation::{
    ERROR_MORE_DATA, ERROR_NO_DATA, ERROR_SUCCESS, HANDLE, LPARAM, LRESULT, WIN32_ERROR, WPARAM,
};
use windows::Win32::Security::{
    GetSidSubAuthority, GetSidSubAuthorityCount, GetTokenInformation, TOKEN_INFORMATION_CLASS,
    TOKEN_MANDATORY_LABEL, TOKEN_QUERY, TokenElevationType, TokenIntegrityLevel,
};
use windows::Win32::System::Power::PowerReadFriendlyName;
use windows::Win32::System::SystemInformation::GetLocalTime;
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, MSG, MsgWaitForMultipleObjects, PM_REMOVE, PeekMessageW, QS_ALLINPUT,
    TranslateMessage, WM_ENDSESSION, WM_QUERYENDSESSION, WM_WTSSESSION_CHANGE,
};
use windows::core::{GUID, Owned};

use cli::{Args, Cmd};
use tidy::{JournalSeen, Step};
use window::Window;

const AREA: &str = "harness";

/// The tray's tick (the spike's hold ran 5 s ticks too, so the gap lines compare with its logs).
const TICK_MS: i64 = 5_000;

/// The tray's ownership check period while anything is owned (architecture section 5, "Ownership").
const PLAN_READ_MS: i64 = 10_000;

/// How often a command with nothing to do on a timer looks at the clock, for `--for`.
const IDLE_MS: i64 = 1_000;

/// Do nothing for the lid and Never for sleep are both 0: the values the core applies (`core::settings::TARGET`).
const TARGET: u32 = 0;

const EXIT_FAILED: u8 = 1;
const EXIT_BAD_ARGS: u8 = 2;

const RUN_ONCE_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\RunOnce";

/// `TestPlan`'s name prefix; the pid after it names the process that made the copy.
const COPY_PREFIX: &str = "Wakeshot test ";

fn main() -> ExitCode {
    let raw: Result<Vec<String>, _> = std::env::args_os()
        .skip(1)
        .map(std::ffi::OsString::into_string)
        .collect();
    let Ok(raw) = raw else {
        print_err("an argument is not valid Unicode");
        return ExitCode::from(EXIT_BAD_ARGS);
    };
    let args = match cli::parse(&raw) {
        Ok(args) => args,
        Err(why) => {
            print_err(&format!("{why}\n\n{}", cli::USAGE));
            return ExitCode::from(EXIT_BAD_ARGS);
        }
    };
    if args.cmd == Cmd::Help {
        let _ = writeln!(std::io::stdout(), "{}", cli::USAGE);
        return ExitCode::SUCCESS;
    }
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(why) => {
            print_err(&why);
            ExitCode::from(EXIT_FAILED)
        }
    }
}

/// What every command shares.
struct Ctx {
    home: Home,
    dir: DataDir,
    out: Out,
    log: Arc<dyn Log>,
    plan_only: bool,
    me: ProcessRef,
    /// The user's plan: the one recorded at the first start, or the active one when none is recorded yet.
    original: Option<Guid>,
    /// Wall clock ms at which `--for` ends the run.
    ends_at: Option<i64>,
}

impl Ctx {
    fn print_plan(&self, cmd: &Cmd, guard: Option<&Path>) {
        self.out.say("what it does:");
        self.out
            .lines(&text::plan_lines(cmd, self.dir.root(), guard, self.me.pid));
        self.print_undo(cmd, None);
    }

    fn print_undo(&self, cmd: &Cmd, copy: Option<Guid>) {
        let copy = copy.map(|g| g.to_string());
        let undo = text::undo_for(cmd, self.original, copy.as_deref(), self.dir.root());
        self.out.lines(&text::undo_lines(&undo));
    }

    fn run_record(&self, command: &str, run_once: Option<String>, copy: Option<Guid>) -> Run {
        Run {
            pid: self.me.pid,
            start: self.me.start,
            command: command.to_owned(),
            dir: self.dir.root().to_path_buf(),
            run_once,
            copy,
        }
    }

    fn save(&self, run: &Run) -> Result<PathBuf, String> {
        self.home.save(run)
    }

    fn owner(&self) -> Result<JournalOwner, String> {
        Ok(JournalOwner {
            pid: self.me.pid,
            proc_start: self.me.start,
            session: process::current_session().map_err(win_text)?,
        })
    }

    /// The plan a copy run goes back to. It must be the active one, or another run's copy may be active.
    fn plan_to_copy(&self, power: &mut Win32Power) -> Result<Guid, String> {
        let active = power.active_plan().map_err(fail_text)?;
        match self.original {
            Some(original) if original == active => Ok(original),
            Some(original) => Err(format!(
                "the active plan {active} is not the plan recorded at the first start ({original}): another run's \
                 copy may be active; run `power_check cleanup` first"
            )),
            None => Err(
                "no plan is recorded and the active one is a test copy: run `power_check cleanup` first".to_owned(),
            ),
        }
    }
}

fn run(args: &Args) -> Result<(), String> {
    let home = Home::at(match &args.home {
        Some(root) => absolute(root)?,
        None => Home::default_root()?,
    });
    let dir = DataDir::at(match &args.dir {
        Some(root) => absolute(root)?,
        None => default_dir(&args.cmd, &home)?,
    });
    let log: Arc<dyn Log> = match args.plan_only {
        true => Arc::new(NoLog),
        false => Arc::new(LogWriter::open(&dir.logs(), Proc::Tray).map_err(|e| {
            format!(
                "cannot open the log in {}: {}",
                dir.logs().display(),
                win_text(e)
            )
        })?),
    };
    let out = Out { log: log.clone() };
    ctrl::install().map_err(win_text)?;
    let me = process::current().map_err(win_text)?;
    let plan_only = match args.plan_only {
        true => "; plan only: nothing is changed",
        false => "",
    };
    out.say(&format!(
        "power_check {} started: pid {}, start {}, {}{plan_only}",
        args.cmd.name(),
        me.pid,
        me.start,
        integrity_text()
    ));
    out.say(&format!(
        "data folder {}; harness folder {}",
        dir.root().display(),
        home.root().display()
    ));
    let mut power = Win32Power::new(log.clone());
    let original = original_plan(&home, &mut power, &out, args)?;
    let start = values::read(&mut power, original);
    out.say("values at start:");
    out.lines(&values::lines(&start));
    let ends_at = args
        .for_s
        .map(|s| clock::now().wall_ms + i64::try_from(s).unwrap_or(i64::MAX / 2_000) * 1000);
    let ctx = Ctx {
        home,
        dir,
        out: out.clone(),
        log,
        plan_only: args.plan_only,
        me,
        original,
        ends_at,
    };
    let result = match &args.cmd {
        Cmd::Hold {
            request,
            active_copy,
        } => hold(&ctx, &mut power, *request, *active_copy),
        Cmd::FakeTray => tray(&ctx, args.guard.as_deref(), false),
        Cmd::EndSession => tray(&ctx, args.guard.as_deref(), true),
        Cmd::SessionWatch => session_watch(&ctx),
        Cmd::Cycles { n, pause_ms } => cycles(&ctx, &mut power, *n, *pause_ms),
        Cmd::MediumProbe => medium_probe(&ctx),
        Cmd::Cleanup => cleanup(&ctx, &mut power),
        Cmd::Help => Ok(()),
    };
    if let Err(why) = &result {
        out.warn(why);
    }
    let end = values::read(&mut power, ctx.original);
    out.say("values at exit:");
    out.lines(&values::lines(&end));
    let changed = values::differences(&start, &end);
    if changed.is_empty() {
        out.say("values at exit: the same as at start");
    }
    for line in changed {
        out.warn(&format!("changed since the start: {line}"));
    }
    out.say(&format!("power_check {} ended", args.cmd.name()));
    ctrl::finished();
    result
}

fn default_dir(cmd: &Cmd, home: &Home) -> Result<PathBuf, String> {
    match cmd {
        // Check 5 runs restore mode at sign-in, which reads only the real folder.
        Cmd::FakeTray => Ok(DataDir::real().map_err(win_text)?.root().to_path_buf()),
        _ => Ok(home.data_dir(cmd.name())),
    }
}

/// The recorded plan, recorded now at the first start; never a test copy, which would send a later cleanup to it.
fn original_plan(
    home: &Home,
    power: &mut Win32Power,
    out: &Out,
    args: &Args,
) -> Result<Option<Guid>, String> {
    if let Some(plan) = home.original() {
        out.say(&format!("plan recorded at the first start: {plan}"));
        return Ok(Some(plan));
    }
    let active = power.active_plan().map_err(fail_text)?;
    if test_copy_pid(&name_text(active)).is_some() {
        out.warn(&format!(
            "the active plan {active} is a Wakeshot test copy and no plan was recorded before it: run `power_check \
             cleanup`, or make your plan active by hand (`powercfg /list`, then `powercfg /setactive <guid>`)"
        ));
        return Ok(None);
    }
    if args.plan_only || args.cmd == Cmd::Cleanup {
        return Ok(Some(active));
    }
    if home.keep_original(active)? {
        out.say(&format!(
            "recorded {active} as the plan active at the first start"
        ));
    }
    Ok(Some(home.original().unwrap_or(active)))
}

// ---------------------------------------------------------------------------------------------------------------------
// hold

fn hold(ctx: &Ctx, power: &mut Win32Power, request: bool, active_copy: bool) -> Result<(), String> {
    let cmd = Cmd::Hold {
        request,
        active_copy,
    };
    ctx.print_plan(&cmd, None);
    if ctx.plan_only {
        return Ok(());
    }
    let out = &ctx.out;
    let mut record = None;
    let copy = match active_copy {
        false => None,
        true => {
            let original = ctx.plan_to_copy(power)?;
            let path = ctx.save(&ctx.run_record(cmd.name(), None, None))?;
            let writes = [(SettingKey::LidDc, TARGET), (SettingKey::SleepDc, TARGET)];
            let copy = match ActiveCopy::make(original, &writes, out, &ctx.log) {
                Ok(copy) => copy,
                Err(why) => {
                    forget(&path, out);
                    return Err(why);
                }
            };
            record = Some(path);
            ctx.save(&ctx.run_record(cmd.name(), None, Some(copy.guid())))?;
            ctx.print_undo(&cmd, Some(copy.guid()));
            Some(copy)
        }
    };
    let mut requests = request.then(PowerRequests::new);
    if let Some(r) = requests.as_mut() {
        match r.set(true, false) {
            Ok(()) => out.say("SYSTEM and EXECUTION power requests set"),
            Err(e) => out.warn(&format!("setting the requests failed: {}", win_text(e))),
        }
    }
    let (tx, rx) = mpsc::channel();
    let subs = Subscriptions::start(Arc::new(ChannelSink {
        tx,
        waker: ctrl::waker(),
    }));
    for f in subs.failures() {
        out.warn(&format!(
            "a notification did not register: {}",
            win_text(*f)
        ));
    }
    let mut ticker = ticks::Ticker::new(clock::now(), local_stamp(), TICK_MS);
    let mut lost = 0;
    let why = run_loop(
        ctx.ends_at,
        TICK_MS,
        || {
            while let Ok(event) = rx.try_recv() {
                out.say(&describe(&event));
                if event == Event::Resumed
                    && let Some(r) = requests.as_mut()
                {
                    match r.retake() {
                        Ok(()) => out.say("requests retaken after the resume"),
                        Err(e) => out.warn(&format!("retake failed: {}", win_text(e))),
                    }
                }
            }
        },
        |now| {
            let (line, gap) = ticker.step(now, local_stamp(), clock::input_idle_ms());
            out.say(&line);
            if let Some(gap) = gap {
                out.warn(&gap);
            }
            if subs.lost_posts() != lost {
                lost = subs.lost_posts();
                out.warn(&format!("{lost} notifications lost so far"));
            }
        },
    );
    out.say(&format!("stopping: {why}"));
    drop(subs);
    if let Some(mut r) = requests {
        match r.set(false, false) {
            Ok(()) => out.say("requests cleared"),
            Err(e) => out.warn(&format!(
                "clearing the requests failed: {}; they end with this process",
                win_text(e)
            )),
        }
    }
    out.say(&format!(
        "{} ticks, {} gaps",
        ticker.count(),
        ticker.gap_count()
    ));
    let undone = copy.is_none_or(ActiveCopy::end);
    if undone && let Some(path) = record {
        forget(&path, out);
    }
    Ok(())
}

/// A plan copy made the active plan, made inactive and deleted again on every way out.
struct ActiveCopy {
    copy: Option<TestPlan>,
    original: Guid,
    out: Out,
    log: Arc<dyn Log>,
    ended: bool,
}

impl ActiveCopy {
    fn make(
        original: Guid,
        writes: &[(SettingKey, u32)],
        out: &Out,
        log: &Arc<dyn Log>,
    ) -> Result<ActiveCopy, String> {
        let copy = TestPlan::new().map_err(|e| format!("making a plan copy: {}", win_text(e)))?;
        let guid = copy.guid();
        out.say(&format!("made the copy {guid} \"{}\"", name_text(guid)));
        let mut power = Win32Power::new(log.clone());
        // A failure here drops `copy`, which is not active yet, so the drop deletes it.
        for &(key, value) in writes {
            power
                .write(guid, key, value)
                .map_err(|f| format!("writing {} on the copy: {}", key_name(key), fail_text(f)))?;
        }
        power
            .set_active(guid)
            .map_err(|f| format!("making the copy active: {}", fail_text(f)))?;
        out.say(&format!("the copy {guid} is the active plan now"));
        Ok(ActiveCopy {
            copy: Some(copy),
            original,
            out: out.clone(),
            log: log.clone(),
            ended: false,
        })
    }

    fn guid(&self) -> Guid {
        self.copy.as_ref().map_or(self.original, TestPlan::guid)
    }

    /// `true`: the original plan is active and the copy is gone.
    fn end(mut self) -> bool {
        self.undo()
    }

    fn undo(&mut self) -> bool {
        self.ended = true;
        let mut power = Win32Power::new(self.log.clone());
        let original = self.original;
        let back = power.set_active(original);
        match back {
            Ok(()) => self
                .out
                .say(&format!("made {original} the active plan again")),
            Err(f) => self.out.warn(&format!(
                "making {original} active again failed: {}; run `powercfg /setactive {original}`",
                fail_text(f)
            )),
        }
        let gone = drop_copy(&mut self.copy, &self.out, &self.log);
        back.is_ok() && gone
    }
}

impl Drop for ActiveCopy {
    fn drop(&mut self) {
        if !self.ended {
            self.undo();
        }
    }
}

/// Deletes a copy that is not active; `true` when it is gone from the plan list.
fn drop_copy(copy: &mut Option<TestPlan>, out: &Out, log: &Arc<dyn Log>) -> bool {
    let Some(plan) = copy.take() else {
        return true;
    };
    let guid = plan.guid();
    let started = Instant::now();
    drop(plan);
    let ms = ms_since(started);
    match Win32Power::new(log.clone()).plans() {
        Ok(list) if !list.contains(&guid) => {
            out.say(&format!("deleted the copy {guid} ({ms:.1} ms)"));
            true
        }
        Ok(_) => {
            out.warn(&format!(
                "the copy {guid} is still there: run `powercfg /delete {guid}` once it is not active"
            ));
            false
        }
        Err(f) => {
            out.warn(&format!(
                "the plan list does not read ({}), so whether the copy {guid} is gone is unknown",
                fail_text(f)
            ));
            false
        }
    }
}

/// Posts each notification to the main loop and wakes it.
struct ChannelSink {
    tx: Sender<Event>,
    waker: ctrl::Waker,
}

impl EventSink for ChannelSink {
    fn post(&self, event: Event) -> Result<(), WinError> {
        self.tx.send(event).map_err(|_| WinError {
            call: "Sender::send",
            code: ERROR_NO_DATA.0,
        })?;
        self.waker.wake();
        Ok(())
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// fake-tray and end-session

type TrayExecutor = PowerExecutor<Timed<CopyAsActive>, Timed<FileNet<TrayGuard>>>;

struct TrayRig {
    exec: TrayExecutor,
    copy: Option<TestPlan>,
    restored: bool,
    last_read: String,
    guard_pid: Option<u32>,
    record: Option<PathBuf>,
    out: Out,
    log: Arc<dyn Log>,
}

impl TrayRig {
    fn note_guard(&mut self) {
        let pid = self.exec.net().inner().guard().guard_pid();
        if pid != self.guard_pid {
            self.guard_pid = pid;
            match pid {
                Some(pid) => self.out.say(&format!("guard pid {pid}")),
                None => self.out.say("no guard"),
            }
        }
    }

    /// The tray's first plan read, then the apply the core sends at a hold start.
    fn apply(&mut self) -> Result<(), String> {
        let events = self.exec.run(&Command::ReadPlan);
        for event in &events {
            self.out.say(&format!("plan read: {}", describe(event)));
        }
        let values = match plan_of(&events) {
            Some(Ok(values)) => values,
            Some(Err(f)) => return Err(format!("the plan read failed: {}", fail_text(f))),
            None => return Err("the plan read gave no Plan event".to_owned()),
        };
        let copy = self.exec.plans().inner().copy();
        let writes = writes_for(&values);
        self.out.say(&format!(
            "apply on the copy {copy}: {}",
            writes_text(&writes)
        ));
        let events = self.exec.run(&Command::Apply { plan: copy, writes });
        for event in &events {
            self.out.say(&describe(event));
        }
        let plans = self.exec.plans().inner();
        self.out.say(&format!(
            "in force on the copy (recorded only: the copy is never active): lid AC {}, lid DC {}, sleep DC {}",
            plans.in_force(SettingKey::LidAc),
            plans.in_force(SettingKey::LidDc),
            plans.in_force(SettingKey::SleepDc)
        ));
        self.note_guard();
        match AutoStart::real().get(StartKey::RunOnce, RUN_ONCE_NAME) {
            Ok(Some(value)) => self.out.say(&format!("RunOnce {RUN_ONCE_NAME} = {value}")),
            Ok(None) => self.out.say(&format!("RunOnce {RUN_ONCE_NAME} is not set")),
            Err(e) => self.out.warn(&format!("RunOnce read: {}", win_text(e))),
        }
        match applied_of(&events) {
            Some(Ok(())) => Ok(()),
            Some(Err(e)) => Err(format!("the apply failed: {e:?}")),
            None => Err("the apply gave no Applied outcome".to_owned()),
        }
    }

    /// The ownership check, as the tray runs it every 10 s; a guard that ended is replaced before it.
    fn read(&mut self) {
        if self.restored {
            return;
        }
        let events = self.exec.run(&Command::ReadPlan);
        let text = events.iter().map(describe).collect::<Vec<_>>().join("; ");
        if text != self.last_read {
            self.out.say(&format!("plan read: {text}"));
            self.last_read = text;
        }
        self.note_guard();
    }

    fn restore(&mut self, why: RestoreWhy) -> bool {
        let started = Instant::now();
        let events = self.exec.run(&Command::Restore { plan: None, why });
        let ms = ms_since(started);
        for event in &events {
            self.out.say(&format!("{} ({ms:.1} ms)", describe(event)));
        }
        self.restored = matches!(restored_of(&events), Some((true, 0)));
        self.restored
    }

    fn finish(&mut self) {
        let gone = drop_copy(&mut self.copy, &self.out, &self.log);
        if self.restored
            && gone
            && let Some(path) = self.record.take()
        {
            forget(&path, &self.out);
        }
    }
}

fn tray(ctx: &Ctx, guard: Option<&Path>, with_window: bool) -> Result<(), String> {
    let cmd = match with_window {
        true => Cmd::EndSession,
        false => Cmd::FakeTray,
    };
    let out = &ctx.out;
    let exe = guard_exe(guard);
    ctx.print_plan(&cmd, exe.as_deref().ok());
    note_data_folder(ctx);
    if ctx.plan_only {
        if let Err(why) = &exe {
            out.warn(why);
        }
        return Ok(());
    }
    let exe = exe?;
    let auto = AutoStart::real();
    let journal_file = ctx.dir.file(DataFile::Journal);
    if journal_file.exists() {
        return Err(format!(
            "a journal is left in {}: run `power_check cleanup` first",
            ctx.dir.root().display()
        ));
    }
    match auto.get(StartKey::RunOnce, RUN_ONCE_NAME) {
        Ok(None) => {}
        Ok(Some(value)) => {
            return Err(format!(
                "the RunOnce value {RUN_ONCE_NAME} is set ({value}): another run holds it or one left it; run \
                 `power_check cleanup` first"
            ));
        }
        Err(e) => return Err(format!("RunOnce read: {}", win_text(e))),
    }
    // `FileNet::tray` writes this command; the record keeps it so a cleanup removes only a value this run set.
    let run_once = exe
        .to_str()
        .map(|path| format!("\"{path}\" --restore"))
        .ok_or("the tray exe path is not valid Unicode")?;
    // A guard inherits this environment, and a debug guard reads its data folder from it (`p3-recovery` Result).
    // SAFETY: always sound on Windows, also with other threads (std docs; research, "Recovery checks").
    unsafe { std::env::set_var("WAKESHOT_DATA_DIR", ctx.dir.root()) };
    let record = ctx.save(&ctx.run_record(cmd.name(), Some(run_once.clone()), None))?;
    let copy = match TestPlan::new() {
        Ok(copy) => copy,
        Err(e) => {
            forget(&record, out);
            return Err(format!("making a plan copy: {}", win_text(e)));
        }
    };
    let guid = copy.guid();
    out.say(&format!(
        "made the copy {guid} \"{}\"; it is never the active plan",
        name_text(guid)
    ));
    ctx.save(&ctx.run_record(cmd.name(), Some(run_once), Some(guid)))?;
    ctx.print_undo(&cmd, Some(guid));
    // On a failure `copy` drops here, never active, so the drop deletes it.
    let exec = match tray_executor(ctx, &copy, &exe, auto) {
        Ok(exec) => exec,
        Err(why) => {
            forget(&record, out);
            return Err(why);
        }
    };
    let rig = Rc::new(RefCell::new(TrayRig {
        exec,
        copy: Some(copy),
        restored: false,
        last_read: String::new(),
        guard_pid: None,
        record: Some(record),
        out: out.clone(),
        log: ctx.log.clone(),
    }));
    let applied = rig.borrow_mut().apply();
    let shown = match with_window && applied.is_ok() {
        true => Some(Window::create(end_session_handler(rig.clone())).map_err(win_text)),
        false => None,
    };
    let why = match (&applied, &shown) {
        (Err(why), _) | (_, Some(Err(why))) => why.clone(),
        _ => {
            if with_window {
                out.say("hidden top-level window up: sign out to run check 6");
            }
            out.say(&format!(
                "holding: Ctrl+C ends it cleanly; for checks 4 and 5 end it in Task Manager or with `taskkill /PID \
                 {} /F`",
                ctx.me.pid
            ));
            run_loop(
                ctx.ends_at,
                PLAN_READ_MS,
                || {},
                |_| rig.borrow_mut().read(),
            )
        }
    };
    out.say(&format!("stopping: {why}"));
    drop(shown);
    let mut rig = rig.borrow_mut();
    if !rig.restored {
        rig.restore(RestoreWhy::Quit);
    }
    rig.finish();
    if !rig.restored {
        return Err(
            "the restore at exit did not finish: see above; `power_check cleanup` retries it"
                .to_owned(),
        );
    }
    applied
}

fn tray_executor(
    ctx: &Ctx,
    copy: &TestPlan,
    exe: &Path,
    auto: AutoStart,
) -> Result<TrayExecutor, String> {
    let plans = CopyAsActive::over(copy, Win32Power::new(ctx.log.clone())).map_err(fail_text)?;
    let net = FileNet::tray(
        ctx.dir.clone(),
        auto,
        exe,
        TrayGuard::new(exe.to_path_buf(), ctx.me, ctx.log.clone()),
        ctx.log.clone(),
    );
    Ok(PowerExecutor::new(
        Timed::new(
            plans,
            ctx.log.clone(),
            "set active (CopyAsActive: recorded only)",
        ),
        Timed::new(net, ctx.log.clone(), "set active"),
        ctx.owner()?,
        Names::real(),
        ctx.dir.clone(),
        Arc::new(SystemLiveness),
        ctx.log.clone(),
    ))
}

/// Check 6: the restore runs inside the end-session messages, before Windows ends the process.
fn end_session_handler(rig: Rc<RefCell<TrayRig>>) -> window::Handler {
    Box::new(move |msg: u32, wparam: WPARAM, lparam: LPARAM| {
        let Ok(mut rig) = rig.try_borrow_mut() else {
            return None;
        };
        match msg {
            WM_QUERYENDSESSION => {
                rig.out.say(&format!(
                    "WM_QUERYENDSESSION ({}): restoring now",
                    end_flags(lparam)
                ));
                session_restore(&mut rig);
                // TRUE: the session may end.
                Some(LRESULT(1))
            }
            WM_ENDSESSION => {
                let ending = wparam.0 != 0;
                rig.out.say(&format!(
                    "WM_ENDSESSION, ending {ending} ({})",
                    end_flags(lparam)
                ));
                match ending {
                    true => {
                        session_restore(&mut rig);
                        rig.finish();
                        rig.out
                            .say("the session ends: Windows ends this process next");
                    }
                    false => rig
                        .out
                        .say("the end was cancelled: the values stay restored, no new apply"),
                }
                Some(LRESULT(0))
            }
            _ => None,
        }
    })
}

fn session_restore(rig: &mut TrayRig) {
    if rig.restored {
        rig.out.say("already restored");
        return;
    }
    let started = Instant::now();
    let done = rig.restore(RestoreWhy::SessionEnd);
    rig.out.say(&format!(
        "restore at the session end: {}, {:.1} ms in all (each step's time is in the log)",
        if done { "done" } else { "NOT done" },
        ms_since(started)
    ));
}

/// What a guard or restore mode reads, when the data folder is not the real one.
fn note_data_folder(ctx: &Ctx) {
    let real = std::env::var_os("LOCALAPPDATA").map(|v| PathBuf::from(v).join("Wakeshot"));
    if real.as_deref() == Some(ctx.dir.root()) {
        return;
    }
    ctx.out.say(
        "note: this run sets WAKESHOT_DATA_DIR for its guard, which only a debug tray exe reads; a release guard and \
         restore mode at the next sign-in read %LocalAppData%\\Wakeshot",
    );
}

fn guard_exe(given: Option<&Path>) -> Result<PathBuf, String> {
    let exe = match given {
        Some(path) => path.to_path_buf(),
        // An example lands in `<profile>\examples`, the tray exe in `<profile>`.
        None => std::env::current_exe()
            .map_err(|e| e.to_string())?
            .parent()
            .and_then(Path::parent)
            .map(|dir| dir.join("wakeshot-tray.exe"))
            .ok_or("this exe has no folder above its own")?,
    };
    // `TrayGuard` passes it as the application name, which must be a full path.
    let exe = absolute(&exe)?;
    match exe.is_file() {
        true => Ok(exe),
        false => Err(format!(
            "no tray exe at {}: build it with `cargo build --locked -p wakeshot-tray` (the same target and profile \
             as this exe), or pass --guard <exe>",
            exe.display()
        )),
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// session-watch

fn session_watch(ctx: &Ctx) -> Result<(), String> {
    ctx.print_plan(&Cmd::SessionWatch, None);
    let out = &ctx.out;
    out.say(&format!(
        "on the console: {}; locked: {}",
        console_text(),
        locked_text()
    ));
    if ctx.plan_only {
        return Ok(());
    }
    let seen = out.clone();
    let shown = Window::create(Box::new(move |msg: u32, wparam: WPARAM, lparam: LPARAM| {
        if matches!(
            msg,
            WM_WTSSESSION_CHANGE | WM_QUERYENDSESSION | WM_ENDSESSION
        ) {
            seen.say(&format!(
                "{}: {:?}; on the console {}, locked {}",
                message_text(msg, wparam, lparam),
                session::decode(msg, wparam, lparam),
                console_text(),
                locked_text()
            ));
        }
        None
    }))
    .map_err(win_text)?;
    let notices = session::register(shown.hwnd).map_err(win_text)?;
    out.say("registered for session notices on a hidden top-level window: lock, unlock or switch user to see them");
    let why = run_loop(ctx.ends_at, IDLE_MS, || {}, |_| {});
    out.say(&format!("stopping: {why}"));
    // The docs ask for the unregister before the window goes.
    if let Err(e) = notices.unregister() {
        out.warn(&format!("unregister: {}", win_text(e)));
    }
    drop(shown);
    Ok(())
}

fn console_text() -> String {
    match session::on_console() {
        Ok(on) => on.to_string(),
        Err(e) => format!("unknown ({})", win_text(e)),
    }
}

fn locked_text() -> &'static str {
    match session::locked_now() {
        Some(true) => "yes",
        Some(false) => "no",
        None => "unknown",
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// cycles

fn cycles(ctx: &Ctx, power: &mut Win32Power, n: u32, pause_ms: u64) -> Result<(), String> {
    let cmd = Cmd::Cycles { n, pause_ms };
    ctx.print_plan(&cmd, None);
    if ctx.plan_only {
        return Ok(());
    }
    let out = &ctx.out;
    if ctx.dir.file(DataFile::Journal).exists() {
        return Err(format!(
            "a journal is left in {}: run `power_check cleanup` first",
            ctx.dir.root().display()
        ));
    }
    let original = ctx.plan_to_copy(power)?;
    let record = ctx.save(&ctx.run_record(cmd.name(), None, None))?;
    let copy = match ActiveCopy::make(original, &[], out, &ctx.log) {
        Ok(copy) => copy,
        Err(why) => {
            forget(&record, out);
            return Err(why);
        }
    };
    let guid = copy.guid();
    ctx.save(&ctx.run_record(cmd.name(), None, Some(guid)))?;
    ctx.print_undo(&cmd, Some(guid));
    // No RunOnce value and no guard: the copy is the only plan written, and `cleanup` puts the user's plan back.
    let mut exec = PowerExecutor::new(
        Timed::new(
            Win32Power::new(ctx.log.clone()),
            ctx.log.clone(),
            "PowerSetActiveScheme",
        ),
        FileNet::dry_run(ctx.dir.clone(), ctx.log.clone()),
        ctx.owner()?,
        Names::real(),
        ctx.dir.clone(),
        Arc::new(SystemLiveness),
        ctx.log.clone(),
    );
    let pause = u32::try_from(pause_ms).unwrap_or(u32::MAX);
    let mut failed = None;
    let mut done = 0;
    for i in 1..=n {
        if ctrl::stopped() {
            out.say(&format!("stopping before cycle {i}: {}", ctrl::why()));
            break;
        }
        let events = exec.run(&Command::ReadPlan);
        let values = match plan_of(&events) {
            Some(Ok(values)) if values.active == guid => values,
            other => {
                failed = Some(format!(
                    "cycle {i}: the plan read gave {other:?}, not the copy"
                ));
                break;
            }
        };
        let before = exec.plans().set_active_ms.len();
        let applied = exec.run(&Command::Apply {
            plan: guid,
            writes: writes_for(&values),
        });
        let apply_ms = exec.plans().set_active_ms[before..].to_vec();
        if !matches!(applied_of(&applied), Some(Ok(()))) {
            let text = applied.iter().map(describe).collect::<Vec<_>>().join("; ");
            failed = Some(format!("cycle {i}: {text}"));
            break;
        }
        pump(pause);
        let before = exec.plans().set_active_ms.len();
        let restored = exec.run(&Command::Restore {
            plan: None,
            why: RestoreWhy::WorkEnded,
        });
        let restore_ms = exec.plans().set_active_ms[before..].to_vec();
        if !matches!(restored_of(&restored), Some((true, 0))) {
            let text = restored.iter().map(describe).collect::<Vec<_>>().join("; ");
            failed = Some(format!("cycle {i}: {text}"));
            break;
        }
        done += 1;
        out.say(&format!(
            "cycle {i} of {n}: apply ok, PowerSetActiveScheme {}; restore ok, PowerSetActiveScheme {}",
            ms_list(&apply_ms),
            ms_list(&restore_ms)
        ));
        pump(pause);
    }
    // Whatever an apply left goes back while the copy is still the active plan, so it is in force again too.
    if ctx.dir.file(DataFile::Journal).exists() {
        let events = exec.run(&Command::Restore {
            plan: None,
            why: RestoreWhy::Quit,
        });
        for event in &events {
            out.say(&describe(event));
        }
    }
    let times = &exec.plans().set_active_ms;
    out.say(&format!(
        "{done} of {n} cycles done; PowerSetActiveScheme: {}",
        stats_text(times)
    ));
    let left = ctx.dir.file(DataFile::Journal).exists();
    drop(exec);
    let undone = copy.end();
    if undone && !left {
        forget(&record, out);
    }
    match (failed, left) {
        (Some(why), _) => Err(why),
        (None, true) => {
            Err("a journal is left after the cycles: run `power_check cleanup`".to_owned())
        }
        (None, false) => Ok(()),
    }
}

fn ms_list(list: &[f64]) -> String {
    match list {
        [] => "not called".to_owned(),
        _ => list
            .iter()
            .map(|ms| format!("{ms:.1} ms"))
            .collect::<Vec<_>>()
            .join(", "),
    }
}

fn stats_text(list: &[f64]) -> String {
    let mut sorted = list.to_vec();
    sorted.sort_by(f64::total_cmp);
    match (sorted.first(), sorted.last()) {
        (Some(min), Some(max)) => format!(
            "{} calls, min {min:.1} ms, median {:.1} ms, max {max:.1} ms",
            sorted.len(),
            sorted[sorted.len() / 2]
        ),
        _ => "no call".to_owned(),
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// medium-probe

fn medium_probe(ctx: &Ctx) -> Result<(), String> {
    ctx.print_plan(&Cmd::MediumProbe, None);
    if ctx.plan_only {
        return Ok(());
    }
    let out = &ctx.out;
    out.say(&format!("this process: {}", integrity_text()));
    let name = format!(r"Global\Wakeshot.Probe.{}", ctx.me.pid);
    let mutex = NamedMutex::open(&name).map_err(|e| format!("{name}: {}", win_text(e)))?;
    match mutex.take(0) {
        Ok(Some(_held)) => {
            out.say(&format!("{name}: created and taken"));
            Ok(())
        }
        Ok(None) => Err(format!("{name}: opened, but another process holds it")),
        Err(e) => Err(format!("{name}: opened, the take failed: {}", win_text(e))),
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// cleanup

fn cleanup(ctx: &Ctx, power: &mut Win32Power) -> Result<(), String> {
    ctx.print_plan(&Cmd::Cleanup, None);
    let out = ctx.out.clone();
    let look = look_around(ctx, power)?;
    let steps = tidy::plan(&look);
    out.say("steps:");
    for step in &steps {
        out.say(&format!("  {}", tidy::step_text(step)));
    }
    if ctx.plan_only {
        return Ok(());
    }
    let mut failed: BTreeSet<PathBuf> = BTreeSet::new();
    let mut problems = 0;
    for step in steps {
        match step {
            Step::Note(_) => {}
            Step::SetActive(plan) => match power.set_active(plan) {
                Ok(()) => out.say(&format!("made {plan} the active plan again")),
                Err(f) => {
                    out.warn(&format!("making {plan} active failed: {}", fail_text(f)));
                    problems += 1;
                }
            },
            Step::Restore {
                file,
                dir,
                owner,
                run_once,
            } => {
                if !restore_gone_run(ctx, power, &dir, owner, run_once) {
                    failed.insert(file);
                    problems += 1;
                }
            }
            Step::RemoveRunOnce(expected) => {
                let auto = AutoStart::real();
                match auto.get(StartKey::RunOnce, RUN_ONCE_NAME) {
                    Ok(Some(value)) if value == expected => {
                        match auto.remove(StartKey::RunOnce, RUN_ONCE_NAME) {
                            Ok(_) => out.say(&format!("removed the RunOnce value {RUN_ONCE_NAME}")),
                            Err(e) => {
                                out.warn(&format!("removing RunOnce: {}", win_text(e)));
                                problems += 1;
                            }
                        }
                    }
                    Ok(_) => out.say(&format!(
                        "the RunOnce value {RUN_ONCE_NAME} is gone already"
                    )),
                    Err(e) => {
                        out.warn(&format!("RunOnce read: {}", win_text(e)));
                        problems += 1;
                    }
                }
            }
            Step::Forget(file) => {
                if !failed.contains(&file) {
                    forget(&file, &out);
                }
            }
            Step::Sweep => match TestPlan::sweep() {
                Ok(list) if list.is_empty() => out.say("no Wakeshot test copy to delete"),
                Ok(list) => {
                    for plan in list {
                        out.say(&format!("deleted the test copy {plan}"));
                    }
                }
                Err(e) => {
                    out.warn(&format!("the sweep failed: {}", win_text(e)));
                    problems += 1;
                }
            },
        }
    }
    let original = ctx.home.original();
    let clean = ctx.home.runs().is_empty()
        && test_copies(power).is_some_and(|copies| copies.is_empty())
        && original.is_some()
        && power.active_plan().ok() == original;
    if clean {
        match ctx.home.forget_original() {
            Ok(()) => out.say(
                "nothing is left: forgot the recorded plan, so the next run records the active one",
            ),
            Err(why) => out.warn(&why),
        }
    }
    match problems {
        0 => Ok(()),
        n => Err(format!("{n} cleanup steps failed: see above")),
    }
}

fn look_around(ctx: &Ctx, power: &mut Win32Power) -> Result<tidy::Look, String> {
    let active = power
        .active_plan()
        .map_err(|f| format!("the active plan read: {}", fail_text(f)))?;
    let active_copy = test_copy_pid(&name_text(active)).map(|pid| {
        let lookup = match pid == ctx.me.pid {
            true => Lookup::Running(ctx.me.start),
            false => SystemLiveness.start_of_pid(pid),
        };
        (pid, lookup)
    });
    let mut runs = Vec::new();
    let mut bad_records = Vec::new();
    for (file, run) in ctx.home.runs() {
        match run {
            Err(why) => bad_records.push((file, why)),
            Ok(run) => {
                let process = ProcessRef {
                    pid: run.pid,
                    start: run.start,
                };
                let alive = match process == ctx.me {
                    true => Alive::Yes,
                    false => SystemLiveness.check(process),
                };
                let journal = journal_seen(&run.dir, &ctx.log);
                runs.push(tidy::RunSeen {
                    file,
                    run,
                    alive,
                    journal,
                });
            }
        }
    }
    let run_once = AutoStart::real()
        .get(StartKey::RunOnce, RUN_ONCE_NAME)
        .map_err(|e| format!("RunOnce read: {}", win_text(e)))?;
    Ok(tidy::Look {
        active,
        active_copy,
        original: ctx.home.original(),
        runs,
        bad_records,
        run_once,
    })
}

fn journal_seen(dir: &Path, log: &Arc<dyn Log>) -> JournalSeen {
    match FileNet::dry_run(DataDir::at(dir.to_path_buf()), log.clone()).load_journal() {
        JournalRead::Missing => JournalSeen::Missing,
        JournalRead::Read(file) => JournalSeen::Owner(file.owner),
        JournalRead::Corrupt => JournalSeen::Unreadable("it does not parse".to_owned()),
        JournalRead::Failed(f) => JournalSeen::Unreadable(fail_text(f)),
    }
}

/// The real restore routine on a gone run's journal, as restore mode runs it. With `run_once`, the clean-up also
/// clears the RunOnce value that run set; without, the net never touches it.
fn restore_gone_run(
    ctx: &Ctx,
    power: &mut Win32Power,
    dir: &Path,
    owner: JournalOwner,
    run_once: bool,
) -> bool {
    let data = DataDir::at(dir.to_path_buf());
    let names = Names::real();
    let done = match run_once {
        true => journal::restore_locked(
            power,
            &mut FileNet::restore_only(data, AutoStart::real(), ctx.log.clone()),
            None,
            Some(owner),
            &names,
            &*ctx.log,
        ),
        false => journal::restore_locked(
            power,
            &mut FileNet::dry_run(data, ctx.log.clone()),
            None,
            Some(owner),
            &names,
            &*ctx.log,
        ),
    };
    let ok = done.result.is_ok() && done.journal.is_empty();
    let text = format!(
        "restore of the journal in {}: found {:?}, {}, journal holds {}",
        dir.display(),
        done.found,
        result_text(&done.result),
        entries_text(&done.journal)
    );
    match ok {
        true => ctx.out.say(&text),
        false => ctx.out.warn(&text),
    }
    ok
}

/// The "Wakeshot test <pid>" copies in the plan list; `None` when the list does not read.
fn test_copies(power: &mut Win32Power) -> Option<Vec<Guid>> {
    let list = power.plans().ok()?;
    Some(
        list.into_iter()
            .filter(|&plan| test_copy_pid(&name_text(plan)).is_some())
            .collect(),
    )
}

fn forget(path: &Path, out: &Out) {
    match store::remove(path) {
        Ok(_) => out.say(&format!("removed the run record {}", path.display())),
        Err(e) => out.warn(&format!(
            "removing the run record {}: {}",
            path.display(),
            win_text(e)
        )),
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// The main loop

/// Runs until a stop (Ctrl+C, the console closing) or `ends_at`: `on_wake` after every wake, `on_tick` every
/// `every_ms` of wall time. Returns why it stopped.
fn run_loop(
    ends_at: Option<i64>,
    every_ms: i64,
    mut on_wake: impl FnMut(),
    mut on_tick: impl FnMut(Now),
) -> String {
    let mut next = clock::now().wall_ms + every_ms;
    loop {
        let now = clock::now().wall_ms;
        if next - now > every_ms {
            // The wall clock went back.
            next = now + every_ms;
        }
        let until = ends_at.map_or(next, |end| end.min(next));
        pump(u32::try_from((until - now).clamp(0, every_ms)).unwrap_or(0));
        on_wake();
        let now = clock::now();
        if now.wall_ms >= next {
            on_tick(now);
            // Ticks keep their own beat, so `--for 15` gets the tick at 15 s; after a long gap it starts again.
            next += every_ms;
            if next <= now.wall_ms {
                next = now.wall_ms + every_ms;
            }
        }
        if ctrl::stopped() {
            return ctrl::why().to_owned();
        }
        if ends_at.is_some_and(|end| now.wall_ms >= end) {
            return "the --for time is up".to_owned();
        }
    }
}

/// Waits up to `timeout_ms` for a wake or a message, then dispatches every message that came. After a stop it no
/// longer waits: the wake event resets on its first wait, and a closing console leaves the clean exit only 4.5 s.
fn pump(timeout_ms: u32) {
    let wake = ctrl::wake_handle();
    let timeout_ms = if ctrl::stopped() { 0 } else { timeout_ms };
    // SAFETY: the wake event lives as long as the process.
    let _ = unsafe { MsgWaitForMultipleObjects(Some(&[wake]), false, timeout_ms, QS_ALLINPUT) };
    let mut msg = MSG::default();
    // SAFETY: `msg` is a live local; each message goes to its window's procedure on this thread.
    while unsafe { PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE) }.as_bool() {
        // SAFETY: a message PeekMessageW just filled.
        unsafe {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

mod ctrl {
    //! The console control handler: Ctrl+C and Ctrl+Break stop the run; a closed console window, a sign-out or a
    //! shutdown stop it too, and the handler then waits for the clean exit, because Windows ends the process as soon
    //! as it returns (research, "Spike launch and stop checks").

    use std::ffi::c_void;
    use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU32, Ordering};

    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::Console::{
        CTRL_BREAK_EVENT, CTRL_C_EVENT, CTRL_CLOSE_EVENT, CTRL_LOGOFF_EVENT, CTRL_SHUTDOWN_EVENT,
        SetConsoleCtrlHandler,
    };
    use windows::Win32::System::Threading::{CreateEventW, SetEvent, WaitForSingleObject};
    use windows::core::{BOOL, PCWSTR};

    use super::{WinError, win_error};

    /// Below the 5 s Windows gives a handler (`SPI_GETHUNGAPPTIMEOUT`, Microsoft's `HandlerRoutine` page).
    const CLOSE_WAIT_MS: u32 = 4_500;

    static STOP: AtomicBool = AtomicBool::new(false);
    static KIND: AtomicU32 = AtomicU32::new(u32::MAX);
    static WAKE: AtomicIsize = AtomicIsize::new(0);
    static DONE: AtomicIsize = AtomicIsize::new(0);

    /// Wakes the main loop from any thread.
    #[derive(Clone, Copy)]
    pub struct Waker(isize);

    impl Waker {
        pub fn wake(self) {
            if self.0 != 0 {
                // SAFETY: the event is never closed.
                let _ = unsafe { SetEvent(HANDLE(self.0 as *mut c_void)) };
            }
        }
    }

    pub fn install() -> Result<(), WinError> {
        // SAFETY: two unnamed events with default security, kept open for the whole process.
        let wake = unsafe { CreateEventW(None, false, false, PCWSTR::null()) }
            .map_err(|e| win_error("CreateEventW", &e))?;
        // SAFETY: as above; manual reset, so a set stays set for the handler's wait.
        let done = unsafe { CreateEventW(None, true, false, PCWSTR::null()) }
            .map_err(|e| win_error("CreateEventW", &e))?;
        WAKE.store(wake.0 as isize, Ordering::SeqCst);
        DONE.store(done.0 as isize, Ordering::SeqCst);
        // SAFETY: a static function that touches only statics.
        unsafe { SetConsoleCtrlHandler(Some(on_ctrl), true) }
            .map_err(|e| win_error("SetConsoleCtrlHandler", &e))
    }

    pub fn waker() -> Waker {
        Waker(WAKE.load(Ordering::SeqCst))
    }

    pub fn wake_handle() -> HANDLE {
        HANDLE(WAKE.load(Ordering::SeqCst) as *mut c_void)
    }

    pub fn stopped() -> bool {
        STOP.load(Ordering::SeqCst)
    }

    pub fn why() -> &'static str {
        match KIND.load(Ordering::SeqCst) {
            CTRL_C_EVENT => "Ctrl+C",
            CTRL_BREAK_EVENT => "Ctrl+Break",
            CTRL_CLOSE_EVENT => "the console window closed",
            CTRL_LOGOFF_EVENT => "sign-out",
            CTRL_SHUTDOWN_EVENT => "shutdown",
            _ => "a stop",
        }
    }

    /// The clean exit is done: a waiting handler may return now.
    pub fn finished() {
        let done = DONE.load(Ordering::SeqCst);
        if done != 0 {
            // SAFETY: the event is never closed.
            let _ = unsafe { SetEvent(HANDLE(done as *mut c_void)) };
        }
    }

    unsafe extern "system" fn on_ctrl(kind: u32) -> BOOL {
        KIND.store(kind, Ordering::SeqCst);
        STOP.store(true, Ordering::SeqCst);
        waker().wake();
        if kind != CTRL_C_EVENT && kind != CTRL_BREAK_EVENT {
            let done = DONE.load(Ordering::SeqCst);
            if done != 0 {
                // SAFETY: the event is never closed.
                unsafe { WaitForSingleObject(HANDLE(done as *mut c_void), CLOSE_WAIT_MS) };
            }
        }
        true.into()
    }
}

mod window {
    //! A hidden top-level window: session notices and the end-session messages go only to top-level windows, never
    //! to message-only ones.

    use std::cell::RefCell;

    use windows::Win32::Foundation::{
        ERROR_CLASS_ALREADY_EXISTS, GetLastError, HINSTANCE, HWND, LPARAM, LRESULT, WPARAM,
    };
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, RegisterClassW, WINDOW_EX_STYLE, WNDCLASSW,
        WS_OVERLAPPED,
    };
    use windows::core::{PCWSTR, w};

    use super::{WinError, win_error};

    /// Gets every message first; `None` hands it to `DefWindowProcW`.
    pub type Handler = Box<dyn FnMut(u32, WPARAM, LPARAM) -> Option<LRESULT>>;

    thread_local! {
        static HANDLER: RefCell<Option<Handler>> = const { RefCell::new(None) };
    }

    const CLASS: PCWSTR = w!("WakeshotPowerCheck");

    pub struct Window {
        pub hwnd: HWND,
    }

    impl Window {
        /// One window per run, on the thread that pumps its messages.
        pub fn create(handler: Handler) -> Result<Window, WinError> {
            // SAFETY: null asks for this exe's own module, which needs no free.
            let module = unsafe { GetModuleHandleW(PCWSTR::null()) }
                .map_err(|e| win_error("GetModuleHandleW", &e))?;
            let instance = HINSTANCE::from(module);
            let class = WNDCLASSW {
                lpfnWndProc: Some(procedure),
                hInstance: instance,
                lpszClassName: CLASS,
                ..Default::default()
            };
            // SAFETY: the class struct and its static name live across the call.
            if unsafe { RegisterClassW(&class) } == 0 {
                // SAFETY: read right after the failed call, on the same thread.
                let error = unsafe { GetLastError() };
                if error != ERROR_CLASS_ALREADY_EXISTS {
                    return Err(WinError {
                        call: "RegisterClassW",
                        code: error.0,
                    });
                }
            }
            HANDLER.with(|slot| *slot.borrow_mut() = Some(handler));
            // SAFETY: the class is registered above; the window is never shown and is destroyed on drop, on this
            // thread.
            let hwnd = unsafe {
                CreateWindowExW(
                    WINDOW_EX_STYLE(0),
                    CLASS,
                    w!("Wakeshot power check"),
                    WS_OVERLAPPED,
                    0,
                    0,
                    0,
                    0,
                    None,
                    None,
                    Some(instance),
                    None,
                )
            };
            match hwnd {
                Ok(hwnd) => Ok(Window { hwnd }),
                Err(e) => {
                    HANDLER.with(|slot| slot.borrow_mut().take());
                    Err(win_error("CreateWindowExW", &e))
                }
            }
        }
    }

    impl Drop for Window {
        fn drop(&mut self) {
            // Taken first, so the messages the destroy sends go to `DefWindowProcW`.
            let handler = HANDLER.with(|slot| slot.borrow_mut().take());
            // SAFETY: the window made in `create`, on this thread.
            let _ = unsafe { DestroyWindow(self.hwnd) };
            drop(handler);
        }
    }

    unsafe extern "system" fn procedure(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        let answer = HANDLER.with(|slot| match slot.try_borrow_mut() {
            Ok(mut handler) => handler.as_mut().and_then(|f| f(msg, wparam, lparam)),
            // A message sent while the handler runs: the handler never pumps, so only Windows' own default fits.
            Err(_) => None,
        });
        match answer {
            Some(result) => result,
            // SAFETY: the arguments Windows passed in.
            None => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// Each step's time, for check 6 and check 15

/// Logs every plan and safety net call with its time; keeps each `set_active` time.
struct Timed<T> {
    inner: T,
    log: Arc<dyn Log>,
    set_active_name: &'static str,
    set_active_ms: Vec<f64>,
}

impl<T> Timed<T> {
    fn new(inner: T, log: Arc<dyn Log>, set_active_name: &'static str) -> Self {
        Self {
            inner,
            log,
            set_active_name,
            set_active_ms: Vec::new(),
        }
    }

    fn inner(&self) -> &T {
        &self.inner
    }

    fn note<R>(&self, what: &str, started: Instant, result: &Result<R, Failure>) -> f64 {
        let ms = ms_since(started);
        let how = match result {
            Ok(_) => "ok".to_owned(),
            Err(f) => format!("failed: {}", fail_text(*f)),
        };
        self.log.line(
            Level::Info,
            AREA,
            &format!("step {what}: {how}, {ms:.1} ms"),
        );
        ms
    }
}

impl<T: PlanIo> PlanIo for Timed<T> {
    fn active_plan(&mut self) -> Result<Guid, Failure> {
        let started = Instant::now();
        let result = self.inner.active_plan();
        self.note("active plan read", started, &result);
        result
    }

    fn plans(&mut self) -> Result<Vec<Guid>, Failure> {
        let started = Instant::now();
        let result = self.inner.plans();
        self.note("plan list", started, &result);
        result
    }

    fn can_write(&mut self, key: SettingKey) -> Result<(), Failure> {
        let started = Instant::now();
        let result = self.inner.can_write(key);
        self.note(&format!("access check {}", key_name(key)), started, &result);
        result
    }

    fn read(&mut self, plan: Guid, key: SettingKey) -> Result<u32, Failure> {
        let started = Instant::now();
        let result = self.inner.read(plan, key);
        let value = result.map_or_else(|_| String::new(), |v| format!(" = {v}"));
        let what = format!("read {}{value} on {plan}", key_name(key));
        self.note(&what, started, &result);
        result
    }

    fn write(&mut self, plan: Guid, key: SettingKey, value: u32) -> Result<(), Failure> {
        let started = Instant::now();
        let result = self.inner.write(plan, key, value);
        let what = format!("write {} = {value} on {plan}", key_name(key));
        self.note(&what, started, &result);
        result
    }

    fn set_active(&mut self, plan: Guid) -> Result<(), Failure> {
        let started = Instant::now();
        let result = self.inner.set_active(plan);
        let what = format!("{} {plan}", self.set_active_name);
        let ms = self.note(&what, started, &result);
        self.set_active_ms.push(ms);
        result
    }

    fn values(&mut self) -> Result<PlanValues, Failure> {
        let started = Instant::now();
        let result = self.inner.values();
        self.note("values read", started, &result);
        result
    }
}

impl<T: SafetyNet> SafetyNet for Timed<T> {
    fn load_journal(&mut self) -> JournalRead {
        let started = Instant::now();
        let read = self.inner.load_journal();
        let what = match &read {
            JournalRead::Missing => "missing".to_owned(),
            JournalRead::Corrupt => "corrupt".to_owned(),
            JournalRead::Failed(f) => format!("failed: {}", fail_text(*f)),
            JournalRead::Read(file) => format!("{} entries", file.entries.len()),
        };
        self.log.line(
            Level::Info,
            AREA,
            &format!("step journal load: {what}, {:.1} ms", ms_since(started)),
        );
        read
    }

    fn save_journal(&mut self, journal: &JournalFile) -> Result<(), Failure> {
        let started = Instant::now();
        let result = self.inner.save_journal(journal);
        let what = format!("journal save ({} entries)", journal.entries.len());
        self.note(&what, started, &result);
        result
    }

    fn delete_journal(&mut self) -> Result<(), Failure> {
        let started = Instant::now();
        let result = self.inner.delete_journal();
        self.note("journal delete", started, &result);
        result
    }

    fn set_aside_journal(&mut self) -> Result<(), Failure> {
        let started = Instant::now();
        let result = self.inner.set_aside_journal();
        self.note("journal set aside", started, &result);
        result
    }

    fn set_run_once(&mut self) -> Result<bool, Failure> {
        let started = Instant::now();
        let result = self.inner.set_run_once();
        self.note("RunOnce set", started, &result);
        result
    }

    fn clear_run_once(&mut self) -> Result<(), Failure> {
        let started = Instant::now();
        let result = self.inner.clear_run_once();
        self.note("RunOnce clear", started, &result);
        result
    }

    fn ensure_guard(&mut self) -> Result<bool, Failure> {
        let started = Instant::now();
        let result = self.inner.ensure_guard();
        self.note("guard ensure", started, &result);
        result
    }

    fn stop_guard(&mut self) -> Result<(), Failure> {
        let started = Instant::now();
        let result = self.inner.stop_guard();
        self.note("guard stop", started, &result);
        result
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// Output

/// A line for the console and the run's log.
#[derive(Clone)]
struct Out {
    log: Arc<dyn Log>,
}

impl Out {
    fn say(&self, text: &str) {
        self.emit(Level::Info, text);
    }

    fn warn(&self, text: &str) {
        self.emit(Level::Warn, text);
    }

    fn lines(&self, lines: &[String]) {
        for line in lines {
            self.say(line);
        }
    }

    fn emit(&self, level: Level, text: &str) {
        let tag = match level {
            Level::Info => "",
            Level::Warn => "WARN ",
            Level::Error => "ERROR ",
        };
        // A console that is gone (its window closed) must not stop the clean exit, so a failed write is dropped.
        let _ = writeln!(std::io::stdout(), "{} {tag}{text}", local_clock());
        self.log.line(level, AREA, text);
    }
}

/// `--plan-only` writes nothing, not even a log.
struct NoLog;

impl Log for NoLog {
    fn line(&self, _: Level, _: &'static str, _: &str) {}
}

fn print_err(text: &str) {
    let _ = writeln!(std::io::stderr(), "power_check: {text}");
}

fn local_clock() -> String {
    // SAFETY: no arguments; returns a struct.
    let t = unsafe { GetLocalTime() };
    format!(
        "{:02}:{:02}:{:02}.{:03}",
        t.wHour, t.wMinute, t.wSecond, t.wMilliseconds
    )
}

fn local_stamp() -> String {
    // SAFETY: no arguments; returns a struct.
    let t = unsafe { GetLocalTime() };
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:03}",
        t.wYear, t.wMonth, t.wDay, t.wHour, t.wMinute, t.wSecond, t.wMilliseconds
    )
}

fn ms_since(started: Instant) -> f64 {
    started.elapsed().as_secs_f64() * 1000.0
}

fn absolute(path: &Path) -> Result<PathBuf, String> {
    std::path::absolute(path).map_err(|e| format!("{}: {e}", path.display()))
}

fn win_error(call: &'static str, error: &windows::core::Error) -> WinError {
    WinError {
        call,
        code: WIN32_ERROR::from_error(error).map_or(error.code().0 as u32, |e| e.0),
    }
}

fn win_text(e: WinError) -> String {
    format!("{} code {}", e.call, e.code)
}

fn fail_text(f: Failure) -> String {
    format!("{} code {}", f.call, f.code)
}

fn key_name(key: SettingKey) -> &'static str {
    PlanSetting::from(key).name()
}

fn lid_text(value: u32) -> &'static str {
    match value {
        0 => "Do nothing",
        1 => "Sleep",
        2 => "Hibernate",
        3 => "Shut down",
        _ => "?",
    }
}

fn sleep_text(seconds: u32) -> String {
    match seconds {
        0 => "Never".to_owned(),
        s => format!("{s} s"),
    }
}

fn values_text(v: &PlanValues) -> String {
    format!(
        "lid AC {} ({}), lid DC {} ({}), sleep AC {}, sleep DC {}, energy saver DC {}%",
        v.lid_ac,
        lid_text(v.lid_ac),
        v.lid_dc,
        lid_text(v.lid_dc),
        sleep_text(v.sleep_ac),
        sleep_text(v.sleep_dc),
        v.saver_threshold_dc
    )
}

fn result_text<E: std::fmt::Debug>(result: &Result<(), E>) -> String {
    match result {
        Ok(()) => "ok".to_owned(),
        Err(e) => format!("failed: {e:?}"),
    }
}

fn entries_text(entries: &[JournalEntry]) -> String {
    match entries {
        [] => "nothing".to_owned(),
        _ => entries
            .iter()
            .map(|e| {
                let restored = if e.restored { ", restored" } else { "" };
                format!(
                    "{} {} to {} on {}{restored}",
                    key_name(e.key),
                    e.original,
                    e.written,
                    e.plan
                )
            })
            .collect::<Vec<_>>()
            .join("; "),
    }
}

fn writes_text(writes: &[(SettingKey, u32)]) -> String {
    match writes {
        [] => "nothing to write: the copy has the targets already".to_owned(),
        _ => writes
            .iter()
            .map(|&(key, value)| format!("{} {value}", key_name(key)))
            .collect::<Vec<_>>()
            .join(", "),
    }
}

/// What the core sends at a hold start: the three keys, less the ones at the target already.
fn writes_for(values: &PlanValues) -> Vec<(SettingKey, u32)> {
    [
        (SettingKey::LidAc, values.lid_ac),
        (SettingKey::LidDc, values.lid_dc),
        (SettingKey::SleepDc, values.sleep_dc),
    ]
    .into_iter()
    .filter(|&(_, now)| now != TARGET)
    .map(|(key, _)| (key, TARGET))
    .collect()
}

fn plan_of(events: &[Event]) -> Option<Result<PlanValues, Failure>> {
    events.iter().find_map(|e| match e {
        Event::Plan(read) => Some(*read),
        _ => None,
    })
}

fn applied_of(events: &[Event]) -> Option<Result<(), ApplyError>> {
    events.iter().find_map(|e| match e {
        Event::Done(Outcome::Applied { result, .. }) => Some(*result),
        _ => None,
    })
}

/// The restore's result and how many entries it left.
fn restored_of(events: &[Event]) -> Option<(bool, usize)> {
    events.iter().find_map(|e| match e {
        Event::Done(Outcome::Restored {
            result, journal, ..
        }) => Some((result.is_ok(), journal.len())),
        _ => None,
    })
}

fn describe(event: &Event) -> String {
    match event {
        Event::Lid { closed } => format!("lid {}", if *closed { "closed" } else { "open" }),
        Event::Power { ac, battery_pct } => format!(
            "power: {}, battery {}",
            if *ac { "AC" } else { "on battery" },
            battery_pct.map_or_else(|| "unknown".to_owned(), |p| format!("{p}%"))
        ),
        Event::ActivePlan(plan) => format!("active plan is now {plan} \"{}\"", name_text(*plan)),
        Event::Suspending => "suspending (PBT_APMSUSPEND)".to_owned(),
        Event::Resumed => "resumed (PBT_APMRESUMEAUTOMATIC)".to_owned(),
        Event::Plan(Ok(values)) => format!("plan {}: {}", values.active, values_text(values)),
        Event::Plan(Err(f)) => format!("the plan read failed: {}", fail_text(*f)),
        Event::UserChanged(owned) => format!(
            "the user changed {} on {}: entry dropped",
            key_name(owned.key),
            owned.plan
        ),
        Event::JournalCorrupt => "the journal did not parse: set aside".to_owned(),
        Event::Done(Outcome::Applied {
            plan,
            written,
            result,
            journal,
        }) => format!(
            "apply on {plan}: wrote {}; {}; journal holds {}",
            match written.as_slice() {
                [] => "nothing".to_owned(),
                keys => keys
                    .iter()
                    .map(|&k| key_name(k))
                    .collect::<Vec<_>>()
                    .join(", "),
            },
            result_text(result),
            entries_text(journal)
        ),
        Event::Done(Outcome::Restored {
            plan,
            why,
            result,
            journal,
        }) => format!(
            "restore ({why:?}{}): {}; journal holds {}",
            plan.map_or_else(String::new, |p| format!(", plan {p}")),
            result_text(result),
            entries_text(journal)
        ),
        other => format!("{other:?}"),
    }
}

/// `WM_QUERYENDSESSION` and `WM_ENDSESSION` flags (Microsoft's pages for both messages).
fn end_flags(lparam: LPARAM) -> String {
    const CLOSEAPP: isize = 0x1;
    const CRITICAL: isize = 0x4000_0000;
    const LOGOFF: isize = 0x8000_0000;
    let mut names = Vec::new();
    if lparam.0 & LOGOFF != 0 {
        names.push("sign-out");
    }
    if lparam.0 & CLOSEAPP != 0 {
        names.push("Restart Manager close");
    }
    if lparam.0 & CRITICAL != 0 {
        names.push("forced");
    }
    if names.is_empty() {
        names.push("shutdown or restart");
    }
    format!("lparam {:#x}: {}", lparam.0, names.join(", "))
}

/// A session message as text, with the raw code, so codes the core has no event for show too.
fn message_text(msg: u32, wparam: WPARAM, lparam: LPARAM) -> String {
    match msg {
        WM_WTSSESSION_CHANGE => {
            let name = match wparam.0 {
                1 => "WTS_CONSOLE_CONNECT",
                2 => "WTS_CONSOLE_DISCONNECT",
                3 => "WTS_REMOTE_CONNECT",
                4 => "WTS_REMOTE_DISCONNECT",
                5 => "WTS_SESSION_LOGON",
                6 => "WTS_SESSION_LOGOFF",
                7 => "WTS_SESSION_LOCK",
                8 => "WTS_SESSION_UNLOCK",
                9 => "WTS_SESSION_REMOTE_CONTROL",
                10 => "WTS_SESSION_CREATE",
                11 => "WTS_SESSION_TERMINATE",
                _ => "unknown code",
            };
            format!(
                "WM_WTSSESSION_CHANGE {} ({name}), session {}",
                wparam.0, lparam.0
            )
        }
        WM_QUERYENDSESSION => format!("WM_QUERYENDSESSION ({})", end_flags(lparam)),
        WM_ENDSESSION => format!(
            "WM_ENDSESSION, ending {} ({})",
            wparam.0 != 0,
            end_flags(lparam)
        ),
        other => format!("message {other:#x}"),
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// Plans and the token

/// A plan's friendly name, or why it did not read.
fn name_text(plan: Guid) -> String {
    match plan_name(plan) {
        Ok(name) => name,
        Err(e) => format!("<name unread: {}>", win_text(e)),
    }
}

fn plan_name(plan: Guid) -> Result<String, WinError> {
    const CALL: &str = "PowerReadFriendlyName";
    let scheme = GUID::from_u128(plan.0);
    let mut size = 0u32;
    // SAFETY: a null buffer asks only for the size, written to `size`.
    let rc = unsafe { PowerReadFriendlyName(None, Some(&scheme), None, None, None, &mut size) };
    if rc != ERROR_SUCCESS && rc != ERROR_MORE_DATA {
        return Err(WinError {
            call: CALL,
            code: rc.0,
        });
    }
    let mut buf = vec![0u8; size as usize];
    // SAFETY: `buf` holds `size` bytes, and both live across the call.
    let rc = unsafe {
        PowerReadFriendlyName(
            None,
            Some(&scheme),
            None,
            None,
            Some(buf.as_mut_ptr()),
            &mut size,
        )
    };
    if rc != ERROR_SUCCESS {
        return Err(WinError {
            call: CALL,
            code: rc.0,
        });
    }
    let wide: Vec<u16> = buf
        .as_chunks::<2>()
        .0
        .iter()
        .map(|&pair| u16::from_le_bytes(pair))
        .take_while(|&unit| unit != 0)
        .collect();
    Ok(String::from_utf16_lossy(&wide))
}

/// The pid of a "Wakeshot test <digits>" copy, the only names a sweep deletes.
fn test_copy_pid(name: &str) -> Option<u32> {
    let digits = name.strip_prefix(COPY_PREFIX)?;
    match !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) {
        true => digits.parse().ok(),
        false => None,
    }
}

fn integrity_text() -> String {
    match integrity() {
        Ok((rid, elevation)) => format!(
            "integrity {} (0x{rid:04X}), elevation type {}",
            level_name(rid),
            elevation_name(elevation)
        ),
        Err(e) => format!("integrity unread ({})", win_text(e)),
    }
}

fn level_name(rid: u32) -> &'static str {
    match rid {
        0x0000 => "Untrusted",
        0x1000 => "Low",
        0x2000 => "Medium",
        0x2100 => "Medium Plus",
        0x3000 => "High",
        0x4000 => "System",
        _ => "other",
    }
}

fn elevation_name(kind: u32) -> &'static str {
    match kind {
        1 => "Default",
        2 => "Full",
        3 => "Limited",
        _ => "other",
    }
}

/// This process's integrity level (the mandatory label's last sub-authority) and elevation type.
fn integrity() -> Result<(u32, u32), WinError> {
    let mut token = HANDLE::default();
    // SAFETY: the pseudo handle of this process; the token is closed by `Owned`.
    unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) }
        .map_err(|e| win_error("OpenProcessToken", &e))?;
    // SAFETY: the token is new and owned only here.
    let token = unsafe { Owned::new(token) };
    let label = token_info(*token, TokenIntegrityLevel)?;
    // SAFETY: the buffer holds a TOKEN_MANDATORY_LABEL whose SID points into the same buffer.
    let rid = unsafe {
        let sid = (*label.as_ptr().cast::<TOKEN_MANDATORY_LABEL>()).Label.Sid;
        let count = u32::from(*GetSidSubAuthorityCount(sid));
        *GetSidSubAuthority(sid, count.saturating_sub(1))
    };
    let elevation = token_info(*token, TokenElevationType)?
        .first()
        .map_or(0, |&word| word as u32);
    Ok((rid, elevation))
}

/// One token information class, in `u64`s: the answer can hold pointers into itself.
fn token_info(token: HANDLE, class: TOKEN_INFORMATION_CLASS) -> Result<Vec<u64>, WinError> {
    let mut len = 0u32;
    // SAFETY: a size query; it fails by design and only sets `len`.
    let _ = unsafe { GetTokenInformation(token, class, None, 0, &mut len) };
    let mut buf = vec![0u64; (len as usize).div_ceil(8).max(1)];
    // SAFETY: the buffer holds at least `len` bytes and lives across the call.
    unsafe {
        GetTokenInformation(
            token,
            class,
            Some(buf.as_mut_ptr().cast::<c_void>()),
            len,
            &mut len,
        )
    }
    .map_err(|e| win_error("GetTokenInformation", &e))?;
    Ok(buf)
}

// ---------------------------------------------------------------------------------------------------------------------
// The harness folder

/// Where the harness keeps what a later `cleanup` needs, apart from the data folders: those differ per command, and
/// `fake-tray`'s is the real one.
struct Home {
    root: PathBuf,
}

/// One run that may leave something behind: its data folder, the RunOnce command it set, its copy.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Run {
    pid: u32,
    start: FileTime,
    command: String,
    dir: PathBuf,
    run_once: Option<String>,
    copy: Option<Guid>,
}

impl Home {
    fn at(root: PathBuf) -> Home {
        Home { root }
    }

    fn default_root() -> Result<PathBuf, String> {
        std::env::var_os("LOCALAPPDATA")
            .filter(|v| !v.is_empty())
            .map(|v| PathBuf::from(v).join("Wakeshot-harness"))
            .ok_or_else(|| "LOCALAPPDATA is not set: pass --home <folder>".to_owned())
    }

    fn root(&self) -> &Path {
        &self.root
    }

    fn data_dir(&self, command: &str) -> PathBuf {
        self.root.join(command)
    }

    fn original_file(&self) -> PathBuf {
        self.root.join("original-plan.txt")
    }

    fn original(&self) -> Option<Guid> {
        std::fs::read_to_string(self.original_file())
            .ok()?
            .trim()
            .parse()
            .ok()
    }

    /// `true`: recorded now. The first writer wins, so two runs that start at once record one plan.
    fn keep_original(&self, plan: Guid) -> Result<bool, String> {
        let file = self.original_file();
        std::fs::create_dir_all(&self.root).map_err(|e| format!("{}: {e}", self.root.display()))?;
        let created = OpenOptions::new().write(true).create_new(true).open(&file);
        match created {
            Ok(mut f) => {
                f.write_all(format!("{plan}\r\n").as_bytes())
                    .and_then(|()| f.sync_all())
                    .map_err(|e| format!("{}: {e}", file.display()))?;
                Ok(true)
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Ok(false),
            Err(e) => Err(format!("{}: {e}", file.display())),
        }
    }

    fn forget_original(&self) -> Result<(), String> {
        store::remove(&self.original_file())
            .map(|_| ())
            .map_err(|e| format!("removing the recorded plan: {}", win_text(e)))
    }

    fn runs_dir(&self) -> PathBuf {
        self.root.join("runs")
    }

    fn save(&self, run: &Run) -> Result<PathBuf, String> {
        let path = self
            .runs_dir()
            .join(format!("{}-{}.json", run.pid, run.start.0));
        store::write_atomic(&path, run.to_json().as_bytes())
            .map_err(|e| format!("saving the run record {}: {}", path.display(), win_text(e)))?;
        Ok(path)
    }

    fn runs(&self) -> Vec<(PathBuf, Result<Run, String>)> {
        let Ok(entries) = std::fs::read_dir(self.runs_dir()) else {
            return Vec::new();
        };
        let mut runs: Vec<_> = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
            .map(|path| {
                let run = std::fs::read_to_string(&path)
                    .map_err(|e| e.to_string())
                    .and_then(|text| Run::parse(&text));
                (path, run)
            })
            .collect();
        runs.sort_by(|a, b| a.0.cmp(&b.0));
        runs
    }
}

impl Run {
    fn to_json(&self) -> String {
        serde_json::json!({
            "pid": self.pid,
            "start": self.start.0.to_string(),
            "command": self.command,
            "dir": self.dir.to_string_lossy(),
            "runOnce": self.run_once,
            "copy": self.copy.map(|g| g.to_string()),
        })
        .to_string()
    }

    fn parse(text: &str) -> Result<Run, String> {
        let v: serde_json::Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
        Ok(Run {
            pid: v["pid"]
                .as_u64()
                .and_then(|pid| u32::try_from(pid).ok())
                .ok_or("no pid")?,
            start: FileTime(text_field(&v, "start")?.parse().map_err(|_| "bad start")?),
            command: text_field(&v, "command")?.to_owned(),
            dir: PathBuf::from(text_field(&v, "dir")?),
            run_once: optional_text(&v, "runOnce")?,
            copy: optional_text(&v, "copy")?
                .map(|g| g.parse::<Guid>().map_err(|_| "bad copy"))
                .transpose()?,
        })
    }
}

fn text_field<'a>(v: &'a serde_json::Value, name: &str) -> Result<&'a str, String> {
    v[name].as_str().ok_or(format!("no {name}"))
}

fn optional_text(v: &serde_json::Value, name: &str) -> Result<Option<String>, String> {
    match &v[name] {
        serde_json::Value::Null => Ok(None),
        serde_json::Value::String(s) => Ok(Some(s.clone())),
        _ => Err(format!("{name} is not text")),
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// Pure parts, tested below

mod cli {
    //! The command line. Each command takes only its own options, so a typo fails before anything changes.

    use std::path::PathBuf;

    pub const DEFAULT_PAUSE_MS: u64 = 1_000;
    const MAX_CYCLES: u32 = 1_000;
    const MAX_FOR_S: u64 = 7 * 24 * 3_600;
    const MAX_PAUSE_MS: u64 = 60_000;

    pub const USAGE: &str = r#"power_check: plays the tray's power side by hand, on a plan copy, for the manual checks
of the power track (docs/tasks/p3-harness.md). The guard and restore mode it starts are the real tray exe's.

usage: power_check <command> [options]

commands:
  hold [--no-request] [--active-copy] [--for <s>]
      Holds SYSTEM and EXECUTION power requests (retaken after each resume) and logs a tick every 5 s (wall,
      unbiased, input idle), a line per lid, power, plan, suspend and resume event, and a GAP line when a tick
      comes late. --no-request: no request. --active-copy: a copy of the active plan with lid DC 0 (Do nothing)
      and sleep DC 0 (Never) is the active plan for the run; at exit the recorded plan is active again and the
      copy is deleted. Checks 1, 2 and 3.
  fake-tray [--guard <exe>] [--for <s>]
      A plan copy, never active; the real Apply to it through the executor (CopyAsActive), with the real
      journal (owner: this process), RunOnce value and guard (the tray exe in --guard mode); then it waits and
      runs the ownership check every 10 s. Checks 4 and 5: end it in Task Manager or with
      taskkill /PID <pid> /F. Ctrl+C restores and deletes the copy. Data folder: the real one.
  end-session [--guard <exe>] [--for <s>]
      Like fake-tray in its own data folder, plus a hidden top-level window: at WM_QUERYENDSESSION and
      WM_ENDSESSION it restores at once and logs each step's time. Check 6.
  session-watch [--for <s>]
      A hidden top-level window registered for session notices; logs each one (lock, unlock, console).
  cycles <n> [--pause <ms>]
      A plan copy made active; n Apply and Restore cycles on it through the executor, --pause ms apart
      (default 1000), each PowerSetActiveScheme time logged; at the end the recorded plan is active again and
      the copy is deleted. Check 15.
  medium-probe
      Prints its integrity level and creates Global\Wakeshot.Probe.<pid>. Check 11, first part.
  cleanup
      Makes the recorded plan active again when the active plan is the copy of a gone process; runs the
      restore on each gone run's journal; removes the RunOnce value a gone run set; deletes every
      "Wakeshot test <pid>" copy whose process is gone. It never touches a live run's.

options:
  --dir <folder>   the data folder (journal.json, logs\wakeshot.log). Default: <home>\<command>; for
                   fake-tray the real one (%LocalAppData%\Wakeshot, or WAKESHOT_DATA_DIR in a debug build).
  --home <folder>  the harness folder: the plan active at the first start (original-plan.txt) and one
                   record per run (runs\). Default: %LocalAppData%\Wakeshot-harness.
  --plan-only      print what the command would do and how to undo it; change nothing, write no log.
  --for <s>        hold, fake-tray, end-session, session-watch: end cleanly after s seconds. Without it
                   they run until Ctrl+C or the console window closes.
  --guard <exe>    fake-tray, end-session: the tray exe. Default: wakeshot-tray.exe in the folder above
                   this exe (cargo puts examples in <profile>\examples).

Every command prints the user's values (active plan; lid AC and DC; sleep AC and DC; energy saver DC) at
start and at exit, and the exact commands that undo what it changes. Run one command that writes RunOnce
(fake-tray, end-session) at a time: they share the one value.

build: cargo build --locked -p wakeshot-windows --features test-support --example power_check
       cargo build --locked -p wakeshot-tray        (the same --target and profile)"#;

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum Cmd {
        Help,
        Hold { request: bool, active_copy: bool },
        FakeTray,
        EndSession,
        SessionWatch,
        Cycles { n: u32, pause_ms: u64 },
        MediumProbe,
        Cleanup,
    }

    impl Cmd {
        pub fn name(&self) -> &'static str {
            match self {
                Cmd::Help => "help",
                Cmd::Hold { .. } => "hold",
                Cmd::FakeTray => "fake-tray",
                Cmd::EndSession => "end-session",
                Cmd::SessionWatch => "session-watch",
                Cmd::Cycles { .. } => "cycles",
                Cmd::MediumProbe => "medium-probe",
                Cmd::Cleanup => "cleanup",
            }
        }

        fn takes(&self, flag: &str) -> bool {
            match flag {
                "--dir" | "--home" | "--plan-only" => true,
                "--for" => matches!(
                    self,
                    Cmd::Hold { .. } | Cmd::FakeTray | Cmd::EndSession | Cmd::SessionWatch
                ),
                "--guard" => matches!(self, Cmd::FakeTray | Cmd::EndSession),
                "--no-request" | "--active-copy" => matches!(self, Cmd::Hold { .. }),
                "--pause" => matches!(self, Cmd::Cycles { .. }),
                _ => false,
            }
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct Args {
        pub cmd: Cmd,
        pub dir: Option<PathBuf>,
        pub home: Option<PathBuf>,
        pub guard: Option<PathBuf>,
        pub for_s: Option<u64>,
        pub plan_only: bool,
    }

    impl Args {
        fn of(cmd: Cmd) -> Args {
            Args {
                cmd,
                dir: None,
                home: None,
                guard: None,
                for_s: None,
                plan_only: false,
            }
        }
    }

    pub fn parse(args: &[String]) -> Result<Args, String> {
        let mut rest = args.iter().map(String::as_str);
        let first = rest
            .next()
            .ok_or("no command given; `power_check --help` lists them")?;
        let cmd = match first {
            "help" | "--help" | "-h" | "/?" => return Ok(Args::of(Cmd::Help)),
            "hold" => Cmd::Hold {
                request: true,
                active_copy: false,
            },
            "fake-tray" => Cmd::FakeTray,
            "end-session" => Cmd::EndSession,
            "session-watch" => Cmd::SessionWatch,
            "cycles" => {
                let n = rest
                    .next()
                    .ok_or("cycles needs a count, for example `cycles 50`")?;
                Cmd::Cycles {
                    n: number(n, "the cycle count", 1, u64::from(MAX_CYCLES))? as u32,
                    pause_ms: DEFAULT_PAUSE_MS,
                }
            }
            "medium-probe" => Cmd::MediumProbe,
            "cleanup" => Cmd::Cleanup,
            other => return Err(format!("unknown command `{other}`")),
        };
        let mut args = Args::of(cmd);
        let mut seen: Vec<&str> = Vec::new();
        while let Some(flag) = rest.next() {
            if flag == "--help" || flag == "-h" {
                return Ok(Args::of(Cmd::Help));
            }
            if !args.cmd.takes(flag) {
                return Err(match flag.starts_with("--") {
                    true => format!("{} does not take {flag}", args.cmd.name()),
                    false => format!("unexpected argument `{flag}`"),
                });
            }
            if seen.contains(&flag) {
                return Err(format!("{flag} is given twice"));
            }
            seen.push(flag);
            let mut value = || rest.next().ok_or(format!("{flag} needs a value"));
            match flag {
                "--dir" => args.dir = Some(PathBuf::from(value()?)),
                "--home" => args.home = Some(PathBuf::from(value()?)),
                "--guard" => args.guard = Some(PathBuf::from(value()?)),
                "--for" => args.for_s = Some(number(value()?, "--for", 1, MAX_FOR_S)?),
                "--plan-only" => args.plan_only = true,
                "--pause" => {
                    let ms = number(value()?, "--pause", 0, MAX_PAUSE_MS)?;
                    if let Cmd::Cycles { pause_ms, .. } = &mut args.cmd {
                        *pause_ms = ms;
                    }
                }
                "--no-request" => {
                    if let Cmd::Hold { request, .. } = &mut args.cmd {
                        *request = false;
                    }
                }
                _ => {
                    if let Cmd::Hold { active_copy, .. } = &mut args.cmd {
                        *active_copy = true;
                    }
                }
            }
        }
        Ok(args)
    }

    fn number(text: &str, what: &str, min: u64, max: u64) -> Result<u64, String> {
        match text.parse::<u64>() {
            Ok(n) if (min..=max).contains(&n) => Ok(n),
            _ => Err(format!(
                "{what} must be a whole number from {min} to {max}, not `{text}`"
            )),
        }
    }
}

mod text {
    //! What a command does and how to undo it by hand, printed before it changes anything.

    use std::path::{Path, PathBuf};

    use wakeshot_core::settings::Guid;
    use wakeshot_windows::power::journal::RUN_ONCE_NAME;

    use super::cli::Cmd;
    use super::{COPY_PREFIX, PLAN_READ_MS, RUN_ONCE_KEY, TICK_MS};

    /// What a run may leave, for its undo lines.
    #[derive(Clone, Debug, Default, PartialEq, Eq)]
    pub struct Undo {
        pub original: Option<Guid>,
        pub copy: Option<String>,
        pub copy_active: bool,
        pub run_once: bool,
        pub journal: Option<PathBuf>,
        pub request: bool,
    }

    /// `copy`: the copy's GUID once it exists; before, the lines name it `<copy guid>`.
    pub fn undo_for(cmd: &Cmd, original: Option<Guid>, copy: Option<&str>, dir: &Path) -> Undo {
        let copy = copy.unwrap_or("<copy guid>").to_owned();
        let journal = dir.join("journal.json");
        match cmd {
            Cmd::Hold {
                request,
                active_copy,
            } => Undo {
                original,
                copy: active_copy.then_some(copy),
                copy_active: *active_copy,
                request: *request,
                ..Undo::default()
            },
            Cmd::FakeTray | Cmd::EndSession => Undo {
                original,
                copy: Some(copy),
                run_once: true,
                journal: Some(journal),
                ..Undo::default()
            },
            Cmd::Cycles { .. } => Undo {
                original,
                copy: Some(copy),
                copy_active: true,
                journal: Some(journal),
                ..Undo::default()
            },
            Cmd::Help | Cmd::SessionWatch | Cmd::MediumProbe | Cmd::Cleanup => Undo::default(),
        }
    }

    pub fn undo_lines(undo: &Undo) -> Vec<String> {
        let mut steps = Vec::new();
        if undo.copy_active {
            steps.push(match undo.original {
                Some(plan) => format!("    powercfg /setactive {plan}"),
                None => {
                    "    powercfg /setactive <your plan's guid, from powercfg /list>".to_owned()
                }
            });
        }
        if let Some(copy) = &undo.copy {
            steps.push(format!("    powercfg /delete {copy}"));
        }
        if undo.run_once {
            steps.push(format!(
                "    reg delete \"{RUN_ONCE_KEY}\" /v {RUN_ONCE_NAME} /f"
            ));
        }
        if let Some(journal) = &undo.journal {
            steps.push(format!("    del \"{}\"", journal.display()));
        }
        let mut lines = Vec::new();
        match steps.is_empty() {
            true => lines.push(
                "undo: nothing: this command writes no power setting, no journal and no RunOnce value".to_owned(),
            ),
            false => {
                lines.push(
                    "undo by hand once the run has ended, in this order (or run `power_check cleanup`):".to_owned(),
                );
                lines.extend(steps);
            }
        }
        if undo.request {
            lines.push(
                "  the power request ends with this process; `powercfg /requests` (elevated) lists it while it runs"
                    .to_owned(),
            );
        }
        lines
    }

    pub fn plan_lines(cmd: &Cmd, dir: &Path, guard: Option<&Path>, pid: u32) -> Vec<String> {
        let copy = format!("a copy of the active plan named \"{COPY_PREFIX}{pid}\"");
        let guard = guard.map_or_else(|| "<tray exe>".to_owned(), |exe| exe.display().to_string());
        let journal = dir.join("journal.json");
        let tick_s = TICK_MS / 1000;
        let read_s = PLAN_READ_MS / 1000;
        let lines: Vec<String> = match cmd {
            Cmd::Help => Vec::new(),
            Cmd::Hold {
                request,
                active_copy,
            } => {
                let mut lines = Vec::new();
                match *active_copy {
                    true => {
                        lines.push(format!(
                            "makes {copy}, writes lid DC 0 (Do nothing) and sleep DC 0 (Never) to it and makes it \
                             the active plan (PowerSetActiveScheme)"
                        ));
                        lines.push(
                            "at exit: the recorded plan active again, the copy deleted".to_owned(),
                        );
                    }
                    false => lines.push("writes no power setting".to_owned()),
                }
                lines.push(match *request {
                    true => "sets SYSTEM and EXECUTION power requests, retakes them after each resume, clears them \
                             at exit"
                        .to_owned(),
                    false => "holds no power request".to_owned(),
                });
                lines.push(format!(
                    "logs a tick every {tick_s} s (wall, unbiased, input idle), each lid, power, plan, suspend and \
                     resume event, and a GAP line when a tick comes late"
                ));
                lines
            }
            Cmd::FakeTray | Cmd::EndSession => {
                let mut lines = vec![
                    format!("makes {copy}; it is never the active plan"),
                    "through the executor: ReadPlan, then Apply lid AC 0, lid DC 0, sleep DC 0 on the copy \
                     (CopyAsActive: what is in force is only recorded)"
                        .to_owned(),
                    format!(
                        "journal {} (owner: this process); RunOnce {RUN_ONCE_NAME} = \"{guard}\" --restore; guard: \
                         {guard} --guard <pid> <start> <event>, under Explorer",
                        journal.display()
                    ),
                    format!(
                        "every {read_s} s: ReadPlan, the ownership check (a guard that ended is replaced first)"
                    ),
                ];
                if *cmd == Cmd::EndSession {
                    lines.push(
                        "a hidden top-level window: at WM_QUERYENDSESSION and WM_ENDSESSION, Restore at once (each \
                         step's time in the log), then the copy deleted"
                            .to_owned(),
                    );
                }
                lines.push(format!(
                    "Ctrl+C: Restore (Quit), then the copy deleted; End task or `taskkill /PID {pid} /F` leaves it \
                     to the guard"
                ));
                lines
            }
            Cmd::SessionWatch => vec![
                "a hidden top-level window registered for session notices (WTSRegisterSessionNotification); logs \
                 lock, unlock, console and end-session messages; changes nothing"
                    .to_owned(),
            ],
            Cmd::Cycles { n, pause_ms } => vec![
                format!("makes {copy} and makes it the active plan"),
                format!(
                    "{n} times through the executor: Apply lid AC 0, lid DC 0, sleep DC 0, then Restore, {pause_ms} \
                     ms apart; each PowerSetActiveScheme time logged"
                ),
                format!(
                    "journal {}; no RunOnce value, no guard",
                    journal.display()
                ),
                "at the end: the recorded plan active again, the copy deleted".to_owned(),
            ],
            Cmd::MediumProbe => vec![format!(
                "prints the integrity level; creates and takes Global\\Wakeshot.Probe.{pid}; changes nothing else"
            )],
            Cmd::Cleanup => vec![
                "makes the recorded plan active again when the active plan is a test copy whose process is gone"
                    .to_owned(),
                "runs the restore on each gone run's journal, removes the RunOnce value a gone run set, and \
                 deletes every Wakeshot test copy whose process is gone; never a live run's"
                    .to_owned(),
            ],
        };
        lines.into_iter().map(|line| format!("  {line}")).collect()
    }
}

mod ticks {
    //! The tick and gap lines of the lid spike (`spikes/lid-sleep/src/ticks.rs`), ported: a gap is named by its
    //! start (research, "Lid and sleep spike", morning results).

    use wakeshot_core::time::Now;
    use wakeshot_windows::WinError;

    /// A tick at least this late is a gap.
    const LATE_MS: i64 = 2_000;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Gap {
        /// Unbiased time stopped too: the PC was suspended (Modern Standby's suspended part, S4).
        Slept,
        /// Unbiased time ran on but this process did not.
        Paused,
    }

    pub fn classify(interval_ms: i64, wall_ms: i64, unbiased_ms: i64) -> Option<Gap> {
        let late = wall_ms - interval_ms;
        if late < LATE_MS {
            return None;
        }
        match wall_ms - unbiased_ms > late / 2 {
            true => Some(Gap::Slept),
            false => Some(Gap::Paused),
        }
    }

    pub struct Ticker {
        interval_ms: i64,
        last: Now,
        last_stamp: String,
        count: u64,
        gaps: u64,
    }

    impl Ticker {
        pub fn new(now: Now, stamp: String, interval_ms: i64) -> Ticker {
            Ticker {
                interval_ms,
                last: now,
                last_stamp: stamp,
                count: 0,
                gaps: 0,
            }
        }

        pub fn count(&self) -> u64 {
            self.count
        }

        pub fn gap_count(&self) -> u64 {
            self.gaps
        }

        /// The tick line, and a gap line when this tick came late.
        pub fn step(
            &mut self,
            now: Now,
            stamp: String,
            idle: Result<u64, WinError>,
        ) -> (String, Option<String>) {
            self.count += 1;
            let wall = now.wall_ms - self.last.wall_ms;
            let unbiased = now.mono_ms.cast_signed() - self.last.mono_ms.cast_signed();
            let idle = match idle {
                Ok(ms) => format!("{:.1} s", ms as f64 / 1000.0),
                Err(e) => format!("unread ({} code {})", e.call, e.code),
            };
            let line = format!(
                "tick {}: wall {stamp} (step {wall} ms), unbiased {:.3} s (step {unbiased} ms), input idle {idle}",
                self.count,
                now.mono_ms as f64 / 1000.0
            );
            let gap = classify(self.interval_ms, wall, unbiased).map(|kind| {
                self.gaps += 1;
                let what = match kind {
                    Gap::Slept => "slept (unbiased time stopped: the PC was suspended)",
                    Gap::Paused => "paused (unbiased time ran on: this process did not run)",
                };
                format!(
                    "GAP {what}: began {} (the tick before), {:.1} s of wall time, {:.1} s of unbiased time",
                    self.last_stamp,
                    wall as f64 / 1000.0,
                    unbiased as f64 / 1000.0
                )
            });
            self.last = now;
            self.last_stamp = stamp;
            (line, gap)
        }
    }
}

mod tidy {
    //! What `cleanup` does, decided from what it sees, so the rules are tested without a plan or a process.

    use std::collections::BTreeSet;
    use std::path::PathBuf;

    use wakeshot_core::process::{Alive, Lookup};
    use wakeshot_core::settings::{Guid, JournalOwner};
    use wakeshot_windows::power::journal::RUN_ONCE_NAME;

    use super::Run;

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum JournalSeen {
        Missing,
        Owner(JournalOwner),
        Unreadable(String),
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct RunSeen {
        pub file: PathBuf,
        pub run: Run,
        pub alive: Alive,
        /// The journal in the run's data folder.
        pub journal: JournalSeen,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct Look {
        pub active: Guid,
        /// The active plan is a "Wakeshot test <pid>" copy: that pid, and what runs under it.
        pub active_copy: Option<(u32, Lookup)>,
        /// The plan recorded at the first start.
        pub original: Option<Guid>,
        pub runs: Vec<RunSeen>,
        pub bad_records: Vec<(PathBuf, String)>,
        /// The RunOnce value now.
        pub run_once: Option<String>,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum Step {
        Note(String),
        SetActive(Guid),
        /// The restore routine on a gone run's journal. `run_once`: its clean-up may clear the RunOnce value too.
        Restore {
            file: PathBuf,
            dir: PathBuf,
            owner: JournalOwner,
            run_once: bool,
        },
        /// Removed only while it still holds this command.
        RemoveRunOnce(String),
        /// The run record goes, unless its restore failed.
        Forget(PathBuf),
        Sweep,
    }

    pub fn plan(look: &Look) -> Vec<Step> {
        let mut steps = Vec::new();
        let active = look.active;
        match (look.active_copy, look.original) {
            (Some((_, Lookup::NotRunning)), Some(original)) if original != active => {
                steps.push(Step::SetActive(original));
            }
            (Some((pid, Lookup::NotRunning)), _) => steps.push(Step::Note(format!(
                "the active plan {active} is the copy of gone pid {pid} and no other plan is recorded: make your \
                 plan active by hand (powercfg /list, then powercfg /setactive <guid>)"
            ))),
            (Some((pid, Lookup::Running(_))), _) => steps.push(Step::Note(format!(
                "the active plan {active} is the copy of pid {pid}, which still runs: left active (a reused pid \
                 keeps it too: then make your plan active by hand, powercfg /setactive <guid>)"
            ))),
            (Some((pid, Lookup::Unknown)), _) => steps.push(Step::Note(format!(
                "the active plan {active} is the copy of pid {pid}, whose liveness is unknown: left active"
            ))),
            (None, Some(original)) if original != active => steps.push(Step::Note(format!(
                "the active plan {active} is not the recorded {original}, and not a test copy: left as it is"
            ))),
            (None, _) => {}
        }
        for (file, why) in &look.bad_records {
            steps.push(Step::Note(format!(
                "the run record {} does not read ({why}): left",
                file.display()
            )));
        }
        // A value a live run set is never removed, also not by a gone run's restore clean-up.
        let live_run_once: BTreeSet<&str> = look
            .runs
            .iter()
            .filter(|r| r.alive != Alive::Gone)
            .filter_map(|r| r.run.run_once.as_deref())
            .collect();
        let mut run_once_removed = false;
        for r in &look.runs {
            let label = format!(
                "run pid {} ({}, data folder {}{})",
                r.run.pid,
                r.run.command,
                r.run.dir.display(),
                r.run
                    .copy
                    .map_or_else(String::new, |copy| format!(", copy {copy}"))
            );
            match r.alive {
                Alive::Yes => {
                    steps.push(Step::Note(format!("{label} still runs: left alone")));
                    continue;
                }
                Alive::Unknown => {
                    steps.push(Step::Note(format!(
                        "{label}: its liveness is unknown: left alone"
                    )));
                    continue;
                }
                Alive::Gone => {}
            }
            let mut forget = true;
            match &r.journal {
                JournalSeen::Missing => {}
                JournalSeen::Owner(owner)
                    if owner.pid == r.run.pid && owner.proc_start == r.run.start =>
                {
                    steps.push(Step::Restore {
                        file: r.file.clone(),
                        dir: r.run.dir.clone(),
                        owner: *owner,
                        run_once: r.run.run_once.is_some() && live_run_once.is_empty(),
                    });
                }
                JournalSeen::Owner(owner) => steps.push(Step::Note(format!(
                    "{label}: the journal in its folder names pid {}, another process: left",
                    owner.pid
                ))),
                JournalSeen::Unreadable(why) => {
                    steps.push(Step::Note(format!(
                        "{label}: its journal does not read ({why}): left, and so is the record"
                    )));
                    forget = false;
                }
            }
            if let Some(command) = &r.run.run_once
                && look.run_once.as_deref() == Some(command.as_str())
                && !live_run_once.contains(command.as_str())
                && !run_once_removed
            {
                steps.push(Step::RemoveRunOnce(command.clone()));
                run_once_removed = true;
            }
            if forget {
                steps.push(Step::Forget(r.file.clone()));
            }
        }
        if let Some(value) = &look.run_once
            && !run_once_removed
        {
            steps.push(Step::Note(format!(
                "RunOnce {RUN_ONCE_NAME} = {value}: not a gone run's of this harness: left"
            )));
        }
        steps.push(Step::Sweep);
        steps
    }

    pub fn step_text(step: &Step) -> String {
        match step {
            Step::Note(text) => format!("note: {text}"),
            Step::SetActive(plan) => {
                format!("make {plan} the active plan (powercfg /setactive {plan})")
            }
            Step::Restore {
                dir,
                owner,
                run_once,
                ..
            } => format!(
                "restore the journal in {} (owner pid {}){}",
                dir.display(),
                owner.pid,
                if *run_once {
                    ", its clean-up clears RunOnce"
                } else {
                    ", RunOnce untouched"
                }
            ),
            Step::RemoveRunOnce(command) => {
                format!("remove the RunOnce value {RUN_ONCE_NAME} = {command}")
            }
            Step::Forget(file) => format!("remove the run record {}", file.display()),
            Step::Sweep => {
                "delete every Wakeshot test copy whose process is gone (TestPlan::sweep)".to_owned()
            }
        }
    }
}

mod values {
    //! The user's values, read at the start and the exit of every command, so a run shows it changed nothing of
    //! theirs.

    use wakeshot_core::command::Failure;
    use wakeshot_core::ownership::io::PlanIo;
    use wakeshot_core::settings::{Guid, PlanValues};
    use wakeshot_windows::power::plan::{ValuesOf, Win32Power};

    use super::{fail_text, name_text, test_copy_pid, values_text};

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct Seen {
        pub active: Result<(Guid, String), Failure>,
        /// The recorded plan, or the active one when none is recorded.
        pub user: Option<(Guid, String, Result<PlanValues, Failure>)>,
        pub plans: Result<Vec<(Guid, String)>, Failure>,
    }

    pub fn read(power: &mut Win32Power, original: Option<Guid>) -> Seen {
        let active = power.active_plan();
        let plans = power
            .plans()
            .map(|list| list.into_iter().map(|g| (g, name_text(g))).collect());
        let user = original
            .or(active.ok())
            .map(|g| (g, name_text(g), ValuesOf::values_of(power, g)));
        Seen {
            active: active.map(|g| (g, name_text(g))),
            user,
            plans,
        }
    }

    pub fn lines(seen: &Seen) -> Vec<String> {
        let mut lines = Vec::new();
        lines.push(match &seen.active {
            Ok((plan, name)) => format!("  active plan {plan} \"{name}\""),
            Err(f) => format!("  active plan: the read failed: {}", fail_text(*f)),
        });
        lines.push(match &seen.user {
            Some((plan, name, Ok(v))) => {
                format!("  user's plan {plan} \"{name}\": {}", values_text(v))
            }
            Some((plan, name, Err(f))) => format!(
                "  user's plan {plan} \"{name}\": the read failed: {}",
                fail_text(*f)
            ),
            None => "  user's plan: unknown".to_owned(),
        });
        match &seen.plans {
            Ok(list) => {
                let all: Vec<String> = list
                    .iter()
                    .map(|(plan, name)| format!("\"{name}\" {plan}"))
                    .collect();
                lines.push(format!("  plans: {}", all.join("; ")));
                let copies: Vec<String> = list
                    .iter()
                    .filter(|(_, name)| test_copy_pid(name).is_some())
                    .map(|(plan, name)| format!("\"{name}\" {plan}"))
                    .collect();
                lines.push(match copies.is_empty() {
                    true => "  Wakeshot test plans: none".to_owned(),
                    false => format!("  Wakeshot test plans: {}", copies.join("; ")),
                });
            }
            Err(f) => lines.push(format!("  plans: the list failed: {}", fail_text(*f))),
        }
        lines
    }

    pub fn differences(start: &Seen, end: &Seen) -> Vec<String> {
        let mut changed = Vec::new();
        let (a, b) = (lines(start), lines(end));
        for (was, now) in a.iter().zip(&b) {
            if was != now {
                changed.push(format!("was `{}`, now `{}`", was.trim(), now.trim()));
            }
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::cli::{Args, Cmd, DEFAULT_PAUSE_MS, parse};
    use super::text::{Undo, plan_lines, undo_for, undo_lines};
    use super::ticks::{Gap, Ticker, classify};
    use super::tidy::{JournalSeen, Look, RunSeen, Step, plan};
    use super::values::{Seen, differences, lines};
    use super::*;

    const BALANCED: Guid = Guid(0x381b4222_f694_41f0_9685_ff5bb260df2e);
    const COPY: Guid = Guid(0x42ffeeaa_0000_4000_8000_000000000001);
    const OTHER_COPY: Guid = Guid(0x42ffeeaa_0000_4000_8000_000000000002);

    fn args(list: &[&str]) -> Result<Args, String> {
        parse(&list.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>())
    }

    fn cmd(list: &[&str]) -> Cmd {
        args(list).unwrap().cmd
    }

    #[test]
    fn every_command_parses() {
        assert_eq!(
            cmd(&["hold"]),
            Cmd::Hold {
                request: true,
                active_copy: false
            }
        );
        assert_eq!(
            cmd(&["hold", "--no-request", "--active-copy"]),
            Cmd::Hold {
                request: false,
                active_copy: true
            }
        );
        assert_eq!(cmd(&["fake-tray"]), Cmd::FakeTray);
        assert_eq!(cmd(&["end-session"]), Cmd::EndSession);
        assert_eq!(cmd(&["session-watch"]), Cmd::SessionWatch);
        assert_eq!(
            cmd(&["cycles", "50"]),
            Cmd::Cycles {
                n: 50,
                pause_ms: DEFAULT_PAUSE_MS
            }
        );
        assert_eq!(
            cmd(&["cycles", "3", "--pause", "0"]),
            Cmd::Cycles { n: 3, pause_ms: 0 }
        );
        assert_eq!(cmd(&["medium-probe"]), Cmd::MediumProbe);
        assert_eq!(cmd(&["cleanup"]), Cmd::Cleanup);
        for help in [["--help"], ["help"], ["-h"], ["/?"]] {
            assert_eq!(cmd(&help), Cmd::Help);
        }
        assert_eq!(cmd(&["cleanup", "--help"]), Cmd::Help);
    }

    #[test]
    fn options_land_in_their_fields() {
        let a = args(&[
            "fake-tray",
            "--dir",
            r"C:\x",
            "--home",
            r"C:\h",
            "--guard",
            r"C:\t\wakeshot-tray.exe",
            "--for",
            "30",
            "--plan-only",
        ])
        .unwrap();
        assert_eq!(a.cmd, Cmd::FakeTray);
        assert_eq!(a.dir.as_deref(), Some(Path::new(r"C:\x")));
        assert_eq!(a.home.as_deref(), Some(Path::new(r"C:\h")));
        assert_eq!(
            a.guard.as_deref(),
            Some(Path::new(r"C:\t\wakeshot-tray.exe"))
        );
        assert_eq!(a.for_s, Some(30));
        assert!(a.plan_only);
        let plain = args(&["session-watch"]).unwrap();
        assert_eq!(
            (
                plain.dir,
                plain.home,
                plain.guard,
                plain.for_s,
                plain.plan_only
            ),
            (None, None, None, None, false)
        );
    }

    #[test]
    fn bad_command_lines_fail_before_anything_runs() {
        for (line, part) in [
            (&[][..], "no command"),
            (&["nap"][..], "unknown command"),
            (&["cycles"][..], "needs a count"),
            (&["cycles", "0"][..], "from 1 to 1000"),
            (&["cycles", "1001"][..], "from 1 to 1000"),
            (&["cycles", "x"][..], "from 1 to 1000"),
            (&["cycles", "2", "--pause", "60001"][..], "--pause"),
            (&["hold", "--for"][..], "needs a value"),
            (&["hold", "--for", "0"][..], "--for"),
            (&["hold", "--guard", "x"][..], "does not take --guard"),
            (
                &["fake-tray", "--active-copy"][..],
                "does not take --active-copy",
            ),
            (&["cycles", "2", "--for", "5"][..], "does not take --for"),
            (&["medium-probe", "--for", "5"][..], "does not take --for"),
            (&["cleanup", "--no-request"][..], "does not take"),
            (&["hold", "--no-request", "--no-request"][..], "twice"),
            (&["hold", "extra"][..], "unexpected argument"),
        ] {
            let error = args(line).unwrap_err();
            assert!(error.contains(part), "{line:?}: {error}");
        }
    }

    #[test]
    fn usage_names_every_command_and_option() {
        for word in [
            "hold",
            "--no-request",
            "--active-copy",
            "fake-tray",
            "end-session",
            "session-watch",
            "cycles <n>",
            "--pause",
            "medium-probe",
            "cleanup",
            "--dir",
            "--home",
            "--plan-only",
            "--for",
            "--guard",
        ] {
            assert!(cli::USAGE.contains(word), "{word}");
        }
    }

    fn undo_text(cmd: &Cmd, copy: Option<&str>) -> String {
        undo_lines(&undo_for(cmd, Some(BALANCED), copy, Path::new(r"C:\d"))).join("\n")
    }

    /// The dry pass the card asks for: every command's plan and undo lines, as `--plan-only` prints them.
    #[test]
    fn plan_only_text_for_every_command() {
        let all = [
            Cmd::Hold {
                request: true,
                active_copy: false,
            },
            Cmd::Hold {
                request: false,
                active_copy: true,
            },
            Cmd::FakeTray,
            Cmd::EndSession,
            Cmd::SessionWatch,
            Cmd::Cycles {
                n: 50,
                pause_ms: 1000,
            },
            Cmd::MediumProbe,
            Cmd::Cleanup,
        ];
        for cmd in &all {
            let plan = plan_lines(cmd, Path::new(r"C:\d"), None, 4242);
            assert!(!plan.is_empty(), "{cmd:?}");
            assert!(plan.iter().all(|l| l.starts_with("  ")), "{cmd:?}");
        }

        let hold = undo_text(
            &Cmd::Hold {
                request: true,
                active_copy: false,
            },
            None,
        );
        assert!(hold.contains("undo: nothing"), "{hold}");
        assert!(hold.contains("powercfg /requests"), "{hold}");

        let copy = COPY.to_string();
        let active = undo_text(
            &Cmd::Hold {
                request: false,
                active_copy: true,
            },
            Some(&copy),
        );
        let set = format!("powercfg /setactive {BALANCED}");
        let delete = format!("powercfg /delete {COPY}");
        assert!(
            active.contains(&set) && active.contains(&delete),
            "{active}"
        );
        assert!(
            active.find(&set) < active.find(&delete),
            "back first, then delete"
        );
        assert!(!active.contains("reg delete") && !active.contains("journal.json"));

        for tray in [Cmd::FakeTray, Cmd::EndSession] {
            let text = undo_text(&tray, None);
            assert!(text.contains("powercfg /delete <copy guid>"), "{text}");
            assert!(!text.contains("/setactive"), "never active: {text}");
            assert!(
                text.contains(r#"reg delete "HKCU\Software\Microsoft\Windows\CurrentVersion\RunOnce" /v WakeshotRestore /f"#),
                "{text}"
            );
            assert!(text.contains(r#"del "C:\d\journal.json""#), "{text}");
        }

        let cycles = undo_text(&Cmd::Cycles { n: 2, pause_ms: 0 }, Some(&copy));
        assert!(
            cycles.contains(&set) && cycles.contains(&delete),
            "{cycles}"
        );
        assert!(!cycles.contains("reg delete"), "{cycles}");

        for quiet in [Cmd::SessionWatch, Cmd::MediumProbe, Cmd::Cleanup] {
            assert!(
                undo_text(&quiet, None).starts_with("undo: nothing"),
                "{quiet:?}"
            );
        }

        let unknown = undo_lines(&Undo {
            copy_active: true,
            ..Undo::default()
        });
        assert!(unknown[1].contains("<your plan's guid"), "{unknown:?}");
    }

    #[test]
    fn plan_lines_name_what_changes() {
        let dir = Path::new(r"C:\d");
        let text =
            |cmd: &Cmd| plan_lines(cmd, dir, Some(Path::new(r"C:\t\tray.exe")), 77).join("\n");
        let tray = text(&Cmd::FakeTray);
        assert!(tray.contains("\"Wakeshot test 77\"") && tray.contains("never the active plan"));
        assert!(tray.contains(r#""C:\t\tray.exe" --restore"#), "{tray}");
        assert!(!tray.contains("WM_QUERYENDSESSION"));
        assert!(text(&Cmd::EndSession).contains("WM_QUERYENDSESSION and WM_ENDSESSION"));
        let hold = text(&Cmd::Hold {
            request: true,
            active_copy: false,
        });
        assert!(hold.contains("writes no power setting") && hold.contains("SYSTEM and EXECUTION"));
        assert!(text(&Cmd::MediumProbe).contains(r"Global\Wakeshot.Probe.77"));
    }

    #[test]
    fn gaps_are_classified_like_the_spike() {
        assert_eq!(classify(5000, 5000, 5000), None);
        assert_eq!(classify(5000, 6999, 6999), None);
        assert_eq!(classify(5000, 571_000, 4_900), Some(Gap::Slept));
        assert_eq!(classify(5000, 61_000, 60_900), Some(Gap::Paused));
    }

    fn at(wall_ms: i64, mono_ms: u64) -> Now {
        Now { wall_ms, mono_ms }
    }

    #[test]
    fn ticker_names_a_gap_by_its_start() {
        let mut t = Ticker::new(at(1_000_000, 50_000), "t0".into(), 5_000);
        let (line, gap) = t.step(at(1_005_010, 55_010), "t1".into(), Ok(12_300));
        assert_eq!(
            line,
            "tick 1: wall t1 (step 5010 ms), unbiased 55.010 s (step 5010 ms), input idle 12.3 s"
        );
        assert_eq!(gap, None);
        let (_, gap) = t.step(at(1_576_010, 59_910), "t2".into(), Ok(0));
        let gap = gap.unwrap();
        assert!(
            gap.starts_with("GAP slept") && gap.contains("began t1"),
            "{gap}"
        );
        assert!(
            gap.contains("571.0 s of wall time") && gap.contains("4.9 s of unbiased"),
            "{gap}"
        );
        let (line, _) = t.step(
            at(1_581_010, 64_910),
            "t3".into(),
            Err(WinError {
                call: "GetLastInputInfo",
                code: 5,
            }),
        );
        assert!(
            line.ends_with("input idle unread (GetLastInputInfo code 5)"),
            "{line}"
        );
        assert_eq!((t.count(), t.gap_count()), (3, 1));
    }

    fn run(pid: u32, command: &str, run_once: Option<&str>) -> Run {
        Run {
            pid,
            start: FileTime(1000 + u64::from(pid)),
            command: command.to_owned(),
            dir: PathBuf::from(format!(r"C:\d\{command}")),
            run_once: run_once.map(str::to_owned),
            copy: Some(COPY),
        }
    }

    #[test]
    fn run_records_round_trip() {
        let r = run(
            12,
            "fake-tray",
            Some(r#""C:\t\wakeshot-tray.exe" --restore"#),
        );
        assert_eq!(Run::parse(&r.to_json()), Ok(r));
        let bare = Run {
            run_once: None,
            copy: None,
            ..run(13, "cycles", None)
        };
        assert_eq!(Run::parse(&bare.to_json()), Ok(bare));
        assert!(Run::parse("{}").is_err());
        assert!(Run::parse(r#"{"pid":1,"start":"x","command":"c","dir":"d"}"#).is_err());
        assert!(Run::parse("not json").is_err());
    }

    #[test]
    fn home_records_the_first_plan_only() {
        let root =
            std::env::temp_dir().join(format!("wakeshot-harness-test-{}", std::process::id()));
        let home = Home::at(root.clone());
        assert_eq!(home.original(), None);
        assert_eq!(home.keep_original(BALANCED), Ok(true));
        assert_eq!(home.keep_original(COPY), Ok(false));
        assert_eq!(home.original(), Some(BALANCED));
        let r = run(14, "hold", None);
        let path = home.save(&r).unwrap();
        assert_eq!(home.runs(), vec![(path.clone(), Ok(r))]);
        assert_eq!(home.data_dir("hold"), root.join("hold"));
        home.forget_original().unwrap();
        assert_eq!(home.original(), None);
        std::fs::remove_dir_all(&root).unwrap();
    }

    fn look() -> Look {
        Look {
            active: BALANCED,
            active_copy: None,
            original: Some(BALANCED),
            runs: Vec::new(),
            bad_records: Vec::new(),
            run_once: None,
        }
    }

    fn seen(run: Run, alive: Alive, journal: JournalSeen) -> RunSeen {
        RunSeen {
            file: PathBuf::from(format!(r"C:\h\runs\{}.json", run.pid)),
            run,
            alive,
            journal,
        }
    }

    fn owner_of(run: &Run) -> JournalOwner {
        JournalOwner {
            pid: run.pid,
            proc_start: run.start,
            session: 1,
        }
    }

    #[test]
    fn cleanup_with_nothing_to_clean_only_sweeps() {
        assert_eq!(plan(&look()), [Step::Sweep]);
    }

    #[test]
    fn cleanup_restores_a_gone_tray_and_its_run_once() {
        let command = r#""C:\t\wakeshot-tray.exe" --restore"#;
        let gone = run(20, "fake-tray", Some(command));
        let owner = owner_of(&gone);
        let l = Look {
            runs: vec![seen(gone.clone(), Alive::Gone, JournalSeen::Owner(owner))],
            run_once: Some(command.to_owned()),
            ..look()
        };
        assert_eq!(
            plan(&l),
            [
                Step::Restore {
                    file: PathBuf::from(r"C:\h\runs\20.json"),
                    dir: gone.dir.clone(),
                    owner,
                    run_once: true,
                },
                Step::RemoveRunOnce(command.to_owned()),
                Step::Forget(PathBuf::from(r"C:\h\runs\20.json")),
                Step::Sweep,
            ]
        );
    }

    #[test]
    fn cleanup_puts_the_plan_back_only_from_a_gone_copy() {
        let gone = Look {
            active: COPY,
            active_copy: Some((30, Lookup::NotRunning)),
            ..look()
        };
        assert_eq!(plan(&gone), [Step::SetActive(BALANCED), Step::Sweep]);

        for alive in [Lookup::Running(FileTime(5)), Lookup::Unknown] {
            let live = Look {
                active_copy: Some((30, alive)),
                ..gone.clone()
            };
            let steps = plan(&live);
            assert!(
                matches!(&steps[0], Step::Note(t) if t.contains("left active")),
                "{steps:?}"
            );
            assert!(!steps.iter().any(|s| matches!(s, Step::SetActive(_))));
        }

        let unrecorded = Look {
            original: None,
            ..gone.clone()
        };
        assert!(matches!(&plan(&unrecorded)[0], Step::Note(t) if t.contains("by hand")));

        let users_own = Look {
            active: OTHER_COPY,
            ..look()
        };
        let steps = plan(&users_own);
        assert!(
            matches!(&steps[0], Step::Note(t) if t.contains("left as it is")),
            "{steps:?}"
        );
        assert_eq!(steps.len(), 2);
    }

    #[test]
    fn cleanup_never_touches_a_live_run() {
        let command = r#""C:\t\wakeshot-tray.exe" --restore"#;
        let live = run(40, "fake-tray", Some(command));
        let gone = run(41, "end-session", Some(command));
        let l = Look {
            runs: vec![
                seen(
                    live.clone(),
                    Alive::Yes,
                    JournalSeen::Owner(owner_of(&live)),
                ),
                seen(
                    gone.clone(),
                    Alive::Gone,
                    JournalSeen::Owner(owner_of(&gone)),
                ),
                seen(
                    run(42, "cycles", None),
                    Alive::Unknown,
                    JournalSeen::Missing,
                ),
            ],
            run_once: Some(command.to_owned()),
            ..look()
        };
        let steps = plan(&l);
        assert!(
            matches!(&steps[0], Step::Note(t) if t.contains("pid 40") && t.contains("still runs"))
        );
        // The gone run's restore must not clear the value the live run holds.
        assert!(steps.contains(&Step::Restore {
            file: PathBuf::from(r"C:\h\runs\41.json"),
            dir: gone.dir.clone(),
            owner: owner_of(&gone),
            run_once: false,
        }));
        assert!(!steps.iter().any(|s| matches!(s, Step::RemoveRunOnce(_))));
        assert!(!steps.contains(&Step::Forget(PathBuf::from(r"C:\h\runs\40.json"))));
        assert!(!steps.contains(&Step::Forget(PathBuf::from(r"C:\h\runs\42.json"))));
        assert!(
            steps
                .iter()
                .any(|s| matches!(s, Step::Note(t) if t.contains("unknown")))
        );
    }

    #[test]
    fn cleanup_leaves_what_is_not_a_gone_runs() {
        let gone = run(50, "cycles", None);
        let stranger = JournalOwner {
            pid: 999,
            ..owner_of(&gone)
        };
        let l = Look {
            runs: vec![
                seen(gone.clone(), Alive::Gone, JournalSeen::Owner(stranger)),
                seen(
                    run(51, "end-session", None),
                    Alive::Gone,
                    JournalSeen::Unreadable("it does not parse".into()),
                ),
            ],
            bad_records: vec![(PathBuf::from(r"C:\h\runs\x.json"), "no pid".into())],
            run_once: Some("someone else".into()),
            ..look()
        };
        let steps = plan(&l);
        let notes: Vec<&str> = steps
            .iter()
            .filter_map(|s| match s {
                Step::Note(t) => Some(t.as_str()),
                _ => None,
            })
            .collect();
        assert!(notes.iter().any(|t| t.contains("does not read (no pid)")));
        assert!(notes.iter().any(|t| t.contains("names pid 999")));
        assert!(
            notes
                .iter()
                .any(|t| t.contains("left, and so is the record"))
        );
        assert!(notes.iter().any(|t| t.contains("someone else")));
        assert!(
            !steps
                .iter()
                .any(|s| matches!(s, Step::Restore { .. } | Step::RemoveRunOnce(_)))
        );
        assert!(steps.contains(&Step::Forget(PathBuf::from(r"C:\h\runs\50.json"))));
        assert!(!steps.contains(&Step::Forget(PathBuf::from(r"C:\h\runs\51.json"))));
        assert_eq!(steps.last(), Some(&Step::Sweep));
    }

    #[test]
    fn test_copy_names_need_digits() {
        assert_eq!(test_copy_pid("Wakeshot test 1234"), Some(1234));
        assert_eq!(test_copy_pid("Wakeshot test e2e"), None);
        assert_eq!(test_copy_pid("Wakeshot test "), None);
        assert_eq!(test_copy_pid("Balanced"), None);
    }

    fn haiks() -> PlanValues {
        PlanValues {
            active: BALANCED,
            lid_ac: 1,
            lid_dc: 1,
            sleep_ac: 10_800,
            sleep_dc: 14_400,
            saver_threshold_dc: 30,
        }
    }

    #[test]
    fn values_print_and_compare() {
        let start = Seen {
            active: Ok((BALANCED, "Balanced".into())),
            user: Some((BALANCED, "Balanced".into(), Ok(haiks()))),
            plans: Ok(vec![(BALANCED, "Balanced".into())]),
        };
        let text = lines(&start).join("\n");
        assert!(
            text.contains(&format!("active plan {BALANCED} \"Balanced\"")),
            "{text}"
        );
        assert!(
            text.contains(
                "lid AC 1 (Sleep), lid DC 1 (Sleep), sleep AC 10800 s, sleep DC 14400 s, energy saver DC 30%"
            ),
            "{text}"
        );
        assert!(text.contains("Wakeshot test plans: none"), "{text}");
        assert!(differences(&start, &start.clone()).is_empty());

        let left = Seen {
            active: Ok((COPY, "Wakeshot test 9".into())),
            plans: Ok(vec![
                (BALANCED, "Balanced".into()),
                (COPY, "Wakeshot test 9".into()),
            ]),
            ..start.clone()
        };
        let changed = differences(&start, &left);
        assert_eq!(changed.len(), 3, "{changed:?}");
        assert!(changed[2].contains("Wakeshot test 9"), "{changed:?}");
    }

    #[test]
    fn apply_writes_only_keys_off_target() {
        assert_eq!(
            writes_for(&haiks()),
            [
                (SettingKey::LidAc, 0),
                (SettingKey::LidDc, 0),
                (SettingKey::SleepDc, 0)
            ]
        );
        let half = PlanValues {
            lid_dc: 0,
            ..haiks()
        };
        assert_eq!(
            writes_for(&half),
            [(SettingKey::LidAc, 0), (SettingKey::SleepDc, 0)]
        );
        assert_eq!(
            writes_text(&[]),
            "nothing to write: the copy has the targets already"
        );
    }

    #[test]
    fn end_session_flags_and_session_codes_read_as_text() {
        assert_eq!(end_flags(LPARAM(0)), "lparam 0x0: shutdown or restart");
        assert_eq!(
            end_flags(LPARAM(0x8000_0000)),
            "lparam 0x80000000: sign-out"
        );
        assert_eq!(
            end_flags(LPARAM(0x4000_0001)),
            "lparam 0x40000001: Restart Manager close, forced"
        );
        assert_eq!(
            message_text(WM_WTSSESSION_CHANGE, WPARAM(7), LPARAM(1)),
            "WM_WTSSESSION_CHANGE 7 (WTS_SESSION_LOCK), session 1"
        );
        assert_eq!(
            message_text(WM_WTSSESSION_CHANGE, WPARAM(42), LPARAM(1)),
            "WM_WTSSESSION_CHANGE 42 (unknown code), session 1"
        );
        assert!(message_text(WM_ENDSESSION, WPARAM(1), LPARAM(0)).contains("ending true"));
    }

    #[test]
    fn events_read_as_text() {
        assert_eq!(describe(&Event::Lid { closed: true }), "lid closed");
        assert_eq!(
            describe(&Event::Power {
                ac: false,
                battery_pct: Some(87)
            }),
            "power: on battery, battery 87%"
        );
        assert_eq!(
            describe(&Event::Resumed),
            "resumed (PBT_APMRESUMEAUTOMATIC)"
        );
        let entry = JournalEntry {
            plan: COPY,
            key: SettingKey::LidDc,
            original: 1,
            written: 0,
            restored: false,
        };
        let applied = describe(&Event::Done(Outcome::Applied {
            plan: COPY,
            written: vec![SettingKey::LidDc],
            result: Ok(()),
            journal: vec![entry],
        }));
        assert_eq!(
            applied,
            format!("apply on {COPY}: wrote lidDc; ok; journal holds lidDc 1 to 0 on {COPY}")
        );
        let restored = describe(&Event::Done(Outcome::Restored {
            plan: None,
            why: RestoreWhy::Quit,
            result: Ok(()),
            journal: Vec::new(),
        }));
        assert_eq!(restored, "restore (Quit): ok; journal holds nothing");
        assert_eq!(restored_of(&[Event::Tick]), None);
    }

    #[test]
    fn numbers_read_as_text() {
        assert_eq!(level_name(0x2000), "Medium");
        assert_eq!(level_name(0x3000), "High");
        assert_eq!(elevation_name(3), "Limited");
        assert_eq!(sleep_text(0), "Never");
        assert_eq!(lid_text(0), "Do nothing");
        assert_eq!(ms_list(&[]), "not called");
        assert_eq!(
            stats_text(&[3.0, 1.0, 2.0]),
            "3 calls, min 1.0 ms, median 2.0 ms, max 3.0 ms"
        );
    }

    /// Real, read only: the token and the plan names read on any Windows PC.
    #[test]
    fn integrity_and_plan_names_read() {
        let (rid, _) = integrity().unwrap();
        assert!(rid >= 0x1000, "{rid:#x}");
        let mut power = Win32Power::new(Arc::new(NoLog));
        let active = power.active_plan().unwrap();
        assert!(!name_text(active).is_empty());
    }
}
