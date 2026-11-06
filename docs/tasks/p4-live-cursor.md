# Live test: Cursor editor and CLI through Wakeshot's hooks

Phase: 4. Status: blocked (2026-11-06 17:50) on "Steps for Haik"; the review fixes are done. Was: ready once `p4-cursor-source`, `p4-agent-hooks-install`, the hooks code review fixes and the live
card before it in the lane are done (new 2026-11-06). Needs Haik: no (he signed in to Cursor and its CLI,
2026-11-06 ~17:30); only if the payload check finds the command line (design Question 4).
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

None (no product question). Steps Haik must do by hand: "Steps for Haik" below.

## Steps for Haik (about 5 minutes; the card is blocked on them)

Why: the Cursor CLI is not signed in for your user (`agent status` at Medium: "Not logged in"), and the editor's chat
box takes no text through UI Automation, so a test prompt would need the keyboard focus while you work. Everything is
set up; my monitors record each step by themselves. Do not use your own "Cursor Agents" window for this.

1. In a normal terminal (not admin): `agent login`, finish the sign-in in the browser, then `agent status`; it should
   say you are logged in.
2. In the Cursor window titled **cursor-live - Cursor** (Wakeshot's test window):
   1. Click the chat box ("Plan, Build, / for skills, @ for context"), type
      `This is an app test. Reply with one word: ok` and press Enter. Wait for the answer, then 5 seconds.
   2. Send the same text again and wait for the answer. (Wakeshot's hook exe is gone for this one on purpose: note any
      message Cursor shows.) Wait 5 seconds.
   3. Send `This is an app test. Count from 1 to 200, one number per line.` and press Esc about 2 seconds after the
      answer starts.
   4. Close the window with its X. If Cursor asks anything, note it and take the default.
3. Within about a minute a new window **cursor-live2 - Cursor** opens by itself. If it asks whether you trust the
   folder, say yes (it is a test folder). Send `This is an app test. Reply with one word: ok`, wait for the answer,
   then close that window.
4. Tell the orchestrator "done", with anything Cursor showed (a plugin prompt, a hook error, a dialog).

## Found outside scope

1. `crates/wakeshot-windows/src/sqlite.rs` (`p4-live-opencode`, mid-edit at 17:40): ARM64 lib clippy (without
   `--all-targets`) fails with "constant `SQLITE_IOERR` is never used" (line 79). x64 lib clippy passed a minute
   before. Not touched.
2. The Cursor CLI was not signed in for Haik's user (the orchestrator's note said it was): `agent status` "Not logged
   in", `agent --list-models` "Authentication required" (17:36, Medium).

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

## Result

Status: blocked, 2026-11-06 17:50 (part 1 done, the live part waits on "Steps for Haik"). No git, no power settings.

**Review fixes** (`docs/reviews/p4-hooks-code.md`; each test failed before the fix where marked):

| Finding | Outcome | Test |
|---|---|---|
| M1: the command-line check only after the walk found an agent | Fixed: it runs before the gate; with no agent process: no file, one log line, the plugin removed (above Medium through the Medium copy's `hook <agent> remove`). A failed process list leaves the parent without a name: one honest log line, nothing written, the next run checks again (no call for one process's parent without a contract change) | `the_command_line_check_runs_without_an_agent_process_too` (broken on purpose: failed) |
| m1: 5 s per agent before the restore | Fixed: `medium_copies` starts every copy, then one shared 5 s wait (`on_uninstall` takes one call for all agents) | `the_copies_share_one_wait`; `an_elevated_uninstall_hands_antigravity_to_its_own_medium_copy` now asserts one call for both |
| m2: the "left behind" line deleted with the logs | Not fixed, accepted: keeping the logs needs a `HookEnv` contract change, for a case that needs a changed or held file and a failed rename. Design text now says the README is the only pointer; change it when a user reports hooks left after an uninstall | none (doc) |
| m3: an unreadable staging folder blocks the main folder | Fixed: each staging problem is a log line; the main folder's add or removal goes on | `a_staging_folder_that_does_not_read_never_blocks_the_main_folder` (by code the old `?` returned first) |
| m4: a second `.wakeshot-removed` cannot be made | Fixed: first free of `.wakeshot-removed`, `-2` to `-9`, never replacing; README line updated | `uninstall_renames_a_folder_it_cannot_empty` (broken on purpose: failed) |
| m5: other chats' files stay after the blocked path | Fixed: chat files (`<chat id>.json` by the id rule) deleted under the mutex on the blocked path and in every Medium `hook <agent> remove` | `the_blocked_path_and_the_remove_form_end_every_chat_of_the_agent` (broken on purpose: failed) |
| m6: `agent_hooks::remove` public with no level check | Fixed: private | compile-time only |
| m7: stale docs | Fixed: `agent-hooks.md` steps 3 and 4, the `tray_mode` lines, a "Review fixes" note; the install card's open items | none (doc) |

Also (orchestrator's item, about 6K tokens): Antigravity hooks for an exe path with a space now use its 8.3 form
(`GetShortPathNameW`) when that passes the allow list with `~` allowed inside a name; else no hooks, as before.
Test: `a_path_cmd_would_split_gets_no_antigravity_hooks` (here the temp path became `WA2A5C~1\MYAPPS~1`, and the hook
file names it).

**Files changed:** `crates/wakeshot-cli/src/hook.rs`, `crates/wakeshot-windows/src/agent_hooks.rs`,
`crates/wakeshot-windows/src/hooks.rs` (copy closure type only), `docs/agent-hooks.md`,
`docs/tasks/p4-agent-hooks-install.md`, this card. No contract or `Cargo` change.

**Checks so far:** `rustfmt --edition 2024` on the three Rust files: clean. Clippy `-p wakeshot-core -p
wakeshot-windows -p wakeshot-cli --all-targets -D warnings`: x64 and ARM64 clean; without `--all-targets`: x64 clean,
ARM64 failed in `sqlite.rs` (another card's file mid-edit, Found outside scope 1). Tests: all pass (Log 17:50).

**Two Wakeshot builds on one profile** (orchestrator's question): the "ours, unchanged, older content" row rewrites a
folder that names another build's exe to name its own. Right for the product: only an installed tray writes, Velopack
installs one per user, so the folder should always name that install's exe, and a rewrite after an update or a moved
install is what keeps it working. It bites only tests that run `at_start` on the real profile beside an installed
tray or another test: run those on a scratch profile. No code change.

**Live part so far:** backup made; plugin added by the product's code (17:32:54); capture plugin and monitors set; my
editor window open. Open: live tests 0 to 6, the CLI modes, fixtures, the dry-run app step, docs, clean-up
(Continuation in the Log).
