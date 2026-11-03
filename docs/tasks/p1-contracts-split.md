# Split the contracts by owner and fix the Phase 1 review findings

Phase: 1. Status: done. Needs Haik: no. Agent: `wakeshot-worker`. Size:
about 150K tokens; if the planners' contract changes make it bigger, split by crate and run the parts one at a time.
Runs after: `p2-plan-core`, `p3-plan-windows-power`, `p4-plan-sources`, `p5-plan-tray-ux`, `p6-plan-ship`, and both
Phase 1 reviews (done). Parallel with: nothing that writes `crates/`, `Cargo.toml`, `Cargo.lock`, `.github/` or
`CLAUDE.md`. It runs alone on those. Build folder: `CARGO_TARGET_DIR=<scratchpad>\p1-contracts-split\target`.

## Goal

Make the five tracks able to build in parallel with zero shared files: every module the design and the track plans
name exists, shared contract files are split by owner, and the seams the reviews ask for exist. Fix the Phase 1
general review's findings in the same pass.

## Read first

- `docs/reviews/p1-contracts-domain.md` (1 blocker, 5 major, 8 minor).
- `docs/reviews/p1-skeleton-general.md` (0 blocker, 0 major, 8 minor).
- The "contract changes" lists in the Results of the five track plan cards (`docs/tasks/p2-plan-core.md` to
  `docs/tasks/p6-plan-ship.md`), and the file ownership in their cards.
- `docs/tasks/p1-contracts.md` Result; `docs/architecture.md` sections 2, 3, 5, 6, 7; `docs/plan.md` "Git".

## Deliverables

1. Every module file the design and the track cards name exists and is declared, so no track edits a module list,
   `lib.rs`, `main.rs`, `Cargo.toml` or `Cargo.lock`. Shared files split so each file has one owning track (as the
   track cards name them). Where two track cards claim one file, stop and list it for the orchestrator.
2. The seams from the domain review: a type for threads to post events to the loop (hidden window, IO queue); apply
   and restore signatures with journal file, RunOnce and guard seams; a `FakePower` that tells stored values from
   values in force and can stop or change a value mid-routine, so the stop-point test can be written.
3. The planners' contract changes, each applied or listed with the reason.
4. The general review's minors: `--locked` in the `CLAUDE.md` check commands; a clippy run without `--all-targets`
   in CI (ships without `test-support`); a `cargo tree` step proving the core has no `windows` dependency; the rest
   as the review says.
5. `docs/architecture.md` sections 2 and 3 updated to match the code (this card owns the file this round).

## Acceptance checks

- `cargo fmt --all --check`; `cargo clippy --workspace --all-targets --locked -- -D warnings` and the same without
  `--all-targets`, on x64 and ARM64; `cargo test --workspace --locked`; release builds on both targets.
- A table in the Result: every file under `crates/` with its owning track (or "integration").
- No logic beyond constructors, serde and fakes.

## Out of scope

- Track logic. Git. `spikes/`. Phase files and track cards (list needed card edits in the Result).

## Questions for Haik

Each with the worker's default.

None.

## Found outside scope

Problems seen but not fixed.

- `docs/architecture.md` sections 4 to 11 still lack the planners' proposals (power D1 to D10, core P4 to P7, sources
  3 to 6, tray D2 to D7, ship 2 to 7). Section 9 still says the Windows layer tests `Apply` and the restore routine
  against `DryRunPower`; those tests are now the core's, against `FakePlans`. This card changed sections 2 and 3 only.
- `docs/phases/05-tray-ux.md` track note 4 lists the checks without `--locked` and without the clippy run that skips
  `--all-targets`; other track cards may do the same (general review minor 1 asks for `--locked` in every card).
- `wakeshot-tray/tests/recovery.rs` (`p3-recovery`) is an integration test of a bin crate: it can run the built exe,
  but it cannot call the functions in `guard.rs` or `restore.rs`.
- The first `cargo test` of this card sat for 7 minutes at 0.6 s of CPU with no `rustc`; the cause was not found. Later
  runs with `--offline` (every crate was already in the local cache) ran normally. Whether the first one waited on
  the network: not verified.
- The CI `dumpbin` pick on the `windows-11-arm` image: whether it has a `HostARM64` folder is not verified; the step
  falls back to the first hit, which runs there either way (ARM64 runs x64 exes).

## Log

