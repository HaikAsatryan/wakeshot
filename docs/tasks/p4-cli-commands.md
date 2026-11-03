# CLI: `keep-awake`, `automatic`, `status`, `log`

Phase: 4. Status: ready. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 90K tokens.
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

Each with the worker's default.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-03 16:53: card written by `p4-plan-sources`.

## Result

What was done, files changed, checks and outcomes, open items.
