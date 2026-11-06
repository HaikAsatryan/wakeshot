# Live test: Antigravity CLI and IDE through Wakeshot's hooks

Phase: 4. Status: blocked for the IDE turn only (Haik's first-run pass, Result "Open items" 1); the rest done,
2026-11-06. Needs Haik: no (he signed in to the Antigravity CLI and IDE, 2026-11-06 ~17:30); only if the payload
check finds the command line.
Agent: `wakeshot-worker`. Size: about 120K tokens.
Runs after: `p4-antigravity-source`, `p4-live-cursor`, `p4-live-harness`; one live card at a time (agent-session
lane).
Parallel with: build cards that own none of the files below; at most four Rust cards at once.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p4-live-antigravity\target`; delete it when done.
Real laptop: Antigravity CLI 1.5.2 and IDE 2.6.0 for Haik's user, Free plan. Writes Wakeshot's real plugin folder
under `%USERPROFILE%\.gemini\config\plugins\wakeshot\` through the product's own install code. No power settings.

## Goal

The live test of the Antigravity source and its hooks, as `docs/agent-hooks.md` "Live test" item 7 and
`docs/research/antigravity-signals.md` list it, redacted fixtures, fixes with tests, and the source wired into the
tray.

## Read first

- `docs/agent-hooks.md` ("Antigravity", "Live test", the privacy Gate), `docs/research/antigravity-signals.md`,
  `docs/tasks/p4-antigravity-source.md` and `docs/tasks/p4-live-cursor.md` Results, `docs/tasks/p4-live-harness.md`
  Result (runbook, driver; the IDE starts `agy.exe --hub`).

## Deliverables

1. The payload form first, at each hook run; command line: remove the plugin, stop, ask Haik (default: no plugin).
2. The plugin added by the product's own install code; then in the CLI and the IDE: the folder loads, the shell and
   the hook's parent, the cost per `PreInvocation`, a working turn, a turn that waits for approval, Esc (does `Stop`
   fire?), `fullyIdle` with a background command such as a dev server, a clean exit, a crash (kill by PID an
   `agy.exe` you started; never the IDE's if you did not start it), `{}` lets a stop through.
3. Each timed from the event to the post, with `agent-watch` and a new `--source antigravity` row.
4. Fixtures into `crates/wakeshot-core/tests/fixtures/antigravity/live/` (redacted), a replay test, fixes each with
   a test that failed before.
5. Wire the source into `crates/wakeshot-windows/src/app/tray_mode.rs` and test it through the app with `--dry-run`.
6. A section "Phase 4 live test (2026-11-..)" in `docs/research/antigravity-signals.md` and short lines in
   `docs/research.md`; anything not run stays "not tested live".
7. Clean-up: the plugin folder removed through the product's remove path (the fresh install adds it again), every
   process you started ended by PID.

## Orchestrator changes (2026-11-06 ~18:50, for speed; Haik asked for more parallel work)

- Runs after `p4-antigravity-source`, in parallel with `p4-live-cursor` and the hooks code review. Fixes in shared
  hooks files (`sources/hooked.rs`, `hook.rs`, `agent_hooks.rs`) belong to `p4-live-cursor` this round: report yours
  to the orchestrator; fix only `sources/antigravity.rs`, its tests and fixtures.
- Before the first write: copy `%USERPROFILE%\.gemini\config\plugins\` whole (if it exists) to
  `<scratchpad>\p4-live-antigravity\backup\`; nothing else under `.gemini` (token files live there). At the end
  compare.
- If the hooks code review reports a blocker while you run, the orchestrator tells you; stop writing real folders.

## Rules

The "Rules for every live card of the bigger v1" in `docs/tasks/p4-live-vscode.md` apply. Also: never edit Haik's
own Antigravity or Gemini settings files.

## Owns

`crates/wakeshot-core/tests/fixtures/antigravity/live/`; fixes in `sources/antigravity.rs`, `sources/hooked.rs`,
`wakeshot-cli/src/hook.rs`, `agent_hooks.rs` and their tests; the Antigravity line in `tray_mode.rs` and in
`agent-watch.rs`; the sections above.

## Questions for Haik

Not blocking; defaults taken, details in Result, "For Haik (product)":

1. After Esc, Antigravity holds until the next prompt ends or `agy` exits. Default: keep it so (no time cap).
2. A background command holds only when `agy` sends `fullyIdle: false` (2 of 4 runs). Default: accept for v1.

## Found outside scope

1. **`.wakeshot-removed` does not stop Antigravity's hooks** (`agent_hooks.rs`, owner `p4-live-cursor` this round;
   sent to the orchestrator 17:46). `agy` 1.5.2 loads every folder in `plugins\`, dot names too; a folder renamed
   `.wakeshot-removed` ran both hooks in a real turn, and `.wakeshot-new-<pid>` is listed as an enabled plugin. So
   when the uninstall cannot empty the folder, the hooks keep running and call a deleted exe (fail open, one `cmd`
   start per model call). Suggestion: for Antigravity move the folder out of `plugins\`, or delete Wakeshot's
   `hooks.json` first. `docs/agent-hooks.md` "Removing" and the README line say the rename stops the plugin.
2. **`at_start` on a real profile rewrites another build's hook folder** (sent 17:42). Its "ours, unchanged, older
   content" row rewrites any Wakeshot folder whose files name another exe, for every agent in one run. Harmless for
   users (one install), but two live cards or a dev tool calling it on the real profile overwrite each other. I ran it
   only on a scratch profile.
3. **`agent-watch` misses a hooked folder made after its start until the next tick**, when its `--data` root is not the
   hook's data folder (the watch thread retries missing folders on a root change). The tray is fine (its root is the
   data folder: seen live). Change it when a live card needs the first turn in `agent-watch`: point `--data` at the
   hook's data folder.
4. **Two installed trays still run** (`wakeshot-tray.exe` 11424 from 04:34 and 26236 from 15:07, both from
   `current`), as `p4-live-vscode` noted. Not touched.

## Log

- 2026-11-06: card written by the orchestrator.
- 2026-11-06: started by a `wakeshot-worker`. Rule 1 of `p4-live-vscode.md` wins over deliverables 3 and 5: no edit
  to `tray_mode.rs` or `agent-watch.rs`; the `--dry-run` step takes turns through `<scratchpad>\dry-run.lock`.
- 2026-11-06 17:16 (laptop clock): no Antigravity process runs. Installed Wakeshot is 0.1.0 from 04:34 (before the
  `hook` command), so its trays add no hooks. Backup: `%USERPROFILE%\.gemini\config\plugins\` copied whole to
  `<scratchpad>\p4-live-antigravity\backup\plugins` before any write; it is an empty folder (made 13:48 by `agy`).
- 2026-11-06 17:22: debug build of the tree as it was at 17:22 (`wakeshot.exe`, `wakeshot-tray.exe`, `agent-watch.exe`,
  copied to `<scratch>\bin`); two earlier tries failed on other cards' half-done edits (`agent_hooks.rs`/`hooks.rs`,
  `watch/agents.rs`), not mine.
- 2026-11-06 17:24, add: `agent_hooks::at_start` (the product's add code) on the real profile would also run the
  Cursor row in `.cursor`, which `p4-live-cursor` works in now, and its "ours, older content" row would rewrite a
  hook folder that names another exe. So a scratch tool (`<scratch>\addhooks`, outside the repo) ran `at_start` at
  Medium on a scratch profile (`<scratch>\profile\.gemini\config`, the real `%LocalAppData%`, hook exe
  `<scratch>\bin\wakeshot.exe`, record in the scratch folder): "Antigravity: hook folder added", record
  `{"version":1,"antigravity":"added"}`. Its three files were copied at Medium into the real `plugins\` under the
  product's staging name `.wakeshot-new-<pid>` in the product's order, then renamed to `wakeshot`.
- 2026-11-06 17:24, no prompt spent: `agy plugin validate --strict --json` on the folder: valid, 1 hook group, one
  note (no logo). `agy plugin list --json` with only the staging folder there: **it lists `.wakeshot-new-49524` as an
  enabled plugin** (its name is the folder name), hooks `wakeshot`: so `agy` 1.5.2 skips no dot name, and the
  uninstall's `.wakeshot-removed` rename would not stop the hooks. After the rename: `wakeshot`, enabled, version
  1.0.0, hooks `wakeshot`. So a dropped folder loads with no `agy plugin install` and is enabled by default (CLI).
- 2026-11-06 17:25: `agent-watch --source antigravity` at Medium through the driver (`drive\watch`), hooks' data folder
  `<scratch>\data\dry-run` (a debug `wakeshot.exe` reads `WAKESHOT_DATA_DIR`). Prompt 1 (`agy -p ... --effort low`,
  Medium script): answered `ok` in 8 s; `agents\antigravity\` made by the first hook; `agent-watch` posted nothing:
  the folder did not exist at its start and its `--data` root is another folder, so it saw the folder only at the
  next tick, after the 2 s file was gone. The tray's root is the data folder itself (`thread.rs` retries missing
  folders on a root change); the dry-run step checks that case. Prompts so far: 1.
- 2026-11-06 17:27, TUI under the driver (`drive\cli1`, agy pid 40292, its parent the driver): header "Antigravity
  CLI 1.5.2", the account, "Gemini 3.8 Flash (Low)". `/hooks` (no prompt): PreInvocation 1 hook, Stop 1 hook. Prompt
  2 (one word): Enter at 17:27:37.79 to .85; kernel trace (`logman`, Microsoft-Windows-Kernel-Process, names and pids
  only): `agy.exe` 40292 started `cmd.exe` 52764 at 37.829, which started `wakeshot.exe` at 37.845 (exe 36 ms, cmd 54
  ms, exit 0); `agent-watch` posted `working`, pid 40292 `alive=Yes`, at 37.879. One `PreInvocation` for the
  one-step turn. `Stop`: `cmd` at 39.907 (49 ms), exe 28 ms; posted `n=0` at 39.952. Clock: the laptop's local time
  (+04:00), the same in PowerShell, the driver, `agent-watch` and the trace (tracerpt writes local time). Prompts: 2.
- 2026-11-06 17:30: a scratch capture plugin `wakeshot-capture` added at Medium (a Python script in the scratch
  folder that keeps each input there and answers `{}`), to see real inputs for the fixtures; the running TUI ran it
  from its next model call, so `agy` reads plugin folders again while it runs. Real input fields: `PreInvocation`
  `artifactDirectoryPath`, `conversationId`, `initialNumSteps`, `invocationNum`, `lastUserInput`, `modelName`,
  `transcriptPath`, `workspacePaths`; `Stop` adds `error`, `executionNum`, `fullyIdle`, `terminationReason` (seen:
  `NO_TOOL_CALL`) and drops the two counters. `ANTIGRAVITY_CONVERSATION_ID` is set and equals the input's id; the
  hook's working folder is its plugin folder; stdin is a pipe. Hooks of two plugins run one after the other (the
  trace: the capture's `cmd` ends, Wakeshot's starts 2 ms later), never in parallel.
- 2026-11-06 17:30, prompt 3 (`ping -n 20`, approval): the approval prompt "Run this command?" came; posted `working`
  at 17:30:50.069 and it stayed working through the 60 s wait. Approved at 17:31:50.07 (Enter on "1. Yes"). `agy`
  ran the command, after about 10 s moved it to the background ("Running ... waiting for it to finish"), and the loop
  ended: `Stop` with `fullyIdle: false` at 17:32:04.61, posted `idle tasks=1` at 04.662. When the ping ended, the
  agent woke by itself: `PreInvocation` (`invocationNum` 0) at 17:32:10.08, posted `working` at 10.124; `Stop` with
  `fullyIdle: true` at 17:32:13.00, posted `n=0` at 13.045. Each hook run: `cmd` 36 to 42 ms, the debug exe 25 to 32
  ms. Prompts: 3.
- 2026-11-06 17:33, prompt 4 (`ping -n 60`), approved at 17:33:29.2, **Esc** at 17:33:32.26 during the command: the TUI
  shows "Interrupted · What should Antigravity CLI do instead?", the ping is ended, and **no `Stop` runs** (no hook
  run in the trace or the capture). The chat's file stays `working` (pid 40292), so the hold would last until the
  next prompt's `Stop` or the end of `agy`. Prompts: 4.
- 2026-11-06 17:34, prompt 5 (start `ping -n 90` as a background command, reply `started`): its `PreInvocation` found
  the file already `working` (left by the Esc) and wrote nothing; `Stop` with `fullyIdle: false` at 17:34:39.78,
  posted `idle tasks=1` at 39.827; the footer showed "1 task(s)". Prompts: 5.
- 2026-11-06 17:35:16, `/exit` with the background task running: `agy` printed the resume line and ended (exit 0) at
  16.385; no hook ran; it ended the ping too. `agent-watch` posted `alive=Gone` at 16.380 (the watch thread waits on
  the process). The file stays as `background` until a later `PreInvocation` prunes it. Driver ended, `active_after`
  0.
- 2026-11-06 17:36, crash: a new TUI (`drive\cli2`, agy 48560). Prompt 6 (`ping -n 30`, approved): its
  `PreInvocation` posted `working` at 17:36:30.822 and pruned the old `background` file of 40292 (posted `n=1` at
  30.823). `agy` runs the command through `powershell.exe`. Killed agy 48560 by pid at 17:36:35.915 (exit 1 at 35.978);
  `agent-watch` posted `alive=Gone` at 35.975. No hook ran; the file stays `working` with the dead pid, which holds
  nothing. Driver ended, every pid gone. Prompts: 6.
- 2026-11-06 17:37, IDE: no Antigravity IDE ran. Started my own at Medium through Explorer (`run\ide.cmd`: main pid
  1832, `WAKESHOT_DATA_DIR` set) with a scratch extension (`<scratch>\ideext`, `--extensionDevelopmentPath`, so
  nothing is installed) that runs IDE commands from step files. On the sign-in folder the window was in Restricted
  Mode (`isTrusted` false) and the Antigravity extension was not loaded, so no hub. Quit it by `workbench.action.quit`
  and started it again with no folder (an empty window is trusted): main pid 53688, `google.google-antigravity`
  active, and the hub `C:\Users\<user>\.gemini\bin\agy.exe` (pid 48548) started at 17:39:58, its parent the extension
  host. `antigravity.sendPromptToAgentPanel` (the IDE's own command; one prompt, prompt 7) did nothing: a capture of
  my own window only (`PrintWindow` on its handle) shows the first-run screen "Welcome to Antigravity IDE / Get
  Started". Finishing it writes Haik's IDE state, so not by me: steps sent to the orchestrator at 17:42 for Haik.
  Prompts counted: 7 (prompt 7 reached no model).
- 2026-11-06 17:44: live fixtures (`tests/fixtures/antigravity/live/`, five inputs of prompt 3 in order, redacted by
  `<scratch>\run\redact.py`: fake chat id, paths under `C:/Users/dev`, prompt replaced; keys, order, types and the
  compact form kept) with a README; a line in the parent README. New tests `every_live_fixture_is_listed_once` and
  `the_live_turn_replays_into_the_sessions_the_live_test_saw`; the marker test now also runs on the live inputs.
  `cargo test --locked -p wakeshot-core --test antigravity_source`: 15 passed. No source fix needed: every real input
  parsed and gave the state the live run showed.
- 2026-11-06 17:45, dot names (prompt 8, `agy -p`, one word): capture plugin moved out, `wakeshot` renamed to
  `.wakeshot-removed` at Medium. `agy plugin list` showed `.wakeshot-removed` enabled with hooks `wakeshot`, and **its
  hooks ran**: `PreInvocation` (`cmd` 55372 under agy 6180) posted `working` at 17:45:12.434, `Stop` posted `n=0` at
  15.434. Renamed back at 15.881. So in `agy` 1.5.2 the uninstall's `.wakeshot-removed` rename does not stop the
  hooks (the folder name is the plugin's name; nothing skips a dot). Sent to the orchestrator for the owner of
  `agent_hooks.rs`. Prompts that reached a model: 7 (of 8 sent).
- 2026-11-06 17:48, through the app: dry-run lock taken at 17:48:00 (after `p4-live-opencode`'s). Dry-run tray (my
  build) at Medium through Explorer, pid 6340, `WAKESHOT_DATA_DIR` `<scratch>\tray` and the other agents' variables
  on empty scratch folders, so only Antigravity can hold; its `agents\` did not exist yet. A TUI (`drive\cli3`, agy
  44624) with the hook writing to `<scratch>\tray\dry-run`. Prompt 9 (one word): Enter 17:48:43.23; "Holding starts:
  Antigravity CLI" (dry-run power request) at 43.349, "Holding ends" at 46.025. So the first file in a folder the hook
  made while the tray ran is seen at once (root change, then the retry of missing folders).
- 2026-11-06 17:49, prompt 10 (start `ping -n 40` in the background, reply `started`): hold from 17:49:10.884; at the
  `Stop` (17:49:16.10) the tray **ended the hold** although the ping ran ("1 task(s)"); the ping's end woke the agent
  (hook at 17:49:54.13), hold again, ended at 58.83. Prompt 11, the same with the capture plugin back and a second
  `agent-watch` on the tray's folder (`drive\watch2`): `Stop` at 17:51:46.85 came with **`fullyIdle: true`** while
  the background ping ran, so the file was deleted (`n=0` at 46.909, tray "Holding ends" at 46.909); the ping's end
  woke the agent at 17:52:23 (`invocationNum` 0), hold again until its `Stop` (`fullyIdle: true`) at 17:52:25.9. So
  `fullyIdle` is not "no background command runs": seen `false` twice (prompt 3, a command `agy` moved to the
  background after waiting; prompt 5, after the Esc'd turn) and `true` twice for a command started in the background.
  The binary has `HasIdleBlockingBackgroundTasks` and a `WaitMsBeforeAsync` argument of its run-command step, so
  likely only some background tasks block "idle" (read as strings; meaning not verified). Each time the command
  ended while `agy` ran, the agent woke itself and the next turn held. Prompts that reached a model: 10.
- 2026-11-06 17:57: dry-run tray quit by `taskkill /PID 6340` (no force, path checked): "Wakeshot ends with code 0".
  Lock deleted at 17:57:21. TUI 44624 ended by `/exit`; drivers `cli3` and `watch2` ended, `active_after` 0.
- 2026-11-06 17:58 to 18:03: research section and `research.md` lines written; checks run (Result). From 18:02 to
  18:10 my IDE's command list was read every 2 minutes: no agent panel commands came, so the first-run screen stayed;
  no relay from the orchestrator. Capture plugin removed at 18:06:31 (Medium).
- 2026-11-06 18:10, clean-up: my IDE quit by `workbench.action.quit` (all 12 pids of its tree gone, hub included).
  Wakeshot's folder removed through the product's remove path, `<scratch>\bin\wakeshot.exe hook antigravity remove`
  at Medium: printed `{}`, exit 0, log "Antigravity: .gemini\config\plugins\wakeshot removed"; `agy plugin list`:
  `[]`. `plugins\` equals the backup (both empty). Seen: `.gemini\config\config.json` grew from 78 to 160 bytes at
  17:40:00, 2 s after my IDE's hub started; written by Antigravity itself, not read, not restored. `agent-watch`
  stopped clean, the trace session stopped, every pid this card started is gone (43 checked; three numbers are reused
  by other images started later). Build folders deleted.

## Result

Status: blocked for the IDE turn only (Haik's first-run pass, "Open items" 1); everything else done, 2026-11-06. Antigravity CLI 1.5.2, IDE 2.6.0, Free plan, Gemini 3.8 Flash low. 11 one-line prompts, 10
reached a model. Details: `docs/research/antigravity-signals.md`, "Phase 4 live test (2026-11-06)".

**Backup and compare:** `%USERPROFILE%\.gemini\config\plugins\` (empty, made by `agy`) copied to
`<scratch>\p4-live-antigravity\backup\` before any write; at the end it equals the backup (both empty). I read or
wrote nothing else under `.gemini` and no settings file. The IDE I started wrote its own `.gemini\config\config.json`
(78 to 160 bytes at 17:40:00, 2 s after its hub started); not read, not restored.

**Payload form (deliverable 1):** stdin at every hook run (a pipe), `ANTIGRAVITY_CONVERSATION_ID` set; the runtime
check found no payload key on `cmd`'s command line (no `blocked.json`, no log line). The plugin stays.

**What held (deliverables 2 and 3), CLI, each timed from the event to the post:**

| Step | Outcome |
|---|---|
| Folder loads | Dropped folder listed `enabled` with no install step; `/hooks` shows both hooks; `validate --strict` passes; a running TUI picks up a folder added later |
| Shell, parent | `wakeshot.exe` under `cmd.exe` under `agy.exe`; working folder the plugin's; two plugins' hooks run one after the other |
| Cost per `PreInvocation` | 21 runs: `cmd` 36 to 111 ms (median 42), debug exe median 31 ms; an unchanged "working" writes nothing. Release exe not measured |
| Working turn | Posted `working` 30 to 90 ms after Enter (17:27:37.879), `n=0` at the `Stop` (39.952); tray "Holding starts" 0.1 s after Enter, "Holding ends" at the `Stop` |
| Approval wait | 60 s at "Run this command?": `working` the whole time; then approved |
| Esc | **No `Stop`**: the file stays `working` until the next prompt's `Stop` or the end of that `agy.exe` |
| `fullyIdle` with a background command | `false` twice (posted `idle tasks=1`), but `true` twice while a background ping ran (hold ended at the `Stop`); the command's end woke the agent each time and that turn held |
| Clean exit | `/exit`: no hook; process and its background command end; `alive=Gone` posted at once |
| Crash | TUI killed by pid mid-command: no hook; `alive=Gone` in 60 ms; the dead pid holds nothing |
| `{}` lets a stop through | Every turn ended normally |
| Dot-named folders | **Loaded**: `.wakeshot-new-<pid>` listed, `.wakeshot-removed` ran both hooks (Found outside scope 1) |
| IDE | Hub `.gemini\bin\agy.exe --hub`, child of the extension host, only in a trusted window; no chat (first-run screen). Not tested live |

**Fixtures and tests (deliverable 4):** `crates/wakeshot-core/tests/fixtures/antigravity/live/` (five real inputs of
one turn, redacted: fake id, paths under `C:/Users/dev`, prompt replaced; README), a line in the parent README. New
tests `every_live_fixture_is_listed_once`, `the_live_turn_replays_into_the_sessions_the_live_test_saw`; the marker
test also runs on the live inputs. **No fix needed** in `sources/antigravity.rs`: every real input parsed and gave
the state the live run showed. Sent to the orchestrator (not mine to fix): the dot-name finding, the `at_start` risk.

**Through the app (deliverable 5, tray line already there):** dry-run tray (my build) at Medium with only
Antigravity able to hold: holds and ends as above, also for the first file in a hook folder made while it ran. No
wiring bug.

**Files changed:** `crates/wakeshot-core/tests/antigravity_source.rs`;
`crates/wakeshot-core/tests/fixtures/antigravity/live/` (new: five `.json`, `README.md`);
`crates/wakeshot-core/tests/fixtures/antigravity/README.md` (one paragraph);
`docs/research/antigravity-signals.md` (new last section); `docs/research.md` (new section "Antigravity live
test"); this card.

**Checks** (own build folder, `--locked`, no `RUSTFLAGS`): `rustfmt --edition 2024 --check` on
`tests/antigravity_source.rs`: clean. `cargo clippy --locked -p wakeshot-core -p wakeshot-windows -- -D warnings`,
with and without `--all-targets`, `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc` (LLVM folder on PATH for
ARM64): clean (4 runs). `cargo test --locked -p wakeshot-core -p wakeshot-windows --no-fail-fast`: core 730 (1
ignored), `antigravity_source` 15, `claude_status` 11, `claude_transcript` 25, `codex_source` 22, `copilot_source` 48,
`cursor_source` 8, `opencode_source` 17, `vscode_source` 33; `wakeshot-windows` 716 (28 ignored); 0 failed. Fixtures:
no user name, real id or scratch path (checked by the redaction script).

**Decided by the worker** (for Haik's review), each with when to change it:

1. The plugin came from `at_start` on a scratch profile, then a copy into the real `plugins\` (the product's staging
   name and order), because `at_start` on the real profile also runs the Cursor row that `p4-live-cursor` works in.
   Change it when no other card writes agent folders: run the installed tray's path itself.
2. The live test ran a debug hook exe on a scratch data folder (`WAKESHOT_DATA_DIR`), so Haik's real data folder got
   nothing. Change it when a release run on the real folder is wanted (the fresh install's check).
3. A scratch capture plugin beside Wakeshot's kept real inputs in the scratch folder for the fixtures (removed at
   18:06). It never ships. Change it when a later live card needs inputs again: the same tool, then removed.
4. The IDE ran as my own instance with a scratch extension (`--extensionDevelopmentPath`, nothing installed) that runs
   IDE commands, and my own window was captured by its handle only. Change it when computer use is available.
5. No code change for Esc or `fullyIdle` (both are Haik's product calls below).

**For Haik (product):**

1. Esc in Antigravity runs no `Stop`, so after an Esc the PC stays awake on AC until the next prompt ends or `agy`
   exits (an idle TUI left open: all night). Default: keep it so (no time cap, design question 5; the battery floor
   still protects). The other way: a time cap, or reading Antigravity's own state, both against today's rules.
2. A background command (a dev server) holds only when `agy` says `fullyIdle: false`, which it did in 2 of 4 runs.
   Default: accept the gap for v1 (no hold while such a command runs; the agent's own wake at its end holds again).
   The other way: also hook `PreToolUse` and `PostToolUse` to see background commands start, which costs a hook run
   per tool call and code in the shared hooked files.

**Clean-up (deliverable 7):** Wakeshot's folder removed through the product's remove path (`wakeshot.exe hook
antigravity remove` at Medium: "Antigravity: .gemini\config\plugins\wakeshot removed"; `agy plugin list` empty); the
capture plugin removed; every process this card started is gone (drivers, five `agy.exe`, the dry-run tray with code
0, my IDE and its hub; 43 pids checked); dry-run lock deleted; trace session stopped; build folders deleted. Left in
Haik's homes: the test chats of the CLI under `~\.gemini\antigravity-cli` (and its prompt history), and my IDE's
window state in the IDE's user data (an empty window and the sign-in folder opened once in Restricted Mode).

**Open items:**

1. **IDE turn, needs Haik once (about 3 minutes), then a short re-run by an agent.** Steps for Haik:
   1. Start "Antigravity IDE" from the Start menu.
   2. Click "Get Started" and go through the setup screens with any choices; if it asks to sign in, use the same
      personal Google account as the CLI.
   3. When the agent panel shows its chat box, close the IDE. Do not type a prompt.
   4. Tell the orchestrator "Antigravity IDE ready".
   Then an agent repeats prompt 7 here (`run\ide.cmd`, the scratch extension's `sendPromptToAgentPanel`) for a working
   turn, an approval through the panel's `antigravity.acceptAgentStep` and an interrupt if the panel registers one, a
   window close and a kill of the hub: about 3 prompts. The scratch tools stay in `<scratch>\p4-live-antigravity\`
   (`run\`, `ideext\`, `addhooks\` without its build folder).
2. Product calls 1 and 2 above.
3. Found outside scope 1 (dot names) for the owner of `agent_hooks.rs`; 2 (`at_start` across builds).
4. Not tested live: the release exe's cost, a `/schedule` timer, a Remote Control chat, `powercfg /requests`.
