# Plan

Start here in every new context: read this file, then the current phase file in `docs/phases/`.

## How we work

### Roles

- **Haik**: product decisions, sign-off on each phase design, physical tests (lid, battery, theme switch).
- **Orchestrator**: a main session started with `/orchestrate`. It turns the plan into task cards, spawns agents,
  commits, and keeps the status current. It does not do task work itself. Haik clears it often, so all state lives in
  files.
- **Worker** (`wakeshot-worker`, Opus, xhigh effort): does one task card end to end.
- **Reviewer** (`wakeshot-reviewer`, Opus, xhigh effort): reviews finished work with fresh eyes. Never fixes.

### Phase loop

1. Plan: a worker writes the phase design into the phase file and splits the work into task cards. Haik signs off.
2. Build: workers do the cards. Cards that touch the same files or the same real power settings run one at a time.
   Everything else runs in parallel, at full effort (Haik, 2026-11-03: parallelize whenever possible, for speed,
   without lowering effort). Full effort means xhigh, the agent files' default: never lower, never max. Work that
   needs the screen or sound runs one card at a time. Haik (2026-11-03): the orchestrator decides how much to run in
   parallel; the goal is zero conflicts, quality work and no work lost to conflicts. While other cards run, a card
   checks only its own package and files (`cargo fmt` on its own files, `cargo clippy -p` and `cargo test -p`), and
   a failure in another card's file is reported, never fixed or formatted. At most four cards build at the same time
   (the laptop has 24 threads and every cargo build takes them all); each card deletes its build folder when done. The whole-workspace check runs when the
   orchestrator closes a wave. A card that signs out, restarts Explorer or reboots runs only when no other agent
   runs. Work non-stop (Haik, 2026-11-03 19:55: "minimum stop, maximum parallelism at safe count"): fill a free
   slot the moment a card ends, from every ready card of every track, instead of waiting for a wave to drain; drain
   only for a card that must run alone. Cards that do not build (planning, docs, research, agent sessions) run beside
   the four building cards. Questions for Haik go to him in chat while the work that does not depend on them goes on.
3. Review: only the risky code gets a separate review (Haik, 2026-11-03): in Phases 2 to 6, two domain reviews with
   a narrow brief, power safety and Claude parsing (`docs/phases/02-06-lighter.md`, "Reviews"). Other cards are
   covered by their acceptance checks, the end-to-end cards and Haik's checks. Findings go to `docs/reviews/`.
