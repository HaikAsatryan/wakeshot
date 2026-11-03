# Integration: tray startup steps 3 to 7 and the `Started` event

Phase: 6. Status: done. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 100K tokens.
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

1. Minor, core (`crates/wakeshot-core/src/core.rs:159`): Automatic from the menu writes `Config::new(on)`, version 1,
   over a `config.json` of a newer version. `read_config` keeps a newer file at start (no `first_run`, so no write),
   but a menu toggle still writes over it. Architecture section 6 says a newer file is ignored; whether a user's own
   toggle may replace it is a product call. Default if nobody decides: leave it (the user chose).
2. Minor, for `p6-int-tray-mode` and `p6-int-dispatch`: `try_settings` gives `None` both for "another session owns
   it" and for a denied open (5), with no log line, so a denied open across users (architecture: not tested) does not
   show in the log. If that case needs a trace, the first try at start could log the `NamedMutex::open` error once;
   that needs a signature change here and in the dispatcher's `try_settings_owner` caller.
3. For `p6-int-tray-mode` (from `p3-session`'s notes): build `started` and read `locked_at_start` after
   `session::register(hwnd)`, so a console attach that ends, or a lock that comes, after the read still reaches the
   window. Written in both functions' docs.
4. Seen while checking, for the record: `p6-int-window`'s `window.rs` failed `--all-targets` clippy twice
   while that card was mid-edit (dead `waiting` at 114, then `assert_eq!` on `Result<&str, &FromUtf16Error>` at
   702 to 703); it passed at 00:01. Not touched.

## Log

- 2026-11-03: card written by `p6-plan-integration`.
- 2026-11-03 21:06, orchestrator, from `p3-power-io`'s Result: `PowerExecutor::startup()` takes no argument (the
  executor holds its own `Liveness`), so item 5's `startup_restore(executor, liveness)` is that call. `Startup.corrupt`
  already reports a corrupt journal: item 8 has nothing left to do. `Started.journal` cannot say "the journal did not
  read" (then nothing counts as owned, so a lid close gets no lock until the next apply). Orchestrator decision: no
  contract change; on a failed read (not corrupt) at startup, retry the read once, and log if it fails again.
- 2026-11-03 23:43, worker: started. Reading the card, architecture, phase notes and the p3 Results.
- 2026-11-03 23:53, worker: read done. Plan: the retry checks `journal.json` with `store::read_whole` (the same open
  `FileNet::load_journal` uses) after the executor's `startup`, since `Startup` cannot say the read failed; so
  `startup_restore` takes `PowerExecutor<P, FileNet<G>>`. Item 8 needs no function (`Startup.corrupt`). Writing
  `startup.rs` now.
- 2026-11-03 23:59, worker: `startup.rs` written, 22 tests pass (`cargo test --lib -- app::startup::`), rustfmt
  clean, lib clippy clean. `--all-targets` clippy stops on `window.rs:114` (`waiting` never used, `p6-int-window`'s
  file, in progress): not mine, retried later. The panic child exits 0xC0000409 with no WER report or Application
  event.
- 2026-11-04 00:03, worker: every check passes once `window.rs` built again (00:01). Result written; build folder
  deleted next.

## Result

Status: done 2026-11-04. Only `crates/wakeshot-windows/src/app/startup.rs` (code, 22 tests), `docs/research.md` (new
section "Tray startup checks", added at the end) and this card changed. No real journal, data folder, power setting,
RunOnce value or registry key was touched: temp folders, `Names::test` mutexes, `FakePlans` with the dry-run
`FileNet`.

### Public API as built (`wakeshot_windows::app::startup`)

Called by `p6-int-tray-mode` on the loop (main) thread, in this order:

1. `Boot::new(dry_run) -> Result<Boot, WinError>`; `Boot::at(dir, names, dry_run) -> Result<Boot, WinError>` for
   tests. `Boot { dir: DataDir, names: Names, dry_run, tray: ProcessRef, session: u32 }` (pub fields, `Clone`) and
   `journal_owner() -> JournalOwner`, the executor's `me`.
2. `take_tray(&Boot) -> Result<Option<TrayLock>, WinError>`; `TrayLock::abandoned()`.
3. `open_log(&Boot) -> Arc<dyn Log>`: roll, then open as `tray`; a failed roll is a Warn line; a failed open gives a
   log that drops every line.
4. `install_panic_hook(log: Arc<dyn Log>)`: one `ERROR panic:` line (thread, `file:line`, message cut to 200
   characters), then `std::process::abort()`.
5. `startup_restore(&mut PowerExecutor<P, FileNet<G>>, &dyn Log) -> Startup`: `executor.startup()`; when it gave no
   journal and `journal.json` still does not read (`store::read_whole`, the open `FileNet::load_journal` makes), one
   Info line, 500 ms, `startup()` once more; a Warn line if it still does not read. `corrupt` is either try's.
6. `try_settings(&Boot) -> Option<SettingsLock>`; `SettingsLock::abandoned()`. Also the dispatcher's tick retry.
7. `read_config(&Boot, &dyn Log) -> (Config, bool)`: missing: default and `first_run`; `Newer`, `Bad` or a failed
   read: default, no `first_run`, one Warn line, the file never written.
8. No `corrupt_left`: `Startup.corrupt` already reports a `.corrupt` file left by others and renames it `.seen`
   (orchestrator Log 21:06). Tested through `startup_restore`.
9. `started(&Boot, Config, first_run, &Startup, owner, &dyn Log) -> Event`: `tray` is `boot.tray`
   (`process::current()` at step 3), `on_console` from `session::on_console()` (`Err`: `false` and a Warn line).
