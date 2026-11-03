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
| `p2-core-holds` | no | `p2-policy-holds`, `p2-policy-power`, `p2-present`, `p4-hold-files` | running (20:12) |
| `p2-ownership-props` | no | `p2-ownership-apply` | ready |
| `p2-core-settings` | no (Phase 1 Question 10: default, console part built; Haik 2026-11-03 18:09) | `p2-core-holds` | ready |
| `p2-core-lid` | no | `p2-core-settings` | ready |
| `p2-core-props` | no | `p2-core-lid`, `p2-ownership-apply` | ready |

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
