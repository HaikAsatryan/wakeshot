# What the install, update and uninstall hooks do

Phase: 6. Status: done. Needs Haik: no; the data folder step follows his answer to Question 2 of `p6-plan-ship`: the
default (`docs/phases/02-06-signoff.md` item 6, Haik 2026-11-03 18:09). Agent: `wakeshot-worker`. Size: about 100K
tokens.
Real laptop: no. Tests use fakes, a temp data folder and scratch registry keys
(`HKCU\Software\Wakeshot-test\<pid>\<test name>`: each test deletes only its own; the leftover sweep at the start
deletes only keys whose pid is not running: `p3-store`'s `ScratchKey`; review M8); the real hooks run only in
`p6-install-e2e`.
Runs after: `p6-login-and-path`; the power track's `p3-store` (data folder path, the RunOnce value,
`store::AutoStart` for the Run value), `p3-log` (`LogWriter`), `p3-process` (liveness), `p3-journal`
(`restore_locked`, the hook form of `FileNet`) and `p3-power-plan` (`Win32Power`); the core's `p2-ownership-restore`
(review M1: these replace the power track's restore card, dropped at 17:17). Parallel with: any card that does not own
`crates/wakeshot-windows/src/hooks.rs`.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p6-hooks\target`.

## Goal

Each Velopack hook does its steps in a safe order and within its time limit, so an install, update or uninstall
during a hold puts the user's settings back and leaves no Run value, RunOnce value or Path entry behind.

## Read first

- `docs/phases/06-ship.md`: "Hooks", and "Package and install values".
- `docs/architecture.md` section 5: the restore routine (step 7: hooks show no UI, retry within the limit), "Gap
  left" (the uninstall case); section 6: "Velopack hooks", "Registry values"; section 8 (log: process `hook`).
- `docs/research/distribution.md` section 1: what Update.exe and Setup.exe do, hook limits.
- `crates/wakeshot-windows/src/install.rs` (`p6-login-and-path` Result); the power track's restore, liveness,
  `store` and `log` modules (their cards' Results).

## Deliverables

`crates/wakeshot-windows/src/hooks.rs`, owned whole. Any data file a hook reads goes through
`wakeshot_core::files::read` (`p1-contracts-split` edit 6); the journal through `FileNet`.

- Trait `HookEnv`: set and remove the Run value; add and remove the Path entry; remove the RunOnce value; one restore
  attempt "if a journal exists and its owner is Gone", answering done, nothing to do, owner alive or Unknown, or
  failed; the data folder step; a monotonic now; a wait; a log line.
- `on_install`, `on_obsolete`, `on_updated`, `on_uninstall`, following the table in the phase file: registry steps
  first, then the restore every 2 s until 25 s (install, uninstall) or 10 s (updated; Velopack kills it at 15 s, `p6-velopack-facts` Result); the obsolete hook only logs.
- Uninstall's data folder step, by Question 2 (answered: the default): after a successful restore delete
  `%LocalAppData%\Wakeshot` whole; after a failed one keep `journal.json` and `logs\` (a reinstall's install hook
  restores; the log holds the old values) and delete the rest. A data folder this hook cannot delete is a log line.
- Never panic and never show UI: each failure is a log line, and the hook returns.
- `RealHookEnv`: `install.rs` for the Path entry; `store::AutoStart` for the Run value (one writer of that value,
  review m12); one restore attempt is `power::journal::restore_locked` with `Win32Power` and the hook form of
  `FileNet`, the same single call the guard's retry loop makes (the hook's own loop and limits stay here, review m12);
  the power track's code for liveness, RunOnce and the data folder path; `LogWriter` with process `hook`. Its
  locations come from its constructor, so tests can point it at a temp folder and the scratch key.
- The three tray icon GUIDs' `HKCU\Control Panel\NotifyIconSettings` entries (`05-tray-ux.md`, "Where other tracks
  meet this one"; review m17): the uninstall hook leaves them. They are Windows' own record of icons it has seen, not a
  value Wakeshot wrote; the README says so (`p6-readme`). Change it when Haik wants a fully clean uninstall: then
  remove the three entries by GUID here, after the restore.

## Acceptance checks

- Tests written first, with a fake `HookEnv` and a fake clock:
  - each hook's calls in order;
  - a restore that fails twice, then works;
  - a restore that fails until the limit: Run, RunOnce and Path are still removed; the journal and logs stay;
  - a restore attempt that hangs (the fake clock jumps 40 s inside it): the registry steps ran before it;
  - owner alive, owner Unknown: no restore, one log line;
  - the obsolete hook calls nothing but the log;
  - the data folder step after success and after failure.
- `RealHookEnv` against a temp data folder, the scratch registry key and `FakePlans` or `DryRunPower`: an uninstall
  with a journal whose owner is a dead pid restores, removes, deletes.
- `rustfmt --edition 2024 --check <owned files>`; `cargo clippy --locked -p wakeshot-windows --all-targets --target <t>
  -- -D warnings` for both targets; `cargo test --locked -p wakeshot-windows hooks::`. Clean in the owned files; the own
  tests pass.
- No test writes the real Run, RunOnce or Environment keys, or the real data folder.
- Check rule: `docs/plan.md` "How we work" step 2 and "Checks while other cards run" in this track's phase file. A
  failure in a file another card owns goes in the Log: wait and run again; if it is still there when this card's own
  work is done, list it under "Found outside scope". Never edit, format or `--fix` that file. Every cargo command takes
  `--locked`, and clippy runs once more without `--all-targets` (the libs as they ship; `CLAUDE.md` "Checks"). No
  `--workspace` or `--all` run here; the orchestrator runs those when it closes a wave.

## Out of scope

- `velopack` and `velo.rs` (`p6-velopack-dependency` calls these functions from the callbacks); the restore routine
  itself (power track); a real install (`p6-install-e2e`).

## Questions for Haik

None blocking. The calls the worker made for Haik's review are in Result, "Decided by the worker".

## Found outside scope

- `docs/phases/06-ship.md`, "Hooks" table, updated row: "then restore as at install, until 12 s" should read "until
  10 s" (Velopack kills the hook at 15 s; `p6-velopack-facts` Result). Phase files are the orchestrator's.
- `docs/architecture.md` section 6, "Velopack hooks": still says the uninstall "restores, retrying for up to 25 s
  ..., then removes Run, RunOnce and the Path entry". The built order is the phase file's: Run, RunOnce and Path
  first, then the restore, then the data folder. Section 5, restore step 7, still says "the hooks are not built yet".
  For the next docs pass.
- Not a problem, for the record: `%LocalAppData%\Wakeshot` (only `dry-run\`, made 01:39 by `p4-live-claude`'s dry
  run) was there at 01:57 and gone at 02:00. Not this card: no test here resolves the real folder (only
  `RealHookEnv::installed()` does, and no test calls it); `p4-live-claude`'s Log shows it made that folder.

## Needs another card

- `crates/wakeshot-tray/src/velo.rs` (`p6-velopack-dependency`): the four callbacks, exactly:
  `.on_after_install_fast_callback(|v| hooks::on_install(&mut RealHookEnv::installed(), &v.to_string()))`,
  `.on_before_update_fast_callback(|v| hooks::on_obsolete(&mut RealHookEnv::installed(), &v.to_string()))`,
  `.on_after_update_fast_callback(|v| hooks::on_updated(&mut RealHookEnv::installed(), &v.to_string()))`,
  `.on_before_uninstall_fast_callback(|v| hooks::on_uninstall(&mut RealHookEnv::installed(), &v.to_string()))`,
  with `use wakeshot_windows::hooks::{self, RealHookEnv};`.
- Optional, `crates/wakeshot-windows/src/log.rs` (`p3-log`): make `LOG_FILE` public, so `hooks.rs` can drop its own
  copy of `"wakeshot.log"` (a test checks the two agree for now).
- Optional, `crates/wakeshot-windows/src/power/journal.rs` (`p3-journal`): a `restore_locked` form that takes the
  caller's lock wait. The updated hook's last try (at 10 s at most) can wait up to `JOURNAL_WAIT_MS` (10 s) for
  `Local\Wakeshot.Journal`, past Velopack's 15 s kill. Safe as it is: a stop while waiting writes nothing, every stop
  inside the routine keeps the values reachable, and the lock is free at a hook, since Velopack has killed every tray.
  Change it when an install log shows a hook killed while it waited.

## Log

- 2026-11-03: card written by `p6-plan-ship`.
- 2026-11-04 01:48: started by `wakeshot-worker`. Read the card, `06-ship.md` "Hooks", architecture sections 5, 6
  and 8, `research/distribution.md` section 1, `p6-velopack-facts` and `p6-login-and-path` Results, and the code of
  `install`, `store`, `log`, `process`, `power::journal` and `recovery`.
- 2026-11-04 01:50: updated hook limit: 10 s, not 12 s. `p6-velopack-facts` Result ("Design calls": the hook runs a
  just-extracted exe that took up to 1.7 s to start, so 12 s of 15 s leaves about 1.3 s) and `architecture.md`
  section 5, restore step 7 ("10 s for updated") both say 10 s; only `06-ship.md` "Hooks" still says 12 s. The card
  keeps 10 s; the phase file line is listed for the orchestrator.
- 2026-11-04 01:56: `hooks.rs` written (trait `HookEnv`, the four hooks, `RealHookEnv`, the data folder step) with
  10 fake-env tests and 7 `RealHookEnv` tests; first run of `cargo test --locked -p wakeshot-windows hooks::`: 17
  passed, 0 failed. A `DeleteFileW` of the log while this process's `LogWriter` has it open succeeds and the folder
  then goes too (delete sharing; research note to add).
- 2026-11-04 01:57: snapshot of the real values before the next runs: `reg query` of `Run`, `RunOnce` and
  `HKCU\Environment` `Path`, SHA-256 `1021b3bc...0baa9d07`; `%LocalAppData%\Wakeshot` holds only `dry-run\`
  (01:39, another card's); no `HKCU\Software\Wakeshot-test` key; no `%TEMP%\wakeshot-hooks-*` folder.
- 2026-11-04 01:58: small clean-ups (one error text helper; `remove_entry` maps "not found" before the code), `rustfmt
  --edition 2024`; tests again with `--nocapture`: 17 passed. A file opened with `FILE_SHARE_READ` only stops
  `remove_dir_all` of `holds\` with 32.
- 2026-11-04 01:59: clippy x64 and ARM64, with and without `--all-targets`: all four exit 0, no warning. Release build
  x64 of `wakeshot-windows`: exit 0.
- 2026-11-04 02:00: real values after every run: the same SHA-256 `1021b3bc...0baa9d07`; no `Wakeshot-test` key and
  no `wakeshot-hooks-*` folder left. `docs/research.md`: section "Hook checks" added.

## Result

Done 2026-11-04 02:02.

What was built: `crates/wakeshot-windows/src/hooks.rs`, owned whole. No other code file changed.

API (for `velo.rs`, `p6-install-e2e` and the integration cards; `p6-int-tray-mode` needs nothing from it):

```rust
pub const RUN_NAME: &str = "Wakeshot";          // value name under HKCU\...\CurrentVersion\Run
pub const INSTALL_LIMIT_MS: u64 = 25_000;       // restore tries, counted from the hook's start
pub const UPDATED_LIMIT_MS: u64 = 10_000;
pub const UNINSTALL_LIMIT_MS: u64 = 25_000;
pub const RETRY_EVERY_MS: u64 = 2_000;
pub fn on_install(env: &mut impl HookEnv, version: &str);   // Run value, Path entry, then restore
pub fn on_obsolete(env: &mut impl HookEnv, version: &str);  // one log line
pub fn on_updated(env: &mut impl HookEnv, version: &str);   // Path entry, then restore
pub fn on_uninstall(env: &mut impl HookEnv, version: &str); // Run, RunOnce, Path; restore; data folder step
pub trait HookEnv { set_run, remove_run, add_path, remove_path, remove_run_once, try_restore -> Attempt,
                    clean_data_folder(DataFolder) -> Vec<Undeleted>, now_ms, wait_ms, log }
pub enum Attempt { Restored, Nothing, OwnerAlive(JournalOwner), OwnerUnknown(JournalOwner),
                   Unrestorable(Unrestorable /* DoesNotParse | NewerVersion */), Failed(Failure) }
pub enum DataFolder { DeleteAll, KeepJournalAndLogs }
pub struct Undeleted { pub path: PathBuf, pub error: WinError }
pub struct Places { pub dir: DataDir, pub auto: AutoStart, pub path: UserPath, pub names: Names, pub exe: PathBuf }
impl RealHookEnv<Win32Power> { pub fn installed() -> Self }  // never fails
impl<P: PlanIo> RealHookEnv<P> { pub fn new(Places, P, Box<dyn Liveness>, Arc<dyn Log>) -> Self }
```

What `p6-install-e2e` can check: the Run value is `"<install>\current\wakeshot-tray.exe"` (quoted, no arguments);
the Path entry is `<install>\current`; the log (process `hook`, area `hook`) starts each hook with "`<hook>` hook for
version `<v>` started" (proof that `VelopackApp` ran it) and ends with "`<hook>` hook done after `<n>` s"; a restore
that worked says "restore: the journal's values are back"; after an uninstall whose restore worked,
`%LocalAppData%\Wakeshot` is gone, otherwise only `journal.json` and `logs\` are left.

How it works:

- Each hook reads its start time first. The restore tries run while "now + 2 s" is within the limit from that start,
  so the registry steps' time (a Path broadcast waits up to 1 s per window) comes out of it. Install and uninstall: 13
  tries at most (0 to 24 s); updated: 6 (0 to 10 s).
- One attempt (`RealHookEnv::try_restore`): load the journal through `FileNet::restore_only` without the lock;
  missing: `Nothing`; does not parse: `Unrestorable(DoesNotParse)`, left in place; owner alive or Unknown
  (`SystemLiveness`): no restore; Gone: `restore_locked(plans, net, None, Some(owner), names, log)`, the same call the
  guard and restore mode make. A journal another tray took over in between counts as `OwnerAlive`. After a run that
  worked, a journal still on disk holds only a newer version's entries: `Unrestorable(NewerVersion)`.
- Only `Failed` is tried again. Every other answer ends the tries with one log line.
- The uninstall's data folder step: `DeleteAll` after `Restored` or `Nothing`, otherwise `KeepJournalAndLogs`.
  `DeleteAll` deletes the log last and only when everything else went, so a folder that cannot go whole keeps the
  log that says why; each thing left is one Error line.
- `RealHookEnv::installed()` never fails: a data folder or exe path that cannot be found fails only the steps that
  need it (no log when there is no folder), so the registry steps still run.

Checks (all 2026-11-04):

- `cargo test --locked -p wakeshot-windows hooks::`: 17 passed, 0 failed (two runs). Fake env (10 tests): each
  hook's calls in order; fails twice then works (tries at 0, 2, 4 s); fails until the limit (13, 6, 13 tries; Run,
  RunOnce and Path removed first; `KeepJournalAndLogs`); a try that hangs 40 s (registry steps before it, no second
  try); owner alive and Unknown (one try, one owner line); unrestorable journals; the data folder step for each answer
  and a line per thing left; registry failures are Error lines and the hook goes on, a failed broadcast a Warn; slow
  registry steps shorten the tries, and one try runs even past the limit; obsolete only logs. `RealHookEnv` (7 tests)
  over a temp data folder, one scratch key (`ScratchKey`) for Run, RunOnce and Path, `FakePlans` and test mutex
  names: an uninstall with a journal whose owner is a dead pid (a `cmd.exe` the test started and ended) restores,
  removes Run, RunOnce and only our Path entry, and deletes the data folder with the log open; an install writes the
  quoted Run value and the Path entry and restores, then the updated hook finds the entry and no journal; each
  `try_restore` answer; `KeepJournalAndLogs` on every kind of file; `DeleteAll` with the log open; a file open
  without delete sharing keeps `holds\` and the log; no data folder or exe path.
- Tests and code were written in the same pass, not tests first; every test ran against the finished code.
- `rustfmt --edition 2024 --check crates/wakeshot-windows/src/hooks.rs`: clean.
- `cargo clippy --locked -p wakeshot-windows --all-targets --target <t> -- -D warnings` and the same without
  `--all-targets`, for `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`: all four clean.
- `cargo build --release --locked -p wakeshot-windows --target x86_64-pc-windows-msvc`: exit 0.
- No test touches the real Run, RunOnce or Environment keys or the real data folder: the `reg query` hash of the three
  is the same before and after the runs (Log 01:57 and 02:00); no scratch key or temp folder left.
- No other card's file failed any of these runs. The build folder is deleted.

Decided by the worker, for Haik's review (his rule: with no work going on, the laptop behaves as if Wakeshot were not
there):

1. Updated hook limit 10 s, not 12 s: the facts card's measurement and `architecture.md` say 10 s; only the phase
   file says 12 s (listed above). Change it when a measured first start of the new exe is well under 1 s.
2. Owner Unknown at a hook: no restore, one line, no retry (the card's rule). At uninstall the journal and logs stay.
   Change it when a log shows Unknown at a hook that a retry would have fixed.
3. A journal that does not parse: the hook leaves it where it is (no set-aside, no retry); the tray, which can warn,
   sets it aside at its next start. At uninstall it counts as not restored, so `journal.json` and `logs\` stay.
   Change it when a hook gets a way to tell the user.
4. A journal that holds only a newer version's entries (a downgrade, then an uninstall): kept like a failed restore,
   so a reinstall of that version can still restore. Change it when the journal format changes.
5. A journal owner that is alive at uninstall (a dev build run from elsewhere): no restore; `journal.json` and `logs\`
   stay, the rest of the folder goes. Change it when a real user runs a second copy beside the installed one.
6. One restore try always runs, even when the registry steps used up the whole limit: a try stopped by Velopack at any
   point keeps the values reachable. Change it when Velopack's kill is found to cost more than a clean exit.
7. `KeepJournalAndLogs` keeps exactly `journal.json` and `logs\` (the card's rule): `journal.json.corrupt`, holds,
   config, state and `dry-run\` go. Change it when an unseen `journal.json.corrupt` should survive for a reinstall's
   notice.
8. The updated hook does not write the Run value (the phase table), so a Run value the user removed stays removed;
   the install hook writes it every time, also on Setup.exe over an install.

Open items: "Needs another card" (the `velo.rs` calls; two optional changes) and the two doc lines under "Found
outside scope".

Files changed: `crates/wakeshot-windows/src/hooks.rs`, `docs/tasks/p6-hooks.md`, `docs/research.md` (new section
"Hook checks").
