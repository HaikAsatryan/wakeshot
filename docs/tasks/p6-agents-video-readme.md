# Video and README for every AI agent, not Claude Code first

Phase: 6. Status: blocked: Part A done 2026-11-06 (the plan in Result waits for Haik's yes); Part B after his yes
and the live tests (new 2026-11-06, Haik). Part A starts when Haik answers the question below; Part B after
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
   Haik, 2026-11-06 ~19:15: "Agree to everything": yes to the Part A plan and to the default (no new tray part).
   Orchestrator: Part B starts now with everything that does not wait for a live result; the agent list, the "tested
   live" marks and the tooltip capture (Claude Code and Codex at once, on the new install) come last, when the
   orchestrator says the live results are in.

## Found outside scope

1. The tooltip names Codex CLI "Codex" and Copilot CLI "Copilot" (`LABEL` in `sources/codex.rs` and
   `sources/copilot.rs`), while log and problem lines say "Codex CLI" and "Copilot CLI" (`source_name` in
   `watch/agents.rs`) and the docs say "Codex CLI". Not a bug; the README will show the tooltip's words. Not changed.
2. The speed numbers in `README.md`, `site/index.html` and the video's end card are from 0.1.0, which watched only
   Claude Code. They go stale once the bigger v1 ships; the plan has them measured again in Part B.

## Log

- 2026-11-06: card written by the orchestrator from Haik's request.
- 2026-11-06: Part A started (worker), with question 1's default: no new tray part. Only the card and the build
  folder are written; no git, no Rust build.
- 2026-11-06: read the card's list: `README.md`, `site/index.html`, `media/README.md`, `media/src/copy.ts` and
  `timeline.ts`, `docs/research/promo-video.md` storyboard, the `p6-media` round 6 and `p6-readme` round 5 Results,
  `docs/product.md` sources and UX, `docs/agent-hooks.md` "README lines", the Phase 4 card table, the Codex live
  Result. Tray labels read in code: "Claude Code", "Codex", "Copilot", "Copilot in VS Code", "OpenCode"
  (`sources/*.rs` `LABEL`); "Cursor", "Cursor CLI", "Antigravity", "Antigravity CLI" (`docs/agent-hooks.md`, not
  built yet). The tooltip joins groups with ", " and fits 127 UTF-16 units, so "Holding awake: Claude Code (1
  session), Codex (1 session)" fits (`text.rs`).
- 2026-11-06: `media/` copied to the build folder (robocopy, without `out/`); only the copy was edited and rendered.
  Draft in the copy: the tray caption "Your agent starts: it turns *green*." and a row of name chips in the light shot
  at beat 7 (frame 105), gone with the whip. `check-pace` on the copy: ok (49 starts on the beat, at most 5 things in
  2 s, longest gap 1.80 s). Stills rendered with the final props (no DRAFT mark) and looked at: frames 108, 135, 148
  (shot 2), 190, 225 (shot 3), the same frames before the change, and frame 135 with only the two agents tested live
  today. Two before and after sheets made. Raw render folders deleted.
- 2026-11-06: the draft edits saved as `draft-video.diff` in the build folder (for Part B); the 848 MB media copy
  deleted. Plan written below. Part A done; the card waits for Haik's yes.

## Result

### Part A: the plan (2026-11-06, for Haik's yes)

Status of Part A: done. Built on question 1's default: the tray gets no new part; the tooltip and the menu's top line
name whichever agent holds, as the code already does. Every agent line below holds **only if its live test
passes**. Today that is Claude Code and Codex CLI; Copilot CLI, Copilot in VS Code, OpenCode 2.x, Cursor (the
editor; the CLI only if its hooks fire) and Antigravity (CLI and IDE) wait for theirs. An agent whose test fails or
does not run is not listed as supported.

Kept as Haik signed off: the final picture of `media/` (f88a034) in every shot not named below, the music "Funky
Disco Beats" (Fupi, CC0), the time cut with "Later," on black, the hook's terminal running `claude` (the example on
screen stays Claude Code, as Haik allowed), and his own words in the README's "Why I built it".

#### The video, shot by shot

