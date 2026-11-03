# Plan the Core track (Phase 2)

Phase: 2. Status: done. Needs Haik: yes, at the end: he signs off all five track plans at once (the orchestrator
relays one summary). Agent: `wakeshot-worker`. Size: about 120K tokens.
Runs after: `p1-design-verify-fixes` and the final design check (`docs/reviews/p1-design-final-check.md`), and
`p1-contracts` (the module layout and types in `crates/`). Parallel with: the other four track plan cards
(`p2-plan-core`, `p3-plan-windows-power`, `p4-plan-sources`, `p5-plan-tray-ux`, `p6-plan-ship`; each owns only its
own phase file and card prefix) and the Phase 1 review (read only).

## Goal

Phases 2 to 6 run as parallel tracks (`docs/plan.md`, the note under the Phases table). Plan this track: State machine, policies, settings ownership rules, file formats, full tests (no OS code).
Split it into task cards that can be built at the same time as the other four tracks with zero conflicts and no
lost work, test-first, against the contracts in `crates/`.

## Read first

- `CLAUDE.md`; `docs/plan.md`: "How we work" (the parallel and Git rules) and the note under the Phases table.
- `docs/architecture.md`: section 3 (the core: events, state, commands, clocks, episode flow, decisions table), section 6 (files and formats), section 8 (errors and logging, the core's part), section 9 (testing, the core's part). Skim the rest for anything your track owns.
- `docs/product.md`: the lines your track serves.
- `docs/research.md`: only the sections the design cites for your track.
- `crates/`: the module layout and contract types written by `p1-contracts` (its card Result says what is where).
- `docs/phases/01-design-skeleton.md`: "Values I set" and the answered questions. Questions 10 to 12 are open and
  Haik's: plan around both answers or mark the card that depends on them; do not decide them.

## Deliverables

1. `docs/phases/02-core.md`: goal, short track notes (only what `architecture.md` leaves open for this track),
   the card table (Card, Needs Haik, Runs after, Status), the order inside the track, and a work log section.
   `architecture.md` stays the design: any design change you need goes in your card's Result as a proposal, with the
   section and the new text.
2. Task cards `docs/tasks/p2-*.md` from `docs/tasks/_template.md`. Each card:
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
- The settings ownership card includes a property test (`proptest`) over the journal entry states: any sequence of
  Apply, user change, plan switch, failed read, failed write and stop, followed by a restore, never drops an entry
  whose value Wakeshot still holds. Three design reviews found ordering bugs here on paper; this is the guard.
- No code written; `crates/`, `architecture.md`, `product.md` and other tracks' files unchanged.

## Out of scope

- Code; `crates/`; `Cargo.toml`; `docs/architecture.md`; `docs/product.md`; other tracks' phase files and cards.
- Integration cards (wiring the tracks into the two exes): planned after the tracks.
- Git.

## Questions for Haik

Each with the worker's default.

None new. `p2-core-settings` builds Question 10's default (console part); `p2-core-lid` builds two items only if the
sign-off takes the tray track's held-notices proposal and the ship track's update contract.

## Found outside scope

Problems seen but not fixed.

- `architecture.md` section 3, Countdown: "with the going to sleep sound right before it" reads as the core's `Play`;
  section 5 has the hibernate thread play it (review M5). Proposal P3 below.
- `architecture.md` section 3, Keep screen on idle lock: no notice or sound, though Question 8's answer gives one to
  every action but the routine hold start and end. Contract item 9 and proposal P3.
- `Core::new(Config)` and `Started { config }` both carry the config (review m7). Contract item 4 says `Started` wins.
- The core has no UTC offset, so no text can show a clock time ("Keep awake until 18:00"). Track note 14.

## Log