4. Fix: the worker that built the card fixes its findings (SendMessage while it is available, otherwise a new worker
   that reads the card's Log). The same reviewer checks the fixes of blockers and majors only. No separate plan,
   close or fix cards.
5. Test: automated tests pass; manual checks that need Haik are listed in the phase file with exact steps.
6. Record: phase file, the status below, and every doc the phase changed.

### Task cards (`docs/tasks/`)

- One file per task, from `docs/tasks/_template.md`. Name: phase plus what it does, for example
  `p1-cargo-workspace.md`.
- Size: card estimates run 2 to 3 times low; wave 1 workers peaked at 190K to 270K tokens and did well. A bundle is
  two cards one worker does in order, each with its own Status, Log and Result; its estimates add up to at most about
  170K. The phase files list the bundles.
- A card holds everything a worker needs: goal, what to read, deliverables, acceptance checks, what is out of scope.
  The worker appends its log and result to the card.

### Context

- One session stays within about 150K to 250K tokens. Stop at a clean point and update the files first.
- The orchestrator reads cards, phase files and agents' short final replies, never agent transcripts or whole reports
  unless a decision depends on them.

### Git (Haik's standing rule, 2026-11-03)

- The orchestrator commits after each finished card and after review fixes. Only Haik's name; no AI mention or
  co-author lines. Workers and reviewers never run git.
- Push after a phase passes review.
- No worktrees in this repo (Question 9, decided by the orchestrator under Haik's delegation of parallelism,
  2026-11-03 15:55). Instead, so parallel work never conflicts or gets lost:
  - each card names the files and folders it owns; two running cards never own the same one;
  - shared files take additions only, re-read right before each edit (`docs/research.md`); the root `Cargo.toml`,
    `Cargo.lock` and the contracts change only through a card the orchestrator runs alone;
  - each worker builds into its own `CARGO_TARGET_DIR`, named after its card (`<scratchpad>\<card>\target`): all
    agents of one session share one scratchpad, so a plain `<scratchpad>\target` collides;
  - the orchestrator commits each finished card's own files by path (`git add <paths>`), never `git add -A` while
    other cards run.
  Change it when an agent must run git, or two cards must edit one file at the same time.
- Fully public from the first push (Haik, 2026-11-03): docs, planning and spikes go public as they are; temporary
  junk in the remote is fine, nobody watches the profile. Secrets never go public: a secret scan before each push.
  Phase 8 cleans the repo and squashes the history into one commit; ask Haik right before that force push.

### Where things go

Facts with how they were checked: `docs/research.md`. Longer research: `docs/research/`. Decisions:
`docs/product.md`. Design: `docs/architecture.md`. Reviews: `docs/reviews/`.

## Phases

| # | Phase | Output | Status |
|---|---|---|---|
| 0 | Facts | Spikes on the test laptop, rival and agent research, final decisions | Done (signed off 2026-11-03) |
| 1 | Detailed design and skeleton | Detailed `architecture.md`, public GitHub repo, Cargo workspace, the contracts between the parts (`p1-contracts`), CI (fmt, clippy, tests, x64 and ARM64 builds), tray menu look that matches Windows 11 (`p1-tray-menu-look`), audible lid-close sound (`p1-lid-sound`) | Done (signed off 2026-11-03 19:13; `docs/phases/01-design-skeleton.md`) |
| 2 | Core (track) | State machine, policies, settings ownership rules, file formats, full tests | Built (`docs/phases/02-core.md`) |
| 3 | Windows power layer (track) | Power requests and settings, journal and restore, guard process, OS events | Built (`docs/phases/03-windows-power.md`) |
| 4 | Sources (track) | Claude Code (with timers), `wakeshot run` and hold files, Keep awake; Codex CLI, Copilot CLI and in VS Code, OpenCode back in v1 (Haik, 2026-11-06) | Built for v1; the new agents in progress (`docs/phases/04-sources.md`) |
| 5 | Tray UX (track) | Icons (design, light and dark, high contrast, every scale), menu, notifications, sounds, first run, log | Built (`docs/phases/05-tray-ux.md`) |
| 6 | Ship (track, then integration) | Velopack installer and updates, start at login, uninstall restore, signing, winget manifest, README, GitHub Pages; then wiring all tracks into the two exes, end-to-end tests, Haik's manual checklist | Built and checked with Haik; README, Pages and media wait for his UI review (`docs/phases/06-ship.md`, `docs/phases/06-integration.md`) |
| 7 | Field test | A week of real use on the test laptop, fixes, v1.0 | |
| 8 | Launch cleanup | A clean public repo (Haik, 2026-11-03): short `CLAUDE.md`, the README for users (long but never boring, Haik 2026-11-04), maybe one Claude skill if it matters; internal planning, reviews and junk removed; history squashed into one commit (Haik's go right before the force push); then the winget submission, and the Microsoft Store if free and automated (`docs/tasks/p8-store.md`); what to remove is listed in `docs/tasks/p8-cleanup.md` | |

Phases 2 to 6 are parallel tracks, not steps (Haik, 2026-11-03: like two devs and a devops person on separate parts,
test-first). They start together after Phase 1 and build against the Phase 1 contracts: the boundary types and
traits from `architecture.md` sections 2, 3 and 6, compiled into the skeleton, with every dependency declared up
front. A contract change goes through one card and the orchestrator, never as a side edit in a track. One plan step
writes the cards for all tracks; Haik signs off once. Code and unit tests run in parallel, each worker with its own
`CARGO_TARGET_DIR` so builds do not block each other. Anything on the real laptop (power settings, screen, sound,
installs) runs one card at a time, and that, plus Haik's manual steps, sets the pace. Integration (wiring, end-to-end
tests, Haik's checklist) starts when the tracks it needs are done.

## Status

- 2026-11-03: decisions fixed (`product.md`). Phase 0 research done: four reports in `docs/research/`. Remaining
  Phase 0 work is in task cards (`docs/phases/00-facts.md`).
- Agents `wakeshot-worker` and `wakeshot-reviewer` are defined in `.claude/agents/`. The `/orchestrate` skill is in
  `.claude/skills/orchestrate/SKILL.md`. If a session does not list the agent types (they load at Claude Code start),
  use a general-purpose agent on Opus, xhigh effort, told to follow the agent file.
- Build Tools 2026 (18.11.2) installed 2026-11-03: MSVC 14.51 with x64 and ARM64 targets, Windows SDK 10.0.26100.
- The orchestrator session runs elevated, so `powercfg /requests` works from it. Spikes test writes through an
  unelevated token, because Wakeshot runs unelevated.
- Phase 0: research, toolchain, distribution facts, Claude live tests, unelevated liveness, spike safety fixes and
  the close list are done. Haik answered every close decision (2026-11-03): no lid-closed time limit, no heat guard,
  every signal checked against a fact; his yes to part 2 is recorded in `docs/reviews/p0-close-proposals.md`. His
  manual list was cut to about 80 minutes (`docs/phases/00-morning-steps.md`, card `p0-morning-trim`).
- Morning steps, agent steps, delta review, M1 battery check and `p0-close-results` done (2026-11-03). No spike,
  holder, keep-awake or requests-log process left (lid card AFTER line, 13:08; M1 restore checked 14:12). Haik's
  T13 verdict on the tray menu (looks old) became card `p1-tray-menu-look`.
- Haik's parallel session (2026-11-03, about 14:24 to 14:58) did `p1-tray-menu-research`
  (`docs/research/tray-menu-look.md`) and the tray Result review (`docs/reviews/p0-tray.md`). Its findings were
  applied by `p1-tray-review-fixes` and verified (`docs/reviews/p0-tray-verify.md`).
- Phase 0 signed off by Haik 2026-11-03 14:35, before the tray review finished. Public repo:
  `HaikAsatryan/wakeshot` (name free at 14:34). Phase 1 file: `docs/phases/01-design-skeleton.md` (card table and
  work log live there from now on).
- 2026-11-03 16:52: Phase 1 design signed off by Haik (16:44). Done: lid sound, tray menu look (waits only for
  Haik's look at the pictures), repo files, Cargo workspace, CI, contracts. Running: five track plan cards
  (`p2-plan-core` to `p6-plan-ship`) and the Phase 1 general review. Contracts domain review done: 1 blocker
  (tracks would share contract files).
- 2026-11-03 17:22: all five track plans done (63 cards, about 6.15M tokens, phase files `02-core.md` to
  `06-ship.md`). Orchestrator decision 17:05: apply, restore and check routines live in the core behind `PlanIo` and
  `SafetyNet`; the Windows track implements them. Running: `p1-contracts-split` and the cross-track review.
- 2026-11-03 17:43: cross-track review done (2 blockers, 10 majors, 19 minors); `p1-track-plan-fixes` running;
  `p1-tray-menu-look` done (Haik likes it; an icon per menu item, Phase 5).
- 2026-11-03 17:57: `p1-contracts-split` done (no shared files left; owner table in its Result). Its 10 card edits
  were sent to `p1-track-plan-fixes`, which still runs (started 17:43, agent of the 14:31 orchestrator session).
- 2026-11-03 18:02: Haik gave Gate B (go public: yes) and signed off the five track plans ahead; the planners'
  product questions still go to him with defaults. Order now: `p1-track-plan-fixes` ends, then the freeze and git
  steps 4 to 10, then the reviewer check of the plan fixes and the contracts split, then the track builds.
- 2026-11-03 18:07: `p1-track-plan-fixes` done; no agent runs.
- 2026-11-03 18:09: Haik answered the 12 questions in `docs/phases/02-06-signoff.md`: all defaults.
- 2026-11-03 18:15: repo public (`HaikAsatryan/wakeshot`, one commit "Initial commit"), CI green on all three jobs.
  The `commit-msg` hook could not be written (auto mode classifier); the orchestrator checks each commit message.
- 2026-11-03 18:39: fixes verify done (0/2/6, `docs/reviews/p1-fixes-verify.md`); `p1-close-contracts` running
  alone (with `p4-claude-live-shapes`, no build). Integration planned (`docs/phases/06-integration.md`, 11 cards,
  about 1.19M tokens); waits for Haik's sign-off and two answers.
- 2026-11-03 19:08: `p1-close-contracts` done and verified (0/1/5, major routed into not-started cards). Haik
  signed off the integration plan (18:58); his dock is always plugged in. Wave 1 of the track builds runs since
  18:52: `p3-store`, `p3-log`, `p3-process`, `p2-ownership-restore`.
- 2026-11-03 19:15: Phase 1 pushed (CI green) and signed off by Haik (19:13). Wave 1 done and committed.
  `p6-int-skeleton` runs alone (it also runs the whole-workspace checks that close wave 1).
- 2026-11-03 19:46: `p6-int-skeleton` done (19:28). Wave 2 (`p2-policy-holds`, `p2-policy-power`, `p2-present`,
  `p4-hold-files`) started 19:29 and was stopped by Haik about 19:45, before any code; cards back to ready. A second
  session (`wakeshot-9d`) is making Phases 2 to 6 lighter for Haik (merge small cards, fewer reviews, Codex and
  Copilot after v1, lid-closed mode stays); it sends its merge plan before it edits cards or phase files. The
  orchestrator starts no card until that plan is applied and Haik confirms the three decisions in its own chat.
- 2026-11-03 20:00: the lighter plan is applied (`docs/phases/02-06-lighter.md`, by session `wakeshot-9d`): nine
  bundles, two separate reviews instead of one loop per phase, Codex and Copilot after v1, lid-closed mode stays.
  Next (orchestrator): start cards at once from the phase tables, bundles as one worker, the old wave 2 cards first
  (`p2-policy-holds`, `p2-policy-power`, `p2-present`, `p4-hold-files`). The line below is older.
- 2026-11-03 21:24: non-stop build since 19:55 (session `wakeshot-91`), at most four building cards. Done and
  committed: 16 cards (core: policy holds and power, present, apply, core holds and settings; power: journal, power
  plan, system events, requests, session, power IO, hibernate, recovery; sources: hold files, watch reads and data)
  plus a docs pass on `architecture.md`. Haik's answers 21:21: unplug settle 30 s; a work-end countdown follows the
  user's own lid action for the new power source. Haik's battery sleep is now 4 h (set 20:41). Running at 21:24:
  `p2-core-lid`, `p4-claude-status`, `p4-cron`, `p3-stop-points` (laptop lane). Not pushed: 391 commits since
  `0f9960a`; push after a track passes its review.
- 2026-11-03 22:18: more done and committed: `p2-core-lid` (settle 30 s, countdown rule), `p3-stop-points` (3142 real
  cases, Haik's values same), `p4-claude-status`, `p4-cron`, `p6-velopack-facts`. Running: `p2-ownership-props`,
  `p6-int-dispatch`, `p4-claude-transcript`, `p3-harness` (laptop lane). Two questions with Haik (asked 21:52): the
  battery floor exception for a user whose own battery lid action is Do nothing, and when the closed-lid report shows.
- 2026-11-03 23:36: Haik answered both 21:52 questions with the defaults and approved the power safety review's lock
  fix (failed lid-close lock: one retry, then step back and countdown). Recorded in `product.md`, `architecture.md`
  and the `02-core.md` work log; the code goes to the power review fix round.
- 2026-11-03 23:45: Haik is away overnight (laptop lid open, sound is fine). His words: do the maximum, even without
  his approval; he reviews such cases later. So the orchestrator answers open questions with its default, records
  each one as "orchestrator decision, for Haik's review" in the phase work log, and lists them for him. Cards that
  need his hands still wait. His rule for open calls: with no work going on, the laptop behaves as if Wakeshot were
  not there (`product.md`). Handover when the context is big: a continuation prompt to the idle Claude Code session.
- 2026-11-04 00:46: since 23:32 (session `wakeshot-91` after a /clear): done and committed: the power review fix
  round and its verify (finding 1 Fixed; the power safety review has passed), the LoopHandler `&self` contract
  change, `p5-text`, `p6-int-window`, `p6-int-startup`, a docs pass, the Claude parsing review (1 blocker, 1 major,
  8 minors) and its fixes, `p4-watch-folders`, the `p4-cli-commands` + `p4-cli-restore` bundle; session
  `wakeshot-12`'s launch planning (Haik's launch request: `p6-launch-research`, `p6-media`, `p8-store`). Pushed at
  00:37. Running: `p4-watch-thread`, `p5-theme` + `p5-menu-paint`, `p3-real-checks` (Part 1; `p3-haik-checks` waits
  for Haik), `p4-cli-args` + `p4-cli-run`, the Claude fixes verify, `p6-launch-research`.
- 2026-11-04 02:02: since 00:45 (session `wakeshot-9d`, after the handover from `wakeshot-91`): done and committed:
  the CI fix (`watch::data` tests used pid 4) and the Claude M1 part b fix with its verify (the Claude parsing review
  has passed), `p5-theme` + `p5-menu-paint`, `p4-watch-thread`, `p3-real-checks`, `p6-launch-research`, `p4-cli-args`
  + `p4-cli-run`, `p5-icon-art`, `p6-int-io-thread`, `p6-login-and-path` + `p6-update-schedule`, docs pass round 2,
  core lid round A, `p4-live-claude`, Part A of `p5-tray-icon` and `p5-sounds` (their Part B waits for the screen and
  sound slot, after 15 minutes with no input). Blocked for Haik: `p3-haik-checks`. Haik's answers: his sleep values
  are now 5 h on AC and battery (`CLAUDE.md`); free paths only, no paid certificate. Running: `p6-hooks`, `p5-notices`
  Part A, core round B, the `p6-pack-and-release` + `p6-winget-manifest` bundle. Haik is away until about 10:00.
- 2026-11-04 02:41: since 02:03 (session `wakeshot-46`, after the handover from `wakeshot-9d`): done and committed:
  `p6-hooks`, the `p6-pack-and-release` + `p6-winget-manifest` bundle, core round B (power findings 8, 9, 10),
  `p5-sounds` (Part B in the locked session), the CLI restore test fix, Part A of `p5-notices` and `p5-menu-view`.
  Pushed 8ff7fa5 at 02:30: CI green on all three jobs; the release dry run passes build, pack, install and uninstall,
  and fails only the 3 hook checks until `p6-velopack-dependency`. Found: the session locks when the display turns off
  (5 minutes on AC), so every screen Part B (`p5-tray-icon`, `p5-menu-view`, `p5-notices`, later `p6-int-tray-mode`)
  needs Haik at the laptop to unlock it. Running: `p6-int-tray-mode` Part A, `p2-6-unread-journal`.
- 2026-11-04 03:58: since 02:41 (session `wakeshot-46`): done and committed: `p2-6-unread-journal` (power findings 3
  and 6, the newer `config.json` check), `p6-int-tray-mode` Part A and its smoke test (the whole dry-run tray runs and
  quits cleanly; the picture waits for Haik), the flaky window timer tests fix, docs pass round 3,
  `p6-velopack-dependency` (run alone, without clang: ARM64 tray CI only), `p6-int-update-wiring`, `p6-install-e2e`
  (81 of 81 checks twice; the laptop as before). Pushed d3b8c6e at 03:58. CI green on cd3d498, and its release dry
  run passed on both CPUs, hooks included. Running: `p6-int-wiring-fixes`.
- 2026-11-04 05:51: since 04:00 (session `wakeshot-12`): done and committed: `p6-int-wiring-fixes`, the
  `p6-install-e2e` rerun (81 of 81 on the wired tray), `p6-int-dry-run-e2e` (12 of 12 with no fix, a Claude Code
  session held and let go; pictures left), `p6-int-real-e2e` (10 of 10 on an active plan copy, AFTER equals BEFORE),
  `p6-int-update-e2e` (125 checks, the notice skipped while locked), new small cards `p6-int-log-wording` and
  `p2-6-unread-lid-countdown`, and a README draft (waits for `p5-haik-check` and `p6-media`). CI green on abd3061.
  A cargo-mutants mutant took 32 GB at about 05:20; Claude Code ended two other keep-awake holders for low memory.
  Running: `p2-6-unread-floor-countdown`.
- 2026-11-04 07:10: since 05:51 (session `wakeshot-12`): done and committed: `p2-6-unread-floor-countdown`, docs pass
  round 4, `p6-startup-switch-check` (the off switch survives `Setup.exe` over the install by the keys; uninstall left
  its value), `p6-uninstall-startup-approval` (uninstall now removes it; `e2e.ps1` 88 of 88), `p6-tray-size` (release
  tray 3.95 to 2.71 MB: `opt-level = "z"` and the IDNA stub), `p6-small-fixes`. Pushed 2bcfd21: CI green on all
  three jobs, and the release dry run (run 41765357171) packed and smoke-tested both CPUs (07:16). Nothing runs now.
- Next (orchestrator): every card left needs Haik. In order: his read of tonight's decisions (phase work logs,
  2026-11-04 lines); a screen session (unlocked, hands off about 20 min) for the Part B runs of `p5-tray-icon`,
  `p5-menu-view`, `p5-notices`, the `p6-int-tray-mode` and `p6-int-dry-run-e2e` pictures, the update notice and the
  real Settings start-up switch; `p5-haik-check`; `p3-haik-checks`; `p6-int-haik-checks`; `p6-media`; then the README
  finish and `p6-pages`. Open calls for him: `p6-int-wiring-fixes` decision 7 (a lock on a user's own Do nothing
  while the journal is unknown), the SChannel option of `p6-tray-size` (1.87 MB and no clang, but a patched
  velopack), the clang install, the winget token owner, actionlint on `release.yml`.
- 2026-11-04 14:09 (session `wakeshot-12`, Haik back since about 09:50): Haik answered every open call (product.md and the
  phase work logs: sleep, not hibernate, after a closed-lid work end and at the floor, through Windows' own idle
  sleep with 60 s timeouts, hibernate as the fallback; a lost journal goes to Windows defaults; the TLS stays rustls,
  velopack issue #1146; winget token on his account; the brand: the Sunrise, night violet and sun gold). Done and
  pushed today: `p3-idle-sleep-check` (with Haik), `p2-6-record-lost`, `p2-6-sleep-route`, its review (0 blockers;
  3 majors fixed and verified) and `p2-6-sleep-route-fixes`, the screen runs of `p5-tray-icon`, `p5-menu-view`,
  `p5-notices`, the `p6-int-tray-mode` and `p6-int-dry-run-e2e` pictures, `p5-haik-check` Part A (`ux_all`),
  docs passes 5 and 6, the README comparison (PowerToys Awake, Caffeine, Don't Sleep), drafts of `p6-pages` and
  `p6-media` Part A. Nothing runs now. GitHub refused the SSH key 13:50 to about 14:30 (cause not verified); pushes
  work again.
- 2026-11-04 16:45 (session `wakeshot-27`, after the handover from `wakeshot-12`): done and pushed: `p2-6-props-flake`
  (a core bug: a sleep route apply kept a key the user took back), `p4-watch-tick-flake` (two Windows test flakes),
  `p2-6-reset-gaps` (two rare core gaps, both real), `p2-6-props-rare` (a core gap: no hibernate or short timeouts right
  after input; two props model gaps). Core props pass 10 of 10 runs at 4096 cases and 50 of 50 at 256. With Haik: README
  table round 3 (his comments), `p5-haik-check` (done: the menu opens above the taskbar; the demo banner names
  Wakeshot), his answers on a restore during the countdown and a hold that starts with the lid closed (`product.md`). CI
  green through 2016893.
- 2026-11-04 18:32: `p3-haik-checks` done (90794e2): A, C, D, E and B1 pass; Haik's values same. B2 (restore at the
  next sign-in) not shown: B2 started before Explorer's RunOnce pass of that sign-in, which ran restore mode at once
  and Windows deleted the value. The same gap is real in the product (a hold that starts in the first minute after
  sign-in loses its RunOnce value). Orchestrator decision, for Haik's review: fix card `p2-6-runonce-keep`. Running:
  its Part A (the fix).
- 2026-11-04 19:12: `p2-6-runonce-keep` Part A (76a44cc), its narrow power safety check (0/0/6, cac66d4) and the fix
  round (113a9a0) done: the RunOnce value comes back within one plan read, also for another owner's leftover
  (orchestrator decision, for Haik's review); 570 tests pass. Review minor 3 (README gap list) waits: `README.md` has
  uncommitted edits not made by this session or its agents (asked Haik whose). New card `p6-real-install-guard`
  (later, before Haik installs the first release). Haik's call: notices stay `Shell_NotifyIcon` banners.
- 2026-11-04 22:50: `p2-6-runonce-keep` done: Haik's B2 redo passed (restore at the next sign-in on the fixed code;
  his values same). README: Haik's trim (dd72a4d), the gap bullet, the flat contents list under Details. Haik:
  `p6-media` Part B waits until his design review of the final software. Pushed 0c7bc26: CI green on all three jobs.
- 2026-11-05 ~02:30: `p6-int-haik-checks` done (decfd5a): blocks 1, 2, 3, 5, 7, 8 pass; 4 safe with findings; 6
  step 3 not shown. Haik: a 30 s grace on battery before the floor step back (`product.md`). Haik's own
  cleanup committed (a47d03f). Running: `p2-6-floor-grace`, `p6-real-install-guard` (promoted from later). Haik is
  asleep (dock in, lid open): orchestrator decisions go to the `06-integration.md` work log for his review.
- 2026-11-05 ~05:00: done and pushed (5f1b1db): `p6-real-install-guard`, `p2-6-floor-grace` (30 s grace; its narrow
  power safety check 0/0/4, all four fixed), `p6-notice-app-icon` Part A (the hooks register the notice app id's
  name and icon), docs pass round 7. Whole-workspace checks on x64 green before the push. Orchestrator and worker
  decisions for Haik's review: the `06-integration.md` work log, 2026-11-05 lines, and each card's decisions. CI
  green on 5f1b1db (all three jobs); release dry run 41931334598 built, packed and smoke-tested both CPUs.
- 2026-11-05 ~13:00: done and committed: clang installed by Haik (22.1.3), `p6-notice-app-icon` Part B with
  `p6-screen-reruns` (the header shows the Wakeshot icon from both start paths; Win+N never keeps a balloon; floor
  grace passes; the countdown waited while input came), `p2-6-sound-overlap-smoke-key` (one sound; the smoke test
  checks the notice key; ARM64 checks pass here), `p6-polite-quit-notice-retry` (a plain taskkill quits the tray; a
  refused notice is tried again). Not pushed: 6 commits since 5f1b1db (auto mode refused the orchestrator's push;
  asked Haik). Haik's brand session (its own orchestrator and `p6-brand-*` cards) runs in the same repo;
  `p6-brand-cleanup` edits `wakeshot-windows`, so this orchestrator starts no code card while it runs.
- Next (orchestrator): the push and CI; an installer for Haik's UI review and the field test (default: the release
  dry run's x64 artifact); then `p6-media` Part B, the README and Pages finish after his brand session; Phase 7
  field test; Phase 8 cleanup. On this laptop Wakeshot is missing from Settings' tray list (`p6-screen-reruns` Found
  outside scope 3): a sign-out after the next install, if still missing. Haik's reads still open: the 2026-11-05
  decisions (06-integration work log, the cards' worker decisions) and `p2-6-runonce-keep` Decisions and finding 6.
  Known minors: a rare two-sound case at the grace's end (`p2-6-sound-overlap-smoke-key` Found outside scope 2); the
  smoke test skips the shortcut's app id (same card, 4).
- 2026-11-05 14:20 (Haik's brand session): brand review done (`docs/phases/06-ship.md` work log, same time). The cup
  is gone from the menu, the menu text is grey. Next: Haik's 8 brand decisions (board "Wakeshot brand review"), then
  a brand apply card (about 150K to 200K tokens), the screen session, `p6-media` Part B, README and Pages.
- 2026-11-05 ~18:20 (Haik's brand session): the new brand is applied everywhere: the held light in Moss in `art.rs`
  (tray, app icon, notice icon, favicon, page icon and lockups, all written by the art writer with golden tests), the
  page, the README header, the social card, `media/` (port at 0 pixel difference, checked in CI), `docs/brand.md`;
  old-brand history listed in `docs/tasks/p8-cleanup.md`. Final review 0/2/6, both majors fixed and verified. The
  screen session ran with the worker alone (Haik away): 46 captures, his settings equal. Phase 6 work log has the
  orchestrator decisions for his review.
- 2026-11-05 20:05 (Haik's brand session): social preview uploaded (Haik; API reads it as set). A local 0.1.0 build
  of 2056c60 is installed for his checks and his last visual review (not the field test build: uninstall it before
  the first release).
- 2026-11-06 ~05:30 (overnight orchestrator, Haik asleep): speed numbers measured on his installed 0.1.0 (4.3 MB
  private memory idle, 109 ms CPU in 10 idle minutes, a hold starts about 0.1 s after Claude Code starts work,
  Setup.exe 12.3 MB; `docs/research.md` "Speed numbers"). The final promo video is rendered (7.1 MB, every check
  passing, review 0/1/10 with the major and nine minors fixed). README: Haik's story ("Why I built it", "What comes
  next"), the icon loop, pictures, state icons and numbers; the video spot waits for his upload. Community files added
  (`p6-repo-files`; GitHub's community profile 100%). His install updated to the latest code. Pushed; CI green.
- 2026-11-06 ~15:00: Haik approved the final video (music: "Funky Disco Beats", Fupi, CC0; a clear time cut) and a
  new README written from scratch (`docs/research/readme.md`); both pushed. Then he grew v1: the upload, Pages and
  launch wait, since the video may change. v1 agents: Claude Code, Codex CLI, Copilot CLI, Copilot in VS Code,
  OpenCode (2.x first), Cursor, Antigravity if testable for free (`docs/product.md`). Done: `p4-codex-source`,
  `p4-copilot-source` (pushed, workspace checks green on x64 and ARM64), research for VS Code, OpenCode, Cursor,
  Gemini and hooks. Haik: "You can do anything": workers may install agents and Ollama for his user, the orchestrator
  decides what is easy to undo and logs it for his review, computer use where a command line cannot drive an app,
  hooks as in `product.md`. He gets the Codex subscription himself.
- 2026-11-06 ~18:55 (orchestrator after a /clear): done and committed: `p4-contracts-r2`, `p4-antigravity-research`
  (hooks only, free test), `p4-hooks-design` (`docs/agent-hooks.md`) with a security review and verify (0 blockers;
  3 majors fixed), `p4-live-harness` (every agent installed for Haik's user, ConPTY driver; Haik did the Cursor and
  Antigravity sign-in), `p4-vscode-source`, `p4-opencode-source` Part B (2.x only) with its review (1 major, moved to
  `p4-contracts-r3`) and minor fixes, `p4-live-codex` (passed; Codex in the tray). Running: `p4-contracts-r3` (alone),
  `p6-agents-video-readme` Part A (Haik's request: video and README for every AI agent). Not pushed since 8c2637a.
- 2026-11-06 ~22:35 (session wakeshot-56, handing over to wakeshot-f5): built and committed since 18:55:
  `p4-contracts-r3`, `p4-cursor-source`, `p4-agent-hooks-install`, `p4-antigravity-source`, `p4-core-linux-tests`
  (CI `core-linux` had been red since fb0ebce), the hooks code review (0/1/7, passed) and the OpenCode review verify.
  Live, run by agents: Codex CLI, Copilot CLI, OpenCode 2.x passed; Copilot in VS Code partly (new chats go to the
  Copilot harness, held by the Copilot source; Local chat files save on a 60 s beat); Antigravity CLI passed. Waiting
  on Haik: `p4-live-cursor` (his 5-minute steps) and the Antigravity IDE first-run pass. Each live test found real
  bugs, now fixed. `p6-agents-video-readme` Part B first pass done. Pushed up to 98c8795 only; everything after waits
  for the whole-workspace checks in `p6-agents-install-e2e`.
- Next (orchestrator): `p6-agents-install-e2e` (runs alone; the Cursor editor turn waits for Haik), then
  `p6-agents-video-readme` Part B last steps, push and CI, Haik's review of README and video, `p4-live-uninstall` last;
  then ask Haik about the video upload, Pages, Phase 7, Phase 8.
