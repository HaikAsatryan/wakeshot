# Live test: Copilot CLI (with Haik)

Phase: 4. Status: ready. Needs Haik: yes, about 20 minutes at the laptop: install or sign in to Copilot CLI with his
own account, then type the numbered steps the worker gives. Agent: `wakeshot-worker`. Size: about 100K tokens.
Runs after: `p4-live-codex` (both may fix `lockowner.rs` and `watch/agents.rs`). Parallel with: code cards of any
track that own none of the files below. Never at the same time as another live card or any card that needs Haik's
hands or the screen.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p4-live-copilot\target`.
Uses the real laptop: yes. May need an install (Copilot CLI) by Haik. No power settings, screen or sound.

## Goal

The one live test of Copilot CLI: check the inferred rules, the in-use lock file name and its tie to the process,
the write timing of `events.jsonl`, and scheduled prompts on a real install; capture redacted lines; mark the docs
tested or not.

## Read first

- `docs/phases/04-sources.md`: track notes 7, 9, Question 2 and Haik's answer (the default, 2026-11-03 18:09).
- `docs/research/other-agent-signals.md`: "GitHub Copilot CLI" and the Codex live test section.
- `crates/wakeshot-core/src/sources/copilot.rs`, `crates/wakeshot-windows/src/lockowner.rs`, `watch/agents.rs`,
  `examples/agent-watch.rs`; `docs/tasks/p4-live-codex.md` Result.

## Deliverables

1. Before Haik: check whether Copilot CLI is installed (`Get-Command copilot`, `%USERPROFILE%\.copilot`; VS Code's
   `copilot-shim` is not the CLI), the version, `COPILOT_HOME`. Write "Steps for Haik", numbered, one action each:
   install if needed (the official way at that date), sign in, open a throwaway folder, then: a prompt that runs a
   60 s command; a prompt that needs an approval, left waiting 1 minute, then approved; `/every 2m` (or the current
   form of a scheduled prompt) with a short prompt, wait for one run, cancel it; Esc during a turn; a clean exit; a
   second session closed with Task Manager. Set Status `blocked` and stop; the orchestrator relays the steps.
2. During the steps: the harness runs with `--raw`. Record: the session folder and the lock file name (does it hold
   the pid?), Restart Manager holder, whether `events.jsonl` lines land at once or in batches (delay per event),
   `permission.requested` and `permission.completed`, the `session.schedule_*` lines and their fields, the activity
   and timers posted, liveness after End task.
3. Fixtures: redacted lines into `crates/wakeshot-core/tests/fixtures/copilot/live/`, a test that replays them, and
   fixes in `sources/copilot.rs`, `lockowner.rs` or `watch/agents.rs`, each with a test that failed before.
4. A section "Phase 4 live test (2026-11-..)" for Copilot in `docs/research/other-agent-signals.md` and short lines in
   `docs/research.md`. If the test could not run (no plan, install failed), Copilot stays "not tested live".
5. Clean-up as in `p4-live-codex`.

## Acceptance checks

- Each step of deliverable 1 has a recorded outcome.
- `rustfmt --edition 2024 --check <owned files>`; `cargo clippy --locked -p wakeshot-core -p wakeshot-windows
  --all-targets --target x86_64-pc-windows-msvc -- -D warnings` and the same for `aarch64-pc-windows-msvc`; `cargo test
  --locked -p wakeshot-core -p wakeshot-windows`: clean as the check rule below says after the fixes.
- The fixtures pass the secret scan patterns and hold no prompt text.
- Check rule: `docs/plan.md` "How we work" step 2 and "Checks while other cards run" in this track's phase file. A
  failure in a file another card owns goes in the Log: wait and run again; if it is still there when this card's own
  work is done, list it under "Found outside scope". Never edit, format or `--fix` that file. Every cargo command takes
  `--locked`, and clippy runs once more without `--all-targets` (the libs as they ship; `CLAUDE.md` "Checks"). No
  `--workspace` or `--all` run here; the orchestrator runs those when it closes a wave.

## Out of scope

- Codex (done). `/keep-alive`. Any file not listed above.

## Steps for Haik

Written by the worker in deliverable 1.

## Questions for Haik

Each with the worker's default.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-03 16:53: card written by `p4-plan-sources`.

## Result

What was done, files changed, checks and outcomes, open items.
