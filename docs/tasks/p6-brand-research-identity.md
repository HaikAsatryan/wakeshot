# Brand research: what a good small-app brand needs, 2026 trends, and how ours compares

Phase: 6 (brand review). Status: done. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 120K tokens. Written by
the orchestrator of Haik's brand session, 2026-11-05.
Runs after: none. Parallel with: `p6-brand-research-windows`, `p6-brand-research-macos`, and any building card (this
card builds nothing and edits no code).
Real laptop: nothing on the laptop; web research only.
Scratch folder: `<scratchpad>\p6-brand-research-identity\`.

## Goal

Haik is not sure the logo is good or the colours are right, and wants the brand to follow 2026 visual trends. Find
what a good brand for a small free desktop tool consists of, what makes a mark good, which 2026 trends are lasting and
which are fads, and how the current Wakeshot brand stands against the field. Facts and judgement, no drawings.

## Read first

- `docs/brand.md` (the current brand: the Sunrise, night violet `#7250F4`, sun gold `#FFCC4D`, and the reasons).
- `docs/product.md`, the first screen (what Wakeshot is and who it is for: people who run AI coding agents and long
  builds on Windows 11).
- `docs/research/rivals-others.md` and `docs/research/rivals-lidguard.md`: skim for the rivals' looks.
- Pictures: `docs/research/img/ux/icon-app.png`, `icon-sheet-dark.png`, `icon-sheet-light.png`; `site/wakeshot.svg`.

## Deliverables

`docs/research/brand-identity.md`. Write it as you go (a section at a time), so a session limit loses nothing. Mark
each fact **verified** (source URL and date) or **unverified**. Separate facts from your judgement. Answer:

