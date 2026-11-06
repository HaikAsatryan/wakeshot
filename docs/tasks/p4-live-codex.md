# Live test: Codex CLI (with Haik)

Phase: 4. Status: done (2026-11-06, Round 3; back in v1, Haik 2026-11-06; see "Round 2" and "Round 3" at the end;
Round 3 wins where they differ). Needs Haik: no (Round 3: agents run every step). Agent:
`wakeshot-worker`. Size: about 130K tokens.
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

None (Round 3: the worker ran every step at Medium with the driver; Codex was already installed and signed in).

## Questions for Haik

None.

## Found outside scope

1. **A command can outlive its Codex turn.** After Esc, the `Start-Sleep` started through unified exec went on as a
   "background terminal" (`/ps`) while the rollout said `turn_aborted`, so Wakeshot read idle; its `item_completed`
   came 53 s later. Whether a model can also end a normal turn with a background terminal still running (a dev
   server, a long build) was not checked. Not fixed: it needs a new signal, and the case is bounded by the command.
2. **Stale lines elsewhere.** `docs/architecture.md` section 7's Codex paragraph still says the Restart Manager tie is
   "not verified for this lock" and the rule is marked "can outlive its session" (I added the live result as a new
   paragraph after it; shared docs take additions only). `docs/phases/04-sources.md` track note 7 can now say the
   live test kept Restart Manager. The orchestrator's files.
