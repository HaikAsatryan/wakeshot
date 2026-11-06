# Live test: Copilot CLI (with Haik)

Phase: 4. Status: done (2026-11-06, Round 3; back in v1, Haik 2026-11-06; see "Round 2" and "Round 3" at the
end; Round 3 wins where they differ). Needs Haik: no (Round 3: agents run every step). Agent: `wakeshot-worker`.
Size: about 130K tokens.
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

None (Round 3: the worker ran every step at Medium; Copilot CLI was installed and signed in through `gh`).

## Questions for Haik

None. The design calls are under "Decided by the worker" in the Result.

## Found outside scope

1. **The contract text is now behind the watch.** `crates/wakeshot-core/src/sources.rs`, `Liveness::AnyProcess` doc:
   "a process running `image` that holds `lock_file` open (Restart Manager), else any live process with `image`".
   The watch now also counts the holder whose pid the lock file's name gives, whatever its image, and when Restart
   Manager fails it counts only that pid for such a name. The type is unchanged. A contract file: the orchestrator's.
2. **Stale lines elsewhere.** `docs/research/copilot-vscode-signals.md`: "Which of the two files is held open:
   unverified" (it is the `.hold`), the "Process" bullet and rule 8 (now built and seen live).
   `docs/architecture.md` section 7 has a Codex live paragraph but none for Copilot (`.hold`, the pid in the name,
   VS Code's harness counted). `docs/tasks/p4-live-harness.md` runbook row "Copilot CLI": Esc does not stop a
   running command, one Ctrl+C does (it writes `abort`).
3. **A missing folder is seen only at the tick.** The first `copilot -p` beside the dry-run tray was held 26.7 s
   late, at the 30 s tick, because `session-state\` did not exist when the tray started (timing matches the tick;
   cause inferred). The second run, with the folder there, was held 0.25 s after its prompt. Any source whose folder
   appears after the tray starts (a first install) can start up to 30 s late. Not fixed: it needs the watch thread
   to watch the nearest existing parent of a missing folder (`watch/thread.rs`), a change beyond this card's fixes.
4. **VS Code's sessions run hooks.** The VS Code harness sessions had `hook.start` and `hook.end` lines; my CLI
   sessions had none. Whose hooks they are was not checked (VS Code's own, or ones another card installed).
5. **Haik's VS Code runs elevated** (from `p4-live-vscode`), so its `copilot-runtime.exe` is elevated. Restart Manager
   and the process list still answer from Medium (checked), so liveness works; the CIM image path is empty from Medium.

## Log

- 2026-11-03 16:53: card written by `p4-plan-sources`.
- 2026-11-06: Round 3 started by `wakeshot-worker`. Status in progress. Reading the card, the harness Result and
  the source code.
- 2026-11-06 15:45: before: Copilot CLI 1.0.106 at Medium both through VS Code's shim and npm's `copilot.cmd`;
  `COPILOT_HOME` unset, home `%USERPROFILE%\.copilot`. Running and not mine (never touched): Haik's VS Code (12
  `Code.exe` since 11:05) with its own `copilot-runtime.exe` (pid 2728, VS Code's `copilot-sdk` folder, since 11:05);
  Haik's installed tray (two `wakeshot-tray.exe`). Left from `p4-live-harness`: stale `inuse.9260.lock` +
  `inuse.9260.hold` (11-03) and `inuse.14796.lock` + `.hold` (its killed TUI); no holder for any (Restart Manager).
  Built `agent-watch` and `wakeshot-tray` (debug, x64) from the current tree into my target folder.
- 2026-11-06 15:51, TUI A through the shim (`cmd /d /c copilot --experimental`, driver at Medium; `agent-watch
  --source copilot --raw` and a change notice logger at Medium beside it). Start, before any prompt: the session
  folder, `inuse.52300.hold` (15:51:22.733) and `inuse.52300.lock` (22.734, 6 bytes); 52300 is the inner
  `copilot.exe` (npm's `copilot-win32-x64\copilot.exe`; tree shim `copilot.exe` 47868 > `cmd` > `node` > it). The
  start also deleted the stale `inuse.14796.*` of the harness's killed TUI. Restart Manager: the holder of
  `inuse.52300.hold` is 52300; `inuse.52300.lock` has **no holder**. So the source, which names the `.lock` file,
  posted the live session `alive=Gone` 66 ms after the lock appeared: **a live Copilot session never holds** (bug).
  No `events.jsonl` until the first prompt: then the buffered `session.start` (11:51:22) and the prompt's lines came
  in one write.
- 2026-11-06 15:53 to 15:55, prompt 1 (one shell command, `Start-Sleep -Seconds 60`): Copilot Fusion picked
  `gpt-6-luna` (Free plan). Posts: working 0.22 s after `user.message`; `tool.execution_start` came before
  `permission.requested`; waiting (since its time) 0.22 s after it; left 60 s, then "1. Yes" (not "don't ask
  again", which saves an approval): `permission.completed`, working 0.14 s later; nothing written during the 60 s
  run (working throughout); `tool.execution_complete`, then `assistant.turn_end`, idle 0.13 s after it. Prompts: 1
  (0.24 AI credits shown).
- 2026-11-06 15:55 to 15:58, prompt 2 (the `ask_user` tool): the form "Copilot needs information" showed; waiting
  posted 0.24 s after `tool.execution_start` (`ask_user`), since its time; left 2 min 40 s; Enter (A):
  `human_response.recorded`, `tool.execution_complete`, working 0.23 s, then the answer's `assistant.turn_end`, idle
  0.15 s after it. Prompts: 2.
- 2026-11-06 15:58 to 16:00: sessions I did not start appeared in the same `session-state`: `f3c45b7f`, `611ad5a1`,
  `ac748c63`, each with `inuse.2728.hold` + `inuse.2728.lock`; 2728 is Haik's VS Code's `copilot-runtime.exe`, and new
  `Code.exe` windows had started at 15:58:32 (most likely `p4-live-vscode`'s chat test). Never touched; only names
  listed, Restart Manager asked, and the watch's kinds-only raw log. Holder of `inuse.2728.hold`: 2728 itself
  (`copilot-runtime.exe`, "GitHub Copilot Runtime Host"); `inuse.2728.lock` no holder. `ac748c63`'s log followed
  the CLI rules: `user.message`, a 60 s tool with `permission.requested` and `permission.completed` 47 ms apart,
  then `assistant.turn_end`; the source read working, then idle 0.75 s after the turn end. Posted `alive=Gone`
  throughout (the `.lock` bug, and the holder's image is not `copilot.exe`). Their lines go into no fixture.
- 2026-11-06 16:02: Restart Manager asked from a Medium process (a script opened through `explorer.exe`): it names
  52300 for `inuse.52300.hold` and the elevated `copilot-runtime.exe` 2728 (with its start time) for the VS Code
  sessions' `.hold`; the CIM path of 2728 is empty from Medium. So the `alive=Gone` of my session (started at
  Medium, `token.txt`) is the `.lock` choice, not an elevation gap.
- 2026-11-06 16:00 to 16:01, prompt 3 (`Start-Sleep -Seconds 40`, approved at once: `permission.requested` and
  `.completed` 85 ms apart in one write, so no waiting post): Esc during the run did nothing in 6 s (the footer says
  "ctrl+c interrupt"); one Ctrl+C: `assistant.turn_end`, `abort` (`reason` `user_initiated`), idle 0.21 s later; the
  TUI stayed. Prompts: 3.
- 2026-11-06 16:01 to 16:04, `/every 2m <one-word prompt>` (an experimental command: needs `--experimental`):
  "Scheduled #1 every 2m", no run at creation; `session.schedule_created` (`id` 1, `intervalMs` 120000,
  `recurring` true, no `at` or `cron`); timer posted 16:03:41 0.24 s later. The fire at 12:03:41.499 UTC (2 min
  0.118 s after creation): `user.message` with `source` `schedule-1`, so the id is the rest after `schedule-`;
  working, then the timer moved to 16:05:41, idle 0.15 s after its turn end. `/every` with no text opened "Scheduled
  prompts"; `x` deleted it: `session.schedule_cancelled` (`id` 1), the timer dropped 0.23 s later. Prompts: 4.
- 2026-11-06 16:04, `/exit`: `session.shutdown` (`shutdownType` routine, `totalPremiumRequests` 4), then
  `inuse.52300.hold` and `.lock` deleted in the same millisecond, `n` down by one 0.1 s later; the shim's tree ended
  with exit 0. Every pid of the driver's tree gone (checked by pid).
- 2026-11-06 16:05 to 16:15, fix 1 (`sources/copilot.rs`): in-use names are `inuse.*.lock` and `inuse.*.hold`; a
  session's liveness names its `.hold` (a `.lock` only while no `.hold` is listed). Fix 2 (`watch/agents.rs`): a lock
  file whose name gives a pid (`named_pid`: a whole-number part of the name) counts the Restart Manager holder with
  that pid whatever its image, so `copilot-runtime.exe` (rule 8 of `copilot-vscode-signals.md`) counts; when
  Restart Manager fails, only the process the name gives counts (not the shim or another session's
  `copilot.exe`), and it goes to the exit waits; a name with no pid (Codex) keeps the old fallback. With both fixes
  switched off, the new tests `liveness_names_the_held_file_while_one_is_listed`,
  `lock_names_are_filtered_before_any_open` (core) and `a_holder_whose_pid_the_lock_name_gives_counts_whatever_its_
  image`, `a_restart_manager_error_counts_only_the_process_the_lock_name_gives` (watch) fail; on, they pass.
  Fixture `live/tui-a.jsonl` (session A, redacted) and its replay test.
- 2026-11-06 16:12: old watch stopped (Ctrl+C, `stopped clean=true`, all its pids gone); the fixed `agent-watch`
  started at Medium: VS Code's harness sessions (holder 2728, elevated) now `alive=Yes`; the stale 11-03 session
  `Gone`.
- 2026-11-06 16:13, TUI B through the shim with `--allow-tool "shell(Start-Sleep)"`: in-use files `inuse.8588.*`
  (8588 the inner `copilot.exe`, shim 53532); posted idle `alive=Yes` 0.3 s after the files. Prompt 5
  (`Start-Sleep -Seconds 60`, no approval asked): working. At 16:13:50.042 the driver killed 8588 by pid: working
  `Gone` posted 61 ms later; the shim's `cmd.exe` ended 66 ms after the kill (exit 1), the whole tree with it. Both
  in-use files stayed, with no holder. Every pid of that tree gone (one pid, 54388, came back as an unrelated
  `bash.exe` started 16:13:57: reused). Prompts: 5.
- 2026-11-06 16:15 to 16:20: fixture `live/tui-crash-mid-turn.jsonl` and its test; a real-file watch test
  `copilot_from_a_held_in_use_file_and_its_events` (this process holds `inuse.<its pid>.hold`, its image is not
  `copilot.exe`, real listing, tail and Restart Manager). `rustfmt` on my files; quick tests pass (core
  `copilot_source` 47, watch `agents` 45). The `--dry-run` lock is held by `p4-live-vscode` since 16:10:44; waiting
  for my turn.
- 2026-11-06 16:19 to 16:23: all checks (below) clean while waiting.
- 2026-11-06 16:29: dry-run lock taken (its tray, pid 53708, had quit). The dry-run data folder held only `logs\`
  from earlier cards. Fixed tray started `--dry-run` at Medium through `explorer.exe`, with `COPILOT_HOME` on an
  empty scratch folder and the other agents' homes on empty folders; it logged "Copilot CLI is watched in the folder
  COPILOT_HOME names". Prompt 6: `copilot -p` (30 s command, allowed) through npm's `copilot.cmd` (no shim; tree
  `cmd` > `node` > `copilot.exe` > `pwsh`): Copilot made `session-state\` in that folder and the `gh` login worked;
  "Holding starts: Copilot" with `dry run: PowerSetRequest SYSTEM` came 26.7 s after `user.message`, at the tray's
  30 s tick (the folder did not exist at the tray's start); "Holding ends; held by Copilot" 0.19 s after the last
  `assistant.turn_end`. Prompt 7 (15 s command, the folder now there): hold started 0.25 s after `user.message`,
  ended 0.24 s after the last turn end. Tray quit by `taskkill /PID 55224` (no force), "ends with code 0"; the
  `config.json` and `state.json` it made in the dry-run folder deleted; lock file deleted. Haik's trays (11424,
  26236) still run. Prompts: 7.
- 2026-11-06 16:32 to 16:35: fixture `live/oneshot-npm.jsonl` (prompt 6) and its test; all three fixtures redacted
  in one run (one id map), secret patterns and the assigned-key pattern: no hit; no prompt word, user name or path.
  Fixed watch stopped (`stopped clean=true`), logger stopped by pid; no process with my folder in its command line
  is left. Checks run again: clean. Build folder, the scratch Copilot home and my file backups deleted.
- 2026-11-06 16:38: one doc comment in `copilot.rs` reworded (the `.lock`'s content was never read, so it no longer
  says "the pid as text"); `rustfmt --check` clean. Status done.

## Result

Done, 2026-11-06 (Round 3; no Haik step). Copilot CLI 1.0.106 (npm), Copilot Free through `gh`'s login, Copilot
Fusion (it picked `gpt-6-luna`), **7 one-line prompts** (one a schedule fire; `totalPremiumRequests` agrees).

**What held live** (all at Medium; detail in `docs/research/other-agent-signals.md`, "GitHub Copilot CLI", "Phase 4
live test (2026-11-06)"):

| Step | Outcome |
|---|---|
| TUI start (through VS Code's shim) | before any prompt: `inuse.<pid>.hold` and `inuse.<pid>.lock`, pid = the inner `copilot.exe` (shim > `cmd` > `node` > it); Restart Manager: that process holds the `.hold`, **nobody holds the `.lock`** |
| Working turn (60 s command) | working 0.22 s after `user.message`; nothing written during the run; idle 0.13 s after the last `assistant.turn_end` |
| Approval left 60 s, then Yes | `tool.execution_start` comes before `permission.requested`; waiting 0.22 s after it; working 0.14 s after `permission.completed` |
| `ask_user` left 2 min 40 s | waiting 0.24 s after its `tool.execution_start`; answer: `human_response.recorded`, working, idle |
| Esc during a command | does nothing (footer: "ctrl+c interrupt"); one Ctrl+C: `assistant.turn_end`, `abort` (`user_initiated`), idle 0.21 s later |
| `/every 2m` (needs `--experimental`) | `schedule_created` (`id` 1, `intervalMs` 120000, `recurring`); no run at creation; timer posted at once; the fire is `source` `schedule-1` (the id follows `schedule-`); timer moved at the fire; `x` in `/every`: `schedule_cancelled`, timer dropped |
| `/exit` | `session.shutdown`, both in-use files deleted in one millisecond, session gone 0.1 s later |
| Crash (inner `copilot.exe` killed by pid mid-command) | working `Gone` 61 ms later (after the fix); the shim's tree ended within 66 ms; both files stay with no holder, until the next Copilot start deletes them |
| Write timing | no `events.jsonl` until the first prompt, then `session.start` and the prompt in one write; after that each event at once (read 0.13 to 0.25 s after its time), each with a change notice |
| npm path, no shim (`copilot.cmd -p`) | same files and events; `COPILOT_HOME` on an empty folder moved the whole home, and the login worked |
| Through the app (dry-run tray, fixed build) | "Holding starts: Copilot" + `PowerSetRequest SYSTEM` 0.25 s after `user.message`, "Holding ends" 0.24 s after the last turn end; the first run 26.7 s late at the tick, since `session-state\` did not exist yet (Found outside scope 3) |
| VS Code's Copilot harness (another card's chats, never touched) | same `session-state`, `inuse.2728.*`, holder 2728 = VS Code's elevated `copilot-runtime.exe`, named by Restart Manager from Medium; same event rules; after the fix `alive=Yes` |

The `alive=Gone` of my Medium session (the orchestrator's question): not elevation. My TUIs ran at Medium
(`token.txt`), and Restart Manager from Medium names both my `copilot.exe` and the elevated runtime. The cause was the
source naming the `.lock`, which no process holds.

**Fixes** (each with tests that fail with the fix switched off and pass with it on):

1. `sources/copilot.rs`: in-use names are `inuse.*.lock` and `inuse.*.hold`; a session's liveness names its `.hold`
   (a `.lock` only while no `.hold` is listed). Before, every live Copilot session read `Gone` and never held. Tests:
   `liveness_names_the_held_file_while_one_is_listed`, `lock_names_are_filtered_before_any_open` (updated).
2. `watch/agents.rs` (the `AnyProcess` fallback and holder rule): a lock file whose name gives a pid (`named_pid`: a
   whole-number part, Copilot's `inuse.<pid>.hold`) counts the Restart Manager holder with that pid whatever its image,
   so `copilot-runtime.exe` (research rule 8) counts; a scanner or another process still does not. When Restart
   Manager fails, only that pid counts, not the shim or another session's `copilot.exe`, and it goes to the exit
   waits; a name with no pid (Codex) keeps the old fallback; its log line says which. Tests:
   `a_holder_whose_pid_the_lock_name_gives_counts_whatever_its_image`,
   `a_restart_manager_error_counts_only_the_process_the_lock_name_gives`, `a_lock_name_gives_a_pid_only_as_a_whole_
   number_part`, and the real-file `copilot_from_a_held_in_use_file_and_its_events` (real listing, tail and Restart
   Manager; this process holds `inuse.<its pid>.hold` and is not `copilot.exe`). `lockowner.rs`: no change needed.

**Fixtures and replay**: `crates/wakeshot-core/tests/fixtures/copilot/live/` (`tui-a.jsonl`,
`tui-crash-mid-turn.jsonl`, `oneshot-npm.jsonl`, `README.md` with the redaction). Tests
`every_live_fixture_is_listed_once`, `the_live_tui_session_moves_at_its_turn_events` (every change point, line by
line, with the waiting times and the schedule timers), `the_live_crash_ends_inside_its_turn`,
`the_live_one_shot_works_until_its_last_turn_ends`, `live_logs_give_the_same_facts_in_any_pieces`. Every real line
reads with no format problem. Paths in the new core tests are built with `Path::join` (no `C:\` literal), for the
Linux CI.

**Tray wiring**: `p4-contracts-r3`'s Copilot line in `tray_mode.rs` and the `agent-watch` row work as built (tested
above); no wiring bug.

**Files changed**: `crates/wakeshot-core/src/sources/copilot.rs`; `crates/wakeshot-core/tests/copilot_source.rs`;
`crates/wakeshot-core/tests/fixtures/copilot/live/` (new: three `.jsonl`, `README.md`);
`crates/wakeshot-windows/src/watch/agents.rs`; `docs/research/other-agent-signals.md` (new section);
`docs/research.md` (new last section "Copilot CLI live test"); this card.

**Checks** (own target folder, `--locked`, no `RUSTFLAGS`; last run 16:33): `rustfmt --edition 2024 --check` on
`copilot.rs`, `copilot_source.rs`, `agents.rs`, `lockowner.rs`: clean. `cargo clippy --locked -p wakeshot-core -p
wakeshot-windows` with and without `--all-targets`, `-D warnings`, x64 and ARM64 (Build Tools `Llvm\x64\bin` on that
command's PATH only): all four clean. `cargo test --locked -p wakeshot-core -p wakeshot-windows --no-fail-fast`: core
730 unit and every integration file passed (`copilot_source` 48); `wakeshot-windows` lib 695 passed, 0 failed, 28
ignored. Fixtures: the repo's secret patterns and the assigned-key pattern give no hit; no prompt word, user name,
path or account in them.

**Clean-up**: every pid the drivers, the watches, the logger, the one-shots and the dry-run tray started is gone
(checked by pid); Haik's installed tray untouched and running; dry-run lock released. Build folder, scratch Copilot
home and backups deleted. Left in Haik's Copilot home: my two TUI sessions of today (`5a3cc26e`, `1c554b72`; the
crashed one keeps its `inuse.8588.*` until the next Copilot start) and Copilot's own index rows, as the earlier cards
did, so the app's index stays consistent.

**Not tested live**: rotation (`session.journal_link`), sub-agents, `/after`, cron and self-paced schedules,
`--resume`, VS Code Background sessions (`Code.exe` holder), two clients on one session, `/keep-alive`.

**Decided by the worker** (for Haik's review), each with when to change it:

1. The pid in a lock file's name names its holder, a generic rule in the watch, instead of a contract change that
   gives `AnyProcess` a list of images. It counts VS Code's runtime and any other host Copilot uses. Change it when an
   agent puts a number in a lock name that is not the holder's pid, or the contracts next change (then a list of
   images, or a `pid` field, is cleaner).
2. When Restart Manager fails, the pid the name gives counts whatever its image; a reused pid could then keep a
   crashed session alive. It needs Restart Manager to fail and a crash at once; the old rule (any `copilot.exe`) was
   looser. Change it when the log shows a Restart Manager failure.
3. VS Code's Copilot harness chats now hold through the Copilot CLI source and show as "Copilot" in the tooltip, not
   "Copilot in VS Code". Change it when the two should read apart (the source would need the holder's image).
4. Two clients on one session: the first `.hold` by name stands for both, so if that client crashes while the other
   lives, the session reads `Gone`. Change it when a live case shows two clients (`alreadyInUse` in `session.start`).

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

## Round 3: agents run it, no Haik (2026-11-06)

Written by the orchestrator. This replaces "Needs Haik" and every "Haik types" step. Order: after `p4-live-codex`.

- `p4-live-harness` installed Copilot CLI 1.0.106 for Haik's user (npm); it uses his `gh` login on Copilot Free
  (`copilot -p` answered, no token passed). Read its Result: the runbook, the driver `spikes/agent-drive/`, and
  `agent-watch.rs --source copilot`.
- You run every session step yourself with the driver, at Medium through `explorer.exe`: a working turn (a command
  that runs about 60 s), a permission prompt left 1 minute then approved, an `ask_user` question, Esc during a turn, a
  schedule if cheap, a clean exit, and a crash: kill by PID a `copilot.exe` that you started. `copilot -p` for the
  cheap checks, the TUI where a step needs it.
- VS Code's `copilot` shim comes first on PATH and stays as a parent: shim `copilot.exe` > `cmd.exe` > `node.exe` >
  `copilot.exe` (the CLI), and the in-use lock is named after the inner `copilot.exe` pid (`p4-live-harness`). Fix
  the `AnyProcess` image fallback to match the lock holder or the full path, not the image name alone, with a test.
  Also run the CLI by its npm path once, without the shim.
- VS Code's Copilot harness and Background sessions (`copilot-runtime.exe`, `Code.exe` owners; `inuse.<pid>.hold`):
  check them only if one starts from the command line; else mark "not tested live" (the `p4-live-vscode` card tests
  VS Code's own chat files).
- The "Rules for every live card of the bigger v1" in `docs/tasks/p4-live-vscode.md` apply (minimum tokens; Copilot
  Free has a monthly limit, so count prompts).
- Owns this round: the Copilot line in `tray_mode.rs` and `agent-watch.rs`, and the files in this card's
  Deliverables.
