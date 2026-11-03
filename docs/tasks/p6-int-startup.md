# Integration: tray startup steps 3 to 7 and the `Started` event

Phase: 6. Status: ready. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 100K tokens.
Real laptop: no (temp data folders, test mutex names from `Names::test`, `FakePlans`; never the real journal).
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p6-int-startup\target`.
Runs after: `p6-int-skeleton`, `p3-store`, `p3-log`, `p3-process`, `p3-session` (`on_console`), `p3-journal`,
`p3-power-io` (`PowerExecutor::startup`). Parallel with: `p6-int-io-thread`, `p6-int-window`, `p6-int-dispatch` and any
track card.

## Goal

Everything the tray does before its window exists, as small functions `p6-int-tray-mode` calls in order: the data
folder and names, the one-tray mutex, the log and panic hook, the startup restore, the settings mutex, a corrupt
journal left by another process, and the `Started` event with every field right.

## Read first

- `docs/phases/06-integration.md`: notes 1 and 6.
- `docs/architecture.md` section 4, "Tray startup" (steps 3 to 7) and "Processes" (mutexes); section 6 (`config.json`,
  the version rule, the journal); section 8 (panic row, log rolling).
- `docs/phases/03-windows-power.md` notes 7 (corrupt journal: `.corrupt`, then `.seen`) and 8 (dry run).
- The Results of `p3-store` (`DataDir`, `Names`, `NamedMutex`), `p3-log` (`LogWriter::open`, `roll`), `p3-process`
  (`current`), `p3-session` (`on_console`), `p3-power-io` (`PowerExecutor::startup`, `Startup`), `p3-journal`.
- `crates/wakeshot-core/src/event.rs` (`Started`), `files.rs` (`Config`, `read`).

## Deliverables

`crates/wakeshot-windows/src/app/startup.rs`, owned whole:

1. `Boot::new(dry_run) -> Boot`: the data folder (`DataDir::real` or `dry_run`; `WAKESHOT_DATA_DIR` in debug builds is
   `p3-store`'s) and the mutex names (with `.DryRun` in dry run).
2. `take_tray(&Boot) -> Result<Option<TrayLock>, WinError>`: `Local\Wakeshot.Tray`; `None` when another tray has it
   (the caller exits 0).
3. `open_log(&Boot) -> Arc<LogWriter>`: roll at start (over 1 MB, three files), then open with process `tray`.
4. `install_panic_hook(log)`: log the panic (message and place, no file content), then `std::process::abort()`. Only
   the tray's startup calls it (architecture step 5).
5. `startup_restore(executor, liveness) -> Startup`: the executor's startup restore (step 6), run before any other
   thread starts.
6. `try_settings(&Boot) -> Option<SettingsLock>` (step 7); the dispatcher's tick retry uses the same function.
7. `read_config(&Boot) -> (Config, first_run: bool)`: missing file: the default and `first_run`; `Newer`: the default,
   no `first_run`, one log line, and the file is never written over (section 6 version rule); `Bad`: the default, a
   log line.
8. `corrupt_left(&Boot) -> bool`: a `journal.json.corrupt` left by restore mode, the guard or a hook is renamed to
   `journal.json.corrupt.seen` and reported once, so `p6-int-tray-mode` passes `JournalCorrupt` after `Started`. If the
   executor's startup already reports it (read `p3-power-io`'s Result), this function only says so and does nothing.
9. `started(config, first_run, startup, owner, ..) -> Event`: `Started { config, journal, owner, on_console, tray,
   first_run }`, `tray` from `process::current()`, `on_console` from `session::on_console()`.
10. Added by the orchestrator (`docs/reviews/p1-close-verify.md` Major 1): `locked_at_start() -> bool` from
    `session::locked_now()` (`Some(true)` only; `None` counts as unlocked, with one log line), so `p6-int-tray-mode`
    passes `Session(SessionChange::Locked)` right after `Started`, before `JournalCorrupt` and `Updated`.
11. Added by the orchestrator (`p3-store` Found outside scope): a temp file left by a stop between the temp write and
    the move is never cleaned up. At start, delete the data folder's `*.tmp` files whose pid is not alive (read
    `p3-store`'s Result for the temp name form), one log line each, with a test. `store::HeldMutex` is not `Send`
    (Windows lets only the owning thread release a mutex): take and hold the tray's mutexes on the loop thread.

## Acceptance checks

- Tests first, in `app::startup::tests`, on temp folders and `Names::test` names:
  - a second `take_tray` with the same names gives `None` while the first is held, and `Some` after it is dropped;
  - `read_config`: missing gives `first_run`; a newer version keeps the file byte for byte; a bad file gives the
    default and a log line;
  - `corrupt_left`: a `.corrupt` file is renamed to `.seen` and reported once; a second call reports nothing;
  - `startup_restore` with `FakePlans`: a journal whose owner is Gone is restored and gives `journal: None`; an owner
    that is alive keeps the entries in `Startup.journal`;
  - `started` carries this process's pid and start time and the given `owner` and `first_run`;
  - the panic hook: a child run of the test binary (an env var picks the case) panics, the log file holds one panic
    line, and the exit code is the abort's, not a normal panic's.
- `rustfmt --edition 2024 --check crates/wakeshot-windows/src/app/startup.rs`.
- `cargo clippy --locked -p wakeshot-windows --all-targets --target x86_64-pc-windows-msvc -- -D warnings` and the same
  without `--all-targets`; both with `--target aarch64-pc-windows-msvc`: nothing in the owned file.
- `cargo test --locked -p wakeshot-windows -- app::startup::` passes.
- Build folder deleted at the end.

## Out of scope

- Creating the window and threads (`p6-int-tray-mode`); the executor's restore itself (`p3-power-io`); any track file.

## Questions for Haik

None.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-03: card written by `p6-plan-integration`.

## Result

What was done, files changed, checks and outcomes, open items. Include the public API as built.
