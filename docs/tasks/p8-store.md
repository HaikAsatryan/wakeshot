# Microsoft Store listing and automated submission

Phase: 8. Status: blocked: signing is not free yet (`docs/research/launch.md` part 3, "Recommendation"). Haik's rule
(`product.md`, Distribution): the Store only if adding it and signing are free and the submission runs in our release
flow without hands, and uninstall still restores. Haik (2026-11-04 01:14): no paid certificate, free paths only for
signing and distribution. So the card re-opens only when SignPath Foundation approves Wakeshot; then the EXE path
only, never MSIX (uninstall must restore).
Needs Haik: yes: creates the Store developer account and the app name reservation (his identity), and reads the
listing before the first submission. Agent: `wakeshot-worker`. Size: about 80K tokens, to set again after the
research.
Real laptop: unknown until the research; installing the Store build to check the uninstall restore would be yes.
Runs after: SignPath Foundation approves Wakeshot (free signing); `p6-launch-research` (done); `p6-media`
(screenshots, video); `p6-readme` (the words); the winget submission.
Parallel with: any card that does not own the files below.

## Goal

Wakeshot in the Microsoft Store with a listing as good as the README (Haik, 2026-11-04), submitted by the release
workflow, without fighting winget or the app's own updates.

## Read first

- `docs/research/launch.md` part 3 and the Result of `p6-launch-research` (Store path, signing, automation, how
  winget, the Store and Velopack share one install).
- `docs/product.md`, "Distribution"; `docs/phases/06-ship.md`, "Release".

## Deliverables

To be set from the research Result before the card starts. At least:

- `packaging/store/`: the listing text (description, features, what it changes, privacy), the asset list from
  `media/`, and how to submit.
- The submission step in `.github/workflows/release.yml`, with the credentials in repo secrets.
- A privacy policy page if the Store needs one (the Pages page can hold it; `p6-pages`).

## Acceptance checks

To be set from the research. At least: an uninstall of the Store build during a hold puts the user's power settings
back; `winget upgrade --all` and a Store update at the same time leave one working install of the newest version.

## Out of scope

- Signing setup itself (SignPath application) unless the research makes it part of this card.

## Questions for Haik

Each with the worker's default.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-04: card written, blocked, from Haik's rule (Store if free and automated) and launch request (good Store
  documentation and pictures).

## Result

What was done, files changed, checks and outcomes, open items.
