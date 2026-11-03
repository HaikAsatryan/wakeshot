# When to check for, download and apply an update

Phase: 6. Status: ready. Needs Haik: no (Question 3 of `p6-plan-ship` decides only the core's notice, not this card).
Agent: `wakeshot-worker`. Size: about 70K tokens.
Real laptop: no.
Runs after: the contracts card (`update.rs` with its module line; `Event::UpdateReady`, `Command::ApplyUpdate`,
`Outcome::Update`). Parallel with: any card that does not own `crates/wakeshot-windows/src/update.rs`.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p6-update-schedule\target`.

## Goal

The update thread's logic, with no `velopack` in it: when to check, what to post to the loop, and how to run an apply
the core asked for. `velo.rs` implements the feed later (`p6-velopack-dependency`).

## Read first

- `docs/phases/06-ship.md`: "Updates".
- `docs/architecture.md` section 3 ("Clocks"), section 4 ("Tray threads": other threads post to the loop window,
  never to the thread), section 8 (errors and log).
- `crates/wakeshot-core/src/event.rs`, `command.rs`, `time.rs`; `crates/wakeshot-core/src/testing.rs` (`FakeClock`).
- `docs/research.md`: "Ship track planning checks" (`UpdateManager` API).

## Deliverables

`crates/wakeshot-windows/src/update.rs`, owned whole:

- Trait `UpdateFeed`: one call checks and downloads, giving the new version or none; one call starts the updater that
  waits for this process to exit; an error that means "not installed". Names are this card's to pick; the doc comment
  says what `velo.rs` must do.
- The thread body: first check 5 minutes after start, then every 12 hours of wall time; a due check missed during
  sleep runs on the next wake-up; one `Event::UpdateReady { version }` per version found; on an apply request, call
  the feed and post `Event::Done(Outcome::Update(result))`; a failed check or download is a log line and waits for the
  next check; "not installed" ends the thread after one log line; a stop request ends it at once.
- Clock, wait, post and log are injected, so tests use the fake clock and never sleep for real.

## Acceptance checks

- Tests written first, with a fake feed and the fake clock: the schedule (5 minutes, then 12 hours); a wall clock
  jump forward (sleep) and back (a clock change); one `UpdateReady` per version; a failed check, then a good one; apply
  success and failure posting `Outcome::Update`; "not installed"; stop while waiting.
- `rustfmt --edition 2024 --check <owned files>`; `cargo clippy --locked -p wakeshot-windows --all-targets --target <t>
  -- -D warnings` for both targets; `cargo test --locked -p wakeshot-windows update::`. Clean in the owned files; the
  own tests pass.
- Check rule: `docs/plan.md` "How we work" step 2 and "Checks while other cards run" in this track's phase file. A
  failure in a file another card owns goes in the Log: wait and run again; if it is still there when this card's own
  work is done, list it under "Found outside scope". Never edit, format or `--fix` that file. Every cargo command takes
  `--locked`, and clippy runs once more without `--all-targets` (the libs as they ship; `CLAUDE.md` "Checks"). No
  `--workspace` or `--all` run here; the orchestrator runs those when it closes a wave.

## Out of scope

- `velopack` and `velo.rs`; when the core issues `ApplyUpdate` (core track); starting the thread from the tray
  (integration).

## Questions for Haik

Each with the worker's default.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-03: card written by `p6-plan-ship`.

## Result

What was done, files changed, checks and outcomes, open items.
