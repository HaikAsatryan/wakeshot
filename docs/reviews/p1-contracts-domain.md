# Phase 1 contracts: domain review

Reviewer: `wakeshot-reviewer`, 2026-11-03. Narrow brief: can five tracks build in parallel against `crates/` without
editing the same files, and can the types and fakes carry every restore path and its stop-point test.

Read: `crates/` (all files), `docs/tasks/p1-contracts.md` (Result, "Found outside scope"), `docs/architecture.md`
sections 2 to 9 as they stood at 16:45 (after `p1-design-verify-fixes` round 3), `docs/plan.md` (Phases note, Git),
the Phase 1 work log. Line numbers in `architecture.md` are from that read; the docs worker may move them.

Ran (own `CARGO_TARGET_DIR`, no `RUSTFLAGS`): `cargo test --workspace --locked` (25 passed);
`cargo tree -p wakeshot-core -e normal,build --target all`; `cargo tree -p wakeshot-tray -p wakeshot-cli -e
normal,features -i wakeshot-core` (and `-i wakeshot-windows`); `cargo build -p wakeshot-tray --all-targets -v`; a
scratch crate outside the repo that checks `Event` and `Command` are `Send`.

Counts: 1 blocker, 5 major, 8 minor. "Blocker" here means: fix in the planned contracts change card, before the track
builds start.

## Blocker

### B1. Four tracks must edit contract files to do their own work

`plan.md` lines 51 to 53: two running cards never own the same file, and the contracts change only through a card the
orchestrator runs alone. The code and the design put work of different tracks into the same files:

| File | Tracks that must edit it | Why |
|---|---|---|
| `crates/wakeshot-core/src/holds.rs` | 2 (`Hold`), 4 (hold file parsing, `expires`) | `architecture.md` line 58 puts both in `holds`; `holds.rs` lines 1 to 2 say the same |
| `crates/wakeshot-core/src/settings.rs` | 2 ("settings ownership rules", `plan.md` line 74), 3 (`plan_apply`, `plan_restore`, Result "Not in code") | `architecture.md` line 61 puts both in `settings` |
| `crates/wakeshot-core/src/view.rs` | 2 (builds `TrayView`, `MenuModel` with their texts), 5 ("notice and menu wording in `view`", Result) | `view.rs` lines 1 to 2: all user-facing text is built in this module |
| `crates/wakeshot-core/src/lib.rs` | 2 (`pub mod policy`, `architecture.md` line 60) | module list is a contract file |
| `crates/wakeshot-core/src/sources.rs` | 4 (`pub mod claude; codex; copilot;`) | the only place those modules can be declared |
| `crates/wakeshot-windows/src/lib.rs` | 3 (`session`, `process`, `store`, `log`), 4 (`watch`) | `lib.rs` lines 6 to 11 list only today's modules |
| `crates/wakeshot-windows/src/power/mod.rs` | 3 (`requests`, `system`) | `mod.rs` line 4 |
| `crates/wakeshot-tray/src/main.rs`, `mode.rs` | 3 (guard and restore modes; checklist 4 and 5 need the real exe), 5 (tray mode for the on-screen checks), 6 (Velopack call first) | `main.rs` lines 10 to 15 dispatch every mode in one `match`; `mode.rs` line 18 leaves guard arguments to Phase 3 |

What breaks: the planners either give one file to two running cards (lost or conflicting edits), or each track stops
for a solo contract card in the middle of its work. The work log already gives `store`, `log` and `process` to Phase 3
(phase file lines 494 to 495), but their `pub mod` lines still go into a file Phase 4 also edits.

Fix, in the contracts change card:

- Declare every module the design names now, as an empty file with one doc line naming its owner: core `policy`,
  `sources::{claude, codex, copilot}`; windows `power::{requests, system}`, `session`, `process`, `watch`, `store`,
  `log`. Then no track touches a module list.
- Split by owner where two tracks share a module: hold file parsing in its own file (Phase 4); `plan_apply` and
  `plan_restore` in their own file with their signatures (Phase 3), or move them into `wakeshot-windows`; notice and
  menu strings in their own file (Phase 5) apart from the view structure (Phase 2), or rule that one track writes all
  strings.
- Tray bin: write the `main.rs` dispatch now, one file per mode (`guard.rs`, `restore.rs` for Phase 3; `tray.rs` for
  Phase 5), each a stub until its track fills it.

## Major

### M1. No contract for posting events to the loop, for the loop's window, or for the IO queue

