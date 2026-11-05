# Review: the brand apply (held light in Moss)

Reviewer: `wakeshot-reviewer`, 2026-11-05. Cards: `p6-brand-apply-art`, `p6-brand-apply-web`,
`p6-brand-apply-media`, `p6-brand-docs-sweep`. Brief: old brand traces, sources against ports, contrast re-run, craft,
the two known items. Scripts: my scratch folder `p6-brand-review\` (`contrast.py`, `rim.py`, `bar.py`, `pix.py`,
`half.py`, `page.py`, `de.py`), run with the web card's scratch venv for Pillow.

## Verdict

Good work, close to shippable. No shipped or user-facing trace of the old brand is left (code, assets, exe icons,
`site/`, `README.md`, `media/`, `packaging/`). Every port I could check matches `art.rs` exactly. The plain contrast
numbers in `docs/brand.md` match my own run.

Two real problems:

1. Two numbers that feed Haik's open rim decision describe a model, not the icon as drawn. The test leaves out the
   bloom. Drawn pixels: the light's edge is 1.4 to 2.0, and the bar's top is under 3:1 from 64 px up.
2. The SVG ports have no generator and no check in the repo. The media port's check only runs by hand.

The rest is minor.

0 blockers, 2 majors, 6 minors.

What I verified, so it need not be redone:

- `cargo test --locked -p wakeshot-windows --lib -- tray::art notify::` in my own build folder: 44 passed. This
  includes the golden tests `app_icon_file_matches_the_art`, `favicon_matches_the_art` and
  `notice_icon_file_matches_the_art`. The printed numbers equal the art card's.
- `node media/scripts/check-art.mjs`: 9,458,740 pixels, 0 differ (3 sheets, 14 ICO frames, notice PNG).
  `node media/scripts/check-video.mjs --draft`: all 25 checks pass. Both loops start on the port's idle glyph and
  hold its holding glyph pixel for pixel.
- `site/favicon.ico`: its 16, 32 and 48 frames are byte-equal to the ICO's frames. The ICO has 14 frames; the 256
  frame is a PNG of 32,229 bytes; the file is 185,835 bytes; the notice PNG is 1,798 bytes. All as `brand.md` says.
- `site/wakeshot.svg` and both lockups, checked by hand against `Plated::at(256)`: margin 11, side 234, corner
  43.875, light at (128, 106) with r 48, bar 58..198 x 175..191. The ramp end (287.825, 170.825) is the 120 degree
  CSS line. Glow r 145.08 with stops 0.6 / 0.21 / 0. Shade from y 139.7 to 245. Bloom r 91.2. Focal point (117.44,
  92.56) with stops 0.5 / 0.86 / 1. Shadow dy 2.667, blur 4.472 (three 9 px box passes). Layer order as in
  `draw_plated`. All equal. Lockup: cap height 107.64 = 0.46 x 234, centred at y 128; ink gap 60.84 = 0.26 x 234.
- Contrast (WCAG 2.2; Machado 2009 at full strength in linear sRGB). Tray, plate and page pairs equal `brand.md` to
  0.02 plain. The colour-blind column is within 0.05. For example, `#3B7A2A` on `#D8DADC` is 3.58 for me against
  3.60 in `brand.md`, likely 8-bit rounding. Every pair passes its minimum. My protan run of the half rule at 48 px:
  61% on light Start, 87% on dark (`brand.md` says "not run"). CIEDE2000 from `#D97757`: 34.8 or more, as stated.
- `python site/check_page.py site`: 0 errors, 8 waits.

## Findings

### 1. Major: the light's edge and bar numbers leave out the bloom, so the test cannot see the drawn icon

- Where: `crates/wakeshot-windows/src/tray/art.rs` lines 1863 to 1887 (`app_icon_contrast`) against lines 542 to
  545 (`draw_plated` paints the bloom over the plate and the bar). Also `docs/brand.md` lines 55 to 57 and 64 to
  65, and `docs/research.md` lines 4847 to 4849.
