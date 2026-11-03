# Core property tests over random event sequences

Phase: 2. Status: ready. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 130K tokens.
Real laptop: no (pure core tests). Build folder: `CARGO_TARGET_DIR=<scratchpad>\p2-core-props\target`.
Runs after: `p2-core-lid` and `p2-ownership-apply`. Parallel with: any card of another track.

## Goal

The property tests of architecture section 9 over the whole core, end to end in pure code: a `Sim` world answers
`Apply`, `Restore` and `ReadPlan` by running the real ownership routines on `FakePlans`, with outcomes that arrive
late, so a property is checked against what Windows would hold, not only against the commands. Write them from the
design: you are a second pair of eyes on the three core cards.

## Read first

- `CLAUDE.md`; `docs/plan.md` "How we work"; `docs/phases/02-core.md` (all track notes).
- `docs/product.md` (the job; power behavior; user settings safety).
- `docs/architecture.md` section 3 (all), section 5 "Power settings", "Ownership", "Lock and hibernate"; section 9
  core property tests (the list this card turns into tests).
- `docs/reviews/p1-design-power-verify.md` minors 4, 6, 9, 11; `p1-contracts-domain.md` M5.
- `crates/wakeshot-core/src/`: `sim.rs`, `ownership/fake.rs`, the contract files; Results of the core and ownership
  cards.

## Owns

`crates/wakeshot-core/src/scenarios/props.rs`, `sim.rs` (the world). Nothing else.

## Deliverables

1. In `sim.rs`, a world: a `FakePlans` the routines run on; the IO side's answers (`Apply` through
   `ownership::apply`, `Restore` through `ownership::restore`, `ReadPlan` through `ownership::check` plus the plan
   values, each `UserChanged` the check reports); answers delivered now or after later events (late outcomes); lid,
   AC, battery, input, suspend and resume, session and console changes, user changes and plan switches in the world,
   each turned into its event.
2. `scenarios/props.rs`, random sequences of world steps and events (hold files and Claude sessions coming and going,
   menu choices, ticks and time passing), one `proptest!` per property:
   - owned keys are always restored after the last hold, Quit and end of session (checked on `FakePlans`: stored and
     in force back at the user's values once the retries settle);
   - a released key is never written again in its episode, on any plan (power verify minor 9);
   - a restore retry never takes an entry a running hold uses (power verify minor 4);
   - no `Apply` after `Session(Ending)` until `EndCancelled` (power verify minor 6);
   - no `Apply` and no display request while another session is on the console;
   - no request is on with no hold, and none while stepped back;
   - no `Hibernate` within 5 minutes of a resume or of input;
   - no `Lock` when the lid action in force is the user's own;
   - every `Lock`, step back and user change comes with a notice or a sound in the same step; every `Hibernate`
     counts as its own explanation (track note 4); each run of failed restores gets one `RestoreFailed` (decision 0);
   - a closed laptop never runs on battery on our Do nothing unlocked for more than 30 s (power verify minor 11; settle 30 s, Haik 2026-11-03);
   - no `ApplyUpdate` while a hold counts, a hold file names a live process, or a key is owned (if `p2-core-lid` built
     the update item);
   - `handle` never panics, and `next_deadline` is never in the past after a `Tick`.
3. Shrinking gives readable cases: steps print as short lines.

## Acceptance checks

Run from the repo root with the build folder above. Record each outcome.

- `cargo test --locked -p wakeshot-core scenarios::props`: all pass at the default case count, in under 60 s in total on
  this laptop (record the time).
- One long run: `PROPTEST_CASES=10000 cargo test --locked -p wakeshot-core scenarios::props --release`: passes (record
  time).
- Mutants, each made by hand in `core.rs` or `src/core/` of a scratch copy of the workspace, never in the repo: those
  files are the core cards' and other cards test against them (verify review Major 2; the copy, its build folder and
  the clean-up are in `docs/phases/02-core.md` note 20). Each must fail a property (record which, and the shrunk
  case): `released` keyed by plan; the retry takes the active plan's entries while holding; `Apply`
  after `Session(Ending)`; a lid-close lock with the user's own Do nothing; no lock at AC to battery with the lid
  closed; `UserChanged` without a notice; requests left on after the last hold.
- `cargo test --locked -p wakeshot-core`: all pass. `rustfmt --edition 2024 --check <owned files>`: no diff. `cargo
  clippy --locked -p wakeshot-core --all-targets --target x86_64-pc-windows-msvc -- -D warnings` and the same for
  `aarch64-pc-windows-msvc`: exit 0.
- A bug a property finds in another card's file: not fixed here; the shrunk case goes under "Found outside scope" and
  the Result says so. The orchestrator routes the fix.
- Only the owned files changed.
- Check rule: `docs/plan.md` "How we work" step 2 and "Checks while other cards run" in this track's phase file. A
  failure in a file another card owns goes in the Log: wait and run again; if it is still there when this card's own
  work is done, list it under "Found outside scope". Never edit, format or `--fix` that file. Every cargo command takes
  `--locked`, and clippy runs once more without `--all-targets` (the libs as they ship; `CLAUDE.md` "Checks"). No
  `--workspace` or `--all` run here; the orchestrator runs those when it closes a wave.

## Out of scope

- Fixing the core or the routines (findings are routed). The journal property test (`p2-ownership-props`).
- Contract files, `lib.rs`, `Cargo.toml`, `docs/architecture.md`.

## Questions for Haik

Each with the worker's default.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-03: card written by `p2-plan-core`.

## Result

What was done, files changed, checks and outcomes, open items.