| # | Shot | Change | Why |
|---|---|---|---|
| 1 | Hook, 0 to 2 s | none | "Agent working." and "PC sleeping." name no agent |
| 2 | The light, 2 to 5 s | **new:** a row of name chips under "keeps it awake while agents work.", at 3.5 s (beat 7), gone with the whip at 5 s. Plain text in the video's own chip style, A to Z: Antigravity, Claude Code, Codex CLI, Copilot, Cursor, OpenCode. A soft pop sound | this is where the viewer learns what Wakeshot is, so this is where "any AI agent" goes. Names, no logos: logos belong to their owners and need each one's brand rules |
| 3 | Tray, 5 to 10 s | caption "Claude Code starts: it turns **green**." becomes "Your agent starts: it turns **green**." (6 words, so the same timing). The terminal (`claude`) and the real tooltip "Holding awake: Claude Code (1 session)" stay | Claude Code stays the example, as Haik allowed; the words say any agent. Haik's round 2 note (show the agent start and the icon turn green) still holds |
| 4 to 7 | Lid, settings, later it sleeps, menu | none | no agent named; the menu captures show no source list |
| 8 | End card | layout unchanged; the speed numbers and "Measured on Wakeshot (build)" come from a new measurement on the bigger v1 | 0.1.0 watched only Claude Code; the new sources add watching, so the old idle numbers may not hold for the build that ships |

Rules for the chips: only agents whose live test passed. One "Copilot" chip when both Copilot CLI and Copilot in VS
Code pass, else the one that passed under its own name; one "Cursor" chip (editor, plus the CLI if it passes); one
"Antigravity" chip (CLI and IDE). With fewer agents the row stays centred (still with two below). A to Z, so no
agent comes first by our choice. Change it when Haik wants an order by use (Codex led npm downloads, 2026-11-06 log).

The draft in the copy passes `check-pace` (49 starts on the beat, at most 5 things in 2 s, longest gap 1.80 s). Part B
also updates the internal event names in `timeline.ts` ("Claude Code starts" becomes "the agent starts") and the
storyboard rows 2 and 3 in `docs/research/promo-video.md`.

Captures: the tray set (S1 to S4) and the menu set (S1, S5a to S5d) stay, unless a fresh capture from the shipped
build differs (Part B compares one of each). Under the default nothing in them should change.

#### `README.md`, section by section