- What is wrong: the test compares the rim and the bar with `plate_colour(x, y, glow)`. That colour has no bloom.
  But the bloom (`LIGHT_EDGE` at up to 0.8 alpha, reaching 1.9 r) is painted on every pixel round the light and over
  the bar. My reads of the real ICO pixels (`pix.py`) give:
  - The light's edge, its outer ring of pixels against the ring just outside: 2.04 at 30 px, 1.87 at 48, 1.61 at
    64, 1.43 at 256. `brand.md` gives 3.13 to 3.16 for the rim on the bare plate, and 2.03 to 2.58 "inside the
    glow". The glow number also leaves out the bloom.
  - The bar's top centre against the row above it: 2.78 (64 px), 2.83 (72), 2.95 (80), 2.92 (96), 2.93 (256);
    protan 2.73 to 2.89. Frames 30 to 60 keep 3.09 to 3.37. `brand.md` says "White bar on the plate round it, every
    frame 3.46 to 4.15".
- Why it matters: Haik's open rim question is judged on the bare-plate number, but the rim never sits on bare plate.
  The bar claim is false for the five frames that Start, Explorer and the installer show largest. And a later change
  to the bloom or glow cannot fail any test.
- What still holds: the light reads as a whole. Its core `#FFFBE8` against the plate just past the bloom (1.9 r) is
  3.25 to 3.70 plain and 3.16 or more under all three simulations (`rim.py`). Microsoft's half rule holds (63% and
  91%).
- Fix: make `app_icon_contrast` measure the pixels of `app_icon(size)`, as `half_rule` already does: the bar's top
  row against the row above, and the light's core against the plate beyond the bloom. Correct the three rows in
  `brand.md` and `research.md`. If the bar must keep 3:1 as drawn (my pick), end the bloom above the bar: it reaches
  1.9 r now, and the bar's top sits at about 1.44 r from the light's centre. Then re-run the half rule. Not computed.

### 2. Major: the SVG ports have no generator and no check in the repo; the media port's check runs only by hand (known item b)

- Where: `site/wakeshot.svg`, `site/wakeshot-lockup-light.svg`, `site/wakeshot-lockup-dark.svg`. Their generators
  (`gen_icon.py`, `gen_lockup.py`, `textpath.py`, with fontTools and uharfbuzz in a venv) exist only in the web
  card's scratch folder, which is temporary. `docs/brand.md` line 164 admits this. `site/check_page.py` does not
  compare the SVGs with anything. `.github/workflows/ci.yml` runs cargo only, not `media/scripts/check-art.mjs`.
- What is wrong: the next art change means a hand port of three shipped files: the README header, the page hero and,
  through the dark lockup, the social card. No check fails if one drifts. The same goes for the media port
  (`media/src/art/heldLight.ts`): after an `art.rs` change and a writer run, the golden tests pass and CI stays green,
  while the GIFs and the video keep the old drawing until someone runs `check-art` by hand.
- Why it matters: Haik's bar is no drift and no legacy. This is exactly how an old drawing survives in a picture.
- Fix (my pick, lean): have `art.rs` write `site/wakeshot.svg` itself. Every number is already in `Plated::at`;
  `gen_icon.py` is about 130 lines of string formatting. Add a golden test that holds the SVG equal to what the
  writer makes. The lockups then become the icon's `<g>` plus the fixed word path (outlines that change only with
  the font), so a small template in the same writer or a script under `site/` covers them. Add a CI step that runs
  `node media/scripts/check-art.mjs`. It needs no `npm ci`: it imports only `node:` modules and a `.ts` file that
  Node runs directly, but not verified on CI. Stronger option: move the three scratch scripts into the repo
  unchanged. Change it when the lockup needs kerning work again.

### 3. Minor: the lit rim `#F7DC7F` under protan (known item a)

- Where: `art.rs` line 109 (`LIGHT_RIM`), `docs/brand.md` line 56.
- Numbers (`rim.py`, `contrast.py`):
  - On the plate where the light sits, without glow: 3.11 to 3.15 plain, 2.87 to 2.91 protan. The card says 3.13 to
    3.16 and 2.88 to 2.92; f32 against f64.
  - By round 1's own wording (`brand-identity.md` line 500: "each mark colour vs each plate end"), the rim fails C3
    plain too: 2.92 on `#628C2B`, 2.72 protan.
  - `#FFEA9C` gives 3.30 plain and 3.10 protan on that end.
  - In the drawn icon (finding 1) the edge is 1.40 to 1.74 with `#F7DC7F` and 1.58 to 1.99 with `#FFEA9C`. Neither
    is near 3; the bloom decides that, not the rim.
