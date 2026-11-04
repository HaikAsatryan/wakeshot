# Phase 2: Core (track)

Plan written 2026-11-03 by card `p2-plan-core`, after the contracts domain review
(`docs/reviews/p1-contracts-domain.md`). Haik signs off all five track plans at once.

## Goal

The pure core, test-first, with no OS code: the state machine behind `Core::handle`, the policies, the settings
ownership rules (the apply, the restore routine and the ownership check), the journal file form, and the full test
suite (scenarios, property tests). It builds against the contracts in `crates/wakeshot-core` and runs in parallel with
Phases 3 to 6. No card touches the real laptop.

## Track notes

Only what `architecture.md` leaves open for this track. Each is a default; the card that builds it may change it with
a reason in its Result. Contract changes behind these notes: `p2-plan-core` Result.

1. **Settings ownership runs in the core.** `ownership::apply`, `restore` and `check` follow section 5 step by step,
   generic over two seams: `PlanIo` (plan calls) and `SafetyNet` (journal file, RunOnce value, guard), one call per
   step. Phase 3 implements both (real and dry run) and calls the routines only through its locked entry points
   (`power::journal::restore_locked`, `apply_locked`, `check_locked`) from the IO thread, the guard and restore mode;
   Phase 4 (CLI) and Phase 6 (hooks) call only `restore_locked` (review m1). Why: the journal property test then guards
   the real order of steps, and one fake (`ownership::fake`, with "in force" values, a call log and a script of failures
   and stops by call index) serves every stop-point test (review M2, M3). Change it when a step needs an interleaving
   the two traits cannot express. Decided 17:05 (orchestrator): the core owns the routines.
2. **Journal keys this version does not know** (`p1-contracts` correction 12): as the power track decided
   (`03-windows-power.md` note 5): kept untouched and skipped by restore, apply and check; with no known entry left,
   the clean-up clears RunOnce and stops the guard but rewrites the file with only those entries instead of deleting
   it; a rewrite always writes `JournalFile::VERSION` (review m2); `Started.journal` holds only known entries.
3. **An entry left on a plan for a key released on another plan** (`p1-design-verify-fixes`, Found outside scope;
   review m7): as the power track decided (`03-windows-power.md` note 6): the apply restores the entries on its plan
   whose key is not in `writes`, in the same run and the same `PowerSetActiveScheme`. The core sends in `writes` the
   three keys less the released ones and nothing more; `Command::Restore` keeps its shape.
4. **Going to sleep sound**: the hibernate thread plays it after its last check (section 5, review M5). The core sends
   only `Hibernate`; the decision 0 property counts `Hibernate` as its own explanation.
