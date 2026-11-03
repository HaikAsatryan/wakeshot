# Claude parsing review fixes: verify B1 and M1

Reviewer: `wakeshot-reviewer`, 2026-11-04 00:30 to 00:50. Brief: `docs/phases/02-06-lighter.md`, "Reviews" (any file
content that makes Wakeshot hold forever or never; any path that stores or logs prompt text or opens a `.key` file),
narrowed to B1, M1 of `docs/reviews/p4-claude-parsing.md` and what their fixes touched.

Read: the review (B1, M1); `docs/tasks/p4-claude-transcript.md`, "Claude parsing review fixes (2026-11-04)"; rules 3
and 5 of `docs/research/claude-code-signals.md`; `docs/product.md:30-46`; `crates/wakeshot-core/src/sources/claude/`
`transcript.rs` and `mod.rs`; `policy/sessions.rs` (how timers and `open_tasks` hold); `tests/claude_transcript.rs`;
the four new fixtures; live fixture `02-monitor` (transcript and status). Claude Code 2.1.317
(`C:\Users\haika\.local\bin\claude.exe`, read only) was searched for the stop, Monitor and queue paths; byte offsets
below point into that file, and minified names are from that build only.

Run once: `cargo test -p wakeshot-core --locked --test claude_transcript` with the scratch target folder: 24 passed
(00:45). The folder was deleted after.

Counts: B1 fixed; M1 partly fixed. New findings: 0 blockers, 0 majors, 3 minors. One major is left from M1 part b,
with a cheaper fix than the worker saw.

## Verdicts

### B1: Fixed

- `TaskStop`, `KillShell` and `KillBash` all map to one call kind (`transcript.rs:1145`) and are kept by `tool_use`
  id (`:320-329`). A result that is not an error closes the result's `task_id`, else the input's `task_id`, else
  `shell_id` (`:403-412`). This matches 2.1.317: `TaskStop.call` returns `task_id: w.taskId`, the id it resolved,
  also when the input gave a name (tool at offset 302,209,074; `ILe` at 302,203,845).
- A stop that names an unknown or already closed id changes nothing: `close` only removes that exact key
  (`:555-559`). Tested (`a0ther000`, `:2153-2160`).
- Across a resume: a task opened before the status file's `procStart` is never counted (`open_tasks`, `:239-246`), so
  a stop of it in the new process (which errors in Claude Code) changes nothing, and a stop line from the old process
  closes it in the replay as before.
- No path closes the wrong task: only the exact id the result (or the call's own input) names is removed, and only for
  a `TaskStop` call, since `task_id` is read into `stopped_task` but used only in the `Call::Stop` arm.
- Not verified live (no live `TaskStop` line); the fixture is built from the 2.1.317 code.
- An error result closes nothing (`:380-382`), on purpose and tested (`:2144-2152`). See N1.

### M1: Partly fixed

Part a (Monitor timeout): fixed.

- A Monitor result with a timeout becomes a task with an end at the result's time plus `timeoutMs` (`:430-443`,
  `:540-553`), reported by `task_timers` (`:218-234`) and merged into `timers` only when `entrypoint` is not `cli`
  (`mod.rs:212-227`). `open_tasks` counts only tasks with no end (`:239-246`).
- It cannot hold past its end: the core holds a timer from due less 1 hour to due plus 2 minutes
  (`policy/sessions.rs:151-161`), and a timer window only adds a hold; it never cancels one (`steady`, `:130-142`).
- It holds over the whole run: 2.1.317 arms the timeout before it returns the result (`We`, offset 301,337,453), so the
  result line's time is at or after the real start, and the due is at or after the real timeout. The 1 hour lead
  covers any timeout up to 1 hour; a compile-time check ties the two (`transcript.rs:1488`).
- A `<status>` notification or a stop removes it (`close`); it is forgotten an hour after its end (`:282-285`).
- A persistent Monitor stays an open task, ended by its own exit notice, a `TaskStop` (B1) or the process end. A
  result of another shape (no `timeoutMs`, 0 without `persistent`, a non-number) ends after 1 hour and is named; a
  timeout over 1 hour gets a due every hour, at most 24 (`:745-755`). Both are bounded.
- A terminal session needs no timer: its status says `shell` while a Monitor runs (live `02-monitor` status lines 8,
  10, 12).
- Fact found while checking: with 2.1.317's default flags every Monitor is `persistent: false`. `hgi` forces
  `persistent: false` and a timeout of at most 30 minutes (10 in some modes) while `bae()` is on, and `bae()` is
  `un("tengu_breezy_crescent", true)` (offsets 291,716,251 to 291,716,484). So the timer path is the common case and a
  persistent Monitor shows only when that remote flag is off.

Part b (stop from the UI): not fixed; documented in `product.md:41-46` as decided by the worker. A shell Monitor
stopped from the UI does get the `<status>killed</status>` notice (2.1.317 `tUr`, offset 301,313,169, is gated by
`Td`, most likely the `type === "local_bash"` check, since the chunk that defines that check exports a `Td`; a shell
Monitor is `local_bash` with `kind: "monitor"`), and
`notice_ids` closes it in every known form. The open question is only when that notice lands. The worker's basis,
"the task's output file is the only other trace", is likely wrong: see the next item.

#### Left from M1 part b (major, as in the review): the UI stop's notice is probably on disk at once, in a line Wakeshot skips