1. **The parts of a brand for an app like this.** Mark (symbol), app icon per platform, small monochrome glyph (tray,
   menu bar), wordmark and lockup, favicon, social preview (GitHub's repo card size), colours, type, motion, voice.
   Which are needed at launch and which can wait. Which must be the same picture and which may differ.
2. **What makes a mark good,** as a short checklist from named sources (simple, distinctive, works at 16 px, works in
   one colour, ownable, tells what the product does or feels like). Then score the current Sunrise against it, plainly.
3. **Is the Sunrise distinctive?** Search for products whose logo or app icon is a half sun over a horizon (weather,
   calendar, alarm, wellness, other dev tools, icon fonts such as Segoe Fluent Icons, Material Symbols, SF Symbols
   `sunrise`). How generic is it; does it read as "weather" or "sunrise alarm" first?
4. **Is the colour distinctive?** The field of AI and developer tools in 2026 and their brand colours (Claude, ChatGPT,
   Copilot, Gemini, Cursor, Windsurf, Codex, Warp, Raycast, Linear, Zed, Ghostty, Vercel, Ollama, LM Studio, and the
   keep-awake rivals). Is violet or a violet gradient now an "AI purple" cliché? What colour space is still open? Keep
   the rule from `docs/brand.md`: nothing that could look like an official Claude or Anthropic tool (Claude's orange).
5. **2026 visual trends** for app icons and small-product brands (Liquid Glass and depth, soft 3D, grain, bold flat
   marks, gradients, monochrome with one accent, variable type, motion marks). For each: lasting or fad, and does it fit
   a tool that should feel calm, trusted and native. Name Pantone's 2026 colour if it matters (verify it).
6. **Name and meaning.** What "Wakeshot" suggests (wake, shot: a camera shot, an espresso shot, a jolt, a "shot" of
   something) and which picture ideas follow from the name and from what the product does (awake only while work runs,
   then lets go). List 6 to 10 mark ideas in one line each, including the current Sunrise, with the strongest reason
   for and against each. Do not draw them.
7. **Type.** A free wordmark font (SIL Open Font License or similar) that fits, with 3 candidates. Settle the open item
   in `docs/brand.md`: may Segoe UI or Segoe UI Variable be used in rendered pictures and video (Microsoft's font
   licence terms, verified); if not, what to use instead.
8. **Colour rules.** WCAG and colour-blind checks any new palette must pass (icon on light and dark Start, text on the
   page, the gold-on-plate kind of pairs). Give a small script-ready list of the checks.

No mockups: `p6-brand-directions` draws the options.

## Acceptance checks

- Every numbered question has an answer or says plainly that no source was found.
- Question 4 names at least 15 products with their colour and a source each.
- Question 6 lists 6 to 10 ideas, each with one reason for and one against.
- Facts carry URL and date; judgement is marked as judgement.

## Out of scope

Code, `docs/brand.md`, `docs/plan.md` and phase files (another orchestrator session owns them). Windows and macOS
platform rules (the two other research cards).

## Questions for Haik

None expected. If one comes up, write it here with your default and go on.

## Found outside scope

- `docs/brand.md`, Contrast: no colour-blind check. The gold sun on the violet plate falls below 3:1 under protan
  simulation (2.6 and 2.8; tritan 2.7 on the lighter end). Report section 8. Not edited (out of scope).
- `docs/brand.md`, Logo and wordmark: the "Not checked" Segoe UI item is now answered (report section 7: allowed in
  bitmaps rendered on Windows; not on a Mac or a CI runner). The owner of `docs/brand.md` should replace that line.
- `docs/brand.md`, Colours: the plate's dark end `#6B49F3` passes dark Start at exactly 3.00, on the edge.
- Apple SF Symbols `sunrise` could not be checked without a Mac; left for `p6-brand-research-macos`.

## Log

- 2026-11-05: card written by the orchestrator.
- 2026-11-05: worker started. Read brand.md, product.md, the icon sheets and site/wakeshot.svg.
- 2026-11-05: orchestrator relayed Haik: the name stays "Wakeshot"; a Mac version is a real plan, so marks must work
  as a Mac icon and menu bar glyph; free fonts only. Report section 1 updated for the Mac parts.
- 2026-11-05: section 1 written. Sources gathered for 2 to 4 (Simple Icons data, icon sets, Microsoft and Apple icon
  guidance, Rand, Haviv, Romaniuk).
- 2026-11-05: sections 2 to 4 written. Findings: Material `wb_twilight` is nearly our mark; our violet is dE00 1.5
  from Qwen and 1.7 from Proton; no brand in the sample sits at yellow-green.
- 2026-11-05: section 5 written. Computed: a dark plate fails Microsoft's 3:1 rule on dark Start (1.1), so the colour
  pick moved to a mid-tone plate in an open hue; the current gold on violet fails 3:1 under protan simulation (2.6,
  2.8). Fonts and the Microsoft font FAQ checked for section 7.
- 2026-11-05: sections 6 to 8 and the verdict written. A check against all 3,469 Simple Icons brands showed magenta
  sits on GraphQL, so olive became the first colour pick. Acceptance checks run; card done.

## Result

Done 2026-11-05. Report: `docs/research/brand-identity.md` (all 8 questions, a verdict on top). Files changed: that
report and this card. No code, no `docs/brand.md`.

Findings (judgement on top of the facts in the report):

- Mark: a good tray glyph, a weak brand mark. Google's Material Symbols `wb_twilight` (Apache 2.0) is nearly the same
  drawing; Phosphor, Fluent, Lucide, Tabler and Bootstrap all ship a sunrise, filed under Weather; 4,515 on the Noun
  Project. Scores 4 pass, 2 partial, 2 fail (distinctive, ownable) on an 8-point checklist from Rand, Haviv,
  Romaniuk, Apple and Microsoft.
- Colour: night violet `#7250F4` is dE00 1.5 from Qwen, 1.7 from Proton, 0.8 from Moonrepo, 4.6 from Google Jules;
  9 of 20 sampled coloured AI and developer brands sit in the violet band. Gold on the plate fails 3:1 under protan
  simulation. A black plate fails Microsoft's 3:1 rule on dark Start.
- Open colour: olive (OKLCH hue about 110 to 125) as a mid-tone plate with a white mark; magenta-rose second (near
  GraphQL).
- Top mark ideas: the sun rising from a terminal prompt, a lit window at night, a pilot light (10 ideas listed).
- 2026: flat and bold, one accent, state-driven motion, a two-layer split for Liquid Glass; skip grain and fake glass.
  Pantone 2026 is Cloud Dancer 11-4201 (verified), not relevant to the icon.
- Type: Segoe UI and Segoe UI Variable are allowed in pictures and video as bitmaps rendered on Windows (Microsoft
  font FAQ); not on a Mac, a CI runner or as a web font. Wordmark pick: Instrument Sans SemiBold (OFL); Figtree and
  Inter the other two.
- Colour rules: a 14-item check list (C1 to C14) in section 8.

Acceptance checks:

- Every numbered question answered: pass (sections 1 to 8).
- Question 4 names at least 15 products with colour and source: pass (25 in the table, plus Copilot, ChatGPT, Codex and
  the keep-awake rivals).
- Question 6 lists 6 to 10 ideas, each with a reason for and against: pass (10).
- Facts carry URL and date; judgement marked: pass (each section splits facts and judgement).

Open items: not verified: SF Symbols `sunrise`; ChatGPT and Codex icon colours (`openai.com/brand` refused the fetch);
the new Copilot icon's colours; the 15 LogoLounge 2026 trends (paywall); whether an SVG with outlined Segoe text is
allowed. The scripts (`colour.py`, `nearest.py`, icon and font sheets) are in the scratch folder
`C:\Users\haika\AppData\Local\Temp\claude\f7c2c08a8f04f07b\ecb73ce7-120e-418c-9724-2485d1f6c8ad\scratchpad\p6-brand-research-identity\`,
not in the repo; `p6-brand-directions` can port the check list from section 8.
