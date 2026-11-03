# Phase 0 close: decisions for Haik and proposed doc changes

Card: `docs/tasks/p0-close-proposals.md`. Worker, 2026-11-03, about 05:30. Nothing was run; files only.

Inputs: `docs/reviews/p0-architecture.md` (findings 1 to 23, tests 1 to 7), the ten proposed rule changes in
`docs/research/claude-code-signals.md:280-305`, the results of `p0-unelevated-liveness` and `p0-distribution-facts`,
and the "for the close" lines in `docs/phases/00-facts.md`.

How to use it:

1. Haik answers the two decisions in part 1, and says yes to part 2 (or names the items to drop).
2. The orchestrator applies part 2.
3. After the morning steps and the delta review, the orchestrator applies part 3.

Rules used: a decision fixed in `product.md` stays unless a fact contradicts it, and the item names that fact. Line
numbers are current. The review's `research.md` lines after line 82 are now about 50 higher, because a section was
added at lines 82 to 130 after the review was written.

## 1. Decisions for Haik

### Decision 1: does the lid-closed time limit apply to a docked laptop?

Source: finding 5. `product.md:58` ("With the lid closed: hibernate after a fixed time limit") against `product.md:8`
(keep the PC awake while something needs it).

**Question.** The lid is closed and an external screen is on. Should Wakeshot still hibernate the laptop when the
lid-closed time limit runs out?

**Default: no.** The time limit (and the heat guard, decision 2) apply only when the lid is closed and no external
screen is active. Reason: the limit exists for the laptop in a bag (`product.md:40-41`). A laptop that drives a cabled
screen is on a desk. Hibernating it stops the agent work the product exists to protect. LidGuard also skips its
suspend while any external screen is on (`rivals-nightcrew-lidguard.md:172`).

**Other choice.** An overnight agent run on a docked, closed laptop is hibernated at the limit, and the work stops.

Built with either answer:

- The lock rule stays as fixed (`product.md:52-54`): for the lock, docked means the user's own lid action is Do
  nothing. Your lid action is Sleep, so a lid close on the DELL still locks: by your own setting a lid close means
  "sleep". The review's "an external screen means docked, so no lock" is not adopted, because no fact contradicts the
  fixed rule. Nightcrew fixed its docked bug the same way (`rivals-nightcrew-lidguard.md:194`). To work on the DELL
  with the lid closed, set your lid action to Do nothing.
- The time-limit hibernate and the end-of-work sleep or hibernate run only after some minutes with no input
  (Nightcrew's "unattended", `rivals-nightcrew-lidguard.md:172`, `174`). Without this, Wakeshot can hibernate someone
  who types on an external screen with the lid closed, against `architecture.md:36-37` ("never hibernates the PC in
  front of the user").
- When the user's own lid action is Do nothing, Wakeshot changed nothing, so at work end it only releases ("then let
  it sleep as usual", `product.md:8`). No forced sleep.
- If the screen count drops to internal only while the lid is closed (undock), the closed-lid rules apply from then.

Step D of `docs/phases/00-morning-steps.md` confirms the mechanism: whether the internal screen leaves the count at
lid close while the DELL stays, and what undocking and unplugging the charger with the lid closed do
(`rivals-others.md:324-326`, trap 12).

**Haik's answer (2026-11-03): no.** The limit does not apply while an external screen is on (the default).

### Decision 2: heat guard

Source: finding 18; `00-facts.md:52`.

**Question.** Should Wakeshot hibernate a closed laptop that runs hot? `product.md` has no heat rule today.

**Default: yes, in a lean form.** Lid closed, no external screen, the thermal zone above one fixed value for a few
minutes: hibernate (or the hibernate-off fallback, part 3 item 2) and notify. No setting. If a laptop has no zone, log
it and skip the guard. Reason: the hot laptop in a bag is the story behind the time limit (`product.md:40-41`), but
the limit bounds how long, not how hot. The sensor reads without admin and follows CPU load within seconds
(`research.md:478-487`). LidGuard ships a similar rule: hibernate at 93 C with the lid closed and no external
screen (`rivals-nightcrew-lidguard.md:175`).

**Other choice.** A closed laptop in a bag runs hot until the time limit. Only Windows' own critical shutdown at
113 C (`_CRT`, `research.md:485`) stops it earlier.

The value comes from the heat log of morning step I (and step D3, if done).

**Haik's first answer (2026-11-03): "no I guess", with the question why temperature matters.** The orchestrator
explained the bag case once (a closed laptop in a bag doing a long job; time limits bound only lost signals, not real
work). Waiting for his final answer.

**Haik's final answer (2026-11-03): no heat guard.** Reason, his words in short: without Wakeshot a person sets the
lid to Do nothing by hand and puts the laptop in a bag, with no heat check at all. Wakeshot automates that path and
should not add extra care on top. The hardware's own protection applies the same in both cases: the CPU slows itself
at its limit, and Windows shuts down at the 113 C critical trip point (`research.md:485`). The heat stop inside the
lid spike tool stays: it only guards Haik's laptop during the test.

### Decision 3 (raised by Haik, 2026-11-03): no safeguards for other programs' bugs

**Haik's answer:** "If some other program has a bug we should not do extra safeguard for it. We can simply not have
that bug." Applied at the close:

- Drop the fixed lid-closed time limit (`product.md:58`). A closed laptop holds as long as the work runs.
- Replace "Every signal has a time limit on our side" (`product.md:40-41`) with: every signal is checked against a
  fact. A session counts only while its process is alive (handle wait, `p0-unelevated-liveness`). A timer read from
  the transcript counts only until its due time plus a short margin (this also ends an Esc cancel, which writes
  nothing to disk). No cap on `busy` or `shell` while the process lives.
- Drop part 2 change 3 ("A cap for `busy`") and any other change that exists only to guard against another
  program's bug. Recheck each part 2 item against this rule at the close.
- Stays: the 1 hour limit on waiting for approval (Haik's earlier product rule; it covers the user walking away, not
  a bug) and the battery floor.

## 2. Changes the orchestrator can apply after Haik's yes

Grouped by file. Each item: where, the old idea, the new idea, the source. Items are numbered so Haik can drop one by
number.

**Recheck against Haik's answers (card `p0-close-recheck`, 2026-11-03).** Each item 1 to 25 has one tag line. Rule
from decision 3: an item that exists only to guard against another program's bug (a lost signal, a hung process, a
stale file) is dropped or reworded. The 1 hour limit on waiting for approval and the battery floor stay.

| Tag | Count | Items |
|---|---|---|
| Keep | 17 | 1, 2, 4, 6, 8, 9, 10, 11, 12, 13, 15, 16, 17, 18, 19, 20, 23 |
| Change | 5 | 5, 21, 22, 24, 25 |
| Drop | 3 | 3, 7, 14 |
| New | 8 | 26 to 33 (the edits decisions 1 to 3 need themselves) |

Three tags rest on two defaults that Haik's yes would confirm: items 21 and 22 (an unknown `status` value counts as
working while the session process lives) and item 14 (no hang check for our own tray). See "Questions from the
recheck" after item 33. Haik's yes to part 2 covers the tagged form: Keep and Change items as tagged, Drop items not
applied, new items 26 to 33 applied.

