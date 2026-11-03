# Store: data folder, atomic writes, Run and RunOnce, named mutexes

Phase: 3. Status: done. Needs Haik: no. Real laptop: no (temp folders; a test registry key under
`HKCU\Software\Wakeshot-test`, deleted at the end; never the real Run or RunOnce key). Agent: `wakeshot-worker`.
Size: about 90K tokens.
Runs after: Haik's sign-off of the track plans; the contracts card. Parallel with: every other wave 1 card of this
track and any card of tracks 2, 4, 5, 6.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p3-store\target`.

## Goal

Fill `store`, the shared module every process uses to find its files, write them so a stop never leaves half a file,
read them, set the Run and RunOnce values, and take the named mutexes. Tracks 4, 5 and 6 wait for this card.

## Read first

- `CLAUDE.md`; `docs/phases/03-windows-power.md` (track notes 1 to 3).
- `docs/architecture.md`: section 4 "Processes" (mutex names and `WAIT_ABANDONED`), "Tray startup" step 3 (dry run
  names), section 5 "Logon restore", section 6 (data folder, atomic write, the file table, registry values).
- `docs/research.md`: "Design verify checks" (abandoned mutex), "Distribution" (install folder vs data folder).
- The stub `crates/wakeshot-windows/src/store.rs` from the contracts card: its signatures are the contract. Keep them;
  a change goes to the orchestrator.

## Deliverables

`crates/wakeshot-windows/src/store.rs`, filled in, with its tests:

- `DataDir`: `real()` is `SHGetKnownFolderPath(FOLDERID_LocalAppData)` plus `Wakeshot`; `dry_run()` adds `dry-run`;
  `at(path)` for tests. In debug builds only, `WAKESHOT_DATA_DIR` replaces the root of `real()` (track note 3). File
  paths for every file of section 6, the `holds` and `logs` folders, `journal.json.corrupt` and
  `journal.json.corrupt.seen`.
- `write_atomic(path, bytes)`: a temp file in the same folder named with the pid and a counter, write, `FlushFileBuffers`,
  `MoveFileExW(MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH)`; on any failure remove the temp file and return the
  error; the old file stays whole.
- `read_whole(path)`: `Ok(None)` only for a missing file or folder; any other error is `Err`. Opens with read, write
  and delete sharing, so a writer's rename never fails because of a reader.
- `remove(path)`: `Ok(false)` when it did not exist.
- `Names`: `Local\Wakeshot.Tray`, `Global\Wakeshot.Settings`, `Local\Wakeshot.Journal`, each with `.DryRun` in dry
  run; `Names::test(tag)` with unique names for tests.
- `NamedMutex`: create or open; `take(timeout_ms)` returns a held guard (also on `WAIT_ABANDONED`, which the guard
  reports so the caller can log it), `None` on timeout, `Err` on a denied open; the guard releases on drop and owns its
  handle (it does not borrow the mutex).
- `AutoStart`: `real()` is `HKCU\Software\Microsoft\Windows\CurrentVersion`; `at(subkey)` for tests. `set`, `get`,
  `remove` for `Run` and `RunOnce` values (`REG_SZ`); removing a missing value is `Ok(false)`.
- Test helper, `#[cfg(test)] pub(crate)` (review M8: these tests run inside every worker's run and in parallel
  threads): `ScratchKey::new(test_name)` makes `HKCU\Software\Wakeshot-test\<pid>\<test name>` and deletes it on
  drop; `sweep_scratch_keys()` deletes only `<pid>` keys whose process is not running (an open and a zero wait; a
  reused pid only delays the delete). `p3-journal`, `p3-recovery`, `p3-stop-points`, `p6-login-and-path` and
  `p6-hooks` use it, so one rule covers every test key.

## Acceptance checks

Tests in `store::tests` (the worker may add more):

- `data_dir_paths`: real ends in `\Wakeshot`, dry run in `\Wakeshot\dry-run`, never under `HaikAsatryan.Wakeshot`;
  every file name matches section 6.
- `atomic_write_replaces_whole_file`, `atomic_write_keeps_old_file_on_failure` (target opened without delete sharing
  by the test: `Err`, old bytes intact, no temp file left), `stop_before_rename_keeps_old_file` (the internal step
  function stopped after the temp write: old file intact).
- `read_whole_tells_missing_from_error` (missing file and missing folder: `None`; a folder path: `Err`).
- `mutex_excludes_and_releases`, `abandoned_mutex_counts_as_taken` (a thread takes it and ends without release),
  `global_mutex_creates` (records the test runner's integrity level in the Log; the Medium check is in
  `p3-real-checks`).
- `auto_start_round_trip` under its `ScratchKey`; the test deletes only its own key. `sweep_skips_live_pids`: a key
  of a live pid (this process) stays, a key of an ended child's pid goes.

Commands: `rustfmt --edition 2024 --check <owned files>`; `cargo clippy --locked -p wakeshot-windows --all-targets
--target x86_64-pc-windows-msvc -- -D warnings` and the same for `aarch64-pc-windows-msvc`; `cargo test --locked -p
wakeshot-windows store::`. Record each outcome. After the tests: no `HKCU\Software\Wakeshot-test` key of this run's pids
and no `%LocalAppData%\Wakeshot` folder made by the tests.

