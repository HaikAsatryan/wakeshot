# CLI: `keep-awake`, `automatic`, `status`, `log`

Phase: 4. Status: done. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 90K tokens.
Runs after: `p4-hold-files`, `p3-store`, `p3-process`, `p3-log`. Parallel with: every other card of this track (it owns no file `p4-cli-run` owns) and every card of
tracks 2, 3, 5, 6.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p4-cli-commands\target`. Uses the real laptop: the screen, once (opens
one file in the default app: a window on Haik's screen; global lane, one at a time with screen, sound and power cards
of all tracks; review M6). Everything else runs on a temp data folder. Its built-exe `--dry-run` checks use the
dry-run data folder: never beside `p6-install-e2e` (review m11).

## Goal

Four commands from `product.md` UX that talk to the tray through files: Keep awake (decisions 11 and 32), Automatic
(decision 31), status, and opening the log.

## Read first

- `docs/product.md` UX (CLI commands, Keep awake, Automatic).
- `docs/phases/04-sources.md`: track note 13.
- `docs/architecture.md` section 4 ("How they talk", "CLI"), section 6 (file table; `keep-awake.json` carries the
  tray's pid and start).
- `crates/wakeshot-core/src/files.rs` (`Config`, `StateFile`, `HoldFile`), `holds/file.rs` (`format_expires`),
  `view.rs` (`KeepAwake`); `crates/wakeshot-cli/src/command.rs`; the power track's `store`, `process`, `log`.

## Deliverables

Each `main(global, ..) -> ExitCode`, called by the dispatch; each logs one line (command and result, no paths of the
user's files):

1. `crates/wakeshot-cli/src/keep_awake.rs`: needs the tray running (`state.json` readable and its pid plus start
   alive), else "Wakeshot is not running. Start it, then try again." and exit 1. Writes `holds\keep-awake.json`
   atomically with the tray's `pid` and `procStart`, `expires` = now plus the minutes (`format_expires`, wall clock)
   or none for `on`, and `screen`; `off` deletes the file (missing is fine). Prints one line saying what was set and
   until when (local time).
2. `crates/wakeshot-cli/src/automatic.rs`: writes `config.json` (`Config::new(on)`) atomically, tray running or not.
   A `config.json` with a newer version is not overwritten (that would drop the newer fields): error, exit 1.
3. `crates/wakeshot-cli/src/status.rs`: "not running" when the tray's pid and start in `state.json` are not alive (or
   the file is missing); else Automatic on or off, what holds (the `holding` labels), what is overridden (setting
   names in words, per plan), stepped back. A newer `state.json`: "update Wakeshot", exit 1. Plain text, one fact per
   line.
4. `crates/wakeshot-cli/src/open_log.rs` and `crates/wakeshot-windows/src/shell.rs`:
   `open_with_default_app(target) -> Result<(), WinError>` (`ShellExecuteW` "open"), for a file path or a URI such as
   an `ms-settings:` page: the tray track uses it for "Open log" and for the first-run click
   (`docs/phases/05-tray-ux.md`), and its `p5-haik-check` waits for this card. Two more requests of the tray track
   (`p5-plan-tray-ux` Result; review m8): call `AllowSetForegroundWindow(ASFW_ANY)` before the open, so the opened
   window can come to the front, and run the call on its own short thread with COM initialized apartment-threaded
   (the shell needs that; the caller's thread may not have it). `wakeshot log` opens `logs\wakeshot.log`, or says
   there is no log yet.
   The real open runs once by hand in the screen slot (or as an `#[ignore]` test `real_open_log`, run once in that
   slot); unit tests check only the path and the no-log message, so no worker's `cargo test` and no CI run opens a
   window (review M6).

Tests first, against a temp data folder with a `state.json` the test writes naming its own process (alive) or a dead
pid: Keep awake each choice (file content, `expires` within 1 s of now plus the minutes), `off`, not running; Automatic
on, off, newer file kept; status for not running, running with holds and overrides, newer file; `log` with no file.

## Acceptance checks