- Off-loop producers in four tracks post `Event`s to the loop window (`architecture.md` lines 423 to 425): IO thread,
  system callbacks and hibernate thread (Phase 3), watch thread (Phase 4), sound thread and `MenuView` (Phase 5). No
  type exists for it: a search of `crates/` for post, sender or channel finds only doc text. `tray.rs` line 17 and
  `sound.rs` lines 8 to 9 promise a post with nothing to post through, so `FakeSoundPlayer` cannot produce
  `Done(Outcome::Sound)` and its test cannot show the real side's result path.
- The hidden window's procedure takes messages for Phase 3 (`WM_QUERYENDSESSION`, `WM_ENDSESSION`,
  `WM_WTSSESSION_CHANGE`) and Phase 5 (icon callback, `TaskbarCreated`, `WM_DISPLAYCHANGE`, `WM_SETTINGCHANGE`). No
  owner is named; both tracks need the same function.
- The loop hands commands to the IO thread and must put the end-of-session restore at the front and drop queued
  notices and `Apply` commands (`architecture.md` lines 428 to 431). That queue sits between the wiring and Phase 3
  and has no type.

What breaks: each track writes its own `PostMessage` wrapper with its own boxing; one that boxes `Event` differently
from the loop's unboxing is a use-after-free or a leak. Or Phases 3, 4 and 5 wait for Phase 6 to build the loop.

Fix: in `wakeshot-windows`, `pub trait EventSink: Send + Sync { fn post(&self, event: Event) -> Result<(), WinError>; }`
with a recording `FakeEventSink`; name the owner of the window and its procedure and give the other track a handler
hook; type the IO queue (`push`, `push_front`, `drop_queued(..)`). Add a compile-time `Send` check on `Event` and
`Command`: both are `Send` today (checked), and a later variant with an `Rc` would break every poster.

### M2. `apply()` and `restore_from_journal()` have no signature and no seam for the journal file, RunOnce and guard

- Other tracks call the routine: the CLI's `wakeshot restore` when the owner is dead (Phase 4, `architecture.md` line
  409) and the Velopack hooks (Phase 6, lines 748 to 751). Restore step 7 (lines 574 to 579) needs an "expected owner"
  input for the guard and restore mode, and the log needs a reason. None of this is in code.
- `PowerApi` (`power/plan.rs` lines 42 to 49) is the only seam. Apply steps 3 and 4 write the HKCU RunOnce value and
  start a guard process; restore step 6 deletes the value and sets the guard's stop event. A unit test of those steps
  against `FakePower` writes the real RunOnce value on Haik's laptop and starts real guards. Section 9 (lines 861 to
  868) asks for "a `SafetyNet` failure at each step (no entry left behind)", which needs a scripted failure of the
  journal write, the RunOnce write and the guard start; nothing can script them.
- The dry run must never load, write or delete the real journal and never touch Run or RunOnce (lines 438 to 439).
  `PowerApi` cannot switch any of that.

Fix, in the contracts change card: a trait for the safety net (journal `load` returning missing, corrupt or entries;
atomic `save`; `delete`; RunOnce set and clear; guard start and stop) with a fake that records each call and can fail
any one, and the two signatures, for example `apply(power, net, plan, writes) -> Outcome` and
`restore_from_journal(power, net, scope: Option<Guid>, expect_owner: Option<JournalOwner>, why) -> report`.

### M3. `FakePower` cannot run the stop-point test

The types can carry every state the restore paths need: `JournalEntry` and `JournalFileEntry` hold `plan: Guid`,
`key`, `original`, `written` and `restored`; `JournalOwner` holds pid, start and session; all derive `Eq` and `Debug`,
so a test can assert them. The fake cannot drive the test:

- No "in force" model. `write` stores the value; `apply` only sets `active` and pushes to `applied` (`testing.rs`
  lines 120 to 133); writes are not logged, so the order of writes and applies is lost. Scenario: the restore writes
  `original` and stops before `PowerSetActiveScheme`. Stored is `original`, in force is still `written`. A buggy
  routine that drops this marked entry passes every assertion on `plans`, though Windows still runs with Do nothing.
  Section 9 asks for exactly this check: "no entry still in force is dropped".
- No stop. `fail_next` returns an error, and the routine's error path then runs (Apply step 6 runs the restore
  routine). A crash runs nothing. There is no way to stop after call N.
- Failures by kind only, on the next call of that kind (`testing.rs` lines 75 to 86): it cannot fail the second write,
  or `can_write` for one key.
- No change between calls: a plan switch between the writes and the call, and a user change between Apply steps 2
  and 5 (both in the section 9 list), need the fake to change `active` or a value in the middle of the routine.

