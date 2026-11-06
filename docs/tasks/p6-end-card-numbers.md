# The video end card's speed numbers on the build that ships

Phase: 6. Status: ready once `p6-logs-diagnostics` is done (new 2026-11-07). Needs Haik: no (Haik approved the video
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

## Result
