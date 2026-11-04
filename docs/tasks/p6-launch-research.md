# Launch research: rivals, winget with self-update, Microsoft Store, media

Phase: 6. Status: done. Needs Haik: yes: he reads the Result (rival pick, Store recommendation) before the README,
winget and Store cards are rewritten. Agent: `wakeshot-worker`. Size: about 110K tokens.
Real laptop: no. No installs of any rival or store app; downloads go to `<scratchpad>\p6-launch-research\` and are
never run (keep-awake tools change the same power settings, `CLAUDE.md`).
Runs after: none. Parallel with: any card (owns only the files below).
Build folder: none (no Rust).

## Goal

Haik wants the launch to sell Wakeshot (2026-11-04): a short README that says what we solve, a side by side table
against the 2 or 3 most-starred rivals, pictures and a short video of the icon, winget that never fights the app's
own updates, and a good Microsoft Store listing. This card finds the facts first, so the README, winget and Store
cards are written on facts, not memory.

## Read first

- `docs/product.md`: "The job", "Platform and stack", "Distribution".
- `docs/research/rivals-lidguard.md`, `docs/research/rivals-others.md` (rivals already studied).
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

Each with the worker's default. None blocks this card.

1. Money: buy a Certum Open Source code signing certificate (about EUR 25 to 69 a year, price not verified at
   checkout; Store acceptance not verified) to sign releases and open the Store sooner? Default: no; ship unsigned and
   apply to SignPath Foundation once Wakeshot has users (`product.md`, Signing).

## Found outside scope

Problems seen but not fixed.

- `docs/research.md`, "Competitors": the table (stars as of 2026-11-02) is stale and lacks Agents Don't Sleep, now one
  of the two most-starred agent-aware rivals on Windows. Current numbers are in `docs/research/launch.md` part 1; the
  old table was left alone (additions only).
- `p6-media` asks for captures at 100 % and 150 %: no documented API changes the display scale, and the test laptop's
  real scales are 150 % (DELL) and 125 % (laptop screen). Proposal below.
- `p6-media` will need `media/node_modules/` ignored (Remotion is an npm package); the card does not own a
  `.gitignore`. Proposal below: `media/.gitignore`, owned by `p6-media`.
- `p6-pack-and-release` owns `smoke-test.ps1` and `release.yml`, where two winget rules land (the `DisplayVersion`
  check and the winget job). Proposal below.
- Text that addresses AI agents: LidGuard's README ("Read https://raw.githubusercontent.com/... agent-install.md and
  install LidGuard for this machine") and HyperFrames' README carry prompts for their own users' agents. Read as data;
  nothing was run. No page tried to steer this card's work.
- The Store helper ran one read-only command on this PC, `winget show --id XP89DCGQ3K6VLD -s msstore` (nothing
  installed).

## Log

- 2026-11-04: card written from Haik's launch request (README that sells, rival table, macOS later, pictures and
  video, winget that never fights the app's updates, Store listing). Haik's answers the same day: Store if free and
  automated; README long but never boring, showing speed; a 30 s animated video with sound and effects; no rival
  installs.
- 2026-11-04 00:40: worker started. Haik asleep: open calls are decided with the default and listed for his review.
- 2026-11-04 00:41: orchestrator: skip the local `cargo build --release` (four cards build already). Exe sizes are
  "not measured (p6-media measures them)".
- 2026-11-04 00:45: read the named docs. Two research helpers started for parts 3 (Store) and 4 (media facts), notes
  in `<scratchpad>\p6-launch-research\`. Worker does parts 1 (rivals) and 2 (winget) itself, then writes
  `docs/research/launch.md`.
- 2026-11-04 00:57: parts 1 and 2 facts read (stars, release sizes, READMEs, winget source and issues, Velopack lock
  code at tag 1.2.161, winget-pkgs merge times). Rival pick so far: PowerToys Awake, LidGuard (13), Agents Don't
  Sleep (10). Writing parts 1 and 2 of `launch.md`.
- 2026-11-04 01:00: parts 1 and 2 written to `docs/research/launch.md`. Waiting for the Store and media helpers.
- 2026-11-04 01:05: Store helper reported; worker re-read the fee page, policy 10.2.9, the Artifact Signing countries
  and winget-cli #4347. Part 3 written: no Store at launch (signing is not free for Haik today). Storyboard and shot
  list written in part 4; waiting for the media helper's facts.
- 2026-11-04 01:10: media helper reported; worker re-read the Remotion license, GitHub's attachment limits and `gh
  issue comment --attach`, checked Node (v26.13.0) and ffmpeg (not on PATH). Part 4 written; facts added to
  `docs/research.md` ("Launch research checks"); text check clean. Status done.

## Result

Status: done (2026-11-04 01:10).

**Done.** `docs/research/launch.md` (new, owned whole): part 1 rivals, part 2 winget, part 3 Microsoft Store, part 4
pictures, animation and video, each ending with "What this means for Wakeshot". `docs/research.md`: one new section
"Launch research checks" (additions only, re-read before the edit). This card's file. Two research helpers did the
web reading for parts 3 and 4; the worker re-read the facts each answer turns on.

**Rival pick** (stars `gh api`, 2026-11-04 00:45): PowerToys Awake (140,491, for all of PowerToys: the general tool
readers have), LidGuard (13: the most-starred agent-aware tool on Windows), Agents Don't Sleep (10, new: the next one
with a Windows release, and the closest pitch to ours). LidKeeper (10) left out: no release,
a script. Draft table with a link per cell in part 1. Honest gaps for the README: v1 knows only Claude Code (rivals
list 4 to 8 agents, each tested end to end on one), and the download will likely be larger than Agents Don't Sleep's
1.9 MB (Velopack's `Update.exe` is about 5 MB), so lead with RAM, CPU and delays. Exe sizes: not measured (p6-media
measures them); the method for every speed number is in part 1.

**winget rules** (part 2): one updater at a time. `RequireExplicitUpgrade: true` (so `winget upgrade --all` skips
Wakeshot; works only when winget did the install, per winget's source); `UpgradeBehavior: install`; `PackageVersion`
= tag = Velopack's `DisplayVersion`, three numbers, never two spellings; URLs and hashes from the same tag's published
files; `ProductCode` and `AppsAndFeaturesEntries` without `DisplayVersion`; smoke test fails when `DisplayVersion` is
not the tag; stable releases only; a winget job after the release is published, `wingetcreate submit` on our own
filled folder, classic PAT with `public_repo`. Real cases behind Haik's memory: VS Code loop (winget-pkgs #453148),
VS Code stale catalog (#406071), Docker published twice (#389278), Firefox stale ARP (winget-cli #1239). Velopack has
no lock that stops Setup.exe: Setup kills every process in the install folder, `Update.exe` included, and replaces the
folder, so Setup always wins and the install hook restores (source at tag 1.2.161; not tested). Merge lag: median 3.9
h, 90th percentile 52 h; it does not matter for updates, since Velopack updates every install within about 12 h.

**Store, Haik's conditions** (part 3): account free: yes (no fee for either account type, Learn 2026-10-29). Signing
free: **no** for the EXE path today (every PE file must be signed by a Trusted Root CA, policy 10.2.9; Azure Artifact
Signing takes individuals only in the US and Canada; SignPath Foundation declines new projects); yes for MSIX.
Automated: mostly (first submission by hand; then API or `msstore` CLI with four secrets; every version waits up to 3
business days for certification). Uninstall restores: very likely yes on the EXE path (Velopack's hook through the
ARP entry; not tested); **no** for MSIX (no uninstall hook). Extra risk: winget may prefer the slower Store copy of
an app (winget-cli #4347), a second updater fighting Velopack. **Recommendation: no Store at launch; revisit on the
EXE path once SignPath Foundation approves, after a two-source winget test.**

**Video tool:** Remotion (free for "an individual"; source-available, not open source; one command for the MP4 with
sound, the GIF loop and stills; FFmpeg bundled; Node v26.13.0 is here). Runner-up HyperFrames (Apache-2.0); change
when the media must rebuild with no license question. Sound: Wakeshot's own chimes, Kenney CC0 effects, one Freesound
CC0 music loop, all committed with `media/CREDITS.md`. GitHub: the video plays inline only as an uploaded attachment
(muted, controls, no autoplay, 10 MB on a free plan; `gh issue comment --attach` uploads by script); the icon loop is
a GIF in `<picture>` light and dark; Pages plays the MP4 from `site/`.

**Storyboard and shot list:** part 4 (eight scenes in 30 s: title, work starts, optional lid beat, work ends,
settings back, Keep awake, speed, end card; shots S1 to S10).

**Decided by the worker, for Haik's review:**

1. The three rivals and the rule to re-read stars on the README day (PowerToys Awake plus the two most-starred
   agent-aware tools that run on Windows and ship a release; a tie goes to the newer release).
2. `RequireExplicitUpgrade: true`: `winget upgrade --all` skips Wakeshot; Velopack is the only routine updater.
3. The winget PR from CI: `wingetcreate submit` with a classic PAT (`public_repo` only) on Haik's own account, as a
   repo secret used by one job. Alternative: a separate bot account (his identity, so his call).
4. No Microsoft Store at launch.
5. Remotion for the video; Kenney CC0 and a Freesound CC0 loop for sound.
6. Captures at the laptop's two real scales, 150 % and 125 %; 100 % only if Haik switches a screen by hand.
7. In the README: the GIF loop at the very top, the video as one bare attachment URL line right after the install
   line, then the rival table.
8. An optional 4 s lid-closed beat in the video (Wakeshot's own chime and the lock), beyond the card's five story
   beats.
9. The video's home: a comment on a pinned "Media" issue in the repo.

**Exact changes proposed, for the orchestrator to route after Haik reads this:**

- `p6-readme`: Read first adds `docs/research/launch.md` parts 1 and 4. Rival table: the three above, from the draft
  table in part 1, stars read again that day, PowerToys stars marked "for all of PowerToys", every cell with its link
  and date; the PowerToys 14.3 MB only with "test harness". Top: the GIF loop
  in `<picture>`, then the pitch, speed numbers, install line, then the video as a bare `github.com/user-attachments`
  URL line. A State, Icon, Description table for the three icons, light and dark. "Claude Code today; Codex and
  Copilot CLI next." Install section adds part 2 rule 9 (`--all` skips Wakeshot; an explicit `winget upgrade` ends a
  `wakeshot run` wrapper and leaves Wakeshot off until the next sign-in; `winget pin add HaikAsatryan.Wakeshot` for a
  GitHub install). No Store line. New check: the video URL is a `github.com/user-attachments` URL.
- `p6-media`: Read first adds launch.md part 1 ("Speed numbers") and part 4. Tool Remotion, version pinned in
  `media/package.json`, rebuilt with `npx remotion render` and `npx remotion still`; owns `media/.gitignore`
  (`node_modules/`). Sound as picked, with `media/CREDITS.md`. Captures at 150 % (DELL) and 125 % (laptop screen),
  light and dark switched through Settings by UI Automation; 100 % only with Haik's hand steps. Video: 1920 x 1080, 30
  fps, H.264 and AAC, 30 s plus or minus 2, under 10 MB, uploaded with `gh issue comment <n> --attach` to a pinned
  "Media" issue, URL recorded in `media/README.md`. Loop: GIF, light and dark, 4 to 6 s, under 1 MB. Store-size
  screenshots dropped (no Store at launch). New checks: ffprobe shows h264, aac, 1920 x 1080, size under 10 MB.
- `p6-winget-manifest`: Read first adds launch.md part 2. Deliverables add `RequireExplicitUpgrade: true`,
  `ProductCode` at the installer level and in `AppsAndFeaturesEntries` (`DisplayName`, `Publisher`, `ProductCode`; no
  `DisplayVersion`), `ReleaseDate`; `fill.ps1` takes the tag's published files (or a local folder), hashes them, and
  refuses a version that is not three numbers or is a pre-release; the folder README names `wingetcreate submit`
  (classic PAT, `public_repo`) with the reason, the GitHub-install caveat and `winget pin add`. New check (script):
  the filled manifest has `RequireExplicitUpgrade: true` and no `DisplayVersion`.
- `p6-pack-and-release` (owns `smoke-test.ps1` and `release.yml`): the smoke test also checks that the uninstall
  key's `DisplayVersion` equals the version; `release.yml` gets a `winget` job after the publish, stable tags only,
  running `fill.ps1` and `wingetcreate submit` with secret `WINGET_TOKEN`, off until Phase 8's first hand submission.
- `p6-pages`: Read first adds launch.md part 4. The page shows the MP4 from `site/` with `<video autoplay muted loop
  playsinline>` and a poster PNG, the GIF loop, and light and dark stills in `<picture>`; `site/` stays a few tens of
  MB (limit 1 GB). No privacy page at v1 (only when the Store is revisited).
- `p8-store`: Status "blocked: signing is not free yet (`research/launch.md` part 3); re-opens when SignPath
  Foundation approves Wakeshot or Haik buys a certificate". When it re-opens: EXE path only; Haik makes the free
  account (ID and selfie at storedeveloper.microsoft.com) and the first submission by hand (age rating, privacy
  policy text); the release workflow sets the package URLs and submits with `microsoft/microsoft-store-apppublisher`
  and four secrets; checks: a hidden-listing install, an uninstall during a hold that restores, the GitHub URL
  accepted, and `winget upgrade --all` on a PC that sees both ids keeps the newest build. Real laptop: yes.
- `product.md`, Distribution: (a) the Store line becomes "Microsoft Store: not at launch (2026-11-04): the account is
  free, but every file of an EXE submission must be signed by a trusted CA, which is not free for us until SignPath
  Foundation approves Wakeshot; MSIX is signed free but has no uninstall hook. Revisit on the EXE path once signing is
  free (`research/launch.md` part 3)."; (b) the winget line adds "winget installs, Wakeshot updates itself; the manifest
  sets `RequireExplicitUpgrade`, so `winget upgrade --all` skips Wakeshot"; (c) the README line names "PowerToys Awake
  and the two most-starred agent-aware rivals that run on Windows (2026-11-04: LidGuard and Agents Don't Sleep)"; (d)
  the video line adds "built with Remotion in `media/`".

**Checks.** Every fact has a link or a command and the date read; unchecked points say "not verified" (part by part
in `launch.md`). Stars and sizes read in this card with `gh api` and the release and NuGet APIs. The Result lists all
items the card asks for. `launch.md`, this card and the `research.md` addition have no em dash, no en dash and no
emoji, and are plain ASCII (`check_text.py`, a scratch script). No install, no account, no git; the release build was
skipped on the orchestrator's word.

**Open items.** Not verified and worth a test later: `RequireExplicitUpgrade` after a Velopack self-update; two Setups
at once; a Store EXE uninstall running Velopack's hook; the GitHub URL as a Store "direct link"; SignPath's
certificate in the Store; the video and WebP behavior in a real browser on github.com; whether deleting the issue
comment breaks the video URL; Remotion on Windows on Arm.
