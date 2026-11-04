# Check: start at login stays off after Setup.exe over the install; what uninstall leaves of the switch

Phase: 6. Status: done. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 50K tokens. Written by the
orchestrator (session `wakeshot-12`, 2026-11-04) from `p6-readme` "Found outside scope" 6 and docs pass round 4
"Found outside scope" 2.
Runs after: `p6-install-e2e` (its `packaging/e2e.ps1` and rerun). Parallel with: no other laptop card (it installs
Wakeshot); code cards that own none of the files below.
Real laptop: yes: installs Wakeshot at Medium, writes the real `HKCU` Run value and Path through the hooks, and the
`StartupApproved\Run` value Windows keeps for the Settings switch. No power setting is written; no tray is started.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p6-startup-switch-check\target`.

## Goal

`docs/phases/06-ship.md` ("Package and install values", Run value) says a reinstall that writes the Run value again
does not turn start at login back on, because Windows keeps the user's off switch in its own key: not verified.
`hooks.rs` `on_install` writes the Run value every time, and `winget upgrade` runs `Setup.exe` over the install.
Find out, on this laptop, what happens to the switch, and what uninstall leaves of it.

## Read first

- `docs/tasks/p6-install-e2e.md` (Log, Result, "Rerun (2026-11-04)"): how `packaging/e2e.ps1` installs at Medium,
  packs, and checks BEFORE and AFTER.
- `crates/wakeshot-windows/src/hooks.rs` (`on_install`, `on_uninstall`), `crates/wakeshot-windows/src/store.rs`
  (`AutoStart`).
- Current Microsoft docs or a reliable source on `HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\
  StartupApproved\Run` (what the Settings > Apps > Startup switch writes: a 12-byte value, first byte 02 on, 03 off;
  verify, do not trust this line).

## Steps

1. BEFORE: the Run key, the `StartupApproved\Run` key (all values, bytes), RunOnce, the user Path (bytes and type); no
   `HaikAsatryan.Wakeshot` install. Haik's own entries in these keys are never changed.
2. Build release x64 and pack one version into the scratch folder (as `e2e.ps1` does), then a silent install at
   Medium. Record the Run value and whether a `StartupApproved\Run` value named `Wakeshot` exists.
3. Turn start at login off the way the Settings switch does. The session is locked, so write the same value the
   switch writes (from step "Read first", verified); say so in the Log. If you can find the exact bytes the switch
   writes only by using the switch, write the documented "off" form and mark the result "not verified with the real
   switch".
4. Run the same `Setup.exe --silent` over the install (what `winget upgrade` does). Record the Run value and the
   `StartupApproved\Run` value after it: is start at login still off by these keys? Sign-out is not possible here, so
   the sign-in behavior itself stays not verified; say so.
5. Uninstall silently. Record what is left in Run and in `StartupApproved\Run`.
6. AFTER: every BEFORE value equal, except a `StartupApproved\Run` `Wakeshot` value that the uninstall left (record
   it, then delete it and check it is gone, so the laptop ends as before). No install folder, no process this card
   started is left (by pid).

## Deliverables

- The outcome of steps 2 to 5 in the Result: one line each, with the values.
- If the switch does not stay off, or uninstall leaves the `StartupApproved` value: do not change code. Write what a
  fix would be (which file, what it does) under "Found outside scope", for the orchestrator and Haik.
- `docs/research.md`: a section "Start at login switch check" with the facts and how they were checked.
- No script file is needed; if you write one, keep it in the scratch folder.

## Acceptance checks

- AFTER equals BEFORE (with the one recorded exception, deleted at the end).
- Build folder and packs deleted at the end.

## Out of scope

- Any code or doc change other than the research section and this card; `06-ship.md` (the orchestrator updates it).

## Questions for Haik

None blocking. Decided by the worker, for Haik's review (principal designer calls):

1. The off value written in step 3 is `03 00 00 00` plus the FILETIME of the write, the form the community sources
   give for the switch (no Microsoft page documents it, and the switch cannot be used behind the lock screen).
   Reason: the closest thing to the real switch that a locked session allows; the outcome does not depend on the exact
   bytes, since nothing in Setup or the hooks touches the key. Change it when Haik can flip the real switch once: read
   the value it writes and compare.
2. No second run to turn the one FAIL into a PASS: it was a wrong assumption in my own check (a same-data write moves
   the key's time), proven wrong on a scratch key; another install cycle on Haik's PC would add nothing.
3. The reinstall-after-uninstall case is not run: it is the same install hook as step 4 and the leftover value of step
   5, so it adds no new fact. Change it when the fix below lands: then run install, switch off, uninstall, install and
   check the value is gone.

## Found outside scope

Problems seen but not fixed.

1. Uninstall leaves `HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run\Wakeshot` once the
   user has used the switch (off, and most likely also on again: then it is `02` and zeros, not verified). Effects: a
   value is left in the user's registry after an uninstall that is meant to leave nothing, and a later fresh install
   (Setup.exe, `winget install`) starts with start at login off by the keys while everything else is new. Suggested
   fix (not made here):
   - `crates/wakeshot-windows/src/store.rs` (owner `p3-store`): `AutoStart`, the one writer of the start values
     (review m12), gets a delete of the `Wakeshot` value under `<base>\Explorer\StartupApproved\Run` (for example
     `AutoStart::remove_approval(name) -> Result<bool, WinError>`, the same "missing is not an error" rule as
     `remove`); a test over the scratch key like the `Run` tests.
   - `crates/wakeshot-windows/src/hooks.rs` (owner `p6-hooks`): a `HookEnv` method for it, called by `on_uninstall`
     only, right after `remove_run`, with the same log lines ("StartupApproved value Wakeshot removed" or "was not
     there"); the fake's `Call` list and the order tests updated. `on_install` and `on_updated` must keep leaving the
     key alone: that is what keeps the user's off choice across `winget upgrade` (step 4).
   - `packaging/e2e.ps1` step 7: check the value is gone (seed one first).
   Whether a reinstall should keep the old off choice is a product call; my default is no (an uninstall leaves
   nothing, and a reinstall starts like a new PC).
2. `docs/phases/06-ship.md` lines 42 and 43 ("... does not turn it back on (not verified)"): now verified by the keys
   on this laptop (Setup over the install leaves the `StartupApproved` value alone); the sign-in itself is still not
   verified. Phase file, so the orchestrator's.
3. Not a bug, for the record: a user who turns start at login off by deleting the Run value (RegEdit, or a tool that
   deletes instead of using `StartupApproved`) gets it back at the next `winget upgrade`, since `on_install` writes the
   Run value every time. Settings > Apps > Startup and Task Manager use the `StartupApproved` key, so the way the
   README gives is covered.

## Log

- 2026-11-04: card written by the orchestrator (session `wakeshot-12`).
- 2026-11-04 06:13: worker started; read the card, CLAUDE.md, plan "How we work", `p6-install-e2e` (Log, Result,
  Rerun). This shell is High (elevated), so installs go through a Medium launcher as `e2e.ps1` does.
- 2026-11-04 06:15 to 06:17: build `cargo build --release --locked --target x86_64-pc-windows-msvc -p wakeshot-tray
  -p wakeshot-cli` (RUSTFLAGS unset), exit 0 in 105 s; tray 3,953,152 bytes (SHA256 `034C766D...`), CLI 668,672
  (`708E8B76...`). Tree: `main` at f33bf50 (ref written 06:13:56), newest file under `crates\` 06:02:02.
- 2026-11-04 06:16: read-only probe (elevated, `probe.ps1` in the scratch folder): Run 7 values (the same 7 names as
  `p6-install-e2e`), RunOnce empty, Path REG_EXPAND_SZ 332 bytes hash `A8FDC61D7FFE5745` (as there). `StartupApproved`
  has two subkeys, `Run` and `StartupFolder`, both empty: no startup app on this laptop was ever switched, so there
  is no switch-written value here to copy the format from. No install trace, no data folder, no `wakeshot*` process.
  LogonUI runs: the session is locked.
- 2026-11-04 06:17: format sources (web, 2026-11-04). No Microsoft page documents `StartupApproved` (Learn search:
  no hit). Community sources agree: REG_BINARY, 12 bytes, first DWORD 02 (or 06) on, 03 off, then a FILETIME of the
  switch-off time, zero when on (windowsir.blogspot.com 2022-07-09, from Task Manager; the `auto-launch` Rust crate,
  used by Tauri's autostart plugin, writes `02` + 11 zero bytes to enable and reads "last 8 bytes all zero" as on).
  Static check here: the UTF-16 string `Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved` is in
  `SettingsHandlers_Startup.dll` (the Settings > Apps > Startup page), `Taskmgr.exe` (both 10.0.26100.9549),
  `explorer.exe` (10.0.26100.8117), `Startupscan.dll` and `twinui.pcshell.dll`; not in `shell32.dll`,
  `windows.storage.dll`, `twinui.dll`. So the Settings page, Task Manager and Explorer name this key; the bytes the
  Settings switch writes are not verified (no one can use the switch behind the lock screen).
- 2026-11-04 06:19: `check.ps1` (scratch folder) written: steps 1 to 6 in one Medium run, with a tidy end like
  `e2e.ps1`; it writes and deletes only `StartupApproved\Run\Wakeshot` and Wakeshot's own Run value. Parser 0 errors;
  an elevated start refuses with exit 2 and creates nothing. `vpk` 1.2.161.
- 2026-11-04 06:19:31 to 06:19:57: the run (`run.log`, work folder `run\`), launched by `launch.ps1` through
  `explorer.exe` and `run.cmd`. Token: `whoami /groups` in the `.cmd` "Mandatory Label\Medium Mandatory Level"; the
  script "integrity Medium, elevated False". Exit 1: 25 checks, 24 PASS, 1 FAIL (my own probe, explained below).
  1. BEFORE (06:19:34): Run 7 values (same 7 name hashes as the probe), RunOnce 0, `StartupApproved\Run` 0 and
     `StartupApproved\StartupFolder` 0 values (all of `StartupApproved` hash `E9B25BDF1FD8AC4E`), Path type 2, 332
     bytes, `A8FDC61D7FFE5745`. No install.
  2. Pack 0.1.0 (`Setup.exe` 12,364,727 bytes); `Setup.exe --silent` exit 0 in 6.9 s (hook 4.4 s by Velopack, 0.0 s
     by its own timer: new exes). Hook lines: "Run value Wakeshot set to ...", "CLI folder added to the user Path",
     "restore: no journal". Run `Wakeshot` = REG_SZ, 158 bytes,
     `"%LocalAppData%\HaikAsatryan.Wakeshot\current\wakeshot-tray.exe"`. `StartupApproved\Run\Wakeshot`: missing.
  3. Start at login off, written by the script (the session is locked, so not through the switch): REG_BINARY
     `03-00-00-00-E1-58-96-F6-02-6C-DD-01` (state 3, FILETIME 2026-11-04 02:19:47Z, the time of the write). Read back
     equal. Not verified with the real switch.
  4. `Setup.exe --silent` over the install: exit 0 in 0.4 s; Velopack renamed the old folder for rollback and removed
     it. Hook lines: "Run value Wakeshot set to ...", "CLI folder already in the user Path", "restore: no journal".
     Run `Wakeshot` the same 158 bytes. `StartupApproved\Run\Wakeshot` the same 12 off bytes, and the
     `StartupApproved\Run` key's last write time unchanged (02:19:47.381Z). So by the keys start at login stays off.
     FAIL "Run key written again (its last write time moved)": the `Run` key's time stayed 02:19:44.908Z although the
     hook logged its write. My assumption was wrong, not the hook: see 06:20.
  5. `Update.exe --uninstall --silent` (QuietUninstallString): exit 0 in 0.5 s; install folder gone 3 s after. Run
     `Wakeshot` gone (the `Run` key's time moved to 02:19:51.235Z). `StartupApproved\Run\Wakeshot` left: the same
     `03-00-00-00-E1-58-96-F6-02-6C-DD-01`. Data folder gone.
  6. AFTER (06:19:56): the left `StartupApproved\Run\Wakeshot` deleted and read again: gone. Run key, RunOnce key,
     all of `StartupApproved` (hash `E9B25BDF1FD8AC4E`), Path type and bytes equal BEFORE; no install trace, no data
     folder; the 3 processes the script started (Setup 20400 and 36880, Update.exe 29276) gone.
- 2026-11-04 06:20: by PID from here: my `explorer.exe` 30604, the Explorer factory instance 19792, `cmd.exe` 28428,
  `conhost.exe` 4568, the script's pwsh 31416, the Windows Terminal 38524 and OpenConsole 26700 that DcomLaunch
  started for the window, Velopack's delete timer `cmd.exe` 2264 and its `conhost.exe` 23492, and the script's three:
  all gone; 19792 ended by itself by 06:20:42. None needed a kill.
- 2026-11-04 06:20: the FAIL explained: on a scratch key (`HKCU\Software\WakeshotSwitchCheckScratch`, made and deleted
  by `keytime.ps1`), a second `SetValue` of the same string kept the key's last write time (02:20:39.515Z), a write of
  other text moved it (02:20:42.544Z). A write of the same data does not change the key, so the check was wrong; the
  hook's line and the code (`set_run` always calls `RegSetValueExW`) show the write happened.
- 2026-11-04 06:20:48: independent elevated AFTER read (`probe.ps1`): every line as at 06:16 (Run 7 names, same
  hashes; RunOnce empty; Path same hash; `StartupApproved` hash `E9B25BDF1FD8AC4E`, both subkeys empty; no trace, no
  data folder, no `wakeshot*` process; no uninstall key). The `Run` and `StartupApproved\Run` keys' last write times
  moved (01:48:03Z and 17:39:05Z before; any write moves them and nothing puts them back): recorded, not a value.
- 2026-11-04 06:21: build folder and packs deleted (checked gone); no `wakeshot-pack-*` staging folder in `%TEMP%`.
  Kept in the scratch folder: the scripts, `run.log`, `run.exit`, `run-whoami.txt`, `pids.txt`, `build.log`,
  `run\before.json`, `run\pack.log`, `run\wakeshot-before-uninstall.log`. `docs/research.md` section "Start at login
  switch check" appended at the end (re-read first; LF, no line over 120, no em dash). Velopack's own logs in
  `%LocalAppData%\velopack` are now 91,811 and 107,712 bytes (this run added to them; `p6-install-e2e` Found outside
  scope 5 has Haik's optional clean-up).
- 2026-11-04 06:22: acceptance checks: AFTER equals BEFORE with the one recorded exception deleted (script and the
  separate elevated read); build folder and packs deleted. No repo file changed except this card and the research
  section. Card done.

## Result

Status: done. One run at Medium (06:19:31 to 06:19:57) of a release x64 build of f33bf50, packed as 0.1.0. The
session was locked, so the off switch was written by the script in the documented form, not by the real switch.

| Step | Outcome |
|---|---|
| 2 install | Run `Wakeshot` = REG_SZ `"%LocalAppData%\HaikAsatryan.Wakeshot\current\wakeshot-tray.exe"`; no `StartupApproved\Run\Wakeshot` value (on by the keys) |
| 3 switch off | `StartupApproved\Run\Wakeshot` = REG_BINARY `03-00-00-00-E1-58-96-F6-02-6C-DD-01` (state 3, FILETIME of the write); not verified with the real switch |
| 4 Setup over the install | exit 0; hook wrote the Run value again (same data); `StartupApproved` value the same 12 off bytes, key untouched: start at login stays off by the keys. Sign-in behavior not verified (no sign-out possible) |
| 5 uninstall | Run value gone; `StartupApproved\Run\Wakeshot` left with the same off bytes |
| 6 AFTER | leftover recorded, deleted, checked gone; Run, RunOnce, all of `StartupApproved`, Path type and bytes equal BEFORE; no install trace, no data folder, no process left |

Format of the value: REG_BINARY, 12 bytes, DWORD 02 on and 03 off, then a FILETIME (community sources; no Microsoft
doc found). The Settings page DLL, Task Manager and Explorer all name the key (string search). Bytes the real switch
writes: not verified.

The run's exit code is 1: 24 of 25 checks passed. The one FAIL was my own wrong check (a same-data registry write does
not move the key's last write time, proven on a scratch key); the hook's write is shown by its log line.

Files changed: this card; `docs/research.md` (new section "Start at login switch check"). No code.

Acceptance checks: AFTER equals BEFORE (script and an independent elevated read), with the one recorded exception
deleted; build folder and packs deleted. Pass.

Open items: Found outside scope 1 (uninstall leaves the switch value; fix in `store.rs` and `hooks.rs`, not made), 2
(`06-ship.md` lines 42 and 43 for the orchestrator), 3 (for the record). Decisions 1 to 3 for Haik's review.