5. **`state.json` debounce** (1 s) is a core deadline. After `Done(Restored { why: UserAsked })` the core sends
   `FinishRestoreRequest(StateFile)`: the IO side writes `state.json` at once, then deletes `restore-request.json`
   (review m4; the power track's name for it).
6. **Log lines** for what only the core knows (hold start and end with labels, a release, a countdown, a step back, a
   format problem at most once a minute per source) go out as `Command::Log`. Never prompts or file content.
7. **First run**: `Started.first_run` (config file missing) gives `Notify(FirstRun)` and `WriteConfig`.
8. **Corrupt journal** (review M4): the power track posts `Event::JournalCorrupt` once per corrupt file (at start for
   a file left by restore mode, the guard or a hook; at once for the tray's own restore); the core shows
   `Notify(JournalCorrupt)`.
9. **Leftover journal at start** (review m5): the core retries it on each tick as `Restore { why: Leftover }`; the IO
   side runs that only when the journal's owner is Gone or this tray, so Unknown never starts a restore.
10. **Keep awake** goes out as `WriteHold` with the tray's pid and start (`Started.tray`); `expires` comes from the
    sources track's `holds::file::format_expires` (stub from the contracts card, filled by `p4-hold-files`). The core
    writes no date code of its own (review m3).
11. **Keep screen on idle lock** gets `Notify(IdleLocked)`: Question 8's answer (a notice or a sound for everything
    but the routine hold start and end) covers it, and a sound for an absent user helps nobody.
12. **Agent rules**: a session holds on `Working` or `open_tasks > 0` while its process lives; the sources track maps
    Claude's `shell` to `Working`. A timer holds from its due time minus 1 hour to its due time plus 2 minutes.
13. **Strings**: every user-facing string is in `text.rs` (tray track words it). `present.rs` (this track) builds the
    structure: which items, order, checks, icon state, the `StateFile` (review B1).
14. **No clock times in text**: `Now` is UTC and the core has no UTC offset. Change it when a contract carries it.
15. **Hold start before the first plan read**: the core sends `ReadPlan` and applies when the first `Plan` arrives.
16. **Keep screen on** is enabled only while Keep awake is on; Keep awake Off clears it. Icon while stepped back:
    Idle, reason in the tooltip. UI defaults; the tray track may change them.
17. **Expired hold files** stop counting; the core deletes a hold file only when its process is Gone (section 6).
18. **Notices while locked or off the console** (tray track proposal; Haik said yes, sign-off item 4, default, Haik
    2026-11-03 18:09): the core keeps notices raised while the session is locked or another session is on the console
    and sends them at `Unlocked` or `ConsoleBack` (`p2-core-lid`). Sounds are not held.
19. **Tests live inside the crate**: `#[cfg(test)]` modules, with `sim` and `ownership::fake` behind `test-support`,
    so tests see internal state and need no self dev-dependency. CI runs them on Linux and both Windows targets. Each
    property runs in a few seconds at the default case count; each card records one long run.
20. **Checks while other cards run** (cross-track review B1; the rule is `docs/plan.md` "How we work" step 2). Every
    card of this track checks only its own package and files:
    - format: `rustfmt --edition 2024 --check <owned files>`. A bare `rustfmt` uses the 2015 edition (the repo has no
      `rustfmt.toml`) and also checks the child modules a file declares (both checked 2026-11-03, `docs/research.md`
      "Track plan fixes checks"), so a diff in another card's child file is not this card's. Format only owned files
      (`rustfmt --edition 2024 <file>`), never a file whose child modules another card owns.
    - lint: `cargo clippy --locked -p <own package> --all-targets --target <t> -- -D warnings` and the same without
      `--all-targets` (the libs as they ship, without `test-support`; `CLAUDE.md` "Checks"), for both targets: nothing
      in the owned files.
    - test: `cargo test --locked -p <own package> <filter>`: the card's own tests pass. Several filters go after `--`
      (`cargo test --locked -p wakeshot-windows -- a:: b::`); two filters before `--` are a cargo error.
    - Every cargo command takes `--locked`: it fails at once if a build would change `Cargo.lock`, which only
      `p6-velopack-dependency` changes, alone.
    - A failure in a file another card owns goes in the Log: wait and run again; never edit, format or `--fix` that
      file. Never run `cargo fmt` without `--check`, `cargo fix` or `cargo clippy --fix`.
    - Every save of an owned file compiles (`todo!()` bodies are fine): one compile error fails every `-p` run of that
      crate and of the crates that use it.
    - No card runs `--workspace` or `--all` as a done check; the orchestrator runs the whole-workspace check when it
      closes a wave.
    - Mutants run only in a scratch copy of the workspace, never in the repo (verify review Major 2): copy
      `Cargo.toml`, `Cargo.lock`, `.cargo\`, `rust-toolchain.toml`, `assets\` (the exes' build scripts embed the icon),
      `docs\` (`holds/file.rs` reads `docs/hold-files.md` at compile time; `p2-policy-holds`, 2026-11-03)
      and `crates\` to `<scratchpad>\<card>\mutants\` after the card's own code is final, plant each mutant there, run
      the tests there with their own `CARGO_TARGET_DIR` (`<scratchpad>\<card>\mutants-target`) and `--locked`, then
      delete both folders. A mutant in the repo would fail other cards' tests, could reach a real-laptop run, and
      would stay behind if the session stops mid-mutant.
    - Once `p6-velopack-dependency` is done, ARM64 builds of `wakeshot-tray` need clang (only the tray depends on
      `velopack`). Haik said yes to the Build Tools clang component (sign-off item 8, Haik 2026-11-03 18:09);
      `p6-velopack-dependency` step 0 has Haik install it, elevated. Without it the tray's ARM64 builds are CI only
      (review M5).
    - Delete the card's own target folder when the card is done (review m14).

## Module layout (after the contracts card)

`crates/wakeshot-core/src/`. Every file below exists (empty, or with contract signatures) before the builds, so no
card of this track edits a module list or a file another track owns.

| File | Owner | Holds |
|---|---|---|
| `ownership/mod.rs`, `ownership/io.rs` | contracts card | module list; traits `PlanIo`, `SafetyNet`, `JournalRead` |
| `ownership/journal.rs`, `restore.rs`, `fake.rs` | `p2-ownership-restore` | file split and merge, restore routine, the fake |
| `ownership/apply.rs`, `check.rs` | `p2-ownership-apply` | the apply, the ownership check |
| `ownership/props.rs` | `p2-ownership-props` | the journal property test |
| `policy/mod.rs` | contracts card | module list |
| `policy/sessions.rs`, `hold_files.rs` | `p2-policy-holds` | agent session rules, hold file rules |
| `policy/requests.rs`, `lid.rs`, `lock.rs`, `floor.rs`, `ending.rs` | `p2-policy-power` | power rules |
| `present.rs` | `p2-present` | `TrayView`, `MenuModel`, `StateFile` from the core's state |
| `text.rs` | tray track | every string (signatures are contract) |
| `core.rs` (bodies only), `state.rs`, `sim.rs` | `p2-core-holds`, then `p2-core-settings`, then `p2-core-lid`; `sim.rs` then `p2-core-props` | state machine, `Sim` harness |
| `scenarios/mod.rs` | contracts card | module list |
| `scenarios/holds.rs`, `settings.rs`, `lid.rs`, `props.rs` | the core card of the same name | tests over `Core` |

`ownership/fake.rs`: its first, working version comes from the contracts card (`FakePlans`; its public API is
contract: add, never change); `p2-ownership-restore` extends it, and it passes to `p2-ownership-apply` when
`p2-ownership-restore` is done (`p1-contracts-split` edit 10). A core card may split
`core.rs` into private submodules under `src/core/`; it then owns them.

## Cards

| Card | Needs Haik | Runs after | Status |
|---|---|---|---|
| `p2-ownership-restore` | no | contracts card | done (19:13): 45 tests, 4 mutants caught |
| `p2-policy-holds` | no | contracts card | done (20:13): 79 tests, 21 of 22 mutants caught (the survivor was dead code, removed) |
| `p2-policy-power` | no | contracts card | done (20:05): 59 tests; settle time question to Haik |
| `p2-present` | no | contracts card | done (20:02): 24 tests |
| `p2-ownership-apply` | no | `p2-ownership-restore` | done (20:36): 46 tests, 10 of 11 mutants caught (the survivor cannot be caught) |
| `p2-core-holds` | no | `p2-policy-holds`, `p2-policy-power`, `p2-present`, `p4-hold-files` | done (20:39): 40 scenarios and a proptest, 13 of 13 mutants caught |
| `p2-ownership-props` | no | `p2-ownership-apply` | done (22:31): 10 properties at 20,000 cases, 13 of 13 mutants caught |
| `p2-core-settings` | no (Phase 1 Question 10: default, console part built; Haik 2026-11-03 18:09) | `p2-core-holds` | done (21:06): 34 tests, 12 of 12 mutants caught |
| `p2-core-lid` | no | `p2-core-settings` | done (21:46): 57 scenarios, 19 of 19 mutants caught; fixed a lid lock gap in the first seconds of a hold |
| `p2-core-props` | no | `p2-core-lid`, `p2-ownership-apply` | done (23:29): 13 properties at 10,000 cases; two findings kept as ignored tests |
| `p2-6-unread-journal` | no | core round B (Log in `p2-core-settings`); runs alone among cards that build | done (committed 02:51): findings 3 and 6, the newer `config.json` write check; 1256 workspace tests |
| `p2-6-unread-lid-countdown` | no | `p6-int-wiring-fixes` | done (05:28: 3 new scenarios, `startup_unread` drawn in the props, 22 props at 4096 cases 3 of 3) |
| `p2-6-unread-floor-countdown` | no | `p2-6-unread-lid-countdown` | done (05:58: 4 new scenarios; 1284 workspace tests; 8 of 10 mutants caught) |
| `p2-6-record-lost` | no (Haik's rule of 2026-11-04 in `product.md`) | nothing | done (11:13: 1312 workspace tests; 14 of 14 mutants killed; 12 worker decisions) |
| `p2-6-sleep-route` | no (Haik's rule of 2026-11-04; his lid test is in `p6-int-haik-checks`) | `p2-6-record-lost`, `p3-idle-sleep-check` | done (12:16: 1340 workspace tests; 13 of 13 mutants killed; 17 worker decisions) |
| `p2-6-sleep-route-fixes` | no | `p2-6-sleep-route` | done (13:49: 1355 workspace tests; 13 of 13 mutants killed; 10 worker decisions) |

No card touches the real laptop (no power settings, screen, sound or installs) or needs Haik's hands. "Contracts
card": the one card the orchestrator runs alone before the track builds.

## Order

Five waves. Cards in one wave own different files and run at the same time, and beside any card of another track.

1. `p2-ownership-restore`, `p2-policy-holds`, `p2-policy-power`, `p2-present`.
2. `p2-ownership-apply`, `p2-core-holds`.
3. `p2-ownership-props`, `p2-core-settings`.
4. `p2-core-lid`.
5. `p2-core-props`.

Where other tracks meet this one:

- Power track: its `PlanIo` and `SafetyNet` implementations need only the contracts card. Its code that calls
  `ownership::restore` runs after `p2-ownership-restore`; `apply` and `check` after `p2-ownership-apply`. Until then
  it tests its implementations call by call.
- Sources and Ship tracks: the CLI's `wakeshot restore` (owner dead) and the hooks call
  `power::journal::restore_locked` (`p3-journal`), which runs `ownership::restore`; so they run after
  `p2-ownership-restore` and `p3-journal` (review m1, M1). `p2-core-holds` (Keep awake) runs after `p4-hold-files`
  (`format_expires`).
- Tray track: words `text.rs` at any time (the signatures are contract); draws hand-built `TrayView`s until
  `p2-present` is done.
- Integration: wires `Core` into the tray loop after `p2-core-lid`.

## Coverage

Where each item this track owns is built. "Integration" is planned after the tracks.

| Item | Card |
|---|---|
| Section 3 events, state, commands; `Started`, config and Automatic, Keep awake, Keep screen on, Open log, first run | `p2-core-holds` |
| Holds from `Sources` and `Holds`; liveness Unknown keeps the last state; `DeleteHold` on Gone; "update Wakeshot" once per start per agent (decisions 5 to 8, 11, 12, 32, 37) | `p2-policy-holds`, `p2-core-holds` |
| Requests, `Retake` after resume, `CannotHold` (decisions 15, 16 as core rules) | `p2-policy-power`, `p2-core-holds` |
| Clocks: wall and mono deadlines, `next_deadline`, idle clamped at resume | `p2-core-holds`, `p2-policy-power`, `p2-core-lid` |
| Tray view, tooltip and menu name what holds, `state.json` and its debounce (decisions 33, 34, 35) | `p2-present`, `p2-core-holds` |
| Apply at hold start, also on AC; released keys on every plan; plan switch, including `Event::ActivePlan` (review M2); restore at hold end and its retry; leftover journal; Restore my power settings; Quit; session end; console left and back; settings owner; requests only (decisions 17, 18, 26 to 29, 38) | `p2-core-settings` |
| The apply, restore routine, ownership check, journal file form and unknown keys (decisions 24, 25, 26; section 5) | `p2-ownership-restore`, `p2-ownership-apply` |
| Journal property test (`p2-plan-core` acceptance check) | `p2-ownership-props` |
| Lid-close lock and chime, power source change lock, battery floor, countdown and hibernate, Keep screen on idle lock, closed-lid report (decisions 18, 20, 21, 23) | `p2-policy-power`, `p2-core-lid` |
| Notices raised while locked or off the console wait until the session is back (note 18; sign-off item 4: yes) | `p2-core-lid` (item 8a) |
| When to apply an update (ship track's contract proposal: `UpdateReady`, `ApplyUpdate`, `Outcome::Update`) | `p2-core-lid` (all taken by the contracts cards; `Updated` gives `Notify(Updated)`: ship Question 3 yes, Haik 2026-11-03 18:09), `p2-core-props` |
| Section 8, the core's part: each failure's fallback, notices once, `Command::Log` | the core card that handles the event |
| Section 9, the core's part: `Sim`, scenarios, the property tests, decision 0 | `p2-core-holds` (Sim), each core card, `p2-core-props` |
| Section 9 parser fixtures; hold file parsing; `expires` parse and format | sources track |
| Strings (decision 1), notice wording | tray track (`text.rs`) |
| The loop, the 30 s safety tick timer, the end-of-session restore at the queue front, the command runner (the IO queue is `p3-power-io`'s; integration only uses it) | integration |

## Work log

- 2026-11-03: plan written by `p2-plan-core`: 10 cards in five waves, 10 contract changes, no real-laptop card.
  Aligned with the power track's decisions (correction 12, the released-key entry, `JournalCorrupt`,
  `FinishRestoreRequest`), the sources track's `holds::file`, and the tray track's `text.rs`. The open question of who
  owns the apply, restore and check routines was decided at 17:05: the core.
- 2026-11-03 19:55: wave 2 restarted after the lighter plan (`02-06-lighter.md`): `p2-policy-holds`,
  `p2-policy-power`, `p2-present` running, with `p4-hold-files`. Next free slot: `p2-ownership-apply`.
- 2026-11-03 20:13: the four policy and view cards done and committed; `p2-ownership-apply` (20:03) and
  `p2-core-holds` (20:12) running. Open, for a docs pass: `architecture.md` section 7 "Waiting limit" lacks the
  run-time cap from `p2-policy-holds` (its Result, decision 3).
- 2026-11-03 20:39: `p2-ownership-apply` done and committed. Orchestrator decision (in `p2-core-settings`'s Log):
  after an apply fails with `JOURNAL_CORRUPT`, the core sends one `Restore { plan: None }`. Open, for the docs pass:
  `architecture.md` section 5 Apply steps 3 and 4 and the accepted cases under "Ownership" (`p2-ownership-apply`'s
  Found outside scope 1, 2, 6). Later, low priority: a `user_changed` field in `Outcome::Applied` (item 5).
- 2026-11-03 21:08: `p2-core-settings` done and committed; `p2-core-lid` running. Open, for the docs pass: plan
  reads and retries at a tick at most once per 10 s, and the restore at hold end only when something may be owned
  (`p2-core-settings` Found outside scope 2). For `p3-power-io`'s worker (next free slot): a `Leftover` restore not
  run because the owner is Unknown must answer `Err`, not `Ok` (item 3). Later, contract: a `RestoreWhy` for the
  restore after a failed apply (item 1; today `WorkEnded`).
- 2026-11-03 21:21: Haik answered the two `p2-policy-power` questions. (1) Settle time after a power source change
  with the lid closed: 30 s (Haik: 15 s or more, even a minute; the orchestrator picked 30 s). `product.md` and
  `architecture.md` updated; `p2-core-lid` changes `SOURCE_SETTLE_MS`. (2) A source change during a work-end countdown:
  if the user's own lid action for the new source is Do nothing, cancel and only release; otherwise hibernate on time.
- 2026-11-03 21:50: `p2-core-lid` done and committed (settle 30 s, countdown rule built). Open: the hibernate
  thread cannot be cancelled once started; the `policy::lid` module doc needs a line on the in-force view; docs pass
  lines are listed in its card. Design choices 8 (the floor countdown has no Do-nothing exception) and 13 (the
  closed-lid report waits for the lid to open) are near product calls; relayed to Haik.
- 2026-11-03 22:33: `p2-ownership-props` done and committed; `p2-core-props` running. The power safety review
  runs since 22:33 (report `docs/reviews/p2-6-power-safety.md`); its fixes go back to the cards' workers, or new
  workers that read the card Logs.
- 2026-11-03 23:30: `p2-core-props` done and committed; every core card is done. Two findings for the power review
  fix round, each kept as an ignored failing test: a plan switch inside the 10 s read window lets the switch's apply
  write a key the user just took back (`core/settings.rs`; proposed: after a release, ask for a new apply); a lid
  close before the first plan read, with a crashed tray's values in force, gives no lock (`core/lid.rs`).
- 2026-11-03 23:36: Haik answered the two 21:52 questions and approved the power safety review's lock fix, all as
  proposed (recorded in `product.md` and `architecture.md` sections 3 and 8 by session `wakeshot-9d`). (1) Floor
  countdown: no hibernate when the user's own battery lid action is Do nothing, on both paths (at the step back and at
  a lid close after it); review finding 7. (2) Closed-lid report: stays as built, it waits for the lid to open
  (`p2-core-lid` design choice 13). (3) Lock fails with the lid closed on our Do nothing: `Lock` once more at the next
  tick; if that fails too, step back as at the floor and start the countdown; review finding 1. Code for (1) and (3)
  goes to the power review fix round in `core/lid.rs`.
- 2026-11-03 23:36: power review fix round running (new worker, session `wakeshot-7c`): finding 1 (major, lid lock retry then step back), findings 2 and 4, and `p2-core-props`' two ignored tests. Owns `core.rs`, `core/`, `scenarios/lid.rs`, `settings.rs`, `props.rs`, `sim.rs`, one line of `architecture.md` section 8. Its Log goes in `p2-core-lid`. Routed: finding 3 to `p6-int-startup`; findings 5, 8, 9, 10 (core) to a second fix round after this one; finding 6 (hibernate thread) to a Windows worker later; finding 7 waits for Haik's answer on the battery floor.
- 2026-11-03 23:42: Haik confirmed in the orchestrator's chat (about 23:41): the 23:36 answers stand, and his rule for open calls is that with no work going on the laptop behaves as if Wakeshot were not there (now in `product.md`). Finding 7 (no floor hibernate on the user's own battery Do nothing, both paths) sent to the running fix round. The orchestrator keeps the 5 s no-`Session(Locked)` check in the finding 1 fix.
- 2026-11-04 00:17: power review fix round done and committed (Log in `p2-core-lid`): finding 1 (a failed or unconfirmed lid lock, 5 s, is retried once at the next tick, then a step back with requests off, restore and the countdown), findings 2 and 7, both `p2-core-props` findings (tests un-ignored), the LockFailed words in `text.rs`. 553 core tests, 21 of 21 mutants caught. Orchestrator decision, for Haik's review: finding 4 is not built. Work that ends while the user's own AC Do nothing keeps a closed laptop running gets no countdown after an unplug: Wakeshot holds nothing then, so Windows decides (his rule: as if Wakeshot were not there). The verify review of finding 1 runs (`docs/reviews/p2-6-power-safety-verify.md`). Next: a second core fix round for findings 5, 8, 9, 10, the core side of finding 3 (`Startup` gains `unread`, `started()` copies it) and `core.rs:159` (an Automatic toggle writes version 1 over a newer `config.json`); finding 6 (hibernate thread cancel) to a Windows worker. Seen by the fix round: during the lock step back the tray shows "Cannot hold awake" (for `p5-haik-check`).
- 2026-11-04 00:29: verify review of finding 1 done and committed (`docs/reviews/p2-6-power-safety-verify.md`): Fixed, 0 blockers, 0 majors, 5 minors; 561 core tests. The power safety review has passed: the `p3-real-checks` + `p3-haik-checks` bundle may start (laptop lane). Orchestrator routing: core round A (lid: power finding 5, verify minors 1 to 5; minor 1, a hibernate that fails after a lock step back leaves the closed laptop unlocked until the user's own sleep timeout, weigh a sleep fallback), then core round B (settings and ownership: power findings 8, 9, 10 and `core.rs:159`), serial since they share files. A small contract card, run alone: the core side of finding 3 (`Startup.unread`, `Started`), with finding 6 (hibernate thread cancel flag).
- 2026-11-04 01:08: core round A, lid, running (a new worker on `p2-core-lid`): power review finding 5 and the five
  minors of `docs/reviews/p2-6-power-safety-verify.md`. It owns `core/lid.rs`, `policy/lid.rs`, `scenarios/lid.rs`,
  `text.rs`, the lid lines of `architecture.md` section 3. Core round B (findings 8, 9, 10 and the Automatic toggle
  that writes version 1 over a newer `config.json`) runs after it.
- 2026-11-04 01:53: core round A, lid, done and committed (Log in `p2-core-lid`): power finding 5 fixed (the lid side
  compares journal copies between settle passes; a later read always wins); verify minors 1 to 5 fixed (relock, repeat
  unseen notices, wait for the first plan read, two scenarios, docs). 15 new scenarios; `wakeshot-core` 577 passed;
  27 of 27 mutants caught. Decided by the worker, for Haik's review: (1) a relock, not a sleep fallback: after a
  hibernate that is not allowed or fails while the lid stays closed, Wakeshot locks with the lid-close chime once input
  is idle 5 minutes, then at most every 5 minutes, until the lid opens (only when unlocked on the console and the
  user's own lid action is not Do nothing; a sleep fallback needs a product change, and on Modern Standby
  `SetSuspendState(FALSE)` also hibernates, tested here); (2) the relock also covers a plain work end where the user
  signed in on an external screen with the lid closed; (3) `HibernateFailed` is shown again too, not only
  `LockFailed` (a user at an external screen sees each twice); (4) the work-end and floor rules wait for the first plan
  read instead of guessing (if every read fails, no countdown starts); (5) a user who sets Do nothing by hand right
  after a restore may get one relock before the next read (up to 30 s). Open for Haik: a sleep fallback in the
  hibernate thread (optional, his call). Core round B running (findings 8, 9, 10; the `config.json` version
  overwrite; round A's props, `state.rs` and architecture items). A property in `scenarios/props.rs` failed in 3 of 7
  runs at 10,000 cases (a model flag, predates round A): round B fixes it; it may flake CI until then.
- 2026-11-04 02:29 (orchestrator, session `wakeshot-46`): core round B done and committed (Log and Result in
  `p2-core-settings`, "Core round B"). Fixed: power finding 8 (a code 5 or 1260 turns off overrides for the run only
  from the access check or the value writes), 9 (an apply saves new entries marked and clears the marks after its
  `PowerSetActiveScheme`), 10 (an apply skips a key the user changed since our write and keeps the entry); round A's
  props item, the flaky property's model flag, `LockWatch` moved into `State`, the architecture items. 587 core tests
  (1 ignored), 182 Windows power and app tests, 10 of 10 props and 3 of 3 ownership properties at 10,000 cases, 11 of
  11 mutants caught. Not fixed in the core: `core.rs:159` (an Automatic toggle over a newer `config.json`): the core
  never learns the file is newer, so the check goes in the IO thread's `WriteConfig` runner; added to
  `p2-6-unread-journal` as deliverable 7. Decided by the worker, for Haik's review: a 5 from `PowerSetActiveScheme`
  counts for the episode, not the run; each apply that adds an entry saves the journal once more; a user change waits up
  to about 10 s for the next check before the core lets the key go; a mark hides only a change back to exactly the
  original value. The sleep fallback after a failed hibernate stays open for Haik.
- 2026-11-04 02:30 (orchestrator, session `wakeshot-46`): `p2-6-unread-journal` started (findings 3 and 6 plus round
  B's `WriteConfig` check). Orchestrator decision, for Haik's review: it runs beside `p6-int-tray-mode` Part A and
  `p5-sounds` Part B, not alone: neither of them builds `Event::Started` or `Startup`, and the worker makes the
  contract change in one quick pass before its first cargo run. Pushed 8ff7fa5 at 02:30 (both known flaky tests fixed).
- 2026-11-04 02:51 (orchestrator, session `wakeshot-46`): `p2-6-unread-journal` done and committed. `Startup.unread`
  and `Event::Started.journal_unread`: a startup journal read that fails now retries as `Leftover` on each tick (power
  finding 3); the hibernate thread checks the lid once more after the going to sleep sound (finding 6; an agent turn
  that starts during the sound is not covered); the `WriteConfig` runner never writes over a newer `config.json`
  (round B's `core.rs:159` item). 6 new core tests (4 of them catch the mutant that ignores the flag), 2 config tests;
  workspace 1256 passed, 30 ignored. Decided by the worker, for Haik's review: the 500 ms startup retry stays, now
  keyed on `unread`; no notice at start for an unread journal; the random World start does not draw the unread case.
  Seen: two `app::window` timer tests failed under load in 2 of 5 runs (not its file; may flake CI; routed to a small
  test fix).
- 2026-11-04 04:38 (orchestrator `wakeshot-12`): new card `p2-6-unread-lid-countdown` from `p6-int-wiring-fixes`
  Found outside scope 3 and 5: with an unread startup journal, a lid closed before the `Leftover` restore lands starts
  no countdown, so a closed laptop runs on until the user's battery sleep timeout. Started beside
  `p6-int-dry-run-e2e` (core files only; that card fixes `wakeshot-windows` `app/*`, tray tests and CLI files).
- 2026-11-04 05:24 (orchestrator `wakeshot-12`): a cargo-mutants test binary of `p2-6-unread-lid-countdown` grew to 32 GB
  (free RAM 0 GB from about 05:20); Claude Code ended the keep-awake holders of `wakeshot-46` and `wakeshot-9d` for low
  memory. The worker ended the run by pid on request; free RAM back to 36 GB, the `wakeshot-12` holder still runs. Rule
  for later mutants runs: one job, a low timeout, memory watched; skip them if memory cannot be capped.
- 2026-11-04 05:28: `p2-6-unread-lid-countdown` done: while the journal is unknown, a Do nothing in force decides
  nothing and the work-end countdown waits; a want that waited starts once a read shows the user's own action as
  not Do nothing, and ends when the lid opens or a hold starts. 3 new scenarios (2 failed before the fix). The
  flaky lock property is fixed in the property (it failed 6 of 60 runs before, also with the change off): it now
  excuses the unknown journal case that `p6-int-wiring-fixes` decision 7 chose (a lock and chime on a user's own
  Do nothing while the journal is unknown; this also covers a restore that fails to load the journal mid-run, not
  only the start). Mutants: 5 of 7 caught; one mutant grew to 35 GB and was stopped; not rerun. For Haik's review:
  that decision 7; a user's own later change to Sleep starts a waited countdown. Left: the `Floor` countdown has
  the same gap, and two `state.rs` doc lines (`LockWatch`, `work_end_waits`) are stale: a follow-up card.
- 2026-11-04 05:29 (orchestrator `wakeshot-12`): new card `p2-6-unread-floor-countdown` (the `Floor` countdown's same gap,
  the two `state.rs` doc lines, whether an empty-copy retry should run every 10 s), started beside
  `p6-int-update-e2e` stop 1.
- 2026-11-04 05:31: CI on 46edb07 failed on x64 only: `scenarios::props::no_lock_when_the_lid_action_in_force_is_the_users_own`,
  the flaky property that `p2-6-unread-lid-countdown` found (6 of 60 local runs) and fixed in 2181c12 (not pushed then).
  Pushed 2181c12 to check it in CI.
- 2026-11-04 05:35: CI green on abd3061 (with the property fix 2181c12), all three jobs.
- 2026-11-04 05:58: `p2-6-unread-floor-countdown` done: the `Floor` countdown waits while the journal is unknown, the
  same shape as the work-end fix (`floor_countdown_waited`); with the user's own battery Do nothing none starts.
  `state.rs` doc lines only; one paragraph in `architecture.md` section 3. The empty-copy retry stays at the 30 s
  tick (it moves a hibernate by at most 20 s, only while the journal stays unreadable). Mutants with a 4 GB cap: 8 of
  10 caught, one stopped at the cap, M10 survives (the `may_own()` wait, older than this card). For Haik's review:
  a journal that stays unreadable keeps both countdowns waiting (the same trade-off as `p6-int-wiring-fixes`
  decision 7); before the first plan read a step back with the lid closed and nothing owned never counts down,
  after it it does (orchestrator default: leave it; with nothing owned the user's own lid action was in force at
  the close, so Windows already acted on it).
- 2026-11-04 06:03: CI green on 31a770a (the Floor countdown fix), all three jobs.
- 2026-11-04 10:13 (orchestrator `wakeshot-12`): Haik's answers (his words in quotes; recorded in `product.md`). A
  journal that cannot be read: "put it to windows defaults and notify the user and log the error"; so a 2-minute
  retry, then each value that looks like Wakeshot's goes to the plan's Windows default, one notice, no lock and no
  countdown in the window ("User settings safety" 7). This replaces `p6-int-wiring-fixes` decision 7. Work end with
  the lid closed: "just make it sleep but if technically it's impossible to control then think about your best effort
  but do not go to critical"; so Windows' own idle sleep (a 1-minute timeout, Wakeshot-owned), hibernate as the
  fallback, the built lock last; the battery floor and the failed-lock step back the same ("battery floor can use
  same"). The relock of `p2-core-lid` decision 1 stays as the last step. New cards `p2-6-record-lost` (now) and
  `p2-6-sleep-route` (after it and `p3-idle-sleep-check`); a narrow review after them.
- 2026-11-04 11:13 (orchestrator `wakeshot-12`): `p2-6-record-lost` done: after 2 minutes of run time with an unreadable
  journal (a damaged one at once) the new `ResetLost` command puts each key still at Wakeshot's value back to the
  plan's Windows default, sets the bad file aside and shows one `JournalLost` notice (none when nothing was reset);
  no lid lock and no countdown while the journal is unknown. Battery sleep's default on this laptop is 180 s, so a
  lost journal gives 3 minutes. 12 worker decisions for Haik's review in the card. Two items moved to
  `p2-6-sleep-route` as deliverables 8 and 9 (orchestrator decision, for Haik's review): a hold that runs through
  the window with the lid closed now runs unlocked until it ends, so lock when the window ends; `power_check`'s
  `Timed` wrapper. `p2-6-sleep-route` starts now.
- 2026-11-04 12:16 (orchestrator `wakeshot-12`): `p2-6-sleep-route` done: when the countdown is due, `SleepCheck` runs the
  checks and the sound, then an apply sets the sleep and screen timeouts to 60 s on AC and DC where the user's own
  is longer or Never (three new journaled keys); no sleep 5 minutes later means `Hibernate` as before. The
  timeouts go back at an unlock, input, lid open, a hold start or the countdown's end (`product.md` now says "an
  unlock"). Deliverables 8 (lock when the unknown window ends with the lid closed while holding) and 9 done. 17
  worker decisions for Haik's review. Files outside its list that had to change: CLI `status.rs`, `restore.rs`, tray
  `e2e_real.rs`, `hooks.rs`, `recovery.rs`. One test fails in the agent shell since about 12:00,
  `watch::agents::a_folder_that_cannot_be_listed_reads_its_known_files` (the deny ACL does not take; it fails on
  f47391d too, CI passed it there; cause not verified): to the small fixes card. Gap for a follow-up card
  (orchestrator default, for Haik's review): rule 7's reset should also put back the 60 s timeouts when they hold
  exactly Wakeshot's value, and the uninstall hook should reset a corrupt or unreadable journal's values.
- 2026-11-04 12:35 (orchestrator `wakeshot-12`): review of `p2-6-record-lost` and `p2-6-sleep-route` done
  (`docs/reviews/p2-6-sleep-route-review.md`): 0 blockers, 3 majors (a closed lid in the unknown window then a hold:
  no lock ever; a lost journal leaves the 60 s timeouts; uninstall with a damaged journal resets nothing), 5 minors.
  New card `p2-6-sleep-route-fixes` (starts now): the findings, plus four notices that are cut after five lines, and
  the watch test that fails in agent shells. Orchestrator decision, for Haik's review: rule 7's reset also covers
  the short timeouts at exactly Wakeshot's value. Review minor 8 fixed in `product.md`.
- 2026-11-04 13:49 (orchestrator `wakeshot-12`): `p2-6-sleep-route-fixes` done: review majors 1 to 3 fixed (the window
  lock waits for the lid or a lock and comes once a hold holds or our Do nothing is in force; the reset covers the
  four 1-minute keys, and a damaged reset that finds this tray's own apply undoes it, resets and applies again; the
  uninstall hook runs the reset and keeps the logs folder); minors 4, 6 and 7 as the review says, 5 leaner (a failed
  reset locks once); the listing test drops the backup privilege on its thread; four notices shortened to fit four
  lines. Not pushed: GitHub rejects the SSH key since about 13:50. Next: the reviewer checks the three majors' fixes.