- Rating: minor. The rim is a one to two pixel warm line on a light whose edge is soft by design. The mark's colours
  (core, bar) pass C3 and C4 against both plate ends: core 3.81 / 3.69 protan, white 3.96 / 3.86 protan on the light
  end.
- Recommendation: keep `#F7DC7F`, the look Haik approved, and change the rule to what is drawn (finding 1): rim
  colours are a detail, and the light's core holds 3:1 against the plate beyond its bloom under all three
  simulations. If Haik wants C3 and C4 met as written, only `#FFEA9C` passes against the plate's light end. Middle
  values such as `#FBE28C` pass only against the plate where the light sits (3.29 / 3.05), not against the end
  (3.08 / 2.88).

### 4. Minor: Task Manager shows the old violet Sunrise icon, and its two captures would be kept as current

- Where: `docs/research/img/brand/screen-taskmgr-processes.png` and `screen-taskmgr-startup.png`: both Wakeshot rows
  show the violet square with the gold sun. `docs/tasks/p8-cleanup.md` lines 23 to 25 say to keep every `screen-*.png`.
- What is wrong: the screen session found the cause (its card, Log 16:13): Task Manager's own cache of this install
  path, which held older builds. The shell's own read of the exe is Moss. No user has an old-brand build, so nothing
  ships wrong. But these two pictures show the old brand and are listed as today's evidence. Whether an updating user
  sees a stale icon after a future icon change (Velopack updates in place at the same path) is not verified.
- Fix: retake both captures after the cache clears (sign out or reboot; not tested which is enough), or mark them as
  "old icon from Task Manager's cache" and add them to the p8 list. Before any later icon change, check whether an
  update over an older build leaves Task Manager stale.

### 5. Minor: gaps in the Phase 8 history list

