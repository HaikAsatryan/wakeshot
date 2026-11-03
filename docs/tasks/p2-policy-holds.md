# Hold rules: agent sessions and hold files

Phase: 2. Status: ready (stopped by Haik about 19:45 before any code). Needs Haik: no. Agent: `wakeshot-worker`. Size: about 100K tokens.
Real laptop: no (pure core code and tests). Build folder: `CARGO_TARGET_DIR=<scratchpad>\p2-policy-holds\target`.
Runs after: the contracts card. Parallel with: `p2-ownership-restore`, `p2-policy-power`, `p2-present`, and any card
of another track.

## Goal

The pure rules that turn agent session facts and hold file facts into holds, with the time each answer next changes,
so the state machine (`p2-core-holds`) only stores and combines them. Parsers never decide policy (section 7); these
rules do.

## Read first

- `CLAUDE.md`; `docs/plan.md` "How we work"; `docs/phases/02-core.md` (track notes 12, 14, 17).
- `docs/product.md` "What keeps the PC awake" and UX "Automatic", "Keep awake".
- `docs/architecture.md` section 3 "Clocks"; section 6 hold format and the bullets under it; section 7 (all);
  section 5 "Liveness" (only Yes, Gone, Unknown).
- `docs/research/claude-code-signals.md` "Recommended detection rules" (only to understand the facts the parser
  gives; parsing is the sources track's).
- `crates/wakeshot-core/src/`: `sources.rs`, `holds.rs`, `process.rs`, `time.rs`, `policy/` stubs.

## Owns

`crates/wakeshot-core/src/policy/sessions.rs`, `policy/hold_files.rs`. Nothing else.

## Deliverables

1. `policy/sessions.rs`: for one session (`SessionFacts`, the adapter's `Alive`, and what the core remembers about it:
   last liveness answer, the wall time it first saw the session waiting) and `Now`: does it hold, its label, whether
   it needs the "update Wakeshot" notice, and the next `Deadline` at which the answer changes. Rules:
   - `Alive::Gone`: no hold. `Unknown`: the last answer stands (the caller passes it).
   - `Working` holds while the process lives; `open_tasks > 0` holds while the process lives (decision 8).
   - A timer holds from its due time minus 1 hour to its due time plus 2 minutes (decision 6, margin from section 7).
   - `Waiting` holds only with `remote`, until `since_wall_ms` (or the first time the core saw it waiting) plus 1 hour
     (decision 7).
   - `FormatHealth::Unreadable`: no signal for that session, notice wanted. `UnknownValue`: counts as working while
     the process lives, notice wanted (decision 37).
   - Wall time can jump either way (`FakeClock::set_wall`): a deadline in the past is due now; a rule never holds
     forever because of a jump.
2. `policy/hold_files.rs`: for one `HoldFacts` and `Now` and Automatic: does it count, its label, its screen wish,
   whether to delete the file (`Alive::Gone`), and its next `Deadline` (`expires`). Rules (section 6, decision 11,
   32): counts while alive and before `expires`; Unknown keeps the last answer; Automatic off stops `Run` and
   `Script` but not `KeepAwake`; `screen` counts only for `KeepAwake`. Also the current Keep awake choice for the menu
   from the `keep-awake.json` facts (Off, a number of minutes left, Until I turn it off).
3. Unit tests, written first, one named test per rule and edge: each activity; each liveness answer; a timer just
   before its window, inside it, at its end, past it; two timers; waiting with and without `remote`, with and without
   `since_wall_ms`, at the hour; a wall clock jump back and forward; each `FormatHealth`; each hold kind with
   Automatic on and off; expiry exactly at `expires`; Gone gives a delete; Unknown keeps the last answer.

## Acceptance checks

Run from the repo root with the build folder above. Record each outcome.

- `cargo test --locked -p wakeshot-core policy`: all pass; list the test names in the Result.
- `cargo test --locked -p wakeshot-core`: all pass. `rustfmt --edition 2024 --check <owned files>`: no diff.
- `cargo clippy --locked -p wakeshot-core --all-targets --target x86_64-pc-windows-msvc -- -D warnings` and the same
  with `--target aarch64-pc-windows-msvc`: exit 0.
- No clock read, no I/O, no new dependency: `cargo tree -p wakeshot-core -e normal` unchanged; a grep of the two
  files finds no `std::time`, `std::fs`, `std::env`.
- Check rule: `docs/plan.md` "How we work" step 2 and "Checks while other cards run" in this track's phase file. A
  failure in a file another card owns goes in the Log: wait and run again; if it is still there when this card's own
  work is done, list it under "Found outside scope". Never edit, format or `--fix` that file. Every cargo command takes
  `--locked`, and clippy runs once more without `--all-targets` (the libs as they ship; `CLAUDE.md` "Checks"). No
  `--workspace` or `--all` run here; the orchestrator runs those when it closes a wave.
- Only the two owned files changed.

## Out of scope

- Parsing status files, transcripts, hold files and `expires` (sources track). Building the Keep awake file
  (`p2-core-holds`). Labels as words and their grouping in the tooltip (`p2-present`, `text.rs`).
- State, events and commands (`p2-core-holds`). Contract files, `lib.rs`, `Cargo.toml`, `docs/architecture.md`.

## Questions for Haik

Each with the worker's default.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-03: card written by `p2-plan-core`.
- 2026-11-03: worker started; reading the card, docs and the contract files.

## Result

What was done, files changed, checks and outcomes, open items. Include the public API for `p2-core-holds`.
