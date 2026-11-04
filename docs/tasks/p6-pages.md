# GitHub Pages page

Phase: 6. Status: blocked: draft done; waits for `p6-media`, `p5-haik-check`, `p2-6-sleep-route`, the README rework
and the first public pre-release. Needs Haik: yes, one look at the page before it goes live at v1 (the orchestrator relays
it). Agent: `wakeshot-worker`. Size: about 60K tokens.
Real laptop: no. No screen use: checks are scripts; Haik's look is the visual check.
Runs after: `p6-readme`; the tray track's `p5-icon-art` and `p5-haik-check` (Haik's pick there rewrites
`assets/wakeshot.ico`; review m3). Parallel with: any card that does not own `site/` or `.github/workflows/pages.yml`.
Build folder: none (no Rust).

## Goal

One small static page at v1 (`product.md`, Distribution) that says what Wakeshot does and links to the download,
built from the README and deployed only when a release is published.

## Read first

- `README.md` (`p6-readme` Result); `docs/product.md`, "The job" and "Distribution".
- `docs/architecture.md` section 10 (action pinning, read-only tokens).
- The tray track's icon art and where it lives (its card's Result).

## Deliverables

- `site/index.html`, `site/style.css`, the icon (copied from the tray track's art). One page: what it does, a picture
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
4. **The checks live in `site/check_page.py`** (Python standard library) and `pages.yml` runs it with `--final`
   before every upload, then removes it from the upload. Why: a future worker reruns the same checks after the
   waiting cards, and a page with a waiting spot can never go live. Python, not PowerShell like `packaging/`, because
   it has an HTML parser built in. Change it when `ci.yml` (not owned here) should check the page on every push: add
   `python3 site/check_page.py site` there.
5. **Header icon as SVG** (`site/wakeshot.svg`), drawn with the shapes and numbers of `art.rs` at 64 px. Why: the
   largest ICO frame is 64 px, which turns soft at 96 CSS px on a high DPI screen; the SVG is 1 KB and sharp at any
   size. The port was checked: rasterised the way `art.rs` does, it equals the ICO's 64 px frame in every pixel
   (0 difference over 4096 pixels). The favicon is `assets/wakeshot.ico` copied as is (its 16 to 32 px frames are
   pixel-snapped, sharper than an SVG there). Change it when the art changes (redo both), or when `art.rs` gains an
   SVG writer (then `write_art_files` should write `site/wakeshot.svg`).
6. **The main button downloads the x64 Setup directly** (`/releases/latest/download/<asset>`), with an ARM64 link and
   Releases beside it. Why: one click for most users. GitHub documents this link for uploaded assets; that `vpk`'s
   uploads count: not verified until the first stable release.
7. **Colours beyond `docs/brand.md`**, picked so every text pair passes WCAG AA by a wide margin (script,
   2026-11-04): light muted text `#55526A` (7.5 on paper), edge `#DDDAE8`; dark text `#ECEAF4` (15.5 on night),
   muted `#B4AFCB` (8.8), panel `#1F1C2E`, edge `#34304A`; links and buttons as the brand says (6.7 and 7.5); focus
   ring violet on light, sun gold on dark (12.3). Gold is only a highlight: the step number discs (ink on gold
   11.6) and a short horizon line under the header. Change it when `brand.md` names dark page colours.
8. **The page is a summary that sends readers to the README** for detail (links to `#readme`, no deep anchors,
   since the README is being reworked). Menu names are in bold, as in the README.
9. **The video gets controls, no autoplay** (in its marker). Why: it is 30 s with sound, and an autoplaying loop that
   long needs a pause control (WCAG 2.2.2). The icon loop gets a still for `prefers-reduced-motion`. Change it when
   Haik wants the video to autoplay muted, as `docs/research/launch.md` "Specs" suggests.
10. **No Open Graph tags or social image.** Why: launch posts link the repo, and an `og:image` needs the page's own
    absolute URL, known only once Pages is on. Change it when the page link is shared on its own.

## Facts checked (2026-11-04)

For `docs/research.md` once the docs pass (round 5) is done with it; that file is not this card's to edit now.

- GitHub docs ("Triggering a workflow", read with WebFetch): "events triggered by the `GITHUB_TOKEN` will not create
  a new workflow run"; `workflow_dispatch` and `repository_dispatch` always do.
- GitHub docs ("Events that trigger workflows"): `workflow_run` fires only if the workflow file is on the default
  branch; its `GITHUB_SHA` is the last commit on the default branch and `GITHUB_REF` the default branch; at most
  three levels of chaining. `release: published` also fires for pre-releases.
- For a run started by a tag push, the run's `head_branch` is the tag name: `gh api` on `BurntSushi/ripgrep`
  (`15.2.0`) and `rhysd/actionlint` (`v1.7.12`) runs.
- Action versions (`gh api` releases and `action.yml` at the major tag): `actions/upload-pages-artifact` v5.0.0
  (inputs `name`, `path`, `retention-days`, `include-hidden-files`; it leaves out dotfiles by default),
  `actions/deploy-pages` v5.0.1 (output `page_url`, Node 24), `actions/checkout` v7.0.1; `actions/configure-pages`
  v6.0.0 is not needed for a plain HTML site (not required by the docs).
- GitHub docs ("Using custom workflows with GitHub Pages"): the deploy job needs at least `pages: write` and
  `id-token: write`; the default environment is `github-pages`.
- The `github-pages` environment refusing a run on a tag: reported by users (community discussion 39054 and failing
  runs found by search), not in GitHub's docs, not tested by us.
- REST "Create a GitHub Pages site": `build_type` is `legacy` or `workflow`; the caller must be an admin or
  maintainer; a classic token needs `repo`. Whether `source` may be left out with `workflow`: the docs do not say;
  not verified until the orchestrator runs the command.
- `HaikAsatryan/wakeshot`: Pages is off (`GET /pages` gives 404), issues are on, default branch `main`.
- GitHub docs ("What is GitHub Pages"): "When a GitHub Pages site is visited, the visitor's IP address is logged
  and stored for security purposes." Project site address: `https://<owner>.github.io/<repository>`, so
  `https://haikasatryan.github.io/wakeshot/` (not verified until Pages is on).
- GitHub docs ("Linking to releases"): `/releases/latest` and `/releases/latest/download/<asset>` for manually
  uploaded assets.
- Runner image readme (`actions/runner-images`, Ubuntu 24.04, image 20261026): Python 3.12.3. Which Ubuntu
  `ubuntu-latest` maps to today: not checked; Python 3 is on both.
- actionlint 1.7.12 limits: it flags a bad permission scope, but not a wrong `upload-pages-artifact@v5` input or a
  wrong `github.event.workflow_run` field (broken copies passed). Those two were checked by hand above.
- Headless Edge: `--window-size=390,...` did not narrow the page (the shot was cut on the right); a 390 px iframe in
  a wider window did. Observed; the cause is not verified.

## Found outside scope

Problems seen but not fixed.

- `README.md` still says Wakeshot "hibernates" the closed laptop after a work end, after a failed lock and at the
  battery floor (story steps 3 and 5; "Lid closed, lock and battery"), while `docs/product.md` says sleep since
  2026-11-04. The README rework or the docs pass owns it; the page already says sleep, marked for
  `p2-6-sleep-route`.
- `docs/brand.md` names no dark page text, muted or panel colours; the page picked them (decision 7). The brand owner
  may want to record them.
- `ci.yml` does not check `site/`; a broken page shows only at release time (decision 4).

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

## Result

Status: blocked: draft done; waits for `p6-media`, `p5-haik-check`, `p2-6-sleep-route`, the README rework and the
first public pre-release (orchestrator decision, session `wakeshot-12`).

**Done.** One static page from what is final: the brand, `docs/product.md` and the README draft. It says what
Wakeshot does (holds, the tray states), a night with it (lid, lock, sleep, battery), what it changes on the PC,
install, updates, uninstall and privacy, with links to Releases, the README, issues and the license, all on
`github.com/HaikAsatryan/wakeshot`. Light and dark through `prefers-color-scheme`. No scripts, no trackers, system
fonts only. No rival table (orchestrator note); one marked spot waits for the README rework. Ten design decisions
are above, each with its reason and when to change it.

**Files** (all new):

- `site/index.html`, `site/style.css`
- `site/wakeshot.svg` (the app icon as shapes, ported from `art.rs`), `site/favicon.ico` (copy of
  `assets/wakeshot.ico`)
- `site/check_page.py` (the acceptance checks; not uploaded to Pages)
- `.github/workflows/pages.yml`

**Checks:**

| Check | Outcome |
|---|---|
| `actionlint` 1.7.12 on `pages.yml` (release zip, SHA-256 matches its checksum list) | 0 errors. A broken copy with a bad permission is flagged; a bad action input and a bad event field are not (actionlint limits, checked by hand instead) |
| `python site/check_page.py site`: the HTML parses into balanced tags, every local link, image and CSS `url()` resolves, no `src` or `href` leaves `https://github.com/HaikAsatryan/wakeshot` and the page | 0 errors, 5 files, 9 spots wait |
| The same script: no em dash, no emoji (literal or as an entity) | 0 found |
| 16 broken copies of the page (em dash, `&mdash;`, emoji, other host, look-alike repo, `//` host, missing file, bad `#id`, `<script>`, `onclick`, unclosed and misnested tags, a path out of `site/`, a font `@import`, a CSS `url()` to another host, no dark mode) | each fails with the right error |
| `--final` (what `pages.yml` runs) | fails on the 9 waiting spots, as it must until they are filled |
| Contrast of every text colour pair (script) | all pass WCAG AA, lowest 6.05 (light link on a panel) |
| Headless Edge shots, light and dark, 1280 px and 390 px | readable; nothing overflows at 390 px. Haik's look is still the real visual check |

**What to redo after each card** (each spot is an HTML comment that starts with `waits for`):

1. `p6-media`: the icon loop picture in the header (with a still for reduced motion), the speed numbers line, the
   pictures of the three icon states and of the menu, and the 30 second video; files go into `site/media/`, never
   another host.
2. `p5-haik-check`: the menu words (two spots: Keep awake, Keep screen on, Automatic, Restore my power settings) and
   the tooltip line. If the icon art changes: redo `site/favicon.ico` (copy) and `site/wakeshot.svg` (the numbers
   of `concept_app_icon` and `risen_sun` at size 64, checked against the new 64 px frame).
3. `p2-6-sleep-route`: the story's steps 3 and 5 say "goes to sleep"; check them against the built behaviour.
4. The README rework (`p6-readme`): the comparison spot, a short feature side by side from the reworked README.
5. The first public pre-release (`v0.1.0-rc.1`): the exact SmartScreen words and steps.
6. After v1, Phase 8: the winget line (its comment starts with `after v1`, so it does not block `--final`).
7. Then: `python site/check_page.py --final site` gives 0 errors; actionlint again; Haik's look.

**Haik's look, before it goes live** (the orchestrator relays it at v1): open
`C:\Users\haika\code\wakeshot\site\index.html` in Edge (double-click it). Read it top to bottom. Then switch Windows
to the other mode (Settings > Personalization > Colors > Choose your mode), press F5 and look again. Say yes, or what
to change.

**The orchestrator's step at v1, not before:**

1. `pages.yml` must be on `main`. Right before pushing the `v1.0.0` tag, turn Pages on with source "GitHub
   Actions": `gh api -X POST repos/HaikAsatryan/wakeshot/pages -f build_type=workflow`. Check it:
   `gh api repos/HaikAsatryan/wakeshot/pages --jq .build_type` prints `workflow`.
2. Push the tag. When the Release run succeeds, the Pages run starts by itself and deploys.
3. If Pages was turned on after the tag, or the Release run failed only in its winget job:
   `gh workflow run pages.yml --ref main`.
4. If the deploy job is refused by the `github-pages` environment: Settings > Environments > `github-pages`, and
   allow `main`.
5. The page should be at `https://haikasatryan.github.io/wakeshot/` (`gh api repos/HaikAsatryan/wakeshot/pages --jq
   .html_url`). Setting it as the repo's website is Haik's call (`gh repo edit --homepage <url>`).

**Open items:** the facts above go into `docs/research.md` after the docs pass; the three items under "Found outside
scope".