- 2026-11-03 16:27: card written by the orchestrator (Phases 2 to 6 as parallel tracks, Haik's ask).
- 2026-11-03 16:38: worker started. Status in progress. Docs only; no code, no git.
- 2026-11-03 16:49: read plan, architecture 2, 3, 5 (apply, restore, ownership), 6, 7, 8, 9, product, phase 1 values
  and questions 10 to 12, the core crate and `p1-contracts` Result, the other track cards, the power review headings.
  Plan: the apply, restore and ownership check run in the core, generic over a new `PlanIo` trait (contract), so the
  journal proptest guards the real order; Phase 3 implements `PlanIo`. Ten cards in three lanes.
- 2026-11-03 16:55: phase file draft written. Orchestrator message: plan around `docs/reviews/p1-contracts-domain.md`
  (one contracts card splits shared files and adds seams first). Read it. Adjusting: two seams in the core
  (`PlanIo`, `SafetyNet`), routine signatures in the contracts card, strings in a Phase 5 file (`text.rs`) apart from
  `present.rs`, Keep awake as a command with wall ms (no core formatter), `RestoreWhy::Leftover`, corrupt journal
  signal at start and in `Restored`.
- 2026-11-03 17:00: phase file rewritten to the review's split; ten cards written (`p2-ownership-restore`, `-apply`,
  `-props`, `p2-policy-holds`, `p2-policy-power`, `p2-present`, `p2-core-holds`, `-settings`, `-lid`, `-props`).
  Writing the Result (contract changes, cross-track needs, design proposals) next.
- 2026-11-03 17:13: read the other planners' phase files. The power plan also puts the apply, restore, journal and
  their property test in `wakeshot-windows`: flagged as an open overlap for the orchestrator (my pick and the delta
  either way in the Result). Aligned with its decisions (correction 12, released-key entry, `JournalCorrupt`,
  `FinishRestoreRequest`), with the sources track's `holds::file::format_expires` (dropped my `KeepAwake` command and
  `Restore { keys }`), and with the tray track's `text.rs` split and its held-notices proposal (conditional item).
- 2026-11-03 17:16: added the ship track's update rule to `p2-core-lid` (conditional) and a property to
  `p2-core-props`. Ran the checks (Result). Result written; Status done.
- 2026-11-03 18:03 (`p1-track-plan-fixes`): history above is kept as written. `p3-restore`, `p3-apply` and
  `p3-power-props` were dropped at 17:17; read `p3-restore` and `restore_from_journal` as `p3-journal`
  (`power::journal::restore_locked`) plus the core's `p2-ownership-restore`. `PowerApi` and `FakePower` are gone
  (`p1-contracts-split`): `FakePlans` is the one plan fake. The Run value's one writer is `store::AutoStart`
  (cross-track review m12). The cards and phase files carry the current text.

## Result

Done 2026-11-03 17:16. Docs only: no code, no git.

### Files

- New: `docs/phases/02-core.md` (goal, 19 track notes, module layout with owners, card table, five waves, where other
  tracks meet this one, coverage table, work log).
- New cards: `docs/tasks/p2-ownership-restore.md`, `p2-ownership-apply.md`, `p2-ownership-props.md`,
  `p2-policy-holds.md`, `p2-policy-power.md`, `p2-present.md`, `p2-core-holds.md`, `p2-core-settings.md`,
  `p2-core-lid.md`, `p2-core-props.md`.
- Changed: this card only. `crates/`, `architecture.md`, `product.md` and other tracks' files untouched.

Sizes: ownership-restore 140K, ownership-apply 130K, ownership-props 110K, policy-holds 100K, policy-power 90K,
present 80K, core-holds 150K, core-settings 150K, core-lid 130K, core-props 130K. Total about 1.21M tokens. If the
power track gets the routines (open item 1): 7 cards, about 0.83M. No card touches the real laptop or needs Haik's
hands. Question 10 can change a few rules in `p2-core-settings` (default built).

### Acceptance checks

