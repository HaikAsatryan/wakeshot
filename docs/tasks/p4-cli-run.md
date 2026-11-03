# CLI: `wakeshot run -- <command>`

Phase: 4. Status: done. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 110K tokens.
Runs after: `p4-cli-args`, `p4-hold-files`, `p3-store`, `p3-process`, `p3-log`. Parallel with: every other card of this track and every card of tracks 2, 3, 5, 6.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p4-cli-run\target`. Uses the real laptop: no (child processes the tests
start, a temp data folder; no power settings). Its built-exe `--dry-run` checks use the dry-run data folder: never
beside `p6-install-e2e` (review m11).

## Goal

`wakeshot run -- <command>` (decision 10): start the command in the same console, hold the PC awake through
`holds\run-<pid>.json` while it runs, leave Ctrl+C to the child, exit with the child's code.

## Read first

- `docs/product.md` UX (CLI commands; `run` always runs and warns).
- `docs/phases/04-sources.md`: track notes 11 to 13.
- `docs/architecture.md` section 4 "CLI"; section 6 (hold format: `pid` is the child's; `reason` is only the program
  name).
- `docs/research.md` "Sources track plan checks" (`Command` ignores `PATHEXT`; Ctrl+C ignore is inherited).
- `crates/wakeshot-cli/src/args.rs` (`run_tail`), `crates/wakeshot-core/src/holds/file.rs`, `files.rs`; the power
  track's `store`, `process`, `log` modules.

## Deliverables

1. `crates/wakeshot-windows/src/child.rs`:
   - `command_line() -> OsString` (`GetCommandLineW`; its feature `Win32_System_Environment` is declared by
     `p1-close-contracts`).
   - `resolve(first_token) -> Resolved { Exe(path), Batch(path), NotRunnable(ext), NotFound }`: a token with a folder
     part is used as given (with `PATHEXT` tried when it has no extension); a bare name is searched in `PATH` only,
     each folder with each `PATHEXT` extension in order (track note 11).
   - `start(resolved, tail) -> std::process::Child`: std `Command` on the resolved full path (so std's own `PATH`
     search, which ignores `PATHEXT`, never runs), with the rest of the tail after the program token passed unchanged
     through `CommandExt::raw_arg`, for `.exe` and `.com`; `%ComSpec%` with `raw_arg("/d /s /c \"<tail>\"")` for
     `.bat` and `.cmd`. Inherited stdio and console; no new process group (that would turn off Ctrl+C in the child). A
     `std::process::Child` lets the power track's `process::start_of(&Child)` give `procStart`. If `raw_arg` cannot
     pass a case exactly (a test shows it), fall back to `CreateProcessW` and say why in the Result.
   - `ignore_ctrl_c()`: a handler that returns TRUE for Ctrl+C and Ctrl+Break and FALSE for the rest, never the NULL
     form (it is inherited, track note 12). Installed after the child starts.
   - The exit code: `ExitStatus::code()` (the full 32-bit value on Windows).
2. `crates/wakeshot-cli/src/run.rs`, `main(global) -> ExitCode` (the dispatch already calls it):
   - Warn on stderr, then run anyway, when the tray is not running (`state.json` missing, unreadable, or its pid and
     start not alive) or Automatic is off (`config.json`).
   - Start the child; write `holds\run-<child pid>.json` atomically (`store::write_atomic`), with `pid`, `procStart`
     (`process::start_of(&child)`, read while this process holds the child's handle, so the pid cannot be reused),
     `reason` = the program's file name without extension. A failed write
     warns and the command still runs.
   - Wait; delete the hold file; exit with the child's code (`std::process::exit(code as i32)`, so codes above 255
     pass through). The CLI log gets one line at start and one at the end (program name and exit code only).
   - `--dry-run` uses the dry-run data folder.

Tests first: `resolve` against a temp `PATH` with `.exe`, `.cmd`, `.ps1` and no-extension decoys, current folder not
searched; end to end on a temp data folder (a test-only data folder from `DataDir`, never the real one): `cmd /c exit
7` exits 7; exit code `0xC000013A` passes through; the hold file exists while `ping -n 3 127.0.0.1` runs and is gone
after; a `.cmd` file with spaced and quoted arguments sees them as typed; `reason` never holds the arguments. Ctrl+C:
a helper process that attaches to a console with no window (`CREATE_NO_WINDOW`), sends `CTRL_C_EVENT`, and checks the
child ended and `wakeshot` exited with the child's code; if that method does not work, write the manual steps for
`p4-live-claude` instead and say so (not verified that it works).

## Acceptance checks

- `rustfmt --edition 2024 --check <owned files>`; `cargo clippy --locked -p wakeshot-cli -p wakeshot-windows
  --all-targets --target x86_64-pc-windows-msvc -- -D warnings` and the same for `aarch64-pc-windows-msvc`; `cargo test
  --locked -p wakeshot-cli` and `cargo test --locked -p wakeshot-windows child::`: clean as the check rule below says,
  new tests named in the Result.
- No test writes to `%LocalAppData%\Wakeshot` (outside its `dry-run` folder) and every child a test starts has ended
  (by handle).
- `wakeshot --dry-run run -- cmd /c echo hi` from a built exe prints `hi` and exits 0 (recorded).
- Check rule: `docs/plan.md` "How we work" step 2 and "Checks while other cards run" in this track's phase file. A
  failure in a file another card owns goes in the Log: wait and run again; if it is still there when this card's own
  work is done, list it under "Found outside scope". Never edit, format or `--fix` that file. Every cargo command takes
  `--locked`, and clippy runs once more without `--all-targets` (the libs as they ship; `CLAUDE.md` "Checks"). No
  `--workspace` or `--all` run here; the orchestrator runs those when it closes a wave.

## Out of scope

- The other commands (`p4-cli-commands`, `p4-cli-restore`). Parsing (`p4-cli-args`). The tray side of holds (watch
  cards, core track). `main.rs`, `command.rs`. Any file not listed above.

## Questions for Haik

None open. Decided by the worker, for Haik's review (each with its reason and a "change it when"):

1. **Wakeshot's own exit codes: 127 when the program is not found, 126 when it is found but not started (a `.ps1`,
   another kind of file, or a failed start), 125 when the child was lost** (a failed wait), as GNU `env` and
   `timeout` use them; usage errors stay 2. Every other code is the child's, whole (`0xC000013A` too). Change it
   when a script runner on Windows is seen to expect `9009`, cmd.exe's "not recognized" code.
2. **A batch file runs as `%ComSpec% /d /s /c ""<full path>" <rest>"`**: the resolved full path, not the name as
   typed, so cmd.exe runs the file found in `PATH` and never searches the current folder first (track note 11). The
   card's `raw_arg("/d /s /c \"<tail>\"")` would have let cmd.exe search again. `%ComSpec%` as the card says (Node and
   Python use it too), when it is a full path; else the system folder's `cmd.exe`. Change it when a user with
   another shell in `%ComSpec%` (TCC, for example) reports that batch files break: then use the system `cmd.exe`, as
   Rust's std does.
3. **`child::start(resolved, rest)` takes the text after the program, not the whole tail**: `args::split_program`
   (in `args.rs`, with the same splitter as `run_tail`) splits the program from the rest, so one splitter serves
   both. Change it when another caller needs the tail whole.
4. **A bare name never matches an extensionless file** (npm's shell script beside `npm.cmd`), and relative `PATH`
   entries (`.`) are skipped, so a bare name never runs from the current folder. A name that has its own extension
   is tried as written first (`deploy.ps1` gives the PowerShell hint; `python3.12` still finds `python3.12.exe`). A
   path to an extensionless file says it is not a program Wakeshot starts, rather than "not found". Change it when
   PowerShell is seen to run a program Wakeshot does not find.
5. **Warnings**: "Wakeshot is not running, so nothing keeps the PC awake for this command until it starts." when
   `state.json` is missing, unreadable or its process gone; "Automatic is off, so this command does not keep the PC
   awake. Turn it on with 'wakeshot automatic on'." only when the tray runs and `config.json` reads off. A newer
   `state.json`, or liveness that cannot be told, gives no warning (nothing sure to say). Both on stderr, before the
   child starts, prefixed `wakeshot:`. Change it when `p5-haik-check` rewords the CLI.
6. **No data folder** (the known-folder call fails): one warning, and the command runs with no hold and no log.
   Change it never, unless `run` is ever allowed to refuse.
7. **The log lines** are `run: <name> started, pid <child pid>` and `run: <name> exited with <code>` (codes above 255
   also in hex), plus a Warn line for each problem (not found, not started, hold file not written or not deleted,
   Ctrl+C handler not set). The pid is not secret and ties the two lines together.

## Found outside scope

1. **`keep_awake.rs` writes `holds\keep-awake.json` by hand** (`hold_json`, numbers and fixed text only, so it is
   correct today). `holds::file::render` now exists in the core for Wakeshot's own hold file writers; Keep awake
   could use it, so the format has one writer. Who: `p6-int-dry-run-e2e` (the file's fixer) or the orchestrator.
   Not urgent.
2. **`p4-cli-commands`, Found outside scope 2** (the CLI needs `serde_json` or its own escaping for `reason`) is
   settled for `run` by the core's `render`; no `Cargo` change was needed.
3. **`docs/architecture.md` section 4 "CLI"** does not name the wrapper's own exit codes (Questions, item 1) or the
   batch form (item 2). Not wrong, only silent; for the orchestrator to add if Haik keeps the defaults.

## Log

- 2026-11-03 16:53: card written by `p4-plan-sources`.
- 2026-11-04 00:56: started by `wakeshot-worker`, second card of the bundle after `p4-cli-args` (done). Build folder
  shared with that card (`<scratchpad>\p4-cli-args\target`).
- 2026-11-04 01:00: `holds::file::render` added to the core (the CLI has no JSON writer), with its own test module
  `render_tests`; the parser and its tests untouched. 24 `holds::file` tests pass.
- 2026-11-04 01:03: `child.rs` written (`command_line`, `resolve`, `command`, `start`, `ignore_ctrl_c`); 14 `child::`
  tests pass, among them the raw text through `cmd /u /c echo` and a `.cmd` file's `%*`.
- 2026-11-04 01:06: `run.rs` written; the two dead-code attributes in `args.rs` removed. 15 `run::` tests pass.
- 2026-11-04 01:08: `tests/run.rs`: exit codes, `--dry-run` and Ctrl+C through the built exe. The Ctrl+C method of
  the card works: the helper's windowless console held only the helper, wakeshot and the child.
- 2026-11-04 01:10: built-exe check `wakeshot --dry-run run -- cmd /c echo hi`: `hi`, exit 0; dry-run files removed.
  App execution alias `winget` found and started.
- 2026-11-04 01:11: rustfmt clean; clippy clean on 4 runs over the three packages; tests pass (Result).
- 2026-11-04 01:12: done.

## Result

Done 2026-11-04 01:12.

What was built:

- `crates/wakeshot-windows/src/child.rs`:
  - `command_line() -> OsString` (`GetCommandLineW`).
  - `resolve(token) -> Resolved { Exe(path), Batch(path), NotRunnable(ext), NotFound }`, and `resolve_in(token, PATH,
    PATHEXT)` for tests: a token with `\`, `/` or `:` is used as given (made absolute, `PATHEXT` tried when it has no
    extension); a bare name is searched in the absolute `PATH` folders only, each folder with the name as written
    (when it has an extension) and then each `PATHEXT` extension in order (Questions, item 4). `PATHEXT` unset or
    empty gives cmd.exe's `.COM;.EXE;.BAT;.CMD`. Files are checked by attributes without following links, so app
    execution aliases are found (`winget`, checked).
  - `command(resolved, rest) -> Option<Command>` and `start(resolved, rest) -> io::Result<Child>`: `.exe` and `.com`
    by full path plus `raw_arg(rest)`; `.bat` and `.cmd` through `%ComSpec% /d /s /c ""<full path>" <rest>"`
    (Questions, item 2). Inherited stdio and console, no new process group. `raw_arg` passed every case exactly
    (tests), so no `CreateProcessW` fallback.
  - `ignore_ctrl_c()`: a handler that returns TRUE for Ctrl+C and Ctrl+Break, FALSE for the rest; never the NULL form.
- `crates/wakeshot-cli/src/run.rs`: `main(global)` reads the raw line (`args::run_tail`, `args::split_program`),
  opens the data folder and log (`--dry-run` gives the dry-run folder), and calls `run(dir, parts, warn, program,
  rest) -> i32`: resolve (not found 127, not runnable 126 with the `pwsh -File` hint for `.ps1`), warn, start (failed:
  126), the Ctrl+C handler after the start, `holds\run-<child pid>.json` through `store::write_atomic` with `pid`,
  `procStart` (`process::start_of(&child)`) and `reason` = the program's file stem, wait, delete the file, two log
  lines, and `std::process::exit(code)` with the child's whole code.
- `crates/wakeshot-core/src/holds/file.rs`: `pub fn render(file: &HoldFile) -> String`, the JSON text `parse` reads
  back (serde, so a free-text `reason` is escaped); tests in a new `render_tests` module; the parser and its tests
  unchanged.
- `crates/wakeshot-cli/src/args.rs`: the two temporary dead-code attributes removed (now called).
- `crates/wakeshot-cli/tests/run.rs`: the built exe, with `WAKESHOT_DATA_DIR` on a temp folder.

New tests:

- `child::tests`: `a_bare_name_takes_the_first_folder_with_a_pathext_match` (extensionless and `.ps1` decoys first),
  `pathext_order_decides_inside_one_folder`, `a_name_with_its_extension_is_taken_as_written`,
  `the_current_folder_is_not_searched`, `a_token_with_a_folder_is_used_as_given`,
  `folders_are_skipped_and_quoted_path_entries_read`, `an_empty_or_odd_pathext_falls_back_to_the_default`,
  `the_command_passes_the_rest_unchanged`, `the_command_prompt_is_a_full_path_to_cmd`,
  `an_exe_child_gets_the_text_exactly`, `a_batch_child_sees_its_arguments_as_typed`,
  `start_starts_and_refuses_what_it_cannot_run`, `the_handler_takes_only_ctrl_c_and_ctrl_break`,
  `the_command_line_names_this_test_program`.
- `run::tests`: `the_childs_exit_code_comes_back` (`cmd /c exit 7` gives 7; 300; the two log lines),
  `a_crash_code_passes_through_whole` (0xC000013A), `the_hold_file_exists_while_the_command_runs_and_is_gone_after`
  (`ping -n 3 127.0.0.1` through `cmd`: name, kind, pid, `procStart` alive, then gone),
  `the_reason_never_holds_the_arguments`, `a_cmd_file_sees_its_arguments_as_typed`,
  `a_program_not_found_starts_nothing`, `a_script_wakeshot_does_not_start_is_named`,
  `a_spaced_script_path_is_quoted_in_the_hint`, `warns_when_the_tray_is_not_running_and_runs_anyway`,
  `warns_when_automatic_is_off`, `a_newer_tray_gets_no_warning`,
  `a_failed_hold_write_warns_and_the_command_still_runs`, `no_data_folder_still_runs_the_command`,
  `a_failed_ctrl_c_handler_is_logged_and_the_command_runs`, `the_name_is_the_file_name_without_extension`.
- `holds::file::render_tests`: `renders_the_documented_form`, `a_reason_with_quotes_and_line_breaks_is_escaped`, and
  the property `rendered_files_parse_back_as_written`.
- `tests/run.rs`: `the_exe_exits_with_the_childs_whole_code`, `dry_run_echo_hi_prints_hi_and_uses_the_dry_run_folder`,
  `quotes_and_spaces_reach_the_child_as_typed`, `a_program_not_found_exits_127`,
  `ctrl_c_ends_the_child_and_wakeshot_exits_with_its_code` (with its ignored helpers `ctrl_c_helper` and `sleeper`).

Checks:

1. `rustfmt --edition 2024 --check` on the seven owned files (`args.rs`, `command.rs`, `run.rs`, `tests/exe.rs`,
   `tests/run.rs`, `child.rs`, `holds/file.rs`): clean. `cargo clippy --locked -p wakeshot-cli -p wakeshot-windows -p
   wakeshot-core` with and without `--all-targets`, `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`, `-D
   warnings`: clean (4 runs). `cargo test --locked -p wakeshot-cli`: 75 passed, 3 ignored (the command cards' by-hand
   tests); `tests/exe.rs` 3 passed; `tests/run.rs` 5 passed, 2 ignored (the helpers). `cargo test --locked -p
   wakeshot-windows child::`: 14 passed. `cargo test --locked -p wakeshot-core holds::file`: 24 passed.
2. No test writes to `%LocalAppData%\Wakeshot`: the unit tests use temp `DataDir`s, the exe tests set
   `WAKESHOT_DATA_DIR` to a temp folder; the folder did not exist before or after the runs. Every child is waited
   for by its handle (`wait`, `output`, the `Running` guard); the Ctrl+C child is checked gone by pid and start. After
   the runs no process from this card's build folder was left, nor any `wakeshot*` temp folder.
3. Built exe: `wakeshot --dry-run run -- cmd /c echo hi` printed `hi`, exit 0, with the not-running warning on stderr;
   the dry-run files it made were removed (`docs/research.md`, "CLI run checks").
4. Ctrl+C: the card's method works (same research section); no manual steps were needed.

Open items, for `p4-live-claude`: Ctrl+C by a real key press in Windows Terminal and in a conhost window, Ctrl+Break,
and closing the window while `wakeshot run` waits (the hold file then stays until the tray sees the child gone); a
`wakeshot run` against a running tray (needs `p4-watch-thread` and `p6-int-io-thread`). For `p6-int-dry-run-e2e`:
`tests/run.rs` shows how to run the built exe on a temp data folder (`WAKESHOT_DATA_DIR`, debug builds only). The
bundle's build folder is deleted.
