# Architecture

Detailed design, 2026-11-03 (card `p1-detailed-design`; review fixes by `p1-design-review-fixes` and
`p1-design-verify-fixes`; sections 2 and 3 matched to the code by `p1-contracts-split` and `p1-close-contracts`).
Haik signs it off before any product code. Decisions: `product.md`. Facts: `research.md`, cited by section name.
"Not tested" marks a point that no test on the test laptop backs yet; the phase named next to it tests it. Haik's
answers to the eight design questions (2026-11-03) are written in where they apply; the questions stay in
`phases/01-design-skeleton.md`.

## 1. Shape

```
inputs                          core                              outputs
agent files (Claude, ...)  ┐                                     ┌ power requests
hold files (CLI, scripts)  │                                     │ power settings (lid action, battery sleep)
menu and CLI choices       ├─► Event ─► Core::handle ─► Command ─┤ lock, hibernate
lid, AC, battery, plan,    │           (pure; fake clock         │ tray icon, tooltip, menu
suspend, resume, session   ┘            in tests)                └ notices, sounds, files, log
```

- The core takes `(Now, Event)` and returns `Vec<Command>`. It has no OS code, no I/O, no clock reads and no
  `unsafe` (`#![forbid(unsafe_code)]`), so it builds and tests on any OS.
- Adapters in `wakeshot-windows` turn OS signals into events and run commands. Each command's result comes back as
  an event, so the core picks every fallback.

## 2. Crates and modules

One Cargo workspace, edition 2024. `spikes/` stays outside it (`exclude = ["spikes"]`; without it, building a spike
fails because it sits under the workspace root).

| Crate | Kind | Holds | Depends on |
|---|---|---|---|
| `wakeshot-core` | lib | events, state, policies, holds, settings ownership, file formats, source parsers, view models | `serde`, `serde_json` |
| `wakeshot-windows` | lib | every Windows call: power requests and settings, OS events, processes, folder watch, files and registry, tray, notices, sounds | `wakeshot-core`, `windows`, `serde_json` (the IO thread reads and writes the files) |
| `wakeshot-tray` | bin `wakeshot-tray.exe`, GUI subsystem | the mode dispatch (`main`, `mode`), guard mode (`guard`), restore mode (`restore`), the Velopack wiring (`velo`, the only file that names `velopack`), and the tray-mode call into `wakeshot_windows::app::tray_mode::run`; no wiring logic | both libs, `windows`; `velopack` in Phase 6 |
| `wakeshot-cli` | bin `wakeshot.exe`, console | the `wakeshot` command: the dispatch (`main`, `command`), argument parsing (`args`), one file per command (`run`, `keep_awake`, `automatic`, `status`, `restore`, `open_log`) | both libs, `windows` |

Both bins depend on both libs and on `windows` from `p1-contracts`, so no track changes `Cargo.lock`; `velopack`
comes in Phase 6 through a card the orchestrator runs alone. The bins call Windows only through `wakeshot-windows`.
Both libs have a `test-support` feature with the fakes; only dev-dependencies turn it on, and the Windows lib's
feature turns on the core's. Both exes take their resources (manifest, app icon `assets/wakeshot.ico`, version) from
`crates/build-resources.rs`, pulled into each `build.rs` with `include!`.

Every module below exists from `p1-contracts-split` (`app` and its files from `p6-int-skeleton`), so no track edits a
module list, a `main.rs` dispatch or a `Cargo` file. Each file's module doc names the card that owns it in its
`Owner:` line; the full table is in that card's Result.

Two exes because a console exe flashes a window at login and a GUI exe cannot print. Two bin crates, not one package
with two bins, because their dependencies differ: from Phase 6 only the tray takes `velopack`, and the bins of one
package share every dependency. Resources are not the reason: `embed-resource`'s `compile_for` can embed a different
`.rc` file in each bin of one package (general design review, docs.rs). Change it to one package if the two dependency
lists end up the same. macOS and Linux later add an adapter crate next to `wakeshot-windows`; the core does not change.

### Modules

`wakeshot-core`:

