# The video end card's speed numbers on the build that ships

Phase: 6. Status: done (2026-11-07 05:24). Needs Haik: no (Haik approved the video
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

1. `docs/research/promo-video.md` lines 351 to 355 still say the final shows "5 MB, 1016 ms" and is 8,497,375 bytes
   (draft 9.10 MB). Now: 4 MB, 141 ms, 8,496,790 bytes, SHA-256 `6921D98B...`; draft 9,093,506 bytes (9.09 MB);
   loudness unchanged. Not this card's file (a docs agent owns it tonight).
2. Idle CPU on a dry run rose from 109.375 ms (87 ms by cycles, `p4-idle-cpu`) to 140.625 ms (93.7 ms) with the log
   work of `p6-logs-diagnostics`. Two ticks, one window each, so it may be noise; not investigated.

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
- 04:58: Part B started on the orchestrator's go. Its call, for Haik's review: the end card reads "Idle: 4 MB of
  memory, 141 ms of CPU in 10 minutes", CPU from the dry run, memory from a 10-minute read-only sample of the installed
  tray (whole MB only if the sample supports it). 04:58:5x: that sample started (`sample.ps1`, pid 20124, 60 samples,
  its own log read for hold lines).
- 05:09, window 04:58:58.6 to 05:08:59.6: installed tray 20124 private memory median 3.98 MB, max 3.98 (all 60
  samples), working set median 20.46 MB, max 20.55; no log line in the window; `state.json` "Claude Code (1
  session)" the whole time (written 04:37:25). Its CPU 109.375 ms (155.2 ms by cycles) while holding and beside cargo
  builds: not used. 3.98 rounds to 4.0, so "4 MB" is supported. Raw: `speed\installed-mem.json`.
