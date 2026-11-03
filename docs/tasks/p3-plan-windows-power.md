# Plan the Windows power layer track (Phase 3)

Phase: 3. Status: done. Needs Haik: yes, at the end: he signs off all five track plans at once (the orchestrator
relays one summary). Agent: `wakeshot-worker`. Size: about 120K tokens.
Runs after: `p1-design-verify-fixes` and the final design check (`docs/reviews/p1-design-final-check.md`), and
`p1-contracts` (the module layout and types in `crates/`). Parallel with: the other four track plan cards
(`p2-plan-core`, `p3-plan-windows-power`, `p4-plan-sources`, `p5-plan-tray-ux`, `p6-plan-ship`; each owns only its
own phase file and card prefix) and the Phase 1 review (read only).

## Goal

Phases 2 to 6 run as parallel tracks (`docs/plan.md`, the note under the Phases table). Plan this track: Power requests and settings, journal and restore, guard process, lock and hibernate, OS events.
Split it into task cards that can be built at the same time as the other four tracks with zero conflicts and no
lost work, test-first, against the contracts in `crates/`.

## Notes from the orchestrator (16:38)

- This track owns and builds first the shared `wakeshot-windows` modules `store` (atomic write, data folder paths),
  `log` (`LogWriter`) and `process` (liveness). Tracks 4 and 5 use them through fakes until your cards are done, so
  put them early in your order.
- Decide `p1-contracts` Result correction 12 (journal entries with a key this version does not know) in your Result,
  with its proposed default or a better one, and the reason.
- The IO thread writes `state.json` before it deletes `restore-request.json`, so `wakeshot restore` never prints the
  state from before the restore (`p1-contracts` "Found outside scope").
- `docs/architecture.md` is getting 13 type corrections from `p1-contracts` while you plan. Where it disagrees with
  `crates/`, the code is right.

## Read first

- `CLAUDE.md`; `docs/plan.md`: "How we work" (the parallel and Git rules) and the note under the Phases table.
- `docs/architecture.md`: section 4 (processes and threads, tray startup, the guard), section 5 (power requests, power settings, lock and hibernate, OS events), section 6 (journal, RunOnce), section 9 (testing, the manual checklist). Skim the rest for anything your track owns.
- `docs/product.md`: the lines your track serves.
- `docs/research.md`: only the sections the design cites for your track.
- `crates/`: the module layout and contract types written by `p1-contracts` (its card Result says what is where).
- `docs/phases/01-design-skeleton.md`: "Values I set" and the answered questions. Questions 10 to 12 are open and
  Haik's: plan around both answers or mark the card that depends on them; do not decide them.

## Deliverables

1. `docs/phases/03-windows-power.md`: goal, short track notes (only what `architecture.md` leaves open for this track),
   the card table (Card, Needs Haik, Runs after, Status), the order inside the track, and a work log section.
   `architecture.md` stays the design: any design change you need goes in your card's Result as a proposal, with the
   section and the new text.
2. Task cards `docs/tasks/p3-*.md` from `docs/tasks/_template.md`. Each card:
   - fits about 150K tokens of one worker; split bigger work;
   - names the files and folders it owns. Inside `crates/`, own whole module files. Never `lib.rs` or `main.rs`
     module lists, `Cargo.toml` or `Cargo.lock`: those change only through the contracts card the orchestrator runs
     alone, so list what you need under deliverable 3;
   - is test-first: the acceptance checks name the tests and the cargo commands (`CLAUDE.md` "Checks");
   - says its build folder: `CARGO_TARGET_DIR=<scratchpad>\<card name>\target`;
   - is marked when it touches the real laptop: power settings, screen, sound, installs, or needs Haik's hands.
     The orchestrator runs those one at a time across all tracks;
   - fills "Runs after" (inside this track, or another track's card by name) and "Parallel with".
3. In your card's Result: the contract changes this track needs (missing or wrong types, traits, module files,
   dependencies), each with the exact change, for one contracts card the orchestrator runs before the builds.
4. In your card's Result: what this track needs from other tracks and from when, and the fake it uses until then.

## Acceptance checks

