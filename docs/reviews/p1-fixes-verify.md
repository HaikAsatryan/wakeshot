# Phase 1 close: verify the contracts split and the track plan fixes

Reviewer: `wakeshot-reviewer`, 2026-11-03, about 18:10 to 18:40. Read only. No registry, power or screen use.

Brief: (1) check `p1-contracts-split` against `docs/reviews/p1-contracts-domain.md` and
`docs/reviews/p1-skeleton-general.md`, plus its owner claims; (2) check `p1-track-plan-fixes` against
`docs/reviews/p2-6-cross-track.md`, its "Runs after" names and the 10 card edits from `p1-contracts-split`.

Note: other agents wrote files while this review ran (`p1-close-contracts.md` 18:13, `p6-plan-integration.md` and
`p4-claude-live-shapes.md` 18:17). Line numbers are from my reads; `p1-close-contracts` may move them.

Counts: 0 blocker, 2 major, 6 minor. Known open (not counted): cross-track M10 and m9.

What I ran (scripts in the scratchpad, `p1-fixes-verify\`; build folder deleted at the end):

- `owners.py`: every `.rs` file under `crates/`, its module doc, the cards it names, and whether each named card exists
  in `docs/tasks/` and names the file.
- `links.py`: every "Runs after" of the 63 p2 to p6 build cards, and every `pN-name` in those cards and the 02 to 06
  phase files, against the card list.
- `cycles.py`: cycles in "Runs after", and whether given card pairs are ordered or may run together.
- `apis.py`, `apis2.py`: every Windows API name in the p3 to p6 cards, phase files and `architecture.md`, looked up in
  the `windows` 0.62.2 source, against the features in `crates/wakeshot-windows/Cargo.toml`.
- In a scratch copy of the workspace (not the repo): `cargo metadata --locked --offline` before and after adding
  `proptest.workspace = true` to the CLI's dev-dependencies (Major 1).

## Major

### Major 1. Two track cards need a `Cargo.toml` change the contracts did not make

- `docs/tasks/p4-cli-args.md:42`: "A `proptest` property: for random argument lists quoted the MSVC way, `run_tail`
  returns exactly the quoted child part." `run_tail` lives in `crates/wakeshot-cli/src/args.rs`. The CLI has no
  `proptest` (`crates/wakeshot-cli/Cargo.toml:25-27`: dev-dependencies are only the two libs; `Cargo.lock:545-552`:
  the `wakeshot-cli` entry has no `proptest`). Checked in a scratch copy: with `proptest.workspace = true` added,
  `cargo metadata --locked --offline` fails with "cannot update the lock file ... because --locked was passed". So
  the card's own `--locked` checks fail the moment it adds the dev-dependency.
- `docs/tasks/p4-cli-run.md:27`: `child::command_line() -> OsString` (`GetCommandLineW`) in
  `crates/wakeshot-windows/src/child.rs`. `GetCommandLineW` is in `windows::Win32::System::Environment`, behind the
  feature `Win32_System_Environment` (`windows-0.62.2/src/Windows/Win32/System/mod.rs:35`). That feature is not in
  `crates/wakeshot-windows/Cargo.toml:24-60`. A feature does not change `Cargo.lock`, but the file is a contract file.
  It was the only API name in the p3 to p6 cards whose feature is missing (`apis.py`).
- What breaks: `CLAUDE.md` and `architecture.md` section 2 promise that no track edits a `Cargo` file. Two sources
  cards cannot meet their acceptance without one: the worker stops for a solo contracts card mid-track, edits a
  contract file beside other running cards, drops the property test quietly, or hand-writes an `extern "system"`
  block for `GetCommandLineW`.
- Fix, in `p1-close-contracts` (it runs alone): add `"Win32_System_Environment"` to the `windows` features of
  `wakeshot-windows`; add `proptest.workspace = true` to `[dev-dependencies]` of `wakeshot-cli` and let cargo add the
  one line to the `wakeshot-cli` entry of `Cargo.lock` (no new package: `proptest` is already locked). That card's
  check "`Cargo.lock` unchanged" then becomes "only the `wakeshot-cli` dependency list gains `proptest`". The other
  way, if the lock must not move: `p4-cli-args` replaces the property with a fixed table of generated cases.

### Major 2. The two property cards make mutants in files they do not own, while other cards build and test against them

- Where: `docs/tasks/p2-ownership-props.md:65` ("Mutants, each made by hand in `ownership/` (restore, apply or check),
  run, then reverted") and `docs/tasks/p2-core-props.md:62` ("Mutants, each made by hand in `core.rs` or
  `src/core/`"). Those files belong to `p2-ownership-restore`, `p2-ownership-apply` and the `p2-core-*` chain. Both
  cards say "Parallel with: any card of another track" (`p2-ownership-props.md:5`, `p2-core-props.md:5`), and their
  own check rules say "Only `ownership/props.rs` changed" (`p2-ownership-props.md:80`) and "Never edit ... that file".
- Who runs beside them (`cycles.py`, not ordered by "Runs after"): `p2-ownership-props` with `p3-recovery`,
  `p3-stop-points`, `p3-real-checks` and `p2-core-props`; also `p3-power-io`, `p4-cli-restore` and `p6-hooks`, which
  test through the routines. `p2-core-props`' `Sim` runs the real routines (`p2-core-props.md:9-10`), so a restore
  mutant from `p2-ownership-props` fails its properties too.
- What breaks: (1) every card whose tests reach the mutated code while it sits in the shared tree (the cards above
  through `restore_locked` and the executor, `p2-core-props` through `Sim`) sees false failures, logs them and waits; (2) `p3-stop-points` (a plan copy) and `p3-real-checks` (an active plan copy, a guard
  exe built from the tree, 50 cycles) can build and run a mutant restore routine on the real laptop, for example
  mutant 4 "the restore drops marked entries when `set_active` fails"; (3) a worker stopped mid-mutant (session limit)
  leaves the mutant in the tree, and the orchestrator's commit by path does not show it. This is the "two writers of
  one file" case the split was meant to remove.
- Fix: in `docs/phases/02-core.md` note 20 and in both cards: "Mutants run only in a scratch copy of the workspace
  (`Cargo.toml`, `Cargo.lock`, `.cargo\`, `rust-toolchain.toml`, `crates\` copied to `<scratchpad>\<card>\mutants\`,
  with its own `CARGO_TARGET_DIR`), never in the repo." The same rule costs little for the cards that mutate their
  own files (`p2-ownership-restore.md:96`, `p2-ownership-apply.md:79`, `p2-core-settings.md:85`, `p2-core-lid.md:76`).

## Minor

### Minor 1. Cards still wait on questions Haik answered at 18:09

Haik's answer (`docs/phases/02-06-signoff.md:27`): all defaults. These spots still say the card waits, may change,
or builds something "only if":

- `docs/tasks/p2-core-lid.md:49-51`, item 8a: "Only if Haik signs off the tray track's proposal ... Not signed off:
  skip this item and its tests." `docs/phases/02-core.md:61-63`, note 18: "if signed off". Sign-off item 4 is yes,
  but nothing the card reads says so. A worker who follows the card skips a feature Haik approved. This is the one
  spot that changes what gets built.
- `docs/tasks/p2-core-settings.md:3-5` and `:113-114`; `docs/phases/02-core.md:126` (Phase 1 Question 10 "is open").
- `docs/tasks/p3-recovery.md:3`, `:45-46`, `:92`; `docs/phases/03-windows-power.md:129` (Question 12 "open").
- `docs/tasks/p3-haik-checks.md:66`: "Run D and E now ... or at integration?" (sign-off item 10: Phase 3).
- `docs/tasks/p4-cli-restore.md:3`, `:74-76`; `docs/phases/04-sources.md:119` (Question 12 "may add a line").
- `docs/tasks/p6-pack-and-release.md:3-4`, `:56-57`; `docs/phases/06-ship.md:141` (ship Question 5, sign-off item 9:
  yes, a public pre-release).
- `docs/tasks/p6-velopack-dependency.md:3-4` ("Needs Haik: his answer to Question 4 ... before it starts"), `:13`,
  `:55-56`; `docs/phases/06-ship.md:90-91`, `:176-177` ("if Haik says yes to Question 4"). The answer is yes (add the
  clang component), so the install is now a planned step, but no card owns it and it needs an elevated install on the
  real laptop with no cargo running. Without an owner it is easy to skip, and then the tray's ARM64 checks fall back
  to "CI only".
- `docs/phases/04-sources.md:193` ("Each with the default the cards take until you answer"), `03-windows-power.md:99`
  and `04-sources.md:19` ("Haik may change them at sign-off"): harmless, but stale.

Fix: `p1-close-contracts` deliverable 4 already names most of these (cards and phase file tables). It does not name
`p3-haik-checks` or `p6-pack-and-release`: add them. Give the clang install an owner: one small real-laptop card, or a
named step of `p6-velopack-dependency` that the orchestrator runs (elevated) before it.

### Minor 2. 17 files name no owner, though `lib.rs` and `architecture.md` say every file does

- Claim: `crates/wakeshot-core/src/lib.rs:5` and `crates/wakeshot-windows/src/lib.rs:4-5` ("Each module's first doc
  line names the card that owns the file"), `docs/architecture.md:45`, `CLAUDE.md` Status, `p1-contracts-split`
  Result line 85. The new `p6-plan-integration.md` Read first relies on it too.
- `owners.py`: no module doc names any owner in core `command.rs`, `core.rs`, `event.rs`, `files.rs`, `holds.rs`,
  `process.rs`, `settings.rs`, `sources.rs`, `testing.rs`, `time.rs`, `view.rs`, `lib.rs`; windows `clock.rs`,
  `error.rs`; tray `mode.rs`; both `build.rs`. Most are contract files in the split's table, so the default reading
  is right. Two are not: `crates/wakeshot-windows/src/clock.rs` belongs to `p3-system-events` (table line 228, 03
  table line 30) and its doc says nothing; `crates/wakeshot-core/src/core.rs:1` says only "the core track" (owner:
  `p2-core-holds`, then `p2-core-settings`, then `p2-core-lid`). Also, where a file has an owner line, it is the third
  doc line, not the first.
- Same family: `crates/wakeshot-windows/src/install.rs:1` ("The start-at-login Run value and the CLI folder in the
  user Path") and `docs/architecture.md:96` ("the Run value and the CLI folder") still give `install` the Run value.
  Cross-track m12 made `store::AutoStart` its one writer (`p6-login-and-path.md:16-17`, `06-ship.md:22`).
  `p1-close-contracts` deliverable 3 adds a section 6 line but does not name these two.
- What breaks: a worker who opens `clock.rs` or `core.rs` cannot tell from the file who may edit it; the
  `p6-login-and-path` worker reads a file doc that contradicts its card.
- Fix: one line per file. "Owner: contracts card." in each contract file; "Owner: `p3-system-events`." in `clock.rs`;
  the three-card chain in `core.rs`; `install.rs:1` and `architecture.md:96` without the Run value. Change the claim
  to "the module doc names the owner".

### Minor 3. `p3-recovery` does not wait for `p3-power-plan`, but builds `Win32Power`

- `docs/tasks/p3-recovery.md:47`: `guard.rs` and `restore.rs` "build the real parts (`Win32Power`, ...)". `Win32Power`
  does not exist yet: `crates/wakeshot-windows/src/power/plan.rs` holds only `PlanSetting`; `p3-power-plan` adds it.
  `p3-recovery`'s Runs after (`p3-recovery.md:6`; `03-windows-power.md:129`) has `p3-journal`, `p3-process`,
  `p3-log` and `p2-ownership-restore`, not `p3-power-plan`. `cycles.py`: the two may run together.
- What breaks: if `p3-power-plan` lags (it waits for a real-laptop slot for its `real_` tests), the tray crate does
  not compile in `p3-recovery`'s own files, and its built-exe tests (`wakeshot-tray/tests/recovery.rs`) cannot run.
- Fix: add `p3-power-plan` to `p3-recovery`'s Runs after and to the 03 table row.

### Minor 4. `p5-tray-icon` may start "with the stub art", but there is no stub art

- `docs/tasks/p5-tray-icon.md:9`: "Part A can start with the stub art if `p5-icon-art` is late".
  `crates/wakeshot-windows/src/tray/art.rs` holds only its doc lines (no function), and `p5-icon-art.md:7` says the
  contracts card creates it "as a stub".
- What breaks: if the orchestrator starts Part A before `p5-icon-art`, the card has nothing to call and must define
  art functions in `p5-icon-art`'s file.
- Fix: drop the parenthetical (it already runs after `p5-icon-art`), or have the contracts card add the art signatures.

### Minor 5. Where tray mode gets the `UpdateFeed` is said two ways

- `docs/phases/06-ship.md:25`: `startup()` "returns the `UpdateFeed` for tray mode". `crates/wakeshot-tray/src/velo.rs`
  `pub fn startup() {}` returns nothing, its doc says the owner "keeps this signature", and `main.rs` calls it as a
  statement. `p6-velopack-dependency.md:35-40` keeps the signature and also builds the feed, without saying how tray
  mode reaches it.
- What breaks: the integration planner (now running, `p6-plan-integration`) and `p6-velopack-dependency` may pick
  different shapes.
- Fix: `06-ship.md:25` and `p6-velopack-dependency.md`: "a second function, `velo::feed() -> impl UpdateFeed`, for tray
  mode; `startup()` keeps its signature".

### Minor 6. Small text errors

- `docs/phases/03-windows-power.md:69`: `cargo test -p wakeshot-windows -- --ignored --test-threads=1 <module>::` has no
  `--locked`; every other cargo command in the cards has it (edit 9).
- `docs/tasks/p1-close-contracts.md:30` names `crates/wakeshot-tray/src/text.rs`; the file is
  `crates/wakeshot-core/src/text.rs`. It has one `notice(&Notice) -> NoticeText` for all notices (line 81), whose
  placeholder formats with `{notice:?}`, so a new `Notice::Updated` needs no new signature and still compiles.

## Known open (not counted)

- Cross-track M10 (integration plan): left out by `p1-track-plan-fixes` on purpose (Result table). Since then the
  orchestrator wrote `docs/tasks/p6-plan-integration.md` (18:17, Status in progress). Still open until it is done.
- Cross-track m9 (`Event::Updated`, `Notice::Updated`): not in the code (`crates/` grep finds no `Updated`). Haik said
  yes (sign-off item 7); `p1-close-contracts` deliverable 1 adds both. Lines that change with it:
  `docs/tasks/p2-core-lid.md:53-59` (item 9a), `docs/phases/02-core.md:171`, `docs/phases/06-ship.md:78` and
  `:190-191`.
- `docs/architecture.md:612` still names `restore_from_journal`, and line 943 the old test plan name: listed by both
  cards, scheduled in `p1-close-contracts` deliverable 3.

## Part 1: contracts split

### `docs/reviews/p1-contracts-domain.md`

| Finding | Outcome | Evidence |
|---|---|---|
| B1 module split | Fixed | Every module is declared (core `lib.rs`, `sources.rs`, `holds.rs`; windows `lib.rs`, `power/mod.rs`, `watch/mod.rs`, `tray.rs`). Shared files split: `holds/file.rs` (sources), `ownership/` (core routines, power seams), `text.rs` (tray) apart from `present.rs` (core). Tray `main.rs` dispatches to `guard.rs`, `restore.rs` (power) and `velo.rs` (ship); tray mode is integration's. |
| M1 event posting, window, IO queue | Fixed | `sink.rs`: `EventSink: Send + Sync`, `FakeEventSink`. `io_queue.rs`: `push`, `push_front`, `drop_queued`, `pop`, `session_ending`, `session_resumed`. `event.rs:87-93`: compile-time `Send` check on `Event` and `Command`. Loop window: integration (`sink.rs:4-5`); `TrayIcon` has its own window (`tray.rs:4`). |
| M2 apply and restore seams | Fixed | `ownership/io.rs`: `PlanIo`, `SafetyNet` (journal load, save, delete, set aside, RunOnce, guard), `JournalRead`. Signatures in `restore.rs`, `apply.rs`, `check.rs`; locked entry points in `power/journal.rs`. The reason (`why`) stays with the caller and `Outcome::Restored`, not the routine: fine. |
| M3 `FakePower` cannot run the stop-point test | Fixed | `ownership/fake.rs`: `stored` and `in_force` per plan, a `Call` log with arguments, a script by call index (`Fail`, `Stop` as a panic, `Run` closure), a test for each. One clone handle for both seams. |
| M4 corrupt journal | Fixed another way | `Event::JournalCorrupt` (`event.rs:42-43`), `Notice::JournalCorrupt` (`view.rs:101`), `restore::Found::SetAside`, marker files `DataFile::JournalCorrupt` and `JournalCorruptSeen` (`store.rs:16-17`, `52-53`). |
| M5 going to sleep sound | Fixed | `Command::Hibernate` doc: the hibernate thread plays it, the core never sends it. `SoundPlayer::play_and_wait` (3 s cap) with the fake; `02-core.md` note 4. |
| m1 Windows `Cargo.toml` gaps | Partly | `Wdk_System_SystemInformation` added; bins' comment now says no track adds a feature there. `proptest` in the Windows crate dropped on purpose (the property test moved to the core). The same kind of gap remains elsewhere: Major 1. |
| m2 version rule | Fixed | `files::read::<T>` with `Read { Value, Newer, Bad }`, a test per file type, the old test now tests `read`; `JournalFile` doc: a save writes `VERSION`; "fields added later are optional"; split and merge in `ownership/journal.rs` (`p2-ownership-restore`). |
| m3 `expires` format | Fixed another way | `holds/file.rs`: `parse_expires`, `format_expires` as contract stubs owned by `p4-hold-files`. |
| m4 restore request delete | Fixed | `Command::FinishRestoreRequest(StateFile)`, `FileKind::RestoreRequest`. |
| m5 leftover journal | Fixed | `RestoreWhy::Leftover` and its rule (`command.rs`); `02-core.md` note 9. |
| m6 dry-run seams for requests, lock, hibernate | Fixed another way | Each power module has its own dry-run form (`requests.rs`, `session.rs` docs; `p3-hibernate` test `dry_run_never_hibernates`). |
| m7 small gaps | Fixed | `Retake` answers as `Outcome::Requests`; the `Started` config wins (`event.rs:33`, `core.rs:13`); `Apply` restores the plan's other entries (`command.rs`, `Apply` doc). |
| m8 `test-support` in exes | Fixed | `ci.yml` comments at "Clippy as shipped" and "Release build". |

### `docs/reviews/p1-skeleton-general.md`

| Minor | Outcome | Evidence |
|---|---|---|
| 1 `--locked` | Fixed | `CLAUDE.md` checks line; every card's cargo commands (one miss: Minor 6). |
| 2 clippy without `--all-targets` | Fixed | `ci.yml` "Clippy as shipped"; `CLAUDE.md`. |
| 3 doc link | Fixed | `wakeshot-windows/src/lib.rs:4` is plain code text now. |
| 4 core has no `windows` dependency | Fixed | `ci.yml` `core-linux` job: `cargo tree -p wakeshot-core -e normal --target all --prefix none --locked`, fails on a `windows` line. |
| 5 `dumpbin` host | Fixed | `ci.yml`: `$hostDir` pick, first hit as fallback. |
| 6 cache pruning | No change | As the review said (trigger: an entry over about 2 GB). |
| 7 copied `build.rs` | Fixed | `crates/build-resources.rs` by `include!`, with `rerun-if-changed` for it; the two `build.rs` differ only in their three arguments. |
| 8 text errors | Fixed | `CLAUDE.md` line 3; `rust-toolchain.toml:4` "every runner". |

### The split's own claims

- Every card named in a module doc exists in `docs/tasks/` and names that file (`owners.py`: no miss). The phase file
  owner tables (02 lines 97 to 108, 03 lines 25 to 37, 04 lines 149 to 167, 05 lines 19 to 28, 06 lines 158 to 166)
  match the split's table.
- Not every file names its owner: Minor 2.
- No two cards that may run at once own one file, by the owner lists: the hand-overs (`core.rs`, `state.rs`, `sim.rs`;
  `ownership/fake.rs`; `sources/claude/mod.rs` and `tests/claude_status.rs`; `text.rs` and `art.rs` to
  `p5-haik-check`; the `p4-live-*` fixes; `packaging/smoke-test.ps1`) are each ordered by "Runs after" (`cycles.py`).
  The exception is temporary edits outside the owner lists: Major 2.

## Part 2: track plan fixes

### `docs/reviews/p2-6-cross-track.md`

| Finding | Outcome | Evidence |
|---|---|---|
| B1 shared-crate checks | Fixed | Notes in 02 (note 20), 03 (note 2), 04 (note 18), 05 (section 7), 06 ("Checks while other cards run"); every build card's checks use `-p`, own files and a "Check rule" bullet; no `--workspace`, `fmt --all` or "all exit 0" left outside `p6-velopack-dependency` (grep). `plan.md:19-28` has the rule. |
| B2 sign-out | Fixed | `p3-haik-checks.md:5-9` runs alone on the whole laptop; 03 table line 133; `p3-real-checks.md:37` step 5. |
| M1 `p3-restore` | Fixed | No `p3-restore`, `FakePower`, `restore_from_journal` or `PowerApi` left in p2 to p6 build cards (only history: 03 work log, plan cards, `p3-power-plan.md:29` saying they are gone). `p4-cli-restore`, `p6-hooks` Runs after as the review asked; `p3-journal.md:37-38` names the CLI form. |
| M2 `Event::ActivePlan` | Fixed | `p2-core-settings.md:42-45`; 02 Coverage. |
| M3 `start_of_pid`, `list` | Fixed | `p3-process.md:33-34`, `:56`; signatures in `process.rs:29-32`. |
| M4 watch thread needs `p3-process` | Fixed | `p4-watch-thread` Runs after; `04-sources.md:72`. |
| M5 velopack runs alone | Fixed | `p6-velopack-dependency.md:6-10`; 06 order step 5. The ARM64 rule is in the phase notes; `plan.md:25-26` now has the four-card cap. The clang step still reads as conditional: Minor 1. |
| M6 `p4-cli-commands` screen | Fixed | Header line 6; deliverable 4 (`real_open_log`, ignored). |
| M7 `p3-harness` cleanup | Fixed | `p3-harness.md:3-5`; "Wakeshot test <pid>" in `p3-power-plan.md:48-50`; "Wakeshot test e2e" in `p6-install-e2e.md:37`. |
| M8 test registry keys | Fixed | `p3-store`'s `ScratchKey` and `sweep_scratch_keys` (`p3-store.md:46-47`), used by `p3-journal`, `p3-recovery`, `p3-stop-points`, `p6-hooks`, `p6-login-and-path`. |
| M9 environment tests | Fixed | `real_` and `#[ignore]` in `p3-system-events.md:49-53`, `p3-process.md:57`, `p3-session.md`; `request_sets_and_clears` renamed (`p3-requests.md:41`), still a real request of under 1 s in plain `cargo test`, kept out of `p3-real-checks` step 5. |
| M10 integration plan | Known open | See "Known open". |
| m1 stale routine owner text | Fixed | `02-core.md:25`, `:183`; no "option A/B" or "Runs only if" left in p2 cards. |
| m2 `p6-readme` Runs after | Fixed | Has `p4-live-codex`, `p4-live-copilot`, `p5-haik-check`. |
| m3 `p6-pages` | Fixed | Names `p5-icon-art` and `p5-haik-check`. |
| m4 `tests/claude_status.rs` | Fixed | `04-sources.md:153`. |
| m5 `restore_locked_round_trip` | Fixed | Moved to `p3-recovery.md:60`. |
| m6 `FakePlans` while apply runs | Fixed | `p2-ownership-apply` Owns: additions only; `p2-ownership-restore.md:52` names `values()`. |
| m7 `p2-present` fields | Fixed | `p2-present.md:28-31`; `text::Status { requests_only }` and `keep_awake_group(KeepAwake)` exist. |
| m8 `shell.rs` requests | Fixed | `p4-cli-commands.md:45-46`. |
| m9 `Updated` variants | Known open | See "Known open". |
| m10 `p6-velopack-facts` Runs after | Fixed | Header lines 6-7. |
| m11 dry-run folder | Fixed | `p6-install-e2e.md:10-11`. |
| m12 duplicate rules | Fixed | `p3-recovery` budget note gone; `p6-login-and-path.md:16-17`, `:29`. The file doc and `architecture.md:96` still say otherwise: Minor 2. |
| m13 split triggers | Fixed | Headers of `p2-core-holds`, `p2-core-settings`, `p5-tray-icon`, `p4-live-claude`. |
| m14 build load | Fixed | Phase notes (delete the target folder); `plan.md:25-26` (at most four building cards). |
| m15 `real_` filter | Fixed | Module filters in 03 note 3 and `p3-real-checks.md:28-32`. |
| m16 `art` API kept | Fixed | `p5-haik-check.md` Owns. |
| m17 `NotifyIconSettings` | Fixed | `p6-hooks.md:49`: leave them (sign-off item 12). |
| m18 stale names | Fixed | `p6-velopack-dependency.md:12-13` names `p1-contracts-split`; no `FakePower` left in `crates/`. |
| m19 live cards' lane | Fixed | `p4-claude-live-shapes.md:8-9`, `p4-live-claude.md:12`. |

### "Runs after"

- All 63 build cards have a "Runs after" line, and every card name in them exists (`links.py`). The only missing
  names: `p1-design-final-check` in the five plan cards (a review file, not a card), and history (`p3-restore`,
  `p3-apply`, `p3-power-props`, `p2-ownership`) in the 03 work log and the plan cards.
- No cycle (`cycles.py`). One missing edge: Minor 3.

### The 10 card edits from `p1-contracts-split`

All landed:

1. `p2-ownership-restore.md:40`, `:47` (extend `FakePlans`, add only, `Stop` is a panic).
2. `p3-power-plan.md:28-31` (no `PowerApi`, `FakePower`; `PlanIo` directly, `values()` included).
3. `p3-journal.md:39-42` (the three locked stubs exist; fill them, add `FileNet`).
4. `p4-cli-args.md:23` (remove the one `#[expect(dead_code)]`).
5. `p4-cli-restore`, `p6-hooks` Runs after; 04 line 119, 06 line 139 (with M1).
6. `p4-hold-files.md:24`, `:29`; `p4-watch-data.md:24`; `p6-hooks.md:33` (`files::read`).
7. `p5-sounds.md:38`, `p5-notices.md:38` (fakes in their files, API contract).
8. `p5-icon-art.md:31-32` (placeholder icon, no build change).
9. `--locked` on the cargo commands of the build cards, and the clippy run without `--all-targets` in each card's
   checks or "Check rule" bullet (grep; one miss in a phase note: Minor 6).
10. `02-core.md:110` (`ownership/fake.rs` first version from the contracts card).