Facts checked here (the `MoveFileExW` atomic write, Nightcrew's way, "not tested by us") go to `docs/research.md`.

- Check rule: `docs/plan.md` "How we work" step 2 and "Checks while other cards run" in this track's phase file. A
  failure in a file another card owns goes in the Log: wait and run again; if it is still there when this card's own
  work is done, list it under "Found outside scope". Never edit, format or `--fix` that file. Every cargo command takes
  `--locked`, and clippy runs once more without `--all-targets` (the libs as they ship; `CLAUDE.md` "Checks"). No
  `--workspace` or `--all` run here; the orchestrator runs those when it closes a wave.

## Out of scope

- The journal file format and its rules (`p3-journal`), log files (`p3-log`), the real RunOnce value's content
  (`p3-journal`, `p3-recovery`), the settings mutex retry (integration).
- Any file but `store.rs`; module lists; `Cargo.toml`.

## Questions for Haik

Each with the worker's default.

None. Every choice below is technical, so the worker made it.

## Found outside scope

Problems seen but not fixed.

- `docs/architecture.md` section 6 says writes go "temp file, flush, then `MoveFileExW(REPLACE_EXISTING |
  WRITE_THROUGH)` (Nightcrew's way; not tested by us)". Now tested, and not enough alone: `MoveFileExW` fails with 5
  while any reader has the target open. `store::write_atomic` falls back to a POSIX rename by handle (decision 2).
  The line needs that fallback and a pointer to `docs/research.md` "Store checks". Not this card's file.
- Temp files `<name>.<pid>.<n>.tmp` left by a process that stopped between the temp write and the move are never
  removed. They are harmless (the watch thread skips non-`.json` names) but pile up over years of crashes. Proposal:
  the tray deletes `*.tmp` files in the data folder and `holds\` whose pid is not running, at start. Not this card's
  scope (startup is integration).
- This shell runs tests at High integrity (0x3000): worker test runs here never cover Medium. `p3-real-checks` has
  the Medium check for the `Global\` mutex; other cards that care about Medium (liveness, the guard) should know.
- Seen while checking: a `clippy::chunks_exact_to_as_chunks` error and an unused `GetExitCodeProcess` import in
  `process.rs` (`p3-process`, then in progress). Both were gone in the final runs.

## Log

- 2026-11-03 17:00: card written by `p3-plan-windows-power`.
- 2026-11-03: worker started; read CLAUDE.md, plan "How we work", the card and the `store.rs` stub.
- 2026-11-03: read the phase track notes, architecture sections 4 to 6 and the research parts; checked the
  `windows` 0.62.2 signatures in the cargo registry; read Microsoft's `MoveFileExW`, `CreateMutexExW`,
  `RegDeleteTreeW` and `RegGetValueW` pages. Writing `store.rs` next.
- 2026-11-03: `store.rs` written. First clippy run (x64, `--all-targets`) stopped on `process.rs:491`
  (`clippy::chunks_exact_to_as_chunks`), a `p3-process` file: not mine, wait and run again.
- 2026-11-03: first test run: 13 of 14 passed. `atomic_write_replaces_a_file_a_reader_has_open` failed:
  `MoveFileExW(REPLACE_EXISTING | WRITE_THROUGH)` returns 5 (`ERROR_ACCESS_DENIED`) while any reader holds the target
  open, even with read, write and delete sharing. So the card's "a writer's rename never fails because of a reader"
  does not hold with `MoveFileExW` alone. Fix: on 5, a rename by handle with POSIX semantics
  (`SetFileInformationByHandle(FileRenameInfoEx)`), as Rust's `std::fs::rename` does (read its source). Second run:
  14 of 14 passed. Test runner integrity level: High (0x3000), so this shell is elevated.
- 2026-11-03: added `remove_and_rewrite_while_a_reader_has_the_file_open` (passes). Clippy clean on both targets,
  with and without `--all-targets` (the `process.rs` lint was gone by then). 15 of 15 store tests pass, 5 runs in a
  row. After the runs: no `HKCU\Software\Wakeshot-test` key at all, no `%LocalAppData%\Wakeshot`, no
  `%TEMP%\wakeshot-store-*` folder. Writing `docs/research.md` next.

- 2026-11-03: `docs/research.md` "Store checks" added. Build folder and scratch files deleted. Card done.

## Result

Status: done. Contract signatures kept.

Files changed: `crates/wakeshot-windows/src/store.rs` (filled, with 15 tests), `docs/research.md` (new section "Store
checks", additions only), this card.

What `store` does now:

- `DataDir::real()`: `SHGetKnownFolderPath(FOLDERID_LocalAppData)` plus `Wakeshot`; in debug builds a non-empty
  `WAKESHOT_DATA_DIR` replaces that root. `dry_run()` adds `dry-run` to whatever `real()` gives.
- `write_atomic`: temp file `<name>.<pid>.<n>.tmp` (`CREATE_ALWAYS`, no sharing), `WriteFile`, `FlushFileBuffers`,
  close, `MoveFileExW(REPLACE_EXISTING | WRITE_THROUGH)`; on error 5 a rename by handle with POSIX semantics; on any
  failure the temp file is deleted and the first error returned. A missing folder is made first.
- `read_whole`: `CreateFileW` with read, write and delete sharing; only errors 2 and 3 are `None`. `remove`: 2 and 3
  are `Ok(false)`.
- `Names::test(tag)`: `Local\Wakeshot.Test.<tag>.<pid>.<n>.Tray`, `Global\...Settings`, `Local\...Journal`.
- `NamedMutex::open`: `CreateMutexExW` asking only `SYNCHRONIZE | MUTEX_MODIFY_STATE`. `take`: duplicates the
  handle for the guard, then waits; `WAIT_ABANDONED` is a held guard with `abandoned() == true`.
- `AutoStart`: `RegCreateKeyExW` plus `RegSetValueExW` (`REG_SZ`), `RegGetValueW` (`RRF_RT_REG_SZ`),
  `RegDeleteKeyValueW`.
- Test only, `pub(crate)`: `ScratchKey::new(test_name)` (runs the sweep, then makes
  `HKCU\Software\Wakeshot-test\<pid>\<test name>`), drop deletes it and then its `<pid>` parent and the root when they
  are empty; `sweep_scratch_keys()`; `ScratchKey::path()` for `AutoStart::at`.

Decisions (each with when to change it):

1. `write_atomic` makes a missing folder. Nobody else creates the data folder or `holds\`, and the CLI can write before
   any tray ran. Change it when a caller must fail on a missing folder.
2. On `MoveFileExW` error 5, `SetFileInformationByHandle(FileRenameInfoEx, REPLACE_IF_EXISTS | POSIX_SEMANTICS)`.
   Tested: `MoveFileExW` alone fails while any reader has the target open, so the card's "a writer's rename never
   fails because of a reader" needs it. Rust's `std::fs::rename` does the same. Change it when a data folder on a
   file system without POSIX rename (FAT, some network shares) shows up: then a write during a read fails as before;
   add a short retry.
3. `HeldMutex` is not `Send` (it holds the raw handle); `NamedMutex` is `Send + Sync` (`unsafe impl`). Windows lets only
   the owning thread release a mutex, so a guard dropped on another thread would never release. The stub's unit
   fields made both `Send`; no code uses `HeldMutex` across threads (`power::journal` takes and drops in one call).
   For the orchestrator: an auto trait change, no signature change. Change it never (a Windows rule).
4. `CreateMutexExW` asks only the two rights a wait and a release need, so a mutex whose security grants just those
   still opens. Change it when a caller needs more rights on the mutex.
5. `AutoStart::get` on a value that is not `REG_SZ` is an error (1630), not `None`: the value is there, so "missing"
   would be wrong. Change it when a real Run value of ours shows up as `REG_EXPAND_SZ`.
6. `ScratchKey` drop also removes the empty `<pid>` key and the empty root, so a finished run leaves nothing in the
   registry. An in-process lock and a retry on `ERROR_KEY_DELETED` cover tests in parallel threads and processes.
   Change it when a registry create still flakes in a parallel run.
7. `core_error` turns an HRESULT `0x8007xxxx` into its Win32 code, so `WinError.code` is 2, 5, 32 and callers compare
   with `ERROR_*`; any other HRESULT stays whole. Change it when the error contract says otherwise.

Checks (all run 2026-11-03, `CARGO_TARGET_DIR` = this card's folder, no `RUSTFLAGS`):

- `rustfmt --edition 2024 --check crates/wakeshot-windows/src/store.rs`: pass.
- `cargo clippy --locked -p wakeshot-windows --all-targets --target <t> -- -D warnings` and the same without
  `--all-targets`, for `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`: all four pass (the first run failed on
  a `p3-process` lint in `process.rs`, gone by the final run).
- `cargo test --locked -p wakeshot-windows store::`: 15 passed, 0 failed, 5 runs in a row. The card's tests:
  `data_dir_paths`, `atomic_write_replaces_whole_file`, `atomic_write_keeps_old_file_on_failure` (returns
  `MoveFileExW` 5), `stop_before_rename_keeps_old_file`, `read_whole_tells_missing_from_error`,
  `mutex_excludes_and_releases`, `abandoned_mutex_counts_as_taken`, `global_mutex_creates` (test runner integrity
  level High, 0x3000; Medium is `p3-real-checks`), `auto_start_round_trip`, `sweep_skips_live_pids`. Added:
  `names_stay_apart`, `atomic_write_replaces_a_file_a_reader_has_open`,
  `remove_and_rewrite_while_a_reader_has_the_file_open`, `remove_tells_missing_from_error`,
  `auto_start_refuses_other_types`.
- After the tests: no `HKCU\Software\Wakeshot-test` key at all, no `%LocalAppData%\Wakeshot` folder (none before
  either), no `%TEMP%\wakeshot-store-*` folder. Build folder deleted.

Open items: the three entries under "Found outside scope" (architecture line, leftover temp files, High integrity).
