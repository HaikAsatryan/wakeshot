# Rewrite the brand doc, fix every doc of the old brand, delete what is left of it

Phase: 6 (brand apply). Status: done. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 120K tokens. Written by
the orchestrator of Haik's brand session, 2026-11-05.
Runs after: `p6-brand-apply-art`, `p6-brand-apply-web`, `p6-brand-apply-media` (their numbers and files are final).
Parallel with: `p6-brand-screen-session` Part A (it writes only its own card). No cargo builds unless a doc comment in
Rust changes (then `cargo fmt --all --check` and `cargo clippy -p <crate>` on that crate only, in
`CARGO_TARGET_DIR=<scratchpad>\p6-brand-docs-sweep\target`).
Real laptop: nothing. Scratch folder: `<scratchpad>\p6-brand-docs-sweep\`.
Owns: `docs/brand.md`, `docs/architecture.md`, `docs/product.md`, `docs/research.md` (edits to old-brand lines; keep
each fact's "how checked"), `CLAUDE.md`, `README.md` (comments and any old-brand words; not the header from
`p6-brand-apply-web`), `packaging/README.md`, `media/README.md`, comments in `site/` and in Rust files that name the
old brand, a new card `docs/tasks/p8-cleanup.md`, and deletes of any old-brand file still left.

## Goal

The app, page and media now carry the held light in Moss. The docs still describe the Sunrise in night violet and sun
gold, and some files of the old brand may remain. Haik's rule: no legacy anywhere, old assets deleted, not kept. Only
the research history stays, until the Phase 8 cleanup, and that history gets an explicit list so nothing is forgotten.

## Read first

- `docs/phases/06-ship.md` work log, line "2026-11-05 ~15:00": Haik's decisions.
- The Results of `p6-brand-apply-art`, `p6-brand-apply-web`, `p6-brand-apply-media` (final numbers, files, decisions).
- `docs/research/brand-directions-2.md` (the design and the reasons), `docs/research/brand-macos.md` (the Mac rules),
  `docs/research/brand-identity.md` (wordmark, colour reasons), `docs/brand.md` (today's, to rewrite).

## Deliverables

1. **`docs/brand.md` rewritten** for the new brand, short, as a cache of facts: the idea (a light left on while work
   runs; filled holding, outline idle, struck through off), the colours with their contrast and colour-blind numbers
   (from the apply cards' final measurements, with how they were checked), the tray rules (holding colour, high
   contrast one colour, sizes, vertical position), the app icon (plated, sizes, the 256 PNG frame, the small
   drawings), the Mac rules from `brand-macos.md` (later work, three layers), the wordmark (Instrument Sans SemiBold,
   where it is used and where not), the social card, the reasons for each choice in one line each, and where each
   asset is made (`art.rs` is the source).
2. **Every doc of the old brand fixed**: `architecture.md`, `product.md`, `research.md` lines, `CLAUDE.md`, `README.md`
   (the media comments describe the old loop: rewrite them to the new loop and art; no user-facing copy change beyond
   old-brand words), `packaging/README.md`, `media/README.md` (only if the media card left a trace), comments in
   `site/` and in Rust doc comments outside the files the art card already fixed. Replace the old line, do not append
   history. In `research.md`, a fact about the old art that is no longer true is replaced by the current fact, or
   removed if nothing replaces it.
3. **Whole-repo search** for old traces, outside `target/`, `node_modules/` and the research history: `#7250F4`,
   `#7957F4`, `#6B49F3`, `#5B3FD6`, `#A996FF`, `#FFCC4D`, `#14121F`, "night violet", "sun gold", "Sunrise",
   "sunrise", "rises", "rising", "half sun", "rays", "cup", "Cup", "EC32", "Inter" (as the font), "violet", "gold".
   Each hit: fixed, or listed as history with its reason. Case-insensitive where it makes sense; read each hit, do not
   replace blindly ("rises" in a power sentence is fine).
4. **Delete old shipped, user-facing and generated assets still left** (the apply cards should have replaced or deleted
   most): any stale GIF, stand-in shot, draft video or poster, old SVG or PNG of the old mark outside the research
   history. List each delete in the Result.
