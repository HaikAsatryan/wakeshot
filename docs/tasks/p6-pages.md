# GitHub Pages page

Phase: 6. Status: done (2026-11-07 04:09; ready to go live: the orchestrator's steps are in the Result).
Needs Haik: his one look before it goes live is waived by his go of 2026-11-07 ~02:39 (see Log); every call is for his
review. Agent: `wakeshot-worker`. Size: about 60K tokens.
Real laptop: no. No screen use: checks are scripts and headless renders.
Runs after: `p6-readme`; the tray track's `p5-haik-check` (menu words); `p6-brand-apply-art` and `p6-brand-apply-web`
(the brand in `site/`, both done 2026-11-05). Parallel with: any card that does not own `site/` or
`.github/workflows/pages.yml`.
Build folder: none (no Rust).

## Goal

One small static page at v1 (`product.md`, Distribution) that says what Wakeshot does and links to the download,
built from the README and deployed only when a release is published.

## Read first

- `README.md` (`p6-readme` Result); `docs/product.md`, "The job" and "Distribution".
- `docs/architecture.md` section 10 (action pinning, read-only tokens).
- The brand: `docs/brand.md`; what `p6-brand-apply-web` changed in `site/` (its Result and decisions).

## Deliverables

- `site/index.html`, `site/style.css`, the icon (from `art.rs`: `docs/brand.md`). One page: what it does, a picture
  if the tray track has one, install, what it changes, privacy (no telemetry), links to Releases and the README.
  Light and dark through `prefers-color-scheme`. No scripts, no trackers, no fonts or files from other hosts.
- `.github/workflows/pages.yml`: deploys `site/` with `actions/upload-pages-artifact` and `actions/deploy-pages` on
  `release: published` and by hand; `pages: write` and `id-token: write` only in the deploy job; actions by major
  tag (section 10).
- In the Result, the orchestrator's step: turn Pages on with source "GitHub Actions" (`gh api -X POST
  repos/HaikAsatryan/wakeshot/pages -f build_type=workflow`) at v1, not before.

## Acceptance checks

- `actionlint` reports 0 errors on `pages.yml`.
- A script checks that the HTML parses, every local link and image resolves, and no `src` or `href` points outside
  `github.com/HaikAsatryan/wakeshot` and the page itself.
- No em dash and no emoji in the page text (script).

## Out of scope

- Turning Pages on (orchestrator, at v1); a code signing policy page (only when SignPath is applied for, after v1).

## Questions for Haik

None. Every call below is a design call, made by the worker for Haik's review.

## Decisions (worker, 2026-11-04, for Haik's review)

1. **Deploy on a finished Release run (`workflow_run`), not on `release: published`.** Why: `release.yml` publishes
   with the `GITHUB_TOKEN`, and GitHub starts no workflow from an event that token made (only `workflow_dispatch` and
   `repository_dispatch` are exempt), so `release: published` would never fire. Also, a `workflow_run` job runs on
   the default branch, while a release event runs on the tag, which users report the `github-pages` environment
   refuses by default. Change it when releases are published by a personal or app token, or by hand: then
   `release: published`, plus a `v*` tag rule on the `github-pages` environment.
2. **Stable releases only.** The job runs only when the Release run was a tag push, succeeded, and the tag has no
   `-` (the pre-release rule of `release.yml`). Why: a pre-release must not change the public page, and before v1
   Pages is off, so a deploy for `v0.1.0-rc.1` would fail. Cost: a winget job failure fails the Release run after
   the release is out; then run Pages by hand (written in `pages.yml`).
3. **The page comes from the released commit** (`workflow_run.head_sha`), not the newest one on main, so it matches
   what users download. A run by hand uses the chosen branch.
4. **The checks live in `site/check_page.py`** (Python standard library) and `pages.yml` runs it with `--final` on
   the built folder before every upload. Why: a future worker reruns the same checks, and a page with a waiting spot
   can never go live. Python, not PowerShell like `packaging/`, because it has an HTML parser built in. Since
   2026-11-07 the build leaves the repo's own files out (decision 11), so the old `rm` step is gone. Change it when
   `ci.yml` (not owned here) should check the page on every push: add `python3 site/check_page.py site` there (no
   network needed).
5. **Header icon as SVG, favicon from the art** (`p6-brand-apply-web`, `p6-brand-apply-art`, 2026-11-05):
   `site/wakeshot.svg` is the 256 px app icon ported layer by layer from `art.rs`, sharp at any size (against the
   ICO's 256 frame: mean difference 0.64 of 255 over white, none a person sees at 100%); `site/favicon.ico` (16, 32
   and 48 px) is written by `art.rs` itself, pixel-snapped, sharper than an SVG there. Change it when `art.rs` gains
   an SVG writer (then `write_art_files` should write `site/wakeshot.svg`).
6. **The main button downloads the x64 Setup directly** (`/releases/latest/download/<asset>`), with an ARM64 link and
   Releases beside it. Why: one click for most users. GitHub documents this link for uploaded assets; that `vpk`'s
   uploads count: not verified until the first stable release. Since 2026-11-07 these links show only once that
   release exists and has both Setup files (decision 11).
7. **Colours**: the Moss set in `docs/brand.md`, light and dark, picked by `p6-brand-apply-web` (its decisions 1 to
   4) so every pair passes WCAG AA, checked by `python site/check_page.py --colours site`. Change it when `brand.md`
   changes.
8. **The page is a summary that sends readers to the README** for detail. Menu names are in bold, as in the README.
   Since 2026-11-07, with the README reworked, links go to its headings (decision 16).
9. **The video gets controls, no autoplay, no mute** (2026-11-07: no mute added). Why: it is 38 s with sound, an
   autoplaying video that long needs a pause control (WCAG 2.2.2), and a muted start would make every viewer unmute
   by hand. `preload="none"`: its 8.5 MB load only for those who press play. Change it when Haik wants the video to
   autoplay muted, as `docs/research/launch.md` "Specs" suggests.
10. **Share tags and a social image** (`p6-brand-apply-web` decision 12, 2026-11-05): `og:*` and `twitter:*` tags
    with `site/social-card.png`, under the Pages address `https://haikasatryan.github.io/wakeshot/` (GitHub's project
    site pattern, not live until Pages is on); `check_page.py` checks them. Change it when a platform's card checker
    asks for more.