- `rustfmt --edition 2024 --check <owned files>`; `cargo clippy --locked -p wakeshot-cli -p wakeshot-windows
  --all-targets --target x86_64-pc-windows-msvc -- -D warnings` and the same for `aarch64-pc-windows-msvc`; `cargo test
  --locked -p wakeshot-cli` and `cargo test --locked -p wakeshot-windows shell::`: clean as the check rule below says,
  new tests named in the Result.
- From a built exe with `--dry-run`: `status` says not running; `keep-awake 1h` says Wakeshot is not running and
  exits 1; `automatic off` writes the dry-run `config.json` (recorded, then the file restored to what it was).
- No test touches `%LocalAppData%\Wakeshot` outside its `dry-run` folder.
- Check rule: `docs/plan.md` "How we work" step 2 and "Checks while other cards run" in this track's phase file. A
  failure in a file another card owns goes in the Log: wait and run again; if it is still there when this card's own
  work is done, list it under "Found outside scope". Never edit, format or `--fix` that file. Every cargo command takes
  `--locked`, and clippy runs once more without `--all-targets` (the libs as they ship; `CLAUDE.md` "Checks"). No
  `--workspace` or `--all` run here; the orchestrator runs those when it closes a wave.

## Out of scope

- `run` (`p4-cli-run`), `restore` (`p4-cli-restore`), parsing (`p4-cli-args`). The tray's own Keep awake menu (core and
  tray tracks). `main.rs`, `command.rs`. Any file not listed above.

## Questions for Haik

None open. Decided by the worker, for Haik's review (each with its reason and a "change it when"):

1. **`keep-awake off` works without the tray.** It deletes `holds\keep-awake.json` (a missing file is fine) and says
   "Keep awake is off." whether or not the tray runs. Why: the file names the tray's own process, so a stale one holds
   nothing, and telling a user to start Wakeshot in order to turn something off is backwards. Change it when Off must
   also reset something only the tray knows.
2. **`status` exits 0 for "not running"**, 1 only when it cannot tell (liveness unknown, `state.json` newer or
   unreadable). Change it when a script needs running and not running apart by exit code (then 3, as `systemctl`).
3. **`wakeshot log` with no log yet** prints "No log yet. Wakeshot writes one once it runs." and exits 0, and writes no
   log line (that line would create the very file it says is missing). Change it when a script must tell "opened"
   from "no log" by exit code.
4. **The open shows no Windows error dialog** (`SEE_MASK_FLAG_NO_UI`): the CLI prints the error with the log's path,
   the tray logs it. Why: a dialog from a short thread would die with a CLI that exits, and tests can check a failed
   open without a window. Cost: a `.log` with no app gets an error, not Windows' "pick an app" box. Change it when
   users report that box missing.
5. **`open_with_default_app` waits at most 10 s** (`OPEN_WAIT_MS`), then returns `ERROR_TIMEOUT` and lets the shell
   thread finish alone, so a slow handler never holds the tray's IO thread long. Change it when a real open is seen to
   take longer.
6. **Status words.** One fact per line: "Wakeshot is running.", "Automatic is on.", one "Holding awake: <group>" per
   `holding` group (or "Nothing holds the PC awake."), one "Changed by Wakeshot: <setting>" per overridden key (or
   "Wakeshot has not changed your power settings."). The plan shows ("in power plan <guid>") only when keys of two
   plans are ours. Stepped back: "Battery low, so Wakeshot is not holding the PC awake." and "Wants the PC awake:
   <group>". Setting names use the tray notices' words (a test keeps them equal). Change it when `p5-haik-check`
   rewords the tray.

## Found outside scope

1. **`p4-cli-args` is not done** (`args.rs` still routes only Help), so the built-exe `--dry-run` checks of this card
   cannot run through the exe yet. The same checks ran through each command's `main` (Result, check 2). Who: the
   `p4-cli-args` + `p4-cli-run` bundle or `p6-int-dry-run-e2e` reruns them from the exe.
2. **The CLI crate has no `serde_json`** (its `Cargo.toml`). This card writes `config.json` and `keep-awake.json` by
   hand (numbers, bools and `format_expires` text only, read back through `files::read` in the tests).
   `p4-cli-run` writes a `reason` (the program name, free text that needs escaping): it needs `serde_json` added to
   `crates/wakeshot-cli/Cargo.toml` by the orchestrator (no `Cargo.lock` change: the workspace has it already), or
   its own escaping.
