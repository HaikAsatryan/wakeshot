# Cross-track review of the Phase 2 to 6 plans

Reviewer: `wakeshot-reviewer`, 2026-11-03. Read only. Inputs: `docs/phases/02-core.md` to `06-ship.md`, all 63 cards
`docs/tasks/p2-*.md` to `p6-*.md`, the five plan cards' Results, `docs/plan.md` ("How we work", "Git", the note under
the Phases table), the orchestrator decision of 17:05 (routines in the core, generic over `PlanIo` and `SafetyNet`).
`crates/` and `docs/architecture.md` were not judged (a worker edits them now).

Counts: 2 blockers, 10 majors, 19 minors. Totals (item 7) are at the end.

## Blockers

### B1. Shared-crate checks cannot pass while other cards are mid-way, and nothing stops a worker from formatting or fixing their files

- Where: `docs/phases/04-sources.md` and `06-ship.md` have no rule for a check that fails in another card's file; every
  p4 and p6 card's Acceptance checks say "all exit 0" for `cargo test -p wakeshot-core`, `-p wakeshot-windows` or
  `--workspace`, `cargo clippy --workspace` and `cargo fmt --all --check`. The p2 cards require `cargo test -p
  wakeshot-core`: all pass and `cargo fmt --all --check`: exit 0, and carve out only "another track's files" for
  `--workspace`. Only `03-windows-power.md` note 2 (line 43) and `05-tray-ux.md` note 4 (line 110) have the rule.
- What fails: `wakeshot-core` holds the files of 10 p2 cards, 6 p4 parser cards and `p5-text`; `wakeshot-windows` holds
  files of p3, p4, p5 and p6. Every card is test-first, so for hours at a time another card's file has red tests, an
  unformatted file, or a save that does not compile. A compile error anywhere in a crate fails every `cargo test -p`
  of that crate, also with a name filter. A p2, p4 or p6 worker then stalls, reports a false failure, or "fixes" the
  other file. A worker that runs `cargo fmt --all` (no `--check`), `cargo fix` or `cargo clippy --fix` to get its own
  check green rewrites files other workers are editing: the two-writers case the plan forbids.
- Fix: one rule in `docs/plan.md`, "How we work", step 2, and a one-line pointer in each phase file:
  "Done checks of a card: its own tests (by name filter) pass; clippy and `rustfmt --check <own files>` show nothing in
  its own files; the whole-tree commands are run and recorded. A failure in a file another card owns goes in the Log:
  wait and run again; never edit, format or `--fix` that file. Never run `cargo fmt` without `--check`, `cargo fix` or
  `clippy --fix`; format only your own files (`rustfmt <file>`). Every save of your files compiles (stub bodies with
  `todo!()` are fine). The orchestrator gets the whole tree green at each commit point."
  Card edits: in every p4 and p6 card replace "All exit 0" / "all exit 0" with "as `docs/plan.md` step 2 says"; in
  every p2 card change "a failure in another track's files" to "a failure in another card's files" and apply it to
  `cargo test -p wakeshot-core` and `cargo fmt --all --check` too.

### B2. `p3-haik-checks` signs out while other agents run

