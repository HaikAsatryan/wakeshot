# Claude parsing review fixes: verify 2 (M1 part b, the UI stop)

Reviewer: `wakeshot-reviewer`, 2026-11-04, test run 01:14, written 01:18. Brief: the same as
`docs/reviews/p4-claude-parsing-verify.md`, narrowed to "Left from M1 part b" and the minors N1 and N3 that the fix
round answered.

Read: `docs/tasks/p4-claude-transcript.md`, "Claude parsing review fixes, round 2"; `transcript.rs` (the `queued`
path, `Line`'s `operation` and `notice`, `notice_ids`, `close`); `tests/claude_transcript.rs`; fixture
`ui-stop.jsonl` and its README row; rule 3 of `docs/research/claude-code-signals.md`; `docs/research.md`, "Claude UI
stop notice checks"; every `queue-operation` line of the live and hand fixtures. Claude Code 2.1.317
(`C:\Users\haika\.local\bin\claude.exe`, read only) was searched again for the stop, queue and notice paths. Byte
offsets point into that file; minified names are from that build only.

Run once: `cargo test -p wakeshot-core --locked --test claude_transcript` with the scratch target folder: 25 passed, 0
failed. The lib unit tests were not run. The folder was deleted after.

**Verdict: M1 part b is Fixed** in code, for background shells and shell Monitors. The `killed` notice of a UI stop
now ends the task at its `queue-operation` `enqueue` line. The line shape is from the 2.1.317 code and matches the live
enqueue lines; the panel stop itself was not run live (`p4-live-claude`). N1: done. N3: rule 3 done;
`product.md` and `architecture.md` still open (m2).

Counts: 0 blockers, 0 majors, 3 minors (one unverified).

## Answers to the brief

### Is M1 part b fixed? Yes, in code

- `transcript.rs:292` sends a `queue-operation` line to `queued` (`:302-306`), which closes the ids `notice_ids` finds
  in the line's `content` when `operation` is `enqueue`. `Line` reads `operation` as a `Word` and `content` only
  through `read_notice` (`:1226-1228`, `:1244-1245`).
- Checked against 2.1.317 (the worker's offsets hold). `ILe` awaits the kill, then calls `tUr` for a shell task with
  a `source` while `Ube()` is on (near 302,206,558). `Ube()` is `k("tengu_rosy_donut", true)` (293,636,218). `tUr`
  (301,313,178) builds `Ai({taskId, toolUseId, outputFile, status: "killed", summary})` and calls `dl` with
  `passive: true` and no `unlogged`. `dl` is `enqueuePendingNotification` (293,970,042). That function writes
  `s("enqueue", $fe(pi))` when it queues the item. `$fe` (293,940,822) gives the string value. `s` in `I3o`
  (293,948,812) writes `{type: "queue-operation", operation, timestamp, sessionId, content}`.
- Fixture `ui-stop.jsonl` line 3 has that shape and the notice text in the order `Ai` writes it.
- Live data agrees: every notice in fixtures 01, 02, 05, 07, 08, 12 and 13 has its `enqueue` line before the line that
  delivers it.

### Can the new path end a task that is still running? No case found

- The new path reads the same notice text as the user line and the `queued_command` attachment, with the same
  `notice_ids`. It only reads it earlier: when Claude Code makes the notice, not when it delivers it. So the question is
  whether any notice with a `<status>` and a shell or Monitor id is made while that task runs.
- `Ai` (292,796,155) writes a field only when it is set (`if(T)`). The order is task-id, tool-use-id, task-type,
  output-file, status, summary. So notices built with no status close nothing: the quiet-shell check-in (near
  298,210,086), the stall notice (`N6n`, near 298,212,200), Monitor events (`Aq`, near 301,312,300) and the bridge
  "pending" notices (no task id either). `block_ids` stops at `<summary>`, so a `<status>` in a notice's body or
  trailing text never counts.
- Builders that set a status were checked: the shell end `pht` (after the process exits), `F6n` (kill), `tUr` and
  `eLo` (after the kill), workflow ends, agent restart failures, MCP tasks (terminal states only) and the failure
  helper `Y`. None names a shell or Monitor id while that task runs. This is a code read, not a search of every `dl`
  caller.
- Other `enqueue` contents cannot pass: poll events start with `<event nonce=` (`Epr`, 289,332,366); agent and peer
  messages are wrapped (`</agent-message>` in fixture 07); typed prompts (fixtures 08, 12, 13) are read in place and
  close nothing. A typed prompt that starts with notification XML could close a task, but a user line already could;
  not new.

### Can it read anything it must not? No

- Kept: task ids only (`is_plain_id`, at most 64 characters, at most 64 per notice), and `operation` as an enum. No
  text is stored. `the_replay_keeps_no_text` (`transcript.rs:2873-2884`) feeds two `enqueue` lines with marked text
  and checks the `Debug` output.
- No new file: `mod.rs` was not changed; `wanted()` still asks only for the sessions folder and
  `<sessionId>.jsonl` tails. No `.key` path is involved.
- `content` is now read at the top level of every line kind, but only looked at in place: a prefix check, plus one
  split when the text starts with Claude Code's header.

### Does an `enqueue` with an unknown id, a closed id, or no notice do nothing? Yes

- `close` (`:567-571`) removes only the exact ids listed. An unknown or already closed id changes nothing. Tested
  in passing: `the_replay_keeps_no_text` queues a `killed` notice for `b0ther001` and `b1irxlyqr` stays open.
- No notice: `a_queued_notice_ends_its_task_at_once` (`:2446-2473`) covers an event with no `<status>`, a quoted
  notice, a prompt, no `content`, a list `content`, a non-text `operation`, and the `remove` and `dequeue` operations.
  Each case leaves the task open and raises no problem. `queued` never calls `flag`.

### Do the tests fail with the new code switched off? Yes, by reading (no mutation run: one run allowed, no code edits)

With the `queue-operation` arm removed, four tests expect a state that only the `enqueue` line gives, before any user
line:

- `a_queued_notice_ends_its_task_at_once`: 0 tasks right after the `enqueue`.
- `a_stop_from_the_ui_ends_the_shell_at_its_queued_notice`: 0 open at line 3; the fixture has no user line.
- `the_live_open_tasks`: 08 and 12 expect 0 at line 6; their user lines are at line 8.
- `the_live_monitors_hold_until_their_timeout`: 02 expects no timer at lines 6, 12 and 29; the user lines are at 9,
  15 and 31.

A mutation that ignores `operation` fails the `remove` case. This matches the worker's "4 tests fail".

## New findings

### m1 (minor, unverified). A stop of an MCP or WebSocket Monitor writes no `<status>` notice, so this fix does not reach it

- Where: `transcript.rs:302-306` needs a `<status>`. In 2.1.317, `ILe` calls `tUr` only for shell tasks (`Td(w)`)
  and for `mcp_task`. The `monitor_ws` kill (301,315,813) and the `monitor_mcp` kill (302,199,369) both end in `rUr`
  (301,314,260). `rUr` sets the status and sends only `Aq(..., "[Monitor stopped]", ...)`, an event with no
  `<status>`.
- What breaks: in a VS Code or SDK session, an MCP or WebSocket Monitor stopped from the panel keeps holding until its
  timeout timer. If its result says `persistent: true`, it holds until the session process ends.
- How likely: low. Not verified: that these Monitors' results carry `taskId` in the shape Wakeshot reads, and whether
  the default remote flags make them non-persistent. The first verify found the flags force every Monitor to
  non-persistent with at most 30 minutes (`hgi`); not rechecked for these kinds. A `TaskStop` still ends them (B1).
- Fix: in rule 3, say that a stop of an MCP or WebSocket Monitor writes no `<status>`, so it ends at its timeout or at
  the process end. Add it to `p4-live-claude` only if cheap.

### m2 (minor). `product.md` and `architecture.md` still describe the old behavior (rest of verify N3)

- `docs/product.md:43-46` says a UI stop "may hold until ... the next prompt, or until the session ends", and calls
  the output file "the only other trace". Both are now wrong for shells and shell Monitors.
- `docs/architecture.md:899-900`: `timers` does not mention Monitor timeouts, and `open_tasks` does not say it counts
  only tasks with no known end.
- Both are already known: the card's "Left" items 1 and 2, with proposed text, and `docs/phases/04-sources.md:261-262`.
  They are not the fix round's files.
- Fix: apply the card's proposed lines. If m1 holds, the `product.md` line should say "a shell or shell Monitor".

### m3 (minor). Rule 3 cites the wrong lines for "No time cap"

- Where: `docs/research/claude-code-signals.md:232`, near the end of rule 3: "No time cap (Haik, 2026-11-03,
  `product.md:44-50`)". That text is at `product.md:38-40`; lines 44-50 are the UI stop text and "Other agents".
- Fix: change the reference to `product.md:38-40`.

## Checked, no finding

- N1: rule 3 now says "A `TaskStop` that fails closes nothing." Done.
- N3, rule 3 part: rule 3 now says a Monitor that is not `persistent: true` is a timer. A timeout over 1 hour gets a
  due time every hour, at most 24; no `timeoutMs`, or 0, ends after 1 hour; both are named. This matches
  `transcript.rs:442-454`. A negative or non-number `timeoutMs` also ends after 1 hour; "no `timeoutMs`" covers it
  well enough. Done.
- Notices consumed as a tool result (`_Ms`, 298,213,767, when the model reads a shell's result with its get tool)
  leave no user line. Before this round those tasks held until the process ended; the `enqueue` line now ends them.
  One possible order is not covered: a shell could end before its tool result line is written. Then the `enqueue`
  comes first and closes nothing, and the later user line or attachment must close it. Not verified that this order
  happens; it is no worse than before.
- Resume: an `enqueue` from the earlier process closes only that process's tasks, which are already not counted.
- Split feeds: `every_fixture_transcript_in_any_pieces_gives_the_same_facts` includes `ui-stop.jsonl`. The lib
  proptest `sample()` has an `enqueue` line.
- Docs of the round: the fixture README row and `docs/research.md`, "Claude UI stop notice checks", match the code
  and the offsets above.

Not checked: the panel stop live (`docs/tasks/p4-live-claude.md:93-95` asks for it); the hook that can take an
enqueue over (`$r`); how soon the recorder's write reaches the disk; the lib unit tests (only the integration suite
was run).
