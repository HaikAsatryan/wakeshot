# Phase 3: Windows power layer (track)

Planned 2026-11-03 by card `p3-plan-windows-power`. Haik signs off all five track plans at once. `architecture.md` is
the design; this file holds only what it leaves open for this track, the cards and the order. The exact signatures
this track needs from the contracts card are in that card's Result, "Contract changes".

## Goal

Build every Windows piece behind the power side of Wakeshot, test-first, against the contracts in `crates/`: the
Windows implementations of the core's `PlanIo` and `SafetyNet` seams (the apply, the restore routine and the ownership
check are the core track's, orchestrator decision 2026-11-03 17:05), power requests, the IO queue and the power part
of the IO thread, the guard and restore mode, lock, hibernate, power and session events. Also the shared modules
`store`, `log` and `process`, first, because tracks 4, 5 and 6 use them. Then the stop-point test against the real
implementations and every real-laptop check of this track, Haik's included.

## Track notes

Only what `architecture.md` leaves open. Design changes are proposals in the Result of `p3-plan-windows-power`.

1. **Files and owners.** Each card owns whole files that exist after the contracts card; nobody else edits them while
   it runs.

   | File | Card |
   |---|---|
   | `wakeshot-windows/src/store.rs` | `p3-store` |
   | `wakeshot-windows/src/log.rs` | `p3-log` |
   | `wakeshot-windows/src/process.rs` | `p3-process` |
   | `wakeshot-windows/src/power/plan.rs` | `p3-power-plan` |
   | `wakeshot-windows/src/power/requests.rs` | `p3-requests` |
   | `wakeshot-windows/src/power/system.rs`, `wakeshot-windows/src/clock.rs` | `p3-system-events` |
   | `wakeshot-windows/src/session.rs` | `p3-session` |
   | `wakeshot-windows/src/power/journal.rs` | `p3-journal` |
   | `wakeshot-windows/src/power/hibernate.rs` | `p3-hibernate` |
   | `wakeshot-windows/src/power/executor.rs`, `wakeshot-windows/src/io_queue.rs` | `p3-power-io` |
   | `wakeshot-windows/src/power/stop_points.rs` (test only) | `p3-stop-points` |
   | `wakeshot-windows/src/recovery.rs`, `wakeshot-tray/src/guard.rs`, `wakeshot-tray/src/restore.rs`, `wakeshot-tray/tests/recovery.rs` | `p3-recovery` |
   | `wakeshot-windows/examples/power_check.rs` | `p3-harness` |

   Never this track's: module lists (`lib.rs`, `power/mod.rs`, the tray's `main.rs`), `testing.rs` re-exports,
   `Cargo.toml`, `Cargo.lock`, `tray.rs`, `notify.rs`, `sound.rs`, `watch/`, `shell.rs`, the event sink, anything in
   `wakeshot-core` (the routines and `FakePlans` are the core track's). A missing signature is a contract change for
   the orchestrator, never a side edit.
2. **Checks while other cards run** (tracks 4, 5 and 6 build in `wakeshot-windows` at the same time) (cross-track review
   B1; the rule is `docs/plan.md` "How we work" step 2). Every card of this track checks only its own package and files:
   - format: `rustfmt --edition 2024 --check <owned files>`. A bare `rustfmt` uses the 2015 edition (the repo has no
     `rustfmt.toml`) and also checks the child modules a file declares (both checked 2026-11-03, `docs/research.md`
     "Track plan fixes checks"), so a diff in another card's child file is not this card's. Format only owned files
     (`rustfmt --edition 2024 <file>`), never a file whose child modules another card owns.
   - lint: `cargo clippy --locked -p <own package> --all-targets --target <t> -- -D warnings` and the same without
     `--all-targets` (the libs as they ship, without `test-support`; `CLAUDE.md` "Checks"), for both targets: nothing in
     the owned files.
   - test: `cargo test --locked -p <own package> <filter>`: the card's own tests pass. Several filters go after `--`
     (`cargo test --locked -p wakeshot-windows -- a:: b::`); two filters before `--` are a cargo error.
   - Every cargo command takes `--locked`: it fails at once if a build would change `Cargo.lock`, which only
     `p6-velopack-dependency` changes, alone.
   - A failure in a file another card owns goes in the Log: wait and run again; never edit, format or `--fix` that file.
     Never run `cargo fmt` without `--check`, `cargo fix` or `cargo clippy --fix`.
   - Every save of an owned file compiles (`todo!()` bodies are fine): one compile error fails every `-p` run of that
     crate and of the crates that use it.
   - No card runs `--workspace` or `--all` as a done check; the orchestrator runs the whole-workspace check when it
     closes a wave.
   - Once `p6-velopack-dependency` is done, ARM64 builds of `wakeshot-tray` need clang (only the tray depends on
     `velopack`). Haik said yes to the Build Tools clang component (sign-off item 8, Haik 2026-11-03 18:09);
     `p6-velopack-dependency` step 0 has Haik install it, elevated. Without it the tray's ARM64 builds are CI only
     (review M5).
   - Delete the card's own target folder when the card is done (review m14).
3. **Real-laptop tests.** A test that touches a power plan, locks the screen or needs Haik is `#[ignore]` and its name
   starts with `real_`. Plain `cargo test` and CI never run it. So is a test that needs a lid, an open lid, Explorer
   as the parent or this session on the console (review M9): GitHub runners have no lid, and a closed lid here would
   fail every worker's run. A marked card runs them with its own module filter,
   `cargo test --locked -p wakeshot-windows -- --ignored --test-threads=1 <module>::`, never with `real_` alone: that
   also runs other tracks' ignored tests with `real_` in the name (review m15). It records Haik's values before and
   after (active plan; lid AC and DC; sleep AC and DC; energy saver DC), checks they are equal, and checks no
   "Wakeshot test" plan is left. Plan tests use a copy that is never made active (section 9); only `p3-real-checks` and
   `p3-haik-checks` make a copy active, as the checklist rule allows. In debug builds `WAKESHOT_DATA_DIR` replaces the
   data folder, so tests and the harness never touch `%LocalAppData%\Wakeshot` unless a check means to.
4. **Stop points.** The core track walks every stop point against its fake (`FakePlans`). This track runs the same
   walk against the real parts (`p3-stop-points`): `CopyAsActive` over a plan copy and `FileNet` in a temp folder,
   behind wrappers that share one call counter and stop at call k. After each stop it runs the next start's restore
   and checks:
   - after the stop: every value Wakeshot wrote that is still stored or in force has a journal entry with the user's
     original; while any known entry exists, RunOnce is set and a guard was started and not told to stop;
   - after the next restore succeeds: every key, stored and in force, equals the user's original (or the user's own
     later change); no known entry is left; whenever the next restore finds a journal, RunOnce is gone and the guard
     is told to stop. It finds none after a stop between the clean-up's journal delete and its last call (walk R6;
     `p2-ownership-restore` Found outside scope 2), or after a fresh apply stopped before its first journal save;
     what the stopped run added or had not cleared then stays, harmless (`p3-stop-points` Found outside scope 1; the
     walk asserts it this way, `assert_released`);
   - when the next restore fails (scripted), the first point still holds.
   `p3-power-io` and `p3-recovery` run the same checks for startup, the guard and restore mode against `FakePlans`.
