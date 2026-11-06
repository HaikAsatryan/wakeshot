# Video and README for every AI agent, not Claude Code first

Phase: 6. Status: Part B, last round (README, site, speed numbers, tooltip capture, final render): done
(2026-11-07; "Part B, last round" in Result). Part E (Haik's notes on the Part D renders, 2026-11-06 23:02): done (steps 15 to 17; Haik picked
the current track and likes the video, ~23:50; its final is in `media/out/`; "Part E" in Result). Part B's README
paragraphs, speed numbers and tooltip capture still wait. Part D (Haik's notes on the Part C draft, 2026-11-06 20:45): done (steps 11 to 14); waits for Haik's
music pick from four renders ("Part D" in Result). Part C (Haik's new video scenario, 2026-11-06 18:45): steps 7 to 10 done ("Part C, round 1" and
"round 2" in Result: storyboard, stills, logo rules, the list from the live cards, a draft and every check; `check`
fails only on the two README paragraphs Part B writes); waits for Haik's look at the cut; Part B: blocked: Part A done and approved (Haik, 2026-11-06 ~19:15); Part B steps that need no live
result done 2026-11-06 (draft rendered, checks pass); the rest waits for the live results, the new install and
its speed numbers ("Part B: what waits" in Result) (new 2026-11-06, Haik). Part A starts when Haik answers the question below; Part B after
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

## Part C: a new video scenario (Haik, 2026-11-06 18:45)

Haik's words, as he wrote them:

> Let's change our video scenario.
>
> As we have multiple agent from start we can dynamically with logos show agents examples
>
> We can show how one of agents working and laptop sleeps and tell tiered of sleepy laptops and start our logo and
> etc.
>
> This is idea but you should professionally design my requirement and redesign video.

He wants it designed by us, "not strictly as I told, inspire by my imagination". So the scenario is yours to design
well; his idea is the brief, not a script. This replaces the storyboard he approved earlier; keep the brand
(`docs/brand.md`) and the music ("Funky Disco Beats", Fupi, CC0); the length and the time cut may change.

7. A new storyboard: the pain first (an AI agent works, the laptop goes to sleep, the work stops; a line in the
   spirit of "Tired of sleepy laptops?"), then Wakeshot (logo), then the many agents it works with, shown with their
   logos and moving, then the PC sleeping as usual once the work ends, then the end card. Write it in
   `docs/research/promo-video.md` (you own the storyboard rows and the counts line), with still frames (PNG, build
   folder) of every shot. Then stop and reply with the stills' folder; the orchestrator shows them to Haik and resumes
   you at once (no wait for his answer).
8. Agent logos: before any logo goes in, read each owner's official brand or press page for logo use (Anthropic for
   Claude Code, OpenAI for Codex, GitHub for Copilot, Anysphere for Cursor, Google for Antigravity, the OpenCode
   project). Use only official files, unchanged, as their rules allow, and only to show "works with", never as an
   endorsement. Where the rules forbid it or are unclear, show the name as text in Wakeshot's style. Write each source
   and rule in `docs/research/promo-video.md`. The repo is public: a logo file in `media/` is published at the next
   push.
9. The agent list stays one data list (`media/src/agents.ts`); the final list comes from the live results (Cursor
   pending; Antigravity may leave v1, `p4-antigravity-exact-end`), so an agent can drop out with no redesign.
10. Render a draft and run every `p6-media` check. The tooltip capture and the speed numbers still wait for the new
    install (Part B).

## Part D: Haik's notes on the Part C draft (2026-11-06 20:45)

Haik's words, as he wrote them:

> First sleep scene is too short, it's ok if we pass 30 second threshold, just make it more professional, and at the
> point of Tired Sleepy laptops we see behind it the regular frame. + music integration is little weird because it
> kind of becomes more silent at some points. Also I like this sound and the type of this sound and want to see
> alternative options as well (not older ones) that are around the same type and can have little bass as well. But
> entertaining.

11. The opening sleep scene longer and more polished: the agent at work long enough to read, then the laptop going to
    sleep in a way that feels real (the screen dims, the work stops), then "Tired of sleepy laptops?". The video may
    pass 30 s.
12. At "Tired of sleepy laptops?", the frame behind the line: look at the render frame by frame. Haik sees the regular
    frame behind the line, which reads as a glitch. Make the background behind the question clean and deliberate.
13. The music level: no dips Haik can hear. The mix ducks the music under each chime (4 dB) and keeps it sparse in
    the hook; even it out so the music feels continuous, and still keep the loudness checks.
14. Music options: he likes "Funky Disco Beats" (Fupi, CC0) and its type. Find 3 new tracks of the same type (funky,
    disco, upbeat, entertaining), with a bit more bass, none of the tracks offered in earlier rounds (`p6-media` Log).
    Each one free for this use: CC0 or a license that allows a public repo and video uploads with no fee; write the
    license and source of each in `docs/research/promo-video.md`. Render the new cut once per track (the current one
    plus the 3 new), each with every check passing, so Haik picks in context.

## Part E: Haik's notes on the Part D renders (2026-11-06 23:02)

Haik's words, as he wrote them:

> Video change request again: First music is not required and the visual can be different. Imagine terminal is open
> and someone is typing something and we only here that typing sounds and sleeping process and we start music when
> starting to introducing our app. Usually agents in terminal are not typing so fast, usually they loading a lot. Try
> in this way.

He has not picked a track yet.

15. The opening, redesigned: a terminal is open, someone types a prompt to an agent (keyboard sounds only, no music),
    then the agent works the way real agents do: mostly waiting and loading (a spinner, a thinking line, a tool line
    now and then), not fast typing. Then the laptop goes to sleep (its own sound, the screen dims and goes dark), then
    "Tired of sleepy laptops?". The opening visual may change from Part D's.
16. The music starts when Wakeshot is introduced (the light comes on), not before. Keyboard and sleep sounds must be
    original or free for this use (CC0 or a license that allows a public repo and video uploads with no fee); record
    each source and license.
17. Render the new cut once per track (the 4 of Part D), each passing every check, for Haik's pick in context.

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
3. `docs/research/promo-video.md` storyboard rows 2 and 3 still describe shot 2 without the name chips and shot 3's
   caption as "Claude Code starts". That file is not owned by this card's round 1, so not changed; it needs the two
   rows (and the counts line) when the orchestrator hands it over. Done in Part C (the whole storyboard is new).
4. Part C: the Antigravity terms say "You must not abuse, harm, interfere with, or disrupt the Service. This
   includes, but is not limited to, using the Service in connection with products not provided by us." Their example
   is reaching the service with its sign-in, which Wakeshot does not do. Not a legal reading; for Haik before a public
   "Antigravity" claim (`promo-video.md`, "Agent logos").
5. Part C: `agents.ts` still lists only Claude Code and Codex CLI as passed, while the orchestrator reports Copilot CLI
   and OpenCode 2.x passed live too. Flipping them runs `npm run agent-lists`, which writes `README.md` and
   `site/index.html` (not owned this round), and `npm run check` then asks for their README paragraphs. Not changed.
   Round 2: flipped; the lists written; the two paragraphs still missing (Part B step 5), so `check` fails there.
6. Part C: the five menu captures (`media/public/shots/menu-S1-dark.png`, `menu-S5a` to `S5d`), their credits and the
   `menu` set in `data/shots.json` are no longer in the video. Not deleted (not made by this card); keep them for a
   longer cut or delete them, the orchestrator's call.
   Round 2: kept, since files refer to them (orchestrator's rule): `media/data/shots.json` (set `menu`),
   `media/CREDITS.md` (five rows, and the grain note names `menu-S1-dark.png`), `media/src/promo/Capture.tsx` (the
   `menu` set's sketch), `media/src/promo/world.ts` (`SET_AT.menu`), `media/README.md`, `promo-video.md` (captures
   table); `build.mjs` refuses a final while any listed file is missing.
7. Part D: the code bed (the fallback music in `make-sounds.mjs`, used only when `data/music.json` names no file)
   still has the old sparse hook and a silence before the light; the evenness rule skips it. Not changed: no build
   uses it.
8. Part D: the committed `media/out/wakeshot-promo.mp4` is the round 6 final (30.02 s), so `check-video` in final mode
   now fails its new length rule (34 s) until the picked track's final is rendered into `media/out/`. Part E: done;
   the 38 s final with Haik's pick is in `media/out/` and `check-video` passes.
9. Part E: the helper read BigSoundBank's pages, which carry text addressed to AI readers (asking to be quoted
   exactly). Treated as data and ignored; no BigSoundBank file was used.
10. Part E: the orchestrator's note said the final list gives "5 tiles". By the chip rule Haik approved in Part A, two
    passed Copilots share one "Copilot" tile, so the video shows 4 (Claude Code, Codex CLI, Copilot, OpenCode); the
    README and site lists name all 5. Five tiles: delete the two `chip` fields in `agents.ts` (the tile layout takes 5
    as 3 and 2; "Copilot in VS Code" steps down to 40 px). Not changed.
11. Part E: `agent-lists --check` (in `npm run check`) now names three missing README paragraphs: **Copilot CLI.**,
    **Copilot in VS Code.**, **OpenCode.** (Part B step 5; README text is not owned this round). Part B last round:
    written; the check passes.
12. Part B last round: idle CPU rose from 109 to 1,016 ms in 10 minutes. Causes and the two places in
    `crates/wakeshot-windows/src/watch/agents.rs` in Result ("What costs the CPU at rest"). Not changed (no Rust here).
13. Part B last round: the installed tray's private memory rose from 3.98 to 7.69 MB while it held for a busy Claude
    Code session (about 1.5 hours). Not investigated.