5. **Research history stays, listed**: the brand reports (`docs/research/brand-*.md`), round 1 and round 2 pictures
   (`docs/research/img/brand/dir-*`, `dir2-*`, `review-*`), the `docs/research/img/ux/` evidence the reports cite, the
   task cards, reviews and phase work logs that name the old brand. Create `docs/tasks/p8-cleanup.md` from
   `docs/tasks/_template.md` (Phase 8, Status: later, Runs after: the Phase 7 field test) with an explicit list of
   these files and folders under a "Brand history to remove" heading, so the Phase 8 cleanup removes or rewrites
   them. Nothing else goes in that card yet beyond a one-line goal.

## Acceptance checks

- The search of deliverable 3 run again at the end: every remaining hit is in the history list, each named.
- `docs/brand.md` numbers equal the apply cards' Results (spot-check each colour pair).
- Every link in the docs you changed points to a file that exists (a small link check script in your scratch folder).
- If a Rust comment changed: `cargo fmt --all --check` and `cargo clippy -p <crate> --locked -- -D warnings`.

## Out of scope

Code behaviour, art, the page's look, media renders (the apply cards); task cards, reviews and work logs (history,
listed for Phase 8, not rewritten); menu words and any user-facing copy beyond old-brand words; anything Mac beyond
`brand.md`'s rules. Do not run git.

## Questions for Haik

None expected. Calls inside the brief: decide and list under "Decisions for Haik's review".

## Decisions for Haik's review

Made by the worker as principal designer, each with its reason and when to change it.

1. **Live cards keep their dated history.** In `p6-media`, `p6-pages` and `p6-readme` I fixed what a future worker
   acts on (Runs after, Read first, deliverables, decisions that still stand, the Result's "what exists" and open
   items); their dated Log lines and done rounds stay as written and are on the Phase 8 list. Why: rewriting a dated
   line would make it say something that did not happen. Change it when Haik wants live cards clean now.
2. **Old-art facts in `research.md` kept, worded art-neutral**, not removed: the Settings snapshot, the stale header
   PNG, Explorer's file icon and the notice look still describe Windows' behaviour; each keeps its date and evidence,
   now as "the app icon" or "the art of that day". Change it when Haik prefers them gone.
3. **`p8-cleanup` has two short notes beyond the list**: the live pointers to repoint before anything is removed, and
   the search hits that are not old brand (the chimes' "rising" pitch, facts about other products). Without them
   Phase 8 would break links or delete true facts.
4. **`pages.yml` also leaves out `site/fonts/InstrumentSans-Regular.woff2`**, beyond the two files asked: only the
   social card's HTML loads it. Change it when the page itself uses the Regular face.
5. **A colour-blind column in `brand.md`**, measured by this card (Machado 2009 at full strength, the method of rounds
   1 and 2), with the rim and bar numbers from a port of `art.rs`'s own test, so they are what the art draws.
6. **`brand.md` names no old mark or colour**, so it passes the legacy search with 0 hits; the reasons the old pick
   was dropped live in the reports until Phase 8.
7. **README icon alt texts**: "A filled light over a line", "An outlined light over a line", "An outlined light over a
   line, struck through" (from `product.md`'s words, each readable alone); the loop comment names the real files and
   the 240 px width `media/README.md` gives.

## Found outside scope

1. **The lit frames' rim is under 3:1 for protan colour blindness** (2.88 to 2.92 under Machado 2009 at full strength;
   plain 3.13 to 3.16), against round 1's rule C4 (3.0 after simulation, `brand-identity.md` section 8). The art
   card's decision 3 (rim `#F7DC7F`, approved by Haik 15:35) was judged on the plain ratio. With the light edge
   `#FFEA9C` as the lit rim, as round 2 drew it, the same check gives 3.53 plain and 3.26 colour-blind (scratch
   `rim_edge.py`). Owner: `art.rs` (Haik's call; the glow and the near-white core still carry the light's shape).
2. **`site/wakeshot.svg` and the two lockups have no generator in the repo**: `gen_icon.py` and `gen_lockup.py` stay
   in `p6-brand-apply-web`'s scratch folder, which is temporary. When `art.rs` changes, the port must be redone by
   hand. Options: move the scripts under `site/` (and out of the Pages upload), or have `art.rs` write the SVG.
3. `p8-cleanup` needs a place in the plan (there is no Phase 8 file yet): the orchestrator's.
4. In the Pages upload the dark lockup is no longer linked by any served page (it was the social card's); harmless,
   `check_page.py` notes it only on an upload copy.
5. `p6-pages`'s check table still gives the draft's lowest contrast (6.05); today's lowest page pair is 5.03 (the step
   number). The card reruns its checks before it closes.
