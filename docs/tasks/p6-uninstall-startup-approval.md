# Uninstall removes the start at login switch value

Phase: 6. Status: done. Needs Haik: no (orchestrator decision for his review: an uninstall leaves nothing, so a
reinstall starts as on a new PC). Agent: `wakeshot-worker`. Size: about 50K tokens. Written by the orchestrator
(session `wakeshot-12`, 2026-11-04) from `p6-startup-switch-check` "Found outside scope" 1.
Runs after: `p6-startup-switch-check` (done). Parallel with: code cards that own none of the files below; no other
laptop card while step 4 runs (it installs Wakeshot).
Real laptop: step 4 only: one `packaging/e2e.ps1` run (installs at Medium, as `p6-install-e2e` did).
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p6-uninstall-startup-approval\target`.

## Goal

Once the user has used the Settings > Apps > Startup switch, Windows keeps a `Wakeshot` value under
`HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run`. Uninstall removes the Run value but
leaves that one (`p6-startup-switch-check`, research "Start at login switch check"). Make the uninstall hook remove
it too. Install and update hooks must keep leaving it alone: that is what keeps the user's off choice across
`winget upgrade`.

## Read first

- `docs/tasks/p6-startup-switch-check.md` (Result, Found outside scope 1: the suggested fix).
- `crates/wakeshot-windows/src/store.rs` (`AutoStart`, `StartKey`, `remove`: the one writer of the start values,
  review m12), `crates/wakeshot-windows/src/hooks.rs` (`HookEnv`, `RealHookEnv`, `on_uninstall`, the fake and its
  `Call` list, the order tests).
- `docs/architecture.md` section 6, "Velopack hooks" (the uninstall order).

## Deliverables

1. `store.rs`: `AutoStart` can delete the `Wakeshot` value under `<base>\Explorer\StartupApproved\Run`, with the same
   "missing is not an error" rule as `remove`. A test over the scratch key, like the `Run` tests.
2. `hooks.rs`: a `HookEnv` method for it, called by `on_uninstall` only, right after the Run value is removed, with log
   lines in the style of the others ("... removed" or "was not there"). The fake's `Call` list and the order tests
   updated; a test that install and update never call it.
3. `packaging/e2e.ps1` step 7: seed a `StartupApproved\Run\Wakeshot` value (the off form `03` plus 8 bytes, as the
   check card wrote) before the uninstall, then check it is gone after. Its BEFORE and AFTER cover that key too.
4. Run `packaging/e2e.ps1` once at Medium (the way `p6-install-e2e` did, release x64 of the tree); record the outcome.
5. `docs/architecture.md` section 6: the uninstall order line. `README.md`: only the uninstall line that lists what is
   removed and what stays, if it names start at login (the README is a draft that waits for Haik; touch nothing else).

## Acceptance checks

- `cargo fmt --all --check`; `cargo clippy --workspace --locked --target <t> -- -D warnings` with and without
  `--all-targets`, x64; ARM64 for `wakeshot-windows` (the tray's ARM64 needs clang: CI only); `cargo test --locked
  -p wakeshot-windows`; `cargo test --workspace --locked` once at the end.
- `e2e.ps1`: every check passes; AFTER equals BEFORE.
- Build folder deleted at the end.

## Out of scope

- Any other hook step; the install and update hooks' behavior.

## Questions for Haik

None blocking. Decided by the worker, for Haik's review (principal designer calls):

1. `e2e.ps1` writes the off switch value at step 5, before Setup over the install, not just before the uninstall. So
   one run also shows that the install and updated hooks leave it (Setup over the install and the update keep its 12
   bytes), which is what keeps an off choice across `winget upgrade`. Change it when a step 5 or 6 failure must be
   told apart from the switch value: write it at step 7 only.
2. A failed delete of the switch value is a WARN hook line; a failed Run, RunOnce or Path removal stays ERROR. Reason:
   the switch value names no exe, so a value left behind breaks nothing; only a later install starts with the old
   switch. Change it when Haik wants every uninstall leftover at the same level.
3. `e2e.ps1` treats a `StartupApproved\Run\Wakeshot` value found at the start as a Wakeshot trace and refuses (exit 2),
   as it does for the Run value; its tidy end deletes the value, and any `StartupApproved` key the run had to make.
   Reason: the same rule as Run and RunOnce, so any such value at the end is the run's own and safe to delete. Change
   it when the script must run on a PC with an old leftover value: then save it in BEFORE and write it back.
4. `AutoStart` gets test-only `set_approval` and `approval` (`#[cfg(test)]`), used by the `store.rs` and `hooks.rs`
   tests. Reason: Wakeshot itself never writes the switch value (only Windows does), so no shipped code may write it.
