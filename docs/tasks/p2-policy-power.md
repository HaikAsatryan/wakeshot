# Power rules: requests, lid, lock, battery floor, countdown

Phase: 2. Status: ready (stopped by Haik about 19:45 before any code). Needs Haik: no. Agent: `wakeshot-worker`. Size: about 90K tokens.
Real laptop: no (pure core code and tests). Build folder: `CARGO_TARGET_DIR=<scratchpad>\p2-policy-power\target`.
Runs after: the contracts card. Parallel with: `p2-ownership-restore`, `p2-policy-holds`, `p2-present`, and any card
of another track.

## Goal

The pure rules the episode flow uses (architecture section 2 `policy`: `requests`, `lid`, `lock`, `floor`,
`ending`), each a small function of plain values with its own tests, so `p2-core-holds` and `p2-core-lid` only store
state and call them.

## Read first

- `CLAUDE.md`; `docs/plan.md` "How we work"; `docs/phases/02-core.md` (track notes 4, 11, 16).
- `docs/product.md` "Power behavior" (requests, never lower security, battery floor, work ends with the lid closed).
- `docs/architecture.md` section 3 "Clocks" and "Episode flow" (lid closes, power source changes, battery floor, Keep
  screen on, countdown); section 5 "Lock and hibernate" (what the hibernate thread re-checks, so the core's countdown
  matches it).
- `docs/research.md`: "AC can drop for a few seconds while docked", "Lock", "Clock" (Lid and sleep spike).
- `crates/wakeshot-core/src/`: `settings.rs` (`PlanValues`, `JournalEntry`), `time.rs`, `command.rs`
  (`CountdownReason`), `policy/` stubs.

## Owns

`crates/wakeshot-core/src/policy/requests.rs`, `policy/lid.rs`, `policy/lock.rs`, `policy/floor.rs`,
`policy/ending.rs`. Nothing else.

## Deliverables

1. `requests`: from (a hold counts, Keep screen on asked, stepped back, this session on the console) to `(system,
   display)`. No request with no hold; none while stepped back; display only with Keep screen on and on the console
   (decision 15, section 3 console bullet).
2. `lid`: the lid action in force for the power source (`PlanValues`, `ac`); whether it is ours (a journal entry for
   that plan and key, live value equal to `written`); lock and chime at lid close only when ours (decision 18; a user
   whose own action is Do nothing gets none); the power source change rule with the lid closed (the action in force
   for the new source is ours, and either the user's own was Do nothing before or the change is AC to battery), with
   its 5 s settle as a `Deadline::Mono`.
3. `lock`: Keep screen on idle lock: the user's own sleep timeout for the current source (the journal's `original`
   while we own the key, else the live value; none when 0, Never); due once per idle period (input after it starts a
   new period).
4. `floor`: the floor from `saver_threshold_dc` (20 when it is 0 or 100); stepped back when on battery and the
   percent is below the floor (strictly below, phase 1 "Values I set"); AC back ends it.
5. `ending`: input idle clamped at resume (min of the reported idle and the mono time since the last `Resumed`);
   countdown due at 5 minutes of clamped idle; cancel rules (a new hold, lid open, AC back for `Floor`); the
   `Deadline::Mono` for the next check. The core sends only `Hibernate`; the hibernate thread plays the sound (track
   note 4).
6. Unit tests, written first, one named test per rule and edge: each source with each lid action; ours versus the
   user's own Do nothing; AC to battery and back inside 5 s; battery to AC; the floor at 0, 100, 19, 20, 21; idle
   just under and at 5 minutes; a resume inside the idle period; input during the countdown; Keep screen on with
   sleep timeout Never; one lock per idle period.

## Acceptance checks

Run from the repo root with the build folder above. Record each outcome.

- `cargo test --locked -p wakeshot-core policy`: all pass; list the test names in the Result.
- `cargo test --locked -p wakeshot-core`: all pass. `rustfmt --edition 2024 --check <owned files>`: no diff.
- `cargo clippy --locked -p wakeshot-core --all-targets --target x86_64-pc-windows-msvc -- -D warnings` and the same
  with `--target aarch64-pc-windows-msvc`: exit 0.
- No clock read, no I/O, no new dependency: `cargo tree -p wakeshot-core -e normal` unchanged; a grep of the five
  files finds no `std::time`, `std::fs`, `std::env`.
- Check rule: `docs/plan.md` "How we work" step 2 and "Checks while other cards run" in this track's phase file. A
  failure in a file another card owns goes in the Log: wait and run again; if it is still there when this card's own
  work is done, list it under "Found outside scope". Never edit, format or `--fix` that file. Every cargo command takes
  `--locked`, and clippy runs once more without `--all-targets` (the libs as they ship; `CLAUDE.md` "Checks"). No
  `--workspace` or `--all` run here; the orchestrator runs those when it closes a wave.
- Only the five owned files changed.

## Out of scope

- State, events, commands, notices (`p2-core-holds`, `p2-core-lid`). The hibernate thread's own checks (power
  track). Contract files, `lib.rs`, `Cargo.toml`, `docs/architecture.md`.

## Questions for Haik

Each with the worker's default.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-03: card written by `p2-plan-core`.
- 2026-11-03: worker started; reading the card's sources.

## Result

What was done, files changed, checks and outcomes, open items. Include the public API for the core cards.