- Where: `transcript.rs:287-293` (a `type: "queue-operation"` line falls to `_ => {}`); `docs/product.md:44-46` ("the
  only other trace").
- Evidence (2.1.317 code): `tUr` queues the `killed` notice with `passive: true`. The pending-notification enqueue
  (`vr`, right after offset 293,955,262) pushes it and writes `s("enqueue", content)` unless the item is `unlogged`;
  passive only changes delivery. The writer (`I3o`, offset 293,948,812) appends
  `{type: "queue-operation", operation: "enqueue", timestamp, sessionId, content}` to the transcript. The live fixtures
  show these lines carry the whole notification before the user line: `02-monitor.transcript.jsonl` line 6 has the
  `<status>failed</status>` notice 1.6 s before line 9 delivers it; the SDK fixtures 08, 12 and 13 have such lines
  too.
- What breaks today: a background shell or persistent Monitor stopped from the VS Code panel holds until the passive
  notice is delivered as a user line (maybe only at the next prompt) or the chat closes, though its end is likely
  already in the transcript.
- How likely: not verified live. Unverified parts: that the panel stop reaches `tUr` with the remote flag
  `tengu_rosy_donut` on, and that no hook (`$r`) holds the enqueue back.
- Fix: read `queue-operation` lines with `operation: "enqueue"` and run `notice_ids` on `content` in place, as for a
  user line. A notice with a `<status>` is queued only after its task ended, so this cannot close a running task, and
  a typed prompt in `content` is looked at in place and never kept. Add one fixture and the panel stop to
  `p4-live-claude` (expect the enqueue line within a second of the stop). If it holds, change `product.md:43-46`.

## New findings

### N1 (minor). A `TaskStop` that fails closes nothing, even when Claude Code says the task is not running

- Where: `transcript.rs:380-382` (every `is_error` result returns before the `Call::Stop` arm); test `:2144-2152`.
- What breaks: 2.1.317's `TaskStop` fails with "Task <id> is not running (status: ...)", or a not-found error, when the
  task already ended. If that end wrote no line Wakeshot reads (a UI stop whose notice has not landed, or with
  `tengu_rosy_donut` off), the failed stop is the only trace, and the task holds until the process ends. Closing on
  every error would be wrong in one case: the machine-field error (`Df`, "This call named a machine but was not
  forwarded") leaves the task running.
- How likely: low. It needs a silent end and then a model stop of the same task; the next turn usually delivers the
  passive notice anyway. Code-read only.
- Fix: none needed if the queue-line fix above lands. Otherwise accept it and say in rule 3 that a failed stop closes
  nothing.

### N2 (minor, unverified). A Monitor's end is on the wall clock; Claude Code's timeout may not count a PC sleep

- Where: `transcript.rs:540-553` (end = result time plus `timeoutMs`, wall ms); the timeout itself is a `setTimeout`
  in Claude Code (`We`, offset 301,337,453).
- What breaks: if the user sleeps the PC (Start > Sleep, lid set to Sleep) during a Monitor and wakes it after the
  wall-clock end plus 2 minutes, and Bun's timer did not count the sleep, the Monitor still runs for the rest of its
  timeout (at most 30 minutes by default) with no hold. Before M1 it held while the process lived.
- How likely: low. Unverified whether Bun's timers on Windows count sleep time. Harm is bounded: the session is
  `busy` at each event, and the PC sleeps only if the idle timeout is shorter than the rest of the run.
- Fix: none now. Add to `p4-live-claude`: sleep the PC for 5 minutes during a 10 minute Monitor and see when its
  timeout event comes.

### N3 (minor). Docs do not fully match the new Monitor and stop rules

- `docs/architecture.md:899-900`: `timers` is "next due times" and `open_tasks` "background tasks seen open in the
  transcript". A Monitor with a timeout is now an open task reported as a timer, not in `open_tasks`.
- `docs/research/claude-code-signals.md:232` (rule 3): says only "`persistent: false` is a timer at its time plus
  `timeoutMs`". The code also makes a timer when `persistent` is missing, ends a Monitor of another shape after 1 hour
  (worker decision 4), gives a timeout over 1 hour a due every hour up to 24, and closes nothing on a failed stop.
- `docs/product.md:44-46`: "the only other trace" (see the item under M1).
- Fix: one line each.

## Checked, no finding

- Nothing in the fixes can make a working session stop holding: `Working` holds without the transcript; timer
  windows only add holds; the new `close` paths remove only the exact id a stop names; a Monitor's due is at or after
  its real end.
- Unknown shapes are bounded: a `TaskStop` result with no id anywhere is named and closes nothing (the task then holds
  with the notice while the process lives, as for an unknown `status`; 2.1.317 refuses a stop without an id anyway);
  a Monitor of an unknown shape ends after 1 hour.
- With M1, a Monitor problem in a session that holds only by that Monitor's timer is not told while it holds, only at
  the next busy turn (`mod.rs:238-248`). Same rule as for every timer; acceptable.
- A possible sibling of B1, checked and refuted: an SDK interrupt without the client's `perTaskStopAffordance`
  runs `Gue` (offset 302,207,374), but it stops only background agents and workflows (`p4`, offset 291,651,511), not
  shells or Monitors.
- The m4 parts the UI stop relies on: both header texts (`Dwe` at offset 292,574,466, `RJr` at 292,575,051) have no
  blank line inside and end with one, so the first `\n\n` split in `after_header` is right; Claude Code's own split
  joins notices with `\n\n` (`Ji`, near offset 319,332,248).
- Privacy: the fixes read `task_id`, `shell_id`, `run_in_background`, `timeoutMs` and `persistent`; `message` and
  `command` are skipped unread (`Key::Other`). No new file is opened; no `.key` path is involved.

Not checked: the `TaskStop`, Monitor timeout and panel stop paths live (all three are for `p4-live-claude`).
