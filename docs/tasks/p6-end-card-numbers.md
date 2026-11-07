# The video end card's speed numbers on the build that ships

Phase: 6. Status: in progress (2026-11-07 04:44). Needs Haik: no (Haik approved the video
2026-11-07 00:05; "the final render may change only the end card's speed numbers, if the new measurement on the final
install differs").
Agent: `wakeshot-worker`. Size: about 80K tokens.
Runs after: `p6-logs-diagnostics` (its build is the one that ships and is on Haik's install), `p6-pages` (the site's
numbers). Parallel with: cards that do not use the dry-run tray, `media/`, the README speed lines or the site's numbers.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p6-end-card-numbers\target` if a release build is needed; delete it when
done.

## Goal

The end card says "Idle: 5 MB of memory, 1016 ms of CPU in 10 minutes". `p4-idle-cpu` brought idle CPU back to about
109 ms. Measure again on the build that ships, by the `p4-idle-cpu` method, and re-render only the end card.

## Read first

- `docs/tasks/p4-idle-cpu.md` Result (the method and its open item 1: a dry run beside the installed tray starts its
  window at least 5 minutes after the last OpenCode activity).
- `docs/research.md` "Speed numbers" and "Speed numbers after the idle CPU fix".
- `media/README.md`, `docs/tasks/p6-media.md` (checks), `docs/tasks/p6-agents-video-readme.md` (the last render and its
  checks).

## Deliverables

1. Idle CPU and private memory in 10 minutes on a release dry run of the current tree (the code of
   `p6-logs-diagnostics`), the `p4-idle-cpu` way. Take turns for the dry-run tray through
   `C:\Users\haika\AppData\Local\Temp\claude\f7c2c08a8f04f07b\b896b7b1-ede4-4a36-9332-d8c5355d7200\scratchpad\dry-run.lock`.
2. The end card with the new numbers, worded as the video already words them; `media/out/wakeshot-promo.mp4` and the
   poster if it shows them. Every frame before the end card and the whole sound track stay the same bytes when
   decoded (compare frame and audio hashes against the approved render), and every `p6-media` check passes.
3. The same numbers wherever the README, `site/` and `docs/research.md` show the old figure (additions only in
   `docs/research.md`, re-read right before each edit; change only the number lines in README and `site/`).

## Acceptance checks

- The measurement table in the Result (before and after, method, date).
- Frame and audio hash compare: only end card frames differ.
- Every `p6-media` check passes on the new render.

## Rules

- Never run git, not even read-only. Never read token files. Kill only processes you started, by PID. Start test
  programs at Medium through explorer.exe. Never touch Haik's installed tray. Never set `RUSTFLAGS`.
- Plain short English, no em dashes, no emojis. Write Log lines as you go.

## Questions for Haik

None tonight: decide and log "for Haik's review".

## Found outside scope

## Log

- 2026-11-07 03:51: card written by the orchestrator (session wakeshot-4e).
- 2026-11-07 04:44: started (worker, session wakeshot-4e). Part A (the idle measurement) first, then a stop until the
  orchestrator says go on with Part B. Exes from `p6-logs-diagnostics`'s x64 nupkg (its pack folder), no cargo build.
- 04:44: `wakeshot-tray.exe` 3,179,520 bytes `9DEAB8A4...` and `wakeshot.exe` 873,472 bytes `8BB76101...` taken from
  the nupkg; both equal Haik's installed exes (`ProductVersion` 0.1.0). Before: installed tray 20124 and guard 50240
  (started 04:37:24); no OpenCode process; OpenCode's last write `opencode.db-wal` 03:27:45 (the `-shm` time 04:37:24
  is the installed tray's start read). No other keep-awake app.
- 04:44:46: dry-run lock taken. 04:44:49: dry run pid 49240 (`--dry-run`, `CLAUDE_CONFIG_DIR` on an empty scratch
  folder, other agent homes real) started through `explorer.exe`, level Medium (`S-1-16-8192`); its log: four sources,
  "Copilot: 7 sessions". 04:45:27 to 04:45:43: its watch thread switched 0 times in 16 s (no 2 s poll held on).
- 04:45:45: 10-minute window started (60 samples, 10 s apart, `sample.ps1` of `p4-idle-cpu`).
- 04:55:53, window 04:45:51.6 to 04:55:52.7, dry run 49240, no log line in the window (0 holds): CPU
  (`TotalProcessorTime`) 140.625 ms in 10 minutes (0.023% of one core, 9 ticks of 15.625 ms); cycles
  (`QueryProcessCycleTime`, 1.988 million per ms) 93.7 ms; private memory median 2.63 MB, max 2.73; working set median
  17.50 MB, max 17.56. `p4-idle-cpu` after its fix: 109.375 ms, 87 ms by cycles, 3.33 MB, 17.62 MB. Raw:
  `<scratchpad>\p6-end-card-numbers\speed\idle-1.json`.
- 04:56:25 to 04:56:50, read only: Haik's installed tray 20124 (same exe bytes, holding for the orchestrator's Claude
  Code session since 04:37:25, its update check done at 04:42:25 "no update found") read 3.98 to 4.01 MB private,
  20.5 MB working set, 50 modules; the dry run 2.67 MB, 41 modules (it checks no update and shows no menu).
- 04:57:10: dry run quit by `taskkill /PID 49240` (exit 0, gone in 0.3 s, its log "Wakeshot ends with code 0", no
  journal entry). 04:57:25: dry-run lock deleted. Only Haik's tray 20124 and guard 50240 run. Sleep read after: AC
  18000, DC 0, the installed tray's held values, as the dry run's start line read them (`powercfg` printed no lid
  lines). Part A done; waiting for the orchestrator's go for Part B.

## Result
