# Core state machine, part 3: lid, battery floor, countdown, locks

Phase: 2. Status: ready. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 130K tokens.
Real laptop: no (pure core code and tests). Build folder: `CARGO_TARGET_DIR=<scratchpad>\p2-core-lid\target`.
Runs after: `p2-core-settings`. Parallel with: any card of another track.

## Goal

The closed-lid and battery side of the episode flow: the lid-close lock and chime, the lock when a closed laptop
changes power source, the battery floor step back, the countdown to hibernate when work ends with the lid closed,
the Keep screen on idle lock, and the closed-lid report.

## Read first

- `CLAUDE.md`; `docs/plan.md` "How we work"; `docs/phases/02-core.md` (track notes 4, 11, 16).
- `docs/product.md` "Power behavior" (never lower security, sounds, closed-lid notice, battery floor, work ends with
  the lid closed).
- `docs/architecture.md` section 3 "Clocks", "Episode flow" (lid closes, power source changes, battery floor, Keep
  screen on, `Resumed`, holding ends, countdown, closed-lid report); section 5 "Lock and hibernate"; section 8 rows
  for lock and hibernate.
- `docs/reviews/p1-design-power.md` minors 17, 18; `p1-design-power-verify.md` minor 11; `p1-contracts-domain.md` M5.
- `crates/wakeshot-core/src/` contract files; Results of `p2-policy-power`, `p2-core-holds`, `p2-core-settings`.

## Owns

`crates/wakeshot-core/src/core.rs` (bodies), `state.rs`, `sim.rs`, `scenarios/lid.rs`, and `src/core/` submodules.
Nothing else.

## Deliverables

1. `Lid { closed }`: closing while the lid action in force is ours: `Lock` and `Play(LidClosedHolding)` together, the
   lock not waiting for the sound; note when the lid closed (`mono_ms`). The user's own Do nothing: neither.
2. `Power { ac, battery_pct }` with the lid closed: the source change rule through `policy::lid`, lock and chime once
   the new source has lasted 5 s (`Deadline::Mono`); a change back inside 5 s: nothing.
3. Battery floor through `policy::floor`: step back (`Requests` off, `Restore { why: Floor }`, `Notify(StepBack)`,
   `Play(BatteryFloor)`; countdown with reason `Floor` if the lid is closed); no hold starts while stepped back; AC
   back: hold again (requests and apply as at a hold start).
4. Countdown through `policy::ending`: starts when holding ends with the lid closed and the user's own lid action is
   not Do nothing (reason `WorkEnded`; settings already restored by the hold end), or at a step back with the lid
   closed; `InputIdle` clamped at resume; cancelled by a new hold, lid open, AC back (`Floor`); due:
   `Hibernate { reason, floor_pct, last_resume_mono_ms }` and no `Play` (track note 4).
5. `Done(Hibernate(..))`: `NotAllowed` or `Failed`: `Notify(HibernateFailed)`; `Skipped`: the countdown goes on or
   ends as the reason says (write the rule in the Result); `Resumed`: as `Resumed`.
6. Keep screen on idle lock through `policy::lock`: `Lock` and `Notify(IdleLocked)` once per idle period (track note
   11). `Done(Locked(Err))`: `Notify(LockFailed)`.
7. Closed-lid report: at the first `Session(Unlocked)` after a hold that ran with the lid closed:
   `Notify(ClosedLidReport { ran_ms, held_by })`, `ran_ms` from `mono_ms`.
8. `Suspending` and `Resumed`: the last resume time for the idle clamp.
8a. Build it: Haik said yes to the tray track's proposal (`docs/phases/02-06-signoff.md` item 4, default, Haik
    2026-11-03 18:09; `docs/phases/02-core.md` note 18). Notices raised while the session is locked or another
    session is on the console wait and go out at `Unlocked` or `ConsoleBack`, in order; sounds never wait. A tray
    that starts while locked gets `Session(Locked)` right after `Started` (integration,
    `docs/reviews/p1-close-verify.md` Major 1): test that `Started`, `Session(Locked)`, `Updated` gives no `Notify`
    until `Unlocked`, and the same for a start-time `RestoreFailed`.
