# Harness: a check tool for the real-laptop and manual checks

Phase: 3. Status: ready. Needs Haik: no. Real laptop: yes for the `cleanup` smoke run only (global lane: it may call
`PowerSetActiveScheme` and deletes plan copies; review M7); other smoke runs touch no setting; the plan-copy runs
happen in `p3-real-checks` and `p3-haik-checks`. Agent: `wakeshot-worker`. Size: about 120K tokens.
Runs after: `p3-power-io`, `p3-recovery`, `p3-requests`, `p3-session`, `p3-system-events` (and through them store,
log, process, power-plan). Parallel with: `p3-stop-points` and any card of tracks 2, 4, 5, 6.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p3-harness\target`.

## Goal

One example exe, `power_check`, that plays the tray's power side on a plan copy, so the agent and Haik can run the
manual checks of this track before integration. The guard and restore mode it uses are the real tray exe's.

## Read first

- `CLAUDE.md`; `docs/phases/03-windows-power.md` (track notes 3, 8; "Manual checklist").
- `docs/architecture.md`: section 9 "Manual checklist" (items 1 to 6, 11, 15 and the plan copy rule).
- `docs/research.md`: "Power settings spike" (plan copy, what Settings writes), "Lid and sleep spike" (the hold runs,
  ticks, gaps), "Spike launch and stop checks".
- The spikes' hold and log code (`spikes/lid-sleep/`): ticks with wall and unbiased time, gap lines. Port by reading.
- After the cards above: the public API of `power::plan` (`TestPlan`, `CopyAsActive`), `power::executor`,
  `power::requests`, `power::system`, `session`, `recovery::TrayGuard`, `store`, `log`.

## Deliverables

`crates/wakeshot-windows/examples/power_check.rs` (built with `--features test-support`; the contracts card declares
it with `required-features`). Every subcommand logs through `LogWriter` into its data folder (`--dir`, default the real
one only where a check needs it), prints Haik's values (active plan; lid AC and DC; sleep AC and DC; energy saver DC)
at start and at exit, and prints the exact `powercfg` commands that undo it.

- `hold [--no-request] [--active-copy]`: SYSTEM and EXECUTION held, retaken on each resume; a tick line every 5 s
  (wall, unbiased, input idle) and a line per lid, power, plan, suspend and resume event, and a "gap" line when a tick
  comes late. `--active-copy`: a plan copy with lid DC 0 and sleep DC 0 made active for the run, the original plan
  made active again and the copy deleted at exit (checks 1, 2, 3).
- `fake-tray`: a plan copy, never active; the real Apply to it through `CopyAsActive` with the executor; the real
  journal (owner: this process), RunOnce value and guard (`TrayGuard`, the real tray exe in `--guard`); then waits
  (checks 4 and 5; ended in Task Manager or by PID).
- `end-session`: like `fake-tray` in its own data folder, plus a hidden top-level window: at `WM_QUERYENDSESSION` and
  `WM_ENDSESSION` it runs the restore at once and logs the time each step took (check 6).
- `session-watch`: a hidden top-level window registered for session notices; logs each one (lock, unlock, console).
- `cycles <n>`: n Apply and restore cycles on an active plan copy through the executor, logging each
  `PowerSetActiveScheme` time; the original plan back and the copy deleted at the end (check 15).
- `medium-probe`: prints its integrity level and creates `Global\Wakeshot.Probe.<pid>` (check 11, first part).
- `cleanup`: the plan active at the first start of any subcommand (stored in the data folder) active again, every
  "Wakeshot test <pid>" copy whose pid is not running deleted (never a copy another card's live run holds, review
  M7), leftover harness journal and RunOnce value removed, values printed.

## Acceptance checks

- `cargo clippy --locked -p wakeshot-windows --examples --features test-support --target <t> -- -D warnings` for both
  targets; `rustfmt --edition 2024 --check` on `examples/power_check.rs`; `cargo test --locked -p wakeshot-windows
  --features test-support --example power_check` passes.
- Smoke runs that touch no setting, each recorded with its output: `--help`; `medium-probe`; `session-watch` for 5 s;
  `hold --no-request` for 15 s (ticks every 5 s); `hold` for 10 s (a request held 10 s, then released). `cleanup`
  with nothing to clean, in a real-laptop slot only (header). Plan-copy runs are not done here.
- A dry pass of every subcommand's argument parsing and its printed undo commands (a unit test or a `--plan-only`
  flag that prints what it would do).
- Check rule: `docs/plan.md` "How we work" step 2 and "Checks while other cards run" in this track's phase file. A
  failure in a file another card owns goes in the Log: wait and run again; if it is still there when this card's own
  work is done, list it under "Found outside scope". Never edit, format or `--fix` that file. Every cargo command takes
  `--locked`, and clippy runs once more without `--all-targets` (the libs as they ship; `CLAUDE.md` "Checks"). No
  `--workspace` or `--all` run here; the orchestrator runs those when it closes a wave.

## Out of scope

- Running `fake-tray`, `end-session`, `cycles` or `hold --active-copy` (the next two cards); fixing a bug in another
  card's file (record it).
- Any file but `examples/power_check.rs`.

## Questions for Haik

Each with the worker's default.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-03 17:00: card written by `p3-plan-windows-power`.
- 2026-11-03 21:25, orchestrator, from `p3-recovery`'s Result: a guard inherits the harness's environment, so the
  harness must set `WAKESHOT_DATA_DIR` before `ensure`; otherwise a debug guard reads the real data folder. Read the
  Results of `p3-power-io`, `p3-recovery`, `p3-requests`, `p3-session` and `p3-system-events` for their APIs.

## Result

What was done, files changed, checks and outcomes, open items.
