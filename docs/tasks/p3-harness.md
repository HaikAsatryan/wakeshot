# Harness: a check tool for the real-laptop and manual checks

Phase: 3. Status: done. Needs Haik: no. Real laptop: yes for the `cleanup` smoke run only (global lane: it may call
`PowerSetActiveScheme` and deletes plan copies; review M7); other smoke runs touch no setting; the plan-copy runs
happen in `p3-real-checks` and `p3-haik-checks`. Agent: `wakeshot-worker`. Size: about 120K tokens.
Runs after: `p3-power-io`, `p3-recovery`, `p3-requests`, `p3-session`, `p3-system-events` (and through them store,
log, process, power-plan). Parallel with: `p3-stop-points` and any card of tracks 2, 4, 5, 6.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p3-harness\target`.

## Goal

One example exe, `power_check`, that plays the tray's power side on a plan copy, so the agent and Haik can run the
manual checks of this track before integration. The guard and restore mode it uses are the real tray exe's.

## Read first

- `CLAUDE.md`; `docs/phases/03-windows-power.md` (track notes 3, 8; "Manual checklist").
- `docs/architecture.md`: section 9 "Manual checklist" (items 1 to 6, 11, 15 and the plan copy rule).
- `docs/research.md`: "Power settings spike" (plan copy, what Settings writes), "Lid and sleep spike" (the hold runs,
  ticks, gaps), "Spike launch and stop checks".
- The spikes' hold and log code (`spikes/lid-sleep/`): ticks with wall and unbiased time, gap lines. Port by reading.
- After the cards above: the public API of `power::plan` (`TestPlan`, `CopyAsActive`), `power::executor`,
  `power::requests`, `power::system`, `session`, `recovery::TrayGuard`, `store`, `log`.

## Deliverables

`crates/wakeshot-windows/examples/power_check.rs` (built with `--features test-support`; the contracts card declares
it with `required-features`). Every subcommand logs through `LogWriter` into its data folder (`--dir`, default the real
one only where a check needs it), prints Haik's values (active plan; lid AC and DC; sleep AC and DC; energy saver DC)
at start and at exit, and prints the exact `powercfg` commands that undo it.

- `hold [--no-request] [--active-copy]`: SYSTEM and EXECUTION held, retaken on each resume; a tick line every 5 s
  (wall, unbiased, input idle) and a line per lid, power, plan, suspend and resume event, and a "gap" line when a tick
  comes late. `--active-copy`: a plan copy with lid DC 0 and sleep DC 0 made active for the run, the original plan
  made active again and the copy deleted at exit (checks 1, 2, 3).
- `fake-tray`: a plan copy, never active; the real Apply to it through `CopyAsActive` with the executor; the real
  journal (owner: this process), RunOnce value and guard (`TrayGuard`, the real tray exe in `--guard`); then waits
  (checks 4 and 5; ended in Task Manager or by PID).
- `end-session`: like `fake-tray` in its own data folder, plus a hidden top-level window: at `WM_QUERYENDSESSION` and
  `WM_ENDSESSION` it runs the restore at once and logs the time each step took (check 6).
- `session-watch`: a hidden top-level window registered for session notices; logs each one (lock, unlock, console).
- `cycles <n>`: n Apply and restore cycles on an active plan copy through the executor, logging each
  `PowerSetActiveScheme` time; the original plan back and the copy deleted at the end (check 15).
- `medium-probe`: prints its integrity level and creates `Global\Wakeshot.Probe.<pid>` (check 11, first part).
- `cleanup`: the plan active at the first start of any subcommand (stored in the data folder) active again, every
  "Wakeshot test <pid>" copy whose pid is not running deleted (never a copy another card's live run holds, review
  M7), leftover harness journal and RunOnce value removed, values printed.

## Acceptance checks

- `cargo clippy --locked -p wakeshot-windows --examples --features test-support --target <t> -- -D warnings` for both
  targets; `rustfmt --edition 2024 --check` on `examples/power_check.rs`; `cargo test --locked -p wakeshot-windows
  --features test-support --example power_check` passes.
- Smoke runs that touch no setting, each recorded with its output: `--help`; `medium-probe`; `session-watch` for 5 s;
  `hold --no-request` for 15 s (ticks every 5 s); `hold` for 10 s (a request held 10 s, then released). `cleanup`
  with nothing to clean, in a real-laptop slot only (header). Plan-copy runs are not done here.
- A dry pass of every subcommand's argument parsing and its printed undo commands (a unit test or a `--plan-only`
  flag that prints what it would do).
- Check rule: `docs/plan.md` "How we work" step 2 and "Checks while other cards run" in this track's phase file. A
  failure in a file another card owns goes in the Log: wait and run again; if it is still there when this card's own
  work is done, list it under "Found outside scope". Never edit, format or `--fix` that file. Every cargo command takes
  `--locked`, and clippy runs once more without `--all-targets` (the libs as they ship; `CLAUDE.md` "Checks"). No
  `--workspace` or `--all` run here; the orchestrator runs those when it closes a wave.

## Out of scope

- Running `fake-tray`, `end-session`, `cycles` or `hold --active-copy` (the next two cards); fixing a bug in another
  card's file (record it).
- Any file but `examples/power_check.rs`.

## Questions for Haik

Each with the worker's default. None.

## Found outside scope

Problems seen but not fixed.

1. `crates/wakeshot-windows/src/power/plan.rs` (`p3-power-plan`, done): the doc of `TestPlan::sweep` says "Only
   `real_` tests call it". `power_check cleanup` calls it too, in a real-laptop slot under the same review M7 rule (a
   copy whose pid runs is kept). The doc is stale by one caller; no code change needed.

## Log

- 2026-11-03 17:00: card written by `p3-plan-windows-power`.
- 2026-11-03 21:25, orchestrator, from `p3-recovery`'s Result: a guard inherits the harness's environment, so the
  harness must set `WAKESHOT_DATA_DIR` before `ensure`; otherwise a debug guard reads the real data folder. Read the
  Results of `p3-power-io`, `p3-recovery`, `p3-requests`, `p3-session` and `p3-system-events` for their APIs.
- 2026-11-03 22:17: started by `wakeshot-worker`. Status: running. Reading the card's sources.
- 2026-11-03 22:25: read the five Results and the APIs (`plan`, `executor`, `journal`, `recovery`, `store`, `log`,
  `process`, `session`, `system`, `requests`, `clock`) and the spike's ticks. No contract change needed so far.
  Writing `examples/power_check.rs`.
- 2026-11-03 22:47: `power_check.rs` written. rustfmt clean; clippy clean in all four forms (`--examples
  --features test-support`, `--all-targets --features test-support`, `--all-targets`, libs) on both targets; `cargo
  test --example power_check --features test-support`: 22 passed. Next: smoke runs.
- 2026-11-03 22:47 to 22:52: smoke runs (Result, "Checks"); Haik's values read by `powercfg` before and after, the
  same. A console close test (`hold --no-request` in its own conhost window, `WM_CLOSE`): the first two tries found
  no window (a console window names its client as owner, not conhost); I ended those two harness processes, mine,
  by PID. The third closed it: clean exit in 2 ms.
- 2026-11-03 22:53: review fix: after a stop `pump` no longer waits, so a console close during `cycles --pause
  60000` still finishes its clean exit inside the handler's 4.5 s. Checks rerun, all clean; `cargo test --locked -p
  wakeshot-windows --features test-support`: 275 passed, 17 ignored.
- 2026-11-03 22:55: Haik's values read again: the same. Status: done.

## Result

Status: done (2026-11-03 22:55). One file, `crates/wakeshot-windows/examples/power_check.rs` (3,826 lines with 22
unit tests), plus a section "Harness checks" added to `docs/research.md` and this card. No contract change.

### Commands (for `p3-real-checks` and `p3-haik-checks`)

Build: `cargo build --locked -p wakeshot-windows --features test-support --example power_check` and `cargo build
--locked -p wakeshot-tray`, same `--target` and profile, so the default guard path (`wakeshot-tray.exe` one folder
above `examples\`) finds the tray exe. `power_check --help` prints all of this. Every command prints the user's
values (active plan; lid AC and DC; sleep AC and DC; energy saver DC of the recorded plan; the plan list and any
"Wakeshot test" plan) at start and at exit, then "the same as at start" or one WARN line per change, and prints the
exact undo commands (`powercfg /setactive`, `powercfg /delete`, `reg delete ... RunOnce /v WakeshotRestore /f`, `del
<journal>`) before it changes anything. `--plan-only` prints the plan and the undo lines, changes nothing, writes no
log. Exit codes: 0 done, 1 failed (the reason is the last line), 2 bad command line. Read them without a pipe
(research, "Harness checks").

| Command | What it does | Ends |
|---|---|---|
| `hold [--no-request] [--active-copy] [--for <s>]` | SYSTEM and EXECUTION requests (`PowerRequests`), retaken on each `Resumed`; tick every 5 s (wall, unbiased, input idle); a line per lid, power, plan, suspend, resume; a WARN "GAP slept/paused" line named by its start. `--active-copy`: `TestPlan` copy, lid DC 0 and sleep DC 0 written, made active with `PowerSetActiveScheme`; at exit the recorded plan active again, copy deleted (checks 1, 2, 3) | `--for`, Ctrl+C, or closing its console window: all clean. Killed by PID: run `cleanup` |
| `fake-tray [--guard <exe>] [--for <s>]` | Copy never active; `PowerExecutor` over `Timed<CopyAsActive>` and `Timed<FileNet::tray>` with the real RunOnce value and `TrayGuard` (the tray exe in `--guard` mode, under Explorer); `Names::real()`; ReadPlan, then Apply lid AC 0, lid DC 0, sleep DC 0; ReadPlan every 10 s (replaces a guard that ended). Data folder: the real one. Prints the guard pid, what is in force on the copy, the RunOnce value | Checks 4 and 5: End task, or `taskkill /PID <pid> /F` (printed). Ctrl+C: Restore (Quit), copy deleted |
| `end-session [--guard <exe>] [--for <s>]` | As `fake-tray` in `<home>\end-session`, plus a hidden top-level window: `WM_QUERYENDSESSION` and `WM_ENDSESSION` run Restore (SessionEnd) at once; every plan and safety net call is a "step ... ms" log line; at `WM_ENDSESSION(TRUE)` the copy is deleted (check 6) | Sign-out; or as `fake-tray` |
| `session-watch [--for <s>]` | Hidden top-level window, `session::register`; logs every `WM_WTSSESSION_CHANGE` (raw code, name, `decode`), `WM_QUERYENDSESSION`, `WM_ENDSESSION`, with `on_console` and `locked_now` | `--for`, Ctrl+C |
| `cycles <n> [--pause <ms>]` | Copy made active; n times ReadPlan, Apply, pause, Restore (WorkEnded), pause, through `PowerExecutor` over `Timed<Win32Power>` and `FileNet::dry_run` (journal written; no RunOnce, no guard); a line per cycle with each `PowerSetActiveScheme` time, then min, median, max; recorded plan back, copy deleted (check 15) | After n, or Ctrl+C or a console close: clean |
| `medium-probe` | Integrity level and elevation type; `NamedMutex` `Global\Wakeshot.Probe.<pid>` created and taken (check 11, first part; run it at Medium) | At once; exit 1 if the mutex fails |
| `cleanup` | Puts the recorded plan back when the active plan is a "Wakeshot test <pid>" copy whose pid is gone; for each gone run record: `restore_locked` on its journal when the journal names that run (RunOnce cleared only when no live run holds one), removes the RunOnce value only while it equals that run's command, removes the record; then `TestPlan::sweep()`; forgets the recorded plan when nothing is left. A live run's plan, journal, RunOnce and copy are never touched | At once |

Options: `--dir <folder>` data folder (default `<home>\<command>`; `fake-tray`: `DataDir::real()`); `--home
<folder>` harness folder (default `%LocalAppData%\Wakeshot-harness`: `original-plan.txt`, `runs\<pid>-<start>.json`);
`--for <s>` (1 s to 7 days); `--guard <exe>`; `--pause <ms>` (0 to 60000, default 1000).

Notes for the two cards:

- `fake-tray` and `end-session` refuse to start while a journal is in their data folder or the RunOnce value is set,
  so run one at a time; `hold --active-copy` and `cycles` refuse unless the active plan is the recorded one. The
  message says to run `cleanup`.
- `fake-tray` and `end-session` set `WAKESHOT_DATA_DIR` to their data folder before the first `ensure`. Only a debug
  tray exe reads it; restore mode at the next sign-in has no such variable and reads `%LocalAppData%\Wakeshot`, so
  check 5 needs `fake-tray` in its default (real) data folder. The run prints a note when the folder is not the real
  one.
- The log of a run is `<data folder>\logs\wakeshot.log` (process "tray", area "harness"); the guard and restore mode
  log into the same file (processes "guard", "restore").
- Haik at Medium: start the harness from a shell Explorer started (research, "Spike launch and stop checks"); the
  first line prints the integrity level.

### Design choices (mine, as principal designer)

| # | Choice | Reason | Change it when |
|---|---|---|---|
| 1 | A harness folder apart from the data folders, with the recorded plan and one record per run | The data folder differs per command and `fake-tray`'s is the real one; `cleanup` must find every run's leftovers, and a record names its owner (pid and start), so a journal or RunOnce value is the harness's only when a gone run's record says so | A Wakeshot install runs beside the harness (then also check the installed tray's process) |
| 2 | `cleanup` switches the plan back only from a test copy whose pid is gone | Never undoes the user's own plan switch, never a live run's copy (review M7) | A run leaves its copy active under a reused pid often |
| 3 | `cleanup` runs the real `restore_locked` on a gone run's journal, before the sweep | The real routine puts what is there back and clears RunOnce in its own clean-up; a deleted copy's entries are dropped by it | Never likely |
| 4 | `fake-tray` and `end-session` refuse to start over a journal or a set RunOnce value | One RunOnce value exists per user; a second run would clear the first one's | Two such runs must run at once |
| 5 | `cycles` uses `FileNet::dry_run`: real journal, no RunOnce, no guard | Check 15 is about `PowerSetActiveScheme`; only the copy is written; a crash leaves the copy active, which `cleanup` and the printed undo lines put back | A cycles run must survive a crash with the user's own values restored automatically |
| 6 | `hold --active-copy` writes the copy directly, without the journal | The card asks a plain copy with lid DC 0 and sleep DC 0; `ActiveCopy` puts the plan back on every way out (drop, Ctrl+C, console close) | The checks need the guard during a battery run |
| 7 | Every plan and safety net call is logged with its time (`Timed`) in `end-session` and `fake-tray`; `cycles` logs plan calls | Check 6 asks each restore step's time; check 15 each `PowerSetActiveScheme` time | The log gets too long for 50 cycles |
| 8 | The console handler waits for the clean exit on close, sign-out and shutdown (4.5 s); `pump` stops waiting after a stop | Windows ends the process when the handler returns (research); tested by a console close | Never |
| 9 | Data folder default `<home>\<command>` for every command but `fake-tray` | Two commands never share a journal; `fake-tray` needs the real one for check 5 | Never |

### Checks (2026-11-03, `CARGO_TARGET_DIR` = this card's folder, no `RUSTFLAGS`)

- `rustfmt --edition 2024 --check crates/wakeshot-windows/examples/power_check.rs`: exit 0 (the file declares no
  child module file).
- `cargo clippy --locked -p wakeshot-windows --examples --features test-support --target <t> -- -D warnings`, the
  same with `--all-targets --features test-support`, with `--all-targets`, and with neither (the libs), for
  `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`: all eight exit 0 (22:53).
- `cargo test --locked -p wakeshot-windows --features test-support --example power_check`: 22 passed. The dry pass
  the card asks for is `plan_only_text_for_every_command` (every command's plan and undo lines), with
  `every_command_parses`, `options_land_in_their_fields`, `bad_command_lines_fail_before_anything_runs` (16 bad lines)
  and `usage_names_every_command_and_option`. Also: gap classes and lines (`gaps_are_classified_like_the_spike`,
  `ticker_names_a_gap_by_its_start`), run records and the harness folder, six `cleanup` decision cases (nothing to
  clean; a gone tray with its journal and RunOnce; the plan back only from a gone copy; a live run never touched; a
  stranger's journal, an unreadable journal, a bad record and a stranger's RunOnce left), values text and
  differences, apply writes, message and event text, and one read-only real test (token, plan name).
- `cargo test --locked -p wakeshot-windows --features test-support`: 275 passed, 17 ignored.
- Smoke runs, all with `--home` in this card's scratch folder, the x64 debug build, elevated (High):
  - `--help`: the usage, exit 0. `nap`, `hold --guard x`, `cycles 0`: exit 2 with the reason.
  - `medium-probe`: integrity High (0x3000), elevation Full; `Global\Wakeshot.Probe.29276: created and taken`; exit 0.
  - `session-watch --for 5`: registered, on the console true, locked yes (Windows had locked at screen off); no
    notice in 5 s; ended at 5.05 s, exit 0.
  - `hold --no-request --for 15`: lid open, power AC 100%, active plan Balanced; ticks 1, 2, 3 at steps 5007, 4996,
    5005 ms wall (unbiased 5018, 4996, 5004 ms), no gap; ended at 15.05 s, exit 0.
  - `hold --for 10`: "SYSTEM and EXECUTION power requests set"; `powercfg /requests` at 4 s listed
    `power_check.exe` under SYSTEM and EXECUTION with the Wakeshot reason, and 6 s after the end did not; "requests
    cleared"; 2 ticks, no gap; exit 0. The orchestrator's SYSTEM request stayed.
  - `--plan-only` for `hold --active-copy`, `fake-tray`, `end-session`, `cycles 50`, `cleanup`: plan and undo lines
    as above, exit 0; nothing changed (no real data folder made).
  - `cleanup --plan-only`, then `cleanup` (real-laptop slot): one step, the sweep; "no Wakeshot test copy to delete";
    "nothing is left: forgot the recorded plan"; exit 0.
  - Console close: `hold --no-request` under its own conhost, `WM_CLOSE` to its console window: "stopping: the
    console window closed", every line through "ended" in the log, process gone 10 ms after the post.
- Haik's values by `powercfg` at 22:47:36, 22:49:42, 22:49:43 and 22:55:12: Balanced active and the only plan, lid AC
  1 and DC 1, sleep AC 10800 and DC 14400, energy saver DC 30, no `WakeshotRestore` value, no `%LocalAppData%\Wakeshot`
  and no `%LocalAppData%\Wakeshot-harness` folder. The same each time. No process I started runs.

### Open items

- Not run here, by the card: `fake-tray`, `end-session`, `cycles`, `hold --active-copy`, and `cleanup` with
  something to clean. Their code paths are covered only by the unit tests of their pure parts and by review;
  `p3-real-checks` runs them first (after the power safety review, which may want to read this file).
- `medium-probe` ran at High here; check 11 wants it at Medium (`p3-real-checks`).
- Found outside scope 1 (a stale doc line in `plan.rs`).
