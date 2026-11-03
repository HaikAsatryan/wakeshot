# Core state machine, part 1: start, holds, requests, view, Sim harness

Phase: 2. Status: ready. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 150K tokens. Split trigger (review
m13: no margin): if the worker passes about 120K before the `Sim` harness is done, it stops at a clean point and the
`Sim` harness (`sim.rs`) becomes its own card, run before `p2-core-settings`.
Real laptop: no (pure core code and tests). Build folder: `CARGO_TARGET_DIR=<scratchpad>\p2-core-holds\target`.
Runs after: `p2-policy-holds`, `p2-policy-power`, `p2-present`, and the sources track's `p4-hold-files`
(`holds::file::format_expires`). Parallel with: `p2-ownership-apply`, and any card of another track.

## Goal

The first part of `Core::handle`: the state, startup, holds from agents and hold files, Automatic, Keep awake, power
requests, the tray view and `state.json`, deadlines, and the `Sim` harness the later core cards test with. Settings
(apply, restore) and the lid rules come in the next two cards; leave clear seams for them.

## Read first

- `CLAUDE.md`; `docs/plan.md` "How we work"; `docs/phases/02-core.md` (all track notes).
- `docs/product.md` "What keeps the PC awake", UX.
- `docs/architecture.md` section 3 (all; this card builds the parts named below), section 4 "Tray threads" (what
  runs where), section 6 (`config.json`, `state.json`, holds), section 8 (rows for requests, liveness, format), section
  9 core.
- `crates/wakeshot-core/src/` (all contract files); Results of `p2-policy-holds`, `p2-policy-power`, `p2-present`.

## Owns

`crates/wakeshot-core/src/core.rs` (bodies and private fields; the three public signatures are contract),
`state.rs`, `sim.rs`, `scenarios/holds.rs`; private submodules under `src/core/` if the card makes them. Nothing else.

## Deliverables

1. `state.rs`: the state of architecture section 3 ("State is one struct"), every field declared now, including those
   the next cards fill (journal mirror, released keys, lid, floor, countdown), so they do not reshape it.
2. `core.rs`, events handled here:
   - `Started`: config (its `config` wins over `Core::new`'s), journal mirror, owner, console, tray; `first_run` gives
     `Notify(FirstRun)` and `WriteConfig`.
   - `ConfigChanged`; `Menu(Automatic)` (gives `WriteConfig`); `Menu(KeepAwake)` and `Menu(KeepScreenOn)` (give
     `WriteHold` for `keep-awake.json` with the tray's pid and start from `Started.tray`, `expires` from
     `holds::file::format_expires`, `screen`; or `DeleteHold(keep-awake.json)`); `Menu(OpenLog)`.
   - `Sources` and `Holds` through `policy::sessions` and `policy::hold_files`: per-session memory (last liveness, first
     seen waiting); `DeleteHold` for a Gone hold file; Unknown keeps the last answer; "update Wakeshot" once per start
     per agent, a `Log` line at most once a minute per source.
   - Holding starts and ends: `Requests` through `policy::requests`, a `Log` line with the labels. A seam (one
     function each) where `p2-core-settings` adds the apply and the restore.
   - `Resumed`: `Retake` if any request is on. `Done(Requests(Err))` (also the answer to `Retake`): `Notify(CannotHold)`
     once per episode.
   - `Tick` and `next_deadline`: the nearest hold deadline (timer windows, waiting hour, `expires`, Keep awake end) and
     the 1 s `state.json` debounce.
   - On every change the user can see: `Tray(view)` and, debounced, `WriteState`, through `present`.
3. `sim.rs` (behind `test-support`): `Sim` with a `FakeClock`; send an event at the current time, get the commands;
   `run(ms)` delivers `Tick` at each `next_deadline` and every 30 s (the safety tick); `suspend(ms)` and `set_wall`;
   builders for common events (a Claude session, a hold file, `Started`); a record of every step for failure messages.
   Later cards extend it.
4. `scenarios/holds.rs`, written before the code, one named test per case: start with and without a config file; a
   busy session holds and ends; a timer window opens and closes on time across a suspend and a wall clock jump; a
   waiting session with Remote Control ends after 1 hour; Unknown liveness keeps a hold; a Gone hold file is deleted;
   Automatic off keeps only Keep awake; Keep awake 30 minutes ends on time; Keep screen on asks for the display;
   "update Wakeshot" once per agent per start; `Retake` after resume; a failed request shows `CannotHold` once; the
   tooltip names what holds; `state.json` writes at most once a second.

## Acceptance checks

Run from the repo root with the build folder above. Record each outcome.

- `cargo test --locked -p wakeshot-core scenarios::holds` and `cargo test --locked -p wakeshot-core`: all pass; list the
  test names.
- `rustfmt --edition 2024 --check <owned files>`: no diff. `cargo clippy --locked -p wakeshot-core --all-targets
  --target x86_64-pc-windows-msvc -- -D warnings` and the same with `--target aarch64-pc-windows-msvc`: exit 0.
- `Core::handle` never panics on any event in any order: a short `proptest` in `scenarios/holds.rs` over random
  events of this card's kinds (the full properties are `p2-core-props`).
- No clock read, no I/O, no new dependency: `cargo tree -p wakeshot-core -e normal` unchanged; a grep of the owned
  files finds no `std::time`, `std::fs`, `std::env`.
- Check rule: `docs/plan.md` "How we work" step 2 and "Checks while other cards run" in this track's phase file. A
  failure in a file another card owns goes in the Log: wait and run again; if it is still there when this card's own
  work is done, list it under "Found outside scope". Never edit, format or `--fix` that file. Every cargo command takes
  `--locked`, and clippy runs once more without `--all-targets` (the libs as they ship; `CLAUDE.md` "Checks"). No
  `--workspace` or `--all` run here; the orchestrator runs those when it closes a wave.
- Only the owned files changed.

## Out of scope

- Apply, restore, plan switch, user changes, session end, console, settings owner, Quit, Restore my power settings
  (`p2-core-settings`). Lid, lock, floor, countdown, closed-lid report (`p2-core-lid`).
- Policies and the view (done; a bug found goes under "Found outside scope" with a failing test name).
- Contract files, `lib.rs`, `Cargo.toml`, `docs/architecture.md`: proposals go to the Result.

## Questions for Haik

Each with the worker's default.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-03: card written by `p2-plan-core`.

## Result

What was done, files changed, checks and outcomes, open items. Describe the seams and the `Sim` API for the next cards.