1. Every item this track owns is in a card or named as integration: pass. `02-core.md` "Coverage" maps the decisions
   marked Phase 2 (0, 3, 11, 12, 17, 18, 20, 21, 23, 25, 26, 31, 32, 35), the core parts of 5 to 8, 15, 16, 24, 27 to
   29, 37, 38, section 3 (events, state, commands, clocks, episode flow, countdown, report, user actions), section 6
   (config, state, the Keep awake hold, the journal form), section 8 and 9 (core parts). The loop, the safety tick
   timer, the IO queue and the command runner are integration.
2. No two cards own one file at the same time, and none owns another track's file: pass. `core.rs`, `state.rs`,
   `sim.rs` pass from card to card in sequence; `ownership/fake.rs` from restore to apply. A grep of the 03 to 06
   phase files and every p3 to p6 card finds no claim on a file this plan owns (the tray track reads `view.rs` and
   `present.rs`; `text.rs` and `holds/file.rs` are theirs). One work overlap (not a file clash): open item 1.
3. Cards that need the real laptop or Haik are marked: pass (none need the laptop; the Question 10 dependency is on
   `p2-core-settings`; two conditional items on `p2-core-lid`).
4. The settings ownership property test: pass. `p2-ownership-props`: any sequence of Apply, user change, plan
   switch, plan delete, check and restore, each with a failed call or a stop at any call, then a fault-free restore,
   never drops an entry whose value Wakeshot still holds (stored or in force), plus five more properties and seven
   mutants taken from the review findings that must each fail. Under option B it is `p3-power-props`, and
   `p2-core-props` carries the core side.
5. No code; `crates/`, `architecture.md`, `product.md`, other tracks' files unchanged: pass.

### Open item 1 for the orchestrator: who owns the apply, restore and ownership check

The power plan (`03-windows-power.md`: `p3-journal`, `p3-restore`, `p3-apply`, `p3-power-props`) and this plan
(`p2-ownership-*`) both build these routines and their property test. Pick one before the contracts card.

- **A (my pick): the routines in the core**, generic over `PlanIo` and `SafetyNet` (contract items 2, 3). Why:
  `plan.md` gives "settings ownership rules" to Phase 2 and `architecture.md` section 2 puts `plan_apply` and
  `plan_restore` in core `settings`; the routines are pure order and rules, so they test on every CI target; and the
  core's `Sim` can run the real routines, so `p2-core-props` checks "owned keys are always restored" against what
  Windows would hold. Power track delta: `p3-power-plan` adds `PlanIo` over `PowerApi`; `p3-journal` becomes the
  `SafetyNet` implementation (journal file, RunOnce, guard; real and dry run); `p3-restore` and `p3-apply` shrink to
  the mutex, the call and the outcome mapping (they could merge into `p3-power-io`); `p3-power-props` is dropped;
  its stop-point kit for the routines is `ownership::fake::FakePlans` (same design as its note 4); `FakePower`
  scripting stays for adapter tests.
- **B: the routines in `wakeshot-windows`** (the power plan as written). Consequence: the core and the real routines
  never run together in pure code; "owned keys are always restored" is checked on a model, and an end-to-end check
  needs a test in `wakeshot-windows` or integration. Core delta: drop `p2-ownership-restore`, `-apply`, `-props`,
  contract items 2 and 3 and the `ownership` module lines; `p2-core-props` uses the model written in its header.

### Contract changes for the contracts card

All in `crates/wakeshot-core/src/`. Items 2 and 3 only under option A. No new dependency (`proptest` is already a
core dev-dependency).