10. `locked_at_start(&dyn Log) -> bool`: `session::locked_now()`, `None` is `false` with a Warn line.
11. `remove_stale_temps(&Boot, &dyn Liveness, &dyn Log)`: in the data folder and `holds\`, a `<name>.<pid>.<n>.tmp`
    file whose pid is `NotRunning`, or `Running` with a start after the file's last write, is removed with one Info
    line; `Unknown` keeps it.

### Design choices (mine)

| Choice | Reason | Change it when |
|---|---|---|
| Signatures differ from the card: `Boot::new` and `Boot::at` return `Result`; `open_log` gives `Arc<dyn Log>`; `read_config`, `started`, `locked_at_start`, `startup_restore` take a log | The folder lookup and the own-process read can fail; the log fallback is not a `LogWriter`; each needs its line | `p6-int-tray-mode` wants another shape |
| `Boot` holds this tray's pid, start and session, read once | The journal owner and `Started.tray` then name the same process, and `started` cannot fail | Never likely |
| The retry check is a second `read_whole` after `startup()`, so `startup_restore` takes only `FileNet` executors | `Startup` cannot say the read failed, and the executor's file is not mine | `Startup` says it (finding 3 fix below) |
| Retry only when the file still does not read, not when it merely exists | A journal of only a newer version's entries would run the restore twice on every start; a failure that clears in the microseconds between the two reads is missed (not likely; not verified) | A log shows a start that missed a journal |
| 500 ms between the tries | Covers a pending delete or a scan; the tray shows no icon yet | A log shows the retry failing with 5 or 32 |
| A log that cannot open gives a silent log | Keeping awake and restoring must not depend on the log | A report shows a tray with no log and no clue why |
| Panic message cut to 200 characters | A library's error text can quote its input, and the log never holds file content | A report's panic line is cut before the useful part |
| Stale temp rule: pid not running, or started after the file's last write | The hold file rule (architecture section 6): a later process cannot be the writer; a live writer's temp is never removed | A temp of ours shows up under another name form |
| `holds\` is cleaned too | `write_atomic` writes there (Keep awake, `wakeshot run`) | Never likely |
| An unknown lock state counts as unlocked (card) | Read in `core/lid.rs`: a lid close on a session locked after all looks like a failed lock and steps back toward sleep; counted as locked, a lock that really failed would go unseen | Never likely |

### Checks (all 2026-11-03 to 2026-11-04, this card's `CARGO_TARGET_DIR`, no `RUSTFLAGS`)

- `rustfmt --edition 2024 --check crates/wakeshot-windows/src/app/startup.rs`: exit 0 (after one `rustfmt` on the
  file itself; it declares no child module).
- `cargo clippy --locked -p wakeshot-windows --all-targets --target <t> -- -D warnings` and the same without
  `--all-targets`, for `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`: all four clean (the `--all-targets`
  runs first failed in `window.rs`, another card's file mid-edit; clean at 00:01).
- `cargo test --locked -p wakeshot-windows -- app::startup::`: 22 passed, 0 failed, four runs. The card's checks:
  `second_tray_is_refused_while_the_first_holds_the_lock`; `read_config_missing_is_the_first_run`,
  `read_config_newer_keeps_the_file_byte_for_byte`, `read_config_bad_gives_the_default_and_a_line`;
  `corrupt_journal_left_by_others_is_reported_once`; `startup_restore_restores_a_gone_owner`,
  `startup_restore_keeps_a_live_owners_entries` (alive and Unknown); `started_carries_this_tray`;
  `panic_hook_logs_then_aborts` (child exits 0xC0000409, not 101; one panic line). Added: the retry
  (`a_journal_read_that_fails_is_tried_once_more`, `a_journal_that_still_does_not_read_is_logged`,
  `startup_restore_without_a_journal_tries_once`), `a_tray_lock_left_by_an_ended_tray_counts_as_taken`,
  `settings_lock_has_one_owner`, `read_config_takes_its_own_version`, `boot_names_and_folders`,
  `lock_state_at_start`, `temp_names_give_their_pid`, `stale_temp_files_are_removed`,
  `stale_temps_in_a_new_folder_need_nothing`, `open_log_rolls_then_opens_as_tray`, `open_log_makes_the_folder`.
- `cargo test --locked -p wakeshot-windows`: 314 passed, 0 failed, 17 ignored.
- After the runs: no `%TEMP%\wakeshot-startup-*` folder, no `%LocalAppData%\Wakeshot`, no WER report or Application
  crash event. Build folder deleted.

### Power safety review finding 3: what the core-side fix needs from startup

The retry here narrows the gap only. The fix the review proposes (a "journal did not read" flag in `Started`, so the
core sets `retry = Some(Leftover)` with an empty copy) needs from startup only that the flag gets filled:

- the source: either `Startup` gains `unread: bool` in `power/executor.rs` (`p3-power-io`'s file; the executor sees
  `JournalRead::Failed` directly, the cleaner way), or `startup_restore` returns the result of its own check after the
  retry (`unreadable_journal`, already computed);
- `started()` copies it into the new `Started` field: about three lines in `startup.rs`;
- nothing else: the executor's `Leftover` path already answers a failed read with `Err` and `[]`, so the core would
  retry on every tick. The 500 ms retry can stay as a quick first try or go.

### Open items

- The four entries under "Found outside scope".
- `p6-int-tray-mode` calls these in the order above and posts `JournalCorrupt` when `Startup.corrupt`.