## Decisions (worker, 2026-11-07, for Haik's review)

11. **A build step decides what the page says, at deploy time.** `site/build_page.py` builds the folder Pages
    serves: `site/` without the repo's own files, plus each `media/<name>` the page names, taken from `media/out/` or
    `media/public/shots/` (no copy in `site/`, so a re-rendered video goes live at the next deploy). It asks GitHub
    whether the latest stable release has every Setup the page links, and whether winget-pkgs has the package, and
    keeps or drops the page's `<!-- if released -->`, `if not released` and `if winget` blocks. Why: the page goes
    live tonight, before any release, and must be honest then ("Not released yet", no download button that gives
    404), and right after Haik's release, when the Release run starts Pages, with nobody editing the page. Any GitHub
    answer but yes or no fails the deploy; so does a stable release that lacks a Setup the page links. Cost: 200
    lines of standard-library Python. Change it when the release and winget states stop changing (after v1 and
    the winget merge): drop the blocks and keep the build for the media.
12. **The video closes the hero**, under the buttons. Why: Haik's README order (what it does, then the video). The
    poster is `media/out/wakeshot-promo-poster.png`, 2.5 MB, so the first load is heavy (found outside scope). Change
    it when the media track writes a small poster.
13. **No icon loop GIF on the page.** Why: it is drawn on GitHub's page colours, so on the Moss page it shows a box
    (`docs/brand.md`, "Page and media"); it loops forever with no pause, which WCAG 2.2.2 does not allow on a page
    without scripts; and the video already shows the light coming on. The page shows the three state icons at the
    tray's 16 px, the menu and the holding tooltip, light and dark, as in the README. Change it when the media track
    renders a loop on the page's colours that stops after 5 s, or with a still for reduced motion.
14. **Comparison: the README's table, every row and cell as is.** Why: shortening a cell loses the nuance the README
    keeps fair to the others. No links to the rivals' sites (the page links only into the repo); the README links the
    sources. On a phone the table scrolls sideways, with a short hint shown only there.
15. **SmartScreen steps on the page now**, from the brand screen session (seen 2026-11-05: "Windows protected your
    PC", More info, Run anyway), instead of waiting for the first release. The README still has its own todo (found
    outside scope).
