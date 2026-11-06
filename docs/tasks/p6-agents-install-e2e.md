# The bigger v1 installed: checks, a fresh install over Haik's, and a hold per agent

Phase: 6. Status: in progress (Part 1 done 2026-11-06: deliverables 1 and 2; Part 2, deliverables 3 to 6, waits
for the Cursor and Antigravity live cards). Needs
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

## Found outside scope

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