- Where: `docs/tasks/p8-cleanup.md` lines 48 to 49.
- What is wrong: `docs/phases/05-tray-ux.md` line 323 (the 2026-11-04 12:07 work log line: "built from an export of
  f47391d (the Sunrise)") is not named. The list covers the 2026-11-04 lines "on the icon pick, the menu glyphs and
  `p5-haik-check`", and this line is the screen slot. The Task Manager captures are also missing (finding 4).
  Everything else my sweep found outside the listed history checks out. The search covered the brief's terms plus
  risen, sun, horizon, orange, steam, purple, indigo, amber, yellow, night, helmet, dome and brown, and every image
  and font outside `docs/research/img/`. The hits that remain are the chimes' "rising", "horizontal", "nightly
  backup", OFL "ARISING", a caption that rises, other products' facts and the high contrast test yellow.
- Fix: add the 12:07 line and the two captures to the list.

### 6. Minor: the video title uses a second lockup that `brand.md` does not define

- Where: `media/src/promo/scenes.tsx` lines 57 and 270 (`Lockup`); picture
  `docs/research/img/brand/apply-media-poster.png`.
- What is wrong: `brand.md` lines 120 to 124 define one lockup, the icon beside the name, and say the page uses "a
  row, not the icon over the name". The web card's decision 8 dropped the stacked form as the generic "centred icon
  over a big name" template (W2). The video title, the outro and the poster use exactly that stacked form, with
  their own sizes (icon 256, name below).
- Why it matters: the brand shows two lockups and only one is written down. A stacked lockup is a normal thing to
  have, but its ratios should be fixed like the row's.
- Fix: either add a stacked lockup to `brand.md` with its ratios (name size against the plate, gap), or use the row
  lockup in the title. Haik's or the media card's call.

### 7. Minor: the rim constant does not match its comment

- Where: `art.rs` line 108: "`LIGHT_EDGE` a third of the way to amber (`#E9C24A`)".
- What is wrong: a third of the way in sRGB gives `#F8DD81`, in OKLab `#F8DD83`, in linear light `#F8DE88`. The
  constant is `#F7DC7F`. The heldLight.ts port copies the constant, so nothing breaks; the comment just cannot
  rebuild the value.
- Fix: say "round 2's rim, `#F7DC7F`" or "about a third"; drop the derivation.

### 8. Minor: `docs/plan.md` status is from before the apply

- Where: `docs/plan.md` lines 281 to 283. The latest status is 2026-11-05 14:20: "Next: Haik's 8 brand decisions,
  then a brand apply card". The Phase 8 line (74) does not mention `p8-cleanup`.
- Why it matters: `CLAUDE.md` sends every worker to `plan.md` first for status and the next step.
- Fix: one status line (apply done, the open rim question, the screen session, then `p6-media` Part B), and point
  Phase 8 at `p8-cleanup.md`. The orchestrator's file; the sweep listed this as its open item 3.

## Outside the brief (copy, not old brand; for `p6-media` or `p6-pages`)

- `site/index.html` line 40 gives the loop's alt text as "Wakeshot's tray icon wakes while an agent works and rests
  when it is done". `README.md` line 8 gives "Wakeshot's tray icon: the light comes on while an agent works and goes
  off when it is done", and line 69 of the page says "Alt text as in the README". The same line 40 asks for "a still
  PNG for `prefers-reduced-motion`", which `media/` does not make yet.

## Fix check (2026-11-05)

Scope: the two majors only, against the "Review fixes" parts of `docs/tasks/p6-brand-apply-art.md` and
`p6-brand-apply-media.md`. Scripts: my scratch folder `p6-brand-recheck\pix2.py` (new: the bar's rows and the light's
core on the real ICO pixels), and `p6-brand-review\pix.py` and `half.py` unchanged. Tests in my own build folder,
deleted after.

### Finding 1: Fixed

- `app_icon_contrast` (`art.rs` lines 2292 to 2375) now reads the pixels of `app_icon(size)`: every fully covered
  pixel of the bar and of the light against the nearest plain plate pixel, so the bloom and the glow count as drawn;
  in lit frames the light's core (its centre pixel) against the 1 px ring of plain plate just past the bloom's end.
  Plain and the worst of protan, deutan and tritan. A later bloom or glow change now moves these numbers and can fail
  the test (follows from the method; I did not run a changed bloom).
- The bloom ends at 1.6 light radii (`art.rs` lines 550 to 554). The SVG's bloom has the same stops (r 76.8; offsets
  0, 0.625, 0.8, 1; alpha 0.8, 0.44, 0.144, 0).
- `cargo test --locked -p wakeshot-windows --lib -- tray::art`: 27 passed, 1 ignored (the writer), among them
  `app_icon_contrast`, `half_the_icon_keeps_3_to_1_on_start` and the golden tests.
- My reads of the new `assets/wakeshot.ico` (184,720 bytes), plain / worst colour-blind (protan is the worst
  everywhere):

  | Frame | Bar's top row against the row above | Core against the plate past 1.6 r | Edge, ring means (record) |
  |---|---|---|---|
  | 30 | 3.64 / 3.53 | 3.55 / 3.44 | 2.18 |
  | 32 | 3.58 / 3.48 | 3.50 / 3.39 | 2.15 |
  | 36 | 3.46 / 3.36 | 3.60 / 3.48 | 2.09 |
  | 40 | 3.74 / 3.63 | 3.60 / 3.48 | 2.02 |
  | 48 | 3.50 / 3.40 | 3.54 / 3.43 | 1.95 |
  | 60 | 3.55 / 3.45 | 3.54 / 3.43 | 1.89 |
  | 64 | 3.12 / 3.05 | 3.12 / 3.04 | 1.66 |
  | 72 | 3.16 / 3.09 | 3.15 / 3.08 | 1.62 |
  | 80 | 3.25 / 3.17 | 3.11 / 3.03 | 1.60 |
  | 96 | 3.21 / 3.13 | 3.11 / 3.04 | 1.55 |
  | 256 | 3.24 / 3.17 | 3.11 / 3.04 | 1.44 |

  Before the fix the bar's top was 2.78 to 2.95 from 64 px up. Now every lit frame keeps 3:1 plain and colour-blind.
  The bar and core numbers equal the test's printed ones at every frame, to 0.01. The bar's bottom row is 4.01 or
  more. My edge method (ring means) differs from the test's pixel pairs, so 1.44 to 2.18 against its 1.40 to 2.03 is
  expected; both say the edge is soft.