16. **Deep links into the README and docs, checked.** `check_page.py` checks every link into the repo: the file
    exists on this checkout, and a `#fragment` matches a heading id GitHub gives that Markdown file (the rule checked
    against GitHub's rendered README, 17 of 17). So a renamed heading or a removed doc (Phase 8 cleanup) fails the
    deploy instead of leaving a dead link. Change it when the page links a heading GitHub writes differently
    (setext headings, explicit anchors): extend `heading_ids`.
17. **Footer**: a trademark line ("Product names are trademarks of their owners; Wakeshot is not affiliated with
    them"), as the video's end card has, and the macOS line in the README's words (the draft's "planned for later"
    said more than `docs/product.md`).
18. **`check_page.py --final` fails on any file the page does not use**, and every `<img>` needs an `alt`. Why:
    nothing unplanned goes live.

## Facts checked (2026-11-04)

Moved to `docs/research.md`, "Pages page: release state, winget path, heading ids (2026-11-07)", on 2026-11-07, with
the new facts of that day.

## Found outside scope

Problems seen but not fixed.

- `README.md` Install: its SmartScreen todo says the words are not in `docs/research.md` yet; they are ("Brand screen
  session: what Windows shows", seen 2026-11-05). Owner: the README (Phase 8 polish).
- `media/out/wakeshot-promo-poster.png` is 2.5 MB of PNG; a JPEG or WebP poster at 1280 px would likely cut the page's
  first load by about 2 MB (estimate, not measured). Owner: the media track (`npm run poster`).
- The hero's speed line links `docs/research.md#speed-numbers-final-v1-build-2026-11-07`; the end card card that
  changes the numbers should point it at the section of the new numbers. A wrong anchor fails the deploy.
- `ci.yml` does not check `site/`; a broken page shows only at deploy time (decision 4).
- Resolved since 2026-11-04: the README's "hibernates" wording (it now says sleep, with hibernate only as the
  fallback); `docs/brand.md` names the page's colours.

## Log

- 2026-11-03: card written by `p6-plan-ship`.
- 2026-11-04 11:33: started as a draft (orchestrator `wakeshot-12`): build from what is final (brand, product.md,
  README draft); one marked comment per spot that waits for `p6-media`, `p5-haik-check`, `p2-6-sleep-route`, the
  first release and winget.
- 2026-11-04 11:37: checked facts (GitHub docs, gh api): `release.yml` publishes with the `GITHUB_TOKEN`, and GitHub
  starts no workflow from an event that token made, so `pages.yml` uses `workflow_run` on Release instead of
  `release: published`. Pages is off (API 404). Latest majors: `upload-pages-artifact` v5, `deploy-pages` v5.
- 2026-11-04 11:44: `site/wakeshot.svg` ported from `art.rs` at 64 px (matches the ICO's 64 px frame exactly);
  `site/favicon.ico` copied from `assets/wakeshot.ico`; `site/index.html` and `site/style.css` written. Orchestrator
  note: no rival table on the page, one marked spot for the comparison.
- 2026-11-04 11:45: `site/check_page.py` and `.github/workflows/pages.yml` written. actionlint 0 errors; the check
  0 errors; 16 broken page copies and 3 broken workflow copies tried (see Result).
- 2026-11-04 11:47: headless Edge shots (scratch profile, processes ended by themselves): light and dark at 1280 px,
  dark and light at 390 px. Fixed: table columns wrapping early, long Setup names breaking, a nested list gap, the
  uninstall line ("first" was wrong: uninstall removes start at login before it restores). Added the video spot.
- 2026-11-04 11:50: Result filled. Status: blocked, draft done.
- 2026-11-07 03:47: restarted to finish (orchestrator `wakeshot-4e`). The waiting cards are done: `p6-media` (final
  video approved), `p5-haik-check`, `p2-6-sleep-route`, the README rework. Haik's go for the night (2026-11-07
  ~02:39, relayed): "Like full polish of the repo ... with good page and so on"; he wants no questions overnight, so
  his one look before the page goes live is waived by that go, and every call here is for his review. The first
  public release and the winget submission are his in the morning: links point where they will work once those
  exist. Scope: `site/`, `pages.yml`, this card; `docs/research.md` additions only.
- 2026-11-07 03:53: read the reworked README, `media/README.md`, `release.yml`, research (SmartScreen seen
  2026-11-05). GitHub state (`gh api`): no releases at all, Pages off, no environments; every media file the page
  needs is on `main`; winget-pkgs has no `manifests/h/HaikAsatryan/Wakeshot` yet (404; the path pattern checked on
  `manifests/m/Microsoft/PowerToys`). Plan: a build script assembles the Pages folder (site plus the media files,
  never a copy in `site/`) and keeps the download or the "not released yet" text by what is out on GitHub at deploy
  time, so the page is honest tonight and right after the release with no edit.
- 2026-11-07 04:02 (this line and the next were first stamped 04:20 and 04:31 by mistake, without reading the
  clock; both happened between 03:53 and 04:02): written: `site/build_page.py` (new), `check_page.py` (source and built folders, repo deep links
  and headings, img alt, `--final` fails on unbuilt blocks and unused files), `index.html` (every waiting spot
  filled: video, states, menu and tooltip pictures, comparison, SmartScreen words, release and winget blocks),
  `style.css`, `pages.yml` (build, check, upload; the `rm` step gone). Source check 0 errors; all four built
  variants and the one built from GitHub's answer (released no, winget no) pass `--final` with 0 errors.
- 2026-11-07 04:02: 33 broken copies and faked GitHub answers (script in the scratch folder) all fail or pass as
  they must. actionlint 1.7.12 on `pages.yml`: 0 errors (the copy in session `ca4fed46` scratch that earlier cards
  checked against the release checksum list; a fresh download was refused by auto mode).
- 2026-11-07 04:05: headless Edge renders (own throwaway profiles, every process gone after; files in scratch, deleted
  after the look): tonight's page and the released one with winget, light and dark, 1280 px and a 390 px frame.
  Fixed: the menu and tooltip pictures wrapped (gap), the comparison's wording; added a phone-only scroll hint. The
  first "light" renders came out dark: headless Edge follows the Windows mode unless told (research).
- 2026-11-07 04:08: research additions (the new facts and the 2026-11-04 ones from this card); every check run once
  more, all green. Scratch outputs deleted. Status: done.

## Result

Status: done (2026-11-07 04:09). Haik's one look before go-live is waived by his 02:39 go; every decision above is for
his review.

**Done.** The page is final for tonight and needs no edit at the release. Every waiting spot is filled: the 38 s
video (controls, sound, poster) closes the hero; "What you see" shows the three tray states at 16 px, the menu and
the holding tooltip, light and dark; "How it compares" is the README's table; the SmartScreen steps are the words seen
2026-11-05; menu words and the sleep story match the shipped app and the README. `site/build_page.py` builds what
Pages serves: the page without the repo's own files, the media files taken from `media/` at build time (no copies in
`site/`), and the blocks that depend on GitHub: before a stable release "Not released yet" and no download button;
after it the x64 download button and the ARM64 link; once winget-pkgs has the package, the `winget install` line.

