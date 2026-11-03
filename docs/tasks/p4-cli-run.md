# CLI: `wakeshot run -- <command>`

Phase: 4. Status: ready. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 110K tokens.
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

Each with the worker's default.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-03 16:53: card written by `p4-plan-sources`.

## Result

What was done, files changed, checks and outcomes, open items.