9. `Log` lines for each lock, step back, countdown start, cancel and due.
9a. Updates: the contracts cards took all four ship track contract changes (`p1-contracts-split` Result, 17:56:
    changes 1 to 3; `p1-close-contracts`: change 4, `Event::Updated` and `Notice::Updated`, after Haik's yes to ship
    Question 3, sign-off item 7, Haik 2026-11-03 18:09), so build the whole item. `Event::UpdateReady` gives
    `Command::ApplyUpdate` only when no hold counts, no hold file names a live process (any kind, also with Automatic
    off) and no key is owned, at once or as soon as that becomes true; `Done(Outcome::Update(Ok))` gives `Quit`; `Err`
    waits for the next `UpdateReady`. `Event::Updated { version }` gives `Notify(Notice::Updated { version })`, once
    (it follows 8a like every notice). Tests: `updated_gives_notice`, and an `Updated` before any `Sources` still
    gives the notice.
10. `scenarios/lid.rs`, written before the code, one named test per item and these cases: lid closed on AC and on
    battery while holding; docked user with their own Do nothing (no lock, no sound); lid closed then unplugged after
    signing in on an external screen (lock after 5 s on battery; none for a 3 s AC drop) (power verify minor 11);
    work ends with the lid closed: hibernate after 5 minutes idle, not while typing; a resume during the countdown
    restarts the idle count; floor reached with the lid open and closed, then AC back; hibernate not allowed; Keep
    screen on with sleep timeout Never (no lock); closed-lid report once, with the right time across a suspend.

## Acceptance checks

Run from the repo root with the build folder above. Record each outcome.

- `cargo test --locked -p wakeshot-core scenarios::lid` and `cargo test --locked -p wakeshot-core`: all pass (including
  the earlier scenarios); list the new test names.
- `rustfmt --edition 2024 --check <owned files>`: no diff. `cargo clippy --locked -p wakeshot-core --all-targets
  --target x86_64-pc-windows-msvc -- -D warnings` and the same with `--target aarch64-pc-windows-msvc`: exit 0.
- No clock read, no I/O, no new dependency (as in `p2-core-holds`).
- Mutants, each made by hand, run, then reverted (record the failing test): lock at lid close when the user's own
  action is Do nothing; no lock at AC to battery with the lid closed; hibernate without the resume clamp; send
  `Play(GoingToSleep)` with `Hibernate`. Each must fail a test. Mutants run only in a scratch copy of the workspace,
  never in the repo (`docs/phases/02-core.md` note 20).
- Check rule: `docs/plan.md` "How we work" step 2 and "Checks while other cards run" in this track's phase file. A
  failure in a file another card owns goes in the Log: wait and run again; if it is still there when this card's own
  work is done, list it under "Found outside scope". Never edit, format or `--fix` that file. Every cargo command takes
  `--locked`, and clippy runs once more without `--all-targets` (the libs as they ship; `CLAUDE.md` "Checks"). No
  `--workspace` or `--all` run here; the orchestrator runs those when it closes a wave.
- Only the owned files changed.

## Out of scope

- The hibernate thread's last check and its sound, `LockWorkStation` (power track); chimes and their device (tray
  track). Contract files, `lib.rs`, `Cargo.toml`, `docs/architecture.md`.

## Questions for Haik

Each with the worker's default.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-03: card written by `p2-plan-core`.
- 2026-11-03 20:12, orchestrator: read `p2-policy-power`'s "Found outside scope" items 1 to 4 (countdown at a power
  source change, `lid::ours` after a failed restore, a second `Lock` after the lid-close lock, the first `Power`
  event has no source before). Item 1 went to Haik as a question; until he answers, keep the architecture's rule (not
  cancelled). The 5 s settle (`policy::lid::SOURCE_SETTLE_MS`) also waits for Haik (recommended 15 s).

## Result

What was done, files changed, checks and outcomes, open items.