- Every item of `architecture.md` and `product.md` this track owns is in a card, or named as integration (later).
- No two cards of this track own one file at the same time, and none owns a file another track's section names.
- Every card that needs the real laptop or Haik is marked as such.
- The journal, Apply and restore cards include a stop-point test: stop (or simulate a stop) between every two
  steps of Apply and of every restore path, then run the next start's restore, and check that no value Wakeshot still
  holds is dropped and the journal, RunOnce entry and guard stay until the restore succeeds. Three design reviews
  found ordering bugs here on paper (`docs/reviews/p1-design-*`); this test is the guard against the next one.
- No code written; `crates/`, `architecture.md`, `product.md` and other tracks' files unchanged.

## Out of scope

- Code; `crates/`; `Cargo.toml`; `docs/architecture.md`; `docs/product.md`; other tracks' phase files and cards.
- Integration cards (wiring the tracks into the two exes): planned after the tracks.
- Git.

## Questions for Haik

Each with the worker's default.

1. The two battery runs of `p3-haik-checks` (re-take after Start, Sleep on battery; lid closed on battery with sleep
   Never past 916 s): about 20 minutes each on battery, about 5 minutes of hands in all. Run them in Phase 3 or at
   integration? Default: Phase 3, because a fail there changes the battery design, and the earlier the better.
2. Values this plan sets (no answer needed unless you want another): journal lock wait 10 s; log files
   `wakeshot.log`, `wakeshot.1.log`, `wakeshot.2.log`; a UPS power source counts as battery; if Explorer cannot be the
   guard's parent, the guard starts as the tray's own child (End task on the group then ends both, a path already in
   "Gap left").
3. Question 12 of the Phase 1 file is still yours and open; `p3-recovery` builds its default (a failed restore starts
   the tray after 2 minutes) and keeps the other answer a one-line change.

## Found outside scope

Problems seen but not fixed.

