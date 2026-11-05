# Rewrite the brand doc, fix every doc of the old brand, delete what is left of it

Phase: 6 (brand apply). Status: ready. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 120K tokens. Written by
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

## Found outside scope

## Log

## Result