5. **Correction 12, unknown journal keys** (decided in this plan's Result; the logic is the core's): an entry whose
   key this version does not know stays in the file untouched; restore, apply and the check skip it; the journal counts
   as empty when no known entry is left, and the clean-up then clears RunOnce and stops the guard but saves the file
   with only those entries, so the newer version restores them at its next start. This track's part: `FileNet` logs
   such entries once per load, and the stop-point test keeps one in every case.
6. **A released key's entry on the plan being applied** (decided in this plan's Result; the logic is the core's):
   `apply` restores the entries on its plan whose key is not in `writes`, in the same run and the same
   `set_active`. This track's part: one starting state in `p3-stop-points`.
7. **Corrupt journal** (review M4): `FileNet::set_aside_journal` moves it to `journal.json.corrupt`. The tray reports
   it as `Event::JournalCorrupt` (at start, for a file left by restore mode, the guard or a hook; at once, for its own
   restore) and renames the file to `journal.json.corrupt.seen`, so the notice shows once per corrupt file.
8. **Dry run.** `DryRunPower` reads the real plans and keeps its writes in memory, logging each one; it never calls a
   Windows write or `PowerSetActiveScheme`. Power requests, lock and hibernate each have a dry-run form that only logs
   (review m6). A dry-run tray starts no guard and sets no RunOnce.
