# Idle CPU back near 0.1.0's

Phase: 4. Status: done 03:37 (new 2026-11-07, orchestrator decision for Haik's review). Needs Haik: no.
Agent: `wakeshot-worker`. Size: about 80K tokens.
Runs after: `p6-agents-video-readme` Part B. Runs alone among Rust cards.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p4-idle-cpu\target`; delete it when done.
Real laptop: a `--dry-run` tray for the measurement; at the end Haik's install updated (as `p4-cursor-off` did it).

## Goal

The tray at rest now uses about 1,016 ms of CPU in 10 minutes; 0.1.0 used 109 ms (`p6-agents-video-readme` Result,
`docs/research.md` "Speed numbers"). Likely causes, inferred, not proven: every 30 s tick asks Restart Manager again
about dead Copilot lock files (14 to 22 ms each), and OpenCode's 2 s check costs about 250 ms per 10 minutes; both in
`crates/wakeshot-windows/src/watch/agents.rs`. Find the real costs, then cut them without losing an exact signal
("No guessing", `docs/product.md`): every hold must still start and end as the live cards measured.

## Deliverables

1. Measure first: where the idle CPU goes (a profile or counters per check), with the method of "Speed numbers".
2. Fixes, each with a test that fails before, for example: do not ask Restart Manager again about a lock file whose
   owner was already found gone until the file changes; run OpenCode's 2 s check only while an OpenCode process
   lives. Any other cost you find. No timer that ends or keeps a hold.
3. Memory while holding rose from 4.0 to 7.7 MB: a short look; fix only if the cause is clear and small.
4. Measure again the same way: idle CPU and memory before and after, in the Result and in `docs/research.md`
   "Speed numbers".
5. Whole-workspace checks on x64 and ARM64 (`CLAUDE.md`, Checks), then pack x64 and update Haik's install the way
   `p4-cursor-off` did (WM_CLOSE only, `Setup.exe --silent` at Medium through `explorer.exe`, hash, `config.json`
   kept, tray at Medium), and one Claude Code, one Codex and one OpenCode turn still hold and let go.

## Out of scope

- `README.md`, `site/`, `media/` (the video card writes the end card numbers after this card).

## Rules

- Never run git, not even read-only. Never kill Haik's tray: only WM_CLOSE. Start programs at Medium through
  `explorer.exe`. Kill only processes you started, by PID. Never read token files. Prompt text is never logged. Never
  set `RUSTFLAGS`; cargo may not be on PATH: `%USERPROFILE%\.cargo\bin\cargo.exe`; ARM64 needs the Build Tools
  `VC\Tools\Llvm\x64\bin` folder on that cargo command's PATH only.
- Plain short English, no em dashes, no emojis. Write Log lines as you go.

## Questions for Haik

## Found outside scope

1. **The installed tray's memory rise is most likely not the watch.** Haik's tray (pid 56164, 3 hours) reads 7.67 MB
   private, 445 handles, 11 threads, 72 modules; a dry run of the same code after 10 minutes reads 2.8 to 3.3 MB,
   about 228 handles, 5 threads, 41 modules. The 31 extra modules are network (`mswsock`, `dnsapi`, `iphlpapi`: the
   update check), shell launch (`windows.system.launcher`, `urlmon`, `edputil`) and UI text (`textinputframework`,
   `coremessaging`, `textshaping`: menu, tooltip, theme changes during the tooltip capture). No audio module is
   loaded in either. The new install (tray 48424) rose from 3.94 to 4.68 MB and from 44 to 51 modules across its
   first update check (03:04:59), which loaded the six network modules. Inferred from the module lists, **not
   verified**; no fix here (`update.rs` and the tray UI are not this card's). Change it when a dry run that shows the
   menu and runs an update check grows the same way.
2. **Copilot's first round costs about 0.77 s of CPU** at every tray start: the 7 sessions' `events.jsonl` are read
   from byte 0 (5 held by VS Code's long-lived `copilot-runtime.exe`). A start cost, not idle; not changed.
3. The two Copilot in-use files of crashed sessions (`inuse.8588.hold` from 2026-11-06 16:13, `inuse.9260.hold` from
   2026-11-03) stay until the next Copilot CLI start deletes them; they now cost nothing at rest.

## Log

- 2026-11-07 01:59: card written by the orchestrator.
- 2026-11-07 02:00: worker started; read the card, `p6-agents-video-readme` Result, `docs/research.md` "Speed numbers".
- 02:07: release build of the tree (x64) as the "before" exes (`bin-before\`; same sizes as the installed 0.1.0,
  hashes differ). A temporary example `examples/idle-profile.rs` (deleted at the end) runs one `Agents` per real
  source as the watch thread does at rest and counts thread cycles per part and per call (Restart Manager, process
  list, liveness checks, queries).
- 02:10: dry-run lock taken. Before dry-run tray (pid 46672, Medium, Claude Code's folder empty) started 02:10:48; the
  profile (Medium, 10 minutes) 02:11:06; the tray sampled 10 minutes from about 02:11:55.
- 02:24: before, measured. Tray 02:14:11 to 02:24:12: CPU 1,109.375 ms (cycles: 1,112 ms), private median 2.76 MB
  (max 3.59), working set 17.50 MB, no log line in the window. Profile, per 10 minutes (thread cycles): Copilot 825 ms,
  of it 780 ms in 38 Restart Manager questions about its 2 dead lock files (20.5 ms each, 2 per tick); OpenCode 299
  ms, of it 286 ms in 281 `Query` polls (1.0 ms each, though a warm look at the 3 files costs 0.09 ms: the cost is
  the cold wake); Claude Code 9, Codex 7; all 1,140 ms. One process list: 8.6 ms (359 processes). Before tray quit
  by `taskkill /PID` (code 0).
- 02:25: fix 1 written: no Restart Manager question again on a tick; an unheld answer is dropped when the facts of
  the session that names the lock file change. Two tests that used the tick to ask again now change the facts.
- 02:28: fix 1 tests: 49 of 49 agents tests pass; with the old tick rule put back the new test fails (20 questions
  in 20 ticks) and so does the changed one; with the facts check taken out 3 tests fail. Cold look experiment
  (Medium, a 2 s sleep before each, 15 rounds): one look at the db files 0.394 ms, three looks 0.620 ms, so one
  look per poll would save only about 65 ms per 10 minutes; the wake is the cost.
- 02:33: fix 2 design: the poll runs only while some process other than Wakeshot holds the database open, as
  Restart Manager tells (asked at a query's first run, after a notice naming the db, `-wal` or `-shm` while nobody
  held it, after a read that found a commit, and after every holder ended; never on a tick). Holders go to
  `Snapshots::live`, so their exits wake the thread. OpenCode's first open truncates and rebuilds a `-shm` it finds
  (SQLite's first-opener path, which gave the `-shm` notices of `p4-live-opencode`), so its start gives a notice; a
  start with no notice is still read at the next tick (the header look stays on the tick).
- 02:39: fix 2 written (`Hosts` per database in `Slot`, `hosts_poll`, `check_hosts`, `forget_hosts`; a `-shm`
  notice runs a query only to ask, never to read). Tests: 5 new in `agents.rs`, 1 new in `thread.rs`
  (`the_poll_stops_when_the_agent_that_held_the_database_exits`), the two thread poll tests now have a child that
  holds the database, `QueryRig` answers "held by an agent" unless a test says otherwise. 132 of 132 watch tests
  pass; with the old always-poll rule put back 7 fail (the 6 new ones and the `-shm` notice test).
- 02:40: `cargo fmt --all`, `cargo clippy -p wakeshot-windows --all-targets -D warnings` clean. Release build of the
  fixed tree (`bin-after\`).
- 02:52: after, measured the same way (dry-run tray pid 42512 started 02:41:04, sampled 02:42:18 to 02:52:19): CPU
  109.375 ms in 10 minutes (cycles: 87 ms), private median 3.33 MB (max 3.38), working set 17.62 MB, no log line in
  the window. Profile 02:41 to 02:51, per 10 minutes: Copilot 32 ms (no Restart Manager question), OpenCode 9 ms
  (no poll), Claude Code 9, Codex 6; all 57 ms (before 1,140). Tray quit by `taskkill /PID` (code 0); dry-run lock
  deleted; `examples/idle-profile.rs` deleted. Whole-workspace checks started.
- 02:56: `docs/research.md` new last section "Speed numbers after the idle CPU fix". Whole-workspace: fmt pass, x64
  clippy with and without `--all-targets` pass, `cargo test --workspace --locked` 1789 passed, 0 failed, 64 ignored
  over 22 binaries (7 more than `p4-cursor-off`'s 1782: the new tests). Two doc comments in `agents.rs` were
  reflowed to 120 columns after clippy ran (comments only; fmt and clippy run again at the end).
- 02:58: rest of the whole-workspace checks pass: x64 release build, ARM64 clippy with and without `--all-targets`,
  ARM64 release build (Build Tools `Llvm\x64\bin` first on PATH for those commands only); no `warning` or `error`
  line in any log; no CRT import in any exe. x64 `wakeshot-tray.exe` 3,153,408 bytes `4CFDB4E0...`, `wakeshot.exe`
  815,616 bytes `A4FD8477...`; ARM64 `wakeshot-tray.exe` 2,777,600 bytes `A5485648...`, `wakeshot.exe` 781,824 bytes
  `2E6CF9BD...`. `cargo fmt --all --check` and x64 clippy `--all-targets` again on the final tree: pass.
- 02:59: x64 packed (`packaging/pack.ps1 -Version 0.1.0 -Cpu x64`, no delta), exit 0:
  `HaikAsatryan.Wakeshot-win-x64-Setup.exe` 12,490,192 bytes, SHA-256
  `E9804FCD8410F31E5A073A567EAE93194D727FBFCEF431B9D6FF5BE4DD620880`; full nupkg 4,790,736 bytes. Before the install
  (02:55): Haik's tray 56164 and guard 54364; installed exes `242B9D7A...` and `ABA496CE...` (`p4-cursor-off`'s);
  `config.json` `4EBE0C4B...` (2026-11-05 20:32:43); power read: lid AC 0, DC 0; sleep AC 18000, DC 0; one SYSTEM
  request, the installed tray's (its held values for the orchestrator's session).
- 02:59:10: read-only monitor started (`monitor.ps1`, pid 41932). 02:59:27: Haik's tray 56164 quit by `WM_CLOSE` to
  its one `Wakeshot.Loop` window: exit 0 after 89 ms, guard 54364 exit 0; its log wrote lidAc 0 to 1, lidDc 0 to 1,
  sleepDc 0 to 18000, journal deleted, "Wakeshot ends with code 0". Read 02:59:30: lid AC 1, DC 1; sleep AC 18000,
  DC 18000; no SYSTEM request: Haik's own values.
- 02:59:39 to 02:59:43: `Setup.exe --silent` at Medium through `explorer.exe` (token `S-1-16-8192`), exit 0; "User
  chose to overwrite existing installation", install hook ran, "Installation completed successfully"; Setup started
  no tray. Installed `wakeshot-tray.exe` `4CFDB4E0...` and `wakeshot.exe` `A4FD8477...` equal the build;
  `config.json` same bytes and time (`4EBE0C4B...`, 2026-11-05 20:32:43).
- 02:59:59: tray started through `explorer.exe`: tray 48424 and guard 48116, both Medium, parent explorer.exe. Four
  sources in its start lines; "Holding starts: Claude Code" at 03:00:00.300 (the orchestrator's session).
- 03:00:35 to 03:00:41: Claude Code, one `claude -p ... --model haiku` test turn at Medium through Explorer (session
  pid 56556), answered `ok`, exit 0. The orchestrator's session holds the whole time; per session, `state.json`: busy
  at 03:00:37.390 (`statusUpdatedAt`), "Claude Code (2 sessions)" seen 03:00:37.477; idle at 03:00:40.833, back to
  "(1 session)" seen 03:00:40.930 (the monitor reads every 100 ms). Pass.
- 03:00:53 to 03:01:00: Codex CLI, one `codex exec --skip-git-repo-check -m gpt-6-luna -c
  model_reasoning_effort="low"` test turn at Medium (session `01a1b317`), answered `ok`, exit 0. Rollout
  `task_started` 03:00:56.300, "Hold starts: Codex" 03:00:56.311 (0.01 s); `task_complete` 03:00:59.170, "Hold ends:
  Codex" 03:00:59.181 (0.01 s). Pass.
- 03:01:16 to 03:01:49: OpenCode 2.2.5, one `opencode run --model ollama/qwen3:4b-instruct` test turn at Medium in an
  empty work folder, Haik's real data home, answered `ok`, exit 0. Its row (`oc-times.py`, SQLite `mode=ro`, times
  only): `time_busy` 03:01:18.666, "Hold starts: OpenCode" 03:01:19.018 (0.35 s); `time_idle` 03:01:48.826, "Hold
  ends: OpenCode" 03:01:50.107 (1.28 s); before the fix the install check measured 2.01 s and 1.63 s. A watcher on the
  data folder with the tray's filter saw `opencode.db-shm` change at 03:01:17.797, at OpenCode's start, then `-wal`
  and `opencode.db` changes by 03:01:18.397: the start notice the gate relies on is **verified**. The run left its
  background service (pid 47556); `opencode service stop` at Medium, exit 0; no OpenCode process left. Pass.
- 03:02 to 03:13, a live check that the poll stops (dry-run lock taken 03:02:50; dry run of the fixed build, pid
  46628, started 03:02:52, after the service stopped): two 2-minute samples read 90 and 82 ms of cycles (about 430 ms
  per 10 minutes, not 87). Its watch thread took 42.8 ms in 60 s and switched every 2 s (thread counters): it polled,
  though Restart Manager now said nobody holds the database (`rm-who.ps1`) and no notice came in 40 s. A profile of
  the same sources at that time asked 3 times in its first round (one question per query want) and polled nothing.
  Cause, inferred: one of the dry run's 3 start questions caught the installed tray reading the database right after
  the dry run's own read (whose `-shm` rebuild gave the installed tray a notice), so it kept that tray as a holder
  for as long as it lives. A virus scan at an agent's exit could do the same. Not verified which process it held.
- 03:18: fix 2b: while a database is held, who holds it is asked again `HOSTS_RECHECK_MS` (5 minutes) after the last
  question, at a poll; the queries on one database ask at most once per call (`Ctx::asked`). Tests:
  `a_database_still_held_is_asked_about_again_after_the_recheck_time`, `the_queries_on_one_database_ask_once_per_call`;
  134 of 134 watch tests pass; with the two guards broken both fail. Dry run 46628 quit (`taskkill /PID`, code 0);
  `examples/idle-profile.rs` (made again at 03:12 for that check) deleted. The installed build lacks fix 2b, so the
  checks, the pack and the install run again.
- 03:19 to 03:21: whole-workspace checks again on the final tree, all pass: fmt; x64 clippy with and without
  `--all-targets`; `cargo test --workspace --locked` 1791 passed, 0 failed, 64 ignored over 22 binaries; x64 and ARM64
  release builds; ARM64 clippy with and without `--all-targets`; no `warning` or `error` line; no CRT import. x64
  `wakeshot-tray.exe` 3,155,968 bytes `D05EC7D2...`, `wakeshot.exe` 815,616 bytes `C0E7D90E...`; ARM64
  `wakeshot-tray.exe` 2,780,672 bytes `CA967332...`, `wakeshot.exe` 781,824 bytes `581FEFD7...`.
- 03:21:45: x64 packed again, exit 0: `HaikAsatryan.Wakeshot-win-x64-Setup.exe` 12,491,601 bytes, SHA-256
  `C1F09BE2E857F06351E15E672D90A3ABE58551F4A14169C0038D55A21008FB63`; full nupkg 4,792,145 bytes.
- 03:22: tray 48424 (first install, 22 minutes) read 4.68 MB private and 51 modules; at 03:00 it read 3.94 MB and 44.
  Its update check at 03:04:59 ("no update found") loaded `mswsock`, `dnsapi`, `iphlpapi`, `nsi`, `rasadhlp`,
  `fwpuclnt`. Quit by `WM_CLOSE` at 03:22:07: exit 0 after 77 ms, guard 48116 exit 0; read 03:22:10: lid AC 1, DC 1;
  sleep AC 18000, DC 18000; no SYSTEM request. `Setup.exe --silent` at Medium through Explorer 03:22:20 to 03:22:25,
  exit 0, "User chose to overwrite", hook ran, "Installation completed successfully"; installed exes equal the build
  (`D05EC7D2...`, `C0E7D90E...`); `config.json` same bytes and time. Tray started through Explorer 03:22:37: tray
  41472 and guard 44468, both Medium, parent explorer.exe; four sources; "Holding starts: Claude Code" 03:22:37.712.
- 03:22:53: final dry run (pid 46820, dry-run lock still held) started for the stop check. Sampled 03:24:05 to
  03:26:05 before any turn: CPU 15.625 ms, cycles 15.7 ms in 2 minutes (about 79 ms per 10 minutes), private 3.54 MB.
- 03:26 to 03:28, the three turns again on the final install (tray 41472), each at Medium through Explorer, each
  answered `ok`, exit 0. Claude Code (session pid 21476): busy at 03:26:35.209, "(2 sessions)" seen 03:26:35.263;
  idle at 03:26:37.798, "(1 session)" seen 03:26:37.849. Codex (session `01a1b32f`): `task_started` 03:26:52.346,
  "Hold starts: Codex" 03:26:52.606 (0.26 s); `task_complete` 03:26:55.804, "Hold ends: Codex" 03:26:55.815
  (0.01 s). OpenCode (row `ses_e4ccfdcb`): `time_busy` 03:27:19.427, "Hold starts: OpenCode" 03:27:19.805 (0.38 s);
  `time_idle` 03:27:45.129, "Hold ends: OpenCode" 03:27:45.155 (0.03 s); the dry run logged the same start and its
  end at 03:27:45.849 (0.72 s). Notices at OpenCode's start again: `opencode.db-shm` 03:27:18.425, then `-wal` and
  `opencode.db`. Service (pid 55900) stopped with `opencode service stop` at Medium, exit 0, by 03:27:59; no OpenCode
  process left. Pass, all three.
- 03:28 to 03:36, the stop check: the dry run read 77.8 ms of cycles in 03:28:25 to 03:30:25 and its watch thread
  still switched every 2 s at 03:31:25, so it kept a holder after the exits (most likely the installed tray again,
  which reads on the same notices; not verified). At 03:33:58 the switches were gone (the 5-minute question), and
  03:34:06 to 03:36:06 read 16.4 ms. The installed tray read 99.3 ms in the same 2 minutes, holding for the
  orchestrator's busy Claude session (not an idle number). Dry run quit by `taskkill /PID` (code 0), dry-run lock
  deleted, monitor stopped by its stop file.
- 03:36: power read: lid AC 0, DC 0; sleep AC 18000, DC 0; one SYSTEM request, the installed tray's: its held values
  for the orchestrator's session (the reads after each quit showed Haik's own values; this card wrote no power
  setting itself). No process of mine left (no dry run, no OpenCode, no profile). Build folder (8.5 GB), `bin-*`,
  the first `pack-x64\`, `claude-empty\` and `work\` deleted. Kept in `<scratchpad>\p4-idle-cpu\`: `pack-x64-final\`,
  scripts, `speed\`, `logs-*\`, `run\`. `docs/research.md` section finished.

## Result

Status: done (2026-11-07 03:37).

**Costs found** (profile of the real sources at rest, thread cycles, per 10 minutes; tray as a dry run beside Haik's):

| Cost | Before | After |
|---|---|---|
| Copilot: Restart Manager asked again at every 30 s tick about the 2 in-use files of crashed sessions (20.5 ms each, 38 questions) | 780 ms | 0 |
| OpenCode: the 2 s `Query` poll with no OpenCode running (281 polls, 1.0 ms each; a warm look costs 0.09 ms, so the cost is the cold wake) | 286 ms | 0 |
| Everything else of the four sources (ticks, tails, listings) | 74 ms | 57 ms |

**Fixes** (`crates/wakeshot-windows/src/watch/agents.rs`; no contract, Cargo or `lockowner.rs` change):

1. Restart Manager is never asked again on a tick. A "nobody holds it" answer for a lock file is kept until the file
   changes (listing or notice) or the facts of the session that names it change (its log moved, so a process may
   have reopened a lock file a crash left, which gives no notice).
2. OpenCode's poll runs only while some process other than Wakeshot holds the database open (Restart Manager), asked
   at a want's first run, after a notice naming the db, `-wal` or `-shm` while nobody held it (OpenCode's start gives
   a `-shm` notice: verified live), after a read that found a commit, after every holder ended (holders are waited
   on), and every 5 minutes while held; once per call. The tick still looks at the header, so a start with no notice
   is read within 30 s.

Tests (each fails with the old rule put back): `a_lock_file_nobody_holds_is_asked_about_once_however_many_ticks_come`,
`a_query_is_not_polled_while_nobody_holds_its_database`,
`a_shm_notice_while_nobody_held_the_database_starts_the_poll_without_a_read`,
`the_poll_stops_after_every_holder_of_its_database_ended`,
`a_commit_a_tick_finds_while_nobody_held_the_database_asks_again`,
`a_failed_question_polls_as_before_and_asks_again_only_on_the_tick`,
`a_database_still_held_is_asked_about_again_after_the_recheck_time`, `the_queries_on_one_database_ask_once_per_call`
(`agents.rs`); `the_poll_stops_when_the_agent_that_held_the_database_exits` (`thread.rs`). Three older tests now get
their answer by a facts change or a holder child instead of the tick.

**Idle, measured the same way before and after** (release dry run at Medium, Claude Code's folder empty, real Codex,
Copilot and OpenCode homes, a quiet minute, 60 samples over 10 minutes):

| Number | Before | After |
|---|---|---|
| CPU in 10 minutes (`TotalProcessorTime`) | 1,109.375 ms (0.18% of one core) | 109.375 ms (0.018%) |
| CPU by cycles (`QueryProcessCycleTime`) | 1,112 ms | 87 ms |
| Private memory, median (max) | 2.76 MB (3.59) | 3.33 MB (3.38) |
| Working set, median | 17.50 MB | 17.62 MB |

What differs from the 0.1.0 method: a dry-run copy, not the installed tray (the orchestrator's session holds that
one); Claude Code's folder empty (0.1.0 watched one idle Claude session; its share here is about 9 ms); three more
sources on real files; a cycle-time column. Details in `docs/research.md`, "Speed numbers after the idle CPU fix".

**Memory, short look**: not the watch. A dry run stays at 2.8 to 4.1 MB; the installed tray grows with its update
check (+0.74 MB, network modules) and likely with menu and tooltip use. Not fixed ("Found outside scope" 1).

**Checks**: `cargo fmt --all --check`; x64 and ARM64 clippy with and without `--all-targets`, `-D warnings`;
`cargo test --workspace --locked` 1791 passed, 0 failed, 64 ignored; x64 and ARM64 release builds; no CRT import:
all pass on the final tree.

**Install** (as `p4-cursor-off`): Haik's tray quit by `WM_CLOSE` (exit 0, his values back: lid Sleep AC and DC, 18000 s
AC and DC); `Setup.exe --silent` at Medium through Explorer, exit 0; installed exes equal the build; `config.json`
same bytes; tray 41472 and guard 44468 at Medium. One Claude Code, one Codex and one OpenCode turn hold and let go:
Claude under 0.06 s both ways; Codex 0.26 s and 0.01 s; OpenCode 0.38 s and 0.03 s (before: 2.01 s and 1.63 s).
**Setup.exe** (x64): `HaikAsatryan.Wakeshot-win-x64-Setup.exe`, 12,491,601 bytes, SHA-256
`C1F09BE2E857F06351E15E672D90A3ABE58551F4A14169C0038D55A21008FB63`, in
`C:\Users\haika\AppData\Local\Temp\claude\f7c2c08a8f04f07b\2a1d3942-3be1-4fdb-95cd-69265f507ad6\scratchpad\p4-idle-cpu\pack-x64-final\`.

**Files changed**: `crates/wakeshot-windows/src/watch/agents.rs`, `crates/wakeshot-windows/src/watch/thread.rs`
(tests only), `docs/research.md` (new last section), this card. The temporary `examples/idle-profile.rs` is deleted.

**Decisions** (principal designer):

1. Gate the poll on Restart Manager, not on a process list: a list costs 7 to 9 ms here, so a list every tick costs
   more than the poll, while Restart Manager runs only on events. Change it when an OpenCode build opens its database
   without a `-shm` (exclusive locking): then its start gives no notice and only the tick catches it.
2. Ask again about a lock file nobody held when its session's facts change, not on a timer. Change it when a live
   case shows a lock file taken up again with no log line while its session should hold.
3. Ask again every 5 minutes while a database is held, since a reader seen for a moment (seen live: most likely the
   other tray) would otherwise keep the poll on. Change it when a scanner or second tray keeps it on in real use often:
   then drop holders whose image is `wakeshot-tray.exe` and Restart Manager's service holders.

**Open items**:

1. The end card and README numbers (video card): 109 ms in 10 minutes (87 ms by cycles), private 3.3 MB, on a dry
   run with Claude Code's folder empty. A dry run beside the installed tray must start its window at least 5 minutes
   after the last OpenCode activity.
2. ARM64 not measured. OpenCode's start notice verified on one run of 2.2.5 only.
