# What the install, update and uninstall hooks do

Phase: 6. Status: ready. Needs Haik: no; the data folder step follows his answer to Question 2 of `p6-plan-ship`: the
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
  first, then the restore every 2 s until 25 s (install, uninstall) or 12 s (updated); the obsolete hook only logs.
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

Each with the worker's default.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-03: card written by `p6-plan-ship`.

## Result

What was done, files changed, checks and outcomes, open items.