5. The README line reads "removes start at login (with its switch in Settings > Apps > Startup), ...". Reason: the
   README's own way to turn start at login off is that switch, so a user sees that it goes too. Change it with
   Haik's README review.

## Found outside scope

Problems seen but not fixed.

1. `docs/phases/06-ship.md` line 58, the hooks table row "uninstall": "remove the Run value, the RunOnce value and the
   Path entry; ...". It now also removes the Run value's `StartupApproved` value, right after the Run value. Phase
   file, so the orchestrator's.
2. `docs/architecture.md` section 6, the "Registry values Wakeshot writes" paragraph (about line 949): it says
   `store::AutoStart` writes or removes the Run and RunOnce values. Not wrong, but `AutoStart` now also removes the
   switch value at uninstall; one clause would say so. Not the uninstall line this card owns.
3. `README.md` "Everything else it keeps" (registry list, about line 200) and the uninstall "What stays" list do not
   name the switch value Windows keeps for Wakeshot. Suggested: "Windows keeps your Settings > Apps > Startup switch
   for Wakeshot under `HKCU\...\Explorer\StartupApproved\Run`; uninstall removes it." The README waits for Haik.
4. `crates/wakeshot-windows/src/store.rs` module doc (line 1) lists "the Run and RunOnce values"; the `AutoStart` doc
   now names the switch value too. Module doc outside `AutoStart`, so left.
5. Velopack's own logs in `%LocalAppData%\velopack` are now 104,698 and 119,808 bytes (this run added about 25 KB);
   Haik's optional clean-up is in `p6-install-e2e` Found outside scope 5.
6. Still open from `p6-startup-switch-check`: whether the real Settings switch writes the same bytes, and that
   Explorer skips Wakeshot at sign-in when the value is off. Both need Haik at an unlocked screen.

## Log