**What the page shows tonight** (GitHub today: no release, no winget package): "Not released yet. The first release
is on its way", buttons to the README and the source, the video, and the rest of the page.

**What changes when the release is out:** nothing to edit. A successful Release run of a stable tag (no `-`) starts
`pages.yml` on its own; the build sees the release and shows the download buttons. A pre-release changes nothing
(Pages does not run; `/releases/latest` skips pre-releases). Once winget merges the package, run `gh workflow run
pages.yml --ref main` once, and the winget line appears. A re-rendered video goes live at the next deploy.

**Files:**

- New: `site/build_page.py`.
- Changed: `site/index.html`, `site/style.css`, `site/check_page.py`, `.github/workflows/pages.yml`, this card,
  `docs/research.md` (additions only: section "Pages page: release state, winget path, heading ids (2026-11-07)").
- Deleted: none. Unchanged from the draft: `site/wakeshot.svg`, `site/favicon.ico`, the lockups, the social card and
  its sources, `site/fonts/`.

**Checks:**

| Check | Outcome |
|---|---|
| `actionlint` 1.7.12 on `pages.yml` | 0 errors (the checksum-checked copy from session `ca4fed46`; a new download was refused by auto mode) |
| `python -B site/check_page.py --colours site` (source: HTML parses, every local link and image resolves, media files found where the build takes them, no `src` or `href` outside the repo and the page, every repo link's file and `#heading` exist, no em dash or emoji, contrast) | 0 errors, 0 spots wait; lowest contrast 5.03 (step number), all pairs WCAG AA in both themes |
| `build_page.py` for all four states (released yes or no, winget yes or no), each then `check_page.py --final` | 0 errors each, 19 files each |
| `build_page.py` asking GitHub (unsigned), then `--final` | released no, winget no; 0 errors |
| 33 broken copies and faked GitHub answers (`broken_copies.py` in the scratch folder): bad README or docs anchor, missing repo file, img without alt, unbuilt block, waiting spot, missing media, em dash, emoji, other host, look-alike repo, script, onclick, unclosed tag, bad `#id`, stray file, unclosed, nested, unknown and inline markers, a release without the ARM64 Setup, HTTP 502, no network, a non-empty output folder | 33 of 33 as expected |
| GitHub's heading ids against the script's (`gh api .../readme`, HTML) | 17 of 17 equal |
| `node media/scripts/agent-lists.mjs --check` (the agent list markers on the page) | ok, lists agree |
| Headless Edge renders, light and dark, 1280 px and 390 px, tonight's page and the released one | readable, nothing overflows; the comparison scrolls inside its box on a phone |
| No `__pycache__` left in `site/` | none (both scripts set `dont_write_bytecode`) |

Preview it yourself: `python site/build_page.py --released yes --winget no <new folder>`, then open `<new
folder>\index.html` (opening `site\index.html` directly shows both sides of every block and no media).

**The orchestrator's steps (after the commit and push; `pages.yml` must be on `main`):**

1. Turn Pages on with source "GitHub Actions":
   `gh api -X POST repos/HaikAsatryan/wakeshot/pages -f build_type=workflow`
   Check: `gh api repos/HaikAsatryan/wakeshot/pages --jq '.build_type, .html_url'` prints `workflow` and
   `https://haikasatryan.github.io/wakeshot/`. If the POST is refused for want of `source`: not verified; add
   `-f "source[branch]=main" -f "source[path]=/"` and run it again.
2. Deploy tonight's page: `gh workflow run pages.yml --ref main`, then `gh run watch` on that run
   (`gh run list --workflow pages.yml -L 1`). The build log's first line says `build: released no, winget no`.
3. If the deploy job is refused by the `github-pages` environment: Settings > Environments > `github-pages`, allow
   `main`.
4. Checks on the live page (each should pass):
   - `curl -sI https://haikasatryan.github.io/wakeshot/` gives 200.
   - 200 for `style.css`, `wakeshot.svg`, `favicon.ico`, `social-card.png`, `fonts/InstrumentSans-SemiBold.woff2`,
     `media/wakeshot-promo.mp4` (`content-type: video/mp4`), `media/wakeshot-promo-poster.png`,
     `media/readme-menu-light.png`, `media/state-holding-dark.png`.
   - 404 for `check_page.py`, `build_page.py`, `social-card.html`, `wakeshot-lockup-light.svg`.
   - `curl -s https://haikasatryan.github.io/wakeshot/ | grep -c "Not released yet"` gives 2, and
     `grep -c "releases/latest/download"` gives 0.
5. After Haik's first stable release: the Pages run starts by itself after the Release run. Then the same page
   gives 0 for "Not released yet" and 2 for `releases/latest/download`, and
   `curl -sIL https://github.com/HaikAsatryan/wakeshot/releases/latest/download/HaikAsatryan.Wakeshot-win-x64-Setup.exe`
   ends in 200 (this settles decision 6). If the Release run failed only in its winget job, or Pages was turned on
   after the tag: `gh workflow run pages.yml --ref main`.
6. Setting the page as the repo's website is Haik's call: `gh repo edit HaikAsatryan/wakeshot --homepage
   https://haikasatryan.github.io/wakeshot/`.

**Open items:** the four under "Found outside scope" (README SmartScreen todo, the 2.5 MB poster, the speed line's
anchor when the numbers change, `ci.yml` not checking `site/`). Not verified until the first runs: the Pages POST
without `source`, the `GITHUB_TOKEN` reading winget-pkgs, `vpk` assets under `/releases/latest/download/`.