- 2026-11-03 16:55: card written by the orchestrator after both Phase 1 reviews.
- 2026-11-03 17:22: worker started; reading reviews and the five track plan Results.
- 2026-11-03 17:50: core done (module files, `ownership` seams and signatures, `FakePlans`, `files::read`, new
  variants); 24 core tests pass. Windows crate, bins, `build-resources.rs` and a placeholder icon written. Found: a
  folder under `crates/` without `Cargo.toml` breaks the member glob, so the shared build file is
  `crates/build-resources.rs`. Next: workspace checks, CI, `CLAUDE.md`, architecture sections 2 and 3.
- 2026-11-03 17:56: CI, `CLAUDE.md`, `rust-toolchain.toml` comment, architecture sections 2 and 3, research section
  done. All acceptance checks pass. Result written; Status done.

## Result

Done 2026-11-03 17:56. Every module the design and the five track plans name exists and is declared; each file has one
owner, named in its first doc line. No track needs to edit a module list, a `main.rs` dispatch, a `Cargo.toml` or
`Cargo.lock`. No logic beyond constructors, serde, path names, the `files::read` version rule and the fakes; every
other body is `unimplemented!("<card>")` or `todo!("<card>")`.

### Checks (own build folder, no `RUSTFLAGS`, `--offline` because every crate was cached)

| Check | Outcome |
|---|---|
| `cargo fmt --all --check` | exit 0 |
| `cargo clippy --workspace --all-targets --locked --target <t> -- -D warnings`, x64 and ARM64 | exit 0, both |
| the same without `--all-targets`, x64 and ARM64 | exit 0, both (its first run caught a fake-only import, fixed) |
| `cargo test --workspace --locked` | exit 0, 36 tests: core 24, windows 10, cli 1, tray 1 |
| `cargo build --release --workspace --locked --target <t>`, x64 and ARM64 | exit 0; PE `0x8664` and `0xAA64`, 226 to 233 KB; the app icon's three frames found in all four exes |
| `cargo tree -p wakeshot-core -e normal --target all --prefix none --locked` | no `windows` crate (list in research) |
| `cargo doc --no-deps --workspace` | no warning (the broken `testing` link is gone) |
| `Cargo.lock` | not changed (last write 16:30, before this card) |

### What changed

- Core: new `ownership/` (`io.rs` with `PlanIo`, `SafetyNet`, `JournalRead`; `restore.rs`, `apply.rs`, `check.rs`
  with the contract signatures; `fake.rs` with a working `FakePlans`; `journal.rs`, `props.rs`), `policy/` (7 files),
  `present.rs`, `state.rs`, `sim.rs`, `scenarios/` (4 files), `text.rs`, `holds/file.rs`, `sources/{claude/{mod,
  status, transcript}, codex, copilot, cron}.rs`. Changed: `lib.rs`, `command.rs`, `event.rs`, `view.rs`,
  `process.rs`, `sources.rs`, `holds.rs`, `files.rs`, `core.rs` (doc), `testing.rs`.
- Windows: new `sink.rs` (`EventSink`, `FakeEventSink`), `log.rs`, `store.rs`, `process.rs`, `io_queue.rs` (contract
  signatures and working fakes), `power/{journal, executor, hibernate, requests, system, stop_points}.rs`,
  `recovery.rs`, `session.rs`, `watch/` (6 files), `lockowner.rs`, `localtime.rs`, `child.rs`, `shell.rs`,
  `install.rs`, `hooks.rs`, `update.rs`, `tray/` (5 files), `examples/power_check.rs`. Changed: `lib.rs`,
  `power/mod.rs`, `power/plan.rs` (`PowerApi` removed, `PlanSetting` kept), `sound.rs`, `notify.rs`, `tray.rs`,
  `testing.rs` (re-exports plus `FakeMenuView`), `Cargo.toml`.
