# Live test: Codex CLI (with Haik)

Phase: 4. Status: waits for Haik's Codex access (back in v1, Haik 2026-11-06; see "Round 2" at the end). Needs Haik: yes, about 20 minutes at the laptop: install or sign in to Codex with his own
account, then type the numbered steps the worker gives. Agent: `wakeshot-worker`. Size: about 100K tokens.
Runs after: `p4-live-claude` (the harness). Parallel with: code cards of any track that own none of the files below.
Never at the same time as another live card or any card that needs Haik's hands or the screen.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p4-live-codex\target`.
Uses the real laptop: yes. May need an install (Codex CLI) by Haik. No power settings, screen or sound.

## Goal

The one live test of Codex (product: "Haik has personal Codex ... access for one live test"): check the Codex rules
and the lock file to process tie on a real install, capture redacted lines so any fix can be tested offline, and mark
the docs tested or not.

## Read first

- `docs/phases/04-sources.md`: track notes 7, 8, Question 1 and Haik's answer (the default, 2026-11-03 18:09).
- `docs/research/other-agent-signals.md`: "OpenAI Codex CLI".
- `crates/wakeshot-core/src/sources/codex.rs`, `crates/wakeshot-windows/src/lockowner.rs`, `watch/agents.rs`,
  `examples/agent-watch.rs` and `docs/tasks/p4-live-claude.md` Result.

## Deliverables

1. Before Haik: check whether Codex is installed (`Get-Command codex`, `%USERPROFILE%\.codex`), which version, and
   whether `CODEX_HOME` is set. Write Haik's steps in this card under "Steps for Haik", numbered, each one action:
   install if needed (the official way at that date, checked in Codex's README; npm or winget), sign in, open a
   throwaway folder the worker made, then the session steps: a prompt that runs a 60 s command; a prompt that needs
   an approval, left waiting 1 minute, then approved; Esc during a turn; a clean exit; a second session closed with
   Task Manager (End task on `codex.exe`: the worker never kills a process it did not start). Set Status `blocked`
   and stop; the orchestrator relays the steps.
2. During the steps: the harness runs with `--raw`. Record per step: lock file appeared and gone, Restart Manager
   holder (pid and start time equal to the `codex.exe` Haik runs?), the rollout found, the activity posted and its
   delay, the liveness after End task (lock file left behind? `Gone` at once?).
3. Fixtures: the redacted lines of this session into `crates/wakeshot-core/tests/fixtures/codex/live/` (same
   redaction rules as the Claude fixtures: no prompts, no paths with the user name, fake ids), a test that replays
   them, and fixes in `sources/codex.rs`, `lockowner.rs` or `watch/agents.rs`, each with a test that failed before.
4. A section "Phase 4 live test (2026-11-..)" in `docs/research/other-agent-signals.md` and short lines in
   `docs/research.md`: what holds, what was verified, whether the "can outlive its session" mark goes (track note 7),
   the Codex version. If the test could not run (no access, install failed), say so: Codex stays "not tested live".
5. Clean-up: the harness and every process this card started ended by pid; the throwaway folder deleted. Haik's Codex
   install and sign-in stay unless he says otherwise.

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

- Copilot (`p4-live-copilot`). Codex goals and the app-server daemon beyond what the steps show. Any file not listed
  above.

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

- Order: after `p4-live-copilot` (swapped 2026-11-06).
- This card also wires the source into the app: add it to the sources the tray starts in
  `crates/wakeshot-windows/src/app/tray_mode.rs`, so the installed app holds for it, and test that through the app.
- Haik's answers to sign-off items 1 and 2 (`docs/phases/02-06-signoff.md`) apply.
- Access: Haik is not sure he can use Codex. OpenAI's pricing page (read 2026-11-06) lists the CLI for Plus, Pro, Business, Enterprise and Edu, and for an API key at API rates; Free and Go are not clear. Starts when Haik says he has access.