3. **The Codex TUI's first-start daemon.** `p4-live-harness` saw two extra `codex.exe` at its first TUI start (and
   `~\.codex\app-server-daemon\` dates from it); none of my four Codex starts made one. If a daemon ever holds a
   thread's lock, the holder is the daemon and the session lives while it does. Not tested live.

## Log

- 2026-11-03 16:53: card written by `p4-plan-sources`.
- 2026-11-06: worker started Round 3 (agents run every step); Status in progress.
- 2026-11-06 14:22: before: Codex CLI 0.167.0 (npm, `%APPDATA%\npm\codex.ps1`), `CODEX_HOME` unset, home
  `%USERPROFILE%\.codex`. The desktop app runs (`ChatGPT.exe` of the `OpenAI.Codex` package, since 13:19) with two
  `codex.exe` children of its own. One stale lock from `p4-live-harness` (`01a1b053`, no holder). Models (cache):
  `gpt-6-luna` and `gpt-5.6-luna` the small ones; lowest effort `low`. Flags: `-a` takes only `on-request` and
  `never` in 0.167.0; features `unified_exec`, `sleep_tool`, `daemon_auto_start`, `goals` on, `prevent_idle_sleep`
  off. Every session at Medium through the driver; `agent-watch --source codex` (own build) at Medium beside it.
- 2026-11-06 14:26 to 14:38, TUI 1 (`-m gpt-6-luna -c model_reasoning_effort=low -s read-only -a on-request`):
  start deleted the stale lock at once and made the new thread's lock (no prompt yet); Restart Manager named the
  TUI's own `codex.exe` (pid and start equal; no daemon started); posted idle `Yes` 0.2 s after the lock. Working
  turn (`Start-Sleep -Seconds 60`): working and idle both posted only at the 30 s tick (23 s and 29 s after the
  rollout lines). Approval wait 60 s then approved: working through the wait (nothing written while it waits).
  Esc during a turn: `turn_aborted` written; that 12 s turn was never posted (start and end within one tick); the
  sleep went on as a "background terminal". Backtrack (Esc Esc, left, Enter, edit): Codex dropped and remade the
  lock twice in 0.8 s and wrote `..._<rollout id>.jsonl`; the source followed the new file (its turn fed). `/exit`:
  lock deleted, `n=0` posted 1 ms later. A change notice logger showed each rollout write gives a notice within
  milliseconds, but the source does not watch `sessions`, so writes wait for the tick.
- 2026-11-06 14:38, TUI 2, crash: lock holder the TUI's `codex.exe` (48448, start equal); killed by pid during a
  turn: posted working `Gone` 115 ms later (no hold); the lock file stays behind with no holder. Prompts so far: 6.
  All driver pids gone (checked by pid).
- 2026-11-06 14:45: fixtures: the three rollouts redacted by a script into `tests/fixtures/codex/live/` (82, 12, 15
  lines; secret patterns: no hit; no user name, prompt or plan left). Replay tests added in `codex_source.rs`.
- 2026-11-06 14:55: fix for the tick delay in `sources/codex.rs`: while a thread is listed, a `Names` want on
  `sessions` (one level, a filter that takes no name) gets the tree watched; the contract already reads a source's
  tails on each change in one of its folders, so no contract change. Tests: `a_codex_rollout_write_is_read_on_its_
  notice_without_a_tick` (agents.rs) and `the_sessions_tree_is_wanted_...` (core); with the want switched off
  both fail (and two old want tests), with it on all pass. Tray: `tray_mode.rs` starts Codex beside Claude Code
  (`CODEX_HOME`, else `.codex`), test `the_tray_watches_claude_code_and_codex_cli_where_each_writes`.
- 2026-11-06 14:50 to 14:53, after the fix: rebuilt `agent-watch` and `wakeshot-tray`; both at Medium, the tray as
  `--dry-run` with `CLAUDE_CONFIG_DIR` on an empty scratch folder (so only Codex can hold). One `codex exec` with
  the 60 s sleep (prompt 7): exec's start deleted TUI 2's stale lock; holder exec's own `codex.exe` (start equal);
  `task_started` to the watch's working post 0.3 s, to the tray's "Holding starts: Codex" with `dry run:
  PowerSetRequest SYSTEM` 0.5 s; `task_complete` to the idle post and "Holding ends; held by Codex" 17 ms; lock gone
  at exit, `n=0` at once. Dry-run tray quit by `taskkill /PID` (no force), code 0; its `dry-run` folder (made by
  this card) removed; Haik's tray untouched. Notice logger stopped by pid. Fixture `exec-one-turn.jsonl` added.
- 2026-11-06 14:55 to 15:00, checks: first clippy `--all-targets` run failed on my new test's tuple type
  (`type_complexity`); fixed with a type alias. Then fmt, the four clippy runs and `cargo test -p wakeshot-core`
  clean; `wakeshot-windows` lib 654 passed, 1 failed: `sqlite::tests::an_agent_in_another_process_that_closes_
  takes_its_files_and_a_read_then_makes_none` (`sqlite.rs`, owned by `p4-opencode-source`, running now; it passed
  in my 14:56 run). Waiting, then once more.
- 2026-11-06 15:02: again: three other `sqlite.rs` tests failed (that card is changing the file); 15:03 with
  `--no-fail-fast`: `wakeshot-windows` lib 655 passed, 0 failed, 25 ignored. So the failures were that card's work in
  progress, not this card's. Build folder deleted; Status done.

## Result

Done, 2026-11-06 (Round 3; no Haik step). Codex CLI 0.167.0, `gpt-6-luna` at `low`, 7 one-line prompts.

**What held live** (all at Medium; detail in `docs/research/other-agent-signals.md`, "Phase 4 live test
(2026-11-06)"):

| Step | Outcome |
|---|---|
| TUI start | lock file made before any prompt; holder the TUI's own `codex.exe` (pid and start equal); idle `Yes` posted 0.2 s after the lock; the stale lock of an earlier crash deleted at start |
| Working turn (60 s sleep) | working, then idle; before the fix both only at the 30 s tick (23 s and 29 s late) |
| Approval wait 60 s, then yes | working all through (Codex writes nothing while it waits) |
| Esc | `turn_aborted` (`interrupted`); the 12 s turn was never posted before the fix; the sleep went on as a background terminal (Found outside scope 1) |
| Backtrack (Esc Esc, left, Enter, edit) | `..._<rollout id>.jsonl` written; lock deleted and made again twice; the source ended on the new file and fed its turn (`FindNewest` from `p4-contracts-r2` works) |
| `/exit` | lock deleted, `n=0` posted 1 ms later |
| Crash (TUI `codex.exe` killed by pid mid-turn) | working `Gone` posted 115 ms later, so no hold; the lock file stays with no holder until the next Codex start deletes it (seen for a TUI and an `exec` start) |
| `codex exec`, after the fix | holder exec's own `codex.exe`; working posted 0.3 s and idle 17 ms after their lines |
| Through the app (`--dry-run` tray) | "Holding starts: Codex" with `dry run: PowerSetRequest SYSTEM` 0.5 s after `task_started`; "Holding ends; held by Codex" 17 ms after `task_complete` |

Track note 7: Restart Manager sees the holder, so the "can outlive its session" mark goes for the Codex rule; it
stays only for the fallback when Restart Manager fails (any `codex.exe`; the desktop app keeps two running).
Not tested live: the app-server daemon as a holder (none started), the desktop app's own threads (it cannot be driven
from the command line), goals, `--oss`.

**Fix** (one): rollout writes waited for the 30 s tick, because the source watched only the lock folder, while each
append to the open rollout gives a change notice within milliseconds (logged live). `sources/codex.rs` now also wants
`sessions\` as a `Names` listing one level deep, with a filter that takes no name, while a thread is listed: the
watch thread then watches the tree, and the contract already reads a source's tails on every change in one of its
folders. No contract, `Cargo` or `read.rs` change. Tests that failed before: `watch::agents::tests::
a_codex_rollout_write_is_read_on_its_notice_without_a_tick` and `the_sessions_tree_is_wanted_for_its_watch_only_
while_a_thread_is_listed` (with the want switched off both fail; three old want tests updated for the new want).

**Fixtures and replay**: `crates/wakeshot-core/tests/fixtures/codex/live/` (`tui-four-turns`, `tui-reverted`,
`tui-crash-mid-turn`, `exec-one-turn`, and a README with the redaction). Tests `every_live_fixture_is_listed_once`,
`live_rollouts_move_the_activity_only_at_their_turn_events` (line by line and in pieces; every real line reads with
no format problem), `the_live_backtrack_ends_on_the_reverted_rollout` (the watch thread's live feed order).

**Tray wiring**: `app/tray_mode.rs` starts `CodexSource` beside Claude Code, in `CODEX_HOME` (empty counts as unset)
else `.codex` of the profile folder, with one log line saying which; test
`the_tray_watches_claude_code_and_codex_cli_where_each_writes`. Tested through the app as above.

**Files changed**: `crates/wakeshot-core/src/sources/codex.rs`; `crates/wakeshot-core/tests/codex_source.rs`;
`crates/wakeshot-core/tests/fixtures/codex/live/` (new: four `.jsonl`, `README.md`);
`crates/wakeshot-windows/src/watch/agents.rs` (one test); `crates/wakeshot-windows/src/app/tray_mode.rs`;
`docs/research/other-agent-signals.md` (new section); `docs/research.md` (new last section); `docs/architecture.md`
(a paragraph after section 7's Codex paragraph); this card. `examples/agent-watch.rs`: no change (Part B had the row).

**Checks** (own target folder, `--locked`, no `RUSTFLAGS`): `rustfmt --edition 2024 --check` on the four Rust files:
clean. `cargo clippy --locked -p wakeshot-core -p wakeshot-windows` with and without `--all-targets`, `-D warnings`,
x64 and ARM64: clean. `cargo test --locked -p wakeshot-core`: 707 unit, 11, 25, 22 (`codex_source`), 42, 14, 31
passed. `cargo test --locked -p wakeshot-windows`: lib 655 passed, 0 failed, 25 ignored (15:03; two earlier runs
had one and three failures in `sqlite.rs`, which `p4-opencode-source` was changing at the time, Log). Fixtures: the repo's secret patterns and the assigned-key pattern give no hit; no user name, prompt
text, account id or plan in them.

**Clean-up**: every pid the drivers, the watch, the tray and the logger started is gone (checked by pid); the
throwaway folder, the dry-run data folder (made by this card) and the build folder deleted. Left in Haik's Codex home:
four test rollouts of today in `sessions\2026\11\06` and Codex's own index rows for them (left so the app's indexes
stay consistent); his install and login untouched.

**Decided by the worker** (for Haik's review), each with when to change it:

1. The `sessions` listing that only gets the tree watched, instead of a contract change that lets a `FindNewest` root
   be watched. Change it when the contracts next change: watch `FindNewest` roots directly and drop the listing.
2. Each notice batch in `sessions\` now runs the `FindNewest` walk of all date folders (about 30 walks in a 70 s turn
   here, on a one-day home). Change it when a large Codex home makes a walk slow: list only the newest date folder
   between full walks (`p4-contracts-r2` choice 1, `watch/read.rs`).
3. A background terminal left after Esc is not counted (Found outside scope 1). Change it when a live case shows
   real work running on after a turn ends.
4. The tray reads `CODEX_HOME` from its own environment only, as it does `CLAUDE_CONFIG_DIR`. Change it when users ask
   for per-shell homes.

## Round 2: back in v1 (2026-11-06)

Written by the orchestrator. Haik, 2026-11-06: Codex CLI and Copilot CLI ship in v1 after all. Changes to this card:

- Order: after `p4-live-copilot` (swapped 2026-11-06).
- This card also wires the source into the app: add it to the sources the tray starts in
  `crates/wakeshot-windows/src/app/tray_mode.rs`, so the installed app holds for it, and test that through the app.
- Haik's answers to sign-off items 1 and 2 (`docs/phases/02-06-signoff.md`) apply.
- Access: Haik is not sure he can use Codex. OpenAI's pricing page (read 2026-11-06) lists the CLI for Plus, Pro, Business, Enterprise and Edu, and for an API key at API rates; Free and Go are not clear. Starts when Haik says he has access.

## Round 3: agents run it, no Haik (2026-11-06)

Written by the orchestrator. This replaces "Needs Haik", deliverable 1's "Set Status blocked and stop" and every
"Haik types" step. Status: ready once `p4-live-harness` Part B is done.

- Haik, about 15:30: Codex is installed and logged in. `p4-live-harness` installed Codex CLI 0.167.0 for his user
  (npm); it uses the desktop app's login (`codex exec` answered). Read its Result: the runbook, the driver
  `spikes/agent-drive/` (ConPTY, step files, started at Medium through `explorer.exe`), and `agent-watch.rs
  --source codex`.
- You run every session step yourself with the driver, at Medium: a working turn (a command that runs about 60 s),
  an approval wait left 1 minute then approved, Esc during a turn, a TUI backtrack (Esc Esc, edit the earlier
  prompt) so Codex writes a reverted thread file (`..._<rollout id>.jsonl`; `WantedFile::FindNewest` from
  `p4-contracts-r2` must follow it), a clean exit, and a crash: kill by PID a `codex.exe` that you started. `codex
  exec` for the cheap checks, the TUI where a step needs it.
- Minimum tokens (Haik): each prompt says it is an app test and asks for one word or `0`, no reasoning; use the
  lowest reasoning setting and the smallest model the login allows. Count the prompts in the Log.
- The desktop app shares `~/.codex` (config and login) and turns on its MCP servers (`node_repl.exe` may show in the
  CLI's tree). Never open `auth.json` or `.sandbox-secrets`. Whether the desktop app's own threads write the same
  lock and rollout files: check only if the app can be started and driven from the command line; else mark it "not
  tested live".
- Time it: a folder listing showed a stale size and time for a file another process held open (`p4-live-harness`,
  Found outside scope). Record how long a rollout write takes to reach a post, with the file open.
- Wire the source into `crates/wakeshot-windows/src/app/tray_mode.rs` (Round 2), and test it through the app with
  `--dry-run` (never beside another dry-run card; never quit Haik's installed tray).
- Owns this round: `tray_mode.rs` (the Codex line), `examples/agent-watch.rs` (the Codex line, if Part B left one to
  add), and the files in Deliverables 3 and 4. Rules as above: never run git, kill only your own PIDs.