- 2026-11-04: card written by the orchestrator (session `wakeshot-12`).
- 2026-11-04 06:24: worker started; Status in progress.
- 2026-11-04 06:29: read the card, CLAUDE.md, plan "How we work", `p6-startup-switch-check`, `p6-install-e2e`,
  `store.rs`, `hooks.rs`, `e2e.ps1`, architecture section 6, README uninstall part, research "Start at login switch
  check". Code written: `AutoStart::remove_approval` (and test-only `set_approval` and `approval`), `HookEnv::
  remove_approval` called by `on_uninstall` right after `remove_run`, tests. `rustfmt` on the two files.
- 2026-11-04 06:31: `cargo clippy -p wakeshot-windows --all-targets --locked --target x86_64-pc-windows-msvc -- -D
  warnings` exit 0 (46 s); `cargo test --locked -p wakeshot-windows` exit 0: 552 passed, 0 failed, 21 ignored (67 s),
  the new `auto_start_removes_the_switch_value`, `only_uninstall_removes_the_switch_value` and
  `real_install_and_updated_keep_the_switch_value` among them.
- 2026-11-04 06:33: `e2e.ps1` changed (step 1 BEFORE, steps 4 to 8; help text). Architecture section 6 uninstall
  sentence and README uninstall line 1 changed. `e2e.ps1` parses with 0 errors, LF, no em dash; an elevated start
  still refuses with exit 2 and makes no work folder; `Get-Help` reads the new synopsis.
- 2026-11-04 06:35: build `cargo build --release --locked --target x86_64-pc-windows-msvc -p wakeshot-tray -p
  wakeshot-cli` (RUSTFLAGS unset), exit 0 in 116 s; tray 3,954,176 bytes (SHA256 `E2028150...`), CLI 668,672
  (`06A22892...`). Tree: `main` at 75233b6 (ref written 06:24:02) plus this card's two Rust files; newest other file
  under `crates\` 06:02:02.
- 2026-11-04 06:36: elevated read-only probe (`probe.ps1`, scratch): no install trace, no data folder, no `wakeshot*`
  process, no `Software\Wakeshot-test` key left by the tests; Run 7 values, RunOnce empty, `StartupApproved` with
  subkeys `Run` and `StartupFolder`, both empty; Path REG_EXPAND_SZ 165 characters; Balanced the only plan and active,
  lid 1/1, sleep 18000/18000, saver DC 30, `/qh` hash `C64AFDB1CFDD13CF`. LogonUI runs: the session is locked.
- 2026-11-04 06:37:15 to 06:37:52: the one `e2e.ps1` run (`run.log`, work folder `run\`), launched by `launch.ps1`
  through `explorer.exe` and `run.cmd`, as `p6-install-e2e` did. Token: `whoami /groups` in the `.cmd` "Mandatory
  Label\Medium Mandatory Level"; the script "integrity Medium, elevated False". Exit 0: 88 checks, 88 PASS, 0 FAIL
  (81 before plus 7 new). Per step:
  1. BEFORE (06:37:16): Run 7 values, RunOnce 0, `StartupApproved\Run` key exists with 0 values, Path type 2, 332
     bytes, `A8FDC61D7FFE5745`; Balanced the only plan and active, lid 1/1, sleep 18000/18000, saver DC 30, `/qh`
     `C64AFDB1CFDD13CF`. No install, no data folder.
  2. Copy `ee1f5b9b-66f3-410c-9de5-8abc482fb6b4`, DC lid 1, never active.
  3. Packs: `Setup.exe` 12,365,225 bytes; full nupkg 4,807,593; delta 8,975; portable zip 4,800,117.
  4. Install exit 0 in 6.1 s; all install checks pass; new check: no `StartupApproved\Run\Wakeshot` value after it.
  5. Switch value written: REG_BINARY `03-00-00-00-A3-40-0B-72-05-6C-DD-01` (state 3, FILETIME of the write), read
     back equal. Hold (owner PING 8236, ended); Setup over the install exit 0 in 0.5 s; restored; the switch value
     the same 12 bytes after it.
  6. Hold (owner 28912); `Update.exe apply`: exit 0 in 0.9 s (updated hook 514 ms by Velopack); restored; `current`
     same path, new folder; the switch value the same 12 bytes.
  7. Hold (owner 23996) and the test RunOnce value; the switch value still the same before the uninstall;
     `Update.exe --uninstall --silent` exit 0 in 0.3 s (hook 112 ms by Velopack), install folder gone 3.1 s after.
     Restored; Run value, switch value, RunOnce value and Path entry gone; data, install and temp folders, uninstall
     key and shortcut gone.
  8. AFTER (06:37:50): Run, RunOnce and `StartupApproved\Run` keys (key there, 0 values) equal BEFORE; Path type and
     bytes, active plan, the real plan's values and `/qh` hash, plan list equal BEFORE; no install trace; data folder
     as BEFORE (none); none of the 8 processes it started left. The tidy part found nothing to put back.
  The hook log saved before the uninstall: 23 hook lines, no `StartupApproved` line (install and updated hooks never
  touch it), no WARN or ERROR line. The uninstall hook's own lines went with the data folder, as in `p6-install-e2e`;
  the missing value is the proof.
- 2026-11-04 06:38:29: by PID from here, all gone, none killed: my `explorer.exe` 26868, the Explorer factory instance
  13428, `cmd.exe` 19560, `conhost.exe` 28348, 26364, 11332, 21728 and 37012, OpenConsole 38876 and Windows Terminal
  32084 (DcomLaunch started them for the window), the script's pwsh 38072, its 8 (Setup 35048 and 10500,
  `wakeshot.exe` 36964, PING 8236, 28912 and 23996, Update.exe 32320 and 38560) and Velopack's delete timer `cmd.exe`
  12800.
- 2026-11-04 06:38:34: independent elevated AFTER read (`probe.ps1`): every line equal to the 06:36 read (Run, RunOnce,
  all three `StartupApproved` keys, Path, plans, power values, no trace, no `wakeshot*` process, no
  `Software\Wakeshot-test`) except Velopack's own logs, now 104,698 and 119,808 bytes (+12,887 and +12,096). No
  `%TEMP%\wakeshot-pack-*` folder.
- 2026-11-04 06:40 to 06:43: acceptance checks: `cargo fmt --all --check` exit 0; `cargo clippy --workspace
  --all-targets --locked --target x86_64-pc-windows-msvc -- -D warnings` exit 0 (70 s) and without `--all-targets`
  exit 0 (units fresh from the earlier runs, `-v` shows them "Fresh"); `cargo clippy -p wakeshot-windows --locked
  --target aarch64-pc-windows-msvc -- -D warnings` with and without `--all-targets` exit 0 (both checked
  `wakeshot-core` and `wakeshot-windows`); `cargo test --workspace --locked` exit 0 in 147 s: 1,287 passed, 0 failed,
  52 ignored (the ignored ones are the real-laptop lanes). No `Software\Wakeshot-test` key left after the tests.
- 2026-11-04 06:43: `docs/research.md` "Install end to end checks": one dated bullet added at its end (re-read first;
  LF; no line over 120).
- 2026-11-04 06:44: build folder deleted (checked gone); the packs in `run\packs` deleted too. Kept in
  `<scratchpad>\p6-uninstall-startup-approval\`: `run.log`, `run.exit`, `run-whoami.txt`, `pids.txt`, `run.cmd`,
  `launch.ps1`, `probe.ps1`, `probe-before.txt`, `probe-after.txt`, `lint.ps1`, `build.log`, `test-windows.log`,
  `test-workspace.log`, `run\` (`before.json`, pack logs, the hook log saved before the uninstall, Velopack's lines).
- 2026-11-04 06:44: decided by the worker, for Haik's review: decisions 1 to 5 under "Questions for Haik".
- 2026-11-04 06:44: Result written; Status done. Repo files changed: `store.rs`, `hooks.rs`, `packaging/e2e.ps1`,
  `docs/architecture.md` (one sentence), `README.md` (one line), `docs/research.md` (one bullet), this card.

## Result

Status: done. The uninstall hook now deletes `HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\
StartupApproved\Run\Wakeshot` right after the Run value; install and updated never touch it. One `e2e.ps1` run at
Medium (06:37:15 to 06:37:52, x64 release of 75233b6 plus this card's code): exit 0, 88 of 88 checks passed. The
install wrote no switch value; the script wrote the off form (`03 00 00 00` and a FILETIME, not the real switch: the
session was locked); Setup over the install and the update kept its 12 bytes; the uninstall removed it with Run,
RunOnce and the Path entry. AFTER equals BEFORE (the script, and an elevated read before and after); no process left.

Files changed:

- `crates/wakeshot-windows/src/store.rs`: `AutoStart::remove_approval(name) -> Result<bool, WinError>` ("missing is
  not an error", as `remove`, which now shares its delete code); test-only `set_approval` and `approval`; test
  `auto_start_removes_the_switch_value` over a scratch key (also checks the real key path).
- `crates/wakeshot-windows/src/hooks.rs`: `HookEnv::remove_approval`, called by `on_uninstall` only, right after
  `remove_run`; lines "StartupApproved value Wakeshot removed" or "was not there", WARN on failure. Fake `Call` list
  and the order tests updated; new tests `only_uninstall_removes_the_switch_value` (fake) and
  `real_install_and_updated_keep_the_switch_value` (real env); the real uninstall test seeds and checks it.
- `packaging/e2e.ps1`: BEFORE and AFTER cover `StartupApproved\Run`; step 4 checks no value; step 5 writes the off
  form; steps 5 to 7 check it kept; step 7 checks it gone; the tidy end deletes it; help text.
- `docs/architecture.md` section 6: the uninstall sentence. `README.md`: uninstall line 1.
- `docs/research.md`: one bullet in "Install end to end checks". This card.

Checks: `cargo fmt --all --check` pass; clippy x64 workspace with and without `--all-targets` pass; clippy ARM64
`wakeshot-windows` with and without `--all-targets` pass; `cargo test --locked -p wakeshot-windows` 552 passed;
`cargo test --workspace --locked` 1,287 passed, 0 failed; `e2e.ps1` 88 of 88; build folder deleted.

Open items: decisions 1 to 5 for Haik's review; Found outside scope 1 to 6 (the phase file's uninstall row, two doc
clauses, the README lists, Velopack's logs, the real-switch checks that need Haik).