6. actionlint ran without shellcheck (not installed), so the `run:` line was not shell-linted.

## Log

- 2026-11-05: started. Read the card, plan.md "How we work", CLAUDE.md. Orchestrator added one rule: task cards not
  done (ready, blocked, later) and phase file sections that state current design are live, so they get fixed too.
  Also: `pages.yml` must leave the social card's sources out of the upload; run actionlint if installed.
- 2026-11-05: slip: one read-only `git status` ran inside a listing command (output thrown away). No other git.
- 2026-11-05: read the three apply cards' Results, the 06-ship decisions line, `brand-directions-2.md`, the Mac rules
  (`brand-macos.md` section 5), the wordmark part of `brand-identity.md`, today's `brand.md`, the new research.md
  sections. Search script `search.py` in scratch (the card's terms plus `risen`, `sun`, `horizon`, the Segoe
  wordmark line). First run: 47 files with hits. Live files with old-brand lines: `brand.md`, `product.md`,
  `research.md` (11 places), `README.md` (4 media comments), `site/index.html` (2 copy lines), `05-tray-ux.md` picks
  2 and 3, live cards `p6-media`, `p6-pages`, `p6-readme`. `architecture.md`, `CLAUDE.md`, `packaging/README.md`,
  `media/README.md`: no old-brand words (one stale detail in `architecture.md`: the notice PNG is the flat drawing).
  No old-brand asset file left outside the research pictures (`assets/`, `site/`, `media/out/` all new; the spike's
  `toast-icon.png` is a blue test disc, not the brand). actionlint is not installed; a 1.7.12 copy from an earlier
  session's scratch matches its checksum list.
- 2026-11-05: colour checks in scratch: `colours.py` (every pair from the apply cards: all 25 card numbers match to
  0.01; colour-blind column added), `rim.py` (a port of `app_icon_contrast`: rim 3.50 to 3.51 flat, 3.13 to 3.16 lit,
  bar 3.46 to 4.15, equal to the art card). Found: the lit rim `#F7DC7F` falls to 2.88 to 2.92 under protan
  simulation (round 1's rule C4 asks 3.0); the flat frames' `#FFEA9C` keeps 3.25. `de.py`: every brand colour 34.8
  or more from Claude's orange. `docs/brand.md` rewritten (idea, colours with both numbers, tray, app icon, wordmark,
  lockup, social card, page and media, Mac rules, where each asset is made).
