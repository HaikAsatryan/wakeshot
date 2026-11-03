# Phases 2 to 6: the lighter plan

Haik, 2026-11-03 (session `wakeshot-9d`, confirmed in the orchestrator's chat 19:55): make the rest of Phases 2 to 6
lighter. Three decisions:

1. Bundle small cards; separate reviews only for the risky code; no more plan, close or fix cards.
2. Codex CLI and Copilot CLI move to after v1. v1 ships Claude Code, `wakeshot run`, hold files and Keep awake.
3. Lid-closed mode stays in v1.

This file is the record of what changed. The phase tables and `docs/plan.md` "How we work" hold the rules from now on.

## Why

Measured 2026-11-03 from the worker transcripts:

- Wave 1 cards peaked at 190K to 270K tokens of context, 2 to 3 times their estimates (`p3-store` 90K estimated,
  260K peak; `p3-log` 80K, 193K; `p3-process` 110K, 232K; `p6-int-skeleton` 50K, 192K). They worked well at that size.
- Each worker starts at about 37K tokens (harness, agent file, CLAUDE.md) and reads 17K to 42K more before it writes
  code. A bundle pays that once for two cards.
- The old review loop (two reviewers per phase, a fix worker, a verify reviewer) was about 24 more agent runs for
  Phases 2 to 6. The new loop is about 4.

## Bundles

One worker does both cards, in the order shown, each with its own Status, Log, Result and acceptance checks. A bundle
starts when the Runs after lines of both cards are met, and owns the files of both. Each bundle's card estimates add up
to at most about 170K. Only serial chains, or two small cards of one area.

| Bundle | Why |
|---|---|
| `p3-requests` + `p3-session` | two small Windows wrappers, no deps, 140K |
| `p3-real-checks` + `p3-haik-checks` | a chain on the real laptop, 130K; the worker blocks for Haik's steps in the second card |
| `p4-cli-args` + `p4-cli-run` | a chain, 160K |
| `p4-cli-commands` + `p4-cli-restore` | one crate, 150K; the bundle waits for `p3-journal` and `p3-power-plan`, which `p6-int-io-thread` waits for anyway |
| `p4-watch-reads` + `p4-watch-data` | a chain, about 145K once the lock-holder code moved out |
| `p5-theme` + `p5-menu-paint` | a chain, 150K; `p5-tray-icon` now waits for the bundle |
| `p6-login-and-path` + `p6-update-schedule` | two small Windows modules, 140K |
| `p6-pack-and-release` + `p6-winget-manifest` | a chain in `packaging/`, 170K |
| `p6-readme` + `p6-pages` | a chain, both read by Haik at the end, 130K |

Not bundled: the core cards, the integration cards, and the other tray, sources and ship cards. Each is 80K to 150K
by its estimate, so two would pass about 170K.

## Reviews

Two separate reviews in all of Phases 2 to 6, each one `wakeshot-reviewer` with a narrow brief:

| Review | Runs | Covers | Brief |
|---|---|---|---|
| Power safety | before the `p3-real-checks` bundle, which waits for it | `p2-ownership-apply`, `p2-ownership-props`, `p2-core-settings`, `p2-core-lid`, `p3-journal`, `p3-power-plan`, `p3-power-io`, `p3-recovery`, `p3-stop-points`, `p3-hibernate`, `p3-session` | Find any path where user power settings are not restored, a user's own change is overwritten, or a closed laptop keeps running unlocked or below the battery floor |
| Claude parsing | before `p4-live-claude`, which waits for it | `p4-claude-status`, `p4-cron`, `p4-claude-transcript`, `p4-hold-files`, `p4-watch-reads` | Find any file content that makes Wakeshot hold forever or never, and any path that stores or logs prompt text or opens a `.key` file |

- Findings go to `docs/reviews/`. The worker that built the card fixes them: SendMessage while it is available,
  otherwise a new worker that reads the card's Log. The same reviewer checks the fixes of blockers and majors only.
- Every other card gets no separate review: its acceptance checks, the end-to-end cards and Haik's checks cover it.
  `p6-hooks` and `p4-cli-restore` call the reviewed `restore_locked`; `p6-int-real-e2e` and `p6-int-update-e2e` run
  them for real.
- No more plan, close or fix cards. The orchestrator writes the Phase 7 and 8 cards itself.

## Codex and Copilot after v1

- `p4-codex-source`, `p4-copilot-source`, `p4-live-codex`, `p4-live-copilot`: Status `later (after v1)`. The
  orchestrator never starts a `later` card.
- The Restart Manager lock-holder code (`lockowner.rs`) and the `AnyProcess` liveness serve only Codex and Copilot
  (track note 7). They moved from `p4-watch-reads` and `p4-watch-agents` into `p4-codex-source`. The stub files stay.
- Runs after lines no longer name the four cards: `p4-live-claude`, `p6-int-tray-mode`, `p6-readme`. The tray starts
  the watch thread with the Claude source only.
- Haik's answers on Codex and Copilot waiting (sign-off items 1 and 2) still apply when they are built.

## Done

2026-11-03 20:00, session `wakeshot-9d`. Files changed:

- `docs/plan.md` ("How we work" steps 3 and 4, the Task cards size line, the Phase 4 row, a Status line);
  `.claude/skills/orchestrate/SKILL.md` (steps 4 and 7); `.claude/agents/wakeshot-worker.md` (bundles).
- Phase files: `03-windows-power.md`, `04-sources.md`, `05-tray-ux.md`, `06-ship.md`, `06-integration.md`,
  `02-06-signoff.md` (a note under items 1 and 2).
- Cards: `p4-codex-source`, `p4-copilot-source`, `p4-live-codex`, `p4-live-copilot`, `p4-watch-reads`,
  `p4-watch-agents`, `p4-live-claude`, `p4-cron`, `p6-int-tray-mode`, `p6-readme`.
- `docs/product.md` (Other agents), `docs/architecture.md` (decision 9 and section 7).
- Code comments only: the Owner line in `crates/wakeshot-windows/src/lockowner.rs`.
