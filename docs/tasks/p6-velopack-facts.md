# Check the Velopack facts the ship track builds on

Phase: 6. Status: done. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 100K tokens.
Real laptop: yes. It installs, updates and uninstalls a throwaway app under the pack id `HaikAsatryan.Wakeshot`. No
power settings, no screen, no sound. The orchestrator runs it alone among the real-laptop cards of all tracks.
Runs after: Haik's sign-off of the track plans; the contracts card (review m10: points 9 and 10 build the repo's
release exes, and the card installs software; the trial crate itself lives outside the repo). Parallel with: every
card not marked real laptop.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p6-velopack-facts\target`; the trial crate in
`<scratchpad>\p6-velopack-facts\trial`.

## Goal

Settle the Velopack points the ship cards depend on, on this laptop, before any Wakeshot code relies on them.

## Read first

- `docs/research/distribution.md`, section 1 (the Phase 0 trial and its commands).
- `docs/architecture.md` section 6: the data folder paragraph and "Velopack hooks".
- `docs/phases/06-ship.md`: "Package and install values", "Hooks", "Updates".
- `docs/research.md`: "Ship track planning checks".

## Deliverables

1. A throwaway Rust crate outside the repo with `velopack = "=1.2.161"`, built for x64 only (an ARM64 build needs
   clang, which this laptop lacks). Two exes, named like Wakeshot's: a GUI `wakeshot-tray.exe` that calls
   `VelopackApp::build()` with the four fast callbacks plus `on_restarted`, and a console `wakeshot.exe`. Every hook
   and start logs its arguments, `VELOPACK_*` variables, working folder and exe path to a file outside the install
   folder, in `<scratchpad>\p6-velopack-facts\`.
2. Answers, each marked verified (how) or not verified:
   1. The pack id with a dot, `HaikAsatryan.Wakeshot`: `vpk pack`, silent install, update and silent uninstall all
      work. The install folder name; the uninstall key name (winget's `ProductCode`), its `DisplayName` and
      `Publisher`.
   2. With `--packTitle Wakeshot`: the root stub's name, what it starts, and whether `wakeshot.exe` in `current`
      and a root `Wakeshot.exe` cause any clash. `--shortcuts StartMenuRoot`: only a Start menu shortcut, and its
      target.
   3. The `current` folder: its full path before and after an update from a local folder (`FileSource`). Same path or
      not decides what the Run value and the Path entry point at.
   4. `wait_exit_then_apply_updates(asset, true, true, [])`: the hook order, that the new version starts, what
      `on_restarted` gets; the app exits 5 s after the call; the app does not exit for 70 s (the 60 s wait).
   5. `UpdateManager::new` in an exe run from the build folder, not installed: the error, so the tray can tell "not
      installed" from a real failure.
   6. A hook that panics, and a hook that sleeps past its limit: what Velopack logs, whether the operation goes on,
      and where hook stdout and stderr go.
   7. `Setup.exe --silent` over a running install (what `winget upgrade` runs): only the install hook runs and the app
      is not started (Phase 0 saw this with another id; check it with the dotted one).
   8. Whether `Update.exe` can apply a staged package by itself (`Update.exe apply -h`), so `p6-install-e2e` can test
      the update hooks without a running tray.
   9. ARM64 packing on this laptop: `vpk pack -r win-arm64 --channel win-arm64` with the ARM64 exes of the repo's own
      release build (no `velopack` in them yet). Does `vpk` accept them, and which CPU are the package's `Setup.exe`
      and `Update.exe` (read the PE header)?
   10. `vpk pack` with a main exe that never calls `VelopackApp` (the skeleton tray): a warning, an error or nothing.
   11. The options of `vpk upload github -h` and `vpk download github -h`: putting both channels in one release,
       draft or published, pre-release, the tag; `vpk pack -h` for `--icon`, `--releaseNotes` and deltas.
3. `docs/research.md`: a new section "Velopack checks for Wakeshot", each fact with how it was checked. Additions
   only; re-read the end of the file right before the edit (other cards append too).
4. Clean up: every trial install uninstalled.

## Acceptance checks

- Each of the 11 points answered with its mark.
- An AFTER check in the Log, with the command output: no `%LocalAppData%\HaikAsatryan.Wakeshot`, no
  `%TEMP%\velopack_HaikAsatryan.Wakeshot`, no `HKCU\...\Uninstall\HaikAsatryan.Wakeshot` key, no Start menu
  shortcut, no `Wakeshot` value under `Run` or `RunOnce`, no trial process running (by the PIDs this card started).
- No file in the repo changed except `docs/research.md` and this card.

## Out of scope

- Wakeshot code, `crates/`, `Cargo.toml`. Power settings. Any upload to GitHub (`-h` only). ARM64 builds with
  `velopack`.

## Questions for Haik

Each with the worker's default.

None.

## Found outside scope

Problems seen but not fixed.

- `docs/architecture.md` section 6 still says "Whether Velopack accepts a dot in it: not verified" and "Not verified:
  that the `current` folder keeps its path across updates". Both are now verified (research "Velopack checks for
  Wakeshot", points 1 and 3).
- `docs/phases/06-ship.md`, "Updates": "Velopack waits at most 60 s for the exit" is true, but after the 60 s it kills
  the tray (exit code 1) and applies anyway; worth one clause.
- `docs/research/distribution.md` section 1: "everything removed at the end" of the Phase 0 trial, but its log
  `%LocalAppData%\velopack\velopack_WakeshotTrial.log` is still there; Velopack never removes its logs on uninstall.
  The hook limits line ("Plan for the doc value, 30 s") can now cite the test: uninstall really waits 60 s.
- Tray track: `VelopackApp::run()` sets the process AUMID to `velopack.HaikAsatryan.Wakeshot`, but only in an
  installed build (a build from `target\` returns before that). How Windows groups the tray's notices and icon
  settings may therefore differ between dev and installed builds (not tested).
- `README` (`p6-pack-and-release`): Velopack leaves `%LocalAppData%\velopack\velopack.log` and
  `velopack_HaikAsatryan.Wakeshot.log` after uninstall; the "what it changes" text should say so or not claim a clean
  removal of every file.

## Log

- 2026-11-03: card written by `p6-plan-ship`.
- 2026-11-03 21:50: worker started; read the card, plan "How we work", CLAUDE.md.
- 2026-11-03 21:51: BEFORE check (`scripts\state.ps1` in the card's scratch folder): no install folder, no
  `%TEMP%\velopack_HaikAsatryan.Wakeshot`, no uninstall key, no Wakeshot Start menu or desktop shortcut, no Wakeshot
  value under Run or RunOnce, no process from the install folder. Tools: cargo 1.99.1, dotnet 10.0.403, targets x64
  and ARM64 installed.
- 2026-11-03 21:54: repo ARM64 release build (`--workspace --locked`) done in 46 s into the card's target folder.
  Trial crate built (x64, 0.1.0 and 0.2.0), packed with the dotted id, silent install, stub run and the 0.1.0 to
  0.2.0 update done; logs in the scratch folder (`trial.log`, and Velopack's own logs in `%LocalAppData%\velopack`).
- 2026-11-03 21:59: point 5 probed from the build folder: `NotInstalled`.
- 2026-11-03 22:00: points 9 and 10: ARM64 pack of the repo exes works, all PE ARM64; no VelopackApp check for
  native exes (vpk source read).
- 2026-11-03 22:02: point 4, app stays 75 s: Update.exe waits 60 s, then applies anyway and kills the app (exit 1).
- 2026-11-03 22:03 to 22:08: points 8, 7 and 6 tested (Update.exe apply alone; Setup over a running install; hook
  panics in obsolete and install; hooks past their limits in updated, install and uninstall); auto apply at startup
  seen; final silent uninstall while the app ran.
- 2026-11-03 22:08: AFTER check (`scripts\state.ps1`, 8 s after the uninstall, which deletes the folder through a
  `cmd.exe` 3 s timer):

  ```
  install folder  : C:\Users\haika\AppData\Local\HaikAsatryan.Wakeshot exists=False
  temp folder     : C:\Users\haika\AppData\Local\Temp\velopack_HaikAsatryan.Wakeshot exists=False
  HaikAsatryan.* : 0
  uninstall key   : exists=False
  start menu lnk  : 0
  desktop lnk     : 0
  Run             : 0
  RunOnce         : 0
  procs in folder : 0
  ```

  Of the 21 PIDs this card started or caused (Setup, Update.exe, stub, trial exes, and the hooks and restarts they
  spawned), 20 are gone. PID 3268 (a Setup run that exited at 22:06) now belongs to `chrome.exe`: a reused PID, not
  touched. No
  process named `wakeshot*` or with `HaikAsatryan` in its command line runs. Left outside the scratch folder, not
  deleted (worker rule: delete only inside the repo or the scratch folder): `%LocalAppData%\velopack\velopack.log`
  and `velopack_HaikAsatryan.Wakeshot.log` (Velopack's own logs, 27 KB and 51 KB), the empty `%TEMP%\velopack`
  (vpk's work folder) and two WER reports `C:\ProgramData\Microsoft\Windows\WER\ReportArchive\AppCrash_wakeshot-tray*`
  from the panic tests.
- 2026-11-03 22:14: section "Velopack checks for Wakeshot" appended to `docs/research.md` (re-read the end first;
  LF kept). Trial crate `cargo fmt` and `cargo clippy --release --all-targets -- -D warnings` clean. Deleted the
  card's `target`, `trial-target` and all pack outputs; kept the trial source, scripts, `trial.log`, help outputs and
  copies of Velopack's logs in the scratch folder.
- 2026-11-03 22:16: acceptance checks run; card done.

## Result

Status: done. All 11 points answered on this laptop; every trial install removed (AFTER check in the Log). Full
facts with how each was checked: `docs/research.md`, "Velopack checks for Wakeshot". Scratch folder:
`C:\Users\haika\AppData\Local\Temp\claude\C--Users-haika-code-wakeshot\a39ae864-ceb2-4d9a-8673-9b782949a810\scratchpad\p6-velopack-facts`
(`trial\` crate, `scripts\`, `trial.log`, `velopack-logs\`, `help\`, `src-read\`).

### Answers (all verified by test on this laptop unless marked)

1. Dotted id `HaikAsatryan.Wakeshot`: pack, silent install, four updates, three Setup runs over an install and silent
   uninstall all work. Install folder `%LocalAppData%\HaikAsatryan.Wakeshot`; uninstall key (winget `ProductCode`)
   `HaikAsatryan.Wakeshot`, `DisplayName` `Wakeshot`, `Publisher` `Haik Asatryan`. Quirk: Setup's rollback folder is
   `%LocalAppData%\HaikAsatryan.<random>`, deleted at the end.
2. Root stub `Wakeshot.exe` runs `Update.exe start`, which starts `current\wakeshot-tray.exe` with the stub's args. No
   clash with `current\wakeshot.exe`. `StartMenuRoot` gives only `Start Menu\Programs\Wakeshot.lnk`, target
   `current\wakeshot-tray.exe`.
3. `current` keeps its full path across updates (the folder object is new each time). Run value and Path entry can
   point at `current`.
4. Hooks: obsolete (old exe), kill, swap, updated (new exe), then the new tray starts with no args,
   `VELOPACK_RESTART=true`, `on_restarted(new version)`. Exit 5 s after the call: applied at once. No exit for 75 s:
   after 60 s Update.exe applies anyway, runs obsolete while the old tray lives, then kills it (exit code 1).
5. Not installed: `Err(Error::NotInstalled("Could not auto-locate app manifest"))`; match `NotInstalled(_)`.
6. Panic (abort): exit 0xC0000409 after about 3 s (WER report), Velopack WARNs and goes on. Past the limit: killed
   with an ERROR line, operation goes on; install 30 s, updated 15 s, uninstall 60 s (docs say 30 s). Hook stdout and
   stderr are lost (no handles; source).
7. `Setup.exe --silent` over a running install: tray killed, only the install hook, app not started.
8. `Update.exe apply --silent --norestart` alone applies the newest staged package with both hooks: yes.
9. ARM64: vpk packs the repo's ARM64 exes; Setup.exe, Update.exe (`lib/app/Squirrel.exe`) and the stub are ARM64
   (PE 0xAA64).
10. Main exe without `VelopackApp`: nothing at all (vpk checks only .NET exes; source).
11. Options listed in research. Both channels in one release: same `--tag v<version>`, second upload with `--merge`,
    `--publish` on the last; the default tag has no `v`. `--releaseNotes` becomes the release body; `--icon` sets
    Setup and Update icons; `--delta None|BestSpeed|BestSize` (source and docs).

### Design calls for the cards that build on this (principal designer)

- **Run value** points at the quoted `current\wakeshot-tray.exe` (point 3). Change it when a Velopack bump changes
  the layout (check again with each bump).
- **`velo::startup()` should call `.set_auto_apply_on_startup(false)`** (`p6-velopack-dependency`, which owns
  `velo.rs`). Reason: with the default, a staged package is applied inside `run()` at the next start, before the core
  can check holds, and the apply kills every process from `current`, so a `wakeshot run` wrapper too. The update
  thread then finds a staged package with `get_update_pending_restart()` and lets the core apply it. Change it when
  Wakeshot wants a staged update to land at sign-in regardless of holds.
- **Updated hook inner limit 10 s, not 12 s**: the hook always runs a just-extracted exe, which took up to 1.7 s to
  start, so 12 s of 15 s leaves about 1.3 s. Install and uninstall keep 25 s (uninstall really allows 60 s, but the
  docs say 30 s). Change it when a measured first start is well under 1 s.
- **Quit on `ApplyUpdate`** must finish within 60 s; a slower tray is killed and the update still goes on, which the
  updated hook's restore already covers.
- **Smoke test and e2e** (`p6-pack-and-release`, `p6-install-e2e`): after the uninstall exits, poll up to 10 s for the
  install folder to go (a `cmd.exe` 3 s timer deletes it); prove `VelopackApp` runs by the install hook's log line,
  since vpk will not; do not expect `%LocalAppData%\velopack\*.log` to go; stage with the app (check and download
  only) and apply with `Update.exe apply --silent --norestart`.
- **Hooks** write only to Wakeshot's own log (stdout is lost) and must never panic (a crash report and 3 s lost).

### Elevation

All runs were elevated, and every hook and restart inherited it (`elevated=yes`). Per user, unelevated, the same
steps run at Medium. Not tested unelevated: whether a Medium Update.exe can kill a tray the user started elevated, and
the working folder of a start from the Run value (Velopack starts use `current`; my manual starts inherited mine, so
the tray must not rely on its working folder). The Access-denied warning on each wait comes from the access mask, not
from elevation (source).

### Files changed

- `docs/research.md`: new section "Velopack checks for Wakeshot" (additions only).
- `docs/tasks/p6-velopack-facts.md`: this card.

### Checks

- 11 points answered with marks: done (above and in research).
- AFTER check with output: in the Log; all clean. 20 of 21 PIDs gone, 1 PID reused by `chrome.exe` (not ours).
- No other repo file changed by this card: files changed in the repo since 21:50 are this card, `docs/research.md`,
  and files of other running cards and the phase files (`p2-ownership-props`, `p4-cron`, `p6-int-dispatch`, the
  orchestrator); `Cargo.toml` and `Cargo.lock` unchanged since 18:32 (checked by write time; no git).
- Repo ARM64 release build `--workspace --locked` passed, into the card's own target folder (deleted).

### Open items

- Leftovers outside the scratch folder that the worker rule did not let me delete (all small, harmless). If Haik wants
  them gone, in PowerShell (the last line needs an elevated shell):
  1. `Remove-Item "$env:LOCALAPPDATA\velopack\velopack.log", "$env:LOCALAPPDATA\velopack\velopack_HaikAsatryan.Wakeshot.log"`
  2. `Remove-Item "$env:LOCALAPPDATA\velopack\velopack_WakeshotTrial.log"` (left by the Phase 0 trial)
  3. `Remove-Item "$env:TEMP\velopack"` (empty)
  4. `Remove-Item 'C:\ProgramData\Microsoft\Windows\WER\ReportArchive\AppCrash_wakeshot-tray*' -Recurse`
- Not tested: GitHub upload (out of scope), `GithubSource`, a non-silent Setup (dialogs, `on_first_run`), an
  unelevated run.
