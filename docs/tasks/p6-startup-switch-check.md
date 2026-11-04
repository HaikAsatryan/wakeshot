# Check: start at login stays off after Setup.exe over the install; what uninstall leaves of the switch

Phase: 6. Status: in progress. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 50K tokens. Written by the
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

None.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-04: card written by the orchestrator (session `wakeshot-12`).
- 2026-11-04 06:13: worker started; read the card, CLAUDE.md, plan "How we work", `p6-install-e2e` (Log, Result,
  Rerun). This shell is High (elevated), so installs go through a Medium launcher as `e2e.ps1` does.

## Result

What was done, files changed, checks and outcomes, open items.