| Module | Main types |
|---|---|
| `time` | `Now { wall_ms: i64, mono_ms: u64 }`, `Deadline { Wall(i64), Mono(u64) }`, trait `Clock { fn now(&self) -> Now }`; `testing::FakeClock` |
| `process` | `ProcessRef { pid, start: FileTime }`, `Alive { Yes, Gone, Unknown }`, `Lookup { Running(FileTime), NotRunning, Unknown }` (a pid-only hold file), `FileTime` (FILETIME; a decimal string in files). In the core because `HoldFacts.alive`, `Liveness::Process` and `Started.tray` need them without the `windows` crate |
| `event`, `command` | `Event`, `Command`, `Outcome` and their field types (section 3); a compile-time check that both are `Send` |
| `core` | `Core::new(Config)`, `Core::handle(&mut self, Now, Event) -> Vec<Command>`, `Core::next_deadline(&self) -> Option<Deadline>`; `state` (private) |
| `holds` | `HoldId`, `HoldKind { Agent, Run, Script, KeepAwake }`, `HoldFacts`; `holds::file`: hold file parsing, `parse_expires`, `format_expires` (sources track) |
| `sources` | trait `AgentSource`, `WantedFile`, `Feed`, `Fed`, `SessionFacts`, `Activity`, `Liveness`, `FormatHealth`; `claude::{status, transcript}`, `codex`, `copilot`, `cron` (Claude's cron syntax, local time) |
| `policy` | pure rules: `requests`, `lid`, `lock`, `floor` (battery floor), `ending` (the countdown to sleep), `sessions` (agent sessions to holds), `hold_files` (hold files to holds) |
| `ownership` | traits `PlanIo` (plan calls) and `SafetyNet` (journal file, RunOnce, guard), `JournalRead`; `apply`, `restore`, `check`: the one apply, restore routine and ownership check every process runs (section 5), generic over the two traits; `journal`: `JournalFile` split into known entries and the rest, and merged back; `fake::FakePlans` (test-support): stored and in-force values, a call log, a script of failures, stops and changes by call index |
| `settings` | `Guid(u128)` (text lowercase `8-4-4-4-12`; the bit order of `windows::core::GUID::from_u128`), `SettingKey { LidAc, LidDc, SleepDc, SleepAc, VideoIdleAc, VideoIdleDc }` (`HOLD` the first three, `SHORT` the sleep route's four), `OwnedKey { plan, key }`, `PlanValues`, `Journal`, `JournalOwner`, `JournalEntry` |
| `files` | `Config`, `StateFile`, `JournalFile`, `JournalFileEntry`, `JournalKey { Known(SettingKey), Other(String) }`, `HoldFile`, `RestoreRequest`, `VersionProbe`: serde structs, each with `version`; `read::<T>(bytes) -> Read<T> { Value(T), Newer(u32), Bad }`, the version rule (section 6) for every file but the journal |
| `view` | `TrayView { icon: IconState, tooltip, menu: MenuModel }`, `MenuModel`, `MenuItem { Label(text), Separator, Action { text, command, checked, enabled }, Submenu { text, items } }`, `KeepAwake { Off, Minutes(u32), UntilOff }`, `MenuCommand { Automatic(bool), KeepAwake(KeepAwake), KeepScreenOn(bool), Restore, OpenLog, Quit }`, `Notice`, `Cue` |
| `present` | `TrayView`, `MenuModel` and `StateFile` from the core's state; every word from `text` |
| `text` | every user-facing string: menu, status line, tooltip pieces, notices (`NoticeText`, `NoticeClick`); worded by the tray track, called by `present` |
| `sim`, `testing` | the `Sim` harness; `FakeClock`, `FakeSource` (test-support) |

`wakeshot-windows`:

| Module | Main types, and what they wrap |
|---|---|
| `error` | `WinError { call, code }`, `From<WinError> for Failure` |
| `clock` | `now() -> Now` (`GetSystemTimePreciseAsFileTime`, `QueryUnbiasedInterruptTime`), `SystemClock` (implements `Clock`), `input_idle_ms()` (`GetLastInputInfo`) |
| `power::requests` | `PowerRequests`: `PowerCreateRequest`, `PowerSetRequest`, `PowerClearRequest`, `retake()`; real and dry run |
| `power::plan` | `Win32Power` and `DryRunPower`, the Windows forms of the core's `PlanIo`; `PlanSetting { LidAc, LidDc, SleepAc, SleepDc, SaverThresholdDc, VideoIdleAc, VideoIdleDc }` (reads need the saver threshold for `PlanValues`; writes stay limited to `SettingKey`) |
| `power::journal` | `FileNet`, the Windows `SafetyNet` (journal file, RunOnce, guard); `restore_locked`, `apply_locked`, `check_locked`: the core's routines under `Local\Wakeshot.Journal`, the only way any process calls them |
| `power::executor` | runs `ReadPlan` (`check_locked`, then `PlanIo::values`), `Apply` and `Restore` on the IO thread; the startup restore |
| `power::hibernate` | the hibernate thread: the last check, the going to sleep sound, the lid once more, `SetSuspendState`; the same check and sound with no hibernate for the sleep route (`run_sleep_check`) |
| `power::system` | lid, AC, battery, active plan, suspend and resume notifications |
| `io_queue` | `IoQueue`: the IO thread's commands in order; `push_front` and the end-of-session rule (section 4) |
| `session` | session notifications, `lock()`; real and dry run |
| `recovery` | guard mode, restore mode, the tray's guard control |
| `process` | trait `Liveness` (`check`, `start_of_pid`, `list`) with `SystemLiveness` and `FakeLiveness`; `liveness(ProcessRef) -> Alive` (types from the core), `ProcessHandle`, exit waits (`ExitWait`), the system process list, spawns outside the tray's tree (`spawn_outside_tree`) |
| `watch` | `dir` (folder notices), `read` (whole reads, tails, finds, names), `data` (holds, config, restore request), `agents` (sources and their liveness), `thread` |
| `lockowner`, `localtime`, `child`, `shell` | which process holds a file open (Restart Manager); `SystemZone` for cron times; `wakeshot run`'s child and Ctrl+C handler; open a file or `ms-settings:` URI with the default app |
| `store` | `DataDir` and `DataFile` (data folder paths), `write_atomic`, `read_whole`, `remove`, `Names` and `NamedMutex` (the three mutexes), `AutoStart` (Run and RunOnce values) |
| `sink` | trait `EventSink`, how off-loop threads post events to the loop; `FakeEventSink`. The real sink is `app::window::WindowSink` |
| `tray` | `Anchor`, `IconIdentity`, trait `MenuView`; `icon` (`TrayIcon`, its own hidden window), `menu` (`OwnerDrawnMenu`), `menu_paint`, `theme`, `art` (icon art as code) |
| `notify`, `sound`, `log` | trait `Notifier` with `BalloonNotifier`; trait `SoundPlayer` with the WASAPI player; trait `Log` with `LogWriter` and `MemoryLog` |
| `install`, `hooks`, `update` | the CLI folder in the user Path (the Run value is `store::AutoStart`'s); what each Velopack hook does (`HookEnv`); update check, download and apply (`UpdateFeed`) |
| `app` | the tray mode's wiring: `window` (the loop's hidden top-level window, the pump, its one timer, `WindowSink`), `dispatch` (`Dispatcher`: the core in the loop, the runner for each command, the tick, end of session, quit), `io_thread` (the IO thread loop over `IoQueue`), `startup` (startup steps 3 to 7 and `Started`), `tray_mode` (the composition root) |
| `testing` | the fakes in one place (re-exports) and `FakeMenuView` (test-support) |

### Boundary traits

Only where an implementation is swapped:

```rust
// wakeshot-core::sources: one per agent. Bytes in, facts out; no I/O. The watch thread owns them as
// Box<dyn AgentSource + Send>. WantedFile: Folder { dir, accept: fn(&str) -> bool } (the name filter runs before any
// open) | Names { dir, depth, accept } (listed, never opened) | Tail(path) | FindTail { root, depth, name_suffix };
// FileId(PathBuf).
pub trait AgentSource {
    fn id(&self) -> SourceId;
    fn wanted(&self) -> Vec<WantedFile>;                  // folders and files the adapter must feed
    fn feed(&mut self, file: &FileId, data: Feed) -> Fed;  // Feed: Whole | Appended | Present | Gone;
                                                           // Fed: Taken | RetryOnce (read again 200 ms later, once)
    fn sessions(&self) -> Vec<SessionFacts>;
}

// wakeshot-core::ownership::io. The steps of the apply, the restore routine and the ownership check, one call each,
// so the routine in the core owns their order. The power track implements both, real and dry run.
pub trait PlanIo {                                    // `plans`: the whole list or an error
    fn active_plan(&mut self) -> Result<Guid, Failure>;
    fn plans(&mut self) -> Result<Vec<Guid>, Failure>;
    fn can_write(&mut self, key: SettingKey) -> Result<(), Failure>;
    fn read(&mut self, plan: Guid, key: SettingKey) -> Result<u32, Failure>;
    fn write(&mut self, plan: Guid, key: SettingKey, value: u32) -> Result<(), Failure>;  // stored only
    fn set_active(&mut self, plan: Guid) -> Result<(), Failure>;                          // now in force
    fn values(&mut self) -> Result<PlanValues, Failure>;  // for Event::Plan; the routines never call it
}
pub trait SafetyNet {
    fn load_journal(&mut self) -> JournalRead;        // Missing | Corrupt | Failed(Failure) | Read(JournalFile)
    fn save_journal(&mut self, journal: &JournalFile) -> Result<(), Failure>;  // atomic, write-through
    fn delete_journal(&mut self) -> Result<(), Failure>;
    fn set_aside_journal(&mut self) -> Result<(), Failure>;   // to journal.json.corrupt
    fn set_run_once(&mut self) -> Result<bool, Failure>;      // true: this call added it
    fn clear_run_once(&mut self) -> Result<(), Failure>;
    fn ensure_guard(&mut self) -> Result<bool, Failure>;      // true: this call started one
    fn stop_guard(&mut self) -> Result<(), Failure>;
}
// The routines: restore(plans, net, plan: Option<Guid>, expect_owner: Option<JournalOwner>) -> Restored;
// apply(plans, net, me: JournalOwner, plan, writes) -> Applied; check(plans, net) -> Checked.

// wakeshot-windows.
pub trait EventSink: Send + Sync { fn post(&self, event: Event) -> Result<(), WinError>; }  // to the loop window
pub trait MenuView { fn show(&mut self, menu: &MenuModel, anchor: Anchor); }  // choice returns as Event::Menu
pub trait Notifier: Send { fn show(&self, notice: &Notice) -> Result<(), WinError>; }  // becomes Outcome::Notified
pub trait SoundPlayer: Send + Sync {
    fn play(&self, cue: Cue);  // never blocks; the result comes back as Done(Sound) through the sink, for the log
    fn play_and_wait(&self, cue: Cue) -> Result<SoundEnd, WinError>;  // hibernate thread only; at most 3 s
}
pub trait Liveness: Send + Sync {
    fn check(&self, process: ProcessRef) -> Alive;
    fn start_of_pid(&self, pid: u32) -> Lookup;
    fn list(&self) -> Result<Vec<ProcessEntry>, WinError>;
}
pub trait Log: Send + Sync { fn line(&self, level: Level, area: &'static str, text: &str); }
```

`MenuView` is the seam for the one pick still open: how the menu is drawn (`p1-tray-menu-look`). The core builds
`MenuModel`; the pick does not change the core or the loop. Expected answer, still subject to the on-screen check:
owner-drawn items in the classic Win32 menu plus `DWMWCP_ROUND` on the menu window (`research/tray-menu-look.md`,
"Pick"). The menu follows the Windows mode (`SystemUsesLightTheme`), like the tray icon (decided 2026-11-03). If a
later pick needs a XAML flyout, its thread and runtime stay inside the `MenuView` implementation.

`SoundPlayer`, picked by `p1-lid-sound` (research, "Lid-close sound"): Wakeshot's own short chimes, built into the exe,
played through WASAPI shared mode on a thread of their own. Device rule for every Wakeshot sound: the default render
device, unless its form factor is display audio (HDMI or DisplayPort, `DigitalAudioDisplayDevice`); then the active
endpoint with form factor Speakers, or the default if there is none. Headphones and headsets stay as they are. With no
default device: the first active Speakers endpoint, else the first active one (`sound/device.rs` `pick`). The three
chimes are made in code (`sound/chime.rs`); the sound scheme "No sounds" (`.None`) skips the sound, which ends as
`SoundEnd::Skipped` (research "Tray UX: chimes"). `PlaySound` with `SND_SYSTEM` is not used: it plays on the default
device, which in Phase 0 run D was a screen with no speakers. A sound starts about 0.3 to 0.8 s after the call here
(stream start-up); nothing waits for it. Which moments get a sound: `product.md` (Phase 5 builds them).

### Ported from the spikes

Ported by reading and rewriting, with their tests; never copied whole.

| Spike code | Goes to |
|---|---|
| `power-settings/src/power.rs`: GUIDs, `read`, `write`, `set_active`, `access_check*`, `schemes` | `power::plan`; the port's `schemes` ends only at 259 (`ERROR_NO_MORE_ITEMS`) and fails on any other code, where the spike's loop stops as if the list ended (section 5, restore step 2) |
| `power-settings/src/journal.rs`: `Journal::plan` and its tests, without the plan copy parts | `ownership` (core) |
| `lid-sleep/src/power.rs`: `Hold::request`, `retake`, `set_suspend_state`, `hibernate_allowed` | `power::requests`, `power::system` |
| `lid-sleep/src/events.rs`: `Subscriptions::start` and the callbacks | `power::system`; callbacks post to the loop |
| `lid-sleep/src/sys.rs`: `enable_shutdown_privilege`, `idle_ms`, `unbiased_ms`, `session_locked`, `power_status` | `clock`, `session`, `power::system` |
| `lid-sleep/src/sound.rs`: the chime, the device rule, the WASAPI play | `sound` |
| `claude-watch/src/probe.rs`, `procs.rs`: open with exact rights, creation time, wait, system process list | `process` |
| `claude-watch/src/names.rs`, `watch.rs` (`first_object`, `find_transcript`), `tail.rs`, `summary.rs` | `sources::claude` (core) and `watch` |
| `tray/src/main.rs` (icon, menu), `theme.rs`, `build.rs`, the manifest | `tray`, the tray's `build.rs` |

Not ported: the spikes' plan copy, heat, display count, wake timer and log code, and the `PlaySound` code (no v1 rule
uses them).

## 3. The core

### Events, state, commands

```rust
pub enum Event {
    Started { config: Config, journal: Option<Journal>, journal_unread: bool, owner: bool, on_console: bool,
              tray: ProcessRef, first_run: bool },
                                                // config wins over Core::new's; journal: known entries only, None
                                                // when there are none; journal_unread: journal.json did not read at
                                                // start, so the core retries it as Restore { why: Leftover } on
                                                // each tick, for two minutes, then ResetLost; owner: holds the
                                                // settings mutex; on_console:
                                                // this session is the one on the PC's own screen
                                                // (WTSGetActiveConsoleSessionId); tray: this tray's own process,
                                                // for state.json and the Keep awake hold; first_run: config.json
                                                // was missing (Notify(FirstRun) and WriteConfig)
    SettingsOwner,                              // the settings mutex was taken later (retried on each tick)
    Tick,                                       // safety tick and due deadlines
    Sources { source: SourceId, sessions: Vec<(SessionFacts, Alive)> },
                                                // full snapshot per source; each session paired with the
                                                // adapter's liveness answer, so the core can tell Gone from Unknown
    Holds(Vec<HoldFacts>),                      // parsed hold files, with liveness
    ConfigChanged(Config),
    Menu(MenuCommand),                          // Automatic, Keep awake, Restore (also from the CLI), Open log, Quit
    Lid { closed: bool },
    Power { ac: bool, battery_pct: Option<u8> },
    Plan(Result<PlanValues, Failure>),          // a failed read is a failure, never values
    ActivePlan(Guid),                           // the active plan changed (GUID_ACTIVE_POWERSCHEME); the core
                                                // sends ReadPlan when it differs from the last read, never more
    UserChanged(OwnedKey),                      // the IO thread saw a value we wrote change; it dropped the entry
    JournalCorrupt,                             // a corrupt journal was set aside; once per corrupt file; the
                                                // journal is lost: the core sends ResetLost { damaged: true }
    InputIdle { ms: u64 },                      // read on each tick
    Suspending, Resumed,
    Session(SessionChange),                     // Locked, Unlocked, Ending, EndCancelled, Ended,
                                                // ConsoleLeft, ConsoleBack (WTS_CONSOLE_DISCONNECT, _CONNECT)
    UpdateReady { version: String },            // an update is downloaded (ship track); once per version per run
    Updated { version: String },                // Velopack restarted the tray after this update (ship track); at
                                                // most once, at start, after Started; the core sends
                                                // Notify(Updated)
    Done(Outcome),                              // result of a command
}

pub enum Command {
    Requests { system: bool, display: bool },   // system means SYSTEM plus EXECUTION
    Retake,                                     // after resume; answers as Outcome::Requests
    ReadPlan,                                   // every 10 s while anything is owned, and at a tick once the
                                                // last read is 10 s old or more (at most one per 10 s)
    Apply { plan: Guid, writes: Vec<(SettingKey, u32)> },
                                                // writes: the three keys less the released ones, never less a key
                                                // already at its target; the apply restores this plan's entries
                                                // for other keys in the same run (section 5)
    Restore { plan: Option<Guid>, why: RestoreWhy }, // None: every entry; Some: that plan's entries only
                                                // why: WorkEnded, UserAsked, PlanSwitch, Floor, Quit, SessionEnd,
                                                // ConsoleLeft, Leftover (a journal left at start; run only when
                                                // its owner is Gone or this tray), ShortTimeouts (the sleep
                                                // route's 1-minute timeouts go back)
    ResetLost { damaged: bool },                // a lost journal ("A journal that cannot be read" below): on the
                                                // plan in use each key at a value Wakeshot writes (a hold's 0,
                                                // the sleep route's 1 minute) goes to the plan's Windows
                                                // default, the bad file goes aside (ownership::reset)
    Lock,
    SleepCheck { reason: CountdownReason, floor_pct: u8, last_resume_mono_ms: Option<u64> },
                                                // the countdown is due: the hibernate thread's last check and the
                                                // going to sleep sound, no hibernate; then the core writes the
                                                // short timeouts ("The sleep route" below)
    Hibernate { reason: CountdownReason, floor_pct: u8, last_resume_mono_ms: Option<u64> },
                                                // the fallback: what the hibernate thread's last check needs
                                                // (section 5); that thread plays the going to sleep sound, never
                                                // the core
    Tray(TrayView), Notify(Notice), Play(Cue),
    WriteState(StateFile), WriteConfig(Config),
    WriteHold { id: HoldId, hold: HoldFile },    // Keep awake from the menu (holds\keep-awake.json)
    DeleteHold(HoldId),
    FinishRestoreRequest(StateFile),            // after Done(Restored { why: UserAsked }): write state.json, then
                                                // delete restore-request.json
    Log(String),                                // a line only the core can write (section 8); no answer
    OpenLog,
    ApplyUpdate,                                // only when no hold counts, no hold file other than Keep awake
                                                // names a live or Unknown process, nothing may be owned, no
                                                // countdown, work end or hibernate is on, and the session is not
                                                // ending (ship track runs it); an expired Keep awake file does not
                                                // wait: it names this tray, which quits for the update
    Quit,
}

pub enum CountdownReason { WorkEnded, Floor }
pub enum HibernateSkip { LidOpen, LidUnknown, InputSeen, OnAc, AboveFloor }  // the why of HibernateEnd::Skipped
pub enum FileKind { State, Config, Hold(HoldId), RestoreRequest }
                                       // Written also answers WriteHold, DeleteHold and FinishRestoreRequest
pub enum SoundEnd { Played, Skipped }  // Skipped: the user's sound scheme is No sounds

pub struct PlanValues { pub active: Guid, pub lid_ac: u32, pub lid_dc: u32, pub sleep_ac: u32, pub sleep_dc: u32,
                        pub saver_threshold_dc: u32 }
pub struct OwnedKey { pub plan: Guid, pub key: SettingKey }
pub struct Failure { pub call: &'static str, pub code: u32 }  // what the Windows layer's WinError carries

pub struct HoldFacts {
    pub id: HoldId,                    // file name
    pub kind: HoldKind,                // Run, Script, KeepAwake
    pub alive: Alive,                  // Yes | Gone | Unknown (section 5, "Liveness")
    pub expires_wall_ms: Option<i64>,
    pub reason: Option<String>,        // cut to 60 characters
    pub screen: bool,                  // read only from keep-awake.json
}

pub enum Outcome {
    Requests(Result<(), Failure>),
    Applied { plan: Guid, written: Vec<SettingKey>, result: Result<(), ApplyError>, journal: Vec<JournalEntry> },
    Restored { plan: Option<Guid>, why: RestoreWhy, result: Result<(), Failure>, journal: Vec<JournalEntry> },
                                       // Err: journal kept, retried on the tick while still wanted (episode flow)
                                       // journal: the entries after the command; the core's mirror is this copy
    ResetLost { reset: Vec<SettingKey>, result: Result<(), Failure>, journal: Vec<JournalEntry> },
                                       // reset: the keys set to the default; journal: known entries when the
                                       // file read after all, and then nothing was reset
    Locked(Result<(), Failure>),
    SleepCheck(Result<(), HibernateSkip>),  // Ok: write the short timeouts; LidUnknown counts as Ok
    Hibernate(HibernateEnd),           // Skipped(HibernateSkip) | NotAllowed | Failed(Failure) | Resumed
    Sound(Result<SoundEnd, Failure>),  // logged only
    Notified(Result<(), Failure>),
    Written { file: FileKind, result: Result<(), Failure> },
    Update(Result<(), Failure>),       // Ok: the updater waits for this process to exit, so the core takes the
                                       // menu's Quit path (requests off, restore, Keep awake file deleted, Quit)
}
pub enum ApplyError {
    PlanChanged,                       // the active plan is not `plan`: at step 1 nothing written; at step 6 written
                                       // to a plan now inactive, journaled, left to the plan switch (section 5)
    SafetyNet(Failure),                // access check, read, RunOnce, guard or journal failed: nothing written
    Write(Failure),                    // a write, read-back or the call failed: the restore routine ran on
                                       // this plan's entries (section 5, Apply step 6)
}
```

`Notice` variants: `FirstRun`, `UserChanged(SettingKey)`, `RestoredByUser`, `StepBack { floor_pct }`,
`RequestsOnly` (settings blocked or a failed apply), `CannotHold`, `RestoreFailed`, `HibernateFailed`, `LockFailed`,
`ClosedLidReport { ran_ms, held_by }`, `UpdateWakeshot { agent }`, `JournalLost { reset, failed }` (a lost journal's
reset set these keys to Windows defaults, or a call of it failed), `IdleLocked` (Keep screen on locked the PC),
`Updated { version }` (Wakeshot updated itself; Haik's yes, 2026-11-03). The tray track words each one in
`text::notice`. A notice raised while the session is locked or another session has the PC's screen waits, and the
waiting ones go out in order once the session is unlocked and on the console again: a balloon shown to nobody is lost.
The same notice twice in one absence waits once. Sounds never wait.

State is one struct owned by the loop thread: `automatic`; holds by source; live `PlanValues`; `ac`, `battery_pct`,
`floor_pct`, `lid`; `episode: Option<Episode>` (from the first hold to the last: `released: Set<SettingKey>`, on every
plan, `lid_closed_since`, labels of what held); a mirror of the journal, a copy of the entries that `Done(Applied)` and
`Done(Restored)` carry, less the keys `UserChanged` names, with when it became unknown and whether a `ResetLost` is
out; `requests`; `stepped_back` (battery floor); `on_console`;
`session_ending`; `ending: Option<Countdown>`; `short` (the sleep route's short timeouts in force, with the fallback's
deadline) and the short keys the user changed in this countdown; `last_resume_mono` and `last_unlock_mono`; notices
already shown; and the lid side's own
memory (`lock: LockWatch`): the lock watch (a closed-lid `Lock` until `Session(Locked)` confirms it), the lock step
back, the lock after a hibernate that did not happen, notices kept for the user's return, countdown rules waiting for
a plan read that tells the user's own lid action, and the journal mirror as the last settle pass saw it.

The lid, lock and countdown rules read the plan as in force now (`Core::in_force`), not the last plan read, which an
apply leaves up to 10 s behind: each mirror entry on the active plan sets its key to `written`, and an `Apply` with no
answer yet on the active plan counts as written for each key not already at its target. Both err toward a lock: an
entry the user changed in the last 10 s, or an apply in flight that fails, still counts as ours. A restore leaves the
plan copy behind the other way, showing what Wakeshot wrote until the next read (up to 30 s once nothing is owned): an
entry that left the mirror, whose key the copy still shows at `written`, puts its `original` back in the copy, and a
later read replaces it (power safety review finding 5). Before the first plan read, a countdown rule that meets the lid
closed while the `Started` journal holds entries (a crashed tray's values) waits for that read and decides on it
(power safety verify minor 3). A work-end or Floor countdown also waits while the journal is unknown ("A journal that
cannot be read", below): a Do nothing in force may be a crashed tray's, and once a restore or the reset puts another
value in, the copy shows the Do nothing until a later read, since no mirror entry names the key. So a countdown that
waited takes no Do nothing as final: it starts once a read shows the user's own action for its power source (battery
for Floor) as not Do nothing, and ends when the lid opens, a hold starts or, for Floor, AC comes back
(`p2-6-unread-lid-countdown`, `p2-6-unread-floor-countdown`). When the unknown window ends, known or lost, with the lid
closed and nothing holding, the work-end want starts waiting the same way; with a hold that holds nothing because of
the battery floor, the Floor want does (`Core::unknown_journal_ended`, `p2-6-record-lost`). A lid close inside the
window, with the session unlocked, gets no lock then; that lock waits until the lid opens or the session locks,
not only until the window ends (`Core::settle_window_lock`; `p2-6-sleep-route` deliverable 8,
`p2-6-sleep-route-fixes` review major 1). It comes, with the chime, once the journal is known and Wakeshot keeps the
closed laptop running (a hold holds: at the window's end, when one starts later, or when a step back ends; or its own
Do nothing is in force), and a read shows the user's own lid action as not Do nothing on a power source that has
lasted 30 s, so a dock's short AC drop does not lock a docked user (review minor 7). With the user's own Do nothing,
no lock, as at a lid close. It is watched like a lid-close lock, also when the hold runs on requests only. A reset of
the lost journal that fails locks such a laptop at once (review minor 5): the journal then counts as known and empty,
so a Do nothing the reset could not put back would read as the user's own, and nothing else would ever lock it.

### Clocks

- `wall_ms` (UTC) for times a person reads as clock time: Keep awake end, Claude timer due times, the 1 hour waiting
  limit (from `statusUpdatedAt`), hold `expires`. After a manual sleep past such a time, the hold ends at resume.
- `mono_ms` (unbiased interrupt time) for how long the PC ran: the closed-lid report, and the run-time cap on the
  waiting limit (section 7). It skips suspended time and counts screen-off standby, where apps still run (research,
  "Clock").
- Input idle (`GetLastInputInfo`) counts suspended time (research, "Clock"). The core treats a resume as input:
  idle = min(reported idle, `mono_ms` since the last `Resumed`). The two count the same time there, because no
  suspend lies between the last resume and now. So a countdown never sleeps a PC the user just woke. The countdown
  also counts idle from the last `Unlocked`: a sign-in by face or fingerprint may give no input Windows counts, and the
  user who just signed in must not hear the going to sleep sound (`p2-6-sleep-route`). Input after the short
  timeouts is a reading under the one before: reported idle only grows while nobody types, a sleep included, and
  readings come at least every 30 s, far below the 5 minutes the short timeouts start from.
- Safety tick every 30 s. It reads input idle, runs due deadlines, issues `ReadPlan` (any tick does, once the last
  read is 10 s old or more, so a burst of short deadlines sends no burst of reads), re-checks liveness that only the
  system process list can see, and retries the settings mutex when this tray does not own it. The loop keeps one
  timer set to the nearer of the next deadline and the next tick.

### Episode flow

Holding starts (first hold, not stepped back):

1. `Requests { system: true, display }`; `display` only for Keep screen on.
2. If this instance owns settings, its session is on the console and no policy blocks them: `Apply` to the active
   plan: lid AC, lid DC (Do nothing, 0) and sleep DC (Never, 0), less the keys the user released in this episode. The
   IO thread skips a key whose live value already is the target and has no journal entry (the user's own value), and
   takes over an entry left by a failed restore (section 5, Apply step 3). Written also on AC (section 5).

While holding:

- `ReadPlan` every 10 s while anything is owned. The IO thread decides ownership (section 5, "Ownership"): a value we
  wrote that changed is the user's; the IO thread drops its entry and posts `UserChanged`; the core marks the key
  released on every plan for the rest of the episode and shows a notice. A failed read releases nothing.
- Plan switch (a `Plan` with a new active id, handled first in that event): `Restore { plan: Some(old) }`, then
  `Apply` to the new plan.
- Lid closes while the lid action in force is our Do nothing: `Lock` and `Play(LidClosedHolding)` at once, side by side;
  note `lid_closed_since`. Also with nothing holding, when a restore that failed or has not landed left our Do nothing
  in force (the countdown below starts too). While the journal is unknown ("A journal that cannot be read", below)
  there is no lock at a lid close (the lock that close did not get waits: "Events, state, commands" above) and no
  source lock: no entry names a key, and a Do nothing in force may be the user's
  own (Haik, 2026-11-04; this replaced the guess of `p6-int-wiring-fixes` decision 7). The lock is watched: a lock that fails, or that no `Session(Locked)` follows within 5 s (`LockWorkStation` only
  starts it), is sent once more at the next tick; if that one goes the same way, step back as at the floor (requests
  off, restore, the work-end countdown) until the session locks or the lid opens, then hold again as at a hold start
  (section 8, "Lock fails"). The same for the source lock below. The lock does not wait for the sound. Tested: the
  Windows alias sound played in full with the lock 0.2 ms after its call, and our chime was heard both in a session
  already locked and with its stream running before the lock (research, "Lid-close sound", "Lid-close results"). Not
  tested: our chime with the lock landing during its stream start, which is the v1 order (checklist 16). Change it when
  a test shows the lock cuts a sound. A user whose own lid action is Do nothing (docked use) gets no lock and no sound.
- Power source changes while the lid is closed: if the lid action in force for the new source is our Do nothing, and
  either it was the user's own before (their AC action is Do nothing, ours is the DC one) or the change is AC to
  battery, then once the new source has lasted 30 s: `Lock` and `Play(LidClosedHolding)`. AC to battery covers a user
  who signed in on an external screen after the lid-close lock and then unplugs the dock: without it the laptop goes
  into a bag running and unlocked. The 30 s (Haik, 2026-11-03) keep a dock's short AC drop (5.3 s and 6.3 s on the
  test laptop; research, "AC can drop for a few seconds while docked", "Power rules checks") from locking someone who types on an external screen; someone who unplugs only the charger while doing so
  is locked, the safe side (decision 18). What Windows itself would do with the user's values at that moment: not
  tested (checklist 8).
- Another user's session takes the PC's own screen and keyboard (fast user switching, `Session(ConsoleLeft)`):
  `Restore { plan: None }` and hold the system request only, also with Keep screen on; `ConsoleBack`: the screen
  request again if asked for, and `Apply` as at a hold start. Plans are machine-wide, so our Do nothing would keep the
  laptop running when the other user closes the lid, this tray can lock only its own session, and the other user's
  tray reads Do nothing as their own value (no lock, no chime). The screen request goes too, so the other user's
  screen still turns off and Windows locks their session (whether Windows honors a screen request from a session not
  on the console: not verified). Remote Desktop into this session most likely counts as leaving too (Microsoft's text
  names "the console terminal or RemoteFX session"; not tested), so a user working over Remote Desktop gets requests
  only. A restore that fails here warns in this session, not the user at the screen; it is retried at the ticks (below).
  Session notices: not tested (checklist 11). Phase 1 Question 10: the default, Haik 2026-11-03 18:09.
- On battery under the floor (energy saver threshold DC; 20 when it is 0 "Never" or 100 "On battery"): step back:
  `Requests` off, `Restore`, notice and `Play(BatteryFloor)`; if the lid is closed and the user's own battery lid
  action is not Do nothing, start the countdown (reason Floor). A lid close after the step back follows the same
  rule (Haik, 2026-11-03; power safety review finding 7).
  No hold starts while stepped back. AC back: hold again.
- Keep screen on: `Lock` and `Notify(IdleLocked)` when input idle passes the user's own sleep timeout for the current
  power source (from the journal's `original` while we own it; none when it is Never), once per idle period. Windows locks by itself at
  screen-off standby otherwise (research, "Lock").
- `Resumed`: `Retake` if any request is on.

Holding ends (last hold gone):

- `Requests` off and, when anything may be owned, `Restore { plan: None }` at once, lid open or closed. May be
  owned: the mirror holds an entry, an `Apply` has no answer yet, a restore failed (the mirror may not know what the
  journal holds), or a lost journal's reset has not answered. The console-left and battery-floor restores follow the same rule; the user's Restore, Quit and
  session end go out whatever the mirror says.
- If the lid is closed and the user's own lid action is not Do nothing: countdown (reason WorkEnded). Settings come
  back first, so a crash during the countdown leaves nothing overridden; the sleep route's short timeouts come later,
  as journal entries like the hold's.

A failed `Restore` is retried at a tick, at most once per 10 s, right before that tick's `ReadPlan`, so the read's
ownership check sees the journal the retry left. While the core applies (holding, on the console, not stepped back,
not at session end, some hold key not released, and no `Apply` failed in this episode; a short key the user changed
in a step back is released too, but never stops the hold: review minor 4 of `p2-6-sleep-route`), the retry takes only
the entries on plans other than the active one, one `Restore { plan: Some(p) }` per plan, and the hold's `Apply` takes
over the entries on the active plan and restores those whose key the user released. Otherwise the retry is
`Restore { plan: None }`, so after a failed apply (`SafetyNet` or `Write`: requests only for the episode) it does not
wait for an apply that never comes. So a retry never undoes a running hold, and a hold that starts while a restore
fails rests on entries the journal still holds. The 10 s read deadline is set only while the mirror holds an entry or
an apply waits for a plan (`core/settings.rs` `plan_read_due`), so with an empty mirror (a journal that did not read)
the retry comes at the 30 s safety tick (measured, `p2-6-unread-floor-countdown` Result item 4).

A journal that cannot be read (product "User settings safety" 7, Haik 2026-11-04; `p2-6-record-lost`). The journal is
unknown while a restore is still wanted and the mirror names no entry (`journal.json` did not read at start, or a
restore failed to load a journal the mirror knew as empty), and while a `ResetLost` waits for its answer. A failed
load with a known mirror is not this case: the mirror names what is ours, and the retry goes on as above. While the
journal is unknown the retry goes on at the ticks, a failed one shows no `RestoreFailed` (the window ends in a read or
the reset, each with its own word), and there is no lid lock and no countdown (both above). Two minutes of run time
after it became unknown (a sleep in between pauses the clock, so the tries still happen), the tick sends
`ResetLost { damaged: false }` in place of the retry; `JournalCorrupt` sends `ResetLost { damaged: true }` at once. The
reset (`ownership::reset`, under the journal mutex) loads the file once more: known entries mean it read after all,
nothing is reset, and the mirror takes them; a missing file, or one with no known entry, means someone else restored
it, unless `damaged`. Otherwise, on the plan in use, each key that holds exactly a value Wakeshot writes gets the
plan's Windows default (`PowerRead*DefaultIndex` of the plan's personality, research "Windows defaults of the three
keys"), then `PowerSetActiveScheme`; any other value is the user's and stays. Those values are a hold's (lid Do
nothing, battery sleep Never) and the sleep route's (sleep and screen off after 1 minute, AC and battery), so a crash
while the 1-minute timeouts are in force does not leave them for good (`p2-6-sleep-route-fixes`, review major 2: the
orchestrator's call under rule 7, for Haik's review; the cost is the same, a user's own 1 minute goes to the
default). With `damaged`, known entries are this tray's own apply when it owns the settings (only the owner applies):
it ran between the set-aside and the reset, since the IO queue held it already, and took the values in force as its
originals, Wakeshot's own among them (a hold that starts over the 1-minute timeouts journals battery sleep at 60 s).
So the core sends `Restore { plan: None }`, the reset again and the apply again (and the short timeouts, if they are
in force), in that order on the IO queue; once only, so a second find keeps the entries (review major 2, case B).
The bad file goes aside afterwards (a failed move is logged, nothing more), and the executor marks it seen, so the next
start does not reset again. The log has a line per key with its old and new value, or why it stayed. The answer makes
the journal known and empty, also when a call failed; then a read goes out, since the plan copy shows the old values.
One `JournalLost` notice when a key was reset or a call failed, kept for the user's return like `HibernateFailed`;
nothing reset is the log's only. A reset whose answer never comes goes out again two minutes later. Only the plan in
use is reset: a lost journal's values on another plan stay (rare: a crash after a plan switch whose restore failed,
then a journal that never reads).

Countdown: due when input idle (clamped at resume and unlock as above) reaches 5 minutes (Haik, 2026-11-03). Cancelled by a new
hold (not while stepped back, at the floor or after a failed lock, since no hold holds then), lid open, or AC back
(Floor reason; a step back after a failed lock is no floor one). It pauses while another session has the PC's
screen: input idle is this session's, so the other user's typing would not count. A work-end countdown also pauses
while the user's own lid action for the current power source is Do nothing; after that pause it starts again only
once a source whose own action is not Do nothing has lasted 30 s (`policy::lid::SOURCE_SETTLE_MS`), so a dock's short
AC drop never sleeps a docked laptop left idle. A lid close on our Do nothing while nothing holds (a restore that
failed or has not landed) starts the countdown as a work end does. When due: the sleep route below; the hibernate
thread plays the "going to sleep" sound after its last check, and the core sends no `Play` for it.

The sleep route (Haik, 2026-11-04: "just make it sleep"; `p2-6-sleep-route`). On Modern Standby no app call starts a
sleep (product, work end), so Wakeshot uses Windows' own idle sleep. The same for the work end, the battery floor and
the step back after a failed lock:

1. Due: `SleepCheck`. The hibernate thread checks the lid, idle and (Floor) the battery as before a hibernate, plays
   the sound, and reads the lid once more; an unknown lid does not stop it, since the core's own reading says closed.
   Input, AC or the floor skip it as they skip a hibernate; the lid open ends the countdown.
2. Passed: `Apply` of the four short keys (sleep after and screen off, AC and DC) at 60 s, less the keys the user
   changed in this countdown or in a stepped-back episode. The apply writes a key only where the value in force is
   longer or Never (`ownership::apply::meets`), as Wakeshot-owned journal entries with the RunOnce value and the
   guard, like a hold's. The screen timeout is needed: on the test laptop the sleep came when the screen timer turned
   the screen off, not at the sleep timeout (research, "Idle sleep with the lid closed").
3. Fallback: 5 minutes of run time after the write, and again after each resume, if the PC has not slept and nothing
   took the short timeouts back: `Hibernate` as before. A sleep is `Suspending` from the suspend-resume callback,
   followed by `Resumed`, or, when that post was lost, by a tick whose wall clock moved at least 5 s past run time. Not
   allowed, failed or the lid unknown: `HibernateFailed`, once per countdown; the short timeouts stay, so Windows
   sleeps the laptop once nothing holds it, and the relock below. A failed write makes the fallback due at once. Where
   nothing can be written (another session owns the settings, a policy blocked them, the session ends, every short key
   is the user's), the countdown sends `Hibernate` at once, as before the sleep route.
4. They go back (`Restore { why: ShortTimeouts }`) at an unlock (the user is there), input (then the countdown waits 5
   minutes again), the lid open, a hold start (before its apply, which also takes battery sleep over), the countdown's
   end (AC back for Floor, a pause, a resumed hibernate) and every other restore point (quit, sign-out, next start,
   uninstall, guard). Not at a wake alone: a laptop that wakes with the lid closed sleeps again. A user's change to a
   short key is released as a hold's is. A plan switch moves them to the new plan.

While the lid stays closed after a hibernate that did not happen, the core locks the laptop, `Lock` with
`Play(LidClosedHolding)`, at an input reading 5 minutes idle with the session unlocked and on the PC's screen, at most
every 5 minutes, until the lid opens; not on a power source whose own lid action is Do nothing (as without Wakeshot),
and only once that source lasted 30 s (power safety verify minor 1; worker's call for Haik's review).

Closed-lid report: after a hold that ran with the lid closed, a notice says how long the PC kept running with the lid
closed (`mono_ms`) and what held it. It goes out at the first unlock with the lid open, or when the lid opens while
unlocked; a docked user who signs in on the external screen with the lid closed gets it when the lid opens (Haik,
2026-11-03). Notices a closed laptop showed with the session unlocked, which nobody may have seen (`LockFailed` at a
lock step back, `HibernateFailed`), go out again right before it, also when no report is due (power safety verify
minor 2).

User actions: "Restore my power settings" (menu, or `wakeshot restore` through its request file, section 4) restores,
marks all three keys released for the episode (so no plan gets a write until the work ends, also after a plan
switch), keeps the requests, shows a notice. Quit: requests off, restore, delete the Keep awake hold, exit. Automatic
off: agent, run and script holds stop counting; Keep awake still counts.

### Decisions and where they live

Every decision in `product.md`, by section. Phase numbers are from `plan.md`.

| # | Decision | Where | Phase |
|---|---|---|---|
| 0 | The user always understands what happens: the icon shows the state; a notice or a sound explains every action Wakeshot takes on its own | the routine start and end of a hold show in the icon and tooltip only (Haik, 2026-11-03); every user change, `Lock`, `Hibernate`, step back and failed restore comes with a `Notify` or `Play`; a property test checks it | 2, 5 |
| 1 | Windows 11 only, x64 and ARM64, English only | section 10 targets; strings only in `text`, no translation layer | 1, 5 |
| 2 | Rust with windows-rs | section 11 | 1 |
| 3 | Core has no OS code | section 1; CI tests the core on Linux | 1, 2 |
| 4 | Claude Code from status files and transcripts; listed fields only; no hooks | section 7 | 4 |
| 5 | `busy` holds, also while background subagents run | section 7 | 4 |
| 6 | Idle with a timer due within 1 hour holds; none for the hidden loop fallback | section 7 | 4 |
| 7 | `waiting` holds only with Remote Control, at most 1 hour | section 7 | 4 |
| 8 | `shell` and open background tasks of non-terminal sessions hold while the process lives | section 7 | 4 |
| 9 | Codex CLI and Copilot CLI after v1 (Haik, 2026-11-03; v1 is Claude Code only); others later; logic may be ported from the MIT rivals with credit | section 7; a credit line in the README for each port | 4, 6 |
| 10 | `wakeshot run -- <command>` with a documented hold file | sections 4 and 6 | 4 |
| 11 | Keep awake on demand | `holds\keep-awake.json` (section 6), episode flow | 2, 4, 5 |
| 12 | Every signal checked against a fact; only the waiting hour and the battery floor limit time | `Liveness` on every hold and session (sections 5, 6, 7); the Codex rule is not yet tied to its session (section 7) | 2, 4 |
| 13 | App rules and WSL later | not in v1 | after v1 |
| 14 | No plugins, no scripting | sources are compiled in | none |
| 15 | SYSTEM plus EXECUTION; DISPLAY only for Keep screen on | section 5, power requests | 3 |
| 16 | Re-take after every resume | section 5, power requests | 3 |
| 17 | Only the lid action and battery sleep (Never); written at hold start, also on AC | section 5, power settings | 2, 3 |
| 18 | Never lower security: lid-close lock, Keep screen on idle lock, sign-in Never exception | episode flow (lock at lid close, and at a power source change with the lid closed; no override while another user is on the console); section 5, lock | 2, 3 |
| 19 | Sounds: lid closed while holding, going to sleep, battery floor; one device rule | `Cue`, `SoundPlayer` (section 2) | 1, 5 |
| 20 | Notice at sign-in after a closed-lid hold | episode flow, closed-lid report | 2, 5 |
| 21 | Battery floor | episode flow; section 5, hibernate | 2, 3 |
| 22 | No closed-lid time limit, no heat guard | no timer on holds; nothing reads temperature | none |
| 23 | Work ends with the lid closed: sleep after 5 minutes with no input (1-minute timeouts, hibernate as the fallback; Haik 2026-11-04) | episode flow, "The sleep route"; section 5 | 2, 3 |
| 24 | Save the original before any write | section 5 apply steps (a failed safety step means no write), in the core's `ownership::apply`; section 6 journal | 2, 3 |
| 25 | Restore only what is still ours | section 5 ownership and restore routine, in the core's `ownership` | 2, 3 |
| 26 | The user's latest change wins; "Restore my power settings" counts as one | `UserChanged` from the IO thread (`ownership::check`); 10 s read; `released` by key, on every plan | 2, 3 |
| 27 | Follow a plan switch | section 5 | 3 |
| 28 | Restore on work end, quit, sign-out, next start, uninstall, crash or kill; the crash gap accepted | sections 4, 5 (guard, RunOnce, gap left), 6 (hooks) | 3, 6 |
| 29 | Policy blocks writes: requests only, tell the user | section 8 | 3 |
| 30 | No window; the menu covers the CLI except `run` | section 4; CLI commands in `product.md` | 4, 5 |
| 31 | Automatic: on by default, survives restart, starts at login | `config.json`; Run value | 2, 6 |
| 32 | Keep awake times, Until I turn it off, Keep screen on; ends at sign-out or restart | Keep awake hold carries the tray's pid | 2, 4, 5 |
| 33 | Menu sketch; wording in Phase 5 | `MenuModel` | 5 |
| 34 | Icon states holding, idle, off; crisp; light, dark, high contrast; menu follows the theme | `IconState`; `tray`; `MenuView` | 1, 5 |
| 35 | Tooltip and menu name what holds | `TrayView` from hold labels | 2, 5 |
| 36 | First run: a notice that offers to pin the icon | `Notice::FirstRun`; method in Phase 5 | 5 |
| 37 | Unreadable format: "update Wakeshot" once per start per agent; unknown status counts as working | section 7 | 4 |
| 38 | Restore fails: warning notice | section 5 restore routine (outside the tray: retries, then the tray starts); section 8 | 3 |
| 39 | No Pause, no icon style, no efficiency mode, nothing that touches other apps | not in `MenuModel` | none |
| 40 | No telemetry; the log stays on the PC | section 8 | none |
| 41 | Public repo from Phase 1, fully public, MIT; cleanup and squash before launch; README; Pages at v1 | Phase 1 git steps (secret scan before each commit and push); Phase 6; Phase 8. The v1 README must say: the crash gap and every path into it (section 5, "Gap left"), one signed-in user per PC, that `winget upgrade` kills `wakeshot run`, and a credit line for each port from a rival | 1, 6, 8 |
| 42 | Velopack per user; updates only when idle; hooks restore; `run` wrapper may die on `winget upgrade` | section 6 hooks; run hold follows the child | 6 |
| 43 | winget first, scoop later, Store only if uninstall is safe | Phase 6 | 6 |
| 44 | v1 unsigned; SignPath later | Phase 6 | 6 |

## 4. Processes and threads

### Processes

| Process | Started by | Lives | Job |
|---|---|---|---|
| Tray: `wakeshot-tray.exe` | Run value at sign-in; Start menu; restore mode or guard after a failed restore | the user session | everything |
| Guard: `wakeshot-tray.exe --guard <pid> <start> <stop event>` | the tray, before its first settings write, outside the tray's process tree (section 5) | while settings are overridden; the tray starts a new one if it ends | wait on the tray; restore if it dies |
| Restore: `wakeshot-tray.exe --restore` | RunOnce at the next sign-in (the Velopack hooks run the restore routine in their own process, section 6) | seconds; up to 2 minutes when the restore fails | restore if a journal exists and its owner is dead |
| CLI: `wakeshot.exe` | the user | one command | run, Keep awake, Automatic, status, restore, open log |

- One tray per session: mutex `Local\Wakeshot.Tray`. One settings owner per machine, because power plans are
  machine-wide: mutex `Global\Wakeshot.Settings`. A tray that cannot take it (another session owns it, or the open is
  denied) holds requests only and tries again on each safety tick; when it gets it, `SettingsOwner` lets the core apply
  as at a hold start. Creating a `Global\` mutex needs no special privilege (Microsoft, "Kernel object namespaces"; a
  Medium process created one, tested, research "Power track real checks"); a denied open across users: not tested
  (no second account). The owner also applies only while its session is on the console;
  under fast user switching it restores and holds requests only (episode flow). Two users on one PC: v1 assumes one
  signed-in user at a time (Phase 1 Question 10: the default, Haik 2026-11-03 18:09).
- Every apply and every restore, in any process, runs under mutex `Local\Wakeshot.Journal`, so two processes never
  change the journal or the plan at the same time (tray and RunOnce restore at sign-in, the CLI, the hooks). A wait
  that returns `WAIT_ABANDONED` (a process ended while holding it) counts as taken: Microsoft grants ownership to the
  caller then, and the journal is whole because every write of it is atomic. A Medium process and an elevated one
  (an admin `wakeshot restore`) open and wait on each other's lock, both ways: tested with the same call on probe
  names, not with the Rust exes (research "Power track real checks").

How they talk: files only. No pipes, no sockets.

| From, to | Through |
|---|---|
| CLI to tray | `config.json` (Automatic), `holds\*.json` (run, Keep awake), `restore-request.json` (restore) |
| Tray to CLI | `state.json` |
| `wakeshot restore` | A journal whose owner (pid and start time, not `state.json`) is alive: write `restore-request.json`; the tray runs the same path as the menu item on its IO thread and deletes the file; the CLI waits up to 5 s for the file to go, then prints the result from `state.json`. Owner dead: the CLI runs the whole restore routine itself. No journal: "nothing to restore" |
| Tray and guard | the tray's process handle and the guard's (each waits on the other); the guard's own stop event; the journal |

### Tray threads

| Thread | Does | Never does |
|---|---|---|
| Loop (main) | hidden top-level window, tray icon and menu, timers, the core, power requests, lock, input idle, the settings mutex | file or registry I/O, power plan reads or writes, WinRT, sound |
| IO | runs commands in order: journal, RunOnce value, guard start, power plan reads and writes, the ownership check on each plan read, restore, config, state and hold files, log appends, notices | tray UI; the loop never waits for it, except for the restore at end of session |
| Watch | overlapped `ReadDirectoryChangesW` on the data folder (`config.json` gives `ConfigChanged`, `holds\` gives `Holds`, `restore-request.json` gives `Menu(Restore)`) and the agent folders; file reads; source and hold parsers; process opens and every process exit wait (`RegisterWaitForSingleObject`; the pool callback only wakes the watch thread) | power calls |
| System callbacks | power setting, suspend and resume callbacks | anything but posting to the loop |
| Sound | one short-lived thread per sound: the WASAPI stream (section 2) | anything else |
| Hibernate | one short-lived thread per sleep check or hibernate: checks, the sound, then `SetSuspendState` for a hibernate (returns only after resume) | |

- Other threads post boxed events to the loop window (`PostMessage(hwnd, WM_APP, ...)`), never to the thread: thread
  messages are lost while a modal loop runs, and a menu runs one (Microsoft, `PostThreadMessage` Remarks). A failed
  post (10,000 message limit, 1816) is logged as a warning; the next tick recovers. A post after the loop window is
  gone (1400, as after a quit) is logged once as Info by the IO and watch threads, since nothing is wrong
  (the `post` of `app/io_thread.rs` and of `watch/thread.rs`; `p6-int-log-wording`).
- The loop window handles `WM_CLOSE` (also `taskkill` without `/F`) as Quit from the menu. A handler call that
  re-enters (a runner that pumps messages, such as the modal menu) queues its event; the outer call feeds it to the
  core after its own batch.
- Power events come through `DEVICE_NOTIFY_CALLBACK` callbacks (tested in the spikes), not `WM_POWERBROADCAST`
  (window registration not tested).
- At `WM_ENDSESSION` the loop puts the restore at the front of the IO queue; the IO thread runs it next and drops
  queued notices and queued `Apply` commands. After `Session(Ending)` the core issues no `Apply`, since the loop keeps
  handling posted events until Windows ends the process. A cancelled end (`WM_ENDSESSION` with FALSE,
  `Session(EndCancelled)`) lifts both. The loop waits up to 4 s for the restore, then returns; Windows ends the process
  after that. Tested with sent messages on an active plan copy: `WM_QUERYENDSESSION` answered TRUE, and the
  `WM_ENDSESSION` send returned after 39 and 56 ms with the copy restored (research "Integration real e2e"). A real
  sign-out: not tested (checklist 6).

### Tray startup

1. (Phase 6) `VelopackApp::build().run()` first; it exits for a hook.
2. Mode from the arguments: tray, `--guard`, `--restore`, `--dry-run`.
3. `--dry-run` uses its own data folder (`%LocalAppData%\Wakeshot\dry-run`), mutex names ending in `.DryRun` and
   `DryRunPower`. It never loads, writes or deletes the real journal, and never touches Run or RunOnce.
4. Take `Local\Wakeshot.Tray`; if another tray has it, exit 0.
5. Panic hook: log, then `std::process::abort()`, in every build profile. `panic = "abort"` is set only for release;
   dev and test builds unwind, and a panic that ends only the IO thread would leave the tray alive with nothing
   restoring. The hook runs before any unwinding (Rust, `std::panic::set_hook`). Only the tray's `main` sets it, so
   tests that need unwinding (proptest shrinking, most likely; not verified) are not affected. No restore in the hook:
   it would run beside the IO thread, which may be mid-apply. The guard restores when the tray dies; the next start
   covers the rest.
6. Restore if a journal exists and its owner (pid and start time) is Gone (section 5, "Liveness"). Unknown: retry on
   each tick. A journal that does not read (a read error, not a parse error) is read once more after 500 ms; still
   unread, `Started` carries `journal_unread`, and the core retries it on each tick as a leftover restore, for two
   minutes, then resets it as a lost journal (section 3, "A journal that cannot be read"; `app/startup.rs`,
   `p2-6-unread-journal`, `p2-6-record-lost`). A tray that writes the journal makes itself the owner and keeps the
   entries already in it (section 5, Apply step 3).
7. Try `Global\Wakeshot.Settings`.
8. Window, icon (retry until the taskbar exists; re-add on `TaskbarCreated`, tested), notifications, watchers, first
   reads, then `Started`, then `Session(Locked)` when the session was locked at start, then `JournalCorrupt` when a
   corrupt journal was set aside at start or left by another process (the core resets it as a lost journal), then
   `Updated` when Velopack restarted the tray. After Quit: the IO
   thread runs what is queued (up to 10 s), the update and watch threads stop, the icon and the requests go, the
   mutexes are released.

### CLI

Commands: `product.md`, UX (Haik, 2026-11-03). `wakeshot run -- <command>`: starts the child with the same console,
then writes `holds\run-<pid>.json` with the child's pid and start time, waits, deletes the file, exits with the child's
code, whole (also above 255, such as `0xC000013A` after Ctrl+C). Its own exit codes are the ones GNU `env` and
`timeout` use, so a script can tell them from the child's: 127, the command is not found; 126, it cannot start (a
file kind it does not start, such as `.ps1`, or a failed start); in both cases no hold file is written. 125, it lost
the child (the wait failed; the hold file stays, and the tray deletes it once the process is gone). From
`wakeshot-cli/src/run.rs`. It ignores Ctrl+C itself; the child gets it from the console. Keep awake and Automatic
write their files; `restore` is in the table above; `status` reads `state.json` and says "not running" when the tray's
pid and start time are not alive.

## 5. Windows layer

### Power requests

- One request object per tray (`PowerCreateRequest`, reason "Wakeshot: keeping the PC awake for running work").
  SYSTEM and EXECUTION together; DISPLAY only for Keep screen on.
- After every resume (`PBT_APMRESUMEAUTOMATIC` through `PowerRegisterSuspendResumeNotification`): clear, then set,
  each type that is on, same handle. That is the tested form (research, "Resume", H4). Taking a new request before
  dropping the old one is not used, so it needs no test. Re-take on battery: Phase 3.
- Never judge a request by `powercfg /requests` or `SystemExecutionState`: the first lists requests Windows no longer
  honors, the second gives no warning before the battery rule (research, "Resume", "Battery rule").

### Power settings

Wakeshot edits the active plan in place, never a copy: Settings writes battery sleep and the battery lid action to the
user's plan even while a copy is active, and a copy loses other changes made during a hold (research, "What Windows
Settings writes").

A hold writes:

| Key | Target | Skipped when |
|---|---|---|
| Lid action AC | 0, Do nothing | the live value is already 0, or the user released it in this episode |
| Lid action DC | 0, Do nothing | same |
| Sleep after DC | 0, Never | same |

The sleep route (section 3) writes, once the countdown after a closed-lid work end is due:

| Key | Target | Skipped when |
|---|---|---|
| Sleep after AC and DC (`STANDBYIDLE`) | 60 s | the live value is already 60 s or less and not Never, or the user changed it in this countdown |
| Screen off AC and DC (`VIDEOIDLE`, `GUID_VIDEO_POWERDOWN_TIMEOUT`) | 60 s | same |

- Written when holding starts, also on AC, so an unplug or a short AC drop on a USB-C dock never meets the battery
  rule first (research, "AC can drop for a few seconds while docked"). Sleep DC Never kept requests honored for 16
  minutes on battery with the lid open (research, "Battery sleep Never stops the battery rule"). With the lid closed:
  not tested (Phase 3). A finite value would only move the point where Windows ends the requests: a time limit, which
  `product.md` rules out (signals are checked against facts, not capped by time).
- A hold never touches AC sleep: on AC a held request blocks idle sleep (research, "Resume", H run 1). Only the sleep
  route writes it, with no hold running.
- A timeout target other than Never is a ceiling: the apply keeps a live value at or under it, and a hold's battery
  sleep Never that a failed restore left over a user's own shorter value goes back to that value, not to 60 s.

Apply, on the IO thread, under `Local\Wakeshot.Journal`:

1. Read the active plan. The read fails: write nothing, report `SafetyNet`. Not the `plan` of the command: write
   nothing and report `PlanChanged`; the next plan read handles the switch.
2. `PowerSettingAccessCheckEx` for each key; read the current values. A key whose live value already is the target
   and has no journal entry holds the user's own value: skip it.
3. The safety net, journal last: the RunOnce value; a new guard if none runs or the current one's stop event is set
   (it is on its way out); then the journal file (atomic, write-through), with this tray as its owner. A key that
   already has an entry for this plan that is still ours (live equals `written`, or marked `restored` with live equal
   to `original`; a restore failed) keeps its `original` and its `restored` mark: the hold takes the entry over, and
   step 6 clears the mark only once our value is in force again. An entry not marked whose live value is its
   `original` was never written: a new entry replaces it. Any other live value is the user's change since our write,
   which no ownership check has seen yet (one runs every 10 s): the key is skipped and its entry kept as it is, so the
   next check reports it and the core releases the key; writing over it would break "the latest change wins at once"
   (power safety review finding 10). A new entry has the live value as `original` and is saved marked `restored`, so
   a run that never writes it (a stop, or a failed write whose own restore fails too) leaves nothing the next check
   reads as the user's change (power safety review finding 9).
4. Any failure in steps 2 or 3: write nothing (decision 24); if the journal held no known entry (a read error does
   not count), take back the RunOnce value and the guard this apply added; report `SafetyNet`. The core holds
   requests only for the episode (for the run when the access check is denied, section 8) and shows one notice. The
   journal comes last, so a failure never leaves entries for keys that were never written.
5. `PowerWrite*ValueIndex` for each key whose live value is not the target; read back.
6. Read the active plan again, right before the call. The same as in step 1: one `PowerSetActiveScheme` with that
   GUID, never a stored one (a write is stored at once but applied only by it, research "Stored at once, applied only
   on `PowerSetActiveScheme`"). Success: clear the `restored` marks of this plan's entries it took over or added (one
   journal write; if it fails the marks stay, which only hides from the ownership check a user change back to exactly
   `original`, until the next restore, and that restore handles a marked entry right). Changed: skip the call and
   report `PlanChanged`; the writes went to a plan that is now inactive, the journal holds them, and the plan switch
   path restores them. A write, the read-back, this second plan read or the call fails: run the restore routine on
   this plan's entries (`plan: Some`) and report `Write`. So an entry leaves the journal only after a successful
   `PowerSetActiveScheme`, from a complete plan list, or on a successful read that shows a value we never wrote or the
   user's own.

Every stop point keeps the user's values reachable (an entry taken over in step 3 is E; one new in this apply is F):
before step 3's journal write, nothing changed. After it: F reads "marked, live equals `original`": the restore drops
it after its call (harmless: nothing of ours was written) and the ownership check keeps it, so it is never taken for a
user change; E keeps its state, so a restore still finishes it. After a write in step 5: F and E read "marked, live
equals `written`": both restored. After the call, before the marks are cleared: the same, restored again; harmless.
The restore routine's own stop points are covered by its write-ahead (step 4 below). The full walk is in card
`p1-design-verify-fixes`, Log.

Ownership. Our writes and the user's look the same to every detector (research), so a key is ours exactly while its
live value equals what we wrote. The IO thread decides it, because it owns the journal and runs commands in order. On
each `ReadPlan` it reads the active plan id first, then each journal entry's value from that entry's own plan. A value
that differs from `written`: drop the entry, post `UserChanged { plan, key }`. A read error: keep the entry, post the
failure. An entry marked `restored` is also ours at its `original` (a restore waits for its call, or an apply for its
write); any other value is the user's, as restore step 3 reads it, so a mark never hides a change to a third value
(power safety review findings 9 and 10). So a read taken before an `Apply` never sees that apply's entries, and a
plan switch never looks like a user change. Accepted: a user who sets exactly
our value is not seen (since Settings already shows our value, that needs two changes inside one 10 s read); a user
change between Apply steps 2 and 5, a few calls apart, is overwritten, and the restore puts back the value read in
step 2; a user change between restore steps 3 and 4 (the read and the write of `original`, with the mark save
between them) is overwritten by `original` (research, "The restore routine overwrites a user change between its steps
3 and 4"). When the check drops the last entry, the clean-up of restore step 6 runs.

- `ReadPlan`: the core issues it every 10 s while anything is owned, and at any tick once the last read is 10 s old or
  more, so on each safety tick (docked use and the Keep screen on lock need the user's values). No registry watch: its
  key path is not a documented API.
- Plan switch (`GUID_ACTIVE_POWERSCHEME`, 4 ms, tested, or a read with a new active id): restore the old plan's keys
  (an inactive plan needs no apply), then apply to the new plan. "No apply" rests on two points, both tested on the
  laptop (research "Power track real checks"): a write to an inactive plan changes only that plan (section 9), and
  activating the plan later applies its stored values.
- Apply and restore run at every hold start and end, which is each agent turn, each with a `PowerSetActiveScheme`.
  Side effects of that many calls, from 50 cycles on the laptop (checklist 15, research "Power track real checks"):
  brightness unchanged, no error or warning in the System and Application logs, one System log event 12 per call. Not
  verified: a visible flicker and other apps' setting callbacks; the cycles ran on battery only.

Restore routine (`ownership::restore`, always called through `power::journal::restore_locked`), the same in the tray,
guard, restore mode, Velopack hooks and the CLI, always under `Local\Wakeshot.Journal`:

1. Load `journal.json`. Missing: nothing to do. Any other read error: this run fails. No entries (a stop before the
   clean-up): the clean-up of step 6. Corrupt (read in full, but it does not parse): move it to
   `journal.json.corrupt`; the routine changes no value. In the tray the journal is then lost (`JournalCorrupt`): the
   reset puts each key that holds a value Wakeshot writes back to Windows' default, with one notice ("A journal that
   cannot be read", section 3). The guard, restore mode, the install and updated hooks and `wakeshot restore` only set
   the file aside, and the tray's next start resets; the uninstall hook runs the reset itself, since no tray starts
   after it (section 6). The log holds the old values (every settings write is logged with old and new values). An
   apply that finds the file corrupt writes nothing and fails with `SafetyNet(JOURNAL_CORRUPT)`; the core then sends
   one `Restore { plan: None }`, once per corrupt file and not per apply, so this step sets the file aside and
   `JournalCorrupt` reaches the user (orchestrator, 2026-11-03).
2. Read the active plan id and the whole plan list: `PowerEnumerate` until it returns 259 (`ERROR_NO_MORE_ITEMS`, the
   end code on the test laptop). If the id read fails, or the list stops on any other code, this run fails before any
   entry is dropped or written: "plan deleted" is judged only from a complete list. A `Restore { plan: Some(p) }`
   takes only `p`'s entries from here on.
3. For each entry, read its live value from its own plan:
   - its plan is not in the list: drop it (plan deleted);
   - the read fails: keep it; this run fails;
   - not marked `restored`, live equals `written`: to restore;
   - marked `restored`, live equals `written`: the write never happened (a stop after step 4's mark), or an apply
     took the entry over and wrote it again (Apply step 3): to restore again;
   - marked `restored`, live equals `original`: keep it for step 5;
   - otherwise: drop it (the user's, or never written).
4. Write ahead: mark every entry to restore `restored` and rewrite the journal (write-through); that write fails: this
   run fails, nothing written. Then write `original` for each; a write fails: keep the entry, this run fails. So a stop
   between the write and the mark cannot happen, and a stop after the mark is caught by step 3.
5. If any entry is marked `restored` with its `original` stored, read the active plan id again right before the call.
   The read fails: keep them, this run fails. Entries on any other plan need no apply (the two points under "Plan
   switch"): drop them. Entries on that plan: `PowerSetActiveScheme` with it; success: drop them; failure: keep them,
   this run fails.
6. Whenever the last entry is dropped, on any path (this routine or the ownership check, each only after a successful
   read): delete the file and the RunOnce value, set the current guard's stop event.
7. A failed run keeps the journal and reports. The tray retries at a tick, at most once per 10 s (episode flow), and
   warns once.
   Restore mode and the guard retry every 10 s for 2 minutes. Each retry reloads the journal and goes on only while
   its owner is still the dead process it started for; another tray took the journal over: exit. In restore mode an
   owner whose liveness is Unknown counts as a failed run, so it is retried. After 2 minutes they start the tray, which
   keeps retrying and shows the warning (decision 38; Phase 1 Question 12: the default, Haik 2026-11-03 18:09). The
   Velopack hooks must not show UI (`research/distribution.md`): they retry within an inner limit, log a failure and
   keep the journal. Velopack kills a hook that runs past its own limit; measured: install 30 s, updated 15 s,
   uninstall 60 s, where the Velopack docs say 30 s (research "Velopack checks for Wakeshot", point 6). The inner
   limits (`hooks.rs`, from `p6-velopack-facts`): 25 s for install and uninstall, 10 s for updated, since the updated
   hook always runs a just-extracted exe, which took up to 1.7 s to start. Each limit counts from the hook's start, so
   the registry steps' time comes out of it; a failed run is tried again every 2 s while another try fits, and one try
   always runs (section 6). `hooks.rs` is built and tested with fakes, and `velo.rs` calls it from Velopack's hook
   callbacks. Real installs, updates and uninstalls on the test laptop ran it and put a journaled value back (research
   "Install end to end checks" and "Integration update e2e").

Liveness, one mapping for hold files, the journal owner, the guard's tray and agent sessions: open the process with
exactly `SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION` and compare its creation time. On `0x80070057` (no such
process), `0x80070005` (denied) or any other open error, read the system process list: alive when a process with that
pid and creation time is listed (research, "Reading elevated Claude Code sessions"). The list call fails: Unknown.
Unknown keeps the last state, is logged and retried on the next tick, and never deletes a file or starts a restore.

Guard:

- A separate process outside the tray's process tree, so Task Manager does not list it under the tray and End task on
  the Wakeshot group does not end it: the tray starts it with `PROC_THREAD_ATTRIBUTE_PARENT_PROCESS` set to the shell's
  Explorer (the process of `GetShellWindow`). A Medium process may use Explorer as the parent (tested, research
  "Process module checks"). Not verified: how Task Manager then groups the guard (checklist 4). If it fails: a
  short-lived launcher that starts the guard and exits; if End task still ends both, the gap list below covers it.
- The tray keeps the guard's handle from `CreateProcess`, waits on it, and starts a new guard if it ends while anything
  is owned.
- Each guard gets its own stop event, `Local\Wakeshot.GuardStop.<tray pid>.<n>`, named on its command line, so an
  event left set never stops the next guard.
- Opens the tray with exactly `SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION` (rights tested, research "Reading
  elevated Claude Code sessions"), checks the start time, waits on the handle and on its stop event. Tray gone with a
  journal left that names this tray as its owner: run the restore routine (with the retries above), log, exit. A
  journal that names another tray: exit. A corrupt journal names no one, so the guard leaves it; the RunOnce value is
  still set, and the next sign-in's restore mode sets it aside (restore step 1). Stop event: exit. Tested with the
  tray ended by pid during a hold on an active plan copy: the copy was back 63 to 87 ms after the end, the RunOnce
  value gone, the guard's exit code 0 (research "Integration real e2e").
- No hang check (Phase 0 close; change it when a tray hang shows up in a test or a report).
- Velopack kills the guard with the tray on update, uninstall and Setup.exe over an install; the hooks restore
  (section 6).

Logon restore: while anything is owned, `HKCU\Software\Microsoft\Windows\CurrentVersion\RunOnce` value
`WakeshotRestore` runs `wakeshot-tray.exe --restore` at the next sign-in, also with start at login off. Windows deletes
the value before it runs the command (Microsoft, "Run and RunOnce Registry Keys"), so the restore mode must not write
to that key, and its retries (step 7) are the only second chance. Picked over a logon task (Nightcrew's way): one
value, no task XML. Not tested (Phase 3).

Gap left. In these cases nothing restores for a while, so the lid action stays Do nothing and battery sleep Never, and
a closed laptop at the sign-in screen does not sleep (accepted by Haik, 2026-11-03; the README says so). Until the next
sign-in:

- an OS crash or a power cut during a hold: a restore before sign-in would need a boot-time task, and that needs admin
  (not verified);
- the user picks "Shut down anyway", or Windows ends the session, before the end-of-session restore finishes;
- End task ends the tray and the guard together, if the guard's own parent does not keep them apart (checklist 4).

Longer, until Wakeshot next starts (a manual start, or the next sign-in when start at login is on), because Windows
deleted the RunOnce value before restore mode ran:

- restore mode fails for 2 minutes at sign-in and the tray it starts fails to start;
- restore mode panics (it aborts, with no retry and no tray start).

Until a reinstall, with no warning: an uninstall during a hold whose restore fails for the hook's whole 25 s. The exe
is gone, hooks show no UI, and the uninstall removes the RunOnce value; the journal stays, so a reinstall's install
hook restores.

Until the next tray start resets them to Windows' defaults (`p2-6-record-lost`): a journal file that reads but does not
parse, met by the guard, restore mode, the install or updated hook or `wakeshot restore` (restore step 1). It is moved
aside; the log holds the old values. The uninstall hook resets at once, since no tray starts after it
(`p2-6-sleep-route-fixes`, review major 3). The reset covers the sleep route's 1-minute timeouts too, so a lost journal
while they were in force ends at the defaults (review major 2). A `journal.json.corrupt` whose rename to `.seen` fails
is reported once and then deleted, so it is reset at most once (when it cannot be deleted either, each start reports
it): reported at each start, it would reset each time, by then maybe over values the user set (review minor 6 and its
verify). One left by `wakeshot restore` while a tray ran is reported only at
that tray's next start; the late reset is kept, since nothing else would ever take back the values that file named.

### Lock and hibernate

- Lock: `LockWorkStation` (tested, returns in 0.5 ms). When: episode flow. The exception "no lock when sign-in after
  sleep is Never" waits for a test that shows which stored value means Never. There is no test account on the test
  laptop (Haik, 2026-11-03), so v1 always locks at lid close while holding.
- Hibernate, on its own thread: check again right before the call: the lid is closed (a fresh lid registration; its
  first callback came at once in the spike, research "Lid notification", but Microsoft says it comes only once a lid
  device and its state are known, so the thread waits at most 2 s and an unknown lid never hibernates), input idle
  still at least 5 minutes with no resume since the countdown started, and for the Floor reason still on battery and
  under the floor. Then SeShutdownPrivilege (works from a process started by Explorer, tested) and
  `IsPwrHibernateAllowed`, then the "going to sleep" sound (Phase 5 sets how long the thread waits for it; a failed
  sound is logged and the run goes on), then the lid once more (open or no answer: no hibernate, so a user who hears
  the sound can open the lid to stop it; an agent turn that starts during the sound is not seen, since nothing
  cancels the thread), then `SetSuspendState(TRUE, FALSE, FALSE)` (hibernates here, returns only after resume,
  tested). The two checks come before the sound, so a PC with hibernate off never plays "going to sleep" and then
  stays awake (card `p3-hibernate`). Not allowed or failed: a notice; the hold's settings are already
  restored and the sleep route's 1-minute timeouts stay, so Windows most likely sleeps the laptop once nothing holds
  it (inferred; with the lid closed not tested, checklist 2); with the lid still closed the core then locks it
  whenever it is left idle (section 3, "Countdown"). Since `p2-6-sleep-route` this is the fallback 5 minutes after the
  short timeouts.
- Sleep check (`Command::SleepCheck`, `power::hibernate::run_sleep_check`), on the same thread: the lid, input idle
  and the floor as above, the sound, the lid once more; no privilege, no hibernate check, no call. An unknown lid is
  logged and the check goes on. It answers `Done(SleepCheck)`; on `Ok` the core writes the short timeouts.
- One hibernate thread at a time, a sleep check included: the dispatcher starts none while one runs, from its start to
  its `Done`, since a second could put the PC down again as soon as it wakes. The thread's last act posts that end; a
  post the full queue refuses (1816) is tried again every 1 s, up to 60 tries, and any other error ends the tries with
  a warning. A thread that cannot start is fed as `Done(Hibernate(Failed))` at once, not posted, and a sleep check that
  cannot start as `Done(SleepCheck(Ok))`: the core checked the lid, idle and floor before it asked, so only the sound
  and the second look are lost (`app/dispatch.rs`, `app/tray_mode.rs`; `p6-int-wiring-fixes` item 2).

### OS events

| Signal | API | Tested here |
|---|---|---|
| Lid | `PowerSettingRegisterNotification(GUID_LIDSWITCH_STATE_CHANGE)`, callback | yes: Medium token, no window, first value at once |
| AC or battery; battery percent | `GUID_ACDC_POWER_SOURCE`; `GUID_BATTERY_PERCENTAGE_REMAINING` | yes |
| Active plan | `GUID_ACTIVE_POWERSCHEME` | yes, 4 ms |
| Suspend, resume | `PowerRegisterSuspendResumeNotification` | yes: `PBT_APMSUSPEND`, `PBT_APMRESUMEAUTOMATIC` |
| Lock, unlock; console left and back | `WTSRegisterSessionNotification`, `WM_WTSSESSION_CHANGE` (`WTS_SESSION_LOCK`, `_UNLOCK`, `WTS_CONSOLE_DISCONNECT`, `_CONNECT`); `WTSGetActiveConsoleSessionId` at start | no (Phase 3) |
| Sign-out, shutdown | `WM_QUERYENDSESSION`, `WM_ENDSESSION` on the hidden top-level window | no (Phase 3) |
| Input idle | `GetLastInputInfo` on each tick | yes |
| Folder changes | `ReadDirectoryChangesW`, overlapped | yes: one folder (`spikes/claude-watch`, `watch_dir`); several folders on one thread, the data folder, `holds\` and each agent folder (`watch/thread.rs` tests, card `p4-watch-thread`; at most 60 folders, `watch/dir.rs` `MAX_FOLDERS`) |
| Process exit | `RegisterWaitForSingleObject` on a process handle, registered by the watch thread | the wait itself yes; the pool wait not tested |
| Theme | `WM_SETTINGCHANGE` "ImmersiveColorSet"; compare registry values after the broadcasts settle | yes |
| Taskbar restart | `TaskbarCreated` | yes |

### Tray, notices, sounds

- Icon: `Shell_NotifyIcon` with `NIF_GUID` and version 4 (tested). The exe has a Common Controls 6 manifest (needed
  for `LoadIconMetric`) and per-monitor v2 DPI (tested). Reload the icon on `WM_DISPLAYCHANGE` and `TaskbarCreated`,
  not on `WM_DPICHANGED`: the hidden window never got that one when the primary display changed (research, "Tray, menu
  and notifications", "`LoadIconMetric` follows the system DPI"). Icon art for each taskbar mode, high contrast and
  scale: Phase 5. Pick the glyph from the registry (`SystemUsesLightTheme`), not from cached uxtheme values.
- Menu: the `MenuView` seam (section 2), built fresh on each open. In v1 it opens only from a click on the icon. A menu
  opened by code (a hotkey, a notice click) may not close on an outside click (research, "Menu details"), so any such
  path needs that fix and a check on screen (Phase 5).
- Notices: `BalloonNotifier`, tray balloons (`NIF_INFO`). Tested: Windows 11 shows them as toast banners with no app
  registration, while an unregistered toast shows no banner. Windows files a balloon under
  `NotifyIconGeneratedAumid_<NotifyIconSettings id>` and does not store it, so its on/off switch most likely follows
  the icon's settings entry and uninstall has no stored balloon to clear. Never count balloon events:
  `NIN_BALLOONTIMEOUT` came twice in the same millisecond (both in research, "Balloon"). A successful call never proves
  the user saw a notice. A balloon click (`NIN_BALLOONUSERCLICK`): not tested. A notice the shell refuses (the icon
  is not added yet, as at a sign-in before the taskbar exists, or Explorer restarts) is kept, in order and once each,
  and shown again through the IO thread when the icon is added (`BalloonNotifier::icon_added`); one shown again skips
  the core's rule that holds notices while the session is locked (`p6-int-wiring-fixes` item 1).
- Change to registered toasts when a notice needs buttons. Then uninstall must clear the stored toasts
  (`ToastNotificationManager.History.Clear`, not tested) before deleting the app id keys, and Phase 5 checks the toast
  icon's colours on screen (R and B showed swapped; the method is in research.md) and tries a different `DisplayName`
  once to see which name Windows shows.
- Sounds: the `SoundPlayer` seam and its device rule (section 2).

## 6. Files

The data folder is `%LocalAppData%\Wakeshot`, never inside Velopack's install folder
`%LocalAppData%\HaikAsatryan.Wakeshot` (pack id `HaikAsatryan.Wakeshot`; both names Haik, 2026-11-03), which Setup.exe
over an install deletes with no hook (research, "Distribution"). The pack id cannot change after the first release.
Velopack accepts the dot: pack, install, updates, Setup.exe over an install and uninstall all worked with it (tested,
research "Velopack checks for Wakeshot", point 1).

Every file is UTF-8 JSON with `"version": 1`. A reader takes its own version and older ones. A file with a newer version
is ignored and logged, and never written over (the tray's menu then changes Automatic for this run only, and `wakeshot
automatic` refuses with a message); the journal is the one exception (below). The reader reads `VersionProbe` first, so
a newer file whose fields changed shape is still recognized as newer. Writes go to a temp file, flush, then
`MoveFileExW(REPLACE_EXISTING | WRITE_THROUGH)` (Nightcrew's way). That gives 5 while a reader has the target open, so
`store::write_atomic` then renames by handle with POSIX semantics (`SetFileInformationByHandle(FileRenameInfoEx)`), as
Rust's `std::fs::rename` does; that works while the reader shares delete (tested, research "Store checks"). What a power
cut right after the move leaves: not tested.

| File | Written by | Content |
|---|---|---|
| `config.json` | tray (menu), CLI | `{ "version": 1, "automatic": true }` |
| `holds\keep-awake.json` | tray (menu), CLI | hold format; `pid` and `procStart` are the tray's, so Keep awake ends with the tray (sign-out, restart) |
| `holds\run-<pid>.json` | `wakeshot run` | hold format; `pid` is the child's, so the hold outlives a wrapper killed by an update |
| `holds\<any>.json` | any tool or script | hold format (the user doc is `docs/hold-files.md`) |
| `restore-request.json` | CLI | `{ "version": 1 }`; the tray restores and deletes it |
| `state.json` | tray, on change (1 s debounce) | tray pid and start, Automatic, what holds, what is overridden, stepped back |
| `journal.json` | IO thread and every restore, under `Local\Wakeshot.Journal` | owned keys (below) |
| `logs\wakeshot.log` | every process | text log (section 8) |

Hold format:

```json
{ "version": 1, "pid": 4242, "procStart": "134381211391959336",
  "reason": "nightly backup", "expires": "2026-11-03T18:00:00Z", "screen": false }
```

- `pid` is required. `procStart` is optional: the process creation time as a FILETIME decimal string, the same form as
  Claude's status files. If it is missing, the process counts only if it started at or before the file's last write
  time: a process that started later cannot be the writer, so a reused pid never counts. The check keeps no state, so
  it holds across a tray restart.
- `reason` (optional) shows in the tooltip and the log, cut to 60 characters. `wakeshot run` writes only the program
  name, never the full command line, which may hold secrets.
- `expires` (optional, UTC). `screen` (optional) counts only in `keep-awake.json`: `product.md` holds the display
  only when the user asks through Keep screen on.
- `HoldFile.expires` stays text. `holds::file` parses it (RFC 3339, `Z` or an offset) and formats it for Keep awake
  (UTC, whole seconds) in its own code: no date crate.
- A hold counts while its process is alive and before `expires`. A file whose process is Gone is deleted; Unknown
  keeps the file and the last state (section 5, "Liveness").

Journal format:

```json
{ "version": 1, "owner": { "pid": 9120, "procStart": "134381211391959336", "session": 1 },
  "entries": [ { "plan": "381b4222-f694-41f0-9685-ff5bb260df2e", "key": "lidAc", "original": 1, "written": 0,
                 "restored": false } ] }
```

- An entry exists while the key is ours, and while its restore waits for `PowerSetActiveScheme` (`restored`). An empty
  journal is deleted.
- One file, no second copy: the atomic write with write-through covers a torn write. Change it when a lost or corrupt
  journal shows up in a test or a report.
- The entry fields above are frozen: a later version only adds fields, and every version restores the entries whose
  `key` it knows, also from a newer journal, so a downgrade (winget can install an older version) still restores.
- Keys: `lidAc`, `lidDc`, `sleepDc` (a hold), and since `p2-6-sleep-route` `sleepAc`, `videoIdleAc`, `videoIdleDc`
  (the sleep route's 1-minute timeouts; `sleepDc` too, with `written` 60). A journal from before them loads as it
  did. A version before them keeps their entries as keys it does not know and restores nothing of them, so a
  downgrade in those minutes leaves the 1-minute timeouts until a newer version runs.

Registry values Wakeshot writes, all under HKCU, all removed by uninstall: `...\CurrentVersion\Run\Wakeshot` (start at
login, set by the install hook); `...\CurrentVersion\RunOnce\WakeshotRestore` (only while overriding). Windows itself
writes `Control Panel\NotifyIconSettings\<id>` for the icon and a `NotifyIconGeneratedAumid_<id>` key for balloons.
`store::AutoStart` is the one code that writes or removes the Run and RunOnce values (the hooks and the journal call
it; review m12), and at uninstall it also removes the Run value's `StartupApproved` switch value
(`p6-uninstall-startup-approval`); `install` keeps only the Path entry.

Velopack hooks (Phase 6, `hooks.rs`): the registry steps come first and the restore last, so a restore that hangs until
Velopack's kill never leaves a Run value or a Path entry that names a deleted folder. Install (also Setup.exe over an
install) writes the Run value and adds the CLI folder to the user Path; updated adds only the Path entry, so a Run value
the user removed stays removed. Both then run the restore routine if a journal exists and its owner is dead. Uninstall
removes the Run value, the RunOnce value and the Path entry first (the exe they name is being removed, so RunOnce goes
even when the restore then fails), and right after the Run value its `Explorer\StartupApproved\Run\Wakeshot` value:
Windows writes that one for the Settings > Apps > Startup switch, install and updated leave it, so an off switch
outlasts `winget upgrade`, and uninstall removes it, so a later install starts as on a new PC
(`p6-uninstall-startup-approval`). Then it restores, then cleans the data folder: after a restore that worked or found
no journal, `%LocalAppData%\Wakeshot` goes whole; otherwise `journal.json` and `logs\` stay (section 5, "Gap left"). A
failed restore is tried again every 2 s until the hook's inner limit, counted from its start: 25 s for install and
uninstall, 10 s for updated (section 5, restore step 7). An owner that is alive or Unknown, or a journal that does not
parse or holds only a newer version's entries, ends the tries at once with one log line; the journal stays as it is,
but for one that does not parse at uninstall: no tray starts after it, so the hook runs the tray's lost-journal reset
(`power::journal::reset_locked`, `damaged`), each lid, sleep and screen value at exactly Wakeshot's goes to the plan's
Windows default and the file goes aside, with log lines only, and `logs\` stays for the old values
(`p2-6-sleep-route-fixes`, review major 3). The
obsolete hook only logs. Updates apply only when nothing holds and nothing is overridden. Tested on a real install with
a local feed: a downloaded update did not apply for 120 s while Keep awake held; after Keep awake off the plan copy came
back, then the update applied and Velopack started the new tray. Setup.exe over the install and an uninstall, each
during a hold, put the copy back through the hook (checklist 14; research "Integration update e2e"). The `current`
folder keeps its path across updates, though the folder itself is new each time, so Run, RunOnce and Path can point at
it (tested, research "Velopack checks for Wakeshot", point 3). Accepted (ship Question 1, Haik 2026-11-03 18:09; the
README says so): `winget upgrade` runs Setup.exe silent, which does not start the app, and a hook cannot start it
(`research/distribution.md`), so after such an upgrade Wakeshot is off until the next sign-in or a manual start.

## 7. Sources

```rust
pub struct SessionFacts {
    pub key: SessionKey,          // source plus its own id
    pub liveness: Liveness,       // Process { pid, start: FileTime }
                                  // | AnyProcess { image: &'static str, lock_file: PathBuf }
    pub activity: Activity,       // Working | Waiting { since_wall_ms: Option<i64>, remote } | Idle
                                  // since_wall_ms None: no statusUpdatedAt in the file; the parser has no
                                  // clock, so the core keeps the time it first saw waiting
    pub timers: Vec<i64>,         // next due times, wall ms: jobs, wakeups, Monitor timeouts
    pub open_tasks: u32,          // background tasks with no known end, seen open in the transcript
    pub label: &'static str,      // "Claude Code"
    pub format: FormatHealth,     // Ok | Unreadable(&'static str) | UnknownValue(field, value)
                                  // Unreadable takes a fixed text, so file content cannot reach the log
}
```

- The watch thread owns the parsers: it feeds whole status files and appended transcript bytes, asks each source
  which files it wants, checks liveness (the adapter, never the parser), and posts one snapshot per source.
- The core turns facts into holds and deadlines. Parsers never decide policy, so one fake clock tests all timing.
- Unknown format: `Unreadable` (still not parsed after one retry 200 ms later) gives no signal for that session.
  `UnknownValue` for Claude's `status` counts as working while the process lives (`product.md`, `CLAUDE.md`). Both
  show the "update Wakeshot" notice once per Wakeshot start for each agent, plus a log line; the tooltip does not
  repeat it (Haik, 2026-11-03).
- Parsers read only the listed fields. Prompts and other text are never stored or logged. Claude's `.key` files are
  never opened: names are filtered before any open, as in the spike.

Claude Code (Phase 4) follows `research/claude-code-signals.md`, "Recommended detection rules" 1 to 9, with these
rulings:

- A session counts only while its process lives: pid plus start time equal to `procStart`, wait on the handle (rule 2).
- Transcripts are read on each status file change and on the safety tick, not watched: each transcript line that
  matters (timer created, task closed, `/clear`) comes with a status write; an Esc cancel writes nothing anywhere.
- Timer margin: due time plus 2 minutes. Fires come 0.1 to 1 s late and the turn then shows `busy`
  (`research/claude-code-signals.md`, "Live test results", test 3).
- Waiting limit: `statusUpdatedAt` plus 1 hour, wall time (the time Wakeshot first saw `waiting` if the field is
  missing), and at most what was left of that hour when Wakeshot first saw the wait, in run time (`mono_ms`), so a
  clock set back cannot stretch it; "dialog open" counts as idle.
- Rule 11 (transcript growth when the status file is missing or unknown) is dropped. It contradicts the
  unknown-format rule above.
- `claude --resume` in a new process reuses the transcript and writes no line for it (live test,
  `research/claude-code-signals.md`, "Phase 4 live shapes", "Resume findings"). So lines older than the session
  process's start (`procStart`) are an earlier process's, and the transcript parser applies the resume rule at that
  time: `CronCreate` jobs come back (a recurring one counts its next fire from the resume and is dropped at 7 days
  old; a one-shot whose time passed is dropped), while wakeups and background tasks from before it are dropped.

Codex CLI and Copilot CLI (after v1, Haik 2026-11-03; their cards are `later` in Phase 4): the same trait. Liveness
comes from `research/other-agent-signals.md` (Codex files hold no pid: "a `codex.exe` is alive and the thread lock file
exists"). That Codex rule can outlive its session: a crash leaves the lock file until the next Codex start, and a shared
`codex app-server` can live for days. Phase 4 maps the lock file to the process that holds it open (for example Restart
Manager `RmGetList`; not verified for this lock) and checks that pid and start time; until then the Codex rule is marked
"can outlive its session". Each agent gets one live test (product). `wakeshot run` and hold files are not a source; the
watch thread parses them into `Event::Holds`.

## 8. Errors and logging

The core never fails. The Windows layer returns `Result<T, WinError>` (`WinError { call: &'static str, code: u32 }`);
every failure that changes behavior comes back as `Event::Done` or `Event::Plan(Err)`, and the core picks the
fallback. The one exception is `watch::read` (the watch thread's file reads, std calls): it returns `io::Result`, so
`NotFound` maps straight to a file that is gone. No `unwrap` on OS results outside tests.

| Failure | What happens |
|---|---|
| Power request fails | log, notice "cannot keep the PC awake" |
| Settings access denied (5 or 1260 from `PowerSettingAccessCheckEx` or a `PowerWrite*` call) | requests only for this run, one notice |
| Journal load, lock or write, RunOnce value or guard start fails, with any code (also 5: an elevated `wakeshot restore` may hold the journal mutex) | nothing written; requests only for the episode; one notice (power safety review finding 8) |
| Active plan changed before a write | nothing written; the next plan read handles the switch |
| Active plan changed between the writes and `PowerSetActiveScheme` | no call; the writes are journaled; the plan switch restores them |
| Plan list or active plan read fails in a restore | the run fails before any entry is dropped or written; retried |
| Write, read-back or `PowerSetActiveScheme` fails in an apply | the restore routine on this plan's entries (they stay until it succeeds), requests only for the episode, notice |
| Active plan read fails at the start of an apply | nothing written; requests only for the episode, notice |
| Plan read fails | nothing released; log; retried at the next read |
| Restore fails | journal kept; the tray retries at a tick, at most once per 10 s, while the restore is still wanted and warns once; restore mode and the guard retry for 2 minutes while the journal's owner is still the dead process, then start the tray; the Velopack hooks retry every 2 s for up to 25 s (updated: 10 s) |
| Journal corrupt | moved aside; the tray resets each key at a value Wakeshot writes to Windows' default, with one `JournalLost` notice (section 3, "A journal that cannot be read"); the guard, restore mode, the install and updated hooks and the CLI only move it aside, and the tray's next start resets; the uninstall hook resets itself, with log lines only (section 5, "Gap left") |
| Liveness Unknown | last state kept, retried on the next tick; no file deleted, no restore started |
| Settings mutex held by another session | requests only; retried on each tick |
| Hibernate not allowed, fails, or lid state unknown | notice, once per countdown; the hold's settings were already restored and the sleep route's 1-minute timeouts stay, so Windows sleeps the laptop once nothing holds it; while the lid stays closed, `Lock` with the chime at each input reading 5 minutes idle with the session unlocked and on the PC's screen, at most every 5 minutes, until the lid opens, unless the user's own lid action for the power source is Do nothing (power safety verify minor 1); a notice raised with the lid closed and the session unlocked is shown again with the closed-lid report (verify minor 2) |
| Lock fails | log, notice; a closed-lid lock also counts as failed when no `Session(Locked)` comes within 5 s (`LockWorkStation` only starts the lock); with the lid closed on our Do nothing: `Lock` once more at the next tick; if that fails too, step back as at the floor (`Requests` off, `restore_all`) and start the work-end countdown, until the session locks or the lid opens (Haik, 2026-11-03; power safety review finding 1); `LockFailed` is shown again with the closed-lid report, when the user is back at the open laptop (power safety verify minor 2) |
| Sound fails | log only |
| Agent file format unknown | "update Wakeshot" notice |
| Tray icon add fails at sign-in | retry with backoff; re-add on `TaskbarCreated` |
| Panic | the hook logs, then aborts, in every build profile; the guard restores; the next start covers the rest |

Log:

- `logs\wakeshot.log` in the data folder, plain English, one line per event or action. Each line: local wall time
  with its UTC offset, monotonic seconds, process (tray, guard, restore, cli, hook), level, area, text. Wall time is
  read for every line, never derived from a start time plus a counter: the tray spike's millisecond counter drifted
  16 s from the wall clock (research, "The spike's log clock drifted 16 s").
- The tray rolls it at start when it is over 1 MB and keeps three files. Every process appends one `WriteFile` per
  line to a handle opened for append; such lines never mix, tested on a local NTFS disk with four writers (research,
  "Log writer checks"). Network shares, FAT and ReFS: not verified.
- Logged: each hold start and end with its label, each request change, each settings write and restore with old and
  new values, each user change, lock, each hibernate check, each notice and sound, each format problem (at most once
  a minute per source). Never logged: prompts, transcript text, full command lines, `.key` file names.
- "Open log" opens the file with the default app (`ShellExecuteW` "open"). The log never leaves the PC.

## 9. Testing

Core (Phase 2):

- Scenario tests: a `Sim` harness feeds `(Now, Event)` pairs and checks the commands. The fake clock moves `wall_ms`
  and `mono_ms` apart, so suspends and wall clock jumps are covered.
- Property tests (`proptest`) on random event sequences, including plan switches, failed plan reads, user changes and
  outcomes that arrive late: owned keys are always restored after the last hold, quit and end of session; a released key
  is never written again in its episode, on any plan; a restore retry never takes an entry a running hold uses, and
  after a failed apply it covers every entry; no `Apply` after `Session(Ending)` until `EndCancelled`; no `Apply` and no
  screen request while another session is on the console; no request is on with no hold; no `Hibernate` within 5 minutes
  of a resume or of input; no `Lock` when the lid action in force is the user's own, except while a restore retry is
  wanted and the mirror is empty, where the lock rules lock on any Do nothing by design (section 3, "Lid closes"); every
  `Lock`, `Hibernate`, step back and user change comes with a notice or a sound.
- Windows layer unit tests for the IO thread's ownership check, `Apply` and the restore routine, against
  `DryRunPower` with a scripted plan: reads in flight around an `Apply`, two plans in one journal, a failed read, a
  failed or short plan list, a failed active plan read, a failed `PowerSetActiveScheme` then a retry, a stop between
  step 4's mark and the write, a plan switch between the writes and `PowerSetActiveScheme`, a `SafetyNet` failure at
  each step (no entry left behind), an `Apply` that takes over entries of a failed restore (marked and not), then a
  failed write, a failed call, and a stop at each point between its steps (no entry still in force is dropped), a
  failed active plan read at Apply step 1, and a guard or restore mode retry after another tray took the journal
  over.
- Parser fixtures: redacted files under `crates/wakeshot-core/tests/fixtures/`, one per case in the Claude live tests
  table, plus broken and unknown-format files. Built from `spikes/claude-watch/fixtures/`, which is public with the
  repo.

Windows layer (Phase 3), never on the user's real settings:

- Power plan tests use a throwaway copy that is never made active (`PowerDuplicateScheme`, named "Wakeshot test
  <pid>" with the test process's pid, deleted at the end; a leftover whose pid is not running is deleted at the next
  run). The install end-to-end test names its copy "Wakeshot test e2e", which no sweep touches. Tests never call
  `PowerSetActiveScheme`. That a write to an inactive plan changes only that plan is tested:
  `real_write_to_inactive_copy_changes_only_the_copy` (`power/plan.rs`) reads the real plan before and after a write
  to the inactive copy, and the values in force stayed the same (research "Power track real checks").
- Liveness with child processes the test starts; folder watch, atomic writes and file formats in a temp folder; the
  restore routine against the copy.
- `--dry-run`: `DryRunPower` logs each request and write instead of making it; lock and hibernate are only logged; its
  own data folder (section 4).
- All automated tests run in CI. Whether hosted runners have power plans: not verified (the CI card finds out).

Integration (Phase 6): wiring tests with fakes and the real core in CI (`app/` tests); Haik's checklist on an active
plan copy. The end-to-end suites and scripts below were built and run on the test laptop on 2026-11-04, never in CI:
each starts the tray, or the whole script, at Medium through `explorer.exe`, and the two suites are `#[ignore]`d tests
named `real_e2e_*`.

- `crates/wakeshot-tray/tests/e2e_dry_run.rs`: the two exes together in dry run on a temp data folder, 12 tests
  (research "Integration dry-run e2e").
- `crates/wakeshot-tray/tests/e2e_real.rs`: the real tray with real holds on plan copies `Wakeshot int A` and `B`,
  made active before the tray starts: hold start and end, user change, plan switch, Restore my power settings, Quit,
  end of session and a killed tray (the agent's parts of checklists 4 and 6; checklist 9), and `wakeshot restore`
  with a dead owner, 10 tests (research "Integration real e2e").
- `packaging/e2e.ps1`: install, update, Setup.exe over the install and uninstall of packed builds, each hook
  putting back a journaled value on the inactive copy `Wakeshot test e2e`, 81 checks (research "Install end to end
  checks").
- `packaging/e2e-update.ps1`: the update path on a real install with a local feed (debug builds only,
  `WAKESHOT_UPDATE_FEED`) and checklist 14, on the active copy `Wakeshot int update`, 125 checks (research
  "Integration update e2e").

Manual checklist for Haik on the test laptop. Rule for any test that uses a plan copy: Settings and the Control Panel
write some battery values to the user's real plan even while a copy is active (research), so read the real plan before
and after, and change nothing by hand during the test.

| # | Check | Phase |
|---|---|---|
| 1 | Lid closed, on battery, request held, battery sleep Never: no suspend past 916 s | 3 |
| 2 | Lid closed, no request, on battery, past the 600 s timeout: how long the PC keeps running | 3 |
| 3 | Re-take after Start > Sleep, on battery | 3 |
| 4 | End task on the tray in Task Manager, then on its group: the guard (Explorer as parent) lives on and restores | 3 |
| 5 | Tray and guard killed, start at login off, sign out and in: RunOnce restores | 3 |
| 6 | Sign out during a hold: restored before the session ends | 3 |
| 7 | Work ends with the lid closed: sleeps (1-minute timeouts) after 5 minutes, hibernates 5 minutes later if it did not; not while typing on an external screen | 6 (integration) |
| 8 | Battery floor with a short AC drop on the dock; the lid closed on the dock with the user's own AC lid action Do nothing, then unplugged: lock and sound, and what Windows does with the user's own values; the lid closed on the dock while holding, signed in on the external screen, then unplugged: lock and sound after 30 s on battery, none for a short AC drop | 6 (integration) |
| 9 | Plan switch during a hold | 6 (integration) |
| 10 | Unlock after a closed-lid hold: the report notice | 6 (integration) |
| 11 | `Global\` mutex as a normal user; a second session (optional): fast user switching during a hold gives `WTS_CONSOLE_DISCONNECT` and the restore, coming back gives `WTS_CONSOLE_CONNECT` and the apply; the same for Remote Desktop into the session | 3; console part 6 |
| 12 | A sign-in delay other than "Every Time", and which stored value means Never (needs an account without Windows Hello; none on the test laptop) | later |
| 13 | Theme switch, high contrast, display scales, sounds by ear, toast icon colours if toasts are used | 5 |
| 14 | `winget upgrade` and uninstall during a hold | 6 |
| 15 | About 50 apply and restore cycles on the active plan: display brightness, other apps, the System log | 3 |
| 16 | Lid close while holding, no loopback capture running: the chime is heard and the session locks (the lock lands during the chime's stream start) | 6 (integration) |

## 10. CI and build targets

- Targets `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`, each built on its own CPU. No 32-bit build.
- Rust pinned in `rust-toolchain.toml` (1.99.1, with rustfmt, clippy and both targets); a bump is its own change.
- Static CRT (`target-feature=+crt-static` in `.cargo/config.toml`), so the exes need no Visual C++ runtime on the
  user's PC. Without the flag Rust links the runtime DLL; with it the exes import only system DLLs (research,
  "Workspace build"). Never set `RUSTFLAGS` (or `CARGO_ENCODED_RUSTFLAGS`), in CI or locally: it replaces the config
  flags and drops the static CRT. The root `.cargo/config.toml` and `rust-toolchain.toml` also apply to spike builds
  run from `spikes/` (cargo and rustup look in parent folders), so spikes built after the workspace card use the static
  CRT and the pinned toolchain.
- Release profile: `lto = true`, `codegen-units = 1`, `panic = "abort"`, `strip = true` (the tray spike's settings).
  The panic hook aborts in every profile (section 4, tray startup).
- GitHub Actions `ci.yml` on push and pull request. Jobs on `windows-latest` (x64) and `windows-11-arm` (ARM64): fmt
  check (x64 only), clippy with `-D warnings` twice (with `--all-targets`, then without it, so the libs are checked
  as they ship, without `test-support`), tests, release build, exes as artifacts. Each clippy, test and build
  command passes `--locked` and `--target <triple>`. A step reads the release exes' imports with `dumpbin
  /dependents` and fails on `VCRUNTIME`, `MSVCP`, `ucrtbase` or `api-ms-win-crt-*`. CI never sets `RUSTFLAGS`. One
  `ubuntu-latest` job tests `wakeshot-core` alone, which proves it has no OS code, and fails if `cargo tree` for every
  target lists a `windows` crate under it. Read-only token. Actions from `actions/` by major tag; any other action
  pinned by commit SHA.
- Releases (Phase 6): `release.yml` on a `v*` tag, or by hand as a dry run (`workflow_dispatch`). It checks the tag
  against `[workspace.package] version`; per CPU it tests, builds, checks the imports as `ci.yml` does, packs with
  `vpk` 1.2.161 (`packaging/pack.ps1`) and runs `packaging/smoke-test.ps1` (silent install, checks, uninstall). One
  upload job (`contents: write`) puts both CPUs in one GitHub release; a version with `-` is a pre-release. The
  winget job runs only for a stable version and when the repository variable `WINGET_SUBMIT` is `true`. How to cut a
  release: `packaging/README.md`.

## 11. Dependencies

Checked 2026-11-03 on crates.io (research, "Phase 1 design checks").

| Crate | Version | Used in | Why |
|---|---|---|---|
| `windows` | 0.62.2 | `wakeshot-windows`, both exes (declared, `Win32_Foundation` only) | Win32 and WinRT bindings; every spike uses it |
| `serde` (derive) | 1.0.229 | core | file formats |
| `serde_json` | 1.0.154 | core, `wakeshot-windows` | file formats and agent files; the IO thread reads and writes the files |
| `embed-resource` | 3.0.12 | both exes' `build.rs` | manifest, icons, version info |
| `proptest` | 1.12.0 | core, declared as a dev-dependency | property tests |
| `velopack` | 1.2.161 | tray only (`velo.rs`), pinned `=1.2.161` in the root `[workspace.dependencies]` since `p6-velopack-dependency` (`Cargo.lock` 80 to 181 packages, 160 since `p6-tray-size`); ARM64 builds need clang for `ring` | installer and updates; 1.3.39-g8d2844f is a pre-release |
| `idna_adapter` | 1.0.0 | tray only, pinned `=1.0.0` in the root `[workspace.dependencies]` since `p6-tray-size`; never named in code | picks the IDNA back end with no Unicode data for velopack's `url`, so the 21 ICU packages leave the build; `url` then rejects non-ASCII host names, and the one URL the tray parses is ASCII (research, "Tray size") |
| cron next time | none: our own code | core (`sources::cron`) | `sources/cron.rs` (`p4-cron`) copies Claude Code's own scheduler (parser, search, jitter; research "Cron checks"), so a due time is the one Claude Code picks; no `croner` or `jiff` |

- `windows`: windows-rs moved its smaller crates to 0.100.0 on 2026-09-03 but has not published `windows` or
  `windows-sys` at that version, and says they may come later with changed API shapes (research). 0.62.2 is the
  current `windows` crate. Change when a `windows` 0.100 or later ships; that move is a port, not a version bump.
- Nothing else: no async runtime, no CLI parser crate (six commands), no logging or error crate.
- Both libs have a `test-support` feature with the fakes; only dev-dependencies turn it on.
