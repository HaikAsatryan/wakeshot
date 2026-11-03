# Live check: the watch thread and the Claude Code source end to end

Phase: 4. Status: ready. Needs Haik: optional, about 3 minutes (the VS Code check in deliverable 4). Agent:
`wakeshot-worker`. Size: about 120K tokens. Split trigger (review m13): if the run shows bugs whose fixes would take
this card past about 150K, stop after the harness and the run, list each bug under "Found outside scope" with its
log lines, and the orchestrator makes a fix card.
Runs after: `p4-watch-thread`, `p4-claude-transcript`, `p4-cli-run`, `p3-process` (the real `process::Liveness`),
and the Claude parsing review with its blocker and major fixes (`docs/phases/02-06-lighter.md`). Codex and Copilot
moved to after v1 (Haik, 2026-11-03), so this card runs the Claude source only. Parallel with: code cards of any track that own none of the
files below. Never at the same time as `p4-claude-live-shapes`, `p4-live-codex`, `p4-live-copilot`.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p4-live-claude\target`.
Uses the real laptop: yes, Claude Code test sessions (Haiku) in throwaway folders. No power settings, screen or sound.
Lane: agent-session lane, one at a time with the other p4 live cards; may run beside the global lane (power, screen,
sound, installs) (review m19). Its `--dry-run` harness uses the dry-run data folder: never beside `p6-install-e2e`
(review m11).

## Goal

Run the real watch thread with the real sources on this laptop, check every Claude Code case of the live tests table
against what it posts, and leave a harness the later Codex and Copilot live tests can reuse.

## Read first

- `CLAUDE.md` "Facts to keep in mind"; never read `.key` files.
- `docs/phases/04-sources.md` (all track notes).
- `docs/research/claude-code-signals.md`: "Live test results", "Live tests still open", the "Phase 4 live shapes"
  section; `docs/tasks/p4-claude-live-shapes.md` Result (how the sessions were driven).
- `crates/wakeshot-windows/src/watch/` and the Claude source in `crates/wakeshot-core/src/sources/claude/`.

## Deliverables

1. `crates/wakeshot-windows/examples/agent-watch.rs`: starts `WatchThread` with `ClaudeSource` (home from the usual
   variables; a source list, so a later source is one more line), the real `process::Liveness`, `SystemZone`, a data folder given on the
   command line (default: the dry-run folder, never the real one), and an `EventSink` that prints each event in one
   redacted line (source, session key, activity, timers as local times, open tasks, liveness, format health; hold ids,
   kinds, liveness). Plus `--raw <file>`: also log the kinds (never the text) of the lines each tail fed. Ctrl+C stops
   it cleanly.
2. Run every case of the live tests table (1 to 12 and 14) plus resume, a `/clear` during a loop and the SDK path,
   driven like `p4-claude-live-shapes`, with the harness running. For each: what the harness posted, how fast after
   the status write or transcript line, and whether it matches the rules. Also: `wakeshot --dry-run run -- ping -n 20
   127.0.0.1` shows a `Run` hold `Yes` then gone; a script-written hold file with a dead pid shows `Gone`; a crashed
   session (hard kill of a test session this card started) shows `Gone` within 1 s (exit wait).
3. Fix what the run shows wrong, in this track's files that are done (`sources/claude/*`, `sources/cron.rs`,
   `holds/file.rs`, `watch/*`), each fix with a test or fixture that failed before it. Bigger problems: a new card, in
   the Result.
4. Optional, only if Haik is at the laptop and agrees (about 3 minutes): the VS Code UI check from "Live tests still
   open" (his steps are in that section; the harness shows the session). Otherwise mark it not tested.
5. A section "Phase 4 end to end (2026-11-..)" at the end of `docs/research/claude-code-signals.md`, and short lines
   in `docs/research.md`.
5a. Added by the orchestrator (2026-11-03 18:51): in the older parts of `docs/research/claude-code-signals.md`, fix
    the lines that `p4-claude-live-shapes` "Found outside scope" lists as contradicted by its live run ("Short
    answer" third bullet, "Claude Code itself uses two outside methods" item 1, the "Wakeup armed" row, rule 3's
    closing statuses), using what your own run confirms. Replace the old text; do not append history.
6. Clean-up: every session and process this card started ended by pid; test transcripts and scratch folders deleted;
   the dry-run data folder left as it was.

## Acceptance checks

- Every case in deliverable 2 has a recorded outcome (pass, fixed with a test, or a new card).
- `rustfmt --edition 2024 --check <owned files>`; `cargo clippy --locked -p wakeshot-core -p wakeshot-windows
  --all-targets --target x86_64-pc-windows-msvc -- -D warnings` and the same for `aarch64-pc-windows-msvc`; `cargo test
  --locked -p wakeshot-core -p wakeshot-windows`: clean as the check rule below says after the fixes.
- No process this card started is left; the real data folder `%LocalAppData%\Wakeshot` was not touched (outside
  `dry-run`).
- Check rule: `docs/plan.md` "How we work" step 2 and "Checks while other cards run" in this track's phase file. A
  failure in a file another card owns goes in the Log: wait and run again; if it is still there when this card's own
  work is done, list it under "Found outside scope". Never edit, format or `--fix` that file. Every cargo command takes
  `--locked`, and clippy runs once more without `--all-targets` (the libs as they ship; `CLAUDE.md` "Checks"). No
  `--workspace` or `--all` run here; the orchestrator runs those when it closes a wave.

## Out of scope

- Codex and Copilot sessions (`p4-live-codex`, `p4-live-copilot`). The tray and the core's decisions (integration).
  Files of other tracks. Any CLI file (their cards).

## Questions for Haik

Each with the worker's default.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-03 16:53: card written by `p4-plan-sources`.
- 2026-11-03 21:39, orchestrator, from `p4-claude-status`' Result: (2) for the resume rule, `procStart` is a
  better divider than `startedAt`; check it in the live resume test. (3) `CLAUDE_CONFIG_DIR` moves the whole Claude
  folder per Claude Code's docs; whether `sessions\` follows it is not verified. Check it once if cheap, else list it
  as not tested.
- 2026-11-03 23:55, orchestrator, from the Claude parsing review (`docs/reviews/p4-claude-parsing.md`): m6 (no
  Claude signal at all under `CLAUDE_CONFIG_DIR`) is yours: check it; if `sessions\` moves, say what the tray must
  read. Also check live, if cheap: a background task stopped with `TaskStop` and a Monitor that hits its timeout
  (B1 and M1, fixed in the core before this card), and whether a stop from the UI writes its `killed` notice before
  the user's next prompt (M1, not verified).

## Result

What was done, files changed, checks and outcomes, open items.
