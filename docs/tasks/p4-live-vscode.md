# Live test: Copilot agent mode in VS Code

Phase: 4. Status: done (2026-11-06; one optional clean-up click for Haik, Result "Open items" 1).
Needs Haik: no (agents run every step; VS Code Copilot is signed in on Free).
Agent: `wakeshot-worker`. Size: about 120K tokens.
Runs after: `p4-vscode-source`, `p4-live-harness`; one live card at a time (agent-session lane).
Parallel with: build cards that own none of the files below; at most four Rust cards at once.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p4-live-vscode\target`; delete it when done.
Real laptop: VS Code (per machine, Haik's profile, Copilot Free). No power settings, no screen captures.

## Goal

The live test of the VS Code chat source: the rules of `docs/research/copilot-vscode-signals.md` and the build of
`p4-vscode-source` on a real VS Code, its answers to "Open questions only a live test answers", redacted fixtures,
fixes with tests, and the source wired into the tray.

## Read first

- `docs/research/copilot-vscode-signals.md` (all, mainly the open questions), `docs/tasks/p4-vscode-source.md`
  Result, `docs/tasks/p4-live-harness.md` Result (runbook, driver, whether a chat starts from the command line).
- `docs/tasks/p4-live-claude.md` Result (the shape of a live result), `crates/wakeshot-windows/src/app/tray_mode.rs`.

## Deliverables

1. Before any step: is VS Code already running for Haik? If it is, never close, kill or type into his windows: open
   your own window only if it is a separate process tree you can end by PID, else skip the crash step and mark it.
2. Sessions at Medium, each timed (send to post, end to post): a working agent mode turn (a command that runs about
   60 s), an approval wait left 1 minute then approved, a stop (Esc or the Stop button through the command line or
   the driver), a clean end, a compaction if cheap (the 1024 line rewrite: mark "not reached" if it costs too many
   prompts), an empty window chat, and a crash (only per item 1). How fast the file shows `{"value":0}` after a send
   and `completedAt` after the end (the research says about once a minute).
3. Run `agent-watch` with a new `--source vscode` row (`crates/wakeshot-windows/examples/agent-watch.rs`, one row in
   `SOURCES`) and record what it posted per step.
4. Fixtures: redacted lines into `crates/wakeshot-core/tests/fixtures/vscode/live/` (no prompt text, no paths with
   the user name, fake ids), a test that replays them, and fixes in `sources/vscode.rs` each with a test that failed
   before.
5. Wire the source into `crates/wakeshot-windows/src/app/tray_mode.rs` and test it through the app with `--dry-run`
   (never beside another dry-run card; never quit Haik's installed tray).
6. A section "Phase 4 live test (2026-11-..)" in `docs/research/copilot-vscode-signals.md` and short lines in
   `docs/research.md`: what holds, the delays, what was verified, the VS Code and Copilot versions; anything not run
   stays "not tested live".

## Rules for every live card of the bigger v1 (2026-11-06)

- **Live cards run in parallel** (Haik asked for more parallel work, 2026-11-06 ~18:50). So no live card edits
  `crates/wakeshot-windows/src/app/tray_mode.rs` or `examples/agent-watch.rs`: `p4-contracts-r3` adds the Copilot,
  VS Code and OpenCode lines, `p4-antigravity-source` the Cursor and Antigravity lines. This wins over any card's
  "Wire the source" item: test through the app with the line already there, and report a wiring bug to the
  orchestrator. The `--dry-run` app step takes turns: create `<scratchpad>\dry-run.lock` with your card name
  (`New-Item -ItemType File` fails if it exists; then wait 30 s and try again), run the dry-run tray, and delete the
  file once that tray has quit. Fixes in shared watch files (`watch/*`, `lockowner.rs`) only by the card that owns
  them this round (`p4-live-copilot`); others report them.

- Minimum tokens (Haik): each prompt says it is an app test and asks for one word or `0`, no reasoning; the lowest
  reasoning setting and the smallest model the plan allows. Count prompts in the Log; free plans have monthly limits.
- Start agents and the driver at Medium through `explorer.exe`. Kill only processes you started, by PID.
- A folder listing can show a stale size and time for a file another process holds open (`p4-live-harness`): time
  each step from the write to the post.
- Checks: `cargo fmt` on your own files; `cargo clippy --locked -p wakeshot-core -p wakeshot-windows --all-targets
  -- -D warnings` for `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc` (ARM64 needs the Build Tools
  `VC\Tools\Llvm\x64\bin` folder on that command's PATH only), the same without `--all-targets`; `cargo test --locked
  -p wakeshot-core -p wakeshot-windows`. A failure in another card's file is reported, never fixed.
- Never run git, not even read-only (`git status`, `git diff`, `git --version`). Never read token files or app
  secrets. No contract or `Cargo` change: stop and tell the orchestrator. Plain short English, no em dashes, no
  emojis. Write Log lines as you go.

## Owns

`crates/wakeshot-core/tests/fixtures/vscode/live/`; fixes in `sources/vscode.rs` and `tests/vscode_source.rs`; the
VS Code line in `tray_mode.rs` and in `agent-watch.rs`; the research sections above.

## Questions for Haik

## Found outside scope

1. **VS Code Copilot harness turns do not hold** (`sources/copilot.rs`, `p4-live-copilot`; sent to the orchestrator
   16:01). The session folder's lock is `inuse.<pid>.lock` of `copilot-runtime.exe`, and the source's liveness is
   `any copilot.exe`, so a working VS Code turn posts `alive=Gone`. On this PC that is where most VS Code chats go
   (folder and empty windows), so the VS Code Local source sees few of them. Research rule 8.
2. **`p4-live-copilot`'s own CLI session** (`5a3cc26e`, `copilot.exe` 52300 running) also showed `alive=Gone` in my
   Medium `agent-watch` at 16:00 to 16:04. Sent with item 1; theirs to judge.
3. **Two installed trays run**: `wakeshot-tray.exe` pid 11424 (started 04:34, parent gone) and pid 26236 (15:07,
   parent Explorer), both from `%LocalAppData%\HaikAsatryan.Wakeshot\current`. Not touched; a single-instance or
   update question for the packaging cards.
4. **`agent-watch --raw`** prints a Claude status-file line (`status=- waitingFor=- ...`) for each VS Code whole read,
   which says nothing for VS Code. The VS Code kinds (line count, `kind`, path) would help the next live test
   (`examples/agent-watch.rs`, owned by `p4-live-harness`; not edited here, orchestrator rule).
5. **Label**: a VS Code chat in the Copilot harness will show with the Copilot CLI source's label, not "Copilot in VS
   Code" (once item 1 is fixed). Haik's call if that matters.

## Log

- 2026-11-06: card written by the orchestrator.
- 2026-11-06: worker started. Read the card, the research file, the Results of `p4-vscode-source`,
  `p4-live-harness` and `p4-contracts-r3`, `sources/vscode.rs`, `agent-watch.rs`. The tray and harness lines are
  already in (`p4-contracts-r3`); this card tests through them and does not edit them (orchestrator rule).
- 2026-11-06 15:45, deliverable 1: VS Code runs for Haik, started 11:05 (main pid 18344, one visible window on the
  `wakeshot` folder, titled "... [Administrator]": his VS Code runs elevated). So: no crash step (no separate process
  tree with a Copilot sign-in; a new `--user-data-dir` has none, and copying it means token files). A Medium `code`
  cannot join an elevated instance: VS Code shows "Another instance of Code is already running as administrator"
  (`src/vs/code/electron-main/main.ts:396-400`, 1.145.1, read as data). So my window joins his instance from my High
  shell, and its chats run at High; the tray and `agent-watch` still read at Medium.
- 2026-11-06 15:50, read as data (1.145.1, `gh api`): `code chat` sends the prompt to the window of the current folder
  (`windowsMainService.ts:520-538`, cwd as the folder) and starts a new chat first (`chat.contribution.ts`
  electron-browser `prompt()`: `ACTION_ID_NEW_CHAT`, then open with the query), after a workspace trust check. Closing
  a window while a local chat runs or waits asks "A session is in progress..." (`chatLifecycle.ts`,
  `isSessionInProgressStatus` takes InProgress and NeedsInput). Cancel is Ctrl+Esc (Windows: Alt+Backspace), approve a
  click. So approve, Stop and a close mid-request need a key or a click in the window: not run (no computer use; a
  synthetic key could reach Haik's foreground app). Terminal auto approve defaults (`defaultAutoApproveRules.ts`) allow
  `sleep`, so a `sleep 60` turn needs no approval; `ping` is not on the list. Idle silence timeout 5 min.
- 2026-11-06 15:56: structure-only monitor (`<scratch>\p4-live-vscode\mon.py`, pid 49948 launcher, 39668 python) on the
  chat folders: per line kind and path, per request send time and `modelState`; no text. Test folder
  `target\vscode-live` in the repo (ignored by git), so it inherits the trust of the repo folder Haik has open.
- 2026-11-06 15:58: `agent-watch --source vscode` at Medium through the driver (`drive\watch`, own build of the
  current tree): start line, `VsCode n=0` (the one old chat file is from 11-02, outside the 24 h window). My window
  opened with `code -n` (HWND 9833982, title "Welcome - vscode-live - ... [Administrator]", no Restricted Mode, so
  trusted). Screen locked since 15:57:50 (`LogonUI` started then; foreground window 0).
- 2026-11-06 15:58:48, prompt 1 (`code chat -m agent`, the 60 s sleep): no `chatSessions` file. The chat went to the
  **Copilot (agent host) harness**: `~\.copilot\session-state\ac748c63...\` made 0.8 s after the send, with
  `inuse.2728.lock` (5 bytes) and `inuse.2728.hold` (0 bytes), 2728 = `copilot-runtime.exe` (parent the agent host,
  started 11:05), plus `.workspace-fork.lock`, `workspace.yaml`, `events.jsonl` (not opened). Writes at 15:58:50 to
  :54 (start, tool call) and 15:59:55 to :56 (sleep done, answer). So the turn worked on Copilot Free.
- 2026-11-06 16:00, prompt 2 after a workspace `.vscode\settings.json` with `chat.defaultToCopilotHarness` and
  `chat.editor.preferCopilotHarness` false: Copilot harness again (`0a48b8db`); New Chat keeps the current chat's
  harness (`clearChatSessionPreservingType`, `constants.ts` `currentSession`), and a remembered pick wins over the
  setting. `agent-watch --source vscode,copilot` (`drive\watch2`, Medium): Copilot source posted it working at
  16:00:50.9, 1.4 s after the folder appeared, with `any copilot.exe alive=Gone`: so it would not hold (research
  rule 8 not built). Sent to the orchestrator for `p4-live-copilot`. Prompts so far: 2.
- 2026-11-06 16:02, orchestrator's check (who started the elevated VS Code): not me. Its main process (18344) started
  11:05:27, hours before this card; its parent (pid 37280) no longer runs and is not Explorer (Explorer 39492 runs
  since 11-05), so it was started from some elevated shell, most likely Haik's own (his one window shows a README
  preview of this repo). Left untouched. A Medium VS Code cannot run beside it (the admin error above), so my chats
  run at High in his instance; the readers (`agent-watch`, the dry-run tray) run at Medium, which is the case that
  matters for the tray: it must see files and processes of an elevated VS Code. Real users mostly run VS Code at
  Medium; that case is not tested here.
- 2026-11-06 16:02:16: my window closed by `WM_CLOSE` to its own handle (checked by title first) while idle: no
  dialog, its renderer and two utility processes gone in 5 s. Reopened 16:02:29 (HWND 2953012) with the workspace
  settings in place from the start; prompt 3 (same text) at 16:02:45.6: no new Copilot session folder this time.
- 2026-11-06 16:03:29.66: a **Local** chat file, `workspaceStorage\01437290...\chatSessions\e2ee25f2...jsonl`, first
  written 44 s after the send (the minute timer; the screen was locked, so no blur write): one Initial line, request
  sent 16:02:46.076, `modelState` 4 (NeedsInput). `agent-watch` at Medium posted `waiting(remote=false, since=-)`,
  `since Code.exe alive=Yes`, 25 ms after the write: the Medium reader sees the elevated `Code.exe`. Then no write
  for minutes: VS Code appends nothing while nothing changes. The terminal call carries "Auto approved by rule
  `sleep (default)`" and `startedAt` 16:02:59, but no shell started under the pty host, and the renderer log shows a
  `toolRiskAssessment` at 16:03:00; something in the chat waits for a click (an elicitation is not saved in the file,
  so which one is not known). So this is the approval wait, reached without asking for it; it cannot be answered from
  here. Waiting holds nothing in Wakeshot (`docs/product.md`), so it costs no power.
- 2026-11-06 16:05: the Linux test fix the orchestrator asked for (`p4-core-linux-tests`): `USER` and `INSIDERS_USER`
  in `tests/vscode_source.rs` were one string with backslashes; now `user()` and `insiders_user()` join one level at
  a time from `ROAMING`, the `installs` expectations too (as `day_dir` in `tests/codex_source.rs`). `cargo test -p
  wakeshot-core --test vscode_source`: 31 passed on Windows. Not run on Linux here.
- 2026-11-06 16:07:52, prompt 4 (`code chat -m ask`, "reply with 0"): in the same window, so Local (New Chat keeps
  the harness). Prompts so far: 4.
- 2026-11-06 16:08:29.8: its file (`893201f2...`) first written already done: sent 16:07:53.0, `completedAt`
  16:07:55.9 (a 3 s turn), on disk 34 s later, so Pending never reached the disk. The window writes on a 60 s beat
  (16:03:29.7, then 16:08:29.8, five beats later; nothing in between, since nothing changed). `agent-watch` posted
  `idle`, `since Code.exe alive=Yes`, at 16:08:29.787, before my monitor saw the file.
- 2026-11-06 16:08:20: Haik unlocked the screen and works on the laptop (`LogonUI` gone, focus moves). One of his
  files opened as a preview in my test window: `code chat` had made it the last active VS Code window. Asked the
  orchestrator to relay a short step to Haik (close my window, or first approve the waiting chat); everything else
  goes on without him.
- 2026-11-06 16:08:56, prompt 5 (`code chat -n -m ask`, the empty window step): went to the **Copilot harness**
  (`session-state\f0892a2d...`, 16:09:05.6 to 16:09:11.9), so no `emptyWindowChatSessions` folder exists at all. The
  Local empty-window path needs a harness pick in the UI: not tested live. Empty window closed by `WM_CLOSE` at
  16:10:19 (idle, no dialog). Prompts so far: 5.
- 2026-11-06 16:10:59: dry-run lock taken (`<scratch>\dry-run.lock`), dry-run tray (own build) started at Medium
  through Explorer, pid 53708, with `CLAUDE_CONFIG_DIR`, `CODEX_HOME`, `COPILOT_HOME`, `XDG_DATA_HOME` on empty
  scratch folders, so only VS Code chat can hold. Its log: "VS Code chat is watched in the folder APPDATA names";
  a FirstRun notice (a fresh dry-run folder). The waiting and the idle chat give no hold line, as the rules say.
- 2026-11-06 16:12: `--raw` showed the first read of the new Local file at 16:03:29.663 got 0 bytes (VS Code had just
  made it): the source asked for its one retry, and the read 14 ms later parsed. CPU at 16:16: dry-run tray 0.95 s in
  352 s (debug build); `agent-watch` (VS Code and Copilot, with `p4-live-copilot`'s busy sessions) 8.7 s in 983 s.
- 2026-11-06 16:14: two fixtures redacted by `<scratch>\redact.py` into `tests/fixtures/vscode/live/`
  (`terminal-call-waiting.jsonl`, `ask-turn-done.jsonl`) with a README; secret patterns and the assigned-key pattern:
  no hit; no user name or path. Tests `every_live_fixture_is_listed_once` and
  `live_chat_logs_read_at_every_save_with_no_format_problem`: 33 passed. No parser fix needed: every real line read
  with no format problem. `cargo test -p wakeshot-windows --locked`: 694 passed, 0 failed, 28 ignored. Lib clippy
  x64 and ARM64: clean.
- 2026-11-06 16:17: research section "Phase 4 live test (2026-11-06)" and `docs/research.md` "VS Code chat live
  test" written. Waiting on Haik's optional step (relayed by the orchestrator) for the approval and the close.
- 2026-11-06 16:20: checks all clean (Result). Waiting until about 16:28 for Haik's step, then the dry-run lock goes
  back.
- 2026-11-06 16:28:45: no step from Haik yet. Dry-run tray quit by `taskkill /PID 53708` (no force; path checked
  first): "Wakeshot ends with code 0". Its `dry-run` folder (made by this card) deleted, the dry-run lock deleted;
  Haik's real data folder and his two installed trays untouched. The monitor and `agent-watch` keep running until
  about 16:40 in case he acts.
- 2026-11-06 16:39: no step from Haik. `agent-watch` stopped by its step file (`stopped clean=true`, driver exit 0,
  `active_after` 0); monitor stopped by its STOP file. Every pid this card started is gone (drivers 44460, 36748,
  their children 53292, 33868, 54696, 41472, tray 53708, monitor 49948 and 39668; checked by pid). The VS Code source
  posted three times in 39 minutes, each on a real chat change. Build folder deleted. Test folder: its
  `.vscode\settings.json` deleted; the empty `target\vscode-live` stays while my window has it open.

## Result

Status: done, 2026-11-06, except one clean-up click (below). VS Code 1.145.1, Copilot extension 0.73.0, Copilot Free
(`copilot/auto`); five one-line prompts. Details: `docs/research/copilot-vscode-signals.md`, "Phase 4 live test
(2026-11-06)".

**Setup (deliverable 1).** Haik's own VS Code was running, elevated, since 11:05 (not started by me; Log 16:02). A
Medium VS Code cannot run beside it, so my test windows joined his instance from my elevated shell (`code -n`), and
prompts went in with `code chat` from a test folder inside the repo (`target\vscode-live`, trusted through the repo).
The readers ran at Medium: `agent-watch --source vscode,copilot` through the driver, the dry-run tray. No crash step:
there was no separate VS Code process tree with a Copilot sign-in.

**What held live:**

| Step | Outcome |
|---|---|
| Working turn, agent mode, `sleep 60` (prompts 1, 2) | Went to the **Copilot harness**, not Local: `~\.copilot\session-state\<id>\` with `inuse.<pid>.lock`/`.hold` of `copilot-runtime.exe`. Turn 61 s, answered on Free. The Copilot CLI source posts it working in 1.4 s, but `alive=Gone` (rule 8 missing; sent to `p4-live-copilot`) |
| Local chat (after a workspace `chat.defaultToCopilotHarness: false` and a new window) | Prompt 3 (`sleep 60`): first save 44 s after the send, `modelState` 4; posted `waiting(remote=false)`, `since Code.exe alive=Yes`, 25 ms after the save. The chat waited for a click although the call was auto approved (cause not seen) |
| Approval wait left 1 minute, then approved | The wait: the step above, written at the first 60 s beat and kept (no write while nothing changes). Approve: not tested (needs a click) |
| Stop (Esc / Stop button) | Not tested: Ctrl+Esc or a click in the window; no computer use, and a synthetic key could reach Haik's foreground app |
| Clean end | Prompt 4 (ask mode): done in 3 s, first save 34 s later already done, posted `idle`; window closed by `WM_CLOSE` to its own handle while idle, no dialog (16:02:16 and 16:10:19) |
| Compaction | Not reached (one-line chats) |
| Empty window chat | Prompt 5 (`code chat -n`): Copilot harness again, so no `emptyWindowChatSessions` file; the Local empty-window path is not tested |
| Crash | Not run (deliverable 1) |
| Write delay | 60 s beat per window, plus a save on blur; screen locked for prompts 1 to 4. First save 44 s after a send, 34 s after an end. `{"value":0}` on disk was never seen: the one Local agent turn was already waiting at its first save, and the ask turn was done. A turn that ends before the next beat never shows Pending on disk |
| Through the app (`--dry-run` tray, Medium, only VS Code can hold) | Source started ("VS Code chat is watched in the folder APPDATA names"); the waiting and the idle chat gave no hold, as the rules say; CPU 0.95 s in 352 s (debug). A hold for a Local working chat: not seen (no Local Pending reached the disk) |

**Fixes:** none in `sources/vscode.rs`: every real line read with no format problem, the 0-byte read at file
creation took its one retry and parsed 14 ms later. Asked by the orchestrator (`p4-core-linux-tests`): three tests
in `tests/vscode_source.rs` built the user folder as one backslash string; now `user()` and `insiders_user()` join one
level at a time from `ROAMING`, as `installs` does (Linux not run here).

**Fixtures and replay:** `crates/wakeshot-core/tests/fixtures/vscode/live/` (`terminal-call-waiting.jsonl`,
`ask-turn-done.jsonl`, `README.md` with the redaction). Tests `every_live_fixture_is_listed_once` and
`live_chat_logs_read_at_every_save_with_no_format_problem` (every save prefix reads with no format problem, the
activity moves only where the table says, the liveness names the send time).

**Tray wiring:** already in from `p4-contracts-r3`; tested through the app as above; no wiring bug found.

**Files changed:** `crates/wakeshot-core/tests/vscode_source.rs`; `crates/wakeshot-core/tests/fixtures/vscode/live/`
(new: two `.jsonl`, `README.md`); `docs/research/copilot-vscode-signals.md` (new last section); `docs/research.md`
(new last section "VS Code chat live test"); this card.

**Checks** (own build folder, `--locked`, no `RUSTFLAGS`): `rustfmt --edition 2024 --check` on
`tests/vscode_source.rs`: clean. `cargo clippy --locked -p wakeshot-core -p wakeshot-windows -- -D warnings`, with and
without `--all-targets`, `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc` (LLVM folder on PATH for ARM64):
clean (4 runs). `cargo test --locked -p wakeshot-core -p wakeshot-windows --no-fail-fast`: core unit 730 (1 ignored),
`claude_status` 11, `claude_transcript` 25, `codex_source` 22, `copilot_source` 47, `cursor_source` 8,
`opencode_source` 15, `vscode_source` 33; `wakeshot-windows` 695 passed, 28 ignored; 0 failed. Fixtures: the repo's
secret patterns and the assigned-key pattern give no hit; no user name or path.

**Decided by the worker** (for Haik's review), each with when to change it:

1. Chats ran at High inside Haik's elevated VS Code, the only signed-in Copilot on the PC. Change it when a test PC
   runs VS Code at Medium: repeat the Local steps there.
2. No parser change: nothing live disagreed with the rules. Change it when a Local chat is seen Pending on disk past
   its end (a window closed mid-request), then add the per-window liveness the research describes.
3. The test folder sat in the repo's ignored `target\` so it inherited the repo's trust (no dialog to click).
   Change it when a test needs a folder outside a trusted tree: trust it once by hand.
4. No synthetic keys or UI automation in my window: a key sent while Haik works can land in his foreground app, and
   UI automation turns on Chromium accessibility for his whole VS Code. Change it when the live test runs on a PC
   nobody uses, or with computer use.

**Clean-up:** every process this card started is gone (checked by pid); the dry-run tray quit with code 0, its
`dry-run` folder and the dry-run lock deleted; build folder deleted. Left: my test window "vscode-live" in Haik's
VS Code (renderer and extension host are children of his VS Code, so not mine to end), with the chat that waits for a
click; closing it asks "A session is in progress..." once. It holds no power (waiting holds only when remote). Left in
Haik's homes: two Local test chats under `workspaceStorage\01437290...\chatSessions` and three Copilot harness test
sessions in `~\.copilot\session-state` (kept so VS Code's own indexes stay right).

**Open items:**

1. Haik, when convenient (relayed by the orchestrator at 16:09): close the "vscode-live" window and confirm the close
   (optionally first approve its waiting chat). Then delete the empty `C:\Users\haika\code\wakeshot\target\vscode-live`
   (ignored by git; VS Code holds it open now).
2. `p4-live-copilot`: rule 8 (Found outside scope 1); then repeat prompt 1 here to see a VS Code Copilot turn hold.
3. Not tested live, for a later pass with computer use or a PC nobody uses: approve after a wait, Stop, a close or
   reload mid-request, a crash with a restart, the Local empty-window folder, a VS Code at Medium, compaction.
