# A silent winget upgrade starts Wakeshot again

Phase: 6. Status: done, no fix possible (see Result) (new 2026-11-07). Needs Haik: no (Haik, 2026-11-07: a
fix only if small and clean; orchestrator decision, for his review: build it, about 70 lines of code and 60 of tests).
Agent: `wakeshot-worker`. Size: about 100K tokens.
Runs after: `p6-winget-upgrade-restart` (the research, done), `p6-logs-diagnostics` (it also edits the tray's start).
Runs alone among Rust cards if it changes a contract (a file format in `docs/architecture.md`); otherwise beside cards
that own other files.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p6-winget-upgrade-restart-fix\target`; delete it when done.
Owns (2026-11-07 04:43): `crates/wakeshot-windows/src/hooks.rs`, `store.rs`, `app/` except `app/io_thread.rs`, the tray's Quit code wherever it lives in `wakeshot-windows`, `crates/wakeshot-tray/src/velo.rs` if needed, `packaging/e2e.ps1`, the README winget limit line, hook lines in `docs/architecture.md`. Beside it run `p6-log-silent-decisions`, `p8-polish-core-engine` and `p8-polish-windows-power`: a build failure in a file it does not own is reported, never fixed. If the fix needs a contract change (a type in `wakeshot-core` other cards use, or a file format other code reads), stop and reply to the orchestrator before making it.

## Goal

Build the fix that `docs/tasks/p6-winget-upgrade-restart.md` Result picked: the install hook starts the tray again
after Setup over an install, when the tray ran before (a `state.json` naming a process that is gone, and no Quit
mark) and the hook is not elevated. Then update the README limit line.

## Read first

- `docs/tasks/p6-winget-upgrade-restart.md` Result (whole), `docs/tasks/p6-hooks.md` Result, `CLAUDE.md`.
- `crates/wakeshot-windows/src/hooks.rs`, the store code it names, the tray's Quit and start code,
  `packaging/e2e.ps1`, `docs/architecture.md` (hooks and file formats).

## Deliverables

1. The Quit mark (written on menu Quit, removed at start) and the hook's restart, as the research Result describes.
   A started tray is detached, inherits no handles, runs from `current`, at the hook's integrity, never elevated.
2. Tests: each rule fails with the old behavior (no restart after a Quit; no restart when elevated; restart when the
   named tray is gone and no mark; no restart on a first install with no `state.json`).
3. `packaging/e2e.ps1`: one step that runs `Setup.exe --silent` over a running install and checks the tray runs
   again at Medium; and one after a Quit that checks it does not.
4. README: the winget upgrade limit line says what is true now (Haik approved the README; change only that line and
   any line that repeats the limit). `docs/architecture.md` if a file format or hook rule changed.
5. Whole-workspace checks on x64 and ARM64 (`CLAUDE.md`, Checks).

## Rules

- Never run git, not even read-only. Never read token files. Never set `RUSTFLAGS`. Kill only processes you started,
  by PID. Never touch Haik's installed tray; `e2e.ps1` refuses while it runs, so run the e2e step only if the card's
  rules allow it without quitting his tray, else leave it written and untested and say so.
- Plain short English, no em dashes, no emojis. Write Log lines as you go.

## Questions for Haik

None tonight: decide and log "for Haik's review".

## Found outside scope

1. `docs/research.md`, section "winget upgrade over a running Wakeshot (2026-11-07)", last bullet, is wrong: "so a
   process the install hook starts outlives it". Velopack's `run_hook` kills every process from the install folder
   right after each hook (Log 05:01). Not fixed here (workers do not edit `research.md` tonight); the replacement
   line is in Result, "Research line".
2. `docs/tasks/p6-winget-upgrade-restart.md` Result, "Is there a small clean fix: yes", rests on that bullet and is
   wrong for the same reason. Not my card; the orchestrator may want a one-line note there pointing to this Result.

## Log

- 2026-11-07 03:54: card written by the orchestrator (session wakeshot-4e) from the research Result.
- 2026-11-07 04:58: worker started; reading the research Result, hooks card, hooks.rs, store.rs, app/, e2e.ps1.
- 2026-11-07 about 05:01: before writing code, read Velopack's `run_hook` with `gh api` (tag `1.2.161`): after the
  wait, whatever the hook's result, it calls `force_stop_package(root_dir)` "in case the hook left running
  processes". That kills every process whose exe is under the install folder, so a tray the install hook starts dies
  right after the hook exits. The research Result missed this line; `docs/research/distribution.md` ("After every
  hook") already says it.
- 2026-11-07 about 05:03: checked the same on Velopack's default branch `develop` (latest release still `1.2.161`),
  `get_processes_running_in_directory` (image path under the folder, `TerminateProcess`), the install flow (hook, then
  uninstall entry, start only when not silent) and Setup's options (no start-after-silent switch). All confirmed.
- 2026-11-07 05:05: decision: no fix. Every way left needs a delayed starter from outside the install folder (hidden
  `cmd.exe` with a sleep, PowerShell, or a scheduled task), which is neither small nor clean. README line already
  true; no code, no build. Writing Result.
- 2026-11-07 05:08: Result written; Status done (no fix possible). No build folder was made.

## Result

Status: done 2026-11-07 05:08, with no code change: the fix this card was to build cannot work.

### Why there is no fix

Velopack's `run_hook` (`src/bins/src/windows/util.rs`) ends every hook run with `force_stop_package(root_dir)`, "in
case the hook left running processes", after the wait and whatever the hook returned. `force_stop_package` lists
every process, keeps those whose exe path is under the install folder (`QueryFullProcessImageNameW`, `is_sub_path`)
and ends each with `TerminateProcess` (`src/bins/src/shared/util_windows.rs`). So a tray the install hook starts,
Explorer-parented or not, is killed moments after the hook exits, and Setup then finishes and exits. Read with
`gh api` on 2026-11-07 at tag `1.2.161` (our pin and still the latest release, 2026-09-29) and on the default branch
`develop`: the same line in both. Not tested on this laptop (it would need a real install, and `e2e.ps1` refuses
while Haik's tray runs). `docs/research/distribution.md`, "After every hook", already said this; the research card
missed it.

What would still work needs a starter from outside the install folder that waits until Setup's kill is over: a hidden
`cmd.exe` with a sleep, a hidden PowerShell `Wait-Process`, or a one-time scheduled task. Each is timing-based or
leaves a task behind, and an installer that starts a hidden shell with a sleep, or registers a task, looks like
malware to antivirus heuristics. None is small and clean, so none is built.

### Call, for Haik's review

Keep today's behavior and the README limit as it is. Reason: Haik asked for a fix only if small and clean, and the
research card's own trigger said to keep the README limit when the fix grows; the only working fixes are neither.
Change it when Velopack gets a start-after-silent Setup option or stops killing the install folder's processes after
a hook (check at each Velopack upgrade: `run_hook` in `src/bins/src/windows/util.rs`, `install.rs` end of
`install_impl`).

### Deliverables

1. Quit mark and hook restart: not built (above). No file in `crates/` changed.
2. Tests: none, since there is no code. Today's behavior already has its check: `packaging/e2e-update.ps1` step 6
   ("Answer 5": after Setup `--silent` over the install, nothing from the install folder runs).
3. `packaging/e2e.ps1`: unchanged; a "tray runs again" step would test a fix that does not exist.
4. README: the winget limit (lines 548 to 553) and the summary line 53 are true as written, so unchanged.
   `docs/architecture.md` section 6 already says "a hook cannot start it (`research/distribution.md`)", so unchanged.
5. Whole-workspace checks: not run. No code changed, so they could only test the cards building beside this one;
   the orchestrator's wave check covers them. The build folder was never made.

### Research line (for the orchestrator to add to `docs/research.md`)

Under "winget upgrade over a running Wakeshot (2026-11-07)", replacing the last bullet:

- Velopack starts hooks with `CreateProcess` and no job object (`src/lib-rust/src/process_win.rs`), but `run_hook`
  (`src/bins/src/windows/util.rs`) ends every hook with `force_stop_package(root_dir)`, which ends every process whose
  exe is under the install folder (`src/bins/src/shared/util_windows.rs`). So a process the install hook starts from
  the install folder is killed right after the hook exits; only a starter from outside the folder could outlive it.
  Verified (source, tag `1.2.161` and branch `develop`, `gh api` 2026-11-07, card
  `p6-winget-upgrade-restart-fix`); not tested. Same as `docs/research/distribution.md`, "After every hook".

### Files changed

- `docs/tasks/p6-winget-upgrade-restart-fix.md` (this card). No other file created, changed or deleted.

### Open items

- The two outside-scope notes above (`docs/research.md` bullet, the research card's Result).