| Section | Change |
|---|---|
| Top lines | none: "An AI agent at work, a long build..." already names no agent |
| What keeps your PC awake | the "Claude Code" row becomes **AI agents**, with the A to Z list of the agents that passed, and "an agent works on your task, or runs work it started in the background; Wakeshot reads the small files each agent keeps and never changes an agent's settings". Under the table, in the main text (as `docs/agent-hooks.md` asks): the two hook lines for Cursor and Antigravity (the folders, the one notice, uninstall removes them) and the prompts line. "Codex CLI and Copilot CLI are next" becomes "The list grows with each version." |
| How it works | none: the night story already says "an agent's task or a build" |
| What you see | the state table's holding example becomes `Holding awake: Codex (1 session)`; the holding tooltip picture becomes a new capture with two agents at once, "Holding awake: Claude Code (1 session), Codex (1 session)", light and dark (Part B, screen slot); alt texts follow. The loop, the state icons, the idle tooltip and the menu picture stay |
| How it compares | "knows one AI agent today" becomes "knows only the agents listed above". The rows and the Claude desktop app paragraph stay: it answers the question a Claude user asks first, and it is about that app, not about Wakeshot |
| Why I built it | unchanged, Haik's words |
| What comes next | its first line "More reasons to stay awake will come, Codex CLI and Copilot CLI first." becomes "More agents and more reasons to stay awake will come.": both ship now, so the old line turns false. The rest stays |
| When each hold counts | summary "AI agents, `wakeshot run`, hold files and Keep awake, exactly". One short paragraph per agent, A to Z: what holds, what Wakeshot reads (or the hook it adds), "Tested live with (agent) (version)" with a link to its results, and each part not tested live marked so. The Claude Code paragraph keeps its detail. The OpenCode paragraph says the database is opened read only and only its session tables are read |
| What it changes on your PC | the summary and one item add the hook folders for Cursor and Antigravity |
| The menu, notices and start at login | the menu's top line example becomes `Holding awake: Codex (1 session), cargo` (39 characters, the block's width); "Automatic: watch Claude Code..." becomes "watch AI agents..."; the "Update Wakeshot" row says "An agent changed its files"; the hooks notice gets a row with its real title from `text.rs` |
| Commands | none, unless the help text changed (`check_help.py` against `USAGE`) |
| Updates and privacy | the hooks prompt line and the OpenCode read only line; the speed line takes the new measurement |
| Limits | "One AI agent in v1: Claude Code..." becomes "An agent not on the list: `wakeshot run -- <agent>`..."; each agent's own limits from its live card (for example Codex approval waits hold with no limit, Copilot's questions do not hold, OpenCode 1.x is not read); "Not tested live yet" lists every part that was not (the Claude Code extension for VS Code and the Desktop app, the Codex desktop app's own threads, and whatever the live cards add); agents inside WSL |
| Uninstall | the uninstall step and the `.wakeshot-removed` case from `docs/agent-hooks.md` "README lines" |
| Credits and license | re-check: SQLite, if a bundled copy ships with the OpenCode read, gets a line |

#### `site/`

| Place in `site/index.html` | Change |
|---|---|
| `description` and `og:description` | "while Claude Code or a long command works" becomes "while an AI agent or a long command works" |
| Hero speed line | the new numbers; "about 0.1 s after Claude Code starts work" goes, since the delay differs per agent (Codex 0.3 s live; Copilot in VS Code writes its state about once a minute) |
| "What keeps the PC awake" cards | the "Claude Code" card becomes "AI agents" with the same list and words as the README |
| Tooltip line | "Holding awake: Codex (1 session)" |
| "Claude Code is the agent Wakeshot knows in v1..." | "For an agent not on the list, `wakeshot run -- <agent>` holds for as long as it runs." |
| "A night with Wakeshot" | step 1 "You give your AI agent a long task"; step 3 "The agent finishes in the night" |
| "It never needs admin rights and never touches other apps." | "It never needs admin rights and never changes another app's settings. For Cursor and Antigravity it adds a small hook folder of its own; uninstall removes it." |
| Privacy | the hooks prompt line |
| `social-card.html` | none: "while your AI agent works" names no agent |

`docs/product.md` and `docs/research/cursor-gemini-signals.md` follow in Part B as the card says (the README entry,
"nothing that touches other apps").

#### If Haik wants a tray part that lists the AI apps (question 1, not the default)

1. A tray card comes first (design, build, review); this card's Part B waits for it.
2. New captures: the menu set S5a to S5d, and the README menu pictures, light and dark, since the menu changes.
3. Shot 7 (the menu) changes: the menu rises with the new part, and the cursor may rest on it for a beat. That
   beat comes out of shot 7's own 6 s, so the cut stays 30 s; the pace check sets the frames.
4. README "What you see" (menu picture, alt text), the menu block and its item list, and the site's menu words change.
5. The name chips in shot 2 stay: they say it before the menu shows it.

#### Still frames (PNG, 1920 x 1080, final look, no DRAFT mark)

In `C:\Users\haika\AppData\Local\Temp\claude\f7c2c08a8f04f07b\b896b7b1-ede4-4a36-9332-d8c5355d7200\scratchpad\p6-agents-video-readme\stills\`:

- `compare-shot2-light-before-after.png` and `compare-shot3-tray-before-after.png`: before left, after right, half size.
- Shot 2: `shot2-light-before-f135.png`, `shot2-light-after-f135.png` (all six names),
  `shot2-light-after-f108-names-pop-in.png`, `shot2-light-after-f148-last-frame.png` (the frame before the whip),
  `shot2-light-after-f135-if-only-two-pass.png` (Claude Code and Codex CLI only, as if no other live test passed).
- Shot 3: `shot3-tray-before-f225.png`, `shot3-tray-after-f225.png`, `shot3-tray-after-f190-caption-in.png`.

The draft code for Part B: `draft-video.diff` in the same build folder (54 lines: `copy.ts`, `timeline.ts`,
`Promo.tsx`).

#### Part B needs, beyond Haik's yes

1. The live cards' Results, for the final list and each "tested live" line.
2. The speed numbers measured again on the bigger v1, installed on the test laptop, in a window with no agent at
   work (the way the orchestrator ran the 0.1.0 run, `docs/research.md` "Speed numbers").
3. A screen slot for the two-agent tooltip (Claude Code and Codex working at once), light and dark, and the check
   captures of the tray and menu sets.

#### Checks run in Part A

| Check | Outcome |
|---|---|
| `check-pace` on the draft copy | ok: 49 starts on the beat, 10 framings, 104 new things, longest gap 1.80 s, at most 5 things in 2 s |
| Stills looked at | frames 108, 135, 148, 190 and 225 after, 135 and 225 before, 135 with two agents: no overlap, the chips clear of the tagline and the frame edge |
| Nothing written outside the card and the build folder | ok: renders ran in a copy; `media/`, `README.md`, `site/`, `docs/product.md` untouched; no git, no Rust build |
