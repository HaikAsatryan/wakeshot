# Live check: the watch thread and the Claude Code source end to end

Phase: 4. Status: done. Needs Haik: optional, about 3 minutes (the VS Code check in deliverable 4). Agent:
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

None. Open points were decided by the worker; they are listed in the Result for Haik's review.

## Found outside scope

Problems seen but not fixed.

- Integration (`p6-int-*`, the card that starts the watch thread in the tray): build the Claude source with
  `ClaudeSource::in_folder(claude_folder(&home, CLAUDE_CONFIG_DIR))`, not `ClaudeSource::new(home)`, or a user with
  `CLAUDE_CONFIG_DIR` set gets no Claude signal at all (live, m6). A value set only in one shell never reaches the
  tray; the user doc could say so (product call, not urgent).
- `docs/research/claude-code-signals.md`, "Background work" table: the task output folder is now
  `%LOCALAPPDATA%\Temp\claude\<16 hex>\<sessionId>\`, not `...\claude\<project folder>\...` (seen live; the new
  section says so; the older rows were left as they are).
- The watch thread watches a data root that is missing at its start only from the next tick (a first hold showed
  19.6 s late in the harness). The tray makes its data folder before the thread starts, so this should not show
  there; integration can confirm (not verified).
- `wakeshot run` with its console closed: the hold went at once, but the CLI log has no exit line for that run, so
  the process likely ended right after it removed the file (inferred). If it ever ends first, the hold shows `Gone`
  and the core deletes it, so no fix is needed; noted for `p5-haik-check`'s window-close step.
- Left on disk: `%LOCALAPPDATA%\Temp\claude\d54d456399d6974f\6b5fff55-...\scratchpad` (empty, made 01:36:55). Very
  likely the resume this card started by mistake in a wrong folder (it exited at the trust dialog), but not proven,
  so not deleted.
- `docs/product.md:43-46` and `docs/architecture.md:899-900` still describe the old UI stop and timer texts (verify 2,
  m2; already tracked there).

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
- 2026-11-04 01:20: worker started; Status in progress.
- 2026-11-04 01:26: harness `examples/agent-watch.rs` builds and runs (smoke test on a scratch data folder). Claude
  Code 2.1.317. Harness started 01:27:39 in a pseudo console on the dry-run folder (missing at start), `--raw` on.
- 2026-11-04 01:28 to 01:41, live (terminal sessions unless named SDK; Haiku; times local): background shell and
  shell Monitor post `working` from the `busy` write (63 ms after `statusUpdatedAt`) to the `idle` write: pass.
  `/loop 1m` timer = `nextFireAt`, then two due times per later fire (built-in and flag-latest): pass. CronCreate
  one-shot timer 01:31:00, dropped at its fire: pass. Dynamic loop: Haiku armed a 1800 s `noop` wakeup (timer
  02:00); Esc cancelled it with no transcript line, so the timer stays (known miss). Waiting: AskUserQuestion and a
  permission prompt post `waiting(remote=true)` with `since` = `statusUpdatedAt`; `/remote-control` dialog stays
  `idle`; Disconnect wrote `bridgeSessionId: null` (now in a write of its own, before the `idle` write); the next
  question posts `waiting(remote=false)`: pass. Durable request: input `durable: true`, result `durable: false`, no
  `scheduled_tasks.json`: still off. `/clear` during a loop: `session_cron_carry` read, timer kept, next fire in the
  new transcript moves it: pass. Resume after `/exit` (Exit and stop tasks) with a `*/5` job: resumed process posts
  the job counted from `procStart` (01:42:12) and no wakeup: pass so far. Hard kill: `Gone` 36 ms after
  `Stop-Process`. `claude agents --json`: exit 0, no harness log line, no unknown value.
- 2026-11-04 01:32: terminal tasks panel stop (Down, Enter, `x`): `queue-operation` `enqueue` with
  `<status>killed</status>` written at the stop (21:32:14.232Z), 6 ms before the `idle` status write, no user line.
- 2026-11-04 01:33 to 01:36, SDK: background shell posts `idle tasks=1`; `TaskStop` closes it (`tasks=0`); a
  `stop_task` control request (what the VS Code panel sends) writes the same `killed` `enqueue`, the harness closed
  the task at its next read (6.8 s later; a tail is read on a sessions folder change or the tick); a Monitor with
  `timeout_ms` 60000 posts a timer at result time plus 60 s; at the timeout Claude wrote only an `<event>` notice
  (no `<status>`), so the timer is the only end: matches rule 3. Background subagent: `working` until its notice
  turn ended: pass.
- 2026-11-04 01:39: `wakeshot --dry-run run -- ping -n 20 127.0.0.1`: first run, data folder missing when the
  harness started, so the hold showed only at the next tick (19.6 s late), then `Gone` at the end; second run with
  the folder present: `Run alive=Yes` 115 ms after start, gone at the child's end. A script hold file with a dead pid:
  `Script alive=Gone` in 24 ms; its reason never printed.
- 2026-11-04 01:41: `CLAUDE_CONFIG_DIR` (m6): a session started with it set to a scratch folder created
  `<that folder>\sessions\` (its `.key` file listed by name only, never opened), nothing under `%USERPROFILE%`. Code
  2.1.317: `$e = (process.env.CLAUDE_CONFIG_DIR ?? join(homedir(), ".claude"))`, sessions folder `join($e(),
  "sessions")`. So with the variable set Wakeshot sees no Claude signal. Session ended by pid at onboarding.
- 2026-11-04 01:42: resumed `*/5` job fired at 01:42:13.7 (posted due time 01:42:12, from `procStart`): pass.
- 2026-11-04 01:43: `wakeshot run` in a pseudo console (ConPTY, as Windows Terminal hosts it): Ctrl+C reached `ping`
  (exit `0xC000013A`), `wakeshot` returned that code and removed its hold 0.34 s later. Ctrl+Break
  (`GenerateConsoleCtrlEvent` inside the pseudo console): `ping` printed its statistics and went on; `wakeshot` kept
  the hold until `ping` ended, then exit 0, hold removed. Closing the pseudo console (its owner killed by pid):
  `wakeshot` and `ping` ended, hold gone 12 ms later. No process left.
- 2026-11-04 01:45 to 01:48: fix for m6: `sources::claude::claude_folder` and `ClaudeSource::in_folder`, test
  `claude_config_dir_names_the_folder_read`; the harness follows `CLAUDE_CONFIG_DIR`. Clippy clean for both
  packages, both targets, with and without `--all-targets`; `wakeshot-core` tests 577 + 11 + 25 pass.
- 2026-11-04 01:52: loop fallback fired at 01:52:00.7, 21 minutes after the wakeup fire at 01:31:00.3, with no timer
  posted in between: pass (no hold by design). All test sessions closed with `/exit` (three through "Exit and stop
  tasks"); the SDK session by closing its stdin (its driver ended by pid): status files removed. Harness stopped with
  Ctrl+C in its pseudo console: `stopped clean=true`, exit 0. 36 pids checked: none alive.
- 2026-11-04 01:53 to 01:58: `wakeshot-windows` tests 471 pass. Docs: new section in `claude-code-signals.md`,
  rules 1, 3 and 5 updated, the stale closing paragraph of "Phase 4 live shapes" and the VS Code steps fixed; new
  section in `research.md`. Clean-up: 11 test project folders, 11 temp folders, 12 `session-env` folders, the
  dry-run folder and the then empty `%LOCALAPPDATA%\Wakeshot` (neither existed before this card), the scratch and
  build folders.

## Result

Done.

What was done:

- Harness `crates/wakeshot-windows/examples/agent-watch.rs`: the real `WatchThread` with the Claude source (a source
  list), `SystemLiveness`, `SystemZone`, `Config::default()`, a tick every 30 s, data folder from `--data` (default
  the dry-run folder; the real one is refused), one redacted line per event, `--raw <file>` with the kinds of every
  fed line (type, subtype, tool names, number and yes/no fields, notice statuses) and each status file's fields, never
  text. Ctrl+C or Ctrl+Break stops it cleanly. It follows `CLAUDE_CONFIG_DIR` like Claude Code.
- Live run, 01:27 to 01:53, Claude Code 2.1.317, Haiku: every case of deliverable 2 passed (table in
  `docs/research/claude-code-signals.md`, "Phase 4 end to end"): cases 1 to 12 and 14, resume (`procStart` divider
  confirmed), `/clear` during a loop, the SDK path, the `wakeshot run` hold (`Yes`, then gone), a dead-pid hold
  (`Gone` in 24 ms), a crash (`Gone` in 36 ms). Case 6: durable jobs still off. Case 4: the Esc cancel stays
  invisible (known miss, bounded).
- Extra checks: `TaskStop` (no notice; closes), a Monitor timeout (only an `<event>` notice; the timer ends it), a UI
  stop of a background shell from the terminal tasks panel and by the SDK `stop_task` request (both write the
  `killed` `enqueue` at the stop; the task closed at the next tail read, 6.8 s and 12 s). Status change to post:
  median 36 ms.
- One bug, fixed: with `CLAUDE_CONFIG_DIR` set, Claude Code writes `sessions\` under that folder (live and code), so
  Wakeshot saw nothing (m6). Added `claude_folder(home, config_dir)` and `ClaudeSource::in_folder`; `new` is
  unchanged. Test `claude_config_dir_names_the_folder_read` (did not compile before).
- Docs: deliverable 5 and 5a (the 5a lines were already corrected by the docs pass; the stale closing paragraph was
  replaced), rules 1, 3, 5 and the VS Code steps updated.

Files changed:

- New: `crates/wakeshot-windows/examples/agent-watch.rs`.
- `crates/wakeshot-core/src/sources/claude/mod.rs` (`claude_folder`, `in_folder`).
- `crates/wakeshot-core/tests/claude_status.rs` (one test).
- `docs/research/claude-code-signals.md` (new section at the end; rules 1, 3, 5; "Status file fields" file line;
  the "Phase 4 live shapes" closing paragraph; "Live tests still open").
- `docs/research.md` (new section "Live Claude end to end checks").
- This card.

Checks:

- `rustfmt --edition 2024 --check` on the three Rust files: clean.
- `cargo clippy --locked -p wakeshot-core -p wakeshot-windows --all-targets -- -D warnings` and without
  `--all-targets`, for `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`: clean (4 runs).
- `cargo test --locked -p wakeshot-core`: 577 + 11 + 25 passed, 2 ignored (old). `cargo test --locked -p
  wakeshot-windows`: 471 passed, 21 ignored (old). No failure in another card's file during the run.
- No process this card started is left (36 pids checked). `%LOCALAPPDATA%\Wakeshot` was not there before; the CLI's
  `--dry-run` made `dry-run\` (holds, one log); both removed, so the folder is as it was. The real data folder was
  never used.

Not tested, for `p5-haik-check` or later:

- VS Code UI check (deliverable 4): waits for Haik (about 3 minutes; steps in "Live tests still open", fixed to use a
  Bash ping since the PowerShell tool refuses `Start-Sleep`).
- A real key press of Ctrl+C or Ctrl+Break, and closing a classic conhost window, while `wakeshot run` waits (only
  the pseudo console path ran). `run` against a live tray waits for `p6-int-tray-mode`.
- Stop from the UI of an MCP or WebSocket Monitor (verify 2, minor 1): not cheap, needs an MCP Monitor. A Monitor end
  across a PC sleep (verify 2, N2): needs a sleep. Desktop app; `claude --continue`.

Decided by the worker, for Haik's review:

1. An empty `CLAUDE_CONFIG_DIR` counts as unset (what Claude Code does with one is not verified). Change it when a
   user reports sessions missed with an empty value.
2. The tray reads `CLAUDE_CONFIG_DIR` from its own environment only; a per-shell value is not followed. Change it
   when users ask for it (it would need a second folder per source).
3. A passed Monitor timer stays in the snapshot; the core drops it 2 minutes after its time, so it was not trimmed in
   the source. Change it when snapshots grow from many old Monitors.
4. The pseudo console runs stand in for Windows Terminal (it hosts the same ConPTY); the classic conhost window and a
   person's key press stay with `p5-haik-check`.
