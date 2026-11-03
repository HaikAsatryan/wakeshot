# Power requests: set, clear, retake

Phase: 3. Status: ready. Needs Haik: no. Real laptop: no (one test holds a SYSTEM request for under a second; no
setting changes). Agent: `wakeshot-worker`. Size: about 60K tokens.
Runs after: Haik's sign-off of the track plans; the contracts card. Parallel with: every other wave 1 card of this
track and any card of tracks 2, 4, 5, 6.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p3-requests\target`.

## Goal

Fill `power::requests`: the one request object per tray that runs `Command::Requests` and `Command::Retake`, with a
dry-run form and a fake.

## Read first

- `CLAUDE.md`; `docs/phases/03-windows-power.md` (track notes 1, 2, 8).
- `docs/architecture.md`: section 5 "Power requests"; section 3 (`Command::Requests`, `Retake`, `Outcome::Requests`).
- `docs/product.md`: "Power behavior", first two points.
- `docs/research.md`: "Lid and sleep spike", the "Resume" and "SystemExecutionState" points.
- `spikes/lid-sleep/src/power.rs`: `Hold::request`, `retake`. Port by reading and rewriting.

## Deliverables

`crates/wakeshot-windows/src/power/requests.rs`:

- Trait `RequestApi { set(system, display) -> Result<(), WinError>; retake() -> Result<(), WinError> }`; `retake`
  answers as `Outcome::Requests` (review m7).
- `PowerRequests`: one `PowerCreateRequest` with a simple-string reason "Wakeshot: keeping the PC awake for running
  work", created on first use; `system` sets or clears SYSTEM and EXECUTION together, `display` sets or clears
  DISPLAY; only changes what differs. `retake`: for each type that is on, clear then set, same handle (the tested
  form). A failure part way: clear what this call set, return the error. Drop clears everything and closes the handle.
- `DryRunRequests` (logs each change through `Log`, calls nothing) and `FakeRequests` (behind `test-support`; records
  calls, scripted failures).

## Acceptance checks

Tests in `power::requests::tests`:

- With a recording backend: `set_changes_only_what_differs`, `display_only_when_asked`, `retake_clears_then_sets_each_on_type`,
  `retake_with_nothing_on_does_nothing`, `failure_part_way_rolls_back`, `drop_clears_all`.
- `request_sets_and_clears` (not ignored: under a second, nothing persistent; no `real_` prefix, since `real_` means
  ignored, review M9): create, set SYSTEM, retake, clear; every call returns `Ok`. Never judge by `powercfg
  /requests` (section 5).

Commands: `rustfmt --edition 2024 --check <owned files>`; `cargo clippy --locked -p wakeshot-windows --all-targets` with
`-D warnings` for both targets; `cargo test --locked -p wakeshot-windows power::requests::`. Record each outcome.

- Check rule: `docs/plan.md` "How we work" step 2 and "Checks while other cards run" in this track's phase file. A
  failure in a file another card owns goes in the Log: wait and run again; if it is still there when this card's own
  work is done, list it under "Found outside scope". Never edit, format or `--fix` that file. Every cargo command takes
  `--locked`, and clippy runs once more without `--all-targets` (the libs as they ship; `CLAUDE.md` "Checks"). No
  `--workspace` or `--all` run here; the orchestrator runs those when it closes a wave.

## Out of scope

- When requests go on or off (core), resume detection (`p3-system-events`), the battery re-take check
  (`p3-haik-checks`).
- Any file but `power/requests.rs`.

## Questions for Haik

Each with the worker's default.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-03 17:00: card written by `p3-plan-windows-power`.

## Result

What was done, files changed, checks and outcomes, open items.
