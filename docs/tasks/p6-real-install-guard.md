# Make real-settings tests refuse to run beside an installed Wakeshot

Phase: 6 (ship). Status: done (promoted 2026-11-05 by the orchestrator, before the first release; for Haik's review). Needs Haik: no.
Agent: `wakeshot-worker`. Size: about 60K tokens.
Written by the orchestrator, 2026-11-04 18:43, from Haik's question: once the real Wakeshot is installed, do dev tests
clash with it?
Runs after: `p2-6-runonce-keep`. Parallel with: cards that own none of the files below.
Real laptop: no (checks read state only; the tests use fakes or scratch keys).
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p6-real-install-guard\target`.

## Goal

`cargo test` and the `--dry-run` tray are already apart from a real Wakeshot (own lock names, own data folder). These
are not: the harness (`power_check`, Haik's step files), the real end-to-end cards, and `packaging/e2e.ps1` (it uses
the real pack id `HaikAsatryan.Wakeshot`; on a real install it logs a failed check at step 1 and, as far as the
orchestrator read, goes on to install over it and uninstall it). Each of them must stop before it changes anything
when a real Wakeshot runs or is installed, with one clear line: quit Wakeshot from its tray first (and, for a
sign-out test, turn start at login off). Even with separate names, two Wakeshots would fight over the same power
settings, so a full dev edition is not the fix.

## Read first

- `crates/wakeshot-windows/src/store.rs` `Names::real` (`Local\Wakeshot.Tray`, `Global\Wakeshot.Settings`) and
  `DataDir::real`; `crates/wakeshot-windows/src/hooks.rs` `RUN_NAME`; `power/journal.rs` `RUN_ONCE_NAME`.
- `crates/wakeshot-windows/examples/power_check.rs` (its start checks); `packaging/e2e.ps1` step 1.
- `docs/architecture.md` section 6 "Files" (install folder and data folder).

## Deliverables

1. One shared check: a real Wakeshot runs (the tray lock name is held) or is installed (the Velopack install folder or
   its HKCU Uninstall key exists), or a real journal or `WakeshotRestore` value exists. Read-only.
2. `power_check` real-settings commands and `packaging/e2e.ps1` call it first and stop on a hit, changing nothing.
3. A line in `CLAUDE.md` "Test laptop": quit the installed Wakeshot before real-settings tests (the same rule as
   for any other keep-awake app).
4. Added 2026-11-05 (`p6-int-haik-checks` finding 8): the real Startup switch writes `01` (off) and `00` (on) as the
   first byte, not `03` and `02` (Haik's block 8; Windows honoured `01`). Fix `docs/research.md` "Where Windows keeps
   the switch" (correct the bytes in place, with the date and how it was seen) and `packaging/e2e.ps1` step 5 (its
   comments, and seed the `01` form the real switch writes). Re-read `research.md` right before the edit.

## Acceptance checks

- Tests for each hit and for the clean case (fakes or scratch keys, never the real lock or folder of a real install).
- `cargo fmt`, `cargo clippy -p` (with and without `--all-targets`), `cargo test -p` on own packages pass.

## Out of scope

- A separate dev edition with other names everywhere. Change to it when Haik wants the real Wakeshot running all day
  while he tests dev builds for real (power tests would still need it quit).

## Questions for Haik

None. Worker decisions, for Haik's review (Haik was asleep; each with its reason and a "change it when"):

1. **An installed Wakeshot that does not run is only a note for `power_check`, not a stop.** The card's goal says
   "stop when it runs or is installed" and also "quit it first": both cannot hold, since quitting does not uninstall.
   Deliverable 3 and Out of scope ("power tests would still need it quit") say quit, so quit is the rule, as for any
   other keep-awake app. `e2e.ps1` still refuses an install: it installs the same pack id and uninstalls at the end.
   Change it when an installed but quit Wakeshot is seen to start during a test that does not sign out.
2. **Start at login on stops only `fake-tray` and `end-session`.** Their checks (5 and 6) cross a sign-in or a
   sign-out, where Windows would start the installed tray beside the test. "On" means the Run value `Wakeshot` with no
   switch value, or one whose state is not 1 or 3; an unknown form counts as on, so the guard errs toward stopping.
   Change it when another command gets a sign-out check.
3. **`cleanup` does not stop on a left journal or RunOnce value.** It touches only what its own run records name
   (`tidy::plan`), and a killed `fake-tray` run (checks 4 and 5) leaves exactly those in the real places, which is
   what `cleanup` is for. It still stops on a running tray and on a read failure. The other commands stop on both,
   also when a power_check run left them; the line says to end that run or run `power_check cleanup`. Change it when
   telling a power_check leftover from a real one matters (the run records hold the owner pid and the command).
4. **The facts live in `store.rs` behind `test-support`**, beside `Names`, `DataDir` and `AutoStart`, so no shipped exe
   carries them and the tests use `ScratchKey`. `e2e.ps1` cannot link Rust, so it reads the same places in
   PowerShell; it already had the install, Run, RunOnce, switch and journal checks, and gains the tray lock. Change it
   when a third, non-Rust caller appears: then add a read-only `power_check` command and call it from the scripts.
5. **A refusal exits 3** (`EXIT_REFUSED`), apart from 1 (failed) and 2 (bad arguments), so a script can tell
   "nothing changed" from "failed part way". `--plan-only` prints what would stop a real run and goes on.
6. **A read failure stops** (Rust): a lock name held by another object type, a Run value of another type, a folder
   that does not read. The PowerShell check treats another object type of the lock's name as no tray (.NET gives no
   way to tell; research "Real install guard checks").

## Found outside scope

Problems seen but not fixed.

1. `crates/wakeshot-windows/src/hooks.rs` lines 862 to 865 (test constants of `p6-hooks`): the comment and
   `SWITCH_OFF`/`SWITCH_ON` still use the web sources' state 3 and state 2, not the real switch's 1 and 12 zero bytes.
   The tests do not depend on the bytes. Fix: the comment and the two arrays, a 4-line change.
2. Old task cards (`p6-startup-switch-check`, `p6-uninstall-startup-approval`) record the `03` form they wrote at the
   time. Dated records, left as they are.

## Log

- 2026-11-04 18:43: card written by the orchestrator.
- 2026-11-05: promoted to ready by the orchestrator; deliverable 4 added. `p6-notice-app-icon` runs after this card
  (both own `packaging/e2e.ps1`); `p2-6-floor-grace` runs beside it.
- 2026-11-05: worker started; reading store, hooks, journal, power_check, e2e.ps1.
- 2026-11-05: read. `e2e.ps1` already refuses (exit 2) on install traces, the RunOnce value and a journal; it misses a
  running tray (only an INFO line). `power_check` checks only its own data folder. Plan: the facts in `store.rs`
  (`RealPlaces::signs`, test-support only), the policy and texts in `power_check`, the same reads in `e2e.ps1`.
- 2026-11-05: `store.rs`: `RealSign`, `RealPlaces` (`real`, `signs`), `switched_off`, `approval` now also under
  test-support; 9 new tests on scratch places pass (store tests 25 of 25). Next: `power_check`.
- 2026-11-05: `power_check`: `mod guard` (`changes`, `stops`, `text`, `verdict`), `real_signs`, the check in `main`
  before `run` (before the log opens), exit 3 on a refusal, USAGE paragraph; 7 new tests (29 of 29). rustfmt clean;
  clippy clean in four forms (x64). Next: smoke run of the real check (read only), then `e2e.ps1`.
- 2026-11-05 02:21: smoke: `power_check hold --plan-only` and `cleanup --plan-only` read the real places (read only):
  no sign, no note, exit 0. `e2e.ps1`: `Test-MutexExists` and the tray lock refusal first; step 5 seeds state 1;
  comments fixed. Its function tested on scratch names (parse 0 errors; missing False, made True, taken True, still
  held by its owner, gone False). A run here (elevated) passed the lock check and refused at elevation, exit 2,
  nothing made. Next: `research.md`, `CLAUDE.md`.
- 2026-11-05 02:30: `research.md`: "The value's format" corrected in place (state 1 off, 12 zero bytes on, with the
  date and how it was seen); the two lines that called `03` "the off form" (Velopack checks, the Setup-over bullet)
  and Haik's checks line now say "web sources' form" / "named"; new section "Real install guard checks". `CLAUDE.md`
  "Test laptop": one bullet. In `store.rs` the switch round-trip test's comment and bytes now use the real forms.
  `cargo test -p wakeshot-windows`: 579 passed, 23 ignored. Build folder deleted. Done.

## Result

**Done.** Before a dev tool changes anything, it now looks for a real Wakeshot and stops with one line per mark.

- **Shared check** (`crates/wakeshot-windows/src/store.rs`, behind `test-support`): `RealPlaces::real()` names the
  real places (tray lock `Local\Wakeshot.Tray`, `%LocalAppData%\HaikAsatryan.Wakeshot`, its HKCU uninstall key, the
  Run value with its switch value, `%LocalAppData%\Wakeshot\journal.json` never moved by `WAKESHOT_DATA_DIR`, RunOnce
  `WakeshotRestore`); `signs()` returns `RealSign`s, read-only (the lock is opened with `SYNCHRONIZE` and closed,
  never created or taken). `AutoStart::approval` now builds with `test-support` too.
- **`power_check`** (`crates/wakeshot-windows/examples/power_check.rs`): `mod guard` decides per command (decisions 1
  to 3 above); the check runs in `main` before the log, the harness folder or any power call; refusal exit 3; notes
  logged first; USAGE says so.
- **`packaging/e2e.ps1`**: `Test-MutexExists` and the tray-lock refusal run first (exit 2); the description names it;
  step 5 seeds the real off form (state 1 and a FILETIME) with corrected comments.
- **Docs**: `docs/research.md` (switch bytes corrected in place; new "Real install guard checks"), `CLAUDE.md`
  "Test laptop" bullet.

Checks: `store.rs` tests 25 of 25 (9 new: the real places' names, clean case, tray lock, install folder and key,
start at login by switch form, the switch forms, journal and RunOnce, all in order, unread); `power_check` tests 29 of 29 (7 new: which commands read, clean case,
tray and unread stop, install is a note, start at login stops sign-out commands, leftovers stop all but cleanup, the
full list); `cargo test --locked -p wakeshot-windows` 579 passed, 23 ignored, 0 failed. rustfmt `--check` clean on both
Rust files. `cargo clippy --locked -p wakeshot-windows --target x86_64-pc-windows-msvc -- -D warnings` clean with
`--all-targets --features test-support`, `--all-targets`, none, and `--features test-support` (aarch64 not run).
`e2e.ps1`: parses with 0 errors; its mutex check tested on scratch names; an elevated run passed the lock check and
refused at elevation (exit 2, nothing made). Real places read once (plan-only runs): no sign.

Open items: Found outside scope 1 (`hooks.rs` test bytes). Not verified: the access-denied paths (a mutex or key this
user may not open counts as present), which need a deny ACL test under a Medium token.