- `docs/architecture.md` section 2: the core `settings` row still lists `plan_apply`, `plan_restore`, and the Windows
  `power::plan` row still holds `apply()` and `restore_from_journal()`. After the 17:05 decision both move: proposal
  D1 below (the core plan's P1 covers the core rows).
- `docs/architecture.md` section 9 says "Tests never call `PowerSetActiveScheme`", while the checklist rule (same
  section) lets manual checks make a copy active, which `p3-real-checks` (checklist 15) and `p3-haik-checks`
  (checklist 1) do. Suggest "Automated tests never call ...".
- Sources and ship plans list `p3-restore` as a dependency (`docs/phases/04-sources.md` `p4-cli-restore`;
  `docs/phases/06-ship.md` `p6-hooks`). That card is dropped: they now need `p3-journal` (`restore_locked`) and the
  core's `p2-ownership-restore`.
- `docs/phases/06-ship.md` gives the start-at-login Run value to its own `install.rs`, while `architecture.md` section
  2 puts "Run and RunOnce values" in `store` (`p3-store` builds `AutoStart` for both). No file conflict; two registry
  helpers. Suggest `install.rs` call `store::AutoStart` for the Run value.
- `crates/wakeshot-windows/Cargo.toml` turns on no core fakes: its `test-support` feature is empty and it has no
  dev-dependencies, so its unit tests cannot use the core's `FakePlans`. Contract change C9 below.

## Log

- 2026-11-03 16:27: card written by the orchestrator (Phases 2 to 6 as parallel tracks, Haik's ask).
- 2026-11-03 16:38: worker started. Reading architecture sections 4, 5, 6, 9, `crates/`, the Phase 1 file.
- 2026-11-03 16:44: read done (architecture 2 to 11, `crates/` both libs, `p1-contracts` Result, other track cards,
  research power, lid, liveness, contracts). Orchestrator added one open point: an entry left on plan P for a key
  released elsewhere (from `p1-design-verify-fixes` "Found outside scope"); to decide in the Result. Found: the
  `windows` features lack `Wdk_System_SystemInformation` (process list fallback).
- 2026-11-03 16:50: checked 50 Win32 calls against the `windows` 0.62.2 source; only the process list lacks a
  feature. Appended research "Power track plan checks". Decided: fix (B) for the released-key entry (Apply restores
  its plan's entries outside `writes`); correction 12: keep unknown entries, count only known ones. Writing the phase
  file and cards next.
- 2026-11-03 16:55: phase file written (first form).
- 2026-11-03 16:58: orchestrator relayed the contracts domain review (`docs/reviews/p1-contracts-domain.md`): one
  contracts card runs alone before the builds. Read B1, M1 to M5, m1 to m7. Reworking: signatures for the safety net,
  apply, restore, store, log, process and the IO queue go into the Result; `FakePower` gets in-force values, a call log
  and a shared script (fail, stop, plan switch, user write); the property test becomes its own card; `p3-recovery`
  owns the tray bin's `guard.rs` and `restore.rs`.
- 2026-11-03 17:10: 17 cards written. Checked other tracks' new files for overlaps: the core plan
  (`docs/phases/02-core.md` note 1) also puts apply, restore, ownership check and their property test in the core
  (`p2-ownership-*`), already aligned with this plan's decisions, and flags the overlap for the orchestrator. Aligned
  with its leftover rule (`Restore { why: Leftover }`) and the tray plan's `play_and_wait(Cue::GoingToSleep)`.
- 2026-11-03 17:15: pick: the core layout (reason in the Result). Reworking: `p3-journal` becomes the real
  `SafetyNet`; `p3-restore`, `p3-apply`, `p3-power-props` on hold as the alternative; `p3-power-plan` implements the
  core's `PlanIo`; `p3-power-io` and `p3-recovery` call the core routines.
- 2026-11-03 17:17: orchestrator decision (17:05): the core owns the routines. Deleted my `p3-restore`, `p3-apply`,
  `p3-power-props` (files I created); added `p3-stop-points` (the walk against the real parts); reworked
  `p3-power-plan`, `p3-journal`, `p3-power-io`, `p3-recovery`, `p3-hibernate`, the phase file. Read the core plan's
  Result (seams, routine signatures) and the sources plan's asks on `process` and `store`.
- 2026-11-03 17:25: acceptance checks run (Result); Result written. Status done.
- 2026-11-03 18:03 (`p1-track-plan-fixes`): history above is kept as written. `p3-restore`, `p3-apply` and
  `p3-power-props` were dropped at 17:17; read `p3-restore` and `restore_from_journal` as `p3-journal`
  (`power::journal::restore_locked`) plus the core's `p2-ownership-restore`. `PowerApi` and `FakePower` are gone
  (`p1-contracts-split`): `FakePlans` is the one plan fake. The Run value's one writer is `store::AutoStart`
  (cross-track review m12). The cards and phase files carry the current text.

## Result

Done 2026-11-03 17:25. Docs only; no code, no git.

### Files

- New: `docs/phases/03-windows-power.md`; 15 cards `docs/tasks/p3-store.md`, `p3-log.md`, `p3-process.md`,
  `p3-power-plan.md`, `p3-requests.md`, `p3-system-events.md`, `p3-session.md`, `p3-journal.md`, `p3-hibernate.md`,
  `p3-power-io.md`, `p3-recovery.md`, `p3-stop-points.md`, `p3-harness.md`, `p3-real-checks.md`, `p3-haik-checks.md`.
- Changed: this card; `docs/research.md` (new section "Power track plan checks", additions only).
- Created and deleted after the 17:05 decision: `p3-restore.md`, `p3-apply.md`, `p3-power-props.md`.

### Plan in short

15 cards, about 1.43M tokens. Wave 1 (seven cards in parallel) starts with `store`, `log` and `process`, which tracks
4, 5 and 6 wait on. The core's `p2-ownership-restore` and `p2-ownership-apply` sit on this track's critical path
(power-io, recovery and stop-points call them). Real laptop: `p3-power-plan` (its `real_` tests), `p3-stop-points`,
`p3-real-checks`. Haik: `p3-haik-checks` (about 60 minutes elapsed, about 15 of hands); `p3-recovery` builds on
Question 12's default.

### Decision: correction 12 (journal entries with a key this version does not know)

Keep such an entry in the file untouched; restore, apply and the ownership check skip it; the journal counts as empty
when no known entry is left, and the clean-up then clears RunOnce and stops the guard but saves the file with only
those entries instead of deleting it; a save always writes `JournalFile::VERSION`; `Started.journal` holds known
entries only, `None` when there are none; the seam (`FileNet`) logs the unknown entries once per load. Why: keeping it
forever as "owned" would keep RunOnce and the guard and fail every restore (a retry loop and a warning at every
sign-in for something this version can never fix); dropping it would lose the user's original for good. This way the
original stays reachable for the newer version, which restores it at its next start, and this version stops nagging.
Cost: on a PC that stays downgraded, that one setting stays overridden until the user sets it or upgrades; the log
names it. The logic lives in the core (`p2-ownership-restore` already follows it); the logging in `p3-journal`.

### Decision: the released key's entry on the plan being applied

Fix (B): `apply` restores the journal entries on its plan whose key is not in `writes`, in the same run and the same
`set_active` (read in step 2, marked in step 3's journal write by the restore step 3 rules, `original` written in step
5, dropped after a successful step 6). Why not `Restore` with keys: that changes `Command::Restore` and
`Outcome::Restored`, and makes the core compute which keys to restore on the active plan, a second place for the same
rule. With (B) the core already sends the `Apply` on the switch back, the apply already holds the journal and reads
the plan, and one call covers both. Its one condition: `writes` lists every key the hold wants on that plan (the three
less the released ones) and never drops a key for "already at target" (the IO side's skip). The core plan took it
(`docs/phases/02-core.md` note 3, `p2-ownership-apply`); this track adds it as a starting state in `p3-stop-points`.

### Contract changes for the contracts card

Lined up with the core plan's Result (`p2-plan-core`, items 1 to 10, option A, which the orchestrator took). Same
shape where both name an item; differences marked "Differs".

- C1. Module files in `wakeshot-windows`, each new file with one `//!` line naming its owner card: `lib.rs` adds
  `pub mod io_queue; pub mod log; pub mod process; pub mod recovery; pub mod session; pub mod store;`;
  `power/mod.rs` adds `pub mod executor; pub mod hibernate; pub mod journal; pub mod requests; pub mod system;` and
  `#[cfg(test)] mod stop_points;`. Tray bin: `src/guard.rs` and `src/restore.rs`, each `pub fn run(args: &[OsString])
  -> ExitCode` returning success, dispatched from `main.rs` for `--guard` (passing the rest of the arguments) and
  `--restore`; owner `p3-recovery`.
- C2. `store.rs` (owner `p3-store`; bodies `unimplemented!("p3-store")` except the pure path code):
  `DataDir { real() -> Result<Self, WinError> (debug builds: WAKESHOT_DATA_DIR wins), dry_run(), at(PathBuf), root(),
  file(DataFile) -> PathBuf, holds(), logs() }`; `enum DataFile { Config, State, Journal, JournalCorrupt,
  JournalCorruptSeen, RestoreRequest, KeepAwake }`; `write_atomic(&Path, &[u8]) -> Result<(), WinError>` (temp name
  `<name>.<pid>.<n>.tmp`, never ending in `.json`, as the sources plan asks); `read_whole(&Path) ->
  Result<Option<Vec<u8>>, WinError>`; `remove(&Path) -> Result<bool, WinError>`; `Names { tray, settings, journal }`
  with `real()`, `dry_run()`, `test(&str)`; `NamedMutex::open(&str)`, `take(timeout_ms) -> Result<Option<HeldMutex>,
  WinError>`, `HeldMutex::abandoned() -> bool` (owns its handle, releases on drop); `enum StartKey { Run, RunOnce }`;
  `AutoStart { real(), at(&str), set(StartKey, name, command), get(StartKey, name) -> Option<String>, remove(StartKey,
  name) -> bool }`.
- C3. `log.rs` (owner `p3-log`): `enum Proc { Tray, Guard, Restore, Cli, Hook }`, `enum Level { Info, Warn, Error }`,
  `trait Log: Send + Sync { fn line(&self, level: Level, area: &'static str, text: &str); }`, `LogWriter::open(&Path,
  Proc) -> Result<Self, WinError>`, `LogWriter::roll(&Path) -> Result<(), WinError>`; working `MemoryLog { lines:
  Mutex<Vec<(Level, &'static str, String)>> }` behind `test-support`.
- C4. `process.rs` (owner `p3-process`): `trait Liveness: Send + Sync { fn check(&self, p: ProcessRef) -> Alive; fn
  start_of_pid(&self, pid: u32) -> Lookup; fn list(&self) -> Result<Vec<ProcessEntry>, WinError>; }` (the last two
  and `ProcessEntry { pid, start: FileTime, image: String }` as the sources plan asks; `Lookup` in the core's
  `process.rs`, its item 3); `SystemLiveness`; `liveness(ProcessRef) -> Alive`; `current() -> Result<ProcessRef,
  WinError>`; `current_session() -> Result<u32, WinError>`; `start_of(&std::process::Child) -> Result<FileTime,
  WinError>`; `ProcessHandle { open(ProcessRef) -> Result<Option<Self>, WinError>, process(), wait(timeout_ms) ->
  Result<bool, WinError>, raw() -> HANDLE }`; `ExitWait::register(&ProcessHandle, Box<dyn Fn() + Send + Sync>) ->
  Result<Self, WinError>`; `Spawned { handle: ProcessHandle, outside_tree: bool }`; `spawn_outside_tree(&Path,
  &[OsString]) -> Result<Spawned, WinError>`; working `FakeLiveness` (scriptable answers for all three methods)
  behind `test-support`.
- C5. `io_queue.rs` (owner `p3-power-io`): `IoQueue: Send + Sync { new(), push(Command), push_front(Command),
  drop_queued(&dyn Fn(&Command) -> bool) -> usize, pop(timeout_ms) -> Option<Command>, session_ending(),
  session_resumed() }`.
- C6. The plan seam. Take the core's `PlanIo` and `SafetyNet` (`ownership/io.rs`) as written. Remove `PowerApi`,
  `FakePower`, `PowerCall`, `FAKE_MISSING` from `wakeshot-windows` (and their `testing.rs` lines): one seam, one fake
  (`FakePlans`), and nothing outside this track uses them. `power::plan` keeps `PlanSetting`. **Differs:** add to
  `PlanIo` `fn values(&mut self) -> Result<PlanValues, Failure>;` ("the active plan id and the five values in one read,
  for `Event::Plan`; the routines never call it"), so the executor's `ReadPlan` path is testable with `FakePlans`.
  If the core track prefers `PlanIo` minimal, the fallback is a local trait in `power::plan` that `FakePlans` cannot
  serve without a wrapper.
- C7. **Differs (an addition):** the journal lock is not in `SafetyNet`, and that is right (the fake needs no mutex),
  so the contract should say who takes it: "every caller of `ownership::{apply, restore, check}` goes through
  `wakeshot_windows::power::journal::{apply_locked, restore_locked, check_locked}`, which hold
  `Local\Wakeshot.Journal`". Stub signatures: the core routine's arguments plus `names: &Names, log: &dyn Log`, same
  return types. Owner `p3-journal`. The CLI (track 4) and the hooks (track 6) call these.
- C8. **Differs (an addition):** `event.rs` `Event::ActivePlan(Guid)` ("the active plan changed, from
  `GUID_ACTIVE_POWERSCHEME`; the core sends `ReadPlan` when it differs from the last read's `active` and never acts on
  it alone"). Without it the 4 ms plan notification (section 5, "Plan switch") reaches nobody.
- C9. `wakeshot-windows/Cargo.toml`: feature `Wdk_System_SystemInformation` (research "Power track plan checks");
  `test-support = ["wakeshot-core/test-support"]`; `[dev-dependencies] wakeshot-core = { workspace = true, features =
  ["test-support"] }` (for `FakePlans`); `[[example]] name = "power_check"`, `required-features = ["test-support"]`,
  with a stub `examples/power_check.rs` (`fn main() {}`). No `proptest` here any more (the property test is the
  core's). `Cargo.lock`: no new package.
- C10. Items this track needs that other plans own, same shape as theirs: the event sink (`EventSink`,
  `FakeEventSink`, review M1); `SoundPlayer::play_and_wait(Cue)` (tray plan); `Event::JournalCorrupt`,
  `Notice::JournalCorrupt`, `RestoreWhy::Leftover`, `Command::FinishRestoreRequest(StateFile)`,
  `FileKind::RestoreRequest`, `Command::Log(String)` and the doc lines on `Apply`, `Retake`, `Hibernate` (core plan
  items 5 to 8, 10). Also a doc line on `files::JournalKey` replacing "What the restore does with an `Other` entry is
  open" with the correction 12 rule above.

Count: 10 items, of which C6 to C8 differ from or add to the core plan's text and C10 is owned by other plans.

### Design proposals for `architecture.md` (section, new text; not edited here)

- D1, section 2, Windows modules: `power::plan`: "`Win32Power` and `DryRunPower`, the Windows `PlanIo`; `PlanSetting`
  for the `PlanValues` read". New rows: `power::journal` "`FileNet`, the Windows `SafetyNet` (journal file, RunOnce,
  guard), and `apply_locked`, `restore_locked`, `check_locked` under `Local\Wakeshot.Journal`"; `power::executor` "runs
  `ReadPlan`, `Apply`, `Restore` on the IO thread; startup restore"; `power::hibernate` "the hibernate thread";
  `io_queue` "the IO queue and its end-of-session rule"; `recovery` "guard mode, restore mode, the tray's guard
  control". `power::system` drops `hibernate()`, `hibernate_allowed()`. `store` adds "named mutexes". `process`:
  "guard start" becomes "a spawn outside the tray's tree".
- D2, section 5 "Apply", after step 2: "Entries on `plan` whose key is not in `writes` (released in this episode, left
  by a failed restore) are restored by this apply: step 2 reads them, step 3's journal write marks the ones restore
  step 3 would restore, step 5 writes their `original`, and a successful step 6 drops them." Section 3, retry rule:
  "the hold's `Apply` takes over or restores the entries on the active plan".
- D3, section 6, journal bullets: replace "every version restores the entries whose `key` it knows, also from a newer
  journal" with the correction 12 rule above, plus "A rewrite always writes this version's `version`; fields added
  later are optional."
- D4, section 5 restore step 1: after "move it to `journal.json.corrupt`, warn" add "(the tray posts
  `Event::JournalCorrupt` and renames the file `journal.json.corrupt.seen`, so the notice shows once per file)".
- D5, section 5 "Ownership": "A read error: keep the entry, post the failure" becomes "A failed active plan read posts
  `Plan(Err)`; a failed entry read keeps the entry and is logged, and the values still go out as `Plan(Ok)`".
- D6, section 4 "Processes": after the journal mutex sentence add "A wait gives up after 10 s; that run fails and is
  retried."
- D7, section 5 "Guard": "If Explorer is missing or its open is denied, the guard starts as the tray's own child and
  the log says so."
- D8, section 8 "Log": the rolled files are `wakeshot.1.log` and `wakeshot.2.log`.
- D9, section 9 "Windows layer (Phase 3)": "Tests that touch a plan, the screen or need Haik are `#[ignore]` with a
  `real_` prefix and run only in marked cards (`-- --ignored --test-threads=1`), never in CI. In debug builds
  `WAKESHOT_DATA_DIR` replaces the data folder. The stop-point walk runs against the core's fake (core track) and
  against `Win32Power` on a plan copy with `FileNet` in a temp folder (`p3-stop-points`)."
- D10, section 5 "OS events", the active plan row: "posted as `Event::ActivePlan`; the core reads the plan".

### What this track needs from other tracks, and gives

| Track | Need or give | From when | Fake until then |
|---|---|---|---|
| Core (needs) | `ownership::{restore, apply, check}` and `FakePlans` | `p3-recovery` after `p2-ownership-restore`; `p3-power-io`, `p3-stop-points` after `p2-ownership-apply` | `FakePlans` from the contracts card for unit tests; the routines' bodies are stubs until then, so those three cards wait |
| Core (needs) | `Apply.writes` lists every wanted key; `ReadPlan` on `Event::ActivePlan`; `Leftover` retry; never `Play(GoingToSleep)` | its scenario cards | none needed here |
| Tray (needs) | `SoundPlayer::play_and_wait` | `p3-hibernate` | `FakeSoundPlayer` |
| Tray (gives) | `log` (`MemoryLog` until `p3-log`); `session::decode` for its window, if it owns the loop window | wave 1 | `MemoryLog` |
| Sources (gives) | `store`, `log`, `process` with its two `Liveness` additions; `restore_locked` for `wakeshot restore` | wave 1; `p3-journal` | `DataDir::at`, `MemoryLog`, `FakeLiveness`; `FakePlans` |
| Ship (gives) | `restore_locked` with the hook's budget, `FileNet` hook form, `store::AutoStart` | `p3-journal`, `p3-store` | `FakePlans` |
| Integration (gives) | executor, `IoQueue`, startup, recovery modes, dry-run forms, harness | end of this track | none |

### Acceptance checks of this card

- Every item of `architecture.md` and `product.md` this track owns is in a card or named as integration: pass. Walked
  section 4 (mutexes: `p3-store`; IO queue and end of session: `p3-power-io`; startup step 6: `p3-power-io`; dry run:
  `p3-power-plan`, `p3-requests`, `p3-session`, `p3-hibernate`, `p3-journal`), section 5 (requests: `p3-requests`;
  plan calls, write logging, plan copy: `p3-power-plan`; journal, RunOnce, lock: `p3-journal`; liveness and spawn:
  `p3-process`; guard and logon restore: `p3-recovery`; lock: `p3-session`; hibernate: `p3-hibernate`; OS events:
  `p3-system-events`, `p3-session`; apply, restore, ownership: core, with the real-part walk in `p3-stop-points`;
  "Gap left": README, track 6), section 6 (data folder, atomic write, registry values: `p3-store`; journal file:
  `p3-journal`), section 8 (this track's rows; log: `p3-log`), section 9 (Phase 3 tests: `p3-power-plan`,
  `p3-stop-points`; checklist: "Manual checklist" table in the phase file), decisions 15, 16, 18 (lock), 21, 23, 24,
  25, 27, 28, 29, 38, and `product.md` "Power behavior" and "User settings safety". Integration items are listed in
  track note 11.
- No two cards of this track own one file, and none owns a file another track's section names: pass. One owner per
  file in track note 1; checked against the 02, 04, 05 and 06 phase files and cards at 17:13 and 17:20 (the only
  shared-looking names are the sources track's `shell.rs` and the ship track's `install.rs`, `hooks.rs`, which no card
  here owns).
- Every card that needs the real laptop or Haik is marked: pass (`p3-power-plan`, `p3-stop-points`,
  `p3-real-checks`, `p3-haik-checks`; `p3-recovery` notes Question 12).
- Stop-point tests: pass. Apply, every restore path and the ownership check: the core's cards against its fake, and
  `p3-stop-points` against the real parts (`real_stop_points_apply`, `_restore`, `_check`, `_unknown_key`, with a
  scripted failure of the next restore); startup, guard and restore mode: `startup_stop_points` (`p3-power-io`),
  `guard_stop_points` and `restore_mode_stop_points` (`p3-recovery`). Each checks that the journal, RunOnce value and
  guard stay while any known entry exists.
- No code written; `crates/`, `architecture.md`, `product.md` and other tracks' files unchanged: pass (only
  `docs/phases/03-windows-power.md`, `docs/tasks/p3-*.md` and an appended section of `docs/research.md` were written).

### Open items

- The contracts card: C1 to C10, and the three "Differs" points to settle with the core track (C6 `values()`, C7 the
  locked entry points, C8 `Event::ActivePlan`).
- Haik: the questions above (battery runs now or later; values; Question 12).
- Sources and ship plans: replace their `p3-restore` dependency (Found outside scope).
