# Phases 2 to 6: sign-off

Five tracks, built in parallel after the contracts card (done 17:56). Your yes signs off all five plans. Detail lives
in `docs/phases/02-core.md` to `06-ship.md`; you do not need them to decide. Checked by `p1-track-plan-fixes`
(2026-11-03): every finding of the cross-track review is fixed or listed, and no two cards that can run at once own a
file.

| Track | What it builds | Cards | Tokens | Your minutes | Laptop sessions |
|---|---|---|---|---|---|
| 2 Core | The decision logic in plain Rust: state machine, rules, the apply and restore of your settings, all tests | 10 | 1.21M | 0 (answers only) | 0 |
| 3 Power | The Windows side: plan reads and writes, the journal, power requests, guard and sign-in restore, lock, hibernate, power and session events | 15 | 1.42M | about 60 elapsed, 15 of hands | 5 |
| 4 Sources | Claude Code (status, transcripts, timers), Codex, Copilot, hold files, the watch thread, the `wakeshot` command | 19 | 1.73M | about 40, plus 3 optional | 1, plus 4 agent sessions |
| 5 Tray UX | Icons for every theme and scale, the Windows 11 menu, notices, three chimes, every word, first-run hint | 9 | 0.93M | about 25, plus two short looks | 5 |
| 6 Ship | Installer, start at login, updates, uninstall restore, release workflow, winget manifest, README, Pages | 10 | 0.88M | not estimated (README read, Pages look; about 10, a guess) | 2, plus 1 if clang |
| All | | 63 | 6.17M | about 125 stated, plus the looks | 13, plus 4 agent sessions |

- Laptop sessions run one at a time across tracks. The 4 agent sessions (test sessions of Claude, Codex, Copilot) may
  run beside them. Two points stop all agent work: your power checks (sign-out and sleep) and adding `velopack`.
- Critical path: 8 cards, about 730K tokens: the watch thread, the Claude live check, your Codex and Copilot sessions,
  then README and Pages. Next: the core chain (5 cards, 660K) and the power chain to your checks (630K). The real pace
  is set by the one-at-a-time laptop sessions and your time.
- Not planned yet: integration (wiring the tracks into the two exes, end-to-end tests, your checklist). The
  orchestrator plans it after the tracks; its minutes are not counted above.

## Questions, each with the default the cards build

Answered 2026-11-03 18:09: Haik, "Answer to all questions are default!". Every question below takes its default.
`p1-close-contracts` wrote the answers into the cards and phase files, and added the update notice of item 7.

1. Codex waiting for your approval looks like working, so it holds with no limit while Codex runs. Default: hold, no
   limit. Other: end a silent Codex turn after 1 hour (also ends long silent builds).
2. Copilot waiting for your approval. Default: no hold, like Claude without Remote Control. Other: hold up to 1 hour.
   Later note: Codex and Copilot moved to after v1 (Haik, 2026-11-03; `02-06-lighter.md`). Answers 1 and 2 apply
   when they are built.
3. Chimes when your sound scheme is "No sounds". Default: honor it, no chime, a log line. Other: always play.
4. Notices raised while you are locked or another user is on the screen wait until you are back. Default: yes.
5. Wakeshot is off after `winget upgrade` until the next sign-in. Default: accept; the README says so.
6. The data folder at uninstall. Default: delete it after a good restore; after a failed one keep the journal and logs.
7. A notice after Wakeshot updated itself. Default: yes. A yes needs one more contracts change, run alone, early.
8. Clang on this laptop, for ARM64 builds once `velopack` is in. Default: add the Build Tools clang component.
   Other: none; the tray's ARM64 checks run only in CI.
9. A public pre-release to test the release workflow. Default: yes; Phase 8 deletes it.
10. Your power checks (two battery runs) in Phase 3, not at integration. Default: Phase 3.
11. Still open from Phase 1, cards build the default: Question 10 (two users: override only while your session is on
    the screen), 11 (the crash gap: accept, the README lists it), 12 (a failed restore starts the tray: yes).
12. Uninstall leaves Windows' own record of Wakeshot's tray icons (`NotifyIconSettings`). Default: leave it, the
    README says so. Other: the uninstall hook removes the three entries.