- Where: `docs/tasks/p3-haik-checks.md` lines 4 to 7 ("Parallel with: cards of any track that are not marked real
  laptop") and lines 32 to 33 (check B: one sign-out for checklists 5 and 6); check D (Start > Sleep); checks D, E, F
  (battery runs that measure when Windows sleeps).
- What fails: a sign-out ends every process of Haik's session: the orchestrator, every worker agent, their cargo
  builds and demos. Any card in progress loses its agent context and may leave a half-edited file: lost work. Start >
  Sleep suspends every agent mid-call. During D, E and F, other workers' `cargo test --workspace` takes a real SYSTEM
  power request (`p3-requests.md` line 41, not ignored), so the sleep timing the checks measure can be skewed (size of
  the effect not verified).
- Fix: `p3-haik-checks.md` header: "Runs alone on the whole laptop: no agent of any track and no cargo command runs;
  the orchestrator records state and stops spawning before step B and D; after sign-in Haik starts the orchestrator
  again. Parallel with: nothing." Same row in the `03-windows-power.md` card table ("Real laptop: yes, whole laptop:
  sign-out, sleep"). `p3-real-checks.md` step 5 (`cycles 50`): "no other card that takes power requests runs".

## Majors

### M1. `p3-restore` is gone, but two cards and two phase files still depend on it, and miss real dependencies

- Where: `docs/tasks/p4-cli-restore.md` lines 5, 9, 36 (`p3-restore`, `restore_from_journal`, `FakePower`);
  `docs/tasks/p6-hooks.md` lines 7 to 8; `docs/phases/04-sources.md` lines 95 and 149; `docs/phases/06-ship.md` lines
  112 and 146. `p3-restore` was deleted at 17:17 and contract C6 removes `FakePower`.
- What fails: the orchestrator cannot schedule a card after a card that does not exist, and the worker looks for an
  API that is not there. Both cards also need `Win32Power` (real path) and `DryRunPower` (`--dry-run`), which come from
  `p3-power-plan`, and `ownership::restore` with `FakePlans`, from `p2-ownership-restore`; neither is in their Runs
  after. `p3-journal`'s forms (line 42 area: "guard, restore mode and hooks") do not name the CLI.
- Fix:
  - `p4-cli-restore.md` line 5: "Runs after: `p3-journal` (`restore_locked`, `FileNet`), `p3-power-plan`
    (`Win32Power`, `DryRunPower`), core `p2-ownership-restore` (`ownership::restore`, `FakePlans`), `p3-store`,
    `p3-process`, `p3-log`." Line 9: "only against the core's `FakePlans` and `FileNet` in a temp folder". Line 36:
    "run `power::journal::restore_locked` (it takes `Local\Wakeshot.Journal` and calls `ownership::restore` with
    `expect_owner` = the dead owner)".
  - `p6-hooks.md` line 8: replace "`p3-restore` (`restore_from_journal()`)" with "`p3-journal` (`restore_locked`, the
    hook form of `FileNet`), `p3-power-plan` (`Win32Power`), core `p2-ownership-restore`". Acceptance: "the power
    track's fake or dry-run power layer" becomes "`FakePlans` or `DryRunPower`".
  - `04-sources.md` lines 95 and 149, `06-ship.md` lines 112 and 146: the same names.
  - `p3-journal.md`, Forms: "guard, restore mode, hooks and the CLI's `wakeshot restore` (RunOnce cleared, never set,
    `NoGuard`)".

### M2. No core card handles `Event::ActivePlan`

- Where: contract C8 (`p3-plan-windows-power.md` lines 232 to 234); `p3-system-events.md` line 32 posts it. No p2 card
  or `02-core.md` line names it; `p2-core-settings.md` lines 39 to 41 read the plan only on `Tick` and every 10 s.
- What fails: the 4 ms plan switch notice (`architecture.md` section 5, "Plan switch") reaches nobody. After a switch
  the new plan runs with the user's own lid action for up to 10 s; a lid close in that window most likely sleeps the
  PC during a hold (inferred from the design, not tested).
- Fix: `p2-core-settings.md` deliverable 2 add: "`Event::ActivePlan(id)`: when `id` differs from the last `Plan`'s
  active id and anything is owned or a hold counts, send `ReadPlan` at once; never act on it alone." Test
  `active_plan_event_reads_plan`; mutant "ignore `ActivePlan`". `02-core.md` Coverage, the Apply row: add it.

### M3. `p3-process` does not build `start_of_pid` and `list`, which the sources track relies on

