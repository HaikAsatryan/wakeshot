# Live test: Copilot CLI (with Haik)

Phase: 4. Status: ready once `p4-codex-source` and `p4-copilot-source` are done (back in v1, Haik 2026-11-06; see "Round 2" at the end). Needs Haik: yes, about 20 minutes at the laptop: install or sign in to Copilot CLI with his
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

## Round 2: back in v1 (2026-11-06)

Written by the orchestrator. Haik, 2026-11-06: Codex CLI and Copilot CLI ship in v1 after all. Changes to this card:

- Order: this card now runs first (Haik is sure of his Copilot access, not yet of Codex), after both source cards; `p4-live-codex` runs after it.
- This card also wires the source into the app: add it to the sources the tray starts in
  `crates/wakeshot-windows/src/app/tray_mode.rs`, so the installed app holds for it, and test that through the app.
- Haik's answers to sign-off items 1 and 2 (`docs/phases/02-06-signoff.md`) apply.
- Access: GitHub's plans page: "All plans include Copilot CLI and the Copilot app" (read 2026-11-06), so Haik's Copilot Free works. The test is Copilot CLI in a terminal, not Copilot chat in VS Code.
- From `p4-copilot-source` (2026-11-06, Copilot 1.0.106): a new `copilot-runtime.exe` hosts sessions for SDK apps, so
  the `AnyProcess` image fallback (`copilot.exe`) would miss those; check which process holds the in-use lock on a
  real install and fix the fallback if needed. Also check that a schedule fire's `source` (`schedule-...`) carries
  the schedule id, and the `events.jsonl` rotation (`session.journal_link`).
- From `p4-copilot-vscode-research` (2026-11-06): VS Code's "Copilot" harness and Background sessions write the same
  `session-state\<id>\events.jsonl` but run as `copilot-runtime.exe` or `Code.exe`; an `inuse.<pid>.hold` file was
  seen next to `inuse.<pid>.lock`. Check both on the real install.
