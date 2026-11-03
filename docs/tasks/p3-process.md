# Process: liveness, exit waits, a child outside the tray's tree

Phase: 3. Status: ready. Needs Haik: no. Real laptop: no (only child processes the tests start and end through their
own handles). Agent: `wakeshot-worker`. Size: about 110K tokens.
Runs after: Haik's sign-off of the track plans; the contracts card. Parallel with: every other wave 1 card of this
track and any card of tracks 2, 4, 5, 6.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p3-process\target`.

## Goal

Fill `process`: the one liveness check for hold files, the journal owner, the guard's tray and agent sessions; waits
on process exit; and the spawn that starts the guard outside the tray's process tree. Track 4 uses it.

## Read first

- `CLAUDE.md`; `docs/phases/03-windows-power.md` (track notes 1 to 3, 9).
- `docs/architecture.md`: section 5 "Liveness" and "Guard" (Explorer as parent, rights), section 4 "Tray threads"
  (exit waits on the watch thread).
- `docs/research.md`: "Reading elevated Claude Code sessions from an unelevated process"; "Power track plan checks".
- `spikes/claude-watch/src/probe.rs`, `procs.rs`: the open with exact rights, creation time, wait, the system process
  list. Port by reading and rewriting, with tests; never copy whole.
- The stub `crates/wakeshot-windows/src/process.rs` from the contracts card: its signatures are the contract.

## Deliverables

`crates/wakeshot-windows/src/process.rs`, filled in, with its tests:

- `liveness(ProcessRef) -> Alive` and `SystemLiveness` (trait `Liveness`): open with exactly `SYNCHRONIZE |
  PROCESS_QUERY_LIMITED_INFORMATION`; creation time differs: Gone; same and not exited (wait 0 times out): Yes; same
  but exited (someone still holds a handle): Gone. Any open error (`0x80070057`, `0x80070005`, other): the system
  process list (`NtQuerySystemInformation`): listed with that pid and creation time: Yes; not listed: Gone; the call
  fails: Unknown.
- `start_of_pid(pid) -> Lookup` and `list() -> Vec<ProcessEntry>` on `Liveness` (contract C4, asked by the sources
  track; review M3). `start_of_pid`: open with the exact rights and read the creation time; a denied open uses the
  process list; not found: `NotRunning`; the list call fails: `Unknown`. `list`: pid, start and image file name of
  every process, from the same system list. `p4-watch-data`, `p4-watch-agents` and `p4-live-claude` rely on both.
- `current()`, `current_session()` (`ProcessIdToSessionId`), `start_of(&std::process::Child)` (for `wakeshot run`).
- `ProcessHandle`: open a `ProcessRef` with the exact rights (`None` when gone or another process), `wait(ms)`, the
  raw handle for `WaitForMultipleObjects`.
- `ExitWait::register(handle, callback)`: `RegisterWaitForSingleObject`, once; drop unregisters with
  `UnregisterWaitEx(INVALID_HANDLE_VALUE)`, so no callback runs after drop.
- `spawn_outside_tree(exe, args)`: parent is the process of `GetShellWindow()`, through
  `PROC_THREAD_ATTRIBUTE_PARENT_PROCESS`; returns the child's handle and `ProcessRef`. No shell window, or the open of
  Explorer is denied: start it as a normal child and say so in the result, so the caller logs it (track note 9).
- `FakeLiveness` stays as the contracts card wrote it.

## Acceptance checks

Tests in `process::tests`:

- `own_process_is_alive`, `killed_child_is_gone`, `wrong_start_time_is_gone`, `exited_child_with_open_handle_is_gone`.
- `process_list_agrees_with_open` (the list path called directly for this process and for an ended child).
- `denied_open_falls_back_to_list`: a process the test cannot open with those rights (for example `csrss.exe`, found
  in the list): the result is Yes through the list. Record what each open returned.
- `exit_wait_fires_once` and `exit_wait_dropped_never_fires`.
- `start_of_pid_self`, `start_of_pid_ended_child` (`NotRunning`), `list_has_self_with_image` (review M3).
- `real_spawn_outside_tree_has_explorer_as_parent`, `#[ignore]` (it needs Explorer; review M9): a child (this test exe
  in a sleep role) whose parent pid in the process list is Explorer's; the test ends it through its handle. Record the
  child's integrity level when the test runner is elevated (expected Medium, from Explorer's token: not verified until
  this test). This card runs it once with `cargo test --locked -p wakeshot-windows -- --ignored process::`;
  `p3-real-checks` step 2 runs it again.

Commands: `rustfmt --edition 2024 --check <owned files>`; `cargo clippy --locked -p wakeshot-windows --all-targets` with
`-D warnings` for both targets; `cargo test --locked -p wakeshot-windows process::`. Record each outcome. After the
tests no child of the test is left (check by the pids the tests logged; end only those).

New facts (the Explorer parent from a test runner, the denied open) go to `docs/research.md` with how they were
checked.

- Check rule: `docs/plan.md` "How we work" step 2 and "Checks while other cards run" in this track's phase file. A
  failure in a file another card owns goes in the Log: wait and run again; if it is still there when this card's own
  work is done, list it under "Found outside scope". Never edit, format or `--fix` that file. Every cargo command takes
  `--locked`, and clippy runs once more without `--all-targets` (the libs as they ship; `CLAUDE.md` "Checks"). No
  `--workspace` or `--all` run here; the orchestrator runs those when it closes a wave.

## Out of scope

- The guard itself (`p3-recovery`); folder watches (track 4); Task Manager grouping (`p3-haik-checks`).
- Any file but `process.rs`.

## Questions for Haik

Each with the worker's default.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-03 17:00: card written by `p3-plan-windows-power`.

## Result

What was done, files changed, checks and outcomes, open items.