- 05:10: backed up into `<scratchpad>\p6-end-card-numbers\`: `before\` (the render now in `media/out/`: MP4
  8,497,375 bytes `8DAFF243...`, poster `B6F7302B...`, the draft and its poster) and `approved\` (Haik's approved
  MP4, 8,497,349 bytes `123D674C...`, copied from `p6-agents-video-readme`'s backup). `data/speed.json` (4.0 MB,
  141 ms, both methods in the note and `raw`), `media/README.md` speed row, the README Lightweight line and the site's
  hero line (4 MB, 141 ms, 0.023%, link to the new research section). 05:11: typecheck, `check-art`, `npm run video`,
  `check-video` started (cargo builds run beside).
- 05:09 to 05:12: typecheck ok; `check-art` ok (9,458,740 pixels equal); `npm run video` ok (soundtrack -15.1 LUFS,
  peak -2.5 dBFS, as before); `check-video` ok (h264, 1920 x 1080, 30 fps, BT.709 tags, aac, 38.02 s, 8.50 MB, both
  loops and the six state icons). New final 8,496,790 bytes `6921D98B...`; the poster the same bytes as before
  (`B6F7302B...`). `docs/research.md` new last section "Speed numbers of the shipping build (2026-11-07)" (re-read
  right before the append). `site/check_page.py site`: 0 errors (the new anchor resolves).
- 05:13: `npm run check` ok (108 files, no em dash or emoji, 18 colour pairs; pace: 47 starts on the beat, 12
  framings; 5 of 5 agents, README and site lists agree). Loudness (`loud.mjs`, ffmpeg loudnorm after AAC): new
  -15.32 LUFS, -1.47 dBTP, LRA 16.2 LU; the approved file the same. 05:14: frame and audio compare against the
  approved and the previous render started; `npm run draft`, `check-video --draft`, `check-rebuild --final` started.
- 05:14:46: compare (`compare-final.mjs`: MD5 of every decoded frame, yuv420p, and of the decoded sound, s16le):
  against Haik's approved file (`123D674C...`) and against the render before (`8DAFF243...`) alike, 1140 frames each,
  frames 0 to 1019 the same bytes, 1020 to 1139 (the end card, from 34 s) differ; the sound the same bytes
  (`334aa818...`, 7,299,186 bytes). Frames 1020, 1030 and 1049 (before the speed line pops at 1050) differ by at most
  3 to 4 levels (encoder). Frames 1100 and 1138 against the render before: the speed line's band (y 690 to 765) up to
  223 levels; the rest 23 channel values over 8 (at most 17, around the text edges at y 857 to 1030: encoder). Against
  the approved file the small print differs too (its date 2026-11-06, the 4.3 MB and 109 ms line; the 01:47 render
  already changed both). Frame 1100 looked at: "Idle: 4 MB of memory, 141 ms of CPU in 10 minutes." and "Measured on
  Wakeshot 0.1.0, 2026-11-07, x64." Pass.
- 05:13 to 05:22: `npm run draft` ok (9,093,506 bytes; its poster the same bytes as before), `check-video --draft` ok
  (38.02 s, 9.09 MB), `check-rebuild --final` ok: a clean copy (`npm ci`, `check-art`, loop, states, draft, video)
  gives all 15 outputs the same bytes, the final (`6921D98B...`), its poster and the soundtrack included; its temp
  folder is gone.
- 05:23: `docs/research.md`: one more bullet in the new section (the render and the compare). Scratch cleaned: frame
  PNGs, the MP4 copy, the extracted exes and the empty Claude folder deleted; kept in
  `<scratchpad>\p6-end-card-numbers\`: scripts, `speed\`, `logs\`, `before\` and `approved\` (the two earlier
  finals). No process of mine left: only Haik's tray 20124 and guard 50240 run.

## Result

Status: done (2026-11-07 05:24).

**Measurement** (x64, the build that ships: `p6-logs-diagnostics`' exes, `wakeshot-tray.exe` `9DEAB8A4...`, the same
bytes as Haik's install; details in `docs/research.md`, "Speed numbers of the shipping build (2026-11-07)"):

| Number | Before (the end card until now) | After (the end card now) | Method now |
|---|---|---|---|
| CPU in 10 idle minutes | 1,015.625 ms, shown 1016 ms (installed tray, 2026-11-07 00:24, before the idle CPU fix) | 140.625 ms (0.023% of one core; 93.7 ms by cycles), shown 141 ms | release dry run at Medium through Explorer, Claude Code's folder empty, other homes real, 60 samples, 04:45:51 to 04:55:52, no log line |
| Private memory | 4.99 MB median, shown 5 MB (installed tray, idle) | 3.98 MB median, max 3.98, shown 4 MB | Haik's installed tray, read only, 60 samples, 04:58:58 to 05:08:59, holding for one Claude Code session, update check done |
| Dry run's own memory | (`p4-idle-cpu`: 3.33 MB) | 2.63 MB median, max 2.73 | not shown: no update check, no menu |

**Render**: only the end card's numbers changed (`data/speed.json`). New `media/out/wakeshot-promo.mp4` 8,496,790
bytes, SHA-256 `6921D98B12DC0A064C01DAA8F394472BB48C9130F7335EB2E913FDF77276E413`; the poster does not show the
numbers and is the same bytes (`B6F7302B...`).

**Checks**

| Check | Outcome |
|---|---|
| Frame and audio hash compare, against Haik's approved file (`123D674C...`) and the render before (`8DAFF243...`) | ok: frames 0 to 1019 the same bytes, 1020 to 1139 (the end card) differ; the decoded sound the same bytes. Outside the speed line, end card frames differ by at most 17 levels in 23 values (encoder) against the render before |
| typecheck | ok |
| `check-art` | ok, 9,458,740 pixels equal |
| `npm run check` (text, credits, colours, pace, agent lists) | ok |
| `check-video` (final) and `check-video --draft` | ok: h264, 1920 x 1080, 30 fps, BT.709, aac, 38.02 s, 8.50 MB and 9.09 MB; loops and state icons |
| `check-rebuild --final` | ok: all 15 outputs the same bytes from a clean copy |
| Loudness | -15.32 LUFS, -1.47 dBTP (at most -1), LRA 16.2 LU, as the approved file |
| `site/check_page.py site` | 0 errors (the new anchor resolves) |

**Files changed**: `media/data/speed.json`, `media/README.md` (speed row), `media/out/wakeshot-promo.mp4`,
`media/out/draft/wakeshot-promo-draft.mp4` (not kept in git), `README.md` (Lightweight line), `site/index.html` (hero
speed line), `docs/research.md` (new last section), this card. `media/public/sound/soundtrack.wav` was rewritten with
the same bytes (not kept in git). No file deleted in the repo.

**Decisions** (principal designer, for Haik's review):

1. The memory figure is the installed tray's, the CPU figure the dry run's. Why: the installed tray held for the
   orchestrator's session, so its CPU is not idle, and the dry run skips the update check and the menu, so its memory
   reads 1.4 MB low; memory barely moves between idle and holding (0.1.0: 4.27 against 4.24 MB). Change it when a
   quiet stretch lets the installed tray be measured idle for 10 minutes: then take both numbers from it.
2. "141 ms" on the README and the site too (not "0.1 s"), so all three say the same thing as the video.

**Open items**: `docs/research/promo-video.md` numbers ("Found outside scope" 1). ARM64 not measured.
