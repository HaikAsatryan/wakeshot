# Process: liveness, exit waits, a child outside the tray's tree

Phase: 3. Status: done. Needs Haik: no. Real laptop: no (only child processes the tests start and end through their
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

None.

## Found outside scope

Problems seen but not fixed.

- `docs/architecture.md` section 5 "Guard" still says "Not verified: that a Medium process gets the right to use
  Explorer as the parent". It is verified now (`docs/research.md` "Process module checks"); the Task Manager grouping
  part stays open (checklist 4).
- `docs/research.md` "Power track plan checks" says `wakeshot-windows` declares no `Wdk_*` feature. The manifest now
  declares `Wdk_System_SystemInformation`, so that line is out of date (shared file: additions only, so not edited).
- Agent sessions on this laptop run elevated with SeDebugPrivilege on (research, same section). Any other card's
  test that expects an open to be denied gets it granted here unless it drops the privilege, as
  `denied_open_falls_back_to_list` does.
- Contract proposal: `Spawned` says only `outside_tree: false`, not why (no shell window, Explorer's open denied, or
  the start under Explorer failed), so the guard's log line cannot name the failed call. Default: leave it; change it
  when the log needs the reason, by adding `pub fallback: Option<WinError>` through the orchestrator.
- Transient: the first test build failed in `store.rs` (`p3-store` mid-edit, two E0282); it built on the next try.

## Log

- 2026-11-03 17:00: card written by `p3-plan-windows-power`.
- 2026-11-03: worker started; read CLAUDE.md, plan "How we work", the card and the stub.
- 2026-11-03: read the phase notes, architecture sections 4 and 5, research on elevated sessions, the spike's
  `procs.rs` and `probe.rs`, and the `windows` 0.62.2 signatures. Writing `process.rs`.
- 2026-11-03: first draft of `process.rs` with tests written. First test build failed in `store.rs` (another card,
  `p3-store`, mid-edit: two E0282 errors); waiting and running again.
- 2026-11-03: `store.rs` fixed by its card; builds. Found: this shell runs High with SeDebugPrivilege on, so an
  empty-DACL child still opened; the denied test now impersonates this token minus its privileges
  (`CreateRestrictedToken`), and the open is denied (0x80070005). An ended child with its handle still open is not
  in the system list, so no thread-count filter is needed (removed). 19 `process::` tests pass, 2 ignored.
- 2026-11-03: ignored run passed: the outside-tree child's parent is Explorer (pid 12528); runner High (Full), child
  Medium (Limited). Added `real_spawn_outside_tree_works_with_explorers_token`: this test exe, started under
  Explorer (Medium), reruns the Explorer test and writes a report file; it passed, so a Medium process may use
  Explorer as the parent. Clippy clean on both targets, with and without `--all-targets`.
- 2026-11-03: facts written to `docs/research.md` "Process module checks"; final checks run; no test child left;
  build folder deleted. Done.

## Result

Status: done. File changed: `crates/wakeshot-windows/src/process.rs` (filled in, with 22 tests). Also
`docs/research.md` (new section "Process module checks") and this card.

What it does:

- `liveness`: open with exactly `SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION`, compare the creation time, wait 0
  ms (ended but held: Gone). Any open error, or a failed call on the handle, goes to the system list
  (`NtQuerySystemInformation`): listed with that pid and start, Yes; not listed, Gone; the list fails, Unknown.
- `SystemLiveness::start_of_pid` (same open, ended: `NotRunning`, any open error: the list) and `list` (pid, start,
  image file name). `current`, `current_session`, `start_of`.
- `ProcessHandle`: `open` gives `None` for no such pid (`0x80070057`), another start or an ended process; a denied
  open is an error. `wait`, `raw`, `process`; `Send + Sync`; closes on drop.
- `ExitWait`: `RegisterWaitForSingleObject` once, on its own duplicate of the handle; drop calls
  `UnregisterWaitEx(INVALID_HANDLE_VALUE)` and then frees the callback.
- `spawn_outside_tree`: Explorer of `GetShellWindow`, opened with only `PROCESS_CREATE_PROCESS`, as
  `PROC_THREAD_ATTRIBUTE_PARENT_PROCESS`; on any failure there, a normal child with `outside_tree: false`. The exe
  path is also the application name; `CREATE_NO_WINDOW`; no handle inheritance; arguments quoted by the C runtime
  rules; a NUL in them is refused.
- `FakeLiveness` unchanged.

Design choices (mine, with the trigger to change them):

- Every open error goes to the list, not only the two named codes: one rule, and the list needs no rights. Change it
  when the list's cost (5 to 9 ms a call) shows up for many dead hold files on one tick.
- No thread-count filter on the list: an ended process is not listed even while a handle is open (checked). Change it
  when a Windows build lists ended processes (`process_list_agrees_with_open` fails).
- `ExitWait` keeps a duplicate handle, because closing a waited handle during the wait is undefined (Microsoft). If
  `UnregisterWaitEx` fails, the callback and the handle are leaked rather than freed under a running callback.
- A started child whose start time cannot be read is ended again, since no caller could check it later.
- List errors carry the raw NTSTATUS as `code`, with `call: "NtQuerySystemInformation"`.
- The test children are this test exe in a sleep role, chosen by an extra test filter argument (not an environment
  variable), so the role also works for a child started under Explorer. Each child ends on its own after 120 s at most.

Checks (all 2026-11-03, own target folder, `--locked`):

- `rustfmt --edition 2024 --check crates\wakeshot-windows\src\process.rs`: clean.
- `cargo clippy --locked -p wakeshot-windows --all-targets --target <t> -- -D warnings` and the same without
  `--all-targets`, for `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`: all four clean.
- `cargo test --locked -p wakeshot-windows process::`: 19 passed, 3 ignored (the two `real_` tests and the child
  role). Every card-named test is in it: `own_process_is_alive`, `killed_child_is_gone`, `wrong_start_time_is_gone`,
  `exited_child_with_open_handle_is_gone`, `process_list_agrees_with_open`, `denied_open_falls_back_to_list`,
  `exit_wait_fires_once`, `exit_wait_dropped_never_fires`, `start_of_pid_self`, `start_of_pid_ended_child`,
  `list_has_self_with_image`. Extra: the drop waits for a running callback, list parent and start, handle waits,
  session, a normal child's parent, command line round trip through `CommandLineToArgvW`, list parsing bounds.
- `cargo test --locked -p wakeshot-windows -- --ignored process::`: 3 passed.
  `real_spawn_outside_tree_has_explorer_as_parent`: parent is Explorer (pid 12528); runner High, Full; child Medium
  (S-1-16-8192), Limited, as expected. `real_spawn_outside_tree_works_with_explorers_token`: the same test run by a
  Medium process under Explorer passed (report file read back).
- Denied open (recorded): `csrss.exe` (2) and an empty-DACL child opened with the runner's token (SeDebugPrivilege
  on); with the token minus privileges, each open gave `0x80070005` and `liveness` said Yes through the list.
- No child left: every pid the tests logged was checked after the runs; no `wakeshot_windows` test exe ran, no report
  file was left. Build folder deleted. No power setting was touched.

Open items: Task Manager grouping of the guard (checklist 4, `p3-haik-checks`); job inheritance from Explorer not
tested; the stale lines and the contract proposal under "Found outside scope".