### `docs/product.md`

1. **Rust reason** (finding 14). "Platform and stack", line 20. Old: "cleanup runs on drop" is one reason for Rust.
   New: remove those words; Rust stays. Fact: the release profile uses `panic = "abort"` (`research.md:333`), which
   skips every `Drop` on panic, and Velopack ends the app with `TerminateProcess` (`distribution.md:36-37`). The
   restore rule goes to `architecture.md` (item 16).

   **Recheck: Keep.** A fact about our own build, not a guard.

   Applied 2026-11-03.

2. **The Claude source reads the transcript too** (finding 23). Line 27. Old: "from its status files". New: "from its
   status files, plus the session transcript for timers and for background tasks of non-terminal sessions. Only the
   listed fields are parsed; prompts are never stored or logged" (`claude-code-signals.md:237-238`).

   **Recheck: Keep.** The transcript is where the timer due times come from, which decision 3 checks against.

   Applied 2026-11-03.

3. **A cap for `busy`** (finding 7; the review's "missing from the list"). New bullet after line 28: "`busy` stops
   holding after a long time with no growth of the transcript, the subagent transcripts or the task output files, and
   a notification says so. An unknown status value gets the same cap. Value set in Phase 1; the review suggests about
   2 hours. It must be long, because a quiet foreground command also shows `busy` with no growth." Fact: status files
   have no heartbeat (`claude-code-signals.md:64`); `product.md:40-41` already wants a limit on every inferred signal.
   The value goes to Phase 1 like the lid-closed limit (`product.md:58`).

   **Recheck: Drop.** Decision 3 names it. A dead session is already caught by the process check; a live session
   stuck in `busy` is Claude's bug, and decision 3 adds no guard for it. Unknown `status` values: items 21 and 22.

4. **Timers: only what the transcript shows** (finding 11, last bullet; Claude change 10). Line 29. Old: "an idle
   session with a pending timer holds when the next run is within 1 hour". New: "an idle session with a pending timer
   that the transcript shows ...". Add: "No hold for Claude's hidden loop fallback." Fact: the fallback leaves no line
   on disk until it fires, and its timing was seen once (`claude-code-signals.md:262`), so holding for it would rest
   on a guess.

   **Recheck: Keep.** It holds only on what the transcript shows, which is decision 3's rule. The due time plus
   margin goes into the new line 40 (item 27).

   Applied 2026-11-03.

5. **Background shell decided** (Open 1; finding 9; finding 23, first bullet). Line 32. Old: "`shell`: see Open 1".
   New: "`shell`, and a background task found in the transcript of a non-terminal session (VS Code, SDK, Desktop):
   holds for at most 1 hour after the session was last `busy`. Then a notification says Wakeshot stopped and suggests
   `wakeshot run`." Delete Open 1. Why it is decided here: at line 114 Haik handed Open 1 and 2 to the product owner,
   and this is the pick Open 1 already applies. Finding 9: VS Code and SDK sessions never show `shell`
   (`claude-code-signals.md:258`), and Desktop sessions very likely write the same files (not verified, `:206-208`).

   **Recheck: Change.** The 1 hour cap is gone: decision 3 says "no cap on `busy` or `shell` while the process
   lives", and that overrides the Open 1 pick. Line 32, old: "`shell` (background command while idle): see Open 1."
   New: "`shell` (background command while idle), and a background task found in the transcript of a non-terminal
   session (VS Code, SDK, Desktop): holds while the session process lives and the task is open. No time cap. A dev
   server that never ends holds until it or the session ends; the tooltip says what holds." Delete Open 1. The
   transcript check for non-terminal sessions (finding 9) stays.

   Applied 2026-11-03.

6. **WSL decided: not in v1** (Open 2; finding 22). Delete Open 2. Add to "Later, not v1" (line 42): "Claude Code
   inside WSL. Its session files carry Linux pids, which Windows cannot check, so stale files would hold forever.
   Revisit with a helper inside WSL." Then delete Haik's message (line 114) and the empty "Open" section.

   **Recheck: Keep.** Its reason is decision 3's rule: a WSL session cannot be checked against a Windows process, so
   it would have no fact behind it.

   Applied 2026-11-03.

7. **"Every inferred signal"** (finding 23). Line 40. Old: "Every signal has a time limit on our side." New: "Every
   inferred signal (agent files) has a time limit on our side. `wakeshot run` ends with its command; Keep awake ends
   at its time or at sign-out."

   **Recheck: Drop.** It keeps a time limit on agent signals, which decision 3 replaces. Item 27 rewrites the same
   lines 40-41 and keeps this item's `wakeshot run` and Keep awake words.

8. **"Restore my power settings" during a hold** (finding 21). "User settings safety", after rule 3 (line 67). New:
   "'Restore my power settings' counts as a user change: restore now, stop overriding until the current work ends,
   keep the power request, notify." Source: rule 3 (`product.md:66-67`). Without it the item either does nothing (the
   next tick overrides again) or the icon says "Holding" while a lid close now sleeps.

   **Recheck: Keep.** A user action, not a guard.

   Applied 2026-11-03.

9. **Tray icon note is verified** (finding 23; `00-facts.md:106-107`). Line 96. Old: "(Windows 11 hides new tray
   icons; not verified)". New: "(a new tray icon lands in the `^` overflow: seen in the tray spike,
   `research.md:288-292`)".

   **Recheck: Keep.** A verified fact.

   Applied 2026-11-03.