- Half rule at 48 px (`half.py`): 69% / 91% plain, 68% / 87% colour-blind. Every lit frame on light Start keeps 56% or
  more colour-blind.
- `docs/brand.md` lines 53 to 59 equal the drawn numbers: flat edge 3.75 to 3.95 / 3.53 to 3.76; core 3.11 to 3.60 /
  3.03 to 3.48; lit edge 1.40 to 2.03 / 1.35 to 1.97; bar 3.12 to 4.34 / 3.05 to 4.20; half rule 69% / 91% and 68% /
  87%. Lines 107 to 108 (184,720 bytes, 256 frame 31,114) equal the file and the test. `docs/research.md` lines 4846
  to 4850 and 5049 to 5057 equal them too. The old section at line 4962 is marked superseded at 4966.
- The rim rule is now the core past the bloom, as my finding 3 recommended; the orchestrator's call, not judged again.
- Margins are thin (bar 3.05 at 64 px, core 3.03 at 80 px, colour-blind). Not a fault: a warmer glow or longer bloom
  later fails the test, which is the point.
- Look: `docs/research/img/brand/apply-fixes.png`, before and after at 256, 64 and 48 px on both Starts. The tighter
  halo is hard to tell from the old one. No new problem seen.

### Finding 2: Fixed, one gap left (minor, not new)

- `art.rs` writes `site/wakeshot.svg` and both lockups (`svg_icon_parts`, `app_icon_svg`, `lockup_svg`, lines 1596 to
  1857) from `Plated::at(256)` and the drawing's own constants. `site_svgs_match_the_art` (line 3014) holds the three
  files byte-equal, and passed. `.gitattributes` gives every text file LF on checkout, so the string compare also
  holds on the Windows runners. The lockups' icon part equals `site/wakeshot.svg` line for line; the word is fixed
  path data with an assert on the plate it was set for (margin 11, side 234).
- `site/social-card.png`: I rendered it again from a copy of `site/` with `social_card.py`. Same SHA-256 as the
  shipped file, so it shows the current dark lockup. It stays a hand step after an art change, and `brand.md` line
  180 says so (GitHub's social preview is uploaded by hand anyway).
- `node media/scripts/check-art.mjs` (Node 26.13.0): 9,458,740 pixels, 0 differ, exit 0.
- CI job `art-port` (`.github/workflows/ci.yml` lines 136 to 153): ubuntu-latest, `actions/setup-node@v7` (v7.1.0 is
  out, 2026-10-08, by `gh api`), Node 26, no npm install, runs the check on every push and pull request. The check
  imports only `node:` modules, `lib/png.mjs` (`node:zlib`, `node:fs`) and `heldLight.ts` (no imports). Not verified
  on a real CI run.
- Gap: the review sheets `docs/research/img/ux/icon-sheet-*.png` and `icon-app.png` have no golden test (the only
  reader is `check-art.mjs`), yet `brand.md` line 179 says "golden tests hold each file equal to the art". The sheets
  are the only path by which the tray glyphs reach `check-art`. Failure: a change to `tray_mask` or `colorize`
  without a writer run passes `cargo test` (the ICO does not change) and passes `art-port` (the port equals the old
  sheets), so the README loop GIFs keep the old glyph while CI is green. This round's off slash change was such a
  change; it was caught only because the worker ran the writer. Fix: a golden test for the three sheets and
  `icon-app.png`, as for the ICO; at least correct line 179.
- Note only: `check-video` holds the loop GIFs to the port but is not in CI, so a port change without a GIF re-render
  also stays green. My finding did not ask for it; this round's GIFs did not change.

### New problems from the fixes

None a user would see. Note: the art card's first Result table (`p6-brand-apply-art.md` lines 312 to 314) keeps the
old numbers (rim 3.13 to 3.51, bar 3.46 to 4.15, half rule 63%); its "Review fixes" part supersedes them. History, no
action.
