# Live test: Cursor editor and CLI through Wakeshot's hooks

Phase: 4. Status: done (2026-11-06 20:40). Was: ready once `p4-cursor-source`, `p4-agent-hooks-install`, the hooks code review fixes and the live
card before it in the lane are done (new 2026-11-06). Needs Haik: done (`agent login` and three short rounds of
editor prompts, 2026-11-06 19:00 to 20:23).
Agent: `wakeshot-worker`. Size: about 130K tokens.
Runs after: `p4-cursor-source`, `p4-agent-hooks-install`, the hooks review in Round 4 (fixes of blockers and
majors), `p4-live-harness`; one live card at a time (agent-session lane).
Parallel with: build cards that own none of the files below; at most four Rust cards at once.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p4-live-cursor\target`; delete it when done.
Real laptop: Cursor 3.27.12 and its CLI for Haik's user, signed in on Free. Writes Wakeshot's real plugin folder
`%USERPROFILE%\.cursor\plugins\local\wakeshot\` through the product's own install code. No power settings.

## Goal

The live test of the Cursor source and the hooks install, as `docs/agent-hooks.md` "Live test" lists it (items 0
to 6), redacted fixtures, fixes with tests, and the source wired into the tray.

## Read first

- `docs/agent-hooks.md` (all; "Live test" and the privacy Gate), `docs/tasks/p4-cursor-source.md` and
  `docs/tasks/p4-agent-hooks-install.md` Results, the hooks reviews in `docs/reviews/`, `docs/tasks/p4-live-harness.md`
  Result (runbook, driver).

## Deliverables

1. Live test 0 first: the payload form on this Cursor build, at each hook run. If any hook input comes on the
   command line, remove the plugin through the product's remove path, stop, and write it under "Questions for Haik"
   (design Question 4, default: no Cursor plugin at all).
2. The plugin added by the product's own install code (the tray path, `--dry-run` if it allows, else the call the
   install card names), then live tests 1 to 6 of the design: the editor loads the plugin with no prompt; each event
   runs the exe; the process found lives as long as the window; one turn id from prompt to stop; which CLI modes fire
   (interactive and `-p`); the delay per prompt with and without the hook; the exe renamed away (prompts still go);
   uninstall removes the folder and a folder the user changed stays.
3. Each timed from the event to the post, with `agent-watch` and a new `--source cursor` row.
4. Fixtures: redacted hook inputs and state files into `crates/wakeshot-core/tests/fixtures/cursor/live/` (no prompt
   text, no user paths, fake ids), a replay test, fixes each with a test that failed before.
5. Wire the source into `crates/wakeshot-windows/src/app/tray_mode.rs` and test it through the app with `--dry-run`.
6. A section "Phase 4 live test (2026-11-..)" in `docs/agent-hooks.md` and short lines in `docs/research.md`; the
   Cursor CLI's status per design Question 1 (covered only if its hooks fired).
7. Clean-up: the plugin folder removed through the product's remove path at the end (the fresh install adds it again),
   every process you started ended by PID.

## Orchestrator changes (2026-11-06 ~18:50, for speed; Haik asked for more parallel work)

- Runs after `p4-antigravity-source` (it adds the Cursor tray and harness lines), not after the hooks code review:
  the review runs at the same time. Runs in parallel with `p4-live-antigravity`; both may fix `sources/hooked.rs`,
  `hook.rs` or `agent_hooks.rs`, so this card owns those fixes and `p4-live-antigravity` reports its own to the
  orchestrator.
- Before the first write: copy `%USERPROFILE%\.cursor\plugins\` whole to `<scratchpad>\p4-live-cursor\backup\`, so a
  bug cannot lose anything; at the end compare the folder with the backup (only Wakeshot's folder may differ, and it
  is removed). Never restore over the user's files without telling the orchestrator.
- If the hooks code review reports a blocker while you run, the orchestrator tells you; stop writing real folders.
- First step (orchestrator, 2026-11-06 ~21:10): fix the hooks code review (`docs/reviews/p4-hooks-code.md`, 0
  blockers, 1 major, 7 minors) in the files you own this round (`sources/hooked.rs`, `sources/cursor.rs`,
  `wakeshot-cli/src/hook.rs`, `agent_hooks.rs`, `hooks.rs`, `app/startup.rs`, and the stale lines in
  `docs/agent-hooks.md` and `docs/tasks/p4-agent-hooks-install.md`): M1 (run the command-line privacy check before
  the parent walk gate, and also when the process list fails) and each minor that is right, with a test that fails
  before each fix. Write a "Review fixes" table in your Result (Fixed, or Not fixed with the reason). Then the live
  steps.

## Rules

The "Rules for every live card of the bigger v1" in `docs/tasks/p4-live-vscode.md` apply. Also: never edit
`%USERPROFILE%\.cursor\hooks.json` or any Cursor settings file of Haik's; never close or kill a Cursor window you did
not start.

## Owns

`crates/wakeshot-core/tests/fixtures/cursor/live/`; fixes in `sources/hooked.rs`, `sources/cursor.rs`,
`wakeshot-cli/src/hook.rs`, `agent_hooks.rs` and their tests; the Cursor line in `tray_mode.rs` and in
`agent-watch.rs`; the sections above.

## Questions for Haik

None (no product question). Steps for Haik: done; the install check reuses the steps below.

## Steps for Haik, for the install check (`p6-agents-install-e2e` Part 2 reuses them)

Rounds 1 to 3 of this card are done (2026-11-06, Log 19:05, 19:28, 20:20). What they taught: a cancel is the Stop
button (the square at the chat box), never Esc; a cancel prompt must run long enough to click Stop (count to 10000);
send each prompt once; a test window of its own, never your "Cursor Agents" window. About 5 minutes, with the
installed Wakeshot running and its tray icon visible:

1. Open a new Cursor window on a test folder (File > New Window, then open the folder). If it asks whether you trust
   the folder, say yes.
2. Send `This is an app test. Reply with one word: ok`. While it answers, Wakeshot's icon shows a hold; it ends a
   second or two after the answer.
3. Send `This is an app test. Count from 1 to 10000, one number per line.` About 3 seconds after the numbers start,
   click the Stop button. The hold ends within about 2 seconds.
4. Send `This is an app test. Count from 1 to 3000, one number per line.` When the numbers start, press Ctrl+F4 (the
   chat's tab closes, the turn goes on unseen). The hold stays until that turn ends, about a minute.
5. Not run yet: send the 3000 count again; when the numbers start, open Show Chat History (the clock icon at the chat
   tabs) and delete that running chat. Note whether the hold ends within about a minute or stays until step 6.
6. Close the window with its X. The hold, if any, ends at once.
7. Tell the orchestrator "done", with anything Cursor showed and the times you saw.

## Found outside scope

1. `crates/wakeshot-windows/src/sqlite.rs` (`p4-live-opencode`, mid-edit at 17:40): ARM64 lib clippy (without
   `--all-targets`) fails with "constant `SQLITE_IOERR` is never used" (line 79). x64 lib clippy passed a minute
   before. Not touched. Clean again at 18:05.
2. The Cursor CLI was not signed in for Haik's user (the orchestrator's note said it was): `agent status` "Not logged
   in", `agent --list-models` "Authentication required" (17:36, Medium). Signed in at 19:13.
3. `crates/wakeshot-core/tests/vscode_source.rs`, `every_live_fixture_is_listed_once`, failed at 20:29 while another
   card added files to `fixtures/vscode/live/`; it passed at 20:36. Not touched.
4. Haik's installed Wakeshot tray (pid 11424 since 04:34, guard 48172) ran through the whole test. This card never
   touched it (no power settings; the hook was a debug build writing a scratch data folder; its version predates the
   agent hooks, so it added no plugin).
5. Left in Haik's Cursor profile by the test windows and CLI runs, not deleted (Cursor's own files): four folders
   `.cursor\projects\<path>-p4-live-cursor-work-{cli,cursor-live,cursor-live2,cursor-live3}`, the test chats in
   Cursor's chat history, and `cli-config.json`, `agent-cli-state.json`, `statsig-cache.json` rewritten by the CLI at
   19:31 to 19:33 (`cli-config.json` does not name the test folder). Harmless; Haik may delete the four folders.
6. The Cursor CLI sets `CURSOR_PLUGIN_ROOT` to a Claude Code plugin's folder (`.claude\plugins\cache\
   claude-plugins-official\postman\2.3.0`) for our plugin's `sessionEnd` hook: it loads Claude Code's plugins too and
   names the wrong one. Cursor's bug; Wakeshot does not read the variable.
7. Product cost to weigh, not a bug: each Cursor prompt waits about 0.85 s for the hook (Windows PowerShell's start;
   the exe is about 20 ms of it). Every Cursor hook pays this, not only Wakeshot's.
8. `crates/wakeshot-core/tests/fixtures/cursor/README.md` (`p4-cursor-source`) says this card replaces those made-up
   inputs with real captures; they went to `fixtures/cursor/live/` instead (this card owns only that folder), and the
   made-up `editor-session-end.json` lacks the real `generation_id` and `final_status`. A one-line README fix.

## Log

- 2026-11-06: card written by the orchestrator.
- 2026-11-06: worker started. Order: review fixes, backup of `.cursor\plugins`, live tests 0 to 6. No edits to
  `tray_mode.rs` or `agent-watch.rs` (live-card rule 1 wins over deliverables 3 and 5).
- 2026-11-06: review fixes in code: M1 (check before the gate; no agent: remove, or hand off `hook <agent> remove`),
  m5 (chat files end on the blocked path and in `remove`), m1 (`medium_copies`, one shared 5 s wait), m3 (staging
  errors are log lines), m4 (`.wakeshot-removed-2` to `-9`), m6 (`remove` private), 8.3 form for Antigravity. CLI
  `hook` 24 tests pass; broken on purpose: M1 and m5 tests fail (2 of 24), m4 test fails; all put back. Windows
  `hooks` filter 58 pass (8.3 test took `WA2A5C~1\MYAPPS~1`). m2 accepted, doc text. m7 docs fixed.
- 2026-11-06 17:28: backup of `%USERPROFILE%\.cursor\plugins\` to `<scratch>\p4-live-cursor\backup\plugins\` with a
  manifest: it held only an empty `local\` (made 14:30 by Cursor).
- 2026-11-06 17:32:54: plugin added by the product's own `agent_hooks::at_start` (scratch tool `hooks-add`, Medium
  through Explorer) with hook exe `<scratch>\p4-live-cursor\bin\wakeshot.exe` (debug build of this round's code) and a
  scratch record. `ANTIGRAVITY_GEMINI_DIR` was set for it, so the Antigravity row stopped at `present` before any
  write (its folder still has its 17:24 times). Log: "Cursor: hook folder added". Haik's real record and data folder
  untouched.
- 2026-11-06 17:33: a capture plugin `.cursor\plugins\local\wakeshot-capture\` (mine, test only, removed at the end)
  saves each raw input to scratch and the hook shell's command line facts. `agent-watch --source cursor` at Medium
  (pid 38644) with `WAKESHOT_DATA_DIR=<scratch>\data\dry-run`; a folder poller and a process trace at High.
- 2026-11-06 17:34:43: my own window `cursor-live - Cursor` opened (Medium `.cmd`, `cursor -n`). It joined Haik's
  running Cursor main process (pid 49860, his window "Cursor Agents" since 14:29, not touched). No dialog in the UIA
  tree at load.
- 2026-11-06 17:36: the Cursor CLI says "Not logged in" at Medium (`agent status`), so the CLI tests need Haik's
  `agent login`. The editor's chat input does not take text through UI Automation (`SetValue` leaves it empty), and
  typing into it would take the focus while Haik works: the editor prompts need Haik too.
- 2026-11-06 17:40: checks for the review fixes: clippy x64 and ARM64 with `--all-targets` clean, x64 libs clean;
  ARM64 libs failed in `sqlite.rs` (`SQLITE_IOERR` never used), `p4-live-opencode`'s file mid-edit: reported.
- 2026-11-06 17:45: orchestrator (from `p4-live-antigravity`): no `at_start` or tray that runs it on the real profile
  while that card runs. Done so: my one `at_start` (17:32:54) skipped Antigravity as above; a dry-run tray never runs
  it (`startup::add_agent_hooks` returns on `boot.dry_run`); any later add goes on a scratch profile.
- 2026-11-06 17:50: tests all pass (core 730 and every integration file, `wakeshot-windows` 715, CLI 101, `exe` 3,
  `hook` 5 plus its 4 Explorer tests with `--ignored`, `run` 5). Blocked on "Steps for Haik".

**Continuation (for me after SendMessage, or a new worker).** `<S>` = `<scratchpad>\p4-live-cursor`.

- Running, all mine (PIDs in `<S>\logs\pids.txt`): `agent-watch` 38644 (Medium, its `cmd` 51892), poller 56304,
  process trace 53724, conductor 46880 (High; each ends by itself after 4 to 5 hours, or on `<S>\run\stop.flag`).
  Window `cursor-live` (hwnd 14618672) is mine, in Haik's Cursor main 49860; its new processes 15964, 34012, 46160,
  48240, 51124, 54404, 56252.
- The conductor (`<S>\run\conductor.ps1`, log `<S>\logs\conductor.log`) keys on the capture plugin's `stop` files: after
  prompt A it renames `<S>\bin\wakeshot.exe` away (test 5), after the second prompt puts it back, after window 1
  closes edits `hooks\hooks.json` (timeout 6) and runs the product's remove (`<S>\run\remove.cmd`, Medium:
  `wakeshot.exe hook cursor remove`), so the folder should become `.wakeshot-removed`, then opens `cursor-live2`
  (test 6: does the editor skip a dot name? `<S>\logs\trace.log` shows a `wakeshot.exe` start if not).
- Evidence: `<S>\capture\*.json` (raw inputs, scratch only) and `*.info.json` (hook shell facts: payload key on the
  command line or not, cwd, `CURSOR_PLUGIN_ROOT`, whether `WAKESHOT_DATA_DIR` reached the hook), `<S>\logs\poll.log`
  and `<S>\states\` (chat files with times), `<S>\logs\watch.log` (posts), `<S>\data\dry-run\logs\` (the hook's
  lines). Check also `%LocalAppData%\Wakeshot\agents\cursor` (the poller watches it): if the env did not reach the
  hook, files land there; delete what this card made.
- Then: tests 0 to 2 and 4 to 6 from the evidence (test 4: rebuild Cursor's exact PowerShell line from the capture
  info and time it 10 times with the Wakeshot command; the trace's times come in about 1 s batches, so they time
  nothing). CLI (test 3) after `agent login`: the plugin back through `at_start` on a scratch profile, the Cursor
  folder copied into `.cursor\plugins\local\` (orchestrator rule while `p4-live-antigravity` runs), `agent -p` with
  `--trust` from a Medium `.cmd`, interactive through `agent-drive`; the `--dry-run` tray step (lock
  `<scratchpad>\dry-run.lock`, tray `<S>\bin\wakeshot-tray.exe --dry-run` with `WAKESHOT_DATA_DIR=<S>\data`, quit by
  `taskkill /PID` without `/F`); fixtures to `tests/fixtures/cursor/live/` and a replay test; docs; clean-up (stop
  flag, `remove.cmd` for the unchanged case, delete `wakeshot-capture` and `.wakeshot-removed*`, compare with
  `<S>\backup\manifest-before.txt`, delete `<S>\target`).
- 2026-11-06 18:05: live fix from `p4-live-antigravity` (agy loads dot-named plugin folders): staging and removed
  folders now go to the agent's own folder (Result, "Review fixes", last row). `agent_hooks` tests 59 pass; with the
  old places on purpose 4 fail; put back. rustfmt clean; clippy x64 and ARM64, with and without `--all-targets`,
  clean (ARM64 libs clean now too). Nothing written in the real profile. The running conductor still uses the
  17:31 build (old rule: `.wakeshot-removed` in `local\`), which is what its editor dot-name test needs; the CLI part
  uses a rebuilt exe. Still blocked on "Steps for Haik".
- 2026-11-06 ~19:05: second worker took over. Haik did the steps about 19:00, not as written: prompt 1 sent twice
  (the first too slow), the 1-to-200 prompt ended before a cancel, his third prompt (count to 10K) is the cancel case,
  cancelled with the Stop button (Esc did nothing); `cursor-live2` did not open. Reading the evidence first.
- 2026-11-06 19:15: round 1 evidence. The capture plugin named `C:\Program Files\PowerShell\7\pwsh.exe`, which does
  not exist here (pwsh is the Store build), so each capture run ended with exit 127 and `<S>\capture` stayed empty.
  The conductor keyed on capture stops, so the exe was never renamed (test 5 not run) and `cursor-live2` never opened.
  Wakeshot's own hook worked: 6 prompts in one chat (UUID-shaped id), each wrote a working file naming pid 48240 (a
  `Cursor.exe` of window 1), each deleted by the next hook run 3.4 to 9.6 s later; one more hook run at 19:09:46 with
  no file (the window close: which event, not verified). Cursor ran each hook as `powershell.exe -NoProfile
  -NonInteractive -ExecutionPolicy Bypass -c "<UTF-8 setup>; Get-Content -LiteralPath '<temp file>' ..."`; the three
  command lines the trace read (1024 to 1038 chars) had no `conversation_id`. agent-watch had ended at 18:47 (`^C` in
  its console, before the prompts): no posts. `agent status` at Medium now: logged in (19:13).
- 2026-11-06 19:27: round 2 set up. Conductor 46880 ended by PID. Capture plugin fixed (Windows PowerShell 5.1, a
  path with no space) and it now redacts at capture time: no prompt text, e-mail or path reaches the disk. Debug build
  of the current code in `<S>\bin` (a release build in `<S>\bin-release`: release ignores `WAKESHOT_DATA_DIR`, so it
  would write the real data folder). agent-watch (debug) at Medium with no console (pid 48456, via `watch2.vbs`),
  conductor `run\conductor2.ps1` at High (pid 56444; log `logs\conductor2.log`): counts only stops whose hook has my
  data folder, renames the exe away after stop 2 and back after stop 3, logs the found process and its end after the
  close. Window `cursor-live2` opened 19:26:45 (hwnd 2231074, new Cursor pids 520 2248 43016 49616 55048 55968, in
  Haik's main 49860); no dialog in its UIA tree. An installed Wakeshot tray runs (pid 11424 since 04:34, guard 48172);
  not touched. Blocked on "Steps for Haik" round 2.
- 2026-11-06 19:28 to 19:29: Haik's round 2 in `cursor-live2` (2 prompts, then the close; he reported nothing shown).
  Prompt 1: `beforeSubmitPrompt` and `stop` `completed`, same `generation_id`. Prompt 2 (count to 10000, Stop
  button): `stop` with `status` `aborted` and the prompt's `generation_id`, 14 s after the send; the hook deleted the
  file 1.4 s after Cursor wrote the payload. Window closed 19:29:05.351; the found process (pid 520, `Cursor.exe
  --type=utility --utility-sub-type=node.mojom.NodeService`, the window's extension host, child of the main 49860)
  ended 19:29:05.370. No `sessionEnd` at the close. Prompt 3 (exe gone) not sent; conductor2 never counted the
  captures (a bug in my script), so no rename happened anyway. Conductor2 ended by PID.
- 2026-11-06 19:31: CLI `-p` at Medium (`agent -p --trust`, CLI 2026.11.05-86047df): answered; only `sessionEnd`
  fired (no `beforeSubmitPrompt`, no `stop`); no chat file. The CLI's hook line: `powershell.exe -ExecutionPolicy
  Bypass -NonInteractive -File %TEMP%\ps-script-<uuid>.ps1` (no `-NoProfile`), payload key not on it.
- 2026-11-06 19:32 to 19:34: CLI interactive through `agent-drive` at Medium: `enter` after a 2 s pause submits
  (`kenter` does not); answered; only `sessionEnd` at the exit. Same as `-p`.
- 2026-11-06 19:35: test 4 at Medium, Cursor's exact line rebuilt, 10 runs each: PowerShell 5.1 alone median 841
  ms; with the debug hook exe 858 ms; exe gone 883 ms, exit 127, empty stdout.
- 2026-11-06 19:38: test 6 on the real folder through `hook cursor remove` at Medium: unchanged folder removed
  ("removed"), `local\` back to the capture plugin only, nothing else in `.cursor`. Then the same bytes copied back,
  `hooks.json` timeout 5 to 6: moved to `.cursor\wakeshot-removed` (changed `hooks.json` and the marker kept,
  `plugin.json` deleted), log line "moved to .cursor\wakeshot-removed". That test folder deleted by me.
- 2026-11-06 19:39 to 19:40: through the app. Lock `<scratchpad>\dry-run.lock` taken 19:39:22; dry-run tray (debug,
  Medium, pid 47084, every other agent on an empty folder). `Cursor.exe` itself as Node (`ELECTRON_RUN_AS_NODE`, pid
  14516, Medium) ran the hook the editor's way (temp file, same PowerShell line, fake ids). Prompt A at 41.554:
  "Holding starts: Cursor" 42.518; `stop` `aborted` at 57.571: "Holding ends" 58.482; prompt B at 40:06.521: holds
  07.469; the process exits with no `stop` at 19.511: "Holding ends" 19.535 (24 ms). Tray quit by `taskkill /PID`
  (no force), code 0; lock released 19:40:50.
- 2026-11-06 19:44: round 3 set up for the "No guessing" check (Steps for Haik, round 3). Wakeshot's plugin stays
  removed (test 6); the capture plugin alone shows whether each turn gets its `stop`. Window `cursor-live3` (hwnd
  4197230, new Cursor pids 8424 8756 38824 47392 53832 55672 56640); logger `run\conductor3.ps1` at High (pid 23260,
  log `logs\conductor3.log`: capture events, transcript sizes and times, never their text, the found process). No
  dialog in the window's UIA tree.
- 2026-11-06 20:20 to 20:23 (clock): Haik's round 3 (he reported nothing shown). An empty chat ended first
  (`sessionEnd`, `user_close`, `final_status` `none`, no turn id). Chat A's prompt 20:20:37; Ctrl+N; chat B's prompt
  20:20:48; Ctrl+F4 on B 20:20:58: `sessionEnd` `user_close`, `final_status` `generating`, B's turn id, and B's turn
  went on. A's `stop` `completed` 20:21:36.9 (its transcript's last write 20:21:35.4); B's `stop` `completed`
  20:21:48.8 (transcript 20:21:47.3). Window closed 20:22:58. So both background turns end with their own `stop`,
  but the old rule (`sessionEnd` deletes) would have ended B's hold at the tab close, 50 s before B's work ended.
- 2026-11-06 20:30: fix: Cursor's `sessionEnd` keeps the chat working when `final_status` is `generating`
  (`Action::EndUnlessRunning` in `hooked.rs`, rule in `cursor.rs`; any other value, type or no field deletes, as
  before). Live fixtures in `tests/fixtures/cursor/live/`, replay test `tests/cursor_live.rs` (5), hook test
  `a_running_chats_closed_tab_keeps_it_working_until_its_own_stop`, unit test
  `an_end_that_names_a_running_turn_keeps_the_chat_working`. With the old rule put back on purpose, the replay test
  and the hook test failed; restored.
- 2026-11-06 20:30 to 20:36: rustfmt check clean on my six files; clippy `-p wakeshot-core -p wakeshot-windows -p
  wakeshot-cli`, with and without `--all-targets`, x64 and ARM64: clean. Tests: one run stopped at another card's
  `vscode_source.rs` (mid-edit, Found outside scope 3); the `--no-fail-fast` rerun passed everything.
- 2026-11-06 20:34 to 20:40: clean-up. agent-watch 48456 ended by PID; poller 56304 and trace 53724 ended on
  `run\stop.flag`; conductor3 had ended by itself (20:25:58). My capture plugin deleted; `.cursor\plugins` now equals
  the 17:28 backup (only an empty `local\`); no `.cursor\wakeshot-removed`; nothing in `%LocalAppData%\Wakeshot\agents`;
  no `cursor-hook-payload-*` left in `%TEMP%`; no test window open (only Haik's "Cursor Agents"); `<S>\target` (14 GB)
  deleted. Docs: `agent-hooks.md` ("Phase 4 live test", events, facts, failure cases), `research.md` ("Cursor live
  test").

## Result

Status: done, 2026-11-06 20:40. No git, no power settings. Live tests 0 to 6, the CLI part, the app step, one fix
with tests, fixtures, docs and the clean-up are done; the review fixes below were part 1.

**Review fixes** (`docs/reviews/p4-hooks-code.md`; each test failed before the fix where marked):

| Finding | Outcome | Test |
|---|---|---|
| M1: the command-line check only after the walk found an agent | Fixed: it runs before the gate; with no agent process: no file, one log line, the plugin removed (above Medium through the Medium copy's `hook <agent> remove`). A failed process list leaves the parent without a name: one honest log line, nothing written, the next run checks again (no call for one process's parent without a contract change) | `the_command_line_check_runs_without_an_agent_process_too` (broken on purpose: failed) |
| m1: 5 s per agent before the restore | Fixed: `medium_copies` starts every copy, then one shared 5 s wait (`on_uninstall` takes one call for all agents) | `the_copies_share_one_wait`; `an_elevated_uninstall_hands_antigravity_to_its_own_medium_copy` now asserts one call for both |
| m2: the "left behind" line deleted with the logs | Not fixed, accepted: keeping the logs needs a `HookEnv` contract change, for a case that needs a changed or held file and a failed rename. Design text now says the README is the only pointer; change it when a user reports hooks left after an uninstall | none (doc) |
| m3: an unreadable staging folder blocks the main folder | Fixed: each staging problem is a log line; the main folder's add or removal goes on | `a_staging_folder_that_does_not_read_never_blocks_the_main_folder` (by code the old `?` returned first) |
| m4: a second `.wakeshot-removed` cannot be made | Fixed: first free name, `-2` to `-9` after the first, never replacing (names and place since the live fix: last row); README line updated | `uninstall_renames_a_folder_it_cannot_empty` (broken on purpose: failed) |
| m5: other chats' files stay after the blocked path | Fixed: chat files (`<chat id>.json` by the id rule) deleted under the mutex on the blocked path and in every Medium `hook <agent> remove` | `the_blocked_path_and_the_remove_form_end_every_chat_of_the_agent` (broken on purpose: failed) |
| m6: `agent_hooks::remove` public with no level check | Fixed: private | compile-time only |
| m7: stale docs | Fixed: `agent-hooks.md` steps 3 and 4, the `tray_mode` lines, a "Review fixes" note; the install card's open items | none (doc) |
| Live (this card, round 3): Cursor's `sessionEnd` deleted the chat's file when a running chat's tab was closed (Ctrl+F4), though the turn ran on 50 s and fired its own `stop` | Fixed: `Action::EndUnlessRunning` (`hooked.rs`); Cursor's `sessionEnd` keeps the chat working when `final_status` is `generating`, any other value, type or no field deletes as before; the Medium copy takes `working` or `none` for it | `a_running_chats_closed_tab_keeps_it_working_until_its_own_stop` in `tests/cursor_live.rs` and in `hook.rs` (both failed with the old rule, broken on purpose); `an_end_that_names_a_running_turn_keeps_the_chat_working` |
| Live (`p4-live-antigravity`, agy 1.5.2): `agy` loads dot-named plugin folders, so `.wakeshot-removed` and a half-staged `.wakeshot-new-*` in `plugins\` load | Fixed for both agents (one rule): staging in the agent's own folder (`.cursor\.wakeshot-new-<pid>`, `.gemini\config\.wakeshot-new-<pid>`), moved into place; a folder the uninstall cannot empty moves to `.cursor\wakeshot-removed` or `.gemini\config\wakeshot-removed` (`-2` to `-9`); leftovers cleaned in both places; the hook file is deleted first (every delete was already tried, so this only orders them) | `the_add_never_fills_a_staging_folder_someone_else_made`, `uninstall_renames_a_folder_it_cannot_empty`, `a_crafted_marker_or_record_deletes_nothing_outside_the_fixed_set`, `a_folder_antigravity_or_the_user_changed_is_never_touched` (all four failed with the old places, broken on purpose); new `a_staging_folder_in_the_agents_own_folder_is_cleaned_too` |

Also (orchestrator's item, about 6K tokens): Antigravity hooks for an exe path with a space now use its 8.3 form
(`GetShortPathNameW`) when that passes the allow list with `~` allowed inside a name; else no hooks, as before.
Test: `a_path_cmd_would_split_gets_no_antigravity_hooks` (here the temp path became `WA2A5C~1\MYAPPS~1`, and the hook
file names it).

**Files changed (whole card):** `crates/wakeshot-core/src/sources/hooked.rs`, `crates/wakeshot-core/src/sources/cursor.rs`,
`crates/wakeshot-core/tests/cursor_live.rs` (new), `crates/wakeshot-core/tests/fixtures/cursor/live/` (new: 11 inputs,
a chat file, README), `crates/wakeshot-cli/src/hook.rs`, `crates/wakeshot-windows/src/agent_hooks.rs`,
`crates/wakeshot-windows/src/hooks.rs` (copy closure type only), `docs/agent-hooks.md`, `docs/research.md`,
`docs/tasks/p4-agent-hooks-install.md`, this card. No contract, `Cargo`, `tray_mode.rs` or `agent-watch.rs` change (the
Cursor lines there came from `p4-antigravity-source`; this card tested through them).

**Checks:** rustfmt check clean on the six Rust files. Clippy `-p wakeshot-core -p wakeshot-windows -p wakeshot-cli
-D warnings`, with and without `--all-targets`, x64 and ARM64: clean (20:30). Tests `--no-fail-fast`, x64: all pass
(core lib 731, its integration files incl. `cursor_live` 5; `wakeshot-windows` 716; CLI 102, `exe` 3, `hook` 5, `run`
5). The new tests failed with the old rule. Release build of the CLI and tray: built (19:20).

**Two Wakeshot builds on one profile** (orchestrator's question): the "ours, unchanged, older content" row rewrites a
folder that names another build's exe to name its own. Right for the product: only an installed tray writes, Velopack
installs one per user, so the folder should always name that install's exe, and a rewrite after an update or a moved
install is what keeps it working. It bites only tests that run `at_start` on the real profile beside an installed
tray or another test: run those on a scratch profile. No code change.

**Live tests** (details: `docs/agent-hooks.md`, "Phase 4 live test"; times in the Log):

| # | Outcome |
|---|---|
| 0 | Payload in a temp file on all 13 editor runs read; the CLI's form also keeps it off the command line. The Gate holds |
| 1 | No prompt or dialog at load; each event ran the exe; the process found is the window's extension host, which ended 20 ms after the close; ids are UUIDs |
| 2 | One `generation_id` from prompt to `stop`, also for the Stop button and background turns |
| 3 | CLI (interactive and `-p`): only `sessionEnd` fires from a plugin, so the CLI is not covered (design Question 1 stays open) |
| 4 | About 0.85 s per prompt, almost all Windows PowerShell's start (exe about 20 ms); `agent-watch` posted 1.1 to 1.6 s after Cursor wrote the payload (3.7 s for a new window's first prompt) |
| 5 | A missing hook exe (same form) failed open with exit 127 on 12 runs, prompts went, no message (Haik). Not run with Wakeshot's own exe renamed |
| 6 | Unchanged folder removed; a changed one moved to `.cursor\wakeshot-removed` with the user's file kept |
| app | Dry-run tray: hold starts 0.96 s after the prompt hook, ends 0.91 s after `stop`, and 24 ms after the agent process exits with no `stop` |

**"No guessing" check (Haik's rule):** the Stop button fires `stop` `aborted` for that turn, and the hold ends about
1.4 s after; exact. Esc does not cancel a turn in 3.27.12. A chat left running behind Ctrl+N, or its tab closed with
Ctrl+F4, runs on and ends with its own `stop`; exact, after this card's `sessionEnd` fix (before it, Ctrl+F4 ended the
hold 50 s too early). Window close: the extension host ends at once. No case seen leaves "working" with no event, so
no blocker for the editor from what was run. Open: a chat deleted while it runs (code: no `stop` when the chat's data
is gone; if so the hold lasts until the window closes, which would be a blocker by the rule), an `error` turn, the
"Cursor Agents" window. The install check's steps above run the deleted-chat case (step 5).

**Open items:** the deleted-chat case (install check step 5); the CLI is not covered (product question 1); Haik may
delete the four `.cursor\projects\...p4-live-cursor-work-*` folders (Found outside scope 5); the 0.85 s per prompt
(Found outside scope 7).