Fix: `in_force` per plan (a copy of the stored values at each `apply` of that plan), a call log with arguments, and a
script keyed by call index with actions Fail(code), Stop (a panic the test catches; dev and test builds unwind,
`architecture.md` line 441) and Run(closure on the fake's state). Or record that Phase 3 may extend `FakePower`
without a contract card.

### M4. A corrupt journal has no signal and no notice

- Restore step 1 (`architecture.md` lines 547 to 552) moves a corrupt journal aside and warns; section 8 (line 825)
  says "warning notice". `Notice` (`view.rs` lines 85 to 97) has no such variant, and the design's list (lines 231 to
  233) has none either.
- The core cannot learn it. `Started.journal` is `Option<Journal>` (`event.rs` line 42): `None` for missing and for
  corrupt. `Outcome::Restored.result` is `Result<(), Failure>`, and a moved-aside file is not a failed call.
- Restore mode at sign-in, the guard and the hooks find it most often, and none of them has UI.

What breaks: the lid action stays Do nothing and battery sleep Never; the next `Apply` takes Do nothing for the user's
own value, so a lid close gets no lock either; the user is never told, only the log has it.

Fix: a `Notice` variant for it; `Started.journal` as a three-way value (none, entries, corrupt found); and a marker the
next tray start can see (for example the `journal.json.corrupt` file itself, cleared once the notice shows).

### M5. Who plays the "going to sleep" sound, and how the hibernate thread waits for it

- Section 3 (lines 317 to 318): `Hibernate` "with the going to sleep sound right before it", which reads as the core.
  Section 5 (line 643): the hibernate thread plays it after its last check, and "Phase 5 sets how long the thread
  waits for it". The `Command::Hibernate` doc (`command.rs` lines 62 to 63) says neither.
- `SoundPlayer::play` never blocks and returns nothing (`sound.rs` lines 10 to 12), so the hibernate thread cannot
  wait for the sound's end, and no constant carries Phase 5's wait.

What breaks: the core track's property test "every `Hibernate` comes with a notice or a sound" (line 860) pushes it to
send `Play(GoingToSleep)` beside `Hibernate`. If the thread plays it too: two chimes. If only the core plays it and
the last check skips (lid opened, input seen): a "going to sleep" chime and no sleep.

Fix: the `Command::Hibernate` doc says the hibernate thread plays `Cue::GoingToSleep` after its last check and the core
never sends it; `SoundPlayer` gets a waitable form (a done handle, or `play_and_wait(cue, max)`), with the fake to
match; the property test treats `Hibernate` as its own explanation.

## Minor

### m1. `wakeshot-windows/Cargo.toml` lacks what two cards already need

- `Wdk_System_SystemInformation`: the system process list fallback ports `NtQuerySystemInformation`
  (`spikes/claude-watch/src/probe.rs` lines 13 and 274; the spike's `Cargo.toml` has the feature). It is not in
  `crates/wakeshot-windows/Cargo.toml` lines 23 to 56, so Phase 3 must edit the file the card says it need not.
- `proptest` as a dev-dependency here: the Windows track plan card must have a property test over journal entry
  states (phase file lines 482 to 483); only the core declares it.
- The bins' comment "the tracks add features here" (`wakeshot-tray/Cargo.toml` line 10, `wakeshot-cli/Cargo.toml`
  line 14) invites Phases 3 and 5 to edit the same tray `Cargo.toml`. Feature unification already gives the bins every
  feature the lib turns on.

Fix: add both now; say the bins call Windows only through the lib, or name one owner of each bin's `Cargo.toml`.

### m2. The newer-version rule lives in no function, one test blesses the opposite, and the journal rewrite is open

- `files.rs` lines 3 to 6: readers read `VersionProbe` first and ignore a newer file. `added_fields_are_ignored`
  (lines 303 to 313) asserts that a version 2 `config.json` parses into `Config` with `version: 2`. Readers sit in
  three tracks (watch thread: config, holds, restore request; CLI: state; hooks), and each must repeat the probe. One
  that forgets accepts a newer file, and the tests agree with it.
- The contract does not say which `version` a journal rewrite writes. `JournalFile` keeps the version it read (the
  test at lines 317 to 335 round-trips 2) and drops unknown fields. Scenario: a later version adds a required entry
  field; the user downgrades (the design allows it); this version's restore marks entries and fails half way, rewriting
  `version: 2` without the field; the user upgrades; the later version cannot parse it, moves it aside as corrupt, and
  the override stays (M4).
- No split or merge between `JournalFile` and `Journal`: the `Other` entries survive only if every writer remembers to
  put them back. Correction 12 (what restore does with them) is still with the power planner.

