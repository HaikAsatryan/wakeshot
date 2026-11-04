# Wait 30 seconds on battery before the battery floor step back

Phase: 2 to 6 (core fix). Status: ready. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 100K tokens.
Written by the orchestrator, 2026-11-05, from `docs/tasks/p6-int-haik-checks.md` "Found outside scope" 6 and 7 and
Haik's answer of 2026-11-05 (yes to a 30 s grace; `docs/product.md` "Battery floor" already says it).
Runs after: `p6-int-haik-checks` (done, decfd5a). Parallel with: `p6-real-install-guard`, `p6-notice-app-icon` (they
own none of the files below).
Real laptop: no (the core's sim, scenarios and props; fakes only).
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p2-6-floor-grace\target`.

## Goal

Under the battery floor, a USB-C dock's short AC drop now gives a step back, the battery chime and a balloon each
time, and a restore of the user's settings. Haik's checks saw drops of 4.3, 4.4, 4.5, 4.7 and 9.65 s; the 9.65 s
drop's balloon was still on screen after AC came back (his 00:50 photo). Haik decided (2026-11-05): under the floor,
Wakeshot steps back only once the PC has been on battery for 30 seconds, the same `SOURCE_SETTLE_MS` the closed-lid
lock already waits. If AC comes back within the 30 seconds, nothing happens: no step back, no chime, no notice, no
restore, no apply. Holds keep holding during the 30 seconds.

Also fix the docs of finding 6: with the lid closed under the floor, the restore right after an AC loss made
Windows' own lid action sleep the laptop 1.4 s after the unplug, so the 5-minute route never ran. Orchestrator
decision (for Haik's review): docs only, no code keeps the lid key until the 5-minute check.

## Read first

- `docs/product.md` "Battery floor" and the work end rule; `docs/tasks/p6-int-haik-checks.md` "Found outside scope"
  6 and 7; `docs/research.md` "Integration checklist with Haik".
- `crates/wakeshot-core/src/policy/floor.rs` (`stepped_back`), `policy/lid.rs` (`SOURCE_SETTLE_MS`),
  `core/lid.rs` (`settle_floor`, `settle_window_lock`, the source lock near line 1011), `core.rs` (the settle pass and
  its deadline), `state.rs`, `sim.rs`, `scenarios/lid.rs`, `scenarios/props.rs`.

## Deliverables

1. The grace in the core. The step back starts when the PC is on battery under the floor and has been on battery for
   at least `SOURCE_SETTLE_MS`, timed from the AC loss the core already records for the source lock. The settle pass
   gives the grace end as a deadline, so the step back comes on time with no other event. Decide and record, with
   tests:
   - the lid closed case: the source lock and the step back both fall due 30 s after the unplug; their order;
   - a percent that falls under the floor after more than 30 s on battery: at once, as now;
   - the tray starting on battery under the floor (no AC loss seen): at once or after 30 s from the start;
   - AC flapping (lost, back, lost again within the 30 s): the grace starts again at the last loss.
2. Tests: `policy::floor` units; scenarios: a 5 s and a 29 s drop give no step back and no `Notify`, sound, restore
   or apply command; a 30 s drop steps back at 30 s; the same with the lid closed; the props model and its
   invariants updated. Any `wakeshot-windows` test that asserts a step back at once on an AC loss: update it in
   `app/dispatch.rs` or `app/tray_mode.rs` tests only.
3. Docs: `docs/product.md` "Battery floor": the rest of the bullet after the 30 s sentence, and the work end rule's
   "putting the lid action back does not sleep a laptop whose lid is already closed (tested)": add that right after a
   power source change Windows slept the laptop at once (Haik's block 4, 2026-11-05; which of the two made it act:
   not verified), and that with the 30 s grace it is not known whether Windows still acts at the restore or the
   5-minute route runs; both are safe. `docs/architecture.md`: the floor lines and "Episode flow" (edit only those
   lines; re-read right before each edit, another card may edit section 6). `text.rs` only if a text says "at once".

## Acceptance checks

- `cargo fmt` on own files; `cargo clippy -p wakeshot-core` (with and without `--all-targets`) and `cargo test -p
  wakeshot-core`; the same for `wakeshot-windows` if you changed its tests. Core props at the usual case counts.
- A scenario shows Haik's night: five drops of 4 to 10 s under the floor give zero chimes and zero balloons.

## Out of scope

- Taking a step back balloon away when AC comes back (finding 7 a; orchestrator decision: not now).
- Keeping the lid key until the 5-minute check (finding 6 code; orchestrator decision: docs only).
- Files of other running cards: `hooks.rs`, `notify.rs`, the tray crate, `packaging/`, `README.md`, `CLAUDE.md`.

## Questions for Haik

None.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-05: card written by the orchestrator.

## Result

What was done, files changed, checks and outcomes, open items.
