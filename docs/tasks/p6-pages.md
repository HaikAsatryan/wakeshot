# GitHub Pages page

Phase: 6. Status: ready. Needs Haik: yes, one look at the page before it goes live at v1 (the orchestrator relays
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

Each with the worker's default.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-03: card written by `p6-plan-ship`.

## Result

What was done, files changed, checks and outcomes, open items.