Fix: one `files::read::<T>(bytes)` returning the value, `Newer(version)` or `Bad`, with a test per file type; turn the
added-fields test into a test of that function; a rewrite always writes `JournalFile::VERSION`; "fields added later
are optional"; `JournalFile` split and merge helpers.

### m3. The core must format `expires`, but that format belongs to Phase 4

`Command::WriteHold { hold: HoldFile }` (`command.rs` lines 92 to 95) carries `expires: Option<String>` (`files.rs`
line 63), built by the core for "Keep awake 30 minutes". `architecture.md` lines 724 to 725 give the parse and format
to Phase 4, and the CLI (Phase 4) writes the same file. Phase 2 either writes a second RFC 3339 formatter or waits.
Fix: `WriteHold` carries `expires_wall_ms` and the IO side formats, or declare `format_expires` and `parse_expires`
signatures now with Phase 4 as owner.

### m4. Nothing deletes `restore-request.json`, so "state first, then delete" cannot be expressed

`architecture.md` line 409: the tray deletes the request file, then the CLI reads `state.json`, which has a 1 s
debounce (line 705). No `Command` deletes the request and `FileKind` (`command.rs` lines 47 to 51) has no variant for
it. The IO thread cannot build a fresh `StateFile` itself (the core builds it, `files.rs` line 109), so it deletes
first and `wakeshot restore` prints the state from before the restore. Routed to the power planner (phase file line
494), but every fix changes the contract, for example a command the core sends after `Done(Restored { why: UserAsked
})` that writes `state.json` and then deletes the request.

### m5. A journal left at start has no restore reason and no owner liveness

Tray startup step 6 (`architecture.md` lines 447 to 449): restore when the owner is Gone; Unknown: retry on each tick.
`Started` carries the journal but not its owner's liveness, and `RestoreWhy` (`command.rs` lines 23 to 31) has no
reason for a leftover. If the core retries, it cannot keep "Unknown never starts a restore" (line 586). Fix: say in
the `Started` doc that the startup code owns that retry, or add the owner's `Alive` to `Started` and a
`RestoreWhy::Leftover`.

### m6. Power requests, lock and hibernate have no seam for the dry run

Section 9 (line 882): the dry run logs requests, lock and hibernate instead of doing them. `PowerApi` holds plan calls
only; requests and lock run on the loop thread, hibernate on its own thread. A dry-run tray built on `PowerApi` alone
would lock the screen and hibernate the test laptop. Fix: a small trait for requests, retake, lock and hibernate with a
real and a dry-run form, or a dry-run flag each takes, named in the contract.

### m7. Small gaps in `Command` and `Core`

- `Retake` has no `Outcome`. A failed retake after resume should give "cannot keep the PC awake" (section 8). Say it
  answers as `Outcome::Requests`.
- `Core::new(Config)` (`core.rs` line 20) and `Started { config }` (`event.rs` line 41) both carry the config. Say
  which wins, or drop one.
- The open choice "a `Restore` with keys (contract) or `Apply` restoring entries outside its writes" (phase file lines
  504 to 507) changes `Command::Restore` if the first is picked. Decide it before the core and power tracks start.

### m8. `test-support` reaches a normal exe when it is built with `--all-targets`

Checked: `cargo build -p wakeshot-tray --all-targets -v` builds `wakeshot_core` and `wakeshot_windows` once each, with
`feature="test-support"`, and the normal tray bin links those builds. The CI release step (`ci.yml` line 64) is a plain
`cargo build --release --workspace`, so the exes it ships are clean, and unused fakes would be dropped at link time
anyway. Fix: one line in the Phase 6 pack card (and a comment at `ci.yml` line 63): build release exes without
`--all-targets`.

## Checked and fine

- The core's normal and build tree on every target: `serde`, `serde_derive`, `serde_core`, `serde_json`, `itoa`,
  `memchr`, `zmij`, `proc-macro2`, `quote`, `syn`, `unicode-ident`. No `windows` crate. `#![forbid(unsafe_code)]` at
  `wakeshot-core/src/lib.rs` line 7.
- In normal builds both bins see only the `default` feature of both libs.
- Every file type (`Config`, `HoldFile`, `RestoreRequest`, `StateFile`, `JournalFile`) has `version`, a round-trip test
  and a test of the documented JSON form. `procStart` rejects a JSON number. GUID text round-trips in Windows order.
- One error type, `WinError`, a struct: no shared error enum for tracks to extend.
- `Event` and `Command` are `Send` (scratch crate check).
- Every restore reason in the episode flow maps to `Command::Restore` and `Outcome::Restored`; the core's journal
  mirror can be fed only from `Done(Applied)`, `Done(Restored)` and `UserChanged`, as section 3 says.