9. **Values set by this plan** (Haik signed them off with the plan, 2026-11-03 18:09): journal mutex wait 10 s, a
   timeout fails the run and it is retried; log files `wakeshot.log`, `wakeshot.1.log`, `wakeshot.2.log` (the `.log`
   extension keeps "Open log" working); `GUID_ACDC_POWER_SOURCE` value 2 (short-term power, a UPS) counts as
   battery; if Explorer is missing or its open is denied, the guard starts as the tray's own child and the log says
   so (End task on the group then ends both: "Gap left", third case).
10. **Ownership check read errors.** The active plan read fails: `Plan(Err)`. One entry's read fails: keep the entry,
    log it, and still post `Plan(Ok)` with the active plan's values.
11. **Integration, not this track:** the loop and its hidden window (the window's owner calls `session::decode` for
    session messages); the real event sink; the IO thread loop that pops `IoQueue` and runs the executor and the file
    commands (`FinishRestoreRequest`: write `state.json`, then delete `restore-request.json`); `Command::Log`; the
    settings mutex retry on each tick; tray startup order; manual checklist items 7, 8, 9, 10, 16 and the
    second-session part of 11.

## Task cards

Haik signed off the five track plans 2026-11-03 18:09, every question with its default. All cards wait for the
contracts cards (the orchestrator runs them alone; changes listed in the Result of `p3-plan-windows-power`). "Real
laptop" cards run one at a time across all tracks.

| Card | Needs Haik | Real laptop | Runs after | Status |
|---|---|---|---|---|
| `p3-store` | no | no (temp folders, a test registry key) | contracts card | done (19:15): 15 tests, five runs; `HeldMutex` not `Send` |
| `p3-log` | no | no | contracts card | done (19:10): no line mix over 7 runs; roll fails only while another app holds a log file (open item) |
| `p3-process` | no | no (child processes it starts and ends by its own handles) | contracts card | done (19:13): 22 tests; Explorer as parent works from High and Medium |
| `p3-power-plan` | no | yes: a plan copy, never active; reads Haik's plan | contracts card | done (20:28): 13 tests; Haik's values same before and after; a write to an inactive copy changes only that copy |
| `p3-requests` | no | no (a power request held under 1 s) | contracts card | done (20:47): 13 tests, real set and clear under 1 s |
| `p3-system-events` | no | no (notifications only) | contracts card | done (20:46): 26 tests, both real tests pass |
| `p3-session` | no | no (the lock test is ignored here) | contracts card | done (20:47): 4 tests, 3 ignored for `p3-real-checks` and `p3-haik-checks` |
| `p3-journal` | no | no | `p3-store`, `p3-log` | done (20:26): 16 tests |
| `p3-hibernate` | no | no (never calls `SetSuspendState`) | `p3-system-events` | done (21:19): 21 tests; hibernate checks now run before the sound |
| `p3-power-io` | no | no | `p3-journal`, `p3-power-plan`, core `p2-ownership-apply` | done (21:04): 24 tests, 22 of 22 mutants caught |
| `p3-recovery` | no (Phase 1 Question 12: the default, Haik 2026-11-03 18:09) | no | `p3-journal`, `p3-process`, `p3-log`, `p3-power-plan` (verify review Minor 3), core `p2-ownership-restore` | done (21:22): 23 unit and 5 exe tests; real guard parent test passed |
| `p3-stop-points` | no | yes: a plan copy, never active | `p3-power-plan`, `p3-journal`, core `p2-ownership-apply` | done (21:48): 3142 real cases passed, Haik's values same; no routine bug |
| `p3-harness` | no | yes for the `cleanup` smoke run only (global lane, review M7); other smoke runs touch no setting | `p3-power-io`, `p3-recovery`, `p3-requests`, `p3-session`, `p3-system-events` | done (22:56): 22 tests, smoke runs done; Haik's values same |
| `p3-real-checks` | no | yes: an active plan copy, 50 cycles | `p3-harness`, `p3-stop-points` | done (00:54): every step passed; 16 real tests in `wakeshot-windows`, 2 in the tray; 50 cycles, `PowerSetActiveScheme` median 51.5 ms, max 121.8 ms |
| `p3-haik-checks` | yes: about 60 min elapsed, about 15 min of hands | yes, whole laptop: battery, lid, sign-out, sleep, lock; no other agent or cargo command runs (review B2) | `p3-real-checks` | blocked: waiting for Haik (01:08); step files ready in its card |