1. Module files, each new file with only a `//!` line naming its owner card:
   - `lib.rs` adds `pub mod policy;`, `pub mod present;`, `mod state;`,
     `#[cfg(any(test, feature = "test-support"))] pub mod sim;`, `#[cfg(test)] mod scenarios;`, and under A
     `pub mod ownership;`. (`pub mod text;` is the tray track's item; `holds/file.rs` the sources track's.)
   - `policy/mod.rs`: `pub mod ending; pub mod floor; pub mod hold_files; pub mod lid; pub mod lock; pub mod requests;
     pub mod sessions;` and those seven files.
   - `present.rs`, `state.rs`, `sim.rs`; `scenarios/mod.rs` with `mod holds; mod lid; mod props; mod settings;` and
     those four files.
   - Under A, `ownership/mod.rs`: `pub mod apply; pub mod check; pub mod io; pub mod journal; pub mod restore;`,
     `#[cfg(any(test, feature = "test-support"))] pub mod fake;`, `#[cfg(test)] mod props;` and those seven files.
2. Under A, `ownership/io.rs`, the two seams (doc lines shortened here; owner: core track for the traits, power track
   implements them, real and dry run):

   ```rust
   use crate::command::Failure;
   use crate::files::JournalFile;
   use crate::settings::{Guid, SettingKey};

   /// What `journal.json` held when a routine loaded it (restore step 1).
   #[derive(Clone, Debug, PartialEq, Eq)]
   pub enum JournalRead { Missing, Corrupt, Failed(Failure), Read(JournalFile) }

   /// Plan calls, one per step. `plans` is the whole list or an error (a list that ends on any code but 259).
   /// `write` stores a value; it is in force only after `set_active` of that plan.
   pub trait PlanIo {
       fn active_plan(&mut self) -> Result<Guid, Failure>;
       fn plans(&mut self) -> Result<Vec<Guid>, Failure>;
       fn can_write(&mut self, key: SettingKey) -> Result<(), Failure>;
       fn read(&mut self, plan: Guid, key: SettingKey) -> Result<u32, Failure>;
       fn write(&mut self, plan: Guid, key: SettingKey, value: u32) -> Result<(), Failure>;
       fn set_active(&mut self, plan: Guid) -> Result<(), Failure>;
   }

   /// The journal file, the RunOnce value and the guard, one call per step, so the routine owns their order.
   pub trait SafetyNet {
       fn load_journal(&mut self) -> JournalRead;
       /// Atomic, write-through.
       fn save_journal(&mut self, journal: &JournalFile) -> Result<(), Failure>;
       fn delete_journal(&mut self) -> Result<(), Failure>;
       /// Moves a corrupt file to `journal.json.corrupt`.
       fn set_aside_journal(&mut self) -> Result<(), Failure>;
       /// `true`: this call added the value.
       fn set_run_once(&mut self) -> Result<bool, Failure>;
       fn clear_run_once(&mut self) -> Result<(), Failure>;
       /// Starts a guard if none runs or the current one is stopping. `true`: this call started it.
       fn ensure_guard(&mut self) -> Result<bool, Failure>;
       /// Sets the current guard's stop event.
       fn stop_guard(&mut self) -> Result<(), Failure>;
   }
   ```

3. Under A, the routine signatures, bodies `unimplemented!("<card>")` until the core cards fill them:

   ```rust
   // ownership/restore.rs
   pub enum Found { Nothing, Entries, SetAside, OtherOwner(JournalOwner) }
   pub struct Restored { pub found: Found, pub result: Result<(), Failure>, pub journal: Vec<JournalEntry> }
   pub fn restore(plans: &mut impl PlanIo, net: &mut impl SafetyNet, plan: Option<Guid>,
                  expect_owner: Option<JournalOwner>) -> Restored;
   // ownership/apply.rs
   pub struct Applied {
       pub written: Vec<SettingKey>,
       pub result: Result<(), ApplyError>,
       pub journal: Vec<JournalEntry>,
   }
   pub fn apply(plans: &mut impl PlanIo, net: &mut impl SafetyNet, me: JournalOwner, plan: Guid,
                writes: &[(SettingKey, u32)]) -> Applied;
   // ownership/check.rs
   pub struct Checked { pub user_changed: Vec<OwnedKey>, pub failures: Vec<Failure>, pub journal: Vec<JournalEntry> }
   pub fn check(plans: &mut impl PlanIo, net: &mut impl SafetyNet) -> Checked;
   ```

   The caller maps `Restored` to `Outcome::Restored` (its `plan` and `why`), `Found::SetAside` to
   `Event::JournalCorrupt`, `Found::OtherOwner` to "another tray took over: exit"; `Applied` to `Outcome::Applied`;
   `Checked` to one `UserChanged` per key.
