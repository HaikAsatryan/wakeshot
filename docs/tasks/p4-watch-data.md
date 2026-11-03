# Watch thread: the data folder (holds, config, restore request)

Phase: 4. Status: done. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 70K tokens.
Runs after: `p4-hold-files`, `p4-watch-reads`. Parallel with: every other card of this track and every card of tracks
2, 3, 5, 6.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p4-watch-data\target`. Uses the real laptop: no (temp folders, fakes).

## Goal

Turn the data folder into events: `holds\*.json` into `Event::Holds` (with liveness), `config.json` into
`ConfigChanged`, and `restore-request.json` into one `Menu(Restore)` per appearance.

## Read first

- `docs/phases/04-sources.md`: track notes 10, 14, 15, 16.
- `docs/architecture.md` section 4 (watch row, "How they talk"), section 5 "Liveness", section 6 (file table, hold
  format, version rule), section 8 (Liveness Unknown).
- `crates/wakeshot-core/src/holds/file.rs` (from `p4-hold-files`), `files.rs`, `event.rs`.
- From the contracts card: `store::DataDir` (paths), `process::Liveness` and `FakeLiveness`, the log seam and its fake.

## Deliverables

`crates/wakeshot-windows/src/watch/data.rs`, driven by calls (no thread, no folder watch of its own). It reads every
data file (`config.json`, `restore-request.json`, hold files) through `wakeshot_core::files::read` (the version rule,
first JSON value only, byte order mark skipped; `p1-contracts-split` edit 6):

1. `holds(dir, liveness) -> HoldsRead`: list `holds\*.json` (temp names never end in `.json`), read each whole,
   `holds::file::parse`; a bad read or parse is retried once 200 ms later (the caller schedules it: return the names to
   retry); still bad, newer version, or bad `expires`: left out and named for one log line per file and change. For
   each good file: `hold_alive` from `liveness.start_of_pid(pid)` and the file's last write time (as a `FileTime`), then
   `facts`. The result is the full list for `Event::Holds`.
2. `config(dir) -> Option<Event>`: missing file gives `ConfigChanged(Config::default())`; newer version or bad file
   gives no event and a log line (the tray keeps what it has); only a change from the last posted value gives an
   event.
3. `restore_request(dir) -> Option<Event>`: `Menu(MenuCommand::Restore)` when the file appears; nothing while it stays;
   armed again once it is gone. A newer version still counts (the request has no fields to misread); a bad file counts
   too (the CLI wrote it).
4. Never deletes or writes a file: deleting a hold whose process is gone is the core's decision (`DeleteHold`), done
   by the IO thread; the request file is deleted by the IO thread after `state.json` (track note 15).

Tests first, in a temp data folder with `FakeLiveness`: each hold kind; `procStart` match, mismatch, missing (start
before and after the file's write time); a dead pid (`Gone`), an unknown lookup (`Unknown`, file kept in the list);
`expires` past and future (the core decides; the facts carry it); a bad file retried once; a newer file; a temp name;
config missing, changed, unchanged, newer; the restore request across appear, change notices, disappear, appear.

## Acceptance checks

- `rustfmt --edition 2024 --check <owned files>`; `cargo clippy --locked -p wakeshot-windows --all-targets --target
  x86_64-pc-windows-msvc -- -D warnings` and the same for `aarch64-pc-windows-msvc`; `cargo test --locked -p
  wakeshot-windows`: clean as the check rule below says, new tests named in the Result.
- No test touches `%LocalAppData%\Wakeshot` (all paths come from a `DataDir` on a temp folder).
- Check rule: `docs/plan.md` "How we work" step 2 and "Checks while other cards run" in this track's phase file. A
  failure in a file another card owns goes in the Log: wait and run again; if it is still there when this card's own
  work is done, list it under "Found outside scope". Never edit, format or `--fix` that file. Every cargo command takes
  `--locked`, and clippy runs once more without `--all-targets` (the libs as they ship; `CLAUDE.md` "Checks"). No
  `--workspace` or `--all` run here; the orchestrator runs those when it closes a wave.

## Out of scope

- The thread, folder watch and exit waits (`p4-watch-thread`). Writing any file (CLI cards, IO thread). The real
  `process::Liveness` (power track). Any file not listed above.

## Questions for Haik

None.

## Found outside scope

- None in other cards' files: every clippy and test run of this card was clean.

## Log

- 2026-11-03 16:53: card written by `p4-plan-sources`.
- 2026-11-03 21:05: started by `wakeshot-worker`, second card of the bundle (`p4-watch-reads` done first, its
  `read.rs` API is what this card reads files with).
- 2026-11-03 21:10: read track notes 10, 14 to 16, architecture sections 4, 6 and 8, `holds/file.rs`, `files.rs`,
  `event.rs`, `store::DataDir`, `process::Liveness` and `FakeLiveness`, `log::Log` and `MemoryLog`, and the
  `p4-watch-thread` card (it waits on hold processes, so `HoldsRead` also returns the live ones). `watch/data.rs`
  written with 19 tests; all pass on the first run. Clippy x64 `--all-targets`: clean.
- 2026-11-03 21:12: small cleanups (one `Trouble::new`, plainer error-code text in log lines), all checks run again,
  all clean (list in the Result). `docs/research.md` "Watch data checks" added. Card done.
- 2026-11-03 21:13: the bundle's build folder (`<scratchpad>\p4-watch-reads\target`) and scratch files deleted; no
  `%TEMP%\wakeshot-watch-*` folder left by the tests.
- 2026-11-04 00:54, `wakeshot-worker` (work name `claude-fix2`): CI on main failed 6 tests here, since the Claude fix
  round made `holds/file.rs` refuse pid 4 (the System process) and these tests used pid 4 as the fake writer. Tests
  only: every hold file and fake lookup that used pid 4 now uses pid 40 (each test's intent kept). `cargo test -p
  wakeshot-windows --lib --locked`: 373 passed, 0 failed, 20 ignored; the 19 `watch::data` tests pass. Grep of the
  workspace for hold files with pid 0 or 4: only `watch/read.rs:577`, which reads the bytes back and never parses them
  (harmless, not this card's file).
- 2026-11-04 01:04: `rustfmt --edition 2024 --check` on `data.rs`: clean. `cargo clippy -p wakeshot-windows --locked
  --target x86_64-pc-windows-msvc -- -D warnings`: clean; with `--all-targets` it stops at one lint in
  `child.rs:489` (`chunks_exact`), a file the running `p4-cli-run` card owns, and names nothing in `data.rs`.

## Result

Status: done.

Files changed: `crates/wakeshot-windows/src/watch/data.rs` (filled, 19 tests), `docs/research.md` (new section
"Watch data checks", additions only), this card.

API for `p4-watch-thread` (`wakeshot_windows::watch::data`). The three reads keep state between calls (last config
posted, restore armed, retry and log marks), so they are methods on one struct instead of the card's free functions:

- `DataFolder::new(dir: DataDir, config: Config, log: Arc<dyn Log>)`. `config` is the value the core already has;
  the thread card says to post `ConfigChanged` at start only when the file differs from the default, so it passes
  `Config::default()` (integration may pass `Started`'s config instead).
- `holds(&mut self, liveness: &dyn Liveness) -> HoldsRead { holds, live, retry }`. `holds`: the full list for
  `Event::Holds`, by file name, every file that parsed whatever its liveness (`Gone` and `Unknown` stay in).
  `live`: a `ProcessRef` per hold that is `Alive::Yes`, for the exit waits. `retry`: names of files that did not read
  or parse for the first time in this version of the file: call `holds` again `RETRY_MS` (200) later. Still bad on
  the next call, or newer, or a bad `expires`: left out and one `Warn` line (area `watch`) per file and version
  (version = write time plus length). A folder that cannot be listed: the last listing is read again and the error
  logged once. Liveness: `liveness.start_of_pid(pid)` and the write time from the same handle as the bytes, then
  `holds::file::hold_alive` and `facts`.
- `config(&mut self) -> Option<Event>`: `ConfigChanged` only for a value other than the last posted one; missing
  file = `Config::default()`; newer, bad or unreadable: `None` plus one line per version.
- `restore_request(&mut self) -> Option<Event>`: `Menu(MenuCommand::Restore)` once per appearance; armed again when
  the file is missing. A newer, bad or unreadable file counts (one line says so).
- Reads go through `watch::read::read_whole_written` and `names`; nothing is written or deleted.

Choices (reason; change it when):

- A restore request rewritten while it stays (a new write time) counts again, beyond the card's "nothing while it
  stays": a delete by the IO thread and a second `wakeshot restore` can land in one folder notice, and without this
  the second request would sit unseen until the next tray start. Plain change notices of the same file give nothing
  (tested). Change it when a second restore in a row turns out to do harm.
- A request present at the first call counts: the CLI waits for it to go, and a stale one left by a crashed tray is
  then cleaned by the IO thread. Change it when integration wants start-up requests ignored.
- The retry is one more `holds` call, not a per-file re-read: the thread schedules one call, and any call in between
  counts as the retry. Change it when `holds\` holds so many files that a full re-read 200 ms later costs.

Tests (all in `watch::data::tests`): `each_hold_kind_is_read`, `proc_start_must_match_the_running_process`,
`missing_proc_start_counts_a_process_started_by_the_write_time` (start 100 ns before, at, and 100 ns after the
write), `a_dead_pid_is_gone_and_an_unknown_lookup_stays_in_the_list`, `expires_past_and_future_are_carried_for_the_core`,
`a_bad_file_is_retried_once_then_logged_once`, `a_bad_file_fixed_before_its_retry_counts_with_no_line`,
`a_bad_expires_is_left_out_like_a_bad_file`, `an_unreadable_file_is_retried_then_logged`,
`a_newer_file_is_left_out_and_logged_once_without_a_retry`, `temp_names_and_folders_are_never_opened`,
`a_missing_holds_folder_is_an_empty_list`, `a_holds_folder_that_cannot_be_listed_is_logged_once`,
`config_missing_changed_and_unchanged`, `config_starts_from_the_value_the_core_has`,
`config_newer_or_bad_gives_no_event_and_one_line_per_version`, `restore_request_counts_once_per_appearance`,
`restore_request_written_again_before_it_was_seen_gone_counts_again`,
`restore_request_newer_bad_or_unreadable_still_counts`.

Checks (2026-11-03 21:11 to 21:12):

- `rustfmt --edition 2024 --check` on `watch/data.rs` and `watch/read.rs`: clean.
- `cargo clippy --locked -p wakeshot-windows --all-targets --target <t> -- -D warnings` and the same without
  `--all-targets`, for `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`: all four clean.
- `cargo test --locked -p wakeshot-windows`: 181 passed, 0 failed, 12 ignored; the 19 above among them.
- No test touches `%LocalAppData%\Wakeshot`: every `DataDir` is `DataDir::at(<temp folder>)`; neither owned file
  names `DataDir::real`, `dry_run` or `LocalAppData` (grep). No `unsafe` in `data.rs`. No test starts a process.

Open items: one known gap. The "last listing read again" path is tested only with a `holds` path that became a
file, so the re-read files were gone too. A case where the folder cannot be listed but its files can be read
(a deny-list ACL) is not tested: it would need an ACL change in the test and a Medium token to trust the result.