Size estimates (one worker's context): store 90K, log 80K, process 110K, power-plan 110K, requests 60K,
system-events 100K, session 80K, journal 90K, hibernate 90K, power-io 110K, recovery 140K, stop-points 110K, harness
120K, real-checks 70K, haik-checks 60K. Total about 1.43M tokens. Wave 1 peaked at 2 to 3 times these estimates
(`docs/phases/02-06-lighter.md`).

Bundles (one worker, both cards in order; `docs/phases/02-06-lighter.md`): `p3-requests` + `p3-session`;
`p3-real-checks` + `p3-haik-checks`. The `p3-real-checks` bundle also waits for the power safety review
(`docs/phases/02-06-lighter.md`, "Reviews"), so its fixes land before the real checks and Haik's checks.

## Order inside the track

1. Wave 1, in parallel: `p3-store`, `p3-log`, `p3-process` (tracks 4, 5 and 6 wait on these three), `p3-requests`,
   `p3-system-events`, `p3-session`, `p3-power-plan` (its code in parallel; its `real_` tests in a real-laptop slot).
2. Wave 2: `p3-journal` (after store and log), `p3-hibernate` (after system-events).
3. Wave 3, once the core's routines exist: `p3-recovery` (after `p2-ownership-restore`), `p3-power-io` and
   `p3-stop-points` (after `p2-ownership-apply`).
4. Wave 4: `p3-harness`.
5. Wave 5: `p3-real-checks`, then `p3-haik-checks`.

Critical path: store, journal, (core ownership cards), power-io, harness, real-checks, haik-checks. The core's
`p2-ownership-restore` and `p2-ownership-apply` are on it; they are wave 1 and 2 of the core track.

## Manual checklist (architecture section 9) in this track

| # | Where |
|---|---|
| 1, 3 | `p3-haik-checks` (battery runs, about 20 min each) |
| 2 | `p3-haik-checks`, optional |
| 4, 5, 6 | `p3-haik-checks`, with the real tray exe's guard and restore modes and the harness |
| 11 | `Global\` mutex at Medium: `p3-real-checks`; lock and unlock notices: `p3-haik-checks`; second session: integration (no second account) |
| 15 | `p3-real-checks` (agent, an active plan copy) |
| 7, 8, 9, 10, 16 | integration (they need the core, notices or sounds) |
| 12 | later (no test account) |

## Work log

- 2026-11-03 16:55: phase file and cards written by `p3-plan-windows-power`.
- 2026-11-03 17:00: reworked for the contracts domain review (event sink, safety-net seam, tray mode files, IO queue).
- 2026-11-03 17:20: reworked for the orchestrator's decision (17:05): the routines are the core's; `p3-restore`,
  `p3-apply` and `p3-power-props` dropped; `p3-journal` is the `SafetyNet` implementation; `p3-stop-points` added.
  Waiting for Haik's sign-off of all five track plans and the contracts card.
- 2026-11-03: Haik signed off with every default (18:09); `p1-close-contracts` wrote the answers into this file and
  the cards, and added `p3-power-plan` to `p3-recovery`'s Runs after (verify review Minor 3).
- 2026-11-03 20:06: `p3-journal` (20:04) and `p3-power-plan` (20:06, the laptop lane) running, beside core cards.
- 2026-11-03 20:30: `p3-journal` and `p3-power-plan` done and committed. `p3-system-events` (20:26) and the
  `p3-requests` + `p3-session` bundle (20:29) running. Laptop lane free. Note for track note 2:
  `cargo clippy -p wakeshot-windows -- -D warnings` also fails on warnings inside `wakeshot-core` while a core card
  is mid-edit (`p3-power-plan`, 20:26): wait and retry, as for any other card's file.
- 2026-11-03 20:50: `p3-system-events` and the `p3-requests` + `p3-session` bundle done and committed. Running:
  `p3-power-io` (20:37), `p3-recovery` (20:49). Open, cleanup for later: the Win32 error mapping has four private
  copies; one helper in `error.rs` would do (`p3-session`).
- 2026-11-03 21:21: `p3-hibernate` done and committed. Open, for the docs pass: `architecture.md` section 5 still lists
  the old hibernate order (privilege and `IsPwrHibernateAllowed` now run before the sound). For `p5-sounds`: keep the
  "going to sleep" sound short, since a lid opened while it plays still hibernates. `testing.rs` does not re-export
  `FakeHibernate`.
- 2026-11-03 21:25: `p3-recovery` done and committed; `p3-stop-points` running in the laptop lane. Open, for the docs
  pass: the architecture's Guard line could say the guard leaves a corrupt journal for the next sign-in's restore.
- 2026-11-03 21:51: `p3-stop-points` done and committed; laptop lane to `p6-velopack-facts` (installs only). Open, for
  the docs pass: track note 4 is wrong for a fresh apply stopped before its first journal save (RunOnce and the guard
  stay; harmless); proposed text in its card, Found outside scope 1.
- 2026-11-03 22:57: `p3-harness` done and committed; laptop lane free. The power safety review is done (0, 1, 9;
  `docs/reviews/p2-6-power-safety.md`). Open, docs: `TestPlan::sweep`'s doc in `power/plan.rs` misses that `cleanup` calls it.
- 2026-11-04 00:04: the docs pass is done and committed (`docs/tasks/p2-6-docs-pass.md`): track note 4 above and the `TestPlan::sweep` doc (log lines 199 to 203) are fixed; section 5 hibernate order, Guard line, 10 s reads in `architecture.md`.
- 2026-11-04 00:30: the `p3-real-checks` + `p3-haik-checks` bundle started (laptop lane; the power safety review and its verify passed). Part 2 waits for Haik and runs alone on the whole laptop: the orchestrator records its state and stops spawning before its steps B and D.
- 2026-11-04 01:11: `p3-real-checks` done and committed: a Medium process started through Explorer made and took the
  `Global\` mutex; the journal lock opens both ways between High and Medium (checked with pwsh on probe names, not the
  Rust exes); after a crash of a Medium fake tray the guard restored the copy and removed the journal and RunOnce
  within 300 ms; a plan's stored values took effect when it was made active; 50 cycles with brightness at 100 and no
  System or Application log errors. Haik's values were equal before (00:32:57) and after (00:52:26): Balanced active and
  the only plan, lid 1/1, sleep AC and DC 18000, energy saver DC 30. Sleep was 10800/14400 at 22:55; Settings changed it
  to 18000/18000 at 00:31:08, before the card started, by no agent as far as the worker could tell (Haik asked in
  session `wakeshot-91`'s chat). The laptop has been on battery and undocked since about 00:30. `p3-haik-checks` is
  blocked, waiting for Haik: about 60 min, 15 of hands (A End task, C lock and sign in, D Start > Sleep on battery,
  E lid closed 20 min on battery, B1 and B2 two sign-outs; 50% battery or more). Before he starts: stop every agent and
  the keep-awake helper, then a new worker does a 5 minute prep that reads both cards' Logs. Decided by the worker,
  for Haik's review: the 00:32 values are the baseline; skip F; B is two sign-outs; Haik double-clicks step files; the
  cycles ran on battery while other cards built. Stale: the Test laptop lines in `CLAUDE.md` and `architecture.md`
  652, 656 and 1017 (sleep values, dock), once Haik confirms the new values are his.
- 2026-11-04 01:22: Haik's answer, relayed by session `wakeshot-91` (his words at 01:14, "1 yes"): the 00:31 sleep
  change and the undocking were his. His baseline from now: sleep AC 18000 and DC 18000, lid AC 1 DC 1, energy saver
  DC 30, Balanced the only plan. Correction (Haik 01:16, relayed): the undocking was for tonight only; the dock
  stays always plugged in, so only the sleep values change in the docs. `p3-haik-checks` uses these values (the
  real-checks worker already took the 00:32 values, which are the same). The Test laptop lines in `CLAUDE.md` and
  `architecture.md` (652, 656, 1017) go to the next docs pass; the `CLAUDE.md` edit waits for Haik's yes in the
  orchestrator's own chat, since the answer came through a peer session.
- 2026-11-04 01:41: docs pass round 2 done. `CLAUDE.md` Test laptop now says sleep 5 h on AC and on battery (read
  01:39: AC and DC 18000; the orchestrator made that edit after Haik's direct message, with the values read on the
  laptop). The earlier "architecture.md 652, 656 and 1017" note was wrong: those lines hold no sleep values; they are
  "not tested" lines that `p3-real-checks` has now tested (next docs pass). Also stale: the old values in `sim.rs:79`.
