# Integration: tray mode, the composition root

Phase: 6. Status: ready. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 140K tokens.
Real laptop: Part A no. Part B: **screen**, about 10 minutes, one dry-run tray icon on the taskbar (one screen or sound
card at a time, across all tracks); no power setting, no sound on purpose, no Haik.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p6-int-tray-mode\target`.
Runs after: `p6-int-io-thread`, `p6-int-window`, `p6-int-dispatch`, `p6-int-startup`, `p3-requests`,
`p3-system-events`, `p3-hibernate`, `p3-recovery`, `p4-watch-thread`, `p4-claude-transcript`, `p4-codex-source`,
`p4-copilot-source`, `p4-cron`, `p5-tray-icon`, `p5-menu-view`, `p5-notices`, `p5-sounds`. Parallel with: any card
that owns none of the files below.

## Goal

`tray_mode::run`: build every real (or dry-run) object, start the threads in the startup order, run the pump, and shut
down in order, so `wakeshot-tray.exe` and `wakeshot-tray.exe --dry-run` run the whole product.

## Read first

- `docs/phases/06-integration.md`: all notes, mainly 4 (runner table) and 6 (startup and shutdown order).
- `docs/architecture.md` section 4 ("Tray threads", "Tray startup"), section 5 ("Lock and hibernate", "Tray, notices,
  sounds").
- `docs/phases/05-tray-ux.md` note 5 (integration notes) and note 4 (demo rules: Medium start, the dry-run GUID, the
  `NotifyIconSettings` clean-up).
- The Results of `p6-int-io-thread`, `p6-int-window`, `p6-int-dispatch`, `p6-int-startup`, `p3-requests`,
  `p3-session` (`Locker`, `register`), `p3-system-events` (`Subscriptions`), `p3-hibernate` (`run_hibernate`,
  `HibernateDeps`), `p3-recovery` (`TrayGuard`, `NoGuard`), `p3-power-io`, `p3-power-plan`, `p4-watch-thread`
  (`WatchThread::start`), the three source cards and `p4-cron` (`SystemZone`), `p5-tray-icon` (`TrayIcon`,
  `IconKind`), `p5-menu-view`, `p5-notices` (`BalloonNotifier`, `take_click`), `p5-sounds` (`WasapiPlayer`),
  `p4-cli-commands` (`shell`).

## Deliverables

1. `crates/wakeshot-windows/src/app/tray_mode.rs`, owned whole:
   - `run(dry_run: bool) -> ExitCode` in the order of note 6, steps 3 to 9. Real or dry run per object: `Win32Power`
     or `DryRunPower`, `PowerRequests` or `DryRunRequests`, `Win32Locker` or `DryRunLocker`, `Win32Hibernate` or
     `DryRunHibernate`, `TrayGuard` or `NoGuard` (and no RunOnce in dry run), `IconKind::Product` or `DryRun`.
   - `RealRunners: LoopRunners` over those objects, the `TrayIcon`, the `Arc<dyn SoundPlayer>`, the settings lock,
     the `WatchHandle` and the sink; `start_hibernate` spawns one thread that runs `run_hibernate` and posts
     `Done(Hibernate(end))`; `apply_update` answers `Done(Update(Err(..)))` with a log line until
     `p6-int-update-wiring` connects the update thread.
   - The balloon click hook: `take_click()`, then `shell::open_with_default_app` with its target (the log, or
     `ms-settings:taskbar` for the first-run notice). If `BalloonNotifier` cannot be shared with the IO thread (not
     `Sync`), stop and report it as a contract gap.
   - Each start failure that leaves the tray useless (window, IO thread) logs and exits non-zero, after the restore
     steps already done; an icon add failure is `TrayIcon`'s retry, not a failure.
2. `crates/wakeshot-tray/tests/smoke.rs`, owned whole: `real_e2e_smoke_dry_run` (`#[ignore]`): builds paths to the
   tray exe (`CARGO_BIN_EXE_wakeshot-tray`), starts it with `--dry-run` at Medium through `explorer.exe <launch .cmd>`
   with `WAKESHOT_DATA_DIR` on a temp folder, waits for `state.json`, writes a script hold `holds\smoke.json` with the
   test's own pid (`docs/hold-files.md`), checks `state.json` shows holding within 10 s, deletes the hold, checks idle,
   quits with `ask_quit`, and checks the dry-run journal is gone and the process ended with code 0.
3. Part B, in a screen slot: run the smoke test once; one picture of the taskbar with the dry-run icon holding
   (`docs/research/img/ux/int-smoke-*.png`, cropped as `05-tray-ux.md` note 4 says); then export and delete the
   dry-run GUID's `NotifyIconSettings` entry and its `NotifyIconGeneratedAumid_` key, and check both are gone a minute
   later.

## Acceptance checks

- Tests first: the smoke test above, written before the body. Unit tests for `RealRunners` parts that need no screen
  (the hibernate thread posts exactly one `Done(Hibernate)` with `DryRunHibernate`; `apply_update` answers `Err`).
- `rustfmt --edition 2024 --check crates/wakeshot-windows/src/app/tray_mode.rs crates/wakeshot-tray/tests/smoke.rs`.
- `cargo clippy --locked -p wakeshot-windows --all-targets --target <t> -- -D warnings` and without `--all-targets`;
  `cargo clippy --locked -p wakeshot-tray --all-targets --target <t> -- -D warnings` and without; both targets (the
  tray's ARM64 runs need clang once `velopack` is in).
- `cargo test --locked -p wakeshot-windows -- app::tray_mode::` passes.
- Part B: `cargo test --locked -p wakeshot-tray --target x86_64-pc-windows-msvc --test smoke -- --ignored
  --test-threads=1` passes; the picture exists; the dry-run entries are gone a minute later; no process this card
  started is left (by pid).
- Build folder deleted at the end.

## Out of scope

- The update thread and `velo.rs` (`p6-int-update-wiring`); the process test suite (`p6-int-dry-run-e2e`); any real
  power setting; any track file (a bug there goes under "Found outside scope").

## Questions for Haik

None.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-03: card written by `p6-plan-integration`.

## Result

What was done, files changed, checks and outcomes, open items.