3. **`text::setting` and `text::length` are private** in `wakeshot-core/src/text.rs`, so `status.rs` keeps a copy of
   the three setting phrases (a test ties them to `text::notice`). Making `text::setting` public would remove the
   copy; for the owner of `text.rs` (`p5-haik-check`).
4. **No public path of the log file**: `log.rs` keeps `wakeshot.log` private, so `open_log.rs` has its own constant.
   `p6-int-io-thread` needs the same path for `OpenLog`; `open_log::log_path` is in the CLI crate, not reachable from
   the tray. A `DataDir::log_file()` or a public `log::LOG_FILE` (owner `p3-log` / `p3-store`) would serve both.

## Log

- 2026-11-03 16:53: card written by `p4-plan-sources`.
- 2026-11-04 00:06: started by `wakeshot-worker` (bundle with `p4-cli-restore`, this card first).
- 2026-11-04 00:21: the four commands and `shell.rs` written; `cargo test -p wakeshot-cli`: 30 pass, 1 ignored. Found:
  `p4-cli-args` is not done, so `args::parse` routes no command yet and the built-exe `--dry-run` checks cannot run
  through the exe; the CLI crate has no `serde_json`, so its three small files are written by hand; Haik's Notepad
  (the `.log` app) is open with an unsaved tab, so the real open uses a self-closing `.cmd`, not a `.log`.
- 2026-11-04 00:26: screen slot used once (`real_open_file` passed, no window or process of this card left); the
  dry-run checks through `main` passed (their log lines read 00:25) and `%LocalAppData%\Wakeshot`, made by that run,
  was removed; all checks clean; done.

## Result

Done 2026-11-04 00:26.

What was built:

- `crates/wakeshot-cli/src/keep_awake.rs`: needs the tray (`state.json` pid and start alive), else "Wakeshot is not
  running. Start it, then try again." exit 1. Writes `holds\keep-awake.json` atomically with the tray's `pid` and
  `procStart`, `expires` = now plus the minutes (`format_expires`), none for `on`, and `screen`. Prints e.g. "Keep
  awake is on for 1 hour, until 01:15." (local time; "tomorrow" or the date when later; lengths in the menu's words).
  `off` deletes the file, tray or not.
- `crates/wakeshot-cli/src/automatic.rs`: writes `config.json` (`Config::new(on)`) atomically, tray or not; a newer
  `config.json` is kept, exit 1; a bad or older one is replaced.
- `crates/wakeshot-cli/src/status.rs`: the status report (Questions, item 6); not running when the file is missing or
  its process is gone (pid and start); newer file: "The running Wakeshot is newer than this command. Update
  Wakeshot." exit 1; liveness unknown: "Cannot tell whether Wakeshot is running. Try again." exit 1. Also the parts
  every CLI command shares: `data_dir` (`--dry-run` gives the dry-run folder), `open` (folder plus log, `Proc::Cli`,
  area `cli`), `tray` (the `Tray` state from `state.json`), `Answer` (text plus exit code, stdout for 0, stderr
  otherwise), and the test `fixture` (temp data folder, own and dead process, `state_json`).
- `crates/wakeshot-cli/src/open_log.rs`: opens `logs\wakeshot.log` through the shell, or "No log yet." The opener is a
  parameter, so no unit test opens a window.
- `crates/wakeshot-windows/src/shell.rs`: `pub fn open_with_default_app(target: &OsStr) -> Result<(), WinError>`, for
  a path or a URI (`ms-settings:taskbar`). `ShellExecuteExW` verb "open" with `SEE_MASK_NOASYNC | SEE_MASK_FLAG_NO_UI`
  (Ex instead of plain `ShellExecuteW`: it gives the real Win32 error and the NOASYNC flag a thread that ends at once
  needs), on its own thread `wakeshot-shell` with `CoInitializeEx(APARTMENTTHREADED | DISABLE_OLE1DDE)`, after
  `AllowSetForegroundWindow(ASFW_ANY)` (its failure ignored: the app then opens behind). Waits at most
  `OPEN_WAIT_MS` = 10 s. For `p6-int-io-thread`: it is a plain `fn(&OsStr) -> Result<(), WinError>`, so it fits an
  opener `fn` or trait; pass `path.as_os_str()`.