4. `event.rs`: `Event::Started` gains `first_run: bool` ("`config.json` was missing at start: the core shows
   `Notice::FirstRun` and writes the config"). Doc: `Started`'s `config` wins over `Core::new`'s (review m7).
5. `command.rs`: `Command::Log(String)` ("a log line about a decision only the core knows: hold start and end with
   labels, a release, a countdown, a step back, a format problem at most once a minute per source; never prompts or
   file content"; the runner writes it at info level, area `core`).
6. `command.rs`: `Command::FinishRestoreRequest(StateFile)` and `FileKind::RestoreRequest` ("after
   `Done(Restored { why: UserAsked })`: write this `state.json` at once, then delete `restore-request.json`"). Same
   need as the power track's `FinishRestoreRequest`; one shape.
7. `command.rs`: `RestoreWhy::Leftover` ("a journal left at start by a dead tray; the IO side runs it only when the
   journal's owner is Gone or this tray, so Unknown never starts a restore", review m5).
8. `event.rs`: `Event::JournalCorrupt` (the power track's item); `view.rs`: `Notice::JournalCorrupt` (review M4).
9. `view.rs`: `Notice::IdleLocked` (Keep screen on idle lock; Question 8's answer).
10. Doc only, `command.rs`: `Hibernate`: "the hibernate thread plays `Cue::GoingToSleep` after its last check; the core
    never sends `Play(GoingToSleep)`" (review M5). `Retake`: "answers as `Outcome::Requests`" (review m7). `Apply`:
    "`writes`: the three keys less the released ones; the apply restores this plan's entries for other keys in the
    same run" (power plan note 6).

Shared items other tracks list, which this track uses: `text.rs` with its string signatures (tray track; `present`
calls them); `holds/file.rs` with `format_expires` (sources track; `p2-core-holds` calls it); the ship track's
`UpdateReady`, `ApplyUpdate`, `Outcome::Update` (handled in `p2-core-lid` if taken).

### What this track needs from other tracks

| From | What | When | Until then |
|---|---|---|---|
| Orchestrator | Open item 1 decided | before the contracts card | nothing starts |
| Sources | `format_expires` filled (`p4-hold-files`) | before `p2-core-holds` (wave 2) | the contract stub compiles |
| Sources | facts as agreed: `shell` as `Working`, `dialog open` as `Idle`, timers as due times with no margin, Copilot waiting with `remote: false` | integration | hand-built `SessionFacts` and `HoldFacts` |
| Power | runs `Log`, `FinishRestoreRequest`, `Leftover` (owner check); posts `JournalCorrupt`, `Started.first_run`; under A implements `PlanIo` and `SafetyNet` | integration | hand-fed events and `Done` outcomes; `FakePlans` |
| Tray | wording in `text.rs` | any time | the contract stub wording |
| Ship | the update contract items | before `p2-core-lid` | the item is skipped |

### What other tracks need from this track

| Track | What | After |
|---|---|---|
| Power (A) | `ownership::restore`; `apply` and `check`; `FakePlans` for caller tests | `p2-ownership-restore`; `p2-ownership-apply` |
| Sources (CLI `wakeshot restore`), Ship (hooks) | `ownership::restore` (A) | `p2-ownership-restore` |
| Tray | `present` builds the `TrayView` it draws | `p2-present` (hand-built views until then) |
| Integration | `Core` with every handler | `p2-core-lid` |

### Design proposals for `architecture.md` (section, new text)

- P1, section 2, core modules (A): `settings` row drops "`plan_apply`, `plan_restore`". New rows: `ownership` |
  "traits `PlanIo` (plan calls) and `SafetyNet` (journal file, RunOnce, guard); `apply`, `restore`, `check`: the one
  apply, restore routine and ownership check every process runs; `fake::FakePlans` (test-support)"; `present` |
  "`TrayView` and `StateFile` from the core's state; every word from `text`"; `text` | "every user-facing string (the
  tray track words it)"; `state`, `sim` | "the core's state (private); the `Sim` harness (test-support)". `policy` row:
  "pure rules: `requests`, `lid`, `lock`, `floor`, `ending`, `sessions` (agent sessions to holds), `hold_files` (hold
  files to holds)". Windows `power::plan` row: "trait `PowerApi` with `Win32Power` and `DryRunPower`; `PlanIo` over
  it; `apply()` and `restore_from_journal()` take `Local\Wakeshot.Journal` and call `ownership::apply` and
  `ownership::restore`; each `ReadPlan` calls `ownership::check`".
- P2, section 2 "Boundary traits" (A): add `pub trait PlanIo { /* active_plan, plans, can_write, read, write,
  set_active */ }` and `pub trait SafetyNet { /* journal load, save, delete, set aside; RunOnce set, clear; guard
  ensure, stop */ }` with "core: the steps of the apply, the restore routine and the check, one call each".
- P3, section 3: `Started { ..., first_run }`; new `Event::JournalCorrupt`; `Command::Log(String)`,
  `Command::FinishRestoreRequest(StateFile)`; `RestoreWhy` adds `Leftover`; `Notice` list adds `JournalCorrupt`,
  `IdleLocked`. Episode flow, Keep screen on: "`Lock` and `Notify(IdleLocked)` when input idle passes ...". Countdown:
  replace "When due: `Hibernate` (section 5), with the "going to sleep" sound right before it." with "When due:
  `Hibernate` (section 5); the hibernate thread plays the "going to sleep" sound after its last check, and the core
  sends no `Play` for it." Restore retry: after "the hold's `Apply` takes over the entries on the active plan" add
  "and restores those whose key the user released".
- P4, section 4, `wakeshot restore` row: "the tray runs the same path as the menu item; after the restore the core
  sends `FinishRestoreRequest`, which writes `state.json` and then deletes the request file; the CLI waits up to 5 s
  ...".
- P5, section 6, `state.json` row: "tray, on change (1 s debounce, a core deadline; at once after a `UserAsked`
  restore)".
- P6, section 8, after the table: "The core's own log lines (hold start and end with labels, releases, countdowns,
  step back, format problems at most once a minute per source) come out as `Command::Log`."
- P7, section 9 (A): replace "Windows layer unit tests for the IO thread's ownership check, `Apply` and the restore
  routine, against `DryRunPower` with a scripted plan:" with "Core unit tests for `ownership::check`, `apply` and
  `restore` against `ownership::fake::FakePlans` (stored and in-force values, a call log, a script of failures, stops
  and changes by call index):", and add "A property test over journal entry states: any sequence of Apply, user
  change, plan switch, failed read, failed write and stop, followed by a restore, never drops an entry whose value
  Wakeshot still holds." Windows layer gets: "`PlanIo` and `SafetyNet` implementations, call by call, against the plan
  copy and a temp folder."
- P8, section 3 decisions table (A): rows 24, 25 and 26, Phase "2, 3".

### Open items

1. Who owns the routines (above). Everything else in this plan holds either way.
2. Contract items 4 to 10 for the contracts card; items 6 and 8 overlap the power track's list (one shape each).
3. Two conditional items in `p2-core-lid`: held notices (tray track proposal) and updates (ship track proposal).
4. Proposals P1 to P8 for whoever edits `architecture.md` next.