- Where: contract C4 puts both on `process::Liveness` (`p3-plan-windows-power.md` lines 207 to 216, from the sources
  plan's item 8). `p3-process.md` deliverables (lines 28 to 41) and its tests name neither.
- What fails: `p4-watch-data` (hold pid-only rule), `p4-watch-agents` (holder image, Codex and Copilot fallback) and
  `p4-live-claude` (real liveness) assume real bodies. With no owner they stay stubs until someone notices at the live
  test or in integration.
- Fix: `p3-process.md` deliverables add: "`start_of_pid(pid) -> Lookup`: open with the exact rights and read the
  creation time; a denied open uses the process list; not found: `NotRunning`; list fails: `Unknown`. `list() ->
  Vec<ProcessEntry>` (pid, start, image file name) from the same system list." Tests `start_of_pid_self`,
  `start_of_pid_ended_child`, `list_has_self_with_image`.

### M4. `p4-watch-thread` needs `p3-process` but does not wait for it

- Where: `p4-watch-thread.md` lines 4, 19 to 20, 32; `04-sources.md` note 16 ("no watch card waits for the power
  track").
- What fails: `ProcessHandle` and `ExitWait` have no fake; their bodies are stubs until `p3-process`. The thread
  registers an exit wait for every live session and hold, so its tests hit the stub and panic.
- Fix: `p4-watch-thread.md` line 4: add `p3-process` to Runs after. `04-sources.md` note 16: "except
  `p4-watch-thread`, which needs the real `ProcessHandle` and `ExitWait` (`p3-process`)".

### M5. `p6-velopack-dependency` does not really run alone, and leaves every other track's ARM64 checks failing

- Where: `p6-velopack-dependency.md` lines 6 to 8 ("Other cards may keep running"); `docs/plan.md` lines 52 to 53 (the
  root `Cargo.toml` and `Cargo.lock` change only in a card the orchestrator runs alone); `06-ship.md` lines 90 to 91;
  `p6-plan-ship.md` Question 4 (lines 73 to 78); every p3, p4 and p5 card's ARM64 `clippy --workspace` and ARM64
  release build check. No p2 to p5 file mentions clang.
- What fails: (a) other workers' cargo runs during the `Cargo.toml` edit resolve the new dependency and can write
  `Cargo.lock` at the same moment (the cards' commands have no `--locked`), or fail on a half edit. (b) After it lands,
  ARM64 clippy and builds of the workspace need clang for `ring` (research, "Ship track planning checks"); only p6
  cards have an "ARM64: CI only" fallback, so every later p3, p4 and p5 card fails its acceptance on this laptop if
  Haik says no to Question 4. (c) Question 4's default installs a Build Tools component from the orchestrator's
  elevated session: an install on the real laptop, not marked, done while workers compile with the same MSVC
  toolchain.
- Fix: `p6-velopack-dependency.md` header: "Runs alone: no other card runs and no cargo command runs anywhere from its
  first edit to its last check." `docs/plan.md` step 2 rule (B1) add: "After `p6-velopack-dependency`, without clang,
  ARM64 checks use `--workspace --exclude wakeshot-tray`; the tray's ARM64 check is CI only." `06-ship.md` order step
  4: "Before it, if Haik says yes to Question 4: install the clang component as its own real-laptop step, with no
  cargo command running."

### M6. `p4-cli-commands` opens a window on Haik's screen and is not marked

- Where: `p4-cli-commands.md` lines 6 to 7 ("the `log` check opens a temp file in the default app once"), deliverable 4.
- What fails: a window opens in the middle of a screen slot (taskbar, menu or banner pictures in `p5-tray-icon`,
  `p5-menu-view`, `p5-notices`) or during Haik's checks. The card has no handle to close it and must not kill by name.
  If it is written as a test, every `cargo test --workspace` of every worker, and CI, opens it.
- Fix: header: "Uses the real laptop: screen, once (opens one file in the default app; global lane)." Deliverable 4:
  "The open runs once by hand in the slot (or as an `#[ignore]` `real_open_log` test); unit tests check only the path
  and the no-log message."

### M7. `p3-harness` deletes plan copies and may set the active plan outside the real-laptop lane

- Where: `p3-harness.md` line 4 ("Real laptop: no"), line 6 ("Parallel with: `p3-stop-points`"), lines 44 to 45
  (`cleanup` makes a stored plan active again and deletes every "Wakeshot test" copy), lines 52 to 53 (`cleanup` is a
  smoke run of this card). `p3-power-plan.md`: `TestPlan::sweep()` deletes every non-active "Wakeshot test" plan.
  `p6-install-e2e.md` also names its copy "Wakeshot test".
- What fails: a `cleanup` smoke run while `p3-stop-points` (or `p3-power-plan`) holds its copy in the lane deletes that
  copy mid-walk; the run fails and has to be repeated in the next slot. `cleanup` may call `PowerSetActiveScheme`,
  a real power call in a card marked "no".
- Fix: `p3-harness.md` line 4: "Real laptop: yes for the `cleanup` smoke run only (global lane); other smoke runs touch
  no setting." `p3-power-plan.md`: "a copy is named `Wakeshot test <pid>`; `sweep()` and `cleanup` delete only copies
  whose pid is not running, and run only in `real_` tests." `p6-install-e2e.md` step 2: name the copy `Wakeshot test
  e2e`.

### M8. Test registry keys collide between workers

- Where: `p6-login-and-path.md` lines 4 and 41 (fixed key `HKCU\Software\WakeshotTest\p6-login-and-path`; "deletes a
  leftover one at the start"); `p6-hooks.md` line 5 (the same key); `p3-store.md` lines 59 to 60 ("a key left by a
  killed run is deleted at the next run", rule not stated).
- What fails: these tests run inside every worker's `cargo test --workspace`, often at the same time, and in parallel
  test threads. One run deletes the fixed key while another run's test is using it, so tests fail at random across
  all tracks. The p3 sweep can do the same to a live pid's key.
- Fix: `p6-login-and-path.md` and `p6-hooks.md`: "scratch key `HKCU\Software\Wakeshot-test\<pid>\<test name>`; delete
  only your own; the leftover sweep deletes only keys whose pid is not running". `p3-store.md` line 60 the same rule.
  Acceptance lines "no `Wakeshot-test` key" (`p3-store`, `p3-journal`, `p3-stop-points`): "no key of this run's pids".

### M9. Tests that need a lid, Explorer or the console run in plain `cargo test`

- Where: `p3-system-events.md` lines 49 and 51 (a lid exists and is open); `p3-process.md` line 52 and
  `p3-recovery.md` line 65 (Explorer is the parent); `p3-session.md` line 40 (this session is on the console);
  `p3-requests.md` line 41 (`real_` name, not ignored, a real SYSTEM request).
- What fails: CI runs `cargo test --workspace` on GitHub runners, which have no lid (Explorer and the console session
  there: not verified), so CI goes red for every track after the push. On this laptop every worker's `cargo test
  --workspace` fails while the lid is closed (`p3-haik-checks` E and F, docked use), which then reads as a failure in
  another card's file for all tracks. The `real_` name breaks the track's own rule (`real_` means ignored).
- Fix: each such test is `#[ignore]` with the `real_` prefix and runs in its card and in `p3-real-checks` step 2; or it
  passes with a printed reason when the condition is absent (no lid callback in 2 s). Rename
  `real_request_sets_and_clears` to `request_sets_and_clears`, or ignore it.

### M10. Integration has no plan card, and its items are spread over five files

- Where: `docs/plan.md` lines 78 and 88 to 89; the items: `02-core.md` lines 131 and 153; `03-windows-power.md` note 11
  (lines 84 to 88); `04-sources.md` lines 152 to 154; `05-tray-ux.md` note 5 (lines 115 to 125); `06-ship.md` lines 98
  to 100. `p4-watch-thread.md` line 53 says the real `EventSink` comes from "the power track and the contracts card";
  `03-windows-power.md` note 11 says integration.
- What fails: nobody plans the tray loop, the real `EventSink`, the IO thread loop and its file commands, startup
  steps 4, 5 and 7 (single tray, panic hook, settings mutex and its retry), `Started` construction, the hibernate and
  update thread starts, the dry-run tray end to end, and checklist items 7 to 10, 11 (second session), 14 and 16.
  Haik's minutes for those checks are not counted anywhere, and the path to v1 cannot be computed.
- Fix: new card `docs/tasks/p6-plan-integration.md` (Phase 6 is "track, then integration"): "Runs after:
  `p2-core-lid`, `p3-power-io`, `p3-recovery`, `p4-watch-thread`, `p5-menu-view`, `p5-notices`, `p6-update-schedule`.
  Plans the wiring cards and Haik's checklist from the list in `docs/reviews/p2-6-cross-track.md`, item 6."
  `p4-watch-thread.md` line 53: "the real `EventSink` (integration)".

## Minors

- m1. Stale "who owns the routines" text. `02-core.md` lines 18 to 28 (note 1, "Open overlap") and 159 to 160;
  `p2-ownership-restore.md` lines 7 to 9, `p2-ownership-apply.md` lines 6 to 7, `p2-ownership-props.md` lines 5 to 7,
  `p2-core-props.md` lines 5 to 9 ("Runs only if ..." and the option B model). Fix: delete those sentences; note 1:
  "Decided 17:05: the core owns the routines." `02-core.md` line 127 says the CLI and hooks call `ownership::restore`;
  per `p3-journal` they call only `power::journal::restore_locked`: change it to that.
- m2. `p6-readme.md` line 7 misses `p4-live-codex` and `p4-live-copilot` (the README says which agents are tested live,
  `p4-plan-sources.md` line 238) and `p5-haik-check` (final menu words and icon). Fix: add all three to Runs after.
- m3. `p6-pages.md` line 6: "the card that makes the app icon" is vague, and Haik's pick in `p5-haik-check` rewrites
  `assets/wakeshot.ico` after `p5-icon-art`. Fix: "`p5-icon-art` and `p5-haik-check`".
- m4. `p4-claude-transcript.md` line 43 changes `ClaudeSource::new(home)` to `new(home, zone)`, which breaks
  `crates/wakeshot-core/tests/claude_status.rs`, owned by `p4-claude-status` (`04-sources.md` line 119). Fix: the
  transcript card also owns `tests/claude_status.rs` after `p4-claude-status` is done (`04-sources.md` line 121).
- m5. `p3-journal.md` line 56: `restore_locked_round_trip` stays `#[ignore]` until `p2-ownership-restore` is done, but no
  later card owns `power/journal.rs` to turn it on. Fix: move that test to `p3-recovery` (runs after
  `p2-ownership-restore`), or make `p3-journal` run after `p2-ownership-restore`.
- m6. `FakePlans` is used by `p3-journal`, `p3-recovery`, `p3-power-io`, `p4-cli-restore` and `p6-hooks` tests while
  `p2-ownership-apply` may still change `ownership/fake.rs`; and contract C6 adds `PlanIo::values()`, which
  `p2-ownership-restore`'s fake deliverable does not list. Fix: `p2-ownership-restore.md` deliverable 2 add "`values()`
  from the fake's active plan (C6)"; `p2-ownership-apply.md` Owns: "`fake.rs`: additions only, no public signature
  change".
- m7. `p2-present.md` line 28: the snapshot has no "requests only" flag and no Keep awake time left, but the tray
  plan's `text::Status { requests_only }` and `keep_awake_group(left)` ("25 min left") need them. Fix: add both to
  deliverable 1 (time left from the hold's `expires` and `Now`). Its tests build expected strings through `text::`, so
  `p5-text` and `p5-haik-check` wording changes do not break them.
- m8. The tray track's three requests on `shell.rs` (`p5-plan-tray-ux.md` line 211: accept a URI, call
  `AllowSetForegroundWindow(ASFW_ANY)`, COM apartment-threaded on its own short thread) are not in `p4-cli-commands.md`
  deliverable 4. Fix: add the last two (the URI is already there).
- m9. The contracts card runs before Haik's sign-off (`plan.md` lines 119 to 125), but ship item 4 (`Event::Updated`,
  `Notice::Updated`) waits for his answer to ship Question 3. Another answer means a second contracts round, which
  stops all builds. Fix: the contracts card adds both variants now; unused if he says no.
- m10. `p6-velopack-facts.md` line 6: "Runs after: none", but points 9 and 10 build the repo's release exes, and it
  installs software. Fix: "Runs after: Haik's sign-off of the track plans; the contracts card".
- m11. `p6-install-e2e.md` line 34 moves `%LocalAppData%\Wakeshot` away and the uninstall hook deletes it whole, with
  the `dry-run` folder that `p4-cli-run`, `p4-cli-commands`, `p4-cli-restore` (built-exe `--dry-run` checks) and the
  `p4-live-*` harness use at any time. Fix: "Parallel with: cards not marked real laptop, except those that use the
  `dry-run` data folder".
- m12. Two copies of one rule: `p3-recovery`'s retry loop ("the hooks of track 6 pass their own budget") and
  `p6-hooks`' own loop; `install.rs` writes the Run value while `store::AutoStart` already does (`p3-plan` Found
  outside scope). Fix: `RealHookEnv` calls recovery's single attempt and `AutoStart`; drop the hooks note in
  `p3-recovery`.
- m13. Sizes at the limit (item 5): `p2-core-holds` and `p2-core-settings` are 150K with no margin; `p5-tray-icon` is
  140K before its Part B resumes; `p4-live-claude` (120K) has open-ended "fix what the run shows". Fix: give each a
  split trigger in its header (for example the `Sim` harness out of `p2-core-holds`; the session and console part out
  of `p2-core-settings`; Part B of `p5-tray-icon` as its own card).
- m14. Build load: each card has its own target folder with two targets, debug and release; about 63 folders, no cap
  on parallel builds and no clean-up rule. Disk and RAM use not measured. Fix: `plan.md` step 2: "delete a card's
  target folder when it is done; the orchestrator caps parallel builds".
- m15. `real_` is a p3 rule, but `cargo test -p wakeshot-windows real_ -- --ignored` runs any ignored test from any
  track with `real_` in its path. Fix: make the prefix a repo rule in `plan.md`, or filter by module (`power::`).
- m16. `p5-haik-check` removes the unpicked art concepts from `art.rs`; that can break `ux_icon`, `ux_menu` and
  `ux_notices`, owned by done cards. Fix: keep the `art` API; the pick becomes the default concept.
- m17. The three icon GUIDs' `NotifyIconSettings` entries at uninstall (`05-tray-ux.md` line 146) are in no ship card.
  Fix: decide in `p6-hooks`: leave them (Windows' own record; README says so) or remove them.
- m18. `p1-contracts-split.md` deliverable 2 still asks for a `FakePower` in `wakeshot-windows`, against the 17:05
  decision and C6; `p6-velopack-dependency.md` line 7 names "the Phase 1 review fix card", which is
  `p1-contracts-split`. Fix: name `FakePlans` and the card.
- m19. `p4-claude-live-shapes` and `p4-live-claude` touch no power setting, screen or sound, but are marked "real
  laptop". In the global lane they would block screen and power cards for hours. Fix: mark them "agent-session lane:
  one at a time with the other p4 live cards; may run beside the global lane".

## The seven questions

### 1. Files owned by two cards that can run at the same time

No file has two owners at the same moment in the cards as written. Checked:

- `crates/wakeshot-core/src/text.rs`: `p5-text`, then `p5-haik-check` (after Haik). `p2-present` only calls it (m7).
- `crates/wakeshot-windows/src/shell.rs`: `p4-cli-commands` only; the tray track uses it (m8).
- CLI `main.rs` and `command.rs`: the contracts card, then frozen. Tray `main.rs`: the contracts card (dispatch,
  `velo::startup()`), later integration; `guard.rs`, `restore.rs` are `p3-recovery`'s, `velo.rs` is
  `p6-velopack-dependency`'s. `mode.rs` (exists) has no named owner: integration.
- `build.rs` (tray and CLI): the contracts card only (icon embed); no track card edits it.
- Module lists (`lib.rs`, `power/mod.rs`, `policy/mod.rs`, `scenarios/mod.rs`, `ownership/mod.rs`, `tray.rs`,
  `watch/mod.rs`, `sources.rs`, `holds.rs`, `testing.rs`): the contracts card only. Child modules declared inside an
  owned file (`sound/*`, `src/core/*`) and auto-found targets (`examples/*.rs`, `tests/*.rs`) need no list edit.
- Hand-overs in sequence: `core.rs`, `state.rs`, `sim.rs` (p2 chain); `ownership/fake.rs`; `sources/claude/mod.rs`;
  `art.rs` and `assets/wakeshot.ico`; `packaging/smoke-test.ps1`; the live cards' fixes in done files. Each waits for
  the earlier card.
- Real collisions are on shared resources, not files: the workspace build (B1), `Cargo.toml` and `Cargo.lock` (M5),
  test registry keys (M8), "Wakeshot test" plan copies (M7), `%LocalAppData%\Wakeshot\dry-run` (m11), and one
  signature change that breaks a file of a done card (m4).

### 2. Stale cross-track links

`p3-restore` in `p4-cli-restore`, `p6-hooks`, `04-sources.md`, `06-ship.md` (M1); `FakePower` and
`restore_from_journal` in `p4-cli-restore` (M1) and `p1-contracts-split` (m18); the option A/B text in four p2 cards
and `02-core.md` (m1); "the Phase 1 review fix card" and "the card that makes the app icon" (m18, m3). A script over
every `p<N>-...` name in the phase files and cards finds no other missing card (the rest are review files or globs).

### 3. Dependency cycles across tracks

None. Cross-track edges go one way: p2 to `p4-hold-files`; p3 to `p2-ownership-*`; p4 to p3 wave 1 and `p3-journal`;
p5 to `p4-cli-commands`; p6 to p3, p4 and p5. Nothing in p2 to p5 waits on p6. Two feedback loops are not in Runs
after: `p5-haik-check`'s locked-balloon result feeds `p2-core-lid` item 8a (decide D5 at sign-off), and the property
cards route bugs to done cards with no fix card planned.

### 4. Real laptop or Haik, not marked

`p4-cli-commands` (screen, M6); `p3-harness` `cleanup` smoke (power, M7); the clang install (install, M5);
`p3-haik-checks` marked, but its sign-out needs the whole laptop (B2); environment tests in plain `cargo test` (M9).

### 5. Cards over about 150K tokens

None over. Two at exactly 150K and two at risk (m13).

### 6. What the five plans leave out

- `Event::ActivePlan` in the core (M2); `start_of_pid` and `list` in `p3-process` (M3).
- Integration, with no plan card (M10): the tray loop and hidden window (`session::decode`), the real `EventSink`, the
  IO thread loop and its file commands (`WriteState`, `WriteHold`, `DeleteHold`, `WriteConfig`,
  `FinishRestoreRequest`, `Log`, `OpenLog`, `Notify`), startup steps 1 to 8 (single tray, panic hook, settings mutex
  and its tick retry, `Started` with `first_run`, journal, owner, console), the end-of-session restore and its 4 s wait,
  the watch thread start with `SystemZone` and real liveness, the hibernate thread, the update thread with
  `ApplyUpdate` then `Quit`, `TrayIcon`, `OwnerDrawnMenu` and `BalloonNotifier` creation and the balloon click hook,
  the dry-run tray end to end, removing `#[expect(dead_code)]` in `velo.rs`, real-exe checks (balloon header, exe icon,
  `NotifyIconSettings` entry, plain-English log lines), end-to-end CLI checks against a running tray, the update path
  end to end, and checklist 7, 8, 9, 10, 11 (second session), 14, 16 with Haik.
- Smaller: the tray track's `shell.rs` requests (m8); `text::Status` fields the core snapshot lacks (m7); the icon
  entries at uninstall (m17).

### 7. Totals

| Track | Cards | Tokens (card headers) | Haik's minutes | Laptop sessions |
|---|---|---|---|---|
| 2 Core | 10 | 1.21M | 0 (answers only: Question 10, D5, update items) | 0 |
| 3 Power | 15 | 1.42M | about 60 elapsed, about 15 of hands (`p3-haik-checks`) | 4 marked (`p3-power-plan` real run, `p3-stop-points`, `p3-real-checks`, `p3-haik-checks`) + 1 unmarked (M7) |
| 4 Sources | 19 | 1.73M | about 40 (`p4-live-codex`, `p4-live-copilot`), plus 3 optional | 4 marked (2 Claude-session, 2 with Haik) + 1 unmarked (M6) |
| 5 Tray UX | 9 | 0.93M | about 25 (`p5-haik-check`); icon sheet and text list looks not estimated | 5 (4 Part B slots, `p5-haik-check`) |
| 6 Ship | 10 | 0.88M | not estimated in the cards (README read, Pages look; my guess about 10, not verified) | 2 (`p6-velopack-facts`, `p6-install-e2e`) + 1 unmarked (clang install) |
| All | 63 | 6.17M | about 125 stated (about 80 of hands) + about 10 guessed; integration checks not counted | 15 marked + 3 unmarked = 18 |

The phase files say 1.43M (p3) and 1.7M (p4); the sums above are from the card headers.

Critical path (longest chain of Runs after, card size as the time measure, from the end of the contracts card):

- As written: the core chain, `p2-policy-holds` -> `p2-core-holds` -> `p2-core-settings` -> `p2-core-lid` ->
  `p2-core-props`, about 660K in 5 cards. Close behind, with M1's fix: `p3-store` -> `p3-journal` -> `p6-hooks` ->
  `p6-velopack-dependency` -> `p6-install-e2e` -> `p6-readme` -> `p6-pages`, about 650K in 7 cards.
- With m2's fix: `p4-watch-reads` -> `p4-watch-agents` -> `p4-watch-thread` -> `p4-live-claude` -> `p4-live-codex` ->
  `p4-live-copilot` -> `p6-readme` -> `p6-pages`, about 730K in 8 cards, with two Haik sessions and three laptop
  sessions on it.
- The 18 serial laptop sessions and Haik's time will most likely set the real pace (as `plan.md` says); integration
  comes after all of this and is not planned (M10).