- Each command logs one line (`cli: status: not running`, `cli: keep-awake: set until <expires>, screen off, tray pid
  N`, `cli: automatic off: written`, `cli: log: opened`), no paths.

New tests (all in the owned files):

- `status::tests`: `not_running_without_a_state_file`, `not_running_when_the_tray_in_the_file_is_gone`,
  `not_running_when_the_pid_runs_with_another_start`, `running_with_holds_and_overrides`,
  `running_idle_with_automatic_off`, `stepped_back_names_what_wants_the_pc_awake`,
  `overrides_in_two_plans_name_the_plan`, `newer_state_file_asks_for_an_update`, `bad_state_file_is_unreadable`,
  `unknown_liveness_decides_nothing`, `setting_words_match_the_notices`, `fixture_state_reads_as_the_tray_writes_it`;
  ignored: `dry_run_folder_checks`.
- `keep_awake::tests`: `each_length_writes_the_trays_hold_with_its_end`,
  `end_is_now_plus_the_minutes_on_the_wall_clock`, `until_off_has_no_end_and_keeps_the_screen_flag`,
  `screen_on_with_a_length`, `a_new_choice_replaces_the_old_one`, `off_deletes_the_file_and_a_missing_one_is_fine`,
  `off_works_without_the_tray`, `needs_the_tray_running`, `newer_or_unknown_state_writes_nothing`,
  `end_reads_as_local_time`, `lengths_are_the_menus_words`.
- `automatic::tests`: `on_and_off_write_the_config`, `a_newer_config_is_kept`, `a_bad_or_older_config_is_replaced`.
- `open_log::tests`: `no_log_yet_opens_nothing_and_writes_nothing`, `opens_the_log_file_of_the_data_folder`,
  `a_failed_open_names_the_file`; ignored: `real_open_log` (needs a person at the screen).
- `shell::tests`: `a_nul_or_an_empty_target_opens_nothing`, `a_missing_file_is_an_error_without_a_dialog`; ignored:
  `real_open_file`.

Checks:

1. `rustfmt --edition 2024 --check` on the five owned files: clean. `cargo clippy --locked -p wakeshot-cli -p
   wakeshot-windows` with and without `--all-targets`, for `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`,
   `-D warnings`: clean (4 runs). `cargo test --locked -p wakeshot-cli`: 30 passed, 2 ignored. `cargo test --locked -p
   wakeshot-windows shell::`: 2 passed, 1 ignored.
2. `--dry-run` checks, through each command's `main` (`status::tests::dry_run_folder_checks`, not the exe: see Found
   outside scope 1): `status` "Wakeshot is not running." exit 0; `keep-awake 1h` "Wakeshot is not running. Start it,
   then try again." exit 1, no hold file; `automatic off` wrote `{"version":1,"automatic":false}`, then the file was
   put back as it was (missing). Recorded in `docs/research.md`, "CLI command checks".
3. No test touches `%LocalAppData%\Wakeshot` outside its dry-run folder: a grep shows `DataDir::real` and
   `DataDir::dry_run` only in `status::data_dir`; every test but `dry_run_folder_checks` passes a temp `DataDir`, and
   that one asserts its folder ends in `Wakeshot\dry-run`.
4. Screen slot, once: `shell::tests::real_open_file` opened a temp `.cmd` through `open_with_default_app` (returned
   after 119 ms; the script ran within 622 ms); its window closed by itself; no process of this card left. Not a
   `.log`: Haik's Notepad was open with an unsaved tab (`docs/research.md`, "CLI command checks").

Open items: the built-exe `--dry-run` checks once `p4-cli-args` lands; `real_open_log` with a person at the screen
(`p5-haik-check` already opens a sample log through `open_with_default_app`). The build folder is shared with
`p4-cli-restore` and is deleted when that card ends.