10. **Updates wait for a quiet moment; every hook restores** (finding 8; finding 1). "Distribution", lines 106-107.
    Old: "auto-update from GitHub Releases, uninstall hook that restores power settings". New: "auto-update from
    GitHub Releases, applied only when nothing holds and no setting is overridden. The install, update and uninstall
    hooks restore power settings from the journal. A `winget upgrade` during `wakeshot run` still kills the wrapper;
    the README says so." Fact: an update kills the tray app, the guard and `wakeshot.exe run`, all in the install
    folder (`distribution.md:36-49`), so a closed laptop goes to standby mid-task.

    **Recheck: Keep.** Velopack ending the app on update is how it works, not a bug, and the update is our own.

    Applied 2026-11-03.

11. **Signing** (finding 23). Line 110. Old: "SignPath Foundation (free for open source). Unsigned preview builds
    until it approves." New: "v1 ships unsigned. Apply to SignPath Foundation once the project has users; signed
    builds then show 'SignPath Foundation' as publisher." Fact: SignPath declined new projects without users in 2026
    (reported), and its certificates name SignPath Foundation (`research.md:217-220`). "Free options only" stays.

    **Recheck: Keep.** Not touched by the answers.

    Applied 2026-11-03.

### `docs/architecture.md`

12. **Data outside the install folder** (finding 1, the blocker; `p0-distribution-facts` design implications 1 to 4).
    "Files", line 48. Old: all data in `%LOCALAPPDATA%\Wakeshot\`. New heading: "Files (a data folder outside the
    Velopack install folder)". Add:
    - Phase 1 picks the pack id and the data folder name. They must differ.
    - The `--veloapp-install` and `--veloapp-updated` hooks run "restore if a journal exists", like app start. The
      uninstall hook restores, removes start at login and any logon task, and stays small (30 s limit; a failed hook
      does not stop the uninstall).
    - A second copy of the original values lives outside the data folder (an HKCU key, as Nightcrew does), unless
      part 3 item 4 picks the plan copy, which marks itself.
    - Phase 6 checks winget `UpgradeBehavior: uninstallPrevious` (not verified with Velopack).

    Fact: Velopack installs to `%LocalAppData%\{packId}`. Setup.exe over an install (what `winget upgrade` does) kills
    the app, runs no uninstall hook and deletes the folder (`distribution.md:43-46`, `114-116`, tested). With pack id
    `Wakeshot` the journal with the user's original values is gone, and Do nothing then looks like the user's own value
    (Nightcrew bug `787bee4`, `rivals-nightcrew-lidguard.md:192`).

    **Recheck: Keep.** It fixes our own design (the blocker), not another program's bug. The HKCU copy guards the
    user's original values if our own journal is lost; part 3 item 4 can still make it unneeded.

    Applied 2026-11-03.

13. **One loop thread, and nothing blocks it** (findings 13 and 3). "Runtime", line 33. Old: "One thread, one Windows
    message loop". New: "One loop thread owns all state. Power and session notifications come to the tray's hidden
    window (`WM_POWERBROADCAST`, Nightcrew's way, `rivals-nightcrew-lidguard.md:170`). Where only a callback exists, the
    callback only posts a message to the loop. Calls that can block (file reads, WinRT) run off the loop thread. Phase
    1 lists which." Fact: the spikes' `DEVICE_NOTIFY_CALLBACK` runs on a system thread
    (`spikes/lid-sleep/src/events.rs:27-30`). Registration with a window handle is documented but not tested here.

    **Recheck: Keep.** This is how Wakeshot can "simply not have" Nightcrew's frozen-tick bug (`7ccb678`, Haik's
    words in decision 3): nothing that can block runs on the loop thread.

    Applied 2026-11-03.

14. **Guard: catch a hung tray, run during every hold** (finding 3). "Processes", Guard, lines 45-46. Old: runs only
    while settings are overridden and reacts only to process death. New: "Runs whenever a hold is active or settings
    are overridden. Restores at once if the tray app dies. Every 30 s it sends the tray window `WM_NULL` with
    `SendMessageTimeout` and `SMTO_ABORTIFHUNG`. After about 2 minutes with no answer it ends the tray process (Windows
    then drops its power requests) and restores." Fact: a live but hung process keeps its request, and the time limit
    and battery floor run on the same thread (Nightcrew bug `7ccb678`, `rivals-nightcrew-lidguard.md:195`).

    **Recheck: Drop (default, question 2).** The hang check is an extra safeguard against a hang in our own tray, the
    bug Nightcrew shipped. Item 13 removes its cause, which is decision 3's "we can simply not have that bug". The
    guard stays as `architecture.md:45-46` has it: it runs while settings are overridden and restores at once if the
    tray app dies. Without the hang check it has no job during a hold with nothing overridden: if the tray dies then,
    Windows drops its requests by itself. Change it when: a tray hang shows up in Phase 3 tests or in a user report.

15. **Guard: start detached; logon restore** (finding 17). Same place. New: "Start the guard detached, not as a child
    of the tray; Phase 3 tests what Task Manager's End task ends. While settings are overridden, register a logon
    restore, so a crash with start at login turned off still restores at the next sign-in; remove it at restore.
    Phase 1 picks the mechanism (Nightcrew uses a logon task, `rivals-nightcrew-lidguard.md:157`)." Not verified:
    whether End task on the app group ends a child guard too.

    **Recheck: Keep.** It covers a user's End task and a power cut with start at login off: real events, not another
    program's bug. It serves product rule 5 (`product.md:69`).

    Applied 2026-11-03.

16. **Restore never only in `Drop`** (finding 14). New line under "Processes": "Settings restore never depends on
    `Drop`. A panic hook restores before the abort (per the review; not tested here). The guard and the restore at
    start cover the rest."

    **Recheck: Keep.** Restore on crash is product rule 5; this makes our own code meet it.

    Applied 2026-11-03.

17. **One settings owner per machine** (finding 16). "Processes", Tray app, line 41. Add: "Power plans are
    machine-wide (HKLM, `research.md:378`), so only one instance at a time changes settings (a `Global\` mutex).
    Instances in other sessions hold power requests only."

    **Recheck: Keep.** Two users on one machine is a real case, not a bug.

    Applied 2026-11-03.

18. **CLI and hold file gaps** (finding 20; finding 8). "Processes", CLI (lines 42-44), and the `holds` row (line 53).
    Add:
    - The CLI's Keep awake cannot use the CLI's own pid, because the CLI exits at once. Phase 1 picks: the hold carries
      the tray's pid and start time (it then ends with the tray, which also gives "ends at sign-out"), or an expiry.
    - `status` reads a state file that the tray writes. Today no file holds the tray's state.
    - Watch `config.json` too. It is not in the watched folders (line 34).
    - A hold from `wakeshot run` also records the child's pid, so the hold outlives a wrapper killed by an update.

    Not adopted: the review's "keep the hold format internal in v1". `product.md:37-38` makes it a documented format
    and no fact contradicts that. Phase 1 should keep it easy to write from a script (the review's worry is the
    FILETIME start time).

    **Recheck: Keep.** Each bullet ties a hold to a fact (a live pid, the tray's own pid) or fills a gap in our own
    design. The child pid covers our own update ending our own wrapper, not another program's bug.

    Applied 2026-11-03.

19. **See the user's changes with a poll** (finding 12). "Runtime". New: "While overriding, read the overridden
    values every 10 s, plus the power source and active plan notifications the core needs anyway. No registry watch:
    its key path is not a documented API (`research.md:381`). Add it only if a 10 s delay proves a real problem."
    Fact: a read of five values takes under 1 ms (`research.md:385`); Nightcrew polls the same way
    (`rivals-nightcrew-lidguard.md:147`).

    **Recheck: Keep.** It serves product rule 3 (the user's change wins).

    Applied 2026-11-03.

20. **The CLI on PATH** (finding 19). "Processes", CLI. New: "The install and updated hooks add the CLI's folder to the
    user Path and broadcast `WM_SETTINGCHANGE`. The uninstall hook removes it." Fact: Velopack has no PATH feature
    (`distribution.md:50-51`; not verified that none exists). Not verified: that the `current` folder keeps its path
    across updates.

    **Recheck: Keep.** Not touched by the answers.

    Applied 2026-11-03.

### `docs/research/claude-code-signals.md`

21. **Adopt the rule changes into "Recommended detection rules"** (Claude changes 1 to 10; findings 2, 7, 9, 10, 11;
    the review's table at `p0-architecture.md:281-296`). Edit the table at lines 223-235 as below. At line 282
    replace "Not adopted yet; the Phase 4 design decides" with "Adopted at the Phase 0 close; Phase 4 refines".

    | Rule | New text | Source |
    |---|---|---|
    | 2 | Confirmed from an unelevated (Medium) reader against elevated sessions too. Open with exactly `PROCESS_QUERY_LIMITED_INFORMATION \| SYNCHRONIZE`; one extra right fails the open. `0x80070057` means gone. `0x80070005` means no access, never dead: fall back to the system process list (create time equal to `procStart` means alive). A leftover also vanishes at the next Claude Code start | change 8; finding 2, resolved by `p0-unelevated-liveness` (`research.md:82-130`); `00-facts.md:145-146` |
    | 3 | Sessions whose `entrypoint` is not `cli` (`claude-vscode`, `sdk-*`, `claude-desktop`, `claude-desktop-3p`, `local-agent`): a tool result with `backgroundTaskId`, or a Monitor `taskId`, opens a task. A user line that starts with `<task-notification>` for that id with `completed`, `failed` or `killed` closes it. An open task is working, under the 1 hour cap (item 5) | change 1; finding 9 |
    | 3 | `busy`, and any unknown value: the busy cap (item 3) | finding 7 |
    | 4 | `waitingFor: "dialog open"` is a slash command dialog: treat it like `idle` | change 2 |
    | 5 | Remember the `tool_use` ids of `CronCreate`, `CronDelete`, `ScheduleWakeup` and the background task tools. Take each result by `tool_use_id`; skip `is_error` results. Also match `scheduled_task_fire`, `session_cron_carry`, and `<task-notification>` at the start of a user line. Keep parser state between reads; after a Wakeshot restart, read each transcript from its start | finding 10 |
    | 5 | `nextFireAt` and `scheduledFor` are floats. Drop a wakeup at its `scheduled_task_fire` (`taskKind: "loop"`), or at `scheduledFor` plus a margin of a few minutes, which also bounds the invisible Esc cancel. Never hold past that | change 3 |
    | 5 | Key all job state by pid, not by `sessionId`. When `sessionId` changes, read the new transcript from its start and take `session_cron_carry` (`otherArmed`, `tasks`) as the armed jobs | change 4 |
    | 5 | A reminder comes as `CronCreate` with `recurring: false` or as `ScheduleWakeup`: handle both | change 5 |
    | 5 | Loop fallback: no hold (item 4) | change 10 |
    | 6 | Replace the jitter formula. A recurring job with a cron period of 1 hour or less holds. A longer period uses the next cron time after the last fire. A one-shot uses its exact `nextFireAt` or `scheduledFor` | finding 11 |
    | 7 | Durable jobs: dropped for v1; add when a `scheduled_tasks.json` is seen. Fact: durable jobs are off in 2.1.316 (live test 6) | change 6; finding 11 |
    | 9 | Decided: within 1 hour (`product.md:29`). Remove "Haik's decision" | stale line |
    | 10 | `claude agents --json`: dropped for v1. It starts a 335 MB process, has open bug #101182, and `bg` sessions write status files and transcripts too (line 182) | change 7; finding 11 |
    | note | `updatedAt` is not a heartbeat or a last-write time | change 9 |

    **Recheck: Change.** Three rows change; every other row stays as above (the rule 5 margin row already matches
    decision 3: a wakeup counts until `scheduledFor` plus a margin).

    - First rule 3 row, last sentence. Old: "An open task is working, under the 1 hour cap (item 5)". New: "An open
      task is working while the session process lives. No time cap (decision 3)".
    - Second rule 3 row. Old: "`busy`, and any unknown value: the busy cap (item 3)". New: "`busy`, `shell` and any
      unknown value: working while the session process lives, no cap. An unknown value also shows 'update Wakeshot'
      (`product.md:97`)". Source: decision 3; the unknown value part is question 1.
    - Rule 4 row. Add: "Otherwise `waiting` holds only while `bridgeSessionId` is set, for at most 1 hour
      (`product.md:31`). Replace 'Product choice: hold for a grace period, longer if `bridgeSessionId` is set'".
      Source: a stale line like rule 9; decision 3 keeps the 1 hour limit.

    Applied 2026-11-03.

### `CLAUDE.md`

22. **Unknown shapes** (finding 23). Line 17. Old: "Treat any unknown shape as 'no signal'." New: "A file that does not
    parse is no signal, and Wakeshot shows 'update Wakeshot' (`product.md:97`). An unknown `status` value counts as
    busy, under the busy cap." Fact: rule 3 (`claude-code-signals.md:227`, high confidence) counts unknown values as
    working, so the two docs disagree today. With the cap, a wrong guess is bounded.

    **Recheck: Change (default, question 1).** The busy cap is dropped (item 3), so the fact that bounds an unknown
    value is the live process (decision 3). New text for line 17: "A file that does not parse is no signal, and
    Wakeshot shows 'update Wakeshot' (`product.md:97`). An unknown `status` value counts as working while the session
    process lives, and shows the same notice." Today's line 17 still reads as quoted above.

    Applied 2026-11-03.

23. **Mark the untested Windows facts** (finding 23; `research.md:26-30`, `34-38`). Lines 19 and 20. Add "(Microsoft
    docs; not tested by us)" to the lid close line and to the ExecutionRequired line.

    **Recheck: Keep.** Lines 19 and 20 checked today.

    Applied 2026-11-03.

### `docs/phases/00-facts.md`

24. **Superseded log line** (finding 23, last bullet). Line 45. Append: "(superseded: Keep awake now has 2 hours,
    `product.md:77`)".

    **Recheck: Change.** Same edit; only the place moved. The line is "- Rejected by earlier decision: "hold 2h" menu
    item (product.md: no "Hold awake" mode; the CLI covers it)." It is line 48 today, and the card table above it
    keeps growing, so find it by its text.

### New files (a small worker card, no Haik)

25. **Keep the live-test fixtures** (review test 7). Not done: `spikes/claude-watch/fixtures/` does not exist. Copy the
    relevant transcript lines and status file snapshots of the live tests to that folder, prompts and paths redacted,
    never a `.key` file. Do it before anyone deletes the `cctest*` folders in `~/.claude/projects`, which
    `p0-claude-live-tests.md:36-39` suggests. Phase 2 and 4 parser tests need real shapes.

    **Recheck: Change.** The plan above no longer works: the five `cctest*` transcript folders in
    `~/.claude/projects` were deleted this morning, and `spikes/claude-watch/fixtures/` does not exist. What is left,
    checked 2026-11-03 by listing file names and counting markers (no line copied; the first 12 lines of the watcher
    log were read to see its format):

    - **The live-test watcher log is still on disk**, against `claude-code-signals.md:327-328` ("not kept"):
      `C:\Users\haika\AppData\Local\Temp\claude\C--Users-haika-code-wakeshot\1828f2de-68d5-4148-9124-8321986ceb05\scratchpad\live\watch.log`
      (1078 lines, 01:27 to 02:00), with `watch.stdout.txt` and `agents.json` (`claude agents --json` output) next
      to it, and `cctest\.claude\scheduled_tasks.lock` one folder up. It holds full status file snapshots and the
      summarizer's line for each watched transcript line. Lines that name: `CronCreate` 6, `CronDelete` 4,
      `ScheduleWakeup` 6, `scheduled_task_fire` 7, `session_cron_carry` 1, `task-notification` 12,
      `backgroundTaskId` 3, `waitingFor` 10, `sdk-cli` 2. Prompts there are already `<str N>`; the user name in
      paths, session ids and `bridgeSessionId` values are not redacted. It sits under `%TEMP%`, so a temp cleanup can
      delete it (whether one runs on this laptop: not verified).
    - The live-tests worker's own transcript, a second copy of much of that output:
      `~/.claude/projects/C--Users-haika-code-wakeshot/1828f2de-68d5-4148-9124-8321986ceb05/subagents/agent-a9478976b79b588cb.jsonl`
      (3 MB; its `.meta.json` says "Do card p0-claude-live-tests"). Not opened.
    - Haik's own transcripts in `~/.claude/projects`: real background shell results with `backgroundTaskId` (26
      lines in 7 files), `<task-notification>` user lines (82 in 3 files), Monitor calls (6), `queue-operation` lines.
      No timer line at all: 0 for `CronCreate`, `ScheduleWakeup`, `scheduled_task_fire`, `session_cron_carry`,
      `nextFireAt` and `scheduledFor` as real fields. These files hold Haik's prompts.
    - One live status file today (`~/.claude/sessions/20636.json`, the orchestrator's session).
    - Field-level shapes quoted in `claude-code-signals.md:247-278`, and hand-made test lines modeled on them in
      `spikes/claude-watch/src/summary.rs:322-404`.
    - `spikes/claude-watch/out/unelevated-liveness/*.log`: summaries only (status, kind, entrypoint, procStart), no
      raw JSON.

    New text for item 25: **Keep the live-test shapes** (review test 7). A small worker card, no Haik. Run it first
    after the yes, because the source sits under `%TEMP%`. Copy from `live\watch.log` one example of each shape the
    parser rules need (each `status` and `waitingFor` value, each `entrypoint`, the `CronCreate`, `CronDelete` and
    `ScheduleWakeup` calls and results, `scheduled_task_fire` with and without `taskKind`, `session_cron_carry`, a
    task notification, a background shell result), plus `agents.json` and the lock file, into
    `spikes/claude-watch/fixtures/`. Replace the user name in every path, session ids and `bridgeSessionId` values;
    keep prompts as `<str N>`. From Haik's own transcripts take at most one background shell result and one task
    notification, every text field replaced. Never a `.key` file. Then fix `claude-code-signals.md:327-328`. If the
    log is gone by then, re-run live tests 3, 4, 5, 9 and 14 with `spikes/claude-watch/ccdrive.py` (Haiku, throwaway
    folders; it marks those folders trusted in `~/.claude.json` again, so remove the entries after). Phase 2 and 4
    parser tests need these shapes.

    Applied 2026-11-03.

### New items for decisions 1 to 3 (recheck, 2026-11-03)

Each item quotes the current text, checked against the file on 2026-11-03. Apply 26 and 28 before part 3 item 2,
which edits the same lines later.

26. **No lid-closed time limit, no heat guard** (decisions 2 and 3). `docs/product.md`, line 58.
    Old: "- With the lid closed: hibernate after a fixed time limit (value set in Phase 1)."
    New: "- With the lid closed, Wakeshot holds for as long as the work runs: no time limit and no heat guard. Someone
    who sets the lid to Do nothing by hand and puts the laptop in a bag gets no heat check either, and the hardware's
    own protection applies the same in both cases: the CPU slows itself at its limit (not tested here), and Windows
    shuts down at the critical trip point (113 C on the test laptop, `research.md:485`)."

    Applied 2026-11-03.
27. **Signals are checked against facts** (decision 3; replaces item 7). `docs/product.md`, lines 40-41.
    Old: "- Every signal has a time limit on our side. Claude's files have no heartbeat, and a lost "work ended" signal
    must never hold forever (LidGuard bug: 59 hours, hot laptop in a bag)."
    New: "- Every signal is checked against a fact, not capped by time. A Claude session counts only while its process
    lives (a wait on the process handle), so a crashed session or a leftover file never holds. A timer counts until
    its due time plus a short margin; this also ends a wakeup cancelled with Esc, which writes nothing to disk. `busy`
    and `shell` have no cap while the process lives. A hold file counts while its process lives, so `wakeshot run`
    ends with its command. Keep awake ends at its time or at sign-out. No extra safeguard for another program's bug
    (Haik, 2026-11-03). Two limits stay because they are product rules, not bug guards: waiting for approval holds at
    most 1 hour (line 31), and the battery floor."

    Applied 2026-11-03.
28. **What is left of the docked rule** (decision 1). `docs/product.md`, line 59.
    Old: "- When work ends with the lid closed, the laptop sleeps or hibernates soon after (method: Phase 0)."
    New: "- When work ends with the lid closed, the laptop sleeps or hibernates once there has been no input for a few
    minutes (value in Phase 1), so someone typing on an external screen with the lid closed is not put to sleep
    (method: Phase 0). If the user's own lid action is Do nothing, Wakeshot changed nothing, so it only releases its
    hold."

    What is left of decision 1: Haik's answer kept the time limit off a laptop with an external screen. With no time
    limit (item 26) and no heat guard, that exception has nothing left to apply to. Of decision 1's "Built with either
    answer": the lock rule stays as written (`product.md:52-54`, no edit); the time-limit hibernate is gone; the wait
    for no input before the end-of-work sleep and the release-only rule for Do nothing are this edit; the undock rule
    is moot, because no closed-lid rule depends on screens any more. So no v1 rule counts screens, and Phase 1 needs
    no `QueryDisplayConfig` (the method stays recorded, `research.md:472-477`). Part 3 item 2 later fills "(method:
    Phase 0)" in this same line.

    Applied 2026-11-03.
29. **Rival ports not adopted** (decisions 2 and 3). `docs/research/rivals-nightcrew-lidguard.md`, "Port to
    Wakeshot".
    - Lines 243-244, port 3. Old: "3. **Bound every "still working" signal by time.** LidGuard's 59 h leak came from
      one signal with no expiry. Give "busy" with no transcript growth a long cap, and drop Remote Control waiting
      after the planned limit." Append: "Not adopted for `busy` (decision 3, 2026-11-03: signals are checked against
      facts). Kept: Remote Control waiting ends after 1 hour (`product.md:31`)."
    - Lines 263-265, port 15. Old: "15. **Heat guard** (LidGuard): read
      `Win32_PerfFormattedData_Counters_ThermalZoneInformation` (works without admin on this laptop) and hibernate
      above a fixed threshold while lid-closed. Cheap, and it addresses the "hot in a bag" report. Threshold needs
      testing; ACPI zones may lag CPU heat (U)." Append: "Not adopted (decision 2, 2026-11-03: no heat guard)."

    Applied 2026-11-03.
30. **Heat idea in the other rivals report** (decisions 2 and 3). `docs/research/rivals-others.md`, lines 288-289,
    idea 14.
    Old: "14. Lid mode safety on top of the 20% floor: release on unplug if the lid is closed and battery is low;
    Windows has no simple user-mode temperature reading, so heat can only be a lid-closed time limit. Unverified for
    heat."
    New: "14. Lid mode safety on top of the 20% floor: release on unplug if the lid is closed and battery is low. A
    user-mode temperature reading does exist (PDH thermal zone, `research.md:478-487`), but Wakeshot has no heat guard
    and no lid-closed time limit (decisions 2 and 3, 2026-11-03)."

    Applied 2026-11-03.
31. **Nightcrew UX note** (decision 3). `docs/research.md`, line 192.
    Old: "- "Stay awake for approvals", "Lid-closed limit" and "Battery floor" are settings that should be fixed
    behavior."
    New: "- "Stay awake for approvals" and "Battery floor" are settings that should be fixed behavior. "Lid-closed
    limit" should not exist at all (decision 3, 2026-11-03)."

    Applied 2026-11-03.
32. **Lid spike design notes** (decisions 1 and 2). `docs/tasks/p0-spike-lid-sleep.md`, "Implications for the design",
    lines 176-179. That card is still open (blocked on the morning steps), so apply this when it closes.
    - Old: "- Docked check: count active `QueryDisplayConfig` paths whose output technology is not internal. No WMI.
      Treat Miracast and indirect displays as not docked. Confirm in D that the internal path drops when the lid
      closes." New: the same line with this in front: "No v1 use after decisions 1 to 3 (no rule counts screens);
      kept for later."
    - Old: "- Heat guard: PDH `\Thermal Zone Information(*)\High Precision Temperature`, not WMI. Pick the threshold
      below the zone's `_CRT` (113 C here). If a laptop has no zone, log it and skip the guard." New: "- Heat: no
      product use (decision 2, no heat guard). The spike's 85 C stop guards only the tests. If one is ever needed: PDH
      `\Thermal Zone Information(*)\High Precision Temperature`, not WMI."

    Moot: neither old line is left in `p0-spike-lid-sleep.md` after its Result was rewritten (delta review, m8).
33. **Phase log design inputs superseded** (decisions 1 to 3). `docs/phases/00-facts.md`, work log, the entry
    "`rivals-nightcrew-lidguard.md` done" (lines 52-55 today; find by text, the card table above keeps growing).
    Append to each line:
    - "Every "still working" signal needs a time limit. LidGuard issue #5: one lost "work ended" event left the lid on
      Do nothing for 59 hours; laptop got hot in a bag." Append: "(superseded: decision 3, signals are checked against
      facts)".
    - "Docked detection: also check external displays (LidGuard WMI rule), not only the user's lid action." Append:
      "(no v1 rule uses it after decisions 1 to 3)".
    - "Heat guard candidate: thermal zone reads without admin here (about 50 C). Verify in spikes." Append: "(not
      adopted: decision 2)".

### Questions from the recheck

Haik's yes to part 2 takes the default in each.

1. **An unknown `status` value** (items 21 and 22). Default: it counts as working while the session process lives,
   and Wakeshot shows "update Wakeshot". Why: missing real work is the failure the product exists to prevent, and
   the hold ends with the session, with the tooltip saying what holds. Other choice: no signal until Wakeshot is
   updated; if Claude renames `busy`, the PC sleeps mid-task until then.
2. **A hang check for our own tray** (item 14). Default: none, item 14 dropped. Why: item 13 removes the cause, and
   decision 3 prefers not having the bug to guarding against it. Other choice: keep the guard's 30 s `WM_NULL` ping
   (small code; it catches any hang of ours, for example a blocking Windows call that Phase 1 missed).
3. **Battery floor with the lid closed and an external screen on** (from the recheck card's "Found outside scope").
   Today `product.md:56-57` hibernates a closed laptop at the floor, even while the user types on an external screen,
   against `architecture.md:36-37`. Default: at the floor, release every request, restore settings and notify at
   once; hibernate only after the same few minutes with no input as item 28.

   Applied 2026-11-03 (as item 34).

**Haik's answer (2026-11-03): "do as you think will be better for everyone (balanced)".** The orchestrator took the
default in all three, and reads this, with his earlier "I want to do minimum work", as his yes to part 2 with the
recheck tags (Keep, Change, Drop, new items 26 to 33) plus question 3 as item 34. Haik can still name an item to
undo.

## 3. Waits for the morning results

Each item: the source, the step in `docs/phases/00-morning-steps.md`, what to read, and the edit for each outcome. The
delta review reads the logs first.

1. **Battery sleep timer** (`product.md:50-51`; review test 2). Steps C and I. Read C's `hold` log and I's first
   RESULT line. The rule is confirmed if, in I, a gap starts about 900 s after the last key press (the 600 s battery
   timeout plus 5 minutes), or if, in C, the request ends about 5 minutes after the copy's 60 s timeout. If confirmed:
   Wakeshot raises the battery sleep timer while it holds on battery, lid open or closed (value in Phase 1). If not:
   lines 50-51 keep only the lid action.

   **Tag: Still needed.** The answers keep the battery rule and the battery floor. With no lid-closed time limit, a
   closed laptop on battery now holds for as long as the work runs, so whether Windows ends the request on battery
   matters more, not less. Steps C and I still feed it.

   Applied 2026-11-03.

2. **Hibernate off or failing** (finding 4). Steps F (F2 hibernate from the app, F1 sleep, F3 screen off) and E (its
   second RESULT line: does a closed lid with no request reach standby). Edit `product.md:56-59`: "If hibernate is
   not allowed (`IsPwrHibernateAllowed` false) or `SetSuspendState(TRUE)` fails: release every request, restore
   settings, notify. Check the lid again right before acting." If E shows that a closed lid with no request does not
   reach standby, the fallback also needs the sleep or screen-off call that F shows to work. Fill "(method: Phase 0)"
   at line 59 from F and E.

   **Tag: Still needed.** The time-limit hibernate is gone (item 26), but two hibernates stay: the battery floor with
   the lid closed (`product.md:56-57`) and the end-of-work sleep or hibernate (line 59, item 28). Steps F and E still
   feed both. Line 58 will be gone after item 26, so the edit covers lines 56-57 and 59.

   Applied 2026-11-03.

3. **Idle lock** (finding 6; review test 3). Step H: when the `locked` field turns true against the display-off
   callback (`p0-spike-lid-sleep.md:74`). Edit `product.md:52-53`: lock when the display was turned off by the idle
   timeout, not when idle time passes the sleep timeout. If H shows that Windows locks by itself at screen off while a
   request is held, drop the idle part and keep only the lid-close lock. Fact behind it: screen off already starts
   Modern Standby here (`research.md:415-418`), and the display timeout (300 s AC, 180 s DC) comes long before the
   sleep timeout. A video player's display request keeps the screen on, so today's rule would lock mid-video.

   **Tag: Still needed.** The lock rule is a security rule; the answers do not touch it. Step H.

   Applied 2026-11-03.

4. **Plan copy or in place** (finding 15). Step B (Settings page, Vantage, restart with a copy active).
   `architecture.md:59-61` picks in place unless a test shows a reason. The review's reason for the copy: it marks
   itself (its name and description can carry the original plan GUID; `PowerWriteDescription` not tested), so restore
   needs no journal. The cost: a change the user makes during a hold lands on the copy
   (`p0-spike-power-settings.md:147-150`). Pick after B. If in place stays, part 2 item 12's HKCU copy is needed.

   **Tag: Still needed.** How Wakeshot changes the lid action is not touched by the answers. Step B.

   Applied 2026-11-03.

5. **Clock across standby** (finding 23, the clock bullet). The gaps in steps E to I. `architecture.md:36` and
   `research.md:170` rely on a clock that ignores sleep, but `QueryUnbiasedInterruptTime` matched interrupt time
   across Modern Standby sessions (`research.md:508-510`). If the gaps show it does not skip standby, line 36 becomes:
   "detect sleep by wall-clock jumps against a short tick, plus suspend and display notifications"
   (`p0-spike-lid-sleep.md:182-183`).

   **Tag: Still needed, with less at stake.** The lid-closed time limit was the longest timer that could hibernate
   the PC, and it is gone. Durations that stay: Keep awake (30 minutes to 4 hours), the 1 hour limit on waiting for
   approval, the few minutes with no input before the end-of-work sleep (item 28). A late end-of-work timer right
   after resume can still sleep the PC in front of the user (`architecture.md:36-37`). Timer due times are wall-clock
   values from the transcript and do not depend on this. The gaps in E to I still feed it; no step exists only for it.

   Applied 2026-11-03.

6. **Sound at lid close** (`product.md:55`). Steps D, E and I: did the sound play? Microsoft's audio rule allows
   playback at idle entry to standby, not at explicit entry such as a lid close (`research.md:467-469`). If it plays,
   no edit. If not, the delta review proposes another cue.

   **Tag: Still needed.** `product.md:55` is not touched by the answers. Steps D, E and I.

   Applied 2026-11-03.

7. **Lock at lid close, re-take after resume** (`product.md:49`, `52`). Step I (locks at close), step E (does Windows
   lock by itself, `--no-lock`), step H (is the request honored after resume without a re-take). Edit only if a
   result differs from the docs. Line 49 already re-takes after every resume, and the review says to keep that always.

   **Tag: Still needed.** The lock part is not touched by the answers. The re-take part meets decision 3: its reason
   in line 49 is a bug seen in another program (PowerToys Awake). If H shows `request NOT honored` without a re-take,
   it is a Windows fact and line 49 stays. If H shows the request honored, the delta review asks Haik whether the
   re-take stays (it costs one call per resume) or goes under decision 3. Steps I, E and H.

   Applied 2026-11-03.

8. **Letting the screen turn off** (the review's "Power and lid spike implications"). Steps E to I: the work stand-in
   must not pause while the screen is off or the lid is closed. `product.md:47-48` already holds the display only on
   request; if E to I confirm, no edit. If work pauses, the delta review proposes a change.

   **Tag: Still needed.** A closed laptop now works for as long as the job runs, so work that pauses with the screen
   off would matter more. Steps E to I.

   Applied 2026-11-03: confirmed, no edit needed.

No consumer: optional steps G and I3 (wake timers) and C7 (the older keep-awake call). Nothing in the design uses
them (review test 5). Block T (tray) feeds no review finding; its results go to `p0-spike-tray`.

Moot after Haik's answers (recheck, 2026-11-03), for the morning trim card:

- The optional heat run D3 and the heat value from the step I log: they fed only the heat guard (decision 2: none).
  The hand heat checks in D8, E6 and I4 stay: they protect the laptop during the test, not the design.
- The screen count with the lid closed (the `displays` field in D and E, "did the DELL stay on" in D5): it fed only
  the docked exception to the time limit (decision 1). With no time limit and no heat guard, no rule counts screens
  (item 28). Undocking (D6) and unplugging the charger (D7) with the lid closed still test the core lid feature: does
  a closed-lid hold survive the change (`rivals-others.md:324-326`, trap 12)?

## Coverage

Every review finding, review test and proposed Claude rule, with its one home. Finding 23 is nine separate doc
fixes, so each bullet is listed.

| Item | Home |
|---|---|
| Finding 1 (journal in the install folder) | Part 2, items 12 and 10 |
| Finding 2 (liveness from unelevated) | Part 2, item 21, rule 2 (resolved) |
| Finding 3 (hung tray) | Part 2, items 14 and 13 |
| Finding 4 (hibernate off) | Part 3, item 2 |
| Finding 5 (docked use) | Part 1, decision 1 (the "external screen means no lock" part is not adopted there, with the reason) |
| Finding 6 (idle lock) | Part 3, item 3 |
| Finding 7 (`busy` cap) | Part 2, items 3 and 21 |
| Finding 8 (updates mid-work) | Part 2, items 10 and 18 |
| Finding 9 (Desktop sessions) | Part 2, items 5 and 21 |
| Finding 10 (rule 5 filter) | Part 2, item 21 |
| Finding 11 (cut timer rules) | Part 2, items 4 and 21 |
| Finding 12 (one way to see changes) | Part 2, item 19 |
| Finding 13 (thread and callbacks) | Part 2, item 13 |
| Finding 14 (cleanup on drop) | Part 2, items 1 and 16 |
| Finding 15 (plan copy) | Part 3, item 4 |
| Finding 16 (two users) | Part 2, item 17 |
| Finding 17 (guard gaps) | Part 2, item 15 |
| Finding 18 (heat guard) | Part 1, decision 2 |
| Finding 19 (PATH) | Part 2, item 20 |
| Finding 20 (hold files, CLI) | Part 2, item 18 (internal format not adopted there, with the reason) |
| Finding 21 (restore during a hold) | Part 2, item 8 |
| Finding 22 (WSL) | Part 2, item 6 |
| Finding 23: Open 1 and 2 not answered | Part 2, items 5 and 6 |
| Finding 23: tray note verified | Part 2, item 9 |
| Finding 23: CLAUDE.md unknown shape | Part 2, item 22 |
| Finding 23: CLAUDE.md ExecutionRequired | Part 2, item 23 |
| Finding 23: status files only | Part 2, item 2 |
| Finding 23: every signal has a limit | Part 2, item 7 |
| Finding 23: signing | Part 2, item 11 |
| Finding 23: clock | Part 3, item 5 |
| Finding 23: "hold 2h" log line | Part 2, item 24 |
| Test 1 (new step D) | Done before this card: morning step D. Read for decision 1 |
| Test 2 (long closed hold on battery) | Done: morning step I. Read in part 3, item 1 |
| Test 3 (lock reading in H) | Done: `p0-spike-lid-sleep.md:74`. Read in part 3, item 3 |
| Test 4 (optional heat run) | Done: morning step D3. Feeds decision 2's value |
| Test 5 (skip wake timers, C7) | Done: moved to optional. Part 3, "No consumer" |
| Test 6 (unelevated check) | Done: `p0-unelevated-liveness`. Recorded in part 2, item 21, rule 2 |
| Test 7 (keep fixtures) | Part 2, item 25 (open) |
| Claude changes 1 to 5, 8, 9 | Part 2, item 21 |
| Claude change 6 (durable reader) | Part 2, item 21, rule 7: dropped for v1 (durable jobs off in this build) |
| Claude change 7 (`claude agents --json`) | Part 2, item 21, rule 10: dropped for v1 (cost, open bug, no use) |
| Claude change 10 (loop fallback) | Part 2, items 4 and 21 |
| `00-facts.md:106-107` (for the close) | Part 2, item 9 |
| `00-facts.md:145-146` (for the close) | Part 2, item 21, rule 2 |

After the recheck (2026-11-03), these homes moved: finding 3 is item 13 only (item 14 dropped); finding 5 is decision
1 and item 28; finding 7 is decision 3 and items 21, 22 and 27 (item 3 dropped); finding 18 is decision 2 and items
26, 29, 30, 32 and 33; finding 23 "every signal has a limit" is item 27 (item 7 dropped); test 4 (heat run) is moot;
test 7 is item 25 as changed.

The review's other judgments (`p0-architecture.md:298-317`: the agreed spike inputs and work log inputs) need no edit
now. Phase 1 reads them from `research.md` and the spike cards. Those that change a doc are above: the registry watch
(item 19), the clock (part 3, item 5), wake timers (no consumer), `SystemExecutionState` (no edit; useful only if
part 3 item 3 needs it).
