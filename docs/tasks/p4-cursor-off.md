# Cursor out of v1, and the hook folders earlier builds added removed

Phase: 4. Status: done (2026-11-06 23:55). Needs Haik: no.
Agent: `wakeshot-worker`. Size: about 90K tokens.
Runs after: `p6-agents-install-e2e`. Runs alone among Rust cards; beside the video card (`media/` only).
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p4-cursor-off\target`; delete it when done.
Real laptop: yes, at the end: Haik's install updated once more (the way `p6-agents-install-e2e` did it).

## Goal

Haik, 2026-11-06 23:21: "ok le'ts drop cursor as well." Why: a Cursor chat deleted while it runs sends no `stop`,
only the `sessionEnd` that Ctrl+F4 also sends, so the hold lasts until the window closes (`p6-agents-install-e2e`
Result; "No guessing" in `docs/product.md`). v1 agents: Claude Code, Codex CLI, Copilot CLI, Copilot in VS Code
(Copilot harness chats), OpenCode 2.x. With Cursor and Antigravity out, v1 adds no hook folder to any agent.

Haik's installed tray (the `p6-agents-install-e2e` build) already added Wakeshot's Cursor plugin folder to his
profile. Left there, its hook runs on every Cursor prompt (about 0.85 s each) for nothing.

## Read first

- `docs/product.md` ("No guessing", the agents and hooks lines), `docs/agent-hooks.md` (adding, removing, the
  notice), `docs/tasks/p4-vscode-local-off.md` (how Antigravity was unwired), `docs/tasks/p6-agents-install-e2e.md`
  (Result, and how Haik's install was updated).

## Deliverables

1. The tray no longer watches Cursor and adds no hook folder at start. Keep the source code, the hook command and
   their tests for later; they must not run in the app. A test that fails if the app would watch Cursor or add a
   folder again.
2. At start, the tray removes a Wakeshot hook folder an earlier build added (Cursor and Antigravity), through the
   product's own remove path: an unchanged folder is deleted, a folder the user changed is moved aside as today.
   Uninstall keeps removing them too. Tests for both, each failing before. If the "hooks added" notice no longer
   has a use, retire it with the least change; say what you did.
3. No contract or `Cargo` change; if one seems needed, stop and tell the orchestrator.
4. Docs: `docs/product.md` (agents line, hooks line: Cursor out of v1 with the reason and Haik's words),
   `docs/agent-hooks.md` (README lines: v1 adds no hooks), `docs/architecture.md` if it names Cursor as wired,
   `docs/research/cursor-gemini-signals.md` or the Cursor research file and `docs/research.md` short lines.
5. Whole-workspace checks on x64 and ARM64 (`CLAUDE.md`, Checks), then pack x64 and update Haik's install the way
   `p6-agents-install-e2e` deliverable 3 did (WM_CLOSE to `Wakeshot.Loop`, `Setup.exe --silent` at Medium through
   `explorer.exe`, hash check, `config.json` kept, tray started through `explorer.exe`, Medium). Then check:
   Wakeshot's Cursor folder is gone from `%USERPROFILE%\.cursor\plugins\local\`, no Antigravity folder exists, one
   cheap Claude Code turn and one Codex turn still hold and let go, and Haik's lid and sleep values are his own
   (lid Sleep on AC and DC, 18000 on AC and DC) apart from the orchestrator's own Claude hold.

## Out of scope

- `README.md`, `site/`, `media/` (the video card updates the lists).
- Cursor windows and Cursor settings of Haik's: never touch them; only Wakeshot's own plugin folder, through the
  product's remove path.

## Rules

- Never run git, not even read-only. Never kill Haik's tray: only WM_CLOSE. Never run another keep-awake app. Start
  programs at Medium through `explorer.exe`. Kill only processes you started, by PID. Never read token files.
  Prompt text is never logged. Never set `RUSTFLAGS`; cargo may not be on PATH:
  `%USERPROFILE%\.cargo\bin\cargo.exe`; ARM64 needs the Build Tools `VC\Tools\Llvm\x64\bin` folder on that cargo
  command's PATH only.
- Plain short English, no em dashes, no emojis. Write Log lines as you go.

## Questions for Haik

None.

## Found outside scope

1. A Cursor window that was already open when the folder went may keep running the hook from memory until its next
   start or Reload Window (not verified). Each such prompt would run `wakeshot.exe hook cursor <event>` (about
   0.85 s) and write a chat file in `agents\cursor\` that nothing reads; its `stop` deletes it. Harmless; it ends at
   Cursor's next start. Haik may restart Cursor to end it at once.
2. The two Cursor chat files of the install check (`7ae3e396...`, `83337d0e...`, both `working`, 23:06) stay in
   `%LocalAppData%\Wakeshot\agents\cursor\`. The start's in-process removal does not end chats (the CLI's
   `hook cursor remove` does), and nothing reads them in v1. Harmless; Haik may delete the folder.
3. `crates/wakeshot-cli/src/hook.rs` still answers `wakeshot hook cursor <event>` and `hook antigravity <event>`
   (only a why comment added here). Kept on purpose: the frozen command form promises hooks left by any build never
   fail, and the `remove` form ends the chats.
4. `docs/plan.md` (line 302, the v1 agents) and the phase files `docs/phases/04-sources.md` and `06-ship.md` still
   name Cursor (not read line by line here); the orchestrator's files. `README.md`, `site/` and `media/` belong to
   the video card.

## Log

- 2026-11-06 23:22: card written by the orchestrator from Haik's answer.
- 2026-11-06: worker started; reading the docs the card names and the hooks code.
- 2026-11-06 23:3x: scope added by the orchestrator from Haik: "You can leave cursor and antigravity code commented
  out or something like that and with in code comment of reason in short why we discontinued integration." Done as
  the orchestrator set it: the code stays compiled and tested but unwired (commented-out code is not built or tested,
  so it rots), with a one to three line "why" comment in `sources/cursor.rs`, `sources/antigravity.rs`,
  `sources/vscode.rs` (Local chats), where `tray_mode.rs` lists the v1 sources, at the hook add, and at the CLI's
  `hook cursor` and `hook antigravity` commands (`wakeshot-cli/src/hook.rs`, comments only).
- 2026-11-06: code written. `agent_hooks.rs`: `ADDED_AT_START` empty (why lines per agent); `at_start` checks the
  level once, runs the new `retire` (the uninstall's own `remove` for every agent a start does not add; the record
  forgets an `added` mark once the folder is gone), then the add through `start_for` (now without the level, and a
  no-op for no agents); `add_as_an_earlier_build` (tests only) plants Cursor's folder for the removal tests.
  `tray_mode.rs`: `hooked_sources` and the Cursor import gone; `watched_sources` lost its unused data root.
  `startup.rs`: doc and the dev-tray line. `hooks.rs`: its uninstall test plants through the new helper. Why
  comments in the three core sources and the CLI's agent list.
- 2026-11-06 23:35: rustfmt clean on my 8 files; clippy x64 `-p core -p windows -p cli`, with and without
  `--all-targets`: clean; tests x64 for those three: all pass (windows lib 723, core 737 plus integration, cli 103).
- 2026-11-06 23:37: mutation check, the old behavior put back on copies (Cursor in `ADDED_AT_START`, no `retire`,
  Cursor's source watched): 7 of the new or changed tests fail (`a_v1_start_adds_no_hook_folder`,
  `..._removes_the_unchanged_folders...`, `..._moves_a_changed_folder...`, `..._cannot_remove_a_folder...`,
  `the_installed_start_removes_an_old_hook_folder_and_posts_no_notice`, `the_tray_starts_the_v1_sources_only`,
  `a_cursor_chat_holds_nothing_through_the_tray`); the uninstall test and the real-token test pass (no change
  there). Files restored, hashes equal to the copies. Lesson: `shutil.copy2` keeps the old mtime, so cargo kept the
  mutated build; the ignored Medium test's first run used it and failed. Both files touched after.
- 2026-11-06 23:37: ignored `a_medium_start_removes_and_an_elevated_one_does_not` (a child under Explorer's token):
  pass. The Medium child planted Cursor's folder and its start removed it; the elevated runner's start deleted
  nothing. Its report now also carries the child's log lines.
- 2026-11-06 23:45: docs: `product.md` (agents line, a Cursor line with Haik's words, hooks line), `agent-hooks.md`
  (user view, a Cursor "Out of v1" note, Adding, the notice, "Source turned off" built, README lines),
  `architecture.md` (the agents row, the record row, the Cursor source paragraph, section 12),
  `research/cursor-gemini-signals.md` (a top note). Whole-workspace checks started (`checks-ws.ps1`, logs in
  `<scratchpad>\p4-cursor-off\logs-ws\`).
- 2026-11-06 23:40 to 23:45: `cargo fmt --all --check` pass; x64 clippy `--workspace` with and without
  `--all-targets` pass; `cargo test --workspace --locked` pass: 1782 passed, 0 failed, 64 ignored over 22 test
  binaries. Power read only (`logs-live\power-before.txt`, 23:41:59): lid AC 0, DC 0; sleep AC 18000, DC 0; one
  SYSTEM request from the installed tray: its held values, for the orchestrator's Claude session. Before the install:
  Haik's tray 43224 and guard 52816, both Medium; installed exes the `p6-agents-install-e2e` build (`CDB54621...`,
  `F9FDC85D...`); `config.json` `4EBE0C4B...`; `agent-hooks.json` `{"version":1,"cursor":"added"}`;
  `.cursor\plugins\local\wakeshot` holds the fixed set (3 files); `.gemini\config\plugins` empty; two old Cursor
  chat files in `agents\cursor\`.
- 2026-11-06 23:46 to 23:49: x64 release build pass; ARM64 clippy with and without `--all-targets` pass; ARM64
  release build pass (Build Tools `Llvm\x64\bin` first on PATH for those commands only). No `warning` or `error`
  line in any log. No CRT import in any exe. x64 `wakeshot-tray.exe` 3,137,536 bytes `242B9D7A...`, `wakeshot.exe`
  815,616 bytes `ABA496CE...`; ARM64 `wakeshot-tray.exe` 2,765,824 bytes `D70E4060...`, `wakeshot.exe` 781,824 bytes
  `6EECA266...`. Every whole-workspace check green.
- 2026-11-06 23:50: x64 packed (`packaging/pack.ps1 -Version 0.1.0 -Cpu x64`, no delta), exit 0:
  `HaikAsatryan.Wakeshot-win-x64-Setup.exe` 12,481,433 bytes, SHA-256
  `7C47833A53FBAAF581B67D1A26689AC23710852BEDA2B8D47710A09517F72426`; full nupkg 4,781,977 bytes `7DAD63F9...`; the
  package's two exes and the notice icon equal the build's; no staging folder left in TEMP.
- 2026-11-06 23:49:59: read-only monitor started (`monitor.ps1`, pid 4416, 100 ms, into `logs-live\monitor.txt`).
- 2026-11-06 23:50:10: Haik's tray 43224 quit by `WM_CLOSE` to its one `Wakeshot.Loop` window: exit code 0 after
  78 ms, guard 52816 exit code 0. Its log: lidAc 0 to 1, lidDc 0 to 1, sleepDc 0 to 18000, journal deleted,
  "Wakeshot ends with code 0". Read after (23:50:17): lid AC 1, DC 1 (Sleep); sleep after AC 18000, DC 18000; no
  SYSTEM request. Haik's own values.
- 2026-11-06 23:50:31 to 23:50:35: `Setup.exe --silent` at Medium through `explorer.exe` (`start-medium.ps1`, token
  `S-1-16-8192`), exit 0; Velopack 1.2.161 "User chose to overwrite existing installation", install hook ran,
  "Installation completed successfully". Setup did not start the tray. Installed `wakeshot-tray.exe` and
  `wakeshot.exe` equal the build (`242B9D7A...`, `ABA496CE...`); `config.json` same bytes and time (2026-11-05
  20:32:43). The Cursor folder still there before the tray start.
- 2026-11-06 23:50:58.8: tray started through `explorer.exe`: tray 56164 and guard 13804, both Medium, parent
  explorer.exe. Its log: four sources (Claude Code, Codex CLI, Copilot CLI, OpenCode), no Cursor line; at
  23:50:59.114 "agent-hooks: Cursor: .cursor\plugins\local\wakeshot removed"; no Antigravity line (no folder there);
  no notice. After: `.cursor\plugins\local\` empty, nothing named `wakeshot*` in `.cursor\`; `.gemini\config\plugins\`
  empty, nothing named `wakeshot*` in `.gemini\config\`; `agent-hooks.json` `{"version":1}`. "Holding starts: Claude
  Code" at 23:50:59.877 (the orchestrator's session, pid 54092); lid and battery timer written as before.
- 2026-11-06 23:51:30 to 23:51:36: Claude Code, one `claude -p ... --model haiku` turn at Medium through Explorer
  (session pid 38956), answered `ok`, exit 0. The orchestrator's session holds the whole time, so the label never
  stops; per session, `state.json`: busy at 23:51:32.023 (its `statusUpdatedAt`), "Claude Code (2 sessions)" seen
  23:51:32.160; idle at 23:51:35.395, back to "(1 session)" seen 23:51:35.416; its status file gone at exit. Pass.
- 2026-11-06 23:51:49 to 23:51:56: Codex CLI, one `codex exec --skip-git-repo-check -m gpt-6-luna -c
  model_reasoning_effort="low"` turn at Medium (session `01a1b26a`), answered `ok`, exit 0. Rollout `task_started`
  23:51:51.872, "Hold starts: Codex" 23:51:52.172 (0.30 s); `task_complete` 23:51:54.621, "Hold ends: Codex"
  23:51:54.632 (0.01 s). Pass.
- 2026-11-06 23:52:16 (deliverable 5 end): `state.json` "Claude Code (1 session)" only (the orchestrator's session,
  pid 54092); `powercfg /requests` one SYSTEM request, the installed tray's. Read now: lid AC 0, DC 0; sleep AC
  18000, DC 0: the held values for that session. The journal's originals are Haik's values (lidAc 1, lidDc 1,
  sleepDc 18000), and the read after the old tray's quit (23:50:17) showed his values. Tray 56164 and guard 13804
  run at Medium. The two old Cursor chat files still sit in `agents\cursor\` (23:06; nothing reads them now); no new
  one appeared while the monitor ran. Monitor 4416 ended by its stop file at 23:52:39; no process I started runs.

- 2026-11-06 23:55: `docs/research.md` new last section; build folder, `backup\`, `work\` and `before-install\`
  deleted. Kept in `<scratchpad>\p4-cursor-off\`: `pack-x64\`, scripts, `logs*\`, `run\`. Done.

## Result

Status: done, 2026-11-06 23:55.

**What changed**

- Tray (`app/tray_mode.rs`): Cursor is no longer watched. `hooked_sources` and the Cursor import are gone, and
  `watched_sources` lost its unused data root. The v1 tray starts four sources: Claude Code, Codex CLI, Copilot CLI,
  OpenCode. Its doc lists the three agents out of v1, each with a one-line why.
- Agent hooks (`agent_hooks.rs`): `ADDED_AT_START` is empty, so a start adds no folder. `at_start` checks the level
  once, then runs the new `retire`: the uninstall's own `remove` for every agent a start does not add (Cursor and
  Antigravity). Unchanged folder deleted, changed folder moved to `wakeshot-removed`; once it is gone the record drops
  the `added` mark (a `removed` mark stays). Then the add (`start_for`, kept and tested) runs for an empty list.
  Uninstall unchanged.
- The "hooks added" notice: retired with the least change. No v1 start adds a folder, so none posts it; the event,
  the notice, their words and the posting loop in `startup.rs` stay for a later version. No contract or `Cargo`
  change. The dev-tray log line now says "touches no agent's hook folder".
- Why comments (Haik's ask): `sources/cursor.rs`, `sources/antigravity.rs`, `sources/vscode.rs`, `tray_mode.rs`,
  `ADDED_AT_START`, `startup::add_agent_hooks`, the CLI's agent list in `hook.rs`. Code kept compiled and tested,
  not commented out.

**Tests** (7 fail against the old behavior, mutation check in the Log): `a_v1_start_adds_no_hook_folder`,
`a_v1_start_removes_the_unchanged_folders_an_earlier_build_added`,
`a_v1_start_moves_a_changed_folder_out_of_the_agents_plugins`,
`a_v1_start_that_cannot_remove_a_folder_tries_again_at_the_next`,
`the_installed_start_removes_an_old_hook_folder_and_posts_no_notice`, `the_tray_starts_the_v1_sources_only`,
`a_cursor_chat_holds_nothing_through_the_tray`. The link, crafted-marker and not-ours tests now also run the v1
start; the real-token and the ignored Medium test now check the removal (Medium child: pass). Uninstall tests
unchanged and pass.

**Checks**: whole workspace, x64 and ARM64: fmt, clippy with and without `--all-targets`, `cargo test --workspace
--locked` (1782 passed, 0 failed, 64 ignored), release builds; no warning; no CRT import.

**Install** (deliverable 5): Haik's tray 43224 quit by `WM_CLOSE` (code 0, his values back: lid Sleep AC and DC,
18000 AC and DC). `Setup.exe --silent` at Medium through Explorer, exit 0; installed exes equal the build;
`config.json` same bytes. Tray 56164 and guard 13804 at Medium. At start it removed `.cursor\plugins\local\wakeshot`
(one log line, no notice, nothing moved aside); no Antigravity folder; `agent-hooks.json` `{"version":1}`. Claude
Code turn: pass (2 sessions, then 1). Codex turn: pass (hold 0.30 s after `task_started`, end 0.01 s after
`task_complete`). At the end the tray holds only for the orchestrator's Claude session (pid 54092), so lid 0 and
sleep DC 0 now; the journal's originals are Haik's values.

**Setup.exe**: `HaikAsatryan.Wakeshot-win-x64-Setup.exe` 12,481,433 bytes, SHA-256
`7C47833A53FBAAF581B67D1A26689AC23710852BEDA2B8D47710A09517F72426`. Installed `wakeshot-tray.exe` SHA-256
`242B9D7A68BD3568B57A73CEB5D85DA3A8E322914A9C527BD365A101095400A4`, `wakeshot.exe`
`ABA496CED5A2BF564065C894B2FC8F2D4A1FF7A7BC3E15B9164925697E7A413E`.

**Files changed**: `crates/wakeshot-windows/src/agent_hooks.rs`, `crates/wakeshot-windows/src/app/tray_mode.rs`,
`crates/wakeshot-windows/src/app/startup.rs`, `crates/wakeshot-windows/src/hooks.rs` (one test's setup),
`crates/wakeshot-core/src/sources/{cursor,antigravity,vscode}.rs` and `crates/wakeshot-cli/src/hook.rs` (comments
only), `docs/product.md`, `docs/agent-hooks.md`, `docs/architecture.md`, `docs/research/cursor-gemini-signals.md`,
`docs/research.md` (new last section), this card.

**Decided by the worker**, each with when to change it:

1. The record drops `added` instead of writing `removed` after a start removed the folder: `removed` means the user
   said no, and a later version that brings Cursor back should add it with its notice. Change it when a version must
   remember a turn-off.
2. `startup::add_agent_hooks` keeps its name and posting loop: the add path stays ready for a later version. Change
   it when no hooked agent is planned.
3. No log line says Cursor is not supported; the README says it. Change it when users report confusion.

**Open items**: "Found outside scope" 1 to 4 (an open Cursor window may run the hook until its restart; two old chat
files; the CLI still answers the hook words on purpose; plan and phase files still name Cursor).