14. `docs/product.md` line "A 30 second promo video" (Distribution): the video is 38 s now. Not this card's line.
15. Tooling: Remotion's bundled `ffmpeg.exe` has no `framemd5`, `md5` or `rawvideo` muxer and no `select` filter; frame
    hashes go through `image2pipe` with the `rawvideo` codec (`compare-final.mjs` in the build folder).

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
- 2026-11-06 ~19:15: Haik, via the orchestrator: "Agree to everything" (the plan and question 1's default). Part B
  started with what needs no live result. Owned now: `media/`, `README.md`, `site/`, the README entry and the
  "other apps" line of `docs/product.md`, the hooks lines of `docs/research/cursor-gemini-signals.md`.
- 2026-11-06: one list of agents: `media/src/agents.ts` (all seven, each `passed` or `pending`; only `passed` ones
  show). `scripts/agent-lists.mjs` writes the list between `<!-- agents -->` markers in `README.md` and
  `site/index.html`; `--check` (now in `npm run check`) fails on a differing list or a README rules paragraph that
  is missing or extra. Proven: with OpenCode flipped to `passed` it failed 3 ways, restored it passed. Chip rule
  checked: both Copilots passed gives one "Copilot" chip, one passed gives "Copilot CLI".
- 2026-11-06: video: chips (from `agents.ts`) and a soft pop at beat 7, the tray caption "Your agent starts", the
  event names. README, site, `product.md` and the research doc edited (Result). `check` (89 files; pace 49 on the
  beat; lists agree), typecheck, `check-art` (9,458,740 pixels) ok. `npm run draft` (2 min): `check-video --draft`
  ok, 7.42 MB; loudness after AAC -15.3 LUFS, -1.1 dBTP. Draft frames 135 and 225 looked at (two chips; the new
  caption). GitHub render of the README (`gh api markdown`): the agents cell reads "Claude Code and Codex CLI", no
  comment text shows, 5 tables. `site/check_page.py`: 0 errors, 7 spots wait (unchanged).
- 2026-11-06: `check-rebuild` (loop, states, draft) ok: 11 outputs the same bytes. Round 1 Result written. Stopped
  here: the rest waits ("Part B: what waits"). The orchestrator resumes this card when the live results are in.
- 2026-11-06: Part C round 1 started (worker): steps 7 and 8 only, then stop with the stills. Owned this round:
  `media/`, `docs/research/promo-video.md`, this card, the build folder. Not `README.md`, `site/`, `docs/product.md`
  or Rust. No screen captures. Agent list today: live passed Claude Code, Codex CLI, Copilot CLI, OpenCode 2.x;
  Copilot in VS Code partly; Cursor testing; Antigravity may leave v1. A helper agent reads each owner's logo rules
  (step 8) into the build folder's `logo-rules.md`.
- 2026-11-06: read `media/` (timeline, copy, agents, Promo, world, laptop, terminal, pace check), the storyboard, the
  `p6-media` round 2 to 6 notes. Design picked: 30 s and 8 shots kept, the same beat grid and music. New: the hook is
  the drawn laptop asleep mid-work, then "Tired of sleepy laptops?"; the light and the name at 4 s; a new agents shot
  (a grid of agent tiles, one per supported agent, from `agents.ts`), the light above it; then the tray proof, lid,
  settings, sleep and end card as signed off, 180 frames later. Out: the menu shot (its 6 s pay for the hook and the
  agents shot). Built in `media/` directly (owned this round), stills rendered from the real composition.
- 2026-11-06: built: `timeline.ts` (new scenes, moments, captions, events, sounds, music marks), `copy.ts`,
  `agents.ts` (`tiles()`, `previewTiles()`), `Promo.tsx` (pain laptop, the light rising to the top, agent tiles, the
  whip from there; menu shot code out), `world.ts`, `LaptopTilt.tsx` (`stopAt`, `screen`), `check-pace.mjs` (name by
  5 s), `build.mjs` (poster frame 180). Typecheck and `npm run check` ok. Test frames looked at and the pain laptop,
  the tile band and the sleeping chip moved twice.
- 2026-11-06: logo rules back from the helper (every page read, quotes exact): Anthropic and Google need approval,
  OpenAI, Cursor and OpenCode unclear, GitHub allows with a written-permission line in the same page. Decision: names
  only for every tile, no logo file in `media/`; caption "Stays awake for these agents." (not "works with", for
  OpenAI's non-partner rule); a trademark line in the end card's small print (Google's legal line). The `mark` code
  taken out again. `promo-video.md`: storyboard rows, counts line, name rule, framings, captures table, risk 6, new
  "Agent logos" section. `media/README.md`: poster frame and the agents line.
- 2026-11-06: stills rendered with the final look (no DRAFT), 21 frames plus a whole-cut sheet; looked at: 0, 50,
  100, 180, 218, 280, 292 (two, four and six tiles), 336, 850 full size, the rest on the sheet. Stopped here for Haik's
  look; steps 9 and 10 next.
- 2026-11-06: Part C round 2 started (worker): steps 9 and 10, the agent list from the live cards, the unused menu
  captures, names only (no logos), a draft render and every `p6-media` check. Haik has not answered on the contact
  sheet yet, so the storyboard stays as it is. Owned: `media/`, `docs/research/promo-video.md`, this card, the build
  folder, and in `README.md` and `site/` only the generated agent lists. No git, no Rust, no screen captures.
- 2026-11-06: live cards read: Claude Code, Codex CLI, Copilot CLI (1.0.106), OpenCode (2.2.5) done and passed;
  Copilot in VS Code done but no hold for a Local working chat seen (waiting and idle only), so `partly` (a new third
  value, counted as not passed); Cursor in progress; Antigravity's exact-end card in progress. `agents.ts` set so;
  `npm run agent-lists` wrote "Claude Code, Codex CLI, Copilot CLI and OpenCode" into `README.md` line 65 and
  `site/index.html` line 57, nothing else. It then fails, as expected, on the two README rules paragraphs
  (**Copilot CLI.**, **OpenCode.**): those are Part B step 5, not this round's README scope.
- 2026-11-06: the five menu captures are still referred to (`data/shots.json` set `menu`, `CREDITS.md` rows and the
  grain note, `src/promo/Capture.tsx` sketch, `src/promo/world.ts` `SET_AT.menu`, `media/README.md`,
  `promo-video.md` captures table), and `build.mjs` checks every set's files before a final: kept.
- 2026-11-06: no logo file or logo code in `media/` (checked by find and grep); the end card's trademark line is in
  (`copy.ts` `trademarks`). `check.mjs` ok (89 files), `check-pace` ok (44 on the beat, 83 new things), typecheck ok,
  `check-art` ok (9,458,740 pixels). `npm run draft` (2 min) ok, `check-video --draft` ok (30.02 s, 7.49 MB).
  Loudness after AAC: -15.25 LUFS, -1.01 dBTP: inside the limit by 0.01 dB, so one more tile tick could cross it.
  The mix's sample ceiling in `make-sounds.mjs` went from -2 to -2.5 dBFS (limiter now -2.2 dB at the loudest spot);
  the draft again: -15.26 LUFS, -1.62 dBTP. `media/README.md` (also "the light at 2 s", now 4 s) and
  `promo-video.md` say so.
- 2026-11-06: draft frames 45, 180, 292, 360, 850 looked at (four tiles: Claude Code, Codex CLI, Copilot CLI,
  OpenCode; the trademark line on the end card). For `check-rebuild --final` the final must sit in `media/out/`:
  the signed-off `out/wakeshot-promo.mp4` (sha256 c35177c7...) and poster (5abbdc2b...) backed up to the build
  folder's `out-backup\`; they go back after the check. If this run is cut off, copy them back over `media/out/`.
- 2026-11-06: `npm run video` (final mode) ok, `check-video` ok (7.37 MB, 30.02 s), -1.62 dBTP. `check-rebuild --final`
  ok: 15 outputs the same bytes. The new final copied to the build folder's `final-check\`; the round 6 final and
  poster put back in `media/out/` (sha256 c35177c7... and 5abbdc2b... again); `out-backup\` deleted. `site/check_page.py`
  0 errors; `gh api markdown` render of `README.md`: the new list, 5 tables, no marker text. Result written.
- 2026-11-06: Part D started (worker): steps 11 to 14, Haik's notes on the Part C draft. Owned: `media/`,
  `docs/research/promo-video.md`, this card, the build folder. Not `README.md`, `site/`, `docs/product.md`, Rust. No
  git, no screen captures. Kept: names only, the trademark line, `agents.ts` as it is (4 passed).
- 2026-11-06: the Part C hook rendered frame by frame (0 to 120). Found: the agent works for only 1 s (tool lines
  from frame 3 to 30); the "asleep" screen still shows the terminal at 25%; at frame 60 the hook words cut and
  "Tired" comes in over the laptop at full light, then a black veil fades to 86% over 12 frames, so the laptop and
  the "Sleeping" chip stay as a ghost at 14% behind the question to the light. That ghost is the "regular frame"
  Haik saw. A helper agent searches for new CC0 funky disco tracks into the build folder's `music-raw\` (step 14).
- 2026-11-06: built (steps 11 to 13): the pain is 8 s (16 beats, so the music's lift stays on a bar line), the video
  34 s, everything after the pain 8 beats later. Close on the screen 0 to 3 s (terminal text about 30 px), pull back,
  the screen dims at 3.5 s while the agent still works, goes off at 4.5 s (work stops, power light breathes, chip),
  the scene sinks into the plain stage at 5.5 s, the question alone from 6 s, the light at 8 s. The 86% veil is gone.
  Music: one level from frame 0 (a slow leveler evens a track to its own median), no silence before the light, no
  5 dB lid dip; under each chime only the chime's band is cut (6 dB bell at 1.5 kHz) instead of 4 dB of the whole
  music; when the laptop sleeps the track goes on behind a low-pass (500 Hz) and opens over the beat before the light.
  `make-sounds.mjs` now stops when the music's 3 s loudness leaves its median by more than 2 LU (6 in the muffle).
  Funky Disco Beats: -0.5 to +0.5 LU outside the muffle, -2.2 LU in it (was: the hook at -6 dB, silence, -9 dB under
  the lid chime). Limiter 2.7 dB after the thud 0.5 to 0.35 and the lid chime 1 to 0.85 (the music no longer ducks
  there). Pace check ok with the name rule moved from 5 s to 9 s; `check-video` takes the timeline's length (45 s at
  most); poster frame 180 to 300.
- 2026-11-06: draft with the current track: 34.01 s, 8.48 MB; -15.25 LUFS, -1.47 dBTP, LRA 5.3 LU (the round 6 final:
  LRA 6.6). The leveler then leaves the muffle stretch as the track plays it (a track's own break before its lift
  belongs under the question). The helper's 10 tracks (9 CC0, 1 CC BY 4.0) measured by script, not heard: tempo
  (whole, low and high bands), key, band shares by FFT. Finding: Funky Disco Beats puts 78% of its energy in 40 to
  100 Hz and only 11% in 100 to 250 Hz, the bass small speakers play; every new track has more there. Picked (all
  CC0, licence lines read again by curl on each page): "Can't Stop My Feet !" (Loyalty Freak Music, 130 BPM, A minor,
  26% in 100 to 250 Hz), "Lost In The Rythm" (HoliznaCC0, 105 BPM, A minor, 20%, the most bass over mids),
  "Funked Up" (joth, 116 BPM, A minor, 45%). Lifts set by hand on bar lines in steady parts (59.07, 27.43, 16.55 s).
  Fitted and mixed: each within 0.5 LU of its median outside the muffle.
- 2026-11-06: renders, one track at a time (`candidate.sh` in the build folder): each sets the track in
  `public/music/`, `data/music.json` and a temporary `CREDITS.md` line, runs check, typecheck, check-art, draft,
  video, check-video (both), loudness, check-rebuild --final, copies the final out, then puts the three files back.
  The round 6 final and poster are backed up in the build folder's `out-backup\` (sha256 c35177c7... and
  5abbdc2b...), `music.json` and `CREDITS.md` in `repo-backup\`. If this run is cut off: copy those back.
- 2026-11-06 21:12: "Can't Stop My Feet !": every check ok except `check-rebuild --final`: the final video differed
  (the draft, the soundtrack and the poster were the same bytes). The pain rendered twice (frames 0 to 245, decoded
  pixels): all equal, so the cause is not the new hook. `candidate.sh` now keeps the rebuild copy on a mismatch and
  names the first video packets that differ. 21:24: "Lost In The Rythm": every check ok, `check-rebuild --final` ok.
  "Can't Stop My Feet !" runs again at the end.
- 2026-11-06 21:33: "Funked Up": every check ok. 21:41: "Can't Stop My Feet !" again: every check ok, and its final is
  the same bytes as the first run's, so the first mismatch was the clean copy's render, once. 21:49: "Funky Disco
  Beats": every check ok. Contact sheet of 0 to 8.5 s rendered (35 frames, final look) and looked at: close shot,
  pull back, dim, off, sink, the question alone on the plain stage, the light. The round 6 final and poster put back
  in `media/out/` (sha256 c35177c7... and 5abbdc2b... again); `public/music/`, `music.json` and `CREDITS.md` are as
  before the runs (compared byte for byte); `out/draft/` holds the new cut with the current track. Test frames, fitted
  WAVs and the backups deleted from the build folder.
- 2026-11-06: Part E round 1 started (worker): steps 15 and 16, the design, the code, the sounds, stills of the new
  opening and a contact sheet, the checks that need no full render. No full render this round (an install check times
  live agents on this laptop). Owned: `media/`, `docs/research/promo-video.md`, this card, the build folder. Kept:
  names only, the trademark line, `agents.ts` as it is. No git, no screen captures.
- 2026-11-06: read the Part D code, `promo-video.md` and the Part D opening sheet. Design picked: the opening is 12 s
  (24 beats, was 8 s), the light at 12 s, everything after it 8 beats later, the video 38 s. 0 to 3 s close on a flat
  terminal window: the input box, a person types "migrate billing to API v2" at about 11 keys a second; Enter at 3 s;
  3 to 6.5 s the agent works the way agents do: a spinner, "Thinking" with a shimmer, a clock that starts in real time
  and then runs as a time lapse into minutes, a tool line now and then; 6.5 s a cut to the laptop with the same screen;
  7.5 s the idle timer dims it; 8.5 s it sleeps (screen off, power light breathes, "Sleeping" chip); 9.5 s it sinks;
  10 s "Tired of sleepy laptops?" in silence; 12 s the light, the music starts there. Sound before the light: keys,
  a faint room tone and fan, the power-down and the fan winding down, a riser into the light. A helper agent looks for
  CC0 typing recordings into the build folder's `sound-raw\` (code-made keys are the fallback).
- 2026-11-06 23:15: the orchestrator: the install check is done, so round 2 (the four renders) follows at once.
- 2026-11-06: built: `timeline.ts` (38 s, `KEYS` and `SHIFT_AT`, the new pain moments and places, no words before the
  question, new sounds, `MUSIC` without the muffle), `promo/AgentTerminal.tsx` (new: the input box, typing, the
  thinking line with a shimmer and the time-lapse clock, tool lines), `LaptopTilt.tsx` (`content`), `Promo.tsx`,
  `world.ts`, `copy.ts`, `lib/fit-music.mjs` (silent before the light, the leveler counts only the music's own
  windows, the muffle out), `make-sounds.mjs` (keys, room, fan, power down, riser; the evenness rule from the light;
  the code bed silent before the light), `check-pace.mjs` (name by 13 s; frame 0 may open on the terminal),
  `build.mjs` (poster frame 420). Typecheck, `check.mjs` and `check-pace` ok.
- 2026-11-06: the helper's report: three CC0 or public domain typing recordings downloaded; picked "Keyboard Soundpack
  #1" by unicaegames (OpenGameArt, CC0, a Cherry KC 1000 low-profile keyboard, 32 single keys). 18 takes copied
  unchanged into `media/public/keys/` (not `public/sound/`, which git ignores), credited in `CREDITS.md`. Levels by
  script, not heard: typing at about -25 LUFS (3 s), 9 to 10 LU under the music; room and fan -36; the power down
  about -29; keys peak about -9 dBFS.
- 2026-11-06: stills (final look) and a 51-frame contact sheet of 0 to 12.5 s rendered and looked at; the laptop
  moved to the frame's middle (no words above it now), the clock put in brackets.
- 2026-11-06 23:21 (orchestrator): the v1 list is final. `agents.ts`: Copilot in VS Code `passed`, Cursor and
  Antigravity lines deleted (their task lines in `copy.ts` too); `npm run agent-lists` wrote "Claude Code, Codex CLI,
  Copilot CLI, Copilot in VS Code and OpenCode" into `README.md` line 65 and `site/index.html` line 57 only. The
  video shows 4 tiles, not 5: both Copilots passed, so they share one "Copilot" tile (the rule Haik approved in Part
  A). `agent-lists --check` now names three missing README paragraphs (Part B).
- 2026-11-06: renders started (`run-all-e.sh` in the build folder: tracks 0 to 3, one at a time, every check, then the
  round 6 final and poster back into `media/out/`). Backups: `out-backup\` (sha256 c35177c7... and 5abbdc2b...),
  `repo-backup\` (`music.json`, `CREDITS.md` with the keys rows). If cut off: copy those back.
- 2026-11-06 23:31: track 0 (Funky Disco Beats): every check ok, `check-rebuild --final` ok; 38.02 s, 8.50 MB,
  -15.32 LUFS, -1.47 dBTP. Frames 1.5, 5.4 and 12.4 s taken from the MP4 and looked at. Docs written while the
  others run: `promo-video.md` (pace and text rows, Sound, storyboard 38 s with row 1 new and rows 2 to 8 four seconds
  later, counts, Music and effects, Formats, the Antigravity note), `media/README.md`, `docs/research.md` (a new
  section on the keystrokes).
- 2026-11-06 23:39 and 23:48: tracks 1 and 2: every check ok, `check-rebuild --final` ok (-1.84 and -1.60 dBTP).
- 2026-11-06 ~23:50 (orchestrator): Haik: "ok our music was the best. I already checked it. + I like the video." So
  track 0 stays. Track 3's run (in its draft render) stopped: the task, then its shells, node, Chrome and ffmpeg by PID
  (`taskkill /T`, only processes this card started). Put back: `public/music/funked-up.mp3` removed (only
  `funky-disco-beats.ogg` left), `music.json` and `CREDITS.md` from the backup (sha256 equal). Track 0's final and
  poster copied into `media/out/` (the bytes `npm run video` makes now; its run passed `check-rebuild --final`). The
  draft built again with track 0 (`check-video --draft` ok, 9.10 MB); `check-video` on the final ok. `check.mjs`,
  `check-pace`, typecheck ok; `agent-lists --check` fails only on the three README paragraphs (Part B). Backups
  deleted. `promo-video.md`: the pick, options 1 to 3 "offered, not picked", the Part E size and loudness.
- 2026-11-06: Part B last round started (worker): items 4 to 6 and "Part B: what waits", on the final v1 list
  (Claude Code, Codex CLI, Copilot CLI, Copilot in VS Code, OpenCode 2.x; Cursor, Antigravity and VS Code Local chats
  out; no hook folders in v1). Order: README and site, speed numbers on the installed final build (p4-cursor-off),
  the two-agent tooltip capture (stop for Haik), then the final render and every check. Owned: `README.md`, `site/`,
  `media/`, `docs/research/promo-video.md`, the README entry and the "other apps" line of `docs/product.md`, this
  card, `docs/research.md` (additions). No git, no Rust.
- 2026-11-07: README: "Each one was tested live." in the agents row; a line under the table (only agents whose end is
  seen exactly; Cursor, Antigravity and VS Code Local chats not yet, link to Limits); the **Copilot CLI.**,
  **Copilot in VS Code.** and **OpenCode.** paragraphs (from the live cards and research sections; anchors checked);
  Limits: Copilot waits, OpenCode approval waits, the four folder variables, "Not supported, and why" (the
  `agent-hooks.md` lines plus VS Code Local chats from `p4-vscode-local-off`), WSL for all agents, OpenCode 1.x, the
  parts not tested live; Privacy: the OpenCode read only line; the video comment no longer says 30 seconds. No hook
  lines (v1 adds none). Site: "each one tested live", a "Not supported yet" line, the privacy line. `product.md`:
  the README entry and the "other apps" line. `agent-lists --check` ok (5 of 5). GitHub render (`gh api markdown`):
  5 tables, 9 folds, no comment text; every in-repo link and anchor resolves (slugs made from the source, since the
  API drops ids); external links all 200.
- 2026-11-07 ~00:05 (orchestrator): Haik: "video approved" (the Part E render, 38 s). The final render changes
  nothing but the end card's speed numbers, and only if the new measurement differs; same numbers: the approved MP4
  stays, no re-render.
- 2026-11-07 00:06 to 00:21, speed numbers on the installed final build (0.1.0 of `p4-cursor-off`, Setup
  12,481,433 bytes). Installed tray 56164 held for the orchestrator's working Claude Code session the whole time
  (`Claude Code (1 session)` in all 60 reads of `state.json`), with guard 41988. So, beside it, the same installed
  exe as `--dry-run` at Medium (pid 9144, through `explorer.exe`) with `CLAUDE_CONFIG_DIR` on an empty scratch
  folder, the other agents' homes real: its log has no hold in the 10 minutes. Sampled together, 60 samples 10 s
  apart, 00:07:42 to 00:17:44: installed tray private median 3.98 MB (max 4.95), working set 22.48, CPU 1,140.625 ms
  (0.19% of one core); guard private 1.80 MB, working set 12.61, CPU 0 ms; dry-run tray private median 2.66 MB (max
  2.71), working set 16.80, CPU 1,140.625 ms. Then 5 `codex exec` one-word test turns at Medium: start delay (the
  rollout's `task_started` to "Holding starts: Codex") 284, 116, 47, 10, 306 ms in the dry-run tray, the installed
  tray's "Hold starts: Codex" within 32 ms of the same; end delay 9 to 12 ms (one 52). Dry-run tray quit by WM_CLOSE
  to its `Wakeshot.Loop.DryRun` window, code 0; Haik's tray untouched. Power settings read before (the tray's hold
  values; nothing written by me). The installed tray idle is still to be measured: a detached watcher (pid 31796,
  `speed\idle-watch.ps1`, reads only, up to 4 h) samples it 10 minutes once it holds nothing for 60 s.
- 2026-11-07 00:22: tooltip capture scripts ready (`tip\session.ps1`, `lib.ps1`, `backdrop.ps1`, from
  `p6-media-captures`' method). The Wakeshot icon is in the `^` overflow since the reinstall, and a registry write of
  `IsPromoted` is not picked up live (tested, put back), so Haik drags it to the taskbar. Stopping for Haik's steps.
- 2026-11-07 00:24 to 00:34 (the watcher, while the orchestrator's session was idle): the installed tray idle, 59
  samples, no hold line in its log: private median 4.99 MB (max 7.29), working set 23.44 MB, CPU 1,015.625 ms
  (0.17% of one core). Watcher exited by itself.
- 2026-11-07 ~01:14 (orchestrator): Haik did the steps (unlocked, icon on the taskbar, hands off 6 minutes).
- 2026-11-07 01:15 to 01:19, the capture (`tip\session.ps1`). Run 1: one `codex exec` test turn at Medium
  (`Start-Sleep -Seconds 240`, answered "0", exit 0 at 01:19:19); the tray held "Claude Code (1 session); Codex (1
  session)" 2 s later; the DELL to 200%; the dark grab was right, then the script stopped: its tooltip variable
  `$tip` is the same name as the folder variable `$TIP` (PowerShell names ignore case), so the log path broke. Its
  `finally` still put the theme and scale back (checked: 0/0, 150%; no backdrop left). Runs 2 and 3 got no tooltip:
  a cursor jump leaves the taskbar's hover on the icon (the base grab shows the icon highlighted), so a new hover is
  no new entry. Fixed with small injected moves after each jump. Run 4: light ok at 01:17:55. Haik then said he had
  moved the mouse (Telegram in the tray): run 1's dark grab still has Telegram's icon, the light one not, so dark
  again, run 5: ok at 01:18:39. Each kept grab passed the pointer check (the cursor within 2 px of where the script
  put it) and shows "Holding awake: Claude Code (1 session), Codex (1 session)". After every run: theme 0/0 and DELL
  150% read back (the laptop screen stayed 125%); no backdrop process left; no headless Claude Code started (the
  orchestrator's session held). The Wakeshot icon stays on the taskbar where Haik put it.
- 2026-11-07 01:20: cut (`tip\cut.ps1`): 636 x 274 from screen 2932,1886, the same rect for both; TrafficMonitor and
  the battery button's edge covered with plain taskbar. Looked at both. Copied over
  `media/public/shots/readme-holding-tooltip-{dark,light}.png`; `data/shots.json`, `CREDITS.md` and the README's
  alt text and size (318 x 137) follow.
- 2026-11-07 01:21: A/B for the idle CPU: a dry-run tray again (pid 42964), now also `XDG_DATA_HOME` on the empty
  folder, so it has no OpenCode database to poll; 10 minutes.
- 2026-11-07 01:21 to 01:44, what costs the CPU at rest (dry-run trays at Medium, each quit by WM_CLOSE, code 0):
  Codex and Copilot real 890.625 ms in 10 min; all four folders empty 15.625 ms in 5 min; Copilot alone 718.75 ms in
  5 min. A 60 s count of change notices in the four agent homes: none. One Restart Manager question (`rm-cost.ps1`,
  20 each): 14 to 22 ms CPU for a lock file nobody holds, about 135 ms for one `copilot-runtime.exe` holds. Code:
  `Agents::on_tick` drops the cached answer of every unheld lock file at each 30 s tick, and this PC has two dead
  Copilot sessions' `inuse.<pid>.hold`. Written up in `docs/research.md`, "Speed numbers, final v1 build
  (2026-11-07)", with the installed tray's memory rising to 7.69 MB while holding (not investigated).
- 2026-11-07 01:45: `data/speed.json` (5.0 MB, 1016 ms, 0.116 s, every raw value), `media/README.md` speed row, the
  README Privacy line and the site hero (5 MB, 1 s in 10 minutes, link to the new section). `npm run check` ok.
  README render and links again: 0 bad links (the new anchor too); `site/check_page.py`: 0 errors, 7 waits.
- 2026-11-07 01:47: approved MP4 and poster backed up (`approved\`, sha256 123d674c... and b6f7302b...); typecheck,
  `check-art` and `npm run video` started.
- 2026-11-07 01:50: typecheck ok, `check-art` ok (9,458,740 pixels), `npm run video` ok. `check-video` ok. New
  final 8,497,375 bytes, 38.016 s; loudness -15.32 LUFS, -1.47 dBTP, LRA 16.2 LU. Against the approved file, every
  frame decoded and hashed: frames 0 to 1019 the same bytes, 1020 to 1139 (the end card) differ; the decoded sound the
  same bytes. Frame 1030 differs by at most 3 (encoder), frame 1100 only in the speed and measured lines (looked
  at). The poster is the same bytes as before (b6f7302b...). `npm run draft`, `check-video --draft` and
  `check-rebuild --final` started (the draft also shows the speed line, so it must be rebuilt for the rebuild check).
- 2026-11-07: `promo-video.md` Formats: Haik's approval and the new final's numbers.
- 2026-11-07 01:58: `npm run draft` ok (9.10 MB), `check-video --draft` ok, `check-rebuild --final` ok (every output the
  same bytes). Read again at 02:00: theme 0/0 (dark); the session is locked now and only the laptop screen is active, so
  the DELL is not listed; its stored scale is `PerMonitorSettings` `DpiValue` 0, the recommended step, which read as
  150% after every run (Windows writes that key at a scale change, as `p6-media-captures` saw; whether it existed
  before this run was not checked). Result written.

## Result

### Part B, last round (2026-11-07): README, site, speed numbers, tooltip capture, final render

Status: done (check-rebuild: see Checks). The v1 list is final: Claude Code, Codex CLI, Copilot CLI, Copilot in VS
Code (Copilot harness chats), OpenCode 2.x, each tested live; no hook lines (v1 adds none).

#### README sections changed

- **What keeps your PC awake**: "Each one was tested live." in the agents row; a new line under the table: Wakeshot
  supports an agent only where it sees exactly when the work ends, so Cursor, Antigravity and VS Code's own Local
  chats are not on the list yet, with a link to Limits.
- **What you see**: the holding tooltip picture is new, two agents at once ("Holding awake: Claude Code (1 session),
  Codex (1 session)"), light and dark, 318 x 137; alt text says two agents.
- **When each hold counts**: new **Copilot CLI.**, **Copilot in VS Code.** and **OpenCode.** paragraphs (what is read,
  what holds, what does not, tested versions, links to the live results). A to Z, so Claude Code's paragraph stays
  first only by the alphabet.
- **Updates and privacy**: the OpenCode read-only line; the speed line now 5 MB and 1 s of CPU in 10 minutes, v1
  build, 2026-11-07, linked to the new research section.
- **Limits**: "Copilot waits do not hold", "OpenCode approval waits hold with no limit", the four folder variables
  (plus Copilot's `--config-dir`, `OPENCODE_DB` and channel databases), a new "Not supported, and why" list (Cursor
  and Antigravity in `docs/agent-hooks.md`'s words, VS Code Local chats in `p4-vscode-local-off`'s), "Not in v1"
  with agents inside WSL and OpenCode 1.x, and "Not tested live yet" per agent.
- The video comment no longer says 30 seconds. "Why I built it" unchanged (Haik's words).

`site/index.html`: "each one tested live", a "Not supported yet" line with a link to Limits, the OpenCode privacy line,
the hero's speed line (5 MB, 1 s in 10 idle minutes, v1 build, 2026-11-07), the video comment's length. The line "never
touches other apps" stays: true again with no hooks. `docs/product.md`: the README entry (left-out agents named in
Limits with their reason; no hook lines in v1) and "nothing that touches other apps" (the hooks rule is the one allowed
exception and v1 uses it for no agent).

#### Speed numbers (installed final build, x64; method and every raw value in `docs/research.md`, "Speed numbers, final v1 build (2026-11-07)")

| Number | 2026-11-06 (first 0.1.0, Claude Code only) | 2026-11-07 (final v1 build) |
|---|---|---|
| Idle private memory, median | 4.27 MB | 4.99 MB (end card: 5 MB) |
| Idle CPU in 10 minutes | 109 ms (0.018% of one core) | 1,016 ms (0.17% of one core) |
| During a hold | 4.24 MB, 109 ms (`wakeshot run -- ping`) | 3.98 MB, 1,141 ms (one working Claude Code session) |
| Start delay | Claude Code, median 114 ms | Codex CLI, median 116 ms (max 306) |
| `Setup.exe` | 12,310,500 bytes | 12,481,433 bytes |

What differs from the old method, plainly: the orchestrator's Claude Code session worked most of the time, so the
installed tray held for it; its idle numbers came from a detached watcher that waited for a quiet stretch (00:24 to
00:34, the session idle, no hold line). Delays are Codex CLI's, since a new Claude Code session joins a hold already on.

**What costs the CPU at rest** (both numbers: 109 ms on 2026-11-06, 1,016 ms now; dry-run copies of the same exe, each
watching only some real agent folders): no agent folder 31 ms per 10 minutes; Codex and Copilot 891 ms; plus OpenCode
1,141 ms; Copilot alone about 1,440 ms (5 minutes, doubled). So:

1. **Copilot's folder, most of it, and not steady.** Most likely cause (code plus a timing of the calls, not verified
   by counting the tray's own calls): `crates/wakeshot-windows/src/watch/agents.rs`, `Agents::on_tick`, drops the
   cached answer for every lock file no process held (`slot.owners.retain(|_, held| !held.is_empty())`) at each 30 s
   safety tick, so Restart Manager is asked about each again; one such question costs 14 to 22 ms of CPU here, and
   this PC has two dead Copilot sessions' `inuse.<pid>.hold` files (a crash leaves them until the next Copilot start).
   A question about a file `copilot-runtime.exe` holds costs about 135 ms; how often those are asked again was not
   measured.
2. **OpenCode's database check**, about 250 ms per 10 minutes: while `opencode.db` exists, its queries read 48 bytes
   of the `-shm` every 2 s (`QUERY_POLL_MS`, same file), needed because a commit gives no change notice.
3. Not the cause: file change notices (none in 60 s in the four agent homes), Codex (its leftover
   `.coordination.lock` is not listed).

Regression? Not a bug: both costs are by design and came with the new sources. It is a 9x rise in idle CPU that scales
with leftover lock files, so it is worth a card for Haik: ask Restart Manager again about an unheld lock file only when
its listing changed or a process of that image (or the pid in its name) started since the last ask; and poll the
`-shm` only while an `opencode.exe` runs. A fix would lower the end card's CPU number, and the end card would be
rendered again.

#### Tooltip capture

`media/public/shots/readme-holding-tooltip-dark.png` and `-light.png`, 636 x 274 (200% on the DELL), the installed
final tray holding for the orchestrator's Claude Code session and one `codex exec` test turn: "Holding awake: Claude
Code (1 session), Codex (1 session)". TrafficMonitor and the battery button's edge covered with plain taskbar. Haik's
mouse move: the first dark grab still showed Telegram's icon (he then moved it out of the tray), so dark was taken
again; each kept grab passed the pointer check. Afterwards the DELL is back at 150% and dark (read back after every
run); the laptop screen stayed at 125%. The Wakeshot icon stays on the taskbar where Haik put it.

#### The video

`C:\Users\haika\code\wakeshot\media\out\wakeshot-promo.mp4`: 38.02 s (38.016), 8,497,375 bytes (8.50 MB), -15.32 LUFS,
-1.47 dBTP, LRA 16.2 LU. Only the end card changed: "Idle: 5 MB of memory, 1016 ms of CPU in 10 minutes." and
"Measured on Wakeshot 0.1.0, 2026-11-07, x64." Frames 0 to 1019 and the whole sound decode to the same bytes as the
approved file; the poster is the same bytes. The approved file: the build folder's `approved\wakeshot-promo.mp4`.

#### Checks

| Check | Outcome |
|---|---|
| `npm run check` (dashes, emoji, credits, colours; pace; `agent-lists --check`) | ok: 108 files, 49 assets credited, 18 colour pairs; 47 starts on the beat; 5 of 5 agents, README and site lists agree |
| `npm run typecheck`, `check-art.mjs` | ok, ok (9,458,740 pixels) |
| `npm run video`, `check-video` | ok |
| Loudness after AAC, at most -1 dBTP | ok: -1.47 dBTP, -15.32 LUFS |
| Frames and sound against the approved final | only frames 1020 to 1139 differ; sound the same bytes |
| `npm run draft`, `check-video --draft` | ok: 9.10 MB, loops and state icons |
| `check-rebuild --final` | ok: a clean copy (`npm ci`, every build) gives every output the same bytes, the final, its poster, the draft and the soundtrack included |
| GitHub render of `README.md` (`gh api markdown`) | 5 tables, 9 folds, no comment text; every in-repo link and anchor resolves; external links 200 |
| `site/check_page.py site` | 0 errors, 7 spots wait (unchanged) |
| No agent listed unless its live card passed; "not tested live" marked | ok (agents.ts: 5 passed; "Not tested live yet" per agent) |
| ASCII, LF, no em dash | ok in every file edited |
| Power settings | none written by this card; the tray's own hold values read before |

#### Files changed

`README.md`, `site/index.html`, `docs/product.md` (README entry, "other apps" line), `media/data/speed.json`,
`media/data/shots.json`, `media/CREDITS.md`, `media/README.md`, `media/public/shots/readme-holding-tooltip-dark.png`
and `-light.png`, `media/out/wakeshot-promo.mp4`, `docs/research/promo-video.md`, `docs/research.md` (new last
section), this card. No Rust, no git.

#### Open items

1. The idle CPU card (above), Haik's call; a fix means one more end card render.
2. The installed tray's private memory rose from 3.98 to 7.69 MB in about 1.5 hours while holding for a busy Claude
   Code session. Not investigated; a leak is not verified.
3. The video upload and the README's video URL stay Haik's (Phase 8).

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

### Part B, round 1 (2026-11-06): what needs no live result

Haik's yes to the plan and to question 1's default came at about 19:15. This round did every step that does not wait
for a live result, rendered a draft and ran the checks.

#### Done

- **One list of agents**: `media/src/agents.ts` holds all seven, A to Z, each `passed` or `pending`; only `passed`
  ones show anywhere. Today: Claude Code and Codex CLI. When a live card passes, flip its word to `passed`, run
  `npm run agent-lists` (it writes the list into `README.md` and `site/index.html` between `<!-- agents -->`
  markers), then `npm run check`, which names the README rules paragraph to add. To drop one, delete its line. Two
  Copilots share one "Copilot" chip only when both passed.
- **Video** (`media/`): the name chips in shot 2 (from `agents.ts`, a pop and a soft pop sound at beat 7, gone with
  the whip), the shot 3 caption "Your agent starts: it turns **green**.", the event names in `timeline.ts`. Files:
  `src/agents.ts` (new), `src/copy.ts`, `src/timeline.ts`, `src/promo/Promo.tsx`, `scripts/agent-lists.mjs` (new),
  `package.json` (`check` runs the list check; `npm run agent-lists`), `README.md` (the check row and "The supported
  agents" under "When something changes"). Rendered: `out/draft/` only; the final and its poster are not rebuilt.
- **`README.md`**: the "AI agents" row with the list; "The list grows with each version."; the state table example
  `Holding awake: Codex (1 session)`; "knows only the agents listed above"; "More agents and more reasons to stay
  awake will come."; the rules summary; a **Codex CLI** paragraph (what it reads, approval waits, the holder process,
  tested live with 0.167.0 and a link to the results); the Automatic lines; the notice example "What held it:
  Codex."; the menu block's top line `Holding awake: Codex (1 session), cargo`; the "Update Wakeshot" row; Limits
  (an agent not on the list, Codex approval waits, `CODEX_HOME`, Codex CLI inside WSL, the Codex parts not tested
  live).
- **`site/index.html`**: both descriptions, the hero's 0.1 s line gone, the "AI agents" card with the list, the
  tooltip example, "For an agent not on the list", night steps 1 and 3.
- **`docs/product.md`**: the README entry rewritten from Haik's 2026-11-06 words (install first, every agent A to Z,
  none first, lightweight is enough, small table low); "nothing that touches other apps" now names the hooks rule as
  its one exception.
- **`docs/research/cursor-gemini-signals.md`**: section 5 starts with a "Superseded" note (one command, not a helper
  exe; added by Wakeshot with one notice, not opt-in; hooks, not `main.log`, for the Cursor editor; pointing to
  `docs/agent-hooks.md`); section 6 says it was answered. The old text stays as the record.

#### Checks (round 1)

| Check | Outcome |
|---|---|
| `npm run check` | ok: 89 files, 31 assets credited, 18 colour pairs; pace: 49 starts on the beat, 10 framings, 104 new things, longest gap 1.80 s, at most 5 things in 2 s; lists agree (2 of 7 passed) |
| The list check catches drift | ok: OpenCode flipped to `passed` gave 3 FAIL lines (README list, site list, missing paragraph); restored, ok |
| `npm run typecheck` | ok |
| `node scripts/check-art.mjs` | ok, all 9,458,740 pixels |
| `npm run draft`, then `check-video --draft` | ok: h264, aac, 7.42 MB; loops and the 6 state icons ok |
| Loudness of the draft (`loudnorm`, after AAC) | -15.3 LUFS, -1.1 dBTP (the limit is -1 dBTP) |
| `npm run check-rebuild` (loop, states, draft) | ok: a clean copy (`npm ci`, every build command) gives all 11 outputs the same bytes, the draft and its soundtrack included; its temp folder is gone |
| Draft frames looked at | 135 (two chips, centred) and 225 (the new caption), taken from the MP4 |
| GitHub render of `README.md` (`gh api markdown`) | the agents cell reads "Claude Code and Codex CLI", no comment text shows, 5 tables, the new links render |
| `site/check_page.py site` | 0 errors, 7 spots wait (unchanged) |
| ASCII only, LF, no em dash | ok in every file edited |
| `check-video` final, `check-rebuild --final` | not run: the final waits (below) |

#### Part B: what waits

1. **The final agent list and the "tested live" lines.** Passed live: Claude Code, Codex CLI. Pending: Copilot CLI,
   Copilot in VS Code, OpenCode, Cursor, Antigravity. For each that passes: its line in `agents.ts`, `npm run
   agent-lists`, its README rules paragraph with its tested version and results link, its limits and the parts not
   tested live, and the README's "Not tested live yet" line.
2. **Hooks lines**, only if Cursor or Antigravity passes: the README main text (the hook folders, the one notice, the
   prompts line, from `docs/agent-hooks.md` "README lines"), "What it changes on your PC", the hooks notice row (its
   title from `text.rs`), Uninstall; on the site, "never touches other apps" and Privacy. Until then those lines stay
   true as written.
3. **OpenCode lines**, if it passes: its read-only database line in Privacy; a SQLite credit if a bundled copy ships.
4. **The tooltip capture on the new install**: Claude Code and Codex working at once, light and dark, for the
   README's holding tooltip picture and its alt text; plus one check capture each of the tray and menu sets against
   `public/shots/`.
5. **Speed numbers on the new install** (a window with no agent at work, as the 0.1.0 run): `data/speed.json`, the
   end card, the README line, the site hero.
6. **The final render**: `npm run video` (the poster, frame 120, changes too: the chips are in by then), then
   `check-video`, `check-rebuild --final`, loudness at most -1 dBTP, and Haik's look.

### Part C, round 1 (2026-11-06): steps 7 and 8, the new storyboard, stills and logo rules

Status of steps 7 and 8: done. Steps 9 and 10 (the list from the live results, a draft render, every `p6-media`
check) come next; the orchestrator resumes this card at once.

#### The scenario in five lines

1. **The pain (0 to 4 s):** a laptop, an agent at work on its screen; at 1 s the laptop sleeps under it (screen dims,
   work freezes, power light breathes, "Sleeping"); then "Tired of sleepy laptops?" over the dark.
2. **Wakeshot (4 to 7 s):** the light comes on, "Wakeshot", "keeps it awake while agents work."
3. **The agents (7 to 11 s):** the light rises to the top, the agent tiles come in one after another under it (name,
   example task, spinner; one per agent that passed live, from `agents.ts`), "Stays awake for these agents.", the
   light pulses.
4. **Proof, then the sleep as usual (11 to 26 s):** the light flies into the real tray: the agent starts, green, the
   real tooltip, done, it lets go; the lid closes and the work goes on; settings back; later the same laptop sleeps
   as usual.
5. **End card (26 to 30 s):** as signed off, plus a trademark line in the small print. 30 s, the music, the beat grid
   and every check kept; the menu shot is out.

Full rows: `docs/research/promo-video.md`, "Storyboard: the 30 s main cut".

#### Still frames (PNG, 1920 x 1080, final look, no DRAFT mark)

In `C:\Users\haika\AppData\Local\Temp\claude\f7c2c08a8f04f07b\2a1d3942-3be1-4fdb-95cd-69265f507ad6\scratchpad\p6-agents-video-readme\stills\`:
`00-whole-cut-every-half-second.png` (60 frames on one sheet), then per shot `01-pain-f000`, `f045`, `f100`;
`02-name-f126`, `f180` (the new poster); `03-agents-f218`, `f240`, `f292-four-tiles-light-pulses`, and the same
frame `if-two-agents` and `if-all-six` (the layout for any count); `04-tray-f336`, `f360`, `f385`, `f440`; `05-lid-f500`,
`f560`, `f592`; `06-settings-f660`; `07-sleeps-f694`, `f745`; `08-end-f850`. The main set shows the four agents the
orchestrator reports as passed live (Claude Code, Codex CLI, Copilot CLI, OpenCode) through the `preview` prop;
`agents.ts` itself still says two.

#### Agent logos (step 8)

Every owner's page read (sources, quotes and verdicts in `promo-video.md`, "Agent logos"): Anthropic and Google need
their approval; OpenAI, Cursor and OpenCode are unclear for a stranger; GitHub allows a "shows integration" logo but
the same page also asks for prior written permission. **No logo goes in**: every tile shows the name in our type. No
logo file was added to the repo (GitHub's files sit only in the build folder's `logos-raw\github\`, unused).

#### Decisions (principal designer)

1. **The pain uses the lid shot's laptop**, so the video opens and ends on the same laptop asleep: once too early,
   once at the right time. Change it when Haik wants the old tilted terminal back as the hook.
2. **The name lands at 4.5 s**; the pace rule moves from "by 3 s" to "by 5 s", since Haik's question needs its own
   1.4 s. Change it when a viewer test shows people leave before the name.
3. **Names only, no logos** (rules above). Change it when every shown agent's owner allows its logo for this use; the
   logo then replaces the name in its tile, smaller than the Wakeshot name.
4. **"Stays awake for these agents.", not "Works with these agents."**: it says what Wakeshot does and steers clear of
   OpenAI's non-partner wording rule ("worked with"). Change it when Haik prefers "works with" and accepts that risk.
5. **A trademark line in the end card's small print** ("Agent names are trademarks of their owners; Wakeshot is not
   affiliated with them."), Google's legal line and the others' no-endorsement rules in one. Change it when the README
   credits carry it and Haik wants a cleaner end card.
6. **The menu shot is out** (Keep awake on demand): its 6 s pay for the pain and the agents in the same 30 s. Change it
   when Haik wants Keep awake in the video (a longer cut, or in place of the settings shot, 1 to 2 s longer).
7. **Each tile has an example task** ("fix the failing tests", ...) under the name, in the terminal's face, keyed by
   name in `copy.ts` with a fallback, so the list can grow with no new copy. Change it when the grid reads busy at the
   README's 900 px: drop the task lines.
8. **The tiles lay themselves out**: one row up to three, two rows from four, at most six (the two Copilots share one
   tile once both pass). A `preview` prop renders a layout for any names, for stills only; no build passes it.
9. **The light stays lit over the tiles and pulses once at 9.5 s**: it holds while any of them works, and the pulse is
   the one new thing the 2 s gap rule needs there.

#### Files changed

`media/src/timeline.ts`, `media/src/copy.ts`, `media/src/agents.ts`, `media/src/promo/Promo.tsx`,
`media/src/promo/world.ts`, `media/src/promo/LaptopTilt.tsx`, `media/scripts/check-pace.mjs`, `media/scripts/build.mjs`,
`media/README.md` (the poster row, the agents line), `docs/research/promo-video.md` (style rows, storyboard, counts,
captures table, risk 6, "Agent logos"), this card. Not touched: `README.md`, `site/`, `docs/product.md`, Rust, any
screen. No git.

#### Checks (round 1 of Part C)

| Check | Outcome |
|---|---|
| `npm run typecheck` | ok |
| `npm run check` (dashes, emoji, credits, colours; pace; agent lists) | ok: 89 files, 31 assets credited, 18 colour pairs; pace: 44 starts on the beat, 9 framings of 2 to 4 s, 83 new things, longest gap 1.90 s, at most 5 things within 2 s, 14 lines of words; lists agree (2 of 7) |
| Stills looked at | 0, 50, 100, 180, 218, 280, 292 (two, four, six tiles), 336, 850 full size; all 60 half-second frames on the sheet: no overlap, tiles clear of the light and the caption, the whip lands on the tray icon |
| ASCII only, no em dash | ok in the card, `promo-video.md`, `media/README.md`; `check` covers `media/` |
| Not run this round (step 10) | the draft render, `check-video`, `check-rebuild`, loudness, `check-art` |

#### Open items for steps 9 and 10

1. The list: `agents.ts` lags the live results ("Found outside scope" 5); flipping writes `README.md` and `site/`.
2. Step 10: `npm run draft` and every check; the music's lift now lands on the light at 4 s (silent 116 to 119), the
   tick per tile follows the list's length, the poster is frame 180.
3. The unused menu captures ("Found outside scope" 6) and `media/README.md` decision 11's history of the menu shot.

### Part C, round 2 (2026-11-06): steps 9 and 10, the list, a draft and every check

Status of steps 9 and 10: done, except what waits for Part B (below). Haik had not answered on the contact sheet, so
the storyboard of round 1 stands.

#### Done

- **The list from the live cards** (`media/src/agents.ts`): passed Claude Code, Codex CLI, Copilot CLI (1.0.106),
  OpenCode (2.2.5); Copilot in VS Code is `partly`, a new third value that counts as not passed (its card is done,
  but no Local working chat reached the disk, so no hold was seen live; waiting and idle were read right); Cursor and
  Antigravity `pending` (Antigravity's card is now `p4-antigravity-exact-end`, which may take it out of v1). The
  video shows four tiles: Claude Code, Codex CLI, Copilot CLI, OpenCode (not one "Copilot" tile, since only one
  Copilot passed).
- **`npm run agent-lists`** wrote "Claude Code, Codex CLI, Copilot CLI and OpenCode" into `README.md` (line 65) and
  `site/index.html` (line 57), between the markers; nothing else changed there.
- **The menu captures** (`media/public/shots/menu-S1-dark.png`, `menu-S5a` to `S5d-dark.png`): kept, since files
  refer to them: `media/data/shots.json` (set `menu`), `media/CREDITS.md` (five rows, and the grain note names
  `menu-S1-dark.png`), `media/src/promo/Capture.tsx` (the set's sketch), `media/src/promo/world.ts` (`SET_AT.menu`),
  `media/README.md`, `docs/research/promo-video.md` (captures table); `build.mjs` refuses a final while any listed
  file is missing. Deleting them means deleting those entries too: a longer cut would lose its Keep awake shot.
- **No logos**: names only, as round 1 decided; no logo file or logo code in `media/` (find and grep). The end card's
  small print keeps "Agent names are trademarks of their owners; Wakeshot is not affiliated with them."
- **Loudness margin** (decision): the mix's sample ceiling in `make-sounds.mjs` went from -2 to -2.5 dBFS. At -2 the
  draft read -1.01 dBTP after AAC, inside the -1 limit by 0.01 dB, and each agent tile adds a tick, so one more agent
  could cross it. Now -1.62 dBTP, loudness unchanged (-15.3 LUFS). Change it when the limiter pulls more than about
  3 dB (it pulls 2.2 now): then limit on 4x oversampled peaks instead of lowering the ceiling again.
- **Docs**: `media/README.md` (the `live` values; the `menu` set kept and where; decision 11 notes the cut has 9
  framings and no menu shot now; the light at 4 s; the new loudness), `docs/research/promo-video.md` (counts line with
  the four agents, captures table row, the draft's size and loudness, why the true peak is measured on the render).

#### Draft

`C:\Users\haika\code\wakeshot\media\out\draft\wakeshot-promo-draft.mp4` (git ignores it): 7,492,783 bytes
(7.49 MB), 30.02 s, H.264 and AAC, 1920 x 1080, 30 fps, BT.709; -15.26 LUFS integrated, -1.62 dBTP true peak, LRA
6.8 LU (`loudnorm`, after AAC). Frames 45, 180, 292, 360, 850 looked at (four tiles, the trademark line).

A copy of the same cut rendered as a final (no DRAFT mark, 0.1.0 speed numbers), for Haik's look:
`C:\Users\haika\AppData\Local\Temp\claude\f7c2c08a8f04f07b\2a1d3942-3be1-4fdb-95cd-69265f507ad6\scratchpad\p6-agents-video-readme\final-check\wakeshot-promo.mp4`
(7.37 MB, -1.62 dBTP). `media/out/wakeshot-promo.mp4` and its poster are the round 6 final again, byte for byte.

#### Checks (round 2 of Part C)

| Check | Outcome |
|---|---|
| `check.mjs` (dashes, emoji, credits, colours) | ok: 89 files, 31 assets credited, 18 colour pairs |
| `check-pace.mjs` | ok: 44 starts on the beat, 9 framings of 2 to 4 s, 83 new things, longest gap 1.90 s, at most 5 things within 2 s, 14 lines of words |
| `agent-lists.mjs --check` (third part of `npm run check`) | **FAIL, expected**: the lists agree (4 of 7), but `README.md` "When each hold counts" has no **Copilot CLI.** and no **OpenCode.** paragraph. Those are Part B step 5 (README text), not this round's README scope |
| `npm run typecheck` | ok |
| `check-art.mjs` | ok, all 9,458,740 pixels |
| `npm run draft`, `check-video --draft` | ok: 30.02 s, 7.49 MB, codecs, size, BT.709 tags, loops and the 6 state icons |
| `npm run video` (final mode, for the rebuild check), `check-video` | ok: 30.02 s, 7.37 MB |
| Loudness after AAC (draft and final) | -15.26 LUFS, -1.62 dBTP (limit -1); before the ceiling change -1.01 |
| `check-rebuild --final` | ok: a clean copy (`npm ci`, `check-art`, loop, states, draft, video) gives all 15 outputs the same bytes, the draft, the final and the soundtrack included; its temp folder is gone. Run with the new cut's final in `media/out/`, then the round 6 final put back (sha256 checked) |
| `site/check_page.py site` | 0 errors, 7 spots wait (unchanged) |
| GitHub render of `README.md` (`gh api markdown`) | the agents cell reads "Claude Code, Codex CLI, Copilot CLI and OpenCode", no marker text shows, 5 tables |
| ASCII only, no em dash | ok in every file edited (`check` covers `media/`; a scan of the card and `promo-video.md`) |

#### What is left

1. **README paragraphs** for Copilot CLI and OpenCode in "When each hold counts" (tested versions, links to the
   results, limits, parts not tested live), the OpenCode read-only line in Privacy, the README "Not tested live yet"
   line with Copilot in VS Code: Part B step 5. Until then `npm run check` fails on them.
2. **Cursor and Antigravity**: when their cards end, one word each in `agents.ts`, `npm run agent-lists`, the tiles
   follow (five tiles lay out as 3 and 2).
3. **Part B**: the two-agent tooltip capture and the speed numbers on the new install; then the final render into
   `media/out/`, `check-video`, `check-rebuild --final`, loudness, and Haik's look.
4. Haik's answer on the contact sheet and on this cut.

### Part D (2026-11-06): Haik's notes on the Part C draft, steps 11 to 14

Status of Part D: done. Waits for Haik's music pick; then that track goes into the repo and its final into
`media/out/` (below).

#### What changed

- **The pain is 8 s, the video 34 s** (step 11). 0 to 3 s close on the laptop's screen, the agent at work with its
  text about 30 px; 3 s the camera pulls back; 3.5 s the idle timer dims the screen while the agent still works;
  4.5 s the screen goes off, the work stops, the power light breathes, "Sleeping" pops, "Laptop sleeping."; 5.5 s it
  all sinks into the dark as the camera eases back; 6 s "Tired of sleepy laptops?" alone; 8 s the light. 16 beats, so
  the music's lift still lands on a bar line. Every shot after it starts 4 s later; nothing else in them changed.
- **A clean frame behind the question** (step 12). Frame by frame, Part C put a veil to 86% black over the laptop
  from the question's first frame: for 12 frames the question came in over the full picture, then the laptop and its
  chip stayed as a ghost at 14%. Now the laptop, its chip, the words and the stage's lift fade out before the
  question starts, and the question sits on the same plain stage the light then comes on in.
- **Music at one level** (step 13). No soft hook, no silence before the light, no 5 dB lid dip, no 4 dB duck of the
  whole music under each chime. Instead: a slow leveler evens each track to its own median; under each chime only the
  chime's band gives way (6 dB bell at 1.5 kHz); when the laptop sleeps the track goes on behind a low-pass, the beat
  still going, and opens over the beat before the light. `make-sounds.mjs` stops when the music's 3 s loudness leaves
  its median by more than 2 LU (6 inside the muffle). Each of the four tracks reads within 0.5 LU of its median
  outside the muffle, and 0.1 to 2.2 LU under it inside.
- **Three new tracks** (step 14), CC0, picked by numbers (not heard) from ten a helper agent found; each licence read
  again by curl on its page. Sources, licences, numbers: `docs/research/promo-video.md`, "Music and effects".

#### The four renders (final look, every check passed)

In `C:\Users\haika\AppData\Local\Temp\claude\f7c2c08a8f04f07b\2a1d3942-3be1-4fdb-95cd-69265f507ad6\scratchpad\p6-agents-video-readme\candidates\`:

| File | Length, size | Loudness after AAC | Track, licence and source | In one line (from tags and numbers, not heard) |
|---|---|---|---|---|
| `wakeshot-promo-34s-0-funky-disco-beats.mp4` | 34.0 s, 8.45 MB | -15.25 LUFS, -1.45 dBTP | "Funky Disco Beats to Boogie/Woogie to", Fupi, CC0, opengameart.org/content/funky-disco-beats-to-boogiewoogie-to | the current pick: a funky disco loop whose bass sits mostly below 100 Hz (78% of its energy), so small speakers show little of it |
| `wakeshot-promo-34s-1-cant-stop-my-feet.mp4` | 34.0 s, 8.45 MB | -15.26 LUFS, -1.82 dBTP | "Can't Stop My Feet !", Loyalty Freak Music, CC0, Free Music Archive (album "ROLLER DISCO DANCE DANCE") | roller disco, brighter and the evenest of the four (130 to 120 BPM, A minor), 2.4 times the current track's share in 100 to 250 Hz; its page says "Instrumental: No" |
| `wakeshot-promo-34s-2-lost-in-the-rythm.mp4` | 34.0 s, 8.45 MB | -15.27 LUFS, -1.58 dBTP | "Lost In The Rythm", HoliznaCC0, CC0, Free Music Archive (album "Lost") | disco-dance with the most bass over mids and a dark, soft top (105 to 120 BPM, sped up 14%), A minor, instrumental |
| `wakeshot-promo-34s-3-funked-up.mp4` | 34.0 s, 8.45 MB | -15.23 LUFS, -2.27 dBTP | "Funked Up", Joth, CC0, opengameart.org/content/funked-up (its notice asks for a credit) | 70s funk loop with live-style drums, bass guitar and Rhodes (116 to 120 BPM), A minor, the most bass in the range small speakers play; funk more than disco |

Also there: `opening-contact-sheet-0-to-8.5s.png` (35 frames of the new opening, every 7.5 frames, final look),
`wakeshot-promo-poster-34s-frame-300.png` (the new poster, the same for all four), and each run's log.

**Pick (principal designer): option 1, "Can't Stop My Feet !".** It is the same type Haik likes (disco, from a roller
disco album), has far more of the bass small speakers play (26% of its energy in 100 to 250 Hz against 11%), keeps a
bright top so it stays lively, has the steadiest level of the four (LRA 4.4 LU), and is in A minor like the current
track, so both chimes stay in key. Risk: its page marks it not instrumental, and no one has heard it yet. Change it
when Haik's ear says otherwise.

To use a pick: copy its file from the build folder's `music-raw\` unchanged into `media/public/music/`, set
`data/music.json` (file, `bpm`, `lift` from `promo-video.md`'s table, title, author, page), write its `CREDITS.md`
line (replacing Fupi's), remove `funky-disco-beats.ogg` if it is not kept, then `npm run video` and the checks. If Haik
keeps the current track, `wakeshot-promo-34s-0-funky-disco-beats.mp4` is the same bytes `npm run video` makes now
(checked), so it can be copied over `media/out/wakeshot-promo.mp4` with the poster above.

#### Files changed

`media/src/timeline.ts` (34 s; the pain's moments, places, camera, captions, events, sounds; `MUSIC.muffle` and
`open`; the thud and lid chime a little softer), `media/src/promo/Promo.tsx` (the pain place, the sink, the chip in
the world, the veil out), `media/src/promo/LaptopTilt.tsx` (`every`, `preset`), `media/src/promo/world.ts`
(`SLEEP_CHIP_AT`), `media/scripts/lib/fit-music.mjs` (one level, leveler, muffle), `media/scripts/lib/audio.mjs`
(`shortTerm`, `svf`), `media/scripts/make-sounds.mjs` (the band cut, the evenness rule), `media/scripts/check-pace.mjs`
(the name by 9 s), `media/scripts/check-video.mjs` (the timeline's length, 45 s at most), `media/scripts/build.mjs`
(poster frame 300), `media/README.md`, `media/CREDITS.md` (poster frame), `docs/research/promo-video.md` (pace rows,
Sound, storyboard, counts, Music and effects, Formats), `docs/research.md` (a new section), this card. Not touched:
`README.md`, `site/`, `docs/product.md`, Rust. No git. No screen capture.

#### Checks (Part D)

| Check | Outcome |
|---|---|
| `check.mjs`, `check-pace.mjs` | ok: 89 files, 31 assets credited, 18 colour pairs; 47 starts on the beat, 11 framings of 2 to 4 s, 96 new things, longest gap 1.90 s, at most 5 things within 2 s, 14 lines |
| `agent-lists.mjs --check` (third part of `npm run check`) | FAIL, expected: no **Copilot CLI.** and **OpenCode.** paragraphs in `README.md` (Part B step 5) |
| typecheck, `check-art.mjs` | ok, ok (all 9,458,740 pixels), for each of the four runs |
| `check-video --draft` and `check-video` | ok for each of the four (34.01 s, 8.45 MB final, 8.48 MB draft) |
| Loudness after AAC, at most -1 dBTP | ok: -1.45, -1.82, -1.58, -2.27 dBTP; -15.23 to -15.27 LUFS; LRA 4.4 to 5.4 LU (round 6: 6.6) |
| Music evenness (`make-sounds.mjs`) | ok: -0.5 to +0.4 LU outside the muffle for all four |
| `check-rebuild --final` | ok for each of the four (14 outputs the same bytes). One earlier run of option 1 had its final differ once; its rerun passed and made the same final bytes; the pain rendered twice was equal in every pixel (`docs/research.md`) |
| Frames looked at | the hook before (16 frames) and after (17, then 35 on the contact sheet), frames 300, 330, 480, 1019 |
| ASCII, LF, no em dash | ok in every file edited |
| `check-video` on the committed final | FAIL, expected: it is the round 6 final (30.02 s) until the pick's final goes in ("Found outside scope" 8) |

#### Open items

1. Haik's pick by ear; then the pick into the repo and its final into `media/out/` (above), and `CREDITS.md`'s line.
2. Part B as before: the README paragraphs, the two-agent tooltip capture, the speed numbers on the new install.
3. The rebuild mismatch seen once: not located. If it comes back, `candidate.sh` in the build folder keeps the copy
   and names the first differing frames.

### Part E (2026-11-06): Haik's notes on the Part D renders, steps 15 to 17

Status of Part E: done. Haik picked the current track ("Funky Disco Beats", Fupi, CC0) and likes the video (about
23:50); the 38 s final with it is in `media/out/`.

#### The opening in five lines

1. **0 to 3 s:** close on a terminal window (`claude` already run); a person types "migrate billing to API v2" into
   the agent's input box, key by key, about 11 a second; only real keystrokes and a faint room and fan, no music.
2. **3 to 6.5 s:** Enter; the agent mostly waits: a turning spinner, "Thinking" with a slow shimmer, its clock in real
   time, then a time lapse into minutes; a tool line now and then.
3. **6.5 to 9.5 s:** a cut to the laptop with the same screen; the idle timer dims it while the agent still thinks;
   it sleeps: the screen goes dark, the power light breathes, "Sleeping", a soft power-down and the fan winding down.
4. **10 to 12 s:** "Tired of sleepy laptops?" alone on the plain stage, near silence, a riser of air into the light.
5. **12 s:** the light comes on and the music starts on its lift; the rest as signed off, 4 s later; 38 s in all.

#### Decisions (principal designer)

1. **A flat terminal seen close, then a cut to the laptop** (not a pull back from the laptop's tilted screen): the
   typed prompt reads straight on at about 52 px. Change it when Haik wants one continuous move.
2. **No words over the pain, only the question:** the prompt, the thinking line and the chip tell it, and big words
   would cover the typing. Change it when a muted viewer misses the sleep: put "Laptop sleeping." back over the laptop.
3. **The agent's clock as a time lapse** (real time 1.2 s, then 150 clock seconds a second: about 4 minutes at the
   cut, 9 at the sleep) says "long task, nobody at the laptop" without words. Change it when it reads as fake: a time
   cut instead.
4. **Real CC0 keystrokes, not code-made:** no one could listen before the pick, and a recording is far likelier to
   sound real. Change it when Haik's ear finds them too clicky: lower the high end or use another take set.
5. **A very low room tone and fan under the opening:** the keys do not sit on digital silence, and the fan winding
   down is the laptop's own sound of sleeping. Change it when they read as hiss: drop the room, keep the fan.
6. **A 0.5 s riser into the light:** the music lands as a drop with the low hit. Change it when it feels like one
   effect too many.
7. **The prompt shortened** to "migrate billing to API v2" (was "... and run all tests"): 25 keys in 2.3 s. The lid
   shot's laptop shows the same prompt.
8. **Four tiles, not five:** both Copilots passed, so they share one "Copilot" tile (the rule approved in Part A);
   the README and site lists name all five ("Found outside scope" 10).

#### The render (Haik's pick)

`C:\Users\haika\AppData\Local\Temp\claude\f7c2c08a8f04f07b\2a1d3942-3be1-4fdb-95cd-69265f507ad6\scratchpad\p6-agents-video-readme\candidates\wakeshot-promo-38s-0-funky-disco-beats.mp4`,
the same bytes as `C:\Users\haika\code\wakeshot\media\out\wakeshot-promo.mp4` (sha256 `123d674c...`): 38.02 s,
8,497,349 bytes (8.50 MB), -15.32 LUFS, -1.47 dBTP, LRA 16.2 LU (the quiet opening widens it; the music alone sits
within -0.5 to +0.4 LU of its median from the light). Poster frame 420 in `media/out/` and the same folder. Also
rendered, every check passing, not picked: options 1 and 2 (`...-38s-1-cant-stop-my-feet.mp4`, -1.84 dBTP;
`...-38s-2-lost-in-the-rythm.mp4`, -1.60 dBTP). Option 3's run stopped at Haik's pick.

Stills (final look): `C:\Users\haika\AppData\Local\Temp\claude\f7c2c08a8f04f07b\2a1d3942-3be1-4fdb-95cd-69265f507ad6\scratchpad\p6-agents-video-readme\stills-e\`:
`00-opening-0-to-12.5s-every-quarter-second.png` (51 frames) and 12 full-size frames, `01-f000` to `12-f530` (the
agents shot with the final list).

#### Files changed

`media/src/timeline.ts`, `media/src/copy.ts`, `media/src/agents.ts` (final v1 list), `media/src/promo/AgentTerminal.tsx`
(new), `media/src/promo/Terminal.tsx` (`TitleBar` exported), `media/src/promo/LaptopTilt.tsx`, `media/src/promo/Promo.tsx`,
`media/src/promo/world.ts`, `media/scripts/make-sounds.mjs`, `media/scripts/lib/fit-music.mjs`,
`media/scripts/check-pace.mjs`, `media/scripts/check-video.mjs` (comment), `media/scripts/build.mjs` (poster 420),
`media/public/keys/` (18 new WAVs, CC0), `media/CREDITS.md`, `media/README.md`, `media/out/wakeshot-promo.mp4` and
`media/out/wakeshot-promo-poster.png` (the new final), `README.md` line 65 and `site/index.html` line 57 (the generated
list only), `docs/research/promo-video.md`, `docs/research.md` (new section), this card. No git, no Rust, no screen.

#### Checks (Part E)

| Check | Outcome |
|---|---|
| `check.mjs` (dashes, emoji, credits, colours) | ok: 108 files, 49 assets credited, 18 colour pairs |
| `check-pace.mjs` | ok: 47 starts on the beat, 12 framings of 2 to 4 s, 90 new things, longest gap 1.90 s, at most 5 within 2 s, 12 lines; the name at 12.5 s (rule now by 13 s); frame 0 opens on the terminal |
| `agent-lists --check` | FAIL, expected: no **Copilot CLI.**, **Copilot in VS Code.**, **OpenCode.** paragraphs in `README.md` (Part B step 5) |
| typecheck, `check-art.mjs` | ok, ok (every pixel), in each run |
| `check-video --draft`, `check-video` | ok: 38.02 s, 8.50 MB final (9.10 MB draft), codecs, BT.709, loops, state icons |
| Loudness after AAC, at most -1 dBTP | ok: -1.47 dBTP, -15.32 LUFS (options 1 and 2: -1.84, -1.60) |
| Music evenness from the light | ok: -0.5 to +0.4 LU |
| `check-rebuild --final` | ok on tracks 0, 1 and 2 (a clean copy gives every output the same bytes, the final and soundtrack included) |
| Frames looked at | the contact sheet, 13 stills, 3 frames from the rendered MP4 |
| ASCII, LF, no em dash | ok in every file edited |

#### Open items

1. Part B: the three README paragraphs, the two-agent tooltip capture, the speed numbers on the new install, then
   the final render again with those numbers.
2. Haik has heard the keys, the power down and the riser only in this render; levels were set by script.
