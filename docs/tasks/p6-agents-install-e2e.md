# The bigger v1 installed: checks, a fresh install over Haik's, and a hold per agent

Phase: 6. Status: done (2026-11-06 23:12; Part 2 lists one failing case, not fixed here: a Cursor chat deleted
while it runs holds until its window closes, a blocker for Cursor in v1). Needs
Haik: no (he is at the laptop for normal work; the tray quit and start take seconds).
Agent: `wakeshot-worker`. Size: about 150K tokens.
Runs after: `p4-live-cursor`, `p4-live-antigravity`, `p4-live-opencode` (with its poll fix), and the hooks code
review verify. Runs alone: no other card builds, runs an agent or a dry-run tray while it runs.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p6-agents-install-e2e\target`; packaging output under the same folder.
Real laptop: yes. Quits and restarts Haik's installed tray; the new tray holds for real (real power requests and the
lid and battery settings it owns), then restores them.

## Goal

Prove the bigger v1 works as users get it: the whole workspace is green, Haik's install is replaced by the new build
the way an update would, and the installed app holds the PC for each agent and lets go when the agent stops.

## Read first

- `CLAUDE.md` (Checks, ARM64 note, Test laptop), `docs/phases/06-ship.md` work log (2026-11-05 20:48 and 2026-11-06
  ~05:00: how Haik's install was updated before), `packaging/` (the pack script and `e2e.ps1`).
- Each `p4-live-*` card's Result of the bigger v1 (how to start a cheap turn in each agent).

## Deliverables

1. Whole-workspace checks from `CLAUDE.md` on `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc` (fmt; clippy
   with and without `--all-targets`; `cargo test --workspace --locked`; release builds). Any failure: stop and tell
   the orchestrator with the failing test and file; never fix another card's file.
2. Pack the x64 release the way `packaging/` does (Setup.exe and the package), and record its size and hash.
3. Replace Haik's install: quit his tray only by `WM_CLOSE` to its `Wakeshot.Loop` window (it restores its own
   settings; wait for its exit and check the code), run `Setup.exe --silent` at Medium through `explorer.exe`, check
   the installed exe hash equals the build and `config.json` is kept, start the tray through `explorer.exe`, and check
   tray and guard run at Medium.
4. One cheap turn per agent through the installed app, one at a time, each prompt one line asking for one word or
   `0`: Claude Code, Codex CLI, Copilot CLI, Copilot in VS Code (a chat on the Copilot harness; never touch Haik's
   own windows: open your own with `code -n` as `p4-live-vscode` did), OpenCode 2.x, Cursor (editor and, if it
   fires, the CLI), Antigravity (CLI and IDE). For each: the tray's log shows the hold start within the time the live
   card measured and its end after the turn; `powercfg /requests` shows Wakeshot's request during the turn and none
   after; the plugin folders for Cursor and Antigravity were added by the tray with one notice.
5. After the last turn: no Wakeshot request left; the lid action and battery sleep timer equal Haik's values from
   `CLAUDE.md` (Test laptop); the tray still runs.
6. Result: a table (agent, hold start delay, hold end delay, pass or fail), the build hash, anything that failed.
   A failing agent is listed, not fixed here.

## Part 2 changes (orchestrator, 2026-11-06 21:56)

- The v1 agent list is now: Claude Code, Codex CLI, Copilot CLI, Copilot in VS Code (Copilot harness chats only),
  OpenCode 2.x, Cursor (editor only). Antigravity and VS Code's Local chats are out of v1 (`p4-vscode-local-off`);
  check instead that the tray adds no Antigravity folder and does not hold for a Local chat.
- Code changed since Part 1, so deliverables 1 and 2 run again first.
- The Cursor turn uses `p4-live-cursor`'s fixed "Steps for Haik" (the Stop button, not Esc), including the chat
  deleted while it runs (its open item). Set it up, then stop early and reply with exact short steps; the
  orchestrator relays them and resumes you.
- This orchestrator is itself a Claude Code session that works the whole time, so the new tray holds for it from
  the start. Judge each agent's hold by the tray log lines for that agent and session, never by "no request at all";
  name the orchestrator's own hold in the Result.

## Rules

- Never run git, not even read-only (`git status`, `git diff`, `git --version`).
- Never kill Haik's tray: only `WM_CLOSE`. Never run another keep-awake app. Kill only processes you started, by PID.
- Never read token files. Start agents and the driver at Medium through `explorer.exe`.
- Never set `RUSTFLAGS`. Cargo may not be on PATH: `%USERPROFILE%\.cargo\bin\cargo.exe`.
- Plain short English, no em dashes, no emojis. Write Log lines as you go.

## Questions for Haik

None.

## Steps for Haik (Part 2, Cursor; about 6 minutes)

The new Wakeshot is installed and running. A Cursor window "wakeshot-cursor-test" is already open for this test. Use
only that window, never "Cursor Agents". The tray icon shows a hold all the time (the orchestrator's own Claude
session), so you do not need to watch it: the log has the times.

1. Unlock the PC. Note whether a Wakeshot notice about Cursor shows, and how many times.
2. Switch to the Cursor window "wakeshot-cursor-test". If it asks whether you trust the folder, say yes. If the chat
   is not open, open it (Ctrl+L).
3. Send `This is an app test. Reply with one word: ok`. Wait for the answer.
4. Send `This is an app test. Count from 1 to 10000, one number per line.` About 3 seconds after the numbers start,
   click the Stop button (the square at the chat box). Not Esc.
5. Send `This is an app test. Count from 1 to 3000, one number per line.` When the numbers start, press Ctrl+F4 (the
   chat tab closes, the turn goes on unseen). Wait 1 minute.
6. Open a new chat and send the 3000 count again. When the numbers start, open Show Chat History (the clock icon at
   the chat tabs) and delete that running chat. Wait 2 minutes.
7. Close the window with its X.
8. Tell the orchestrator "done", with anything Cursor showed and the times you saw (roughly is fine).

## Found outside scope

1. The tray log names only label changes (`core.rs` `log_changes`): a second session of an agent that already holds
   (here any Claude Code turn beside the orchestrator's session) gives no log line. Only `state.json` ("Claude Code
   (2 sessions)") shows it. Not a bug; a diagnostics gap if a user report ever needs per-session times.
2. The new tray's start changed `opencode.db-shm`'s write time (22:04:34): the first SQLite reader rebuilds the WAL
   index, as `p4-live-opencode` noted for leftover files. The db and `-wal` were not written. Harmless.
3. `p4-live-vscode`'s open item: the window it opened ("vscode-live", in Haik's VS Code) is still open and Haik now
   uses it; `target\vscode-live` still exists. Not touched.
4. A Git `tail.exe` (pid 51104, since 18:18:50, parent gone) follows Part 1's `logs\summary.txt`. Not started in this
   run, so not killed; most likely a watch from Part 1's session (not verified). Harmless; the orchestrator may end it.
5. Left in Haik's Cursor profile by the test window (Cursor's own files): the folder
   `.cursor\projects\c-Users-haika-AppData-Local-Temp-...-work-wakeshot-cursor-test` and the test chats in Cursor's
   history. Haik may delete them.
6. Cursor's hooks log says "ERROR: Failed to parse project hooks configuration" for the test folder, which has no
   hooks file. Cursor's message; our hooks ran.

## Log

- 2026-11-06: card written by the orchestrator.
- 2026-11-06: Part 1 started (deliverables 1 and 2 only, by the orchestrator's brief). Part 1 does no install, no
  tray start, no agent runs, no `wakeshot hook` add or remove, and nothing in `.cursor` or `.gemini`: the Cursor and
  Antigravity live cards still wait for Haik, and a new tray's `agent_hooks::at_start` would rewrite their plugin
  folders. Build folder: `<scratchpad>\p6-agents-install-e2e\target`, kept for Part 2.
- 2026-11-06: note for Part 2, from the orchestrator: one installed tray runs, not two. At 18:25 it was tray 11424
  with its guard 7664 (`--guard 11424 ...`); the 26236 seen earlier was most likely a guard too (not verified).
  `WM_CLOSE` goes to the one tray's `Wakeshot.Loop` window; the tray ends its own guard.
- 2026-11-06: toolchain cargo and rustc 1.99.1 (pinned), `vpk` 1.2.161 at `%USERPROFILE%\.dotnet\tools`, no
  `RUSTFLAGS`, no `VPK_*` variable set. ARM64 steps put `C:\Program Files (x86)\Microsoft Visual
  Studio\18\BuildTools\VC\Tools\Llvm\x64\bin` first on PATH for those cargo commands only. Script and logs in
  `<scratchpad>\p6-agents-install-e2e\` (`checks.ps1`, `logs\`).
- 2026-11-06: power read only, before the checks (`logs\power-before.txt`): Balanced active; lid action AC 0 and DC
  0 (Do nothing); sleep after AC 18000, DC 0 (Never). Not Haik's own values from `CLAUDE.md` (lid Sleep, DC 18000):
  most likely the installed tray holding for the live Claude Code sessions (not verified). Part 1 writes no power
  setting. Checks started in the background, one step after the other, stop at the first failure.
- 2026-11-06 18:18 to 18:22: `cargo fmt --all --check` pass; x64 clippy `--all-targets` pass (73 s), x64 clippy
  libs only pass (all four crates checked); `cargo test --workspace --locked` pass (176 s): 1752 passed, 0 failed,
  64 ignored (the real-laptop and dry-run tray suites), over 19 test binaries; wakeshot-core lib 730, wakeshot-windows
  lib 716, wakeshot-cli 101.
- 2026-11-06 18:24: x64 `cargo build --release --workspace --locked` pass (112 s). The `release.yml` CRT check by
  hand (`crt-check.ps1`, dumpbin `/dependents`): no Visual C++ runtime import in either exe. `wakeshot-tray.exe`
  3,202,048 bytes, SHA-256 `16E41459E15E39477248FD09D5555B2CB4A3E8FE9C54E42D68D1842EDCD3A8A9`; `wakeshot.exe`
  806,912 bytes, SHA-256 `3A87DDA815CDC746D202ECC3E9FEB843FB43AFEC64C51BCD2B6EA947739B3BCF`.
- 2026-11-06 18:25 to 18:26: ARM64 clippy `--all-targets` pass (36 s; `ring` compiled with the Build Tools clang),
  ARM64 clippy libs only pass, ARM64 release build pass (79 s). No CRT import; `wakeshot-tray.exe` 2,823,168 bytes,
  SHA-256 `980FE828611A9006180584C75CDE4F6E3D502C753B216BE06E580817F0AB4205`; `wakeshot.exe` 773,120 bytes, SHA-256
  `BE7650E86356E379F4BAEB5C0FE093E25A8C6DF68590242AF41F31102BCC559F`. No `warning` or `error` line in any log.
  Deliverable 1 done: every check green on both targets.
- 2026-11-06 18:27: x64 packed with `packaging/pack.ps1 -Version 0.1.0 -Cpu x64 -ExeDir
  <target>\x86_64-pc-windows-msvc\release -OutDir <scratchpad>\p6-agents-install-e2e\pack-x64 -Icon
  assets/wakeshot.ico` (no `-PreviousDir`: no earlier release, so no delta), exit 0. Version 0.1.0 is the
  `Cargo.toml` version and the one Haik has installed (`06-ship.md` 2026-11-05 20:48 and 2026-11-06 04:35).
  The exes are those of the deliverable 1 workspace release build, the same build `release.yml` packs.
- 2026-11-06 18:28: `HaikAsatryan.Wakeshot-win-x64-Setup.exe` 12,508,502 bytes, SHA-256
  `BF5A309AE1713F08BF99976DD4CD5A2F1A4AF860C02567CF3480307165CA1EB5`. `HaikAsatryan.Wakeshot-0.1.0-win-x64-full.nupkg`
  4,809,046 bytes, SHA-256 `1D5E30192C90D1822BA1DC0433D57353D1E170BFB0D6708F332894E124D15DDD`. Portable zip
  4,727,206 bytes; `releases.win-x64.json`, `assets.win-x64.json`, `RELEASES-win-x64` written. Inside the package
  (`pack-check.ps1`, read as a zip): `lib/app/wakeshot-tray.exe`, `wakeshot.exe` and `wakeshot-notice.png` have the
  same SHA-256 as the build and `assets/wakeshot-notice.png`. pack.ps1's TEMP staging folder is gone. Setup.exe not
  run, nothing installed.
- 2026-11-06 18:28: power read again (`logs\power-after.txt`): same bytes as before the checks. Part 1 stopped here,
  at the brief's clean point; the build folder and `pack-x64` stay for Part 2.
- 2026-11-06 21:57: Part 2 started, by the "Part 2 changes" above. Order: deliverables 1 and 2 again (code changed),
  then the install (3), one turn per v1 agent (4), stop for Haik's Cursor steps, then 5 and 6. Runs alone. Haik's tray
  per the orchestrator: pid 11424, guard 54868.
- 2026-11-06 21:59: power read only (`logs-p2\power-before.txt`, `powercfg /qh`: the lid action is a hidden setting,
  `/q` printed no value for it this time): lid AC 0, DC 0; sleep after AC 18000, DC 0. The installed tray holds (the
  orchestrator's Claude session), so these are its held values, not Haik's own. Checks started in the background
  (`checks-p2.ps1`, logs in `logs-p2\`), same steps as Part 1, stop at the first failure.
- 2026-11-06 21:58 to 22:02: deliverable 1 again, all green. fmt pass; x64 clippy `--all-targets` and libs only pass
  (all four crates checked again); `cargo test --workspace --locked` pass (96 s): 1779 passed, 0 failed, 64 ignored
  over 22 test binaries (core lib 737, windows lib 720, cli 103); x64 release build pass (55 s); ARM64 clippy both ways
  pass, ARM64 release build pass (49 s). No `warning` or `error` line in any log. No CRT import in any exe. x64
  `wakeshot-tray.exe` 3,168,256 bytes, SHA-256 `CDB5462155A78ABE2918C77A83317433A6C5ECCCF155D84B041885AC12CDF408`;
  `wakeshot.exe` 815,616 bytes, `F9FDC85D497709A084F6F9C38C8F3FB88BAA5160B82CEF55377A2926577BD15F`. ARM64
  `wakeshot-tray.exe` 2,789,376 bytes, `228DC4AB34CA585C11291E35EA1E25BC56BC54734D11907CE88275D44CAE2E39`;
  `wakeshot.exe` 781,824 bytes, `72074079292BC07026FFE5C01B98CAB16554508150807EFD2F03269FBF6C2EE7`.
- 2026-11-06 22:03: deliverable 2 again: Part 1's `pack-x64` deleted, the same `pack.ps1` command packed the new build
  (version 0.1.0, no delta), exit 0 (`logs-p2\pack-x64.log`). `HaikAsatryan.Wakeshot-win-x64-Setup.exe` 12,495,450
  bytes, SHA-256 `F853722CABDD6251BF93918F1B381CE0C964E1BD8222FAD15F3971B20A8FCB16`; full nupkg 4,795,994 bytes,
  `908E09DD42B6214D47C9057463D39ADA22087A6C9D636437786DAA49E0C18300`; portable zip 4,714,154 bytes. The package's
  `wakeshot-tray.exe`, `wakeshot.exe` and `wakeshot-notice.png` are the build's bytes; no staging folder left in TEMP.
- 2026-11-06 22:03: before the install: tray 11424 and guard 54868 both Medium (`level.ps1`); installed
  `wakeshot-tray.exe` 2,907,648 bytes `497A5ACF...`, the 04:34 build; `config.json` SHA-256 `4EBE0C4B...` (Automatic on,
  written 2026-11-05 20:32) copied to `before-install\`; Run value `Wakeshot` names the installed tray;
  `.cursor\plugins\local` empty, `.gemini\config\plugins` empty, no `agent-hooks.json` in the data folder. Monitor
  (`monitor.ps1`, pid 53600, reads only, 100 ms) started into `logs-p2\monitor.txt`.
- 2026-11-06 22:03:51: tray 11424 quit by `WM_CLOSE` to its one `Wakeshot.Loop` window (`quit-tray.ps1`): exit code 0
  after 81 ms, guard 54868 exit code 0. Its log: lidAc 0 to 1, lidDc 0 to 1, sleepDc 0 to 18000, journal deleted,
  RunOnce removed, "Wakeshot ends with code 0". Read after (`powercfg /qh`): lid AC 1, DC 1 (Sleep); sleep after AC
  18000, DC 18000: Haik's own values. `powercfg /requests`: no Wakeshot request.
- 2026-11-06 22:04:12 to 22:04:16: `Setup.exe --silent` at Medium through `explorer.exe` (`start-medium.ps1`, token
  `S-1-16-8192`), exit 0. Velopack 1.2.161: "User chose to overwrite existing installation", install hook ran (Run
  value set, notice app id registered, no journal to restore), "Installation completed successfully". Setup did not
  start the tray. Installed `wakeshot-tray.exe` and `wakeshot.exe` SHA-256 equal the build's (`CDB54621...`,
  `F9FDC85D...`); `config.json` same bytes and time as before (`4EBE0C4B...`, 2026-11-05 20:32:43).
- 2026-11-06 22:04:34: tray started through `explorer.exe`: tray 43224 and its guard 38456, both Medium (parent
  explorer.exe). Its log: five sources (Claude Code, Codex CLI, Copilot CLI, OpenCode, Cursor), no VS Code or
  Antigravity line; "Cursor: hook folder added (.cursor\plugins\local\wakeshot)" (3 files and the marker);
  `agent-hooks.json` `{"version":1,"cursor":"added"}`; `.gemini\config\plugins` still empty (no Antigravity folder).
  "Holding starts: Claude Code" at 22:04:35.473 (the orchestrator's session, pid 54092), lid and battery timer written
  as before. The `HooksAdded` notice is not shown yet: the session is locked (LogonUI since 21:59:24), and the core
  keeps notices for the unlock (track note 18). Checked again after Haik's Cursor steps.
- 2026-11-06 22:05 to 22:06: Claude Code 2.1.319, two one-shot turns (`claude -p ... --model haiku`, Haiku, at Medium
  through Explorer, in `work\claude`), each answered `ok`, exit 0. The orchestrator's session (pid 54092) holds the
  whole time, so the tray's label "Claude Code" never stops holding and the log gets no line for a second session
  (the core logs only label changes). The per-session evidence is the tray's own `state.json`: turn 2 (session pid
  57084) busy at 22:06:08.726 (its `statusUpdatedAt`), `holding` "Claude Code (2 sessions)" seen 22:06:08.779;
  idle at 22:06:10.161, back to "(1 session)" seen 22:06:10.192; its status file gone at exit. Turn 1 (pid 33728)
  the same within the monitor's 100 ms. Monitor restarted at 22:05:54 as `monitor2.txt` (adds status times and a
  `powercfg /requests` read every 2 s; first monitor pid 53600 ended by its stop file).
- 2026-11-06 22:06: Codex CLI, one `codex exec --skip-git-repo-check -m gpt-6-luna -c model_reasoning_effort="low"`
  turn at Medium (session `01a1b20a`), answered `ok`, 3,729 tokens. Rollout `task_started` 22:06:34.663, tray log
  "Hold starts: Codex" 22:06:34.706 (43 ms); `task_complete` 22:06:38.590, "Hold ends: Codex" 22:06:38.602 (12 ms);
  `state.json` showed "Codex (1 session)" beside Claude Code in between. My `.cmd` called `codex.cmd` without `call`,
  so cmd ended inside it and wrote no end time or exit code (my script's bug, fixed with `call`); the Codex tree is
  gone (no process with the run's command line). An update check ran at 22:09:35: "no update found".
- 2026-11-06 22:10: Copilot CLI 1.0.106 through VS Code's shim on PATH, one `copilot -p ... -s --trust-folder` turn at
  Medium (Copilot Free, auto model; session `634ffd71`), answered `ok`, exit 0. `user.message` 22:10:54.518, "Hold
  starts: Copilot" 22:10:54.660 (142 ms); `assistant.turn_end` 22:10:56.794, "Hold ends: Copilot" 22:10:56.813
  (19 ms); `session.shutdown` 22:10:56.815. No process of the run left.
- 2026-11-06 22:11 to 22:12: OpenCode 2.2.5, one `opencode run --model ollama/qwen3:4b-instruct` turn at Medium in
  `work\opencode` (own `opencode.jsonc` as the harness card's: local model, tools ask), Haik's real data home (the one
  the tray watches), answered `ok`, exit 0. Its row (read with `oc-times.py`, SQLite `mode=ro`, times only):
  `time_busy` 22:11:43.996, "Hold starts: OpenCode" 22:11:46.009 (2.01 s); `time_idle` 22:12:11.464, "Hold ends:
  OpenCode" 22:12:13.095 (1.63 s). Both inside the 2 s `-shm` poll of `p4-live-opencode` (its one-shot measured
  0.21 s and 0.79 s; a poll can take up to 2 s by its design). The `run` started the background service (pid 47584,
  22:11:42); stopped with `opencode service stop` at Medium (exit 0), gone. No WARN or "format problem" line in the
  tray log since its start.
- 2026-11-06 22:14: Copilot in VS Code 1.145.1, Copilot Free: `code chat -n -m agent` (one prompt) from my elevated
  shell, so a new empty window of my own joined Haik's elevated VS Code (a Medium `code` cannot join it, as
  `p4-live-vscode`); his windows were never touched. It ran on the Copilot harness (session `8626228f`, in-use files
  of Haik's `copilot-runtime.exe` 2728, elevated). `user.message` 22:14:28.869, "Hold starts: Copilot" 22:14:29.009
  (140 ms); `assistant.turn_end` 22:14:32.163, "Hold ends: Copilot" 22:14:32.226 (63 ms). The Medium tray found the
  elevated holder. My window (handle 2098752, "Welcome") closed by `WM_CLOSE` at 22:14:48 while idle, no dialog; no
  hold change after. The session keeps `inuse.2728.*` (the runtime lives on in Haik's VS Code) and reads idle, as
  `p4-live-vscode-stop` saw; one pre-warmed session `ba42aa5f` the same.
- 2026-11-06 22:15: VS Code Local chat: not sent live. With a one-word prompt a Local turn ends before VS Code's first
  save (34 to 44 s after a send, `p4-live-vscode`), so it never shows working on disk and cannot tell a wired source
  from an unwired one; a longer Local turn breaks the one-word rule, and a Local terminal call waits for a click in
  Haik's VS Code. Evidence instead: the installed tray's start lines name five sources and no VS Code one, and the
  test `a_local_vs_code_chat_holds_nothing_through_the_tray` passed in deliverable 1.
- 2026-11-06 22:05:54 to 22:16:17: `powercfg /requests` (every 2 s, `monitor2.txt`): SYSTEM held by the installed
  `wakeshot-tray.exe` the whole time (one request, the only one): during every turn above, and after each, for the
  orchestrator's session.
- 2026-11-06 22:16: Cursor set up. Monitor restarted detached (`monitor3.txt`, pid 56112, reads only, also lists the
  hook's chat files in `agents\cursor\` by short name, state and write time; ends by itself at 02:16 or by its stop
  file). Test folder `work\wakeshot-cursor-test` (one `note.txt`); `cursor -n <folder>` at Medium through Explorer
  (exit 0) opened the window "wakeshot-cursor-test - Cursor" (handle 787766) in Haik's running Cursor (pid 49860,
  Medium, since 14:29); its new extension hosts started 22:16:36. Haik's "Cursor Agents" window untouched. The
  session is still locked, so the steps wait for Haik. Status blocked.
- 2026-11-06 23:08: resumed. Haik (relayed): steps 1 to 5 as written; step 6 twice, the first chat "not deleted in
  time", the second deleted while the numbers ran; no Cursor message or notice count reported.
- 2026-11-06 22:17:53 to 22:55:48 (`monitor3.txt`): the orchestrator's session went idle while it waited, so the tray
  let go ("Holding ends; held by Claude Code, Codex, Copilot, OpenCode"), wrote lid 0 to 1, 0 to 1 and sleepDc 0 to
  18000, and `powercfg /requests` showed no Wakeshot request; every hold end today wrote back Haik's values. The
  `HooksAdded` notice was shown once, at 22:54:05.553, after the unlock (one line `notice shown: HooksAdded`).
- 2026-11-06 23:04 to 23:08, Cursor 3.27.12 in the test window (`cursor-hooks.ps1` reads Cursor's hooks output log
  for times, ids and status fields only; `logs-p2\cursor-hooks.txt`):
  - Step 3, one word (chat `83337d0e`): `beforeSubmitPrompt` requested 23:04:22.702 (the hook ran 2279 ms, first
    prompt in the window), chat file 23:04:24.895, "Hold starts: Cursor" 23:04:24.910 (2.21 s); `stop` `completed`
    23:04:27.478, "Hold ends: Cursor" 23:04:28.355 (0.88 s).
  - Step 4, Stop button: prompt 23:04:33.246, hold 23:04:34.198 (0.95 s); `stop` `aborted` 23:04:45.135, hold ends
    23:04:45.992 (0.86 s). Exact.
  - Step 5, Ctrl+F4: prompt 23:04:59.066, hold 23:05:00.008 (0.94 s); `sessionEnd` `generating` `user_close` at
    23:05:06.394 kept it working (right: the turn ran on); its own `stop` `completed` 23:05:54.261, hold ends
    23:05:55.194 (0.93 s). Exact.
  - Step 6, first try (new chat `7ae3e396`): prompt 23:06:29.902, hold 23:06:30.816 (0.91 s). Haik did delete it:
    Cursor's agent host log at 23:06:42.522 "Cancel action received", `conversation_action_abort`, at 23:06:42.669
    "Deleted agent host session 7ae3e396..."; the renderer released its wakelock "generation-ended". The hook got
    only `sessionEnd` `generating` `user_close` (23:06:42.448), no `stop`.
  - Step 6, second try (sent in chat `83337d0e`): prompt 23:06:50.163, `state.json` "Cursor (2 sessions)" 23:06:51.137
    (0.97 s); deleted at 23:06:54.33 (renderer "watchSessions.deleted 83337d0e", wakelock released
    "generation-ended"); hook `sessionEnd` `generating` `user_close` 23:06:54.248, no `stop`.
  - Both chat files stayed `working`, naming the window's extension host 54692. Step 7: the window shut down at
    23:08:23.674, extension host 54692 "exiting with code 0" 23:08:23.700, "Hold ends: Cursor" 23:08:23.736 (36 ms
    after the exit). So the deleted chats held 101 s and 89 s past their deletes, until the window closed: not exact.
  - `cursor-session-ends.ps1` (field names and short values only): the second delete's `sessionEnd` has the same
    fields and values as step 5's Ctrl+F4 one (`reason` `user_close`, `final_status` `generating`, `duration_ms` 0, a
    transcript path), only its `generation_id` differs; the first delete differs in `duration_ms` 19152 and
    `transcript_path` null. The transcript file of `83337d0e` still exists. No exact signal for a delete in what the
    hook gets.
- 2026-11-06 23:11 (deliverable 5): the orchestrator's session (pid 54092, session `2a1d3942`) works, so the tray
  holds for it alone: `state.json` "Claude Code (1 session)", `powercfg /requests` one SYSTEM request from the
  installed `wakeshot-tray.exe`, no Cursor or other agent hold. Read now (`powercfg /qh`, `logs-p2\power-end.txt`): lid
  AC 0, DC 0, sleep AC 18000, DC 0, the held values; the journal's originals are lidAc 1, lidDc 1, sleepDc 18000 (Haik's
  values), and every restore today wrote them back (22:03:51, 22:17:53, 22:17:59, 22:55:52). Tray 43224 and guard
  52816 run at Medium.
- 2026-11-06 23:12: clean-up. Monitor 56112 ended by its stop file; no process I started runs. Deleted my build
  folder `target\` and `work\` (test folders). Kept in the scratch folder: `pack-x64\`, scripts, `logs-p2\`, `run\`.
  The two Cursor chat files stay in `%LOCALAPPDATA%\Wakeshot\agents\cursor\` (they name a dead process, so they read
  ended). `docs/research.md` got a section "Install check".

## Result

### Part 1 (deliverables 1 and 2), done 2026-11-06 18:28

- Checks, all green, on `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`: `cargo fmt --all --check`; clippy
  with and without `--all-targets` (`--locked`, `-D warnings`) on both targets; `cargo test --workspace --locked`
  (1752 passed, 0 failed, 64 ignored); `cargo build --release --workspace --locked` on both targets. No warning in
  any log. Both targets' exes import no Visual C++ runtime.
- Code built: the working tree as this session found it. The session's start snapshot named HEAD `cd140e9` with a
  clean tree (not checked with git, by the rules).
- x64 pack (`packaging/pack.ps1`, version 0.1.0, no delta) in
  `<scratchpad>\p6-agents-install-e2e\pack-x64`: `HaikAsatryan.Wakeshot-win-x64-Setup.exe` 12,508,502 bytes,
  SHA-256 `BF5A309AE1713F08BF99976DD4CD5A2F1A4AF860C02567CF3480307165CA1EB5`. The package's exes are the build's
  bytes. Part 2 checks the installed exe against the build's `wakeshot-tray.exe` SHA-256
  `16E41459E15E39477248FD09D5555B2CB4A3E8FE9C54E42D68D1842EDCD3A8A9`.
- Kept for Part 2: the build folder `<scratchpad>\p6-agents-install-e2e\target`, the pack, and the scripts
  `checks.ps1`, `crt-check.ps1`, `pack-check.ps1` with `logs\`.
- Not done (by the brief): no Setup.exe run, no install, no tray start, no agent run, no `wakeshot hook`, nothing in
  `.cursor` or `.gemini`, no power write (power read before and after: the same).
- Files changed: this card only.
- Open: deliverables 3 to 6. They wait for the Cursor and Antigravity live cards, because a new tray's
  `agent_hooks::at_start` would rewrite their plugin folders.

### Part 2 (deliverables 1 to 6 on the new code), done 2026-11-06 23:12

Status: done, with one failing case listed (Cursor, a chat deleted while it runs), not fixed here.

**Deliverable 1 again:** all green on `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`: fmt; clippy with and
without `--all-targets`; `cargo test --workspace --locked` 1779 passed, 0 failed, 64 ignored; release builds; no
Visual C++ runtime import. **Build hash** (x64 `wakeshot-tray.exe`, the installed one):
`CDB5462155A78ABE2918C77A83317433A6C5ECCCF155D84B041885AC12CDF408`; `wakeshot.exe`
`F9FDC85D497709A084F6F9C38C8F3FB88BAA5160B82CEF55377A2926577BD15F`.

**Deliverable 2 again:** `HaikAsatryan.Wakeshot-win-x64-Setup.exe` 12,495,450 bytes, SHA-256
`F853722CABDD6251BF93918F1B381CE0C964E1BD8222FAD15F3971B20A8FCB16`; full nupkg 4,795,994 bytes; the package holds the
build's exes. Kept in `<scratchpad>\p6-agents-install-e2e\pack-x64`.

**Deliverable 3:** Haik's tray 11424 quit by `WM_CLOSE` (code 0 after 81 ms, guard code 0, his values back: lid
Sleep AC and DC, sleep after 18000 AC and DC, no Wakeshot request). `Setup.exe --silent` at Medium through Explorer,
exit 0 (Velopack overwrote the same version). Installed exes equal the build; `config.json` same bytes. Tray 43224
started through Explorer; tray and guard at Medium. It added Cursor's hook folder (one log line, one `HooksAdded`
notice, shown at the unlock) and no Antigravity folder; it watches five sources, no VS Code one.

**Deliverable 4** (the orchestrator's Claude session held most of the time, so each agent is judged by its own log
lines, or `state.json` for a second Claude session; delays from the agent's own record to the tray):

| Agent | Turn | Hold start delay | Hold end delay | Result |
|---|---|---|---|---|
| Claude Code 2.1.319 | `claude -p`, Haiku | 0.05 s (`state.json` 2 sessions) | 0.03 s (back to 1) | pass |
| Codex CLI 0.167.0 | `codex exec`, `gpt-6-luna` low | 0.04 s | 0.01 s | pass |
| Copilot CLI 1.0.106 | `copilot -p`, Free | 0.14 s | 0.02 s | pass |
| OpenCode 2.2.5 | `opencode run`, local model | 2.01 s | 1.63 s | pass (inside its 2 s poll) |
| Copilot in VS Code 1.145.1 | harness chat, own window | 0.14 s | 0.06 s | pass |
| VS Code Local chat | not sent (Log 22:15) | none | none | pass by design: no VS Code source, its test passed |
| Cursor 3.27.12, one word | editor chat | 2.21 s (first prompt in a new window) | 0.88 s | pass |
| Cursor, Stop button | editor chat | 0.95 s | 0.86 s | pass, exact |
| Cursor, tab closed (Ctrl+F4) | editor chat | 0.94 s | 0.93 s after its own `stop` | pass, exact |
| Cursor, chat deleted while it runs (twice) | editor chat | 0.91 s, 0.97 s | 101 s and 89 s, at the window close | **fail: not exact** |
| Cursor, window closed | editor | none | 0.04 s after the extension host exit | pass |

Cursor delays count from Cursor's hook request; about 0.9 s of each is the hook's own PowerShell start (known,
`p4-live-cursor`).

**The deleted chat, under "No guessing": the hold did not end exactly.** Deleting a running chat cancels its turn
(Cursor's agent host: "Cancel action received", "Deleted agent host session <id>"; its own wakelock released
"generation-ended"), but the hook gets no `stop`, only a `sessionEnd` with `final_status` `generating` and `reason`
`user_close`. That is what a tab closed with Ctrl+F4 sends, whose turn runs on and does send `stop`; the second
delete's fields matched it except the turn id. Wakeshot keeps such a chat working (`EndUnlessRunning`, right for
Ctrl+F4), so the hold lasted until the window closed: 101 s and 89 s here, and as long as the window stays open in
real use. **This is a blocker for Cursor in v1.** No timer or cap added. What is known for the fix card: no field in
the hook input tells a delete from a tab close; the chat's transcript file stays after a delete; Cursor's own window
logs name the delete (not a contract). Product choice for Haik: drop Cursor from v1, or ship it with this gap in the
README.

**Deliverable 5** (23:11): no hold left but the orchestrator's own Claude Code session (pid 54092, session
`2a1d3942`): `state.json` "Claude Code (1 session)", `powercfg /requests` one SYSTEM request from the installed tray,
none for Cursor or another agent. The lid and battery timer read now are the held values (lid 0 and 0, sleep DC 0)
because that session still works; the journal's originals are Haik's values (lid 1 and 1, sleep DC 18000), the read
after the old tray's quit showed his values (lid Sleep AC and DC, 18000 AC and DC), and every hold end today wrote
them back (22:03:51, 22:17:53, 22:17:59, 22:55:52). The tray still runs (43224, guard 52816, Medium).

**Checks run:** the whole-workspace checks (green), the pack check, the install checks (hashes, config bytes, levels,
folders), one live turn per agent with the monitor, `powercfg /requests` every 2 s, `powercfg /qh` before, after the
quit and at the end.

**Files changed:** this card; `docs/research.md` (new last section "Install check"). No code change.

**Clean-up:** every process I started is gone (monitors 53600, 31424, 56112; the `.cmd` runs; the OpenCode service
47584 stopped by `opencode service stop`); my VS Code window closed by `WM_CLOSE`; the Cursor window closed by Haik.
Build folder and `work\` deleted. Kept: `pack-x64\`, scripts, `logs-p2\`, `run\`.

**Open items:**

1. Cursor, a chat deleted while it runs: the blocker above; the orchestrator decides the fix card or Haik's product
   call.
2. Left in Haik's homes by the test turns (the apps' own files, kept so their indexes stay right): one Codex rollout,
   one Copilot CLI session and two VS Code harness sessions, one OpenCode session, the Cursor project folder and test
   chats; two Cursor chat files in Wakeshot's `agents\cursor\` that read ended.
3. Found outside scope 1 to 6.
