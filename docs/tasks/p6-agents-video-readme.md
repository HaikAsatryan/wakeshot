# Video and README for every AI agent, not Claude Code first

Phase: 6. Status: later (new 2026-11-06, Haik). Part A starts when Haik answers the question below; Part B after
the Round 3 live tests (the supported list and the tray text are final then). Needs Haik: yes: his yes to Part A's
plan, then his look at the render and the README.
Agent: `wakeshot-worker`. Size: about 200K tokens (Part A about 50K, Part B about 150K).
Runs after: `p6-media` (round 6, the final of 2026-11-06), `p6-readme` (round 5); Part B after every `p4-live-*` card
of the bigger v1. Parallel with: cards that do not own `media/`, `README.md`, `site/`, `docs/product.md`.
Build folder: `<scratchpad>\p6-agents-video-readme\` (renders, captures, drafts).
Real laptop: screen captures only in Part B, in the screen slot (unlocked session; `p6-media` Log on the lock at
display off). No power settings.

## Goal

Haik, 2026-11-06: "The focus should not be directly on Claude Code. It's ok that the example is on Claude Code in
the video, but the tray will have a new look (AI apps), so these parts should be reworked carefully." The video and
`README.md` say what Wakeshot is now: the PC stays awake while any supported AI agent works (Claude Code, Codex
CLI, Copilot CLI, Copilot in VS Code, OpenCode, Cursor, Antigravity: the ones the live tests passed), plus builds and
Keep awake, then sleeps as usual.

## Read first

- `docs/product.md` (what holds, UX, the README entry), `docs/brand.md`, `docs/research/readme.md`.
- `README.md`, `site/`, `media/README.md`, `docs/tasks/p6-media.md` (rounds 1 to 6, Haik's notes and picks: the
  final picture, the music "Funky Disco Beats" by Fupi, CC0, the time cut) and `docs/tasks/p6-readme.md` (round 5).
- Every `p4-live-*` card's Result of the bigger v1 (which agents passed live, which are "not tested live").
- What the tray shows for each agent (`crates/wakeshot-windows/src/app/tray_mode.rs`, the core's tooltip and menu
  text in `text.rs`).

## Part A: the plan (for Haik's yes)

1. A short list of what changes and why, shot by shot for the video (keep what Haik signed off unless it says
   "Claude Code" where "AI agent" is meant), and section by section for the README and `site/`. Claude Code may stay
   the agent on screen; words, captions and the tray's look must say "any AI agent".
2. Still frames (PNG, in the build folder) of every shot that changes, so Haik can say yes without a render.
3. Stop; the orchestrator sends the plan to Haik.

## Part B: the build (after Haik's yes and the live tests)

4. The video re-rendered with the plan, the music and the time cut kept; every check `p6-media` runs (`check`,
   typecheck, `check-art`, `check-video`, `check-rebuild --final`, loudness at most -1 dBTP).
5. `README.md`: the supported agents as the real list (each one says "tested live" or not), the hooks line from
   `docs/agent-hooks.md` "README lines" (what Wakeshot adds for Cursor and Antigravity, and that uninstall removes
   it), "Codex CLI and Copilot CLI are next" replaced, Claude Code no longer first in the story. Keep Haik's own
   words in "Why I built it" unless the plan says otherwise.
6. `site/`: the same list and words. `docs/product.md`: the README entry (it still describes the old order) and the
   line "nothing that touches other apps" (hooks design review minor 13). The stale hooks lines in
   `docs/research/cursor-gemini-signals.md` (same minor).

## Acceptance checks

- Every check of `p6-media` passes on the new render; the GitHub render of `README.md` checked; every link works.
- No agent is listed as supported unless its live card passed; each "not tested live" is marked.
- No em dash, no emoji, plain ASCII in `media/` text files.

## Out of scope

- The video upload, GitHub Pages going live, the launch (Haik's go). Tray code: if the tray's look must change for
  the video, that is a separate card first.
- Haik's style reference videos never go in the repo or are named in it. Never mention Nightcrew.

## Rules

- Never run git, not even read-only (`git status`, `git diff`, `git --version`).
- Never read token files or any app's secrets. No real prompt text on screen or in captures beyond your own short
  test prompts.
- Kill only processes you started, by PID. Never quit Haik's installed Wakeshot tray.
- Plain short English, no em dashes, no emojis. Write Log lines as you go.

## Questions for Haik

1. "The tray will have a new look (AI apps)": does this mean the tooltip and menu now name whichever agent holds (for
   example "Codex is working"), as the code already does per source, or do you want a new tray part that lists the AI
   apps Wakeshot watches? Default: the first, no new tray part (you dropped the per-source menu at 12:40 today).
   Orchestrator, 2026-11-06 ~18:40: Part A starts with this default while Haik is away (a plan is cheap to redo);
   his answer can change it before Part B.

## Found outside scope

## Log

- 2026-11-06: card written by the orchestrator from Haik's request.

## Result
