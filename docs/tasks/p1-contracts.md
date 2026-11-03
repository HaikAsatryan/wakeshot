# Put the contracts between the parts into code

Phase: 1. Status: done. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 100K tokens.
Runs after: `p1-cargo-workspace`. Parallel with: `p1-ci` (it writes only `.github/`) and `p1-tray-menu-look` (a
spike outside the workspace). Use your own `CARGO_TARGET_DIR` (in your scratch folder) so your builds do not block
other workers' builds.

## Goal

Phases 2 to 6 run as parallel tracks (`docs/plan.md`, note under the Phases table). They can only do that if the
types and traits between the parts exist as code first. Put the boundary types and traits from `architecture.md`
into the skeleton crates as compiling code with no logic, plus a fake for each boundary, so every track can build
and test against them on its own.

## Read first

- `docs/plan.md`: the note under the Phases table.
- `docs/architecture.md`: section 2 (crates, modules, "Boundary traits"), section 3 ("Events, state, commands",
  "Clocks"), section 6 (files and their formats), section 7 (the interface a source implements), section 11
  (dependencies).
- `docs/tasks/p1-cargo-workspace.md`: Result (what the skeleton has).

## Deliverables

1. In `wakeshot-core`: the event, state and command types; the clock trait with a fake clock; the file format types
   (hold file, config, journal and any other in section 6), each with its version field and serde derives; the
   source interface from section 7. Names and fields as in `architecture.md`.
2. The boundary traits from section 2 in the crate the design puts them in, each with a fake in a test-support
   module, so a track can test its side without the other.
3. A doc comment on each public type and trait: which track owns it and what the other side may rely on. Why, not
   what.
4. Tests: a serde round trip for every file format type (including a newer `version` value, if the design says how
   that is handled); one test per fake showing it can stand in for the real side.
5. If writing the code shows the design is wrong or vague somewhere, fix `architecture.md` in the same change and
   list each such edit in the Result. Product behavior is out of scope: a gap there goes under Questions for Haik.

## Acceptance checks

- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` pass for the workspace on x64,
  and `cargo build` passes for ARM64 (`aarch64-pc-windows-msvc`).
- `wakeshot-core` still builds with no Windows code (check the way the workspace card set up).
- No logic beyond constructors, serde and the fakes: no Windows API calls, no state machine rules.
- Every type and trait named in `architecture.md` sections 2, 3, 6 and 7 exists in code, or is listed in the Result
  with the track that adds it and why.

## Out of scope

- The state machine rules, policies, Windows calls, parsers (the tracks build those).
- CI files (`p1-ci`), repo files (`p1-repo-files`), `spikes/`.
- Git. No repo exists until the orchestrator creates it.

## Questions for Haik

Each with the worker's default.

None. Every gap found is a design point, listed in the Result for routing.

## Found outside scope

Problems seen but not fixed.

- Track planning: `wakeshot-windows` `store` (atomic write, data folder paths), `log` (`LogWriter`) and `process`
  (liveness) are needed by Phases 3, 4 and 5 at the same time. The track cards should name one owner for each, or have
  Phase 3 build them first; otherwise two tracks write the same file.
- `wakeshot restore` (section 4): the CLI waits for `restore-request.json` to go, then reads `state.json`, which has a
  1 s debounce, so it can print the state from before the restore. The IO thread should write `state.json` before it
  deletes the request file (Phase 3 or 4).
- `CLAUDE.md` Status is still true ("skeleton only, no product logic yet"), but a reader would not know the contracts
  exist. Suggested addition to that paragraph: "The contracts between the parts are in code (`p1-contracts`), with
  fakes behind the `test-support` feature." Not edited: not mine this round.

## Log

- 2026-11-03 15:52: card written by the orchestrator after Haik asked why Phases 2 to 6 cannot run in parallel
  (they can, once the contracts exist as code).
- 2026-11-03 16:14: `wakeshot-worker` started the card. Status in progress. Orchestrator note: this round I own
  `crates/`, root `Cargo.toml` and `Cargo.lock`; `architecture.md` belongs to `p1-design-verify-fixes`, so design
  corrections go into the Result for routing, not into the file.
- 2026-11-03 16:24: read the card, plan, architecture sections 2, 3, 5 (apply, restore), 6, 7, 11 and the
  workspace Result. Plan: core types, `Clock` and the source trait in `wakeshot-core`; the four Windows traits in
  `wakeshot-windows`; fakes behind a `test-support` feature in both; every dependency the tracks need declared now
  (serde, serde_json, proptest, both libs in both bins, the `windows` features from the spikes), so no track touches
  `Cargo.lock`. Internal core state (`State`, `Episode`, `Countdown`, `Hold`) stays out: a contract would freeze it.
- 2026-11-03 about 16:25: orchestrator note: `p1-design-verify-fixes` done (16:21). Re-read sections 2, 3, 5 (apply,
  restore), 6, 7, 11: the six changed types it named were already in the version I read; nothing else changed for
  the contracts. Section 2 now says the skeleton bins depend on neither lib; this card adds them (correction listed).
- 2026-11-03 16:28: velopack probe (scratch project, `cargo add velopack@=1.2.161`): 113 packages on x64, with
  `ring`, `rustls`, `ureq`, `zip`. Not declared now (every tray build in every track would compile it). Core written:
  `time`, `process`, `settings`, `holds`, `sources`, `files`, `view`, `command`, `event`, `core`, `testing`.
  `cargo test -p wakeshot-core`: 16 passed.
- 2026-11-03 16:31: Windows side written: `error`, `power::plan`, `tray`, `notify`, `sound`, `testing`, and
  `SystemClock` in `clock`. Both bins now depend on both libs and `windows` (`Win32_Foundation`), with the
  `test-support` fakes as dev-dependencies. `cargo fmt --all` rewrapped two test lines.
- 2026-11-03 16:33: checks (own `CARGO_TARGET_DIR`, no `RUSTFLAGS`): see Result. All pass.
- 2026-11-03 16:34: research section "Contracts build" appended. A script found a doc comment on all 70 public types
  and traits.
- 2026-11-03 16:37: orchestrator note (architecture round 2: no shape change; `ApplyError::Write` now means the
  restore routine already ran, Apply step 6). `ApplyError` doc updated to match. Re-ran fmt check, clippy `-D warnings`
  on both targets and `cargo test --workspace`: all exit 0, 25 tests. Status done.

## Result

Done 2026-11-03 16:37. The contracts compile on both targets with no logic, and every acceptance check passes.
`ApplyError` follows architecture round 2 (`Write`: the restore routine already ran).

### Files

- `Cargo.toml`: `wakeshot-windows`, `serde` 1.0.229 (derive), `serde_json` 1.0.154 and `proptest` 1.12.0 added to
  `[workspace.dependencies]`. `Cargo.lock`: regenerated by cargo (serde, serde_json, proptest trees).
- `crates/wakeshot-core`: `Cargo.toml` (feature `test-support`; serde, serde_json; dev proptest). `src/lib.rs`,
  `time.rs` (`Deadline`, trait `Clock`), new `process.rs`, `settings.rs`, `holds.rs`, `sources.rs`, `files.rs`,
  `view.rs`, `command.rs`, `event.rs`, `core.rs` (`Core` stub: `handle` returns no commands, `next_deadline` None),
  `testing.rs` (`FakeClock`, `FakeSource`).
- `crates/wakeshot-windows`: `Cargo.toml` (feature `test-support`, serde_json, 32 `windows` features from the spikes'
  lists for code v1 ports, so Phases 3 and 5 need not edit it). `src/lib.rs`, `clock.rs` (`SystemClock`), new
  `error.rs` (`WinError`), `power/mod.rs`, `power/plan.rs` (`PowerApi`, `PlanSetting`), `tray.rs` (`MenuView`,
  `Anchor`), `notify.rs` (`Notifier`), `sound.rs` (`SoundPlayer`), `testing.rs` (`FakePower`, `FakeMenuView`,
  `FakeNotifier`, `FakeSoundPlayer`).
- `crates/wakeshot-tray/Cargo.toml`, `crates/wakeshot-cli/Cargo.toml`: both libs, `windows`, dev fakes.
- `docs/research.md`: section "Contracts build" (velopack weight, feature isolation, core tree, unknown journal keys).

### Checks

- `cargo fmt --all --check`: exit 0.
- `cargo clippy --workspace --all-targets --target <t> -- -D warnings`: exit 0 for x64 and ARM64.
- `cargo test --workspace` (x64): exit 0, 25 tests: core 16 (serde round trip for every file type, the documented
  JSON forms, newer `version` handling, GUID text, FakeClock, FakeSource), windows 7 (FakePower, FakeMenuView,
  FakeNotifier, FakeSoundPlayer, three clock), tray 1, cli 1.
- `cargo build --workspace --target aarch64-pc-windows-msvc`: exit 0. Release builds, both targets: exit 0, PE machine
  `0x8664` and `0xAA64`, sizes identical to the workspace card's.
- Core without Windows code: `cargo tree -p wakeshot-core -e normal` lists only serde, serde_json and their deps; for
  `x86_64-unknown-linux-gnu` with dev-deps, no `windows` crate. On the Windows host, dev-only `proptest` brings
  `windows-sys` into test builds.
- Fakes stay out of normal builds: `-e normal,features` shows only `default` for both libs in both bins.
- No logic: no Windows call added (only `SystemClock` calling the existing `now()`), no rules; GUID and FILETIME text
  forms are serde.

### Not in code, with the track that adds it

- Core track (Phase 2): `State`, `Episode`, `Countdown`, `Hold`, module `policy`, the `Sim` harness. Internal to the
  core; as contracts they would need a contract card for every change.
- Power track (Phase 3): `plan_apply`, `plan_restore` (the pure halves of the apply and restore routine; their shape
  follows that routine), `Win32Power`, `DryRunPower`, `apply()`, `restore_from_journal()`, `power::requests`,
  `power::system`, `session`, `process::liveness()`, `input_idle_ms()`, `store`, `LogWriter`.
- Sources track (Phase 4): hold file parsing (with `expires`), `sources::claude`, `codex`, `copilot`, `watch`.
- Tray track (Phase 5): `TrayIcon`, `BalloonNotifier`, the WASAPI player, notice and menu wording in `view`.
- Not declared: `velopack` (113 packages with `ring`, `rustls`; every tray build in every track would compile it; Phase
  6 adds it in a card the orchestrator runs alone) and the cron crate (Phase 4 has not picked).

### Design corrections for `architecture.md` (routed by the orchestrator; not edited here)

1. Section 2, paragraph under the crate table: replace "The skeleton bins depend on neither lib yet; the first card
   that wires a bin adds them." with "Both bins depend on both libs and on `windows` from `p1-contracts`, so no track
   changes `Cargo.lock`; `velopack` comes in Phase 6 through a card the orchestrator runs alone." In the table,
   `wakeshot-windows` depends on `wakeshot-core`, `windows`, `serde_json` (the IO thread reads and writes the files).
2. Section 2, core modules: new row `process` | `ProcessRef { pid, start: FileTime }`, `Alive { Yes, Gone, Unknown }`,
   `FileTime` (FILETIME; a decimal string in files). Windows `process` row becomes "`liveness(ProcessRef) -> Alive`
   (types from the core), exit waits, ...". Why: `HoldFacts.alive`, `Liveness::Process` and `Started.tray` need them
   without the `windows` crate.
3. Section 2, core `time` row: add "trait `Clock { fn now(&self) -> Now }`; `testing::FakeClock`". Windows `clock`
   row: add "`SystemClock` (implements `Clock`)".
4. Section 2, core `settings` row: add `Guid(u128)` (text lowercase `8-4-4-4-12`; the bit order of
   `windows::core::GUID::from_u128`) and `JournalOwner`. `files` row: add `VersionProbe`, `JournalFileEntry`,
   `JournalKey { Known(SettingKey), Other(String) }`. `view` row: add `MenuItem { Label(text), Separator, Action { text,
   command, checked, enabled }, Submenu { text, items } }`, `KeepAwake { Off, Minutes(u32), UntilOff }`, and
   `MenuCommand { Automatic(bool), KeepAwake(KeepAwake), KeepScreenOn(bool), Restore, OpenLog, Quit }`.
5. Section 2, windows modules: new row `error` | `WinError { call, code }`, `From<WinError> for Failure`. `power::plan`
   row: add `PlanSetting { LidAc, LidDc, SleepAc, SleepDc, SaverThresholdDc }` (reads need AC sleep and the saver
   threshold for `PlanValues`; writes stay limited to `SettingKey`).
6. Section 2, "Boundary traits" block. Replace the `PowerApi` and `Notifier` lines with:
   `pub trait PowerApi { fn active_plan(&mut self) -> Result<Guid, WinError>; fn plans(&mut self) ->
   Result<Vec<Guid>, WinError>; fn read(&mut self, plan: Guid, setting: PlanSetting) -> Result<u32, WinError>; fn
   write(&mut self, plan: Guid, key: SettingKey, value: u32) -> Result<(), WinError>; fn apply(&mut self, plan: Guid)
   -> Result<(), WinError>; fn can_write(&mut self, key: SettingKey) -> Result<(), WinError>; }` with the note "one IO
   thread owns it, so `&mut self`; `plans` is the whole list or an error". And `pub trait Notifier { fn show(&self,
   notice: &Notice) -> Result<(), WinError>; }` with "the result becomes `Outcome::Notified`" (without it nothing could
   fill `Notified`). In the `AgentSource` comment add "`WantedFile`: `Folder { dir, accept: fn(&str) -> bool }` (the
   name filter runs before any open) or `Tail(path)`; `FileId(PathBuf)`".
7. Section 3, `Event`: `Started { config, journal, owner, on_console, tray: ProcessRef }`, with "tray: this tray's
   own process, for `state.json` and the Keep awake hold". `Sources { source, sessions: Vec<(SessionFacts, Alive)> }`,
   with "each session paired with the adapter's liveness answer; without it the core cannot tell Gone from Unknown".
8. Section 3, `Command`: `Hibernate { reason: CountdownReason, floor_pct: u8, last_resume_mono_ms: Option<u64> }`,
   with "what the hibernate thread's last check needs (section 5)". New `WriteHold { id: HoldId, hold: HoldFile }`,
   with "Keep awake from the menu; no command wrote `holds\keep-awake.json`". New types: `CountdownReason { WorkEnded,
   Floor }`; `HibernateSkip { LidOpen, LidUnknown, InputSeen, OnAc, AboveFloor }` (the `why` of `Skipped`); `FileKind
   { State, Config, Hold(HoldId) }` (`Written` also answers `WriteHold` and `DeleteHold`).
9. Section 3, `Notice`: three variants carry data: `UserChanged(SettingKey)`, `ClosedLidReport { ran_ms, held_by }`,
   `UpdateWakeshot { agent }`.
10. Section 7, `SessionFacts`: `Waiting { since_wall_ms: Option<i64>, remote }`, with "None when the file has no
    `statusUpdatedAt`: the parser has no clock, so the core keeps the time it first saw waiting". `Liveness::Process {
    pid, start: FileTime }`; `AnyProcess { image: &'static str, lock_file: PathBuf }`; `Unreadable(&'static str)` (a
    fixed text, so file content cannot reach the log).
11. Section 6, after "A file with a newer version is ignored and logged": add "The reader reads `VersionProbe` first,
    so a newer file whose fields changed shape is still recognized as newer." Hold format: add "`HoldFile.expires`
    stays text; Phase 4 parses it (and formats it for Keep awake). No date crate is declared: Phase 4 writes the RFC
    3339 parse and format or picks `jiff`, its cron candidate."
12. Section 6, journal, open point for the power track: an entry whose key this version does not know (written by a
    newer version) cannot be restored here. Kept forever, the journal never empties, so RunOnce and the guard stay;
    dropped, that override is never restored. Proposed default: keep it in the file untouched, log it, and let restore
    step 6's clean-up count only known entries. Also: this version drops unknown entry fields when it rewrites the
    journal (fine while later versions read older forms).
13. Section 11: `serde_json` used in core and `wakeshot-windows`; `windows` in both exes (declared, `Win32_Foundation`
    only); `proptest` declared as a core dev-dependency; `velopack` not declared yet (113 packages, research "Contracts
    build"). Add: "Both libs have a `test-support` feature with the fakes; only dev-dependencies turn it on."

### Open items

- The design corrections above (orchestrator routes them; the code already follows them).
- Correction 12 needs a decision by whoever plans the power track; the type already keeps unknown keys either way.
- See "Found outside scope" (owners for `store`, `log`, `process`; `wakeshot restore` and the state debounce; a
  `CLAUDE.md` line).
