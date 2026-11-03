# Launch research: rivals, winget with self-update, Microsoft Store, media

Phase: 6. Status: ready. Needs Haik: yes: he reads the Result (rival pick, Store recommendation) before the README,
winget and Store cards are rewritten. Agent: `wakeshot-worker`. Size: about 110K tokens.
Real laptop: no. No installs of any rival or store app; downloads go to `<scratchpad>\p6-launch-research\` and are
never run (Nightcrew and similar tools change the same power settings, `CLAUDE.md`).
Runs after: none. Parallel with: any card (owns only the files below).
Build folder: none (no Rust).

## Goal

Haik wants the launch to sell Wakeshot (2026-11-04): a short README that says what we solve, a side by side table
against the 2 or 3 most-starred rivals, pictures and a short video of the icon, winget that never fights the app's
own updates, and a good Microsoft Store listing. This card finds the facts first, so the README, winget and Store
cards are written on facts, not memory.

## Read first

- `docs/product.md`: "The job", "Platform and stack", "Distribution".
- `docs/research/rivals-nightcrew-lidguard.md`, `docs/research/rivals-others.md` (rivals already studied).
- `docs/research/distribution.md` sections 1 and 2 (Velopack, winget), and its Store line in `product.md`.
- `docs/phases/06-ship.md`: "Package and install values", "Updates", "Not in this track".
- `docs/tasks/p6-readme.md`, `p6-winget-manifest.md`, `p6-pages.md` (what is planned now).

## Deliverables

`docs/research/launch.md`, owned whole; short facts with how they were checked added to `docs/research.md` (additions
only, re-read right before each edit). Each part ends with "What this means for Wakeshot".

1. Rivals for the README table.
   - Candidates: the rivals in the two rival reports plus a fresh `gh search repos` for Windows keep-awake and
     agent-aware tools. Star counts from `gh api repos/<owner>/<repo>` with the date read.
   - Pick 2 or 3, with the reason: at least the most-starred agent-aware tool on Windows and the most-starred general
     keep-awake tool (PowerToys Awake counts; say that its stars are for all of PowerToys).
   - For each: stack and runtime it needs, download and install size (release asset sizes, no run), idle RAM and CPU
     only where the project publishes them (else "not published"), license, price, and features: which agents it
     detects, lid closed, which settings it writes and whether it puts them back, crash safety, battery safety, auto
     update, per-user or admin install.
   - Wakeshot's own row: the facts from `product.md`, `architecture.md` and the release exe sizes from a local
     `cargo build --release` (own `CARGO_TARGET_DIR`, `--locked`). Speed is measured later, on the integrated app, by
     `p6-media`: name the method for each number the README shows (Haik wants readers to see it is very fast): idle
     RAM (working set), CPU time over 10 minutes idle and during a hold, exe and install size, and how long from
     Claude Code starting work to the hold starting, and from its end to the hold ending (log timestamps).
   - No claim the facts do not back. Every rival cell has a source link.
2. winget with an app that updates itself.
   - How winget decides an upgrade is available (installed ARP `DisplayVersion` against the manifest), and what breaks
     for self-updating apps. Find the real winget-cli issues behind the VS Code and Docker Desktop cases Haik saw
     (`winget upgrade` that does not reach the latest, or loops), with links.
   - Manifest fields that help, checked in the current schema: `RequireExplicitUpgrade`, `UpgradeBehavior`,
     `AppsAndFeaturesEntries`, `ProductCode`, and any other.
   - What happens when `winget upgrade` runs Setup.exe while Velopack applies an update, or the app is running: does
     Velopack lock the install (mutex, lock file; read the Velopack source), what wins, can the install break.
   - The lag between our GitHub release and the merged winget-pkgs PR, and whether a release workflow can open the PR
     itself (Komac or `wingetcreate` in a GitHub Action, the token it needs).
   - Output: the rules for `p6-winget-manifest` and `release.yml`, so there is one updater at a time and winget never
     shows a stale or looping upgrade.
3. Microsoft Store. Haik's rule (`product.md`, Distribution): we go to the Store too if adding it and signing are free
   and the submission runs in our release flow without hands; uninstall must still restore. Answer each condition.
   - Current Store rules for a Win32 app: MSIX against an EXE or MSI submission. For an EXE submission: does the Store
     run our `Setup.exe` and its uninstall (so the Velopack uninstall hook restores power settings), must the installer
     be signed and by what kind of certificate, may the app update itself, what the installer URL must look like, and
     what an individual developer account costs today. For MSIX: the Store signs it for free, but there is no
     uninstall hook; is there any safe way around that.
   - Free: account fee; signing cost for each path (Store-signed MSIX, SignPath Foundation for the EXE, its approval
     rules and when we qualify, `docs/research/distribution.md` section 3).
   - Automated: the Store submission API or the Microsoft Store Developer CLI (`msstore`) from a GitHub Action, the
     credentials it needs and where they live (repo secrets), whether a new version needs a human review each time and
     how long it takes.
   - How winget's `msstore` source shows a Store app; whether one PC can see the same app twice (Store id and
     `HaikAsatryan.Wakeshot`) and what `winget upgrade --all` then does; the case Haik saw, where `winget upgrade --all`
     and a Store update ran at once and one waited for the other.
   - Listing needs: icon sizes, screenshot sizes and count, trailer video specs, description limits, age rating,
     privacy policy URL (a Pages page counts?).
   - Output: a recommendation (Store yes or no, at launch or after signing) and, if yes, how winget, Store and Velopack
     share one install without fighting.
4. Pictures, animation and video. Haik wants (2026-11-04) a 30 second promo video with sound and effects, animated
   by Claude in code (not a screen recording), plus pictures, and a README that is long and detailed but never boring.
   - What the picked rivals and well-liked small tools show in their READMEs and listings (screenshots, GIFs, video),
     and how good long READMEs stay easy to read (top part, contents, folded `<details>` sections, tables).
   - What GitHub renders in a README: GIF, animated WebP or PNG, MP4 (in-repo file against an uploaded attachment, with
     sound or not), size limits, light and dark images (`<picture>` with `prefers-color-scheme`).
   - The tool for the video, built from code so it can be rebuilt when the icon changes: compare Remotion, Motion
     Canvas and any other current option on license (Remotion has its own license: check that a personal free project
     is covered), output (MP4 with audio, GIF), Windows support, and how well an agent can drive it. Pick one.
   - Sound: music and effects with a license that allows use in a free MIT project's promo (CC0 or similar), or
     Wakeshot's own chimes (`p5-sounds`); where to get them; credit lines needed.
   - Specs: GitHub, the Store trailer and screenshots (sizes, length, format), the Pages page.
   - How to capture the real tray area and menu on Windows 11 by script (ffmpeg `gdigrab` or a Windows tool), at
     100 % and 150 % scale, light and dark, for the screenshots and as source for the animation.
   - Output: the storyboard and shot list for `p6-media`. The story: Claude Code starts work and the icon goes to
     working; the work ends and the icon goes idle; the user's own power settings are back; Keep awake from the menu;
     the speed numbers. The video shows only what Wakeshot really does.

## Acceptance checks

- Every fact has a link or a command and the date it was read; anything not checked says "not verified".
- Star counts and sizes are read with `gh` or the release API in this card, not taken from the old reports.
- The Result lists: the rival pick with reasons; the winget rules; the Store answer for each of Haik's conditions
  (free, automated, safe uninstall) and the recommendation; the video tool pick; the storyboard and shot list; and
  the exact changes it proposes to `p6-readme`, `p6-media`, `p6-winget-manifest`, `p6-pages`, `p8-store` and
  `product.md`, for the orchestrator to route after Haik reads it.
- `docs/research/launch.md` has no em dash and no emoji (script).

## Out of scope

- Writing the README, the manifest, the Pages page or a Store listing; any code; any install or account sign-up.
- Changing `product.md` or a phase file: proposals go in the Result.

## Questions for Haik

Each with the worker's default.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-04: card written from Haik's launch request (README that sells, rival table, macOS later, pictures and
  video, winget that never fights the app's updates, Store listing). Haik's answers the same day: Store if free and
  automated; README long but never boring, showing speed; a 30 s animated video with sound and effects; no rival
  installs.

## Result

What was done, files changed, checks and outcomes, open items.