- 2026-11-05: `docs/research.md`: "Tray UX: icon art" cut to what still holds (taskbar and glyph colours, glyph
  contrast, Microsoft's guidance, padding, stroke, writers), the old art's numbers out, pointer to "Brand apply";
  old-art words out of "menu painting", "notices", "Velopack dependency", "dry-run e2e", "Haik's check", "Integration
  checklist with Haik" (the Settings snapshot fact kept, now art-neutral) and the 2026-11-05 screen session, each
  fact's how-checked kept; the media GIF fact keeps its evidence without the old hex. New section "Brand colours under
  colour-blind simulation".
- 2026-11-05: `product.md` tray line (the held light; brand colour only while holding), `architecture.md` (the notice
  PNG is the flat drawing), `README.md` media comments (loop and three icon spots: the held light, new alt texts,
  the real loop files), `site/index.html` two copy lines (old-brand words only; `check_page.py site`: 0 errors, 8
  waits, as before). `pages.yml`: the upload step also removes `social_card.py`, `social-card.html` and the Regular
  font only that HTML loads; actionlint 1.7.12 (the checksum-checked copy): 0 findings. A copy of `site/` with those
  files removed: `check_page.py` 0 errors (one note: the dark lockup is no longer linked, as expected).
- 2026-11-05: phase and live cards. `05-tray-ux.md` picks 2 (14 ICO frames) and 3 (brand colour while holding), and
  "Where other tracks meet" (favicon from `art.rs`). `p6-media`: Runs after, Read first, the built list, decisions 2
  to 4 (now the media card's), Part B item 2. `p6-pages`: Runs after, Read first, a deliverable, decisions 4, 5, 7,
  10, Found outside scope, the Result's fonts, files and redo step. `p6-readme`: Runs after and the open items (brand
  line, the lockup's 64 px trigger). Dated Log lines and done rounds left as history. Deliverable 4: no old-brand
  asset left to delete (every picture outside `docs/research/img/` looked at or dated after the apply cards; the social
  card seen). `brand.md` no longer points at a round 2 picture.
- 2026-11-05: `docs/tasks/p8-cleanup.md` written (Phase 8, later, after the Phase 7 field test) with the list. Final
  checks: search again (every remaining hit named in that list), numbers against the cards, link check
  (`links.py`), files read back from disk (ICO 14 frames, 256 PNG 32,229 bytes, ICO 185,835, notice 1,798, social
  card 285,642). No Rust file changed, so no cargo. Status done.

## Result

Done 2026-11-05. The docs describe the held light in Moss; the old brand is left only in the history that
`docs/tasks/p8-cleanup.md` lists for Phase 8.

What was done:

- **`docs/brand.md` rewritten**: the idea; colours with WCAG and colour-blind numbers; tray rules (three states by
  shape, brand colour only while holding, high contrast one colour, sizes, vertical position); the plated app icon
  (14 frames, 256 as PNG, flat frames under 30 px, the notice picture, no splash); Instrument Sans SemiBold, where it
  is used and not; the lockup and the social card; the page and the media; the Mac rules (later work); where each
  asset is made. A reason and a "change it when" for each choice.
- **Docs fixed** (old lines replaced, not appended): `docs/research.md` ("Tray UX: icon art" cut to what still holds;
  old-art words out of seven other sections, each how-checked kept; one GIF fact without the old hex; new section
  "Brand colours under colour-blind simulation"), `docs/product.md` (tray line), `docs/architecture.md` (notice
  picture), `README.md` (the loop comment and the three icon comments), `site/index.html` (two copy lines, old-brand
  words only), `docs/phases/05-tray-ux.md` (picks 2 and 3, the favicon line), live cards `p6-media`, `p6-pages`,
  `p6-readme`. `CLAUDE.md`, `packaging/README.md`, `media/README.md`: nothing old left, unchanged.
- **`.github/workflows/pages.yml`**: the upload leaves out `site/social_card.py`, `site/social-card.html` and the
  Regular font only that HTML loads.
- **Deleted**: nothing. No old-brand asset was left outside the research pictures: `assets/`, `site/`, `media/out/`
  are all from the apply cards (checked by date, frame list and by eye); the spike's `toast-icon.png` is a blue test
  disc.
- **New**: `docs/tasks/p8-cleanup.md`, "Brand history to remove": the six brand reports, 46 brand pictures, 35 old-art
  screen pictures, 24 done cards, dated lines in `p6-media`, `p6-readme`, three phase files and `plan.md`; what to
  repoint first; the hits that are not old brand.

Checks:

| Check | Outcome |
|---|---|
| Search of deliverable 3 again (scratch `search.py`; plus `risen`, `sun`, `horizon`) | pass: outside the listed history files, hits remain in 13 files; each is a dated line named in `p8-cleanup` or a "not old brand" item named there (chimes' rising pitch, a caption that rises, facts about other products). `brand.md`, `product.md`, `architecture.md`, `README.md`, `site/`, `media/`, `crates/`: 0 old-brand hits |
| `brand.md` numbers against the apply cards | pass: 25 colour pairs recomputed (`colours.py`), all equal to 0.01; rim and bar per frame from a port of `app_icon_contrast` (`rim.py`), equal to the art card; sizes read from disk |
| Links in every changed doc (`links.py`: Markdown links, HTML src and href, backticked repo paths, anchors) | pass: 470 checked; the 8 misses are expected: two path patterns, two folders a later card creates, one dated Log line, three on-purpose example paths in an old research section |
| `python site/check_page.py site` after the copy change | pass: 0 errors, 8 spots wait (as before); an upload copy without the removed files: 0 errors |
| actionlint on `pages.yml` | pass: 0 findings (1.7.12, not installed; a copy in an earlier session's scratch, checksum-checked against its release list) |
| Rust comment changed | none, so no cargo run |

Open items: Haik's call on the lit rim under protan (Found outside scope 1); the SVG port scripts (2); the
orchestrator places `p8-cleanup` in the plan (3); `p6-brand-screen-session` was running and is on the list once done.
