# Store: data folder, atomic writes, Run and RunOnce, named mutexes

Phase: 3. Status: ready. Needs Haik: no. Real laptop: no (temp folders; a test registry key under
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

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-03 17:00: card written by `p3-plan-windows-power`.

## Result

What was done, files changed, checks and outcomes, open items.