- Tray bin: `main.rs` (dispatch, `velo::startup()` first), new `guard.rs`, `restore.rs`, `velo.rs`; `build.rs`.
- CLI bin: `main.rs` (the frozen dispatch), new `command.rs`, `args.rs` (today's logic moved in), `run.rs`,
  `keep_awake.rs`, `automatic.rs`, `status.rs`, `restore.rs`, `open_log.rs`; `build.rs`.
- New `crates/build-resources.rs` (both exes' resources, by `include!`); new `assets/wakeshot.ico` (placeholder ring).
- Bins' `Cargo.toml`: comments only. `.github/workflows/ci.yml`, `CLAUDE.md`, `rust-toolchain.toml` (comment),
  `docs/architecture.md` (header, sections 2 and 3), `docs/research.md` (new section "Contracts split checks").

### Planners' contract changes

Applied, all: core 1 to 10; Windows power C1 to C10; sources 1 to 12; tray 1 to 8; ship 1, 2, 3, 5 and 6. Left out:
ship 4 (`Event::Updated`, `Notice::Updated`; waits for Haik's answer to ship Question 3) and ship 7 (`velopack`, a solo
Phase 6 card). Correction 12 as the Windows planner decided: written into the `JournalKey` and `JournalFile` docs.

The three Windows items that differed from the core plan, merged:

1. `PlanIo::values()` is in the core trait, with the doc "for `Event::Plan`; the routines never call it", so the
   stop-point call indexes stay the routines' own. `PowerApi`, `FakePower`, `PowerCall` and the Windows
   `FAKE_MISSING` are gone: one plan seam (`PlanIo`), one fake (`FakePlans`, which also has `FAKE_MISSING`).
2. The locked entry points are in `wakeshot-windows/src/power/journal.rs`: `restore_locked`, `apply_locked`,
   `check_locked`, each the core routine's arguments plus `names: &Names, log: &dyn Log`, with the same return type;
   a lock not taken comes back as the routine's failed result. The core's `ownership` doc says every caller goes
   through them.
3. `Event::ActivePlan(Guid)` added, with the rule "the core sends `ReadPlan` when it differs from the last read and
   never acts on it alone".

Where this card's shape differs from a plan's text (design calls, each with its reason and trigger):

- `FakePlans` is a cheap clone handle over one shared state: the routines take the two seams as two `&mut`
  arguments, so one struct cannot be passed twice. Tests pass two clones; both log into one call list. `Arc<Mutex>`,
  so the executor and guard tests may move it to another thread. `Stop` panics at the call (dev and test builds
  unwind), as a crash would, and the next start runs against the same state. Change it when a test needs a stop
  that the routine under test must not be able to catch.
- `AutoStart::get` and `remove` return `Result` (the plan wrote bare values): every call into Windows in this crate
  returns `Result<_, WinError>` (`error.rs`). `DataDir::dry_run` returns `Result` too. `Names::real` and `dry_run` are
  filled from section 4's names; `Names::test` waits for `p3-store`.
- `text.rs` placeholders are built from the values (`format!("{:?}")`), not the product sketch's words, so nobody
  mistakes them for final wording. Change it when `p5-text` is done (it replaces them).
- `files::read::<T>` added (domain review m2): first JSON value only, bytes after it ignored (sources item 5), a
  UTF-8 byte order mark skipped (Windows PowerShell 5.1 writes one with `-Encoding UTF8`, per Microsoft docs; not
  re-checked today). The old test that blessed a newer file now tests `read`.
- `SoundEnd` lives in core `command.rs` (it is part of `Outcome`). `FakeSoundPlayer` has no `Debug` (it holds an
  `Arc<dyn EventSink>`).
- The CLI's `command.rs` carries `#[expect(dead_code)]` until `p4-cli-args` builds every variant; that card must
  remove that one attribute (card edit 4 below).
- The shared build file is `crates/build-resources.rs`, not the review's `crates/build/resources.rs`: a folder there
  without a `Cargo.toml` breaks the workspace (research).

### Reviews

- Domain review: B1 (module split), M1 (`EventSink`, `IoQueue`, `Send` check on `Event` and `Command` in the core),
  M2 (seams and signatures), M3 (`FakePlans`: stored and in-force values, call log with arguments, script of `Fail`,
  `Stop`, `Run` by call index), M4 (`JournalCorrupt` event and notice), M5 (`play_and_wait`, `Hibernate` doc), m1 to
  m5 and m7 through the planners' items and `files::read`, m6 left to the power track (each of its modules has its
  own dry-run form, note 8; only the integration step picks them), m8 (CI comment and the clippy run without `--all-targets`).
- General review: 1 (`--locked` in `CLAUDE.md`), 2 (CI and `CLAUDE.md` clippy without `--all-targets`), 3 (doc link),
  4 (`cargo tree` step in the Linux job), 5 (`dumpbin` host pick), 7 (shared build file), 8 (`CLAUDE.md` wording,
  `rust-toolchain.toml` comment). 6: none now, as the review says.

### Card edits the orchestrator needs to make

1. `p2-ownership-restore`: `fake.rs` already holds a working `FakePlans` (API in its doc and `architecture.md`
   section 2); deliverable 2 becomes "extend it" (still missing: who set each stored value). Its public API is
   contract now: add, never change. `Stop` is a panic (chosen here; reason above).
2. `p3-power-plan`: `PowerApi` and `FakePower` are gone; `Win32Power` and `DryRunPower` implement the core's `PlanIo`
   directly, `values()` included (no local trait needed).
3. `p3-journal`: the three locked signatures exist as stubs in its file; it fills them and adds `FileNet`.
4. `p4-cli-args`: may and must remove the one `#[expect(dead_code, ...)]` attribute in
   `crates/wakeshot-cli/src/command.rs` once `args::parse` builds every command.
5. `p4-cli-restore`, `p6-hooks` (and `docs/phases/04-sources.md`, `06-ship.md`): depend on `p3-journal`
   (`restore_locked`) and `p2-ownership-restore`, not the dropped `p3-restore`.
6. `p4-hold-files`, `p4-watch-data`, `p6-hooks`: read data files with `wakeshot_core::files::read`.
7. `p5-sounds`, `p5-notices`: `FakeSoundPlayer` and `FakeNotifier` now live in their files (`sound.rs`, `notify.rs`),
   re-exported from `testing.rs`; their API is contract.
8. `p5-icon-art`: the placeholder `assets/wakeshot.ico` exists and both exes embed it through
   `crates/build-resources.rs`; replacing the file needs no build change. No card edits `build.rs` or that file.
9. Every track card and `docs/phases/05-tray-ux.md` note 4: checks with `--locked`, plus the clippy run without
   `--all-targets` (as in `CLAUDE.md`).
10. `docs/phases/02-core.md` module layout: `ownership/fake.rs` first version from the contracts card.

No file is claimed by two track cards: checked against the owner tables in `docs/phases/02` to `06`.

### File owners (every file under `crates/`)

"Contracts" means a contract file: no track edits it; a change goes through a contracts card the orchestrator runs.

| File | Owner |
|---|---|
| `build-resources.rs` | contracts |
| `wakeshot-core/Cargo.toml` | contracts |
| `wakeshot-core/src/lib.rs`, `command.rs`, `event.rs`, `files.rs`, `holds.rs`, `process.rs`, `settings.rs`, `sources.rs`, `time.rs`, `view.rs`, `testing.rs` | contracts |
| `wakeshot-core/src/core.rs` | core: `p2-core-holds`, then `p2-core-settings`, then `p2-core-lid` (bodies only; signatures contract) |
| `wakeshot-core/src/state.rs` | core: `p2-core-holds`, then `p2-core-settings`, then `p2-core-lid` |
| `wakeshot-core/src/sim.rs` | core: `p2-core-holds`, then `p2-core-settings`, `p2-core-lid`, `p2-core-props` |
| `wakeshot-core/src/present.rs` | core: `p2-present` |
| `wakeshot-core/src/policy/mod.rs` | contracts |
| `wakeshot-core/src/policy/sessions.rs`, `hold_files.rs` | core: `p2-policy-holds` |
| `wakeshot-core/src/policy/requests.rs`, `lid.rs`, `lock.rs`, `floor.rs`, `ending.rs` | core: `p2-policy-power` |
| `wakeshot-core/src/ownership/mod.rs`, `io.rs` | contracts |
| `wakeshot-core/src/ownership/restore.rs`, `journal.rs` | core: `p2-ownership-restore` (signatures contract) |
| `wakeshot-core/src/ownership/fake.rs` | core: `p2-ownership-restore`, then `p2-ownership-apply` (API contract) |
| `wakeshot-core/src/ownership/apply.rs`, `check.rs` | core: `p2-ownership-apply` (signatures contract) |
| `wakeshot-core/src/ownership/props.rs` | core: `p2-ownership-props` |
| `wakeshot-core/src/scenarios/mod.rs` | contracts |
| `wakeshot-core/src/scenarios/holds.rs`, `settings.rs`, `lid.rs`, `props.rs` | core: `p2-core-holds`, `p2-core-settings`, `p2-core-lid`, `p2-core-props` |
| `wakeshot-core/src/text.rs` | tray: `p5-text` (signatures contract) |
| `wakeshot-core/src/holds/file.rs` | sources: `p4-hold-files` (two signatures contract) |
| `wakeshot-core/src/sources/claude/mod.rs`, `status.rs` | sources: `p4-claude-status` (`mod.rs` then `p4-claude-transcript`) |
| `wakeshot-core/src/sources/claude/transcript.rs` | sources: `p4-claude-transcript` |
| `wakeshot-core/src/sources/codex.rs`, `copilot.rs`, `cron.rs` | sources: `p4-codex-source`, `p4-copilot-source`, `p4-cron` |
| `wakeshot-windows/Cargo.toml`, `src/lib.rs`, `error.rs`, `sink.rs`, `tray.rs`, `testing.rs`, `power/mod.rs`, `watch/mod.rs` | contracts |
| `wakeshot-windows/src/store.rs`, `log.rs`, `process.rs` | power: `p3-store`, `p3-log`, `p3-process` (signatures and fakes contract) |
| `wakeshot-windows/src/power/plan.rs` | power: `p3-power-plan` |
| `wakeshot-windows/src/power/requests.rs` | power: `p3-requests` |
| `wakeshot-windows/src/power/system.rs`, `clock.rs` | power: `p3-system-events` |
| `wakeshot-windows/src/session.rs` | power: `p3-session` |
| `wakeshot-windows/src/power/journal.rs` | power: `p3-journal` (three signatures contract) |
| `wakeshot-windows/src/power/hibernate.rs` | power: `p3-hibernate` |
| `wakeshot-windows/src/power/executor.rs`, `io_queue.rs` | power: `p3-power-io` (`IoQueue` signatures contract) |
| `wakeshot-windows/src/power/stop_points.rs` | power: `p3-stop-points` |
| `wakeshot-windows/src/recovery.rs` | power: `p3-recovery` |
| `wakeshot-windows/examples/power_check.rs` | power: `p3-harness` |
| `wakeshot-windows/src/watch/dir.rs`, `read.rs`, `data.rs`, `agents.rs`, `thread.rs` | sources: `p4-watch-folders`, `p4-watch-reads`, `p4-watch-data`, `p4-watch-agents`, `p4-watch-thread` |
| `wakeshot-windows/src/lockowner.rs` | sources: `p4-watch-reads` |
| `wakeshot-windows/src/localtime.rs` | sources: `p4-cron` |
| `wakeshot-windows/src/child.rs` | sources: `p4-cli-run` |
| `wakeshot-windows/src/shell.rs` | sources: `p4-cli-commands` |
| `wakeshot-windows/src/tray/theme.rs`, `art.rs`, `menu_paint.rs`, `icon.rs`, `menu.rs` | tray: `p5-theme`, `p5-icon-art`, `p5-menu-paint`, `p5-tray-icon`, `p5-menu-view` |
| `wakeshot-windows/src/notify.rs` | tray: `p5-notices` (trait and fake contract) |
| `wakeshot-windows/src/sound.rs` | tray: `p5-sounds` (trait and fake contract; may add `sound/*.rs`) |
| `wakeshot-windows/src/install.rs`, `hooks.rs`, `update.rs` | ship: `p6-login-and-path`, `p6-hooks`, `p6-update-schedule` |
| `wakeshot-tray/Cargo.toml` | contracts; `p6-velopack-dependency` (run alone) |
| `wakeshot-tray/build.rs`, `wakeshot-tray.exe.manifest`, `src/main.rs`, `src/mode.rs` | contracts (integration fills tray mode) |
| `wakeshot-tray/src/guard.rs`, `restore.rs` | power: `p3-recovery` |
| `wakeshot-tray/src/velo.rs` | ship: `p6-velopack-dependency` |
| `wakeshot-cli/Cargo.toml`, `build.rs`, `wakeshot.exe.manifest`, `src/main.rs`, `src/command.rs` | contracts (`command.rs`: one attribute line to `p4-cli-args`) |
| `wakeshot-cli/src/args.rs` | sources: `p4-cli-args` |
| `wakeshot-cli/src/run.rs` | sources: `p4-cli-run` |
| `wakeshot-cli/src/keep_awake.rs`, `automatic.rs`, `status.rs`, `open_log.rs` | sources: `p4-cli-commands` |
| `wakeshot-cli/src/restore.rs` | sources: `p4-cli-restore` |

Outside `crates/`: `assets/wakeshot.ico` belongs to tray `p5-icon-art`. New test, example and fixture files the
track cards name (`wakeshot-core/tests/`, `wakeshot-windows/examples/ux_*.rs`, `agent-watch.rs`,
`wakeshot-tray/tests/recovery.rs`) are created by their own cards; cargo finds them without a `Cargo.toml` change.

### Open items

- Ship Question 3 (update notice): if yes, `Event::Updated` and `Notice::Updated` need one more contracts change.
- `velopack`: the solo `p6-velopack-dependency` card.
- The card edits above, and the architecture proposals for sections 4 to 11 (Found outside scope).
