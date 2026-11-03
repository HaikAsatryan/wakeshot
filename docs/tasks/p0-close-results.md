# Apply the morning results to the design docs

Phase: 0. Status: done. Needs Haik: no (his answers are below). Agent: `wakeshot-worker`.
Runs after: the delta review (`docs/reviews/p0-delta.md`) and the M1 battery check (done 13:29). Parallel with: none.

## Goal

Close Phase 0 on paper: apply the delta review's section 1 edits and the small fixes it lists, so `product.md`,
`architecture.md` and `research.md` match every fact found today.

## Read first

- `docs/reviews/p0-delta.md`: findings M1 to M3, minors m1 to m10, section 1 (part 3 items with exact edits),
  sections 4 to 6, "Before Haik signs off Phase 0".
- `docs/reviews/p0-close-proposals.md` part 3 and item 32 (to mark what is applied).
- The target files at the lines the edits name; find each by its old text.

## Answers and facts (2026-11-03)

- M1 passed: `power-settings-spike hold --sleep-dc 0 --minutes 16` on battery, request held, copy with battery sleep
  Never, 13:13:03 to 13:29:05. Display-off standby entry at 13:16:04 (Kernel-Power 506), `gaps: none` for the full
  16 minutes (past the 900 s where I failed), plan restored, copy removed (`spikes/power-settings/out/hold-20261103-131303.log`).
  So item 1's edit applies without the "Not tested" sentence; say it was tested with Never only.
- M2: Haik took the default: no sound at lid close; a notification at the next sign-in after a closed-lid hold.
  (Superseded 2026-11-03 14:45: the orchestrator had read Haik's silence as a yes. Haik wants sounds wherever they
  help, the lid-close sound included. The orchestrator reverted the `product.md` line; card `p1-lid-sound`.)
- M3: the review's default (Keep screen on keeps its idle lock) stands.
- B5 dropped (in place stays). Item 32 is moot.

## Deliverables

1. Section 1 items 1 to 7 of the delta review applied, item 6 with the M2 answer, item 3 with the M3 fix.
2. m2 added to `research.md`; every other minor that is a short doc edit, applied. The four tool misreads (m1): note
   them in `research.md` next to the results they affect; do not change spike code.
3. In `docs/reviews/p0-close-proposals.md`, under each part 3 item and item 32: one line "Applied 2026-11-03" or
   "Moot: <reason>". Change nothing else there.
4. Spike cards: only stale status lines the review names (m10), not their Results.

## Acceptance checks

- Every section 1 item and every minor has "applied" or a reason in this card's Result.
- No edited line contradicts Haik's decisions 1 to 3 (part 1 of the proposals file) or the M1 to M3 answers above.
- `CLAUDE.md` facts still true; if one changed, say so in the Result (the orchestrator edits `CLAUDE.md`).

## Out of scope

- `CLAUDE.md`, `docs/plan.md`, `docs/phases/*` (the orchestrator edits those; give exact edits in the Result).
- Spike code, any spike run, any power setting.

## Questions for Haik

None. Nothing here needs a product decision.

## Found outside scope

Problems seen but not fixed.

- Stale line references (lines moved when `research.md` grew): `product.md` "Power behavior" cites `research.md:485`
  for the 113 C trip point (now about line 572, "Temperature without admin"); `architecture.md:40` cites
  `research.md:381` (now about 430) and `architecture.md:47` cites `research.md:378` (now about 427). Fix: cite the
  section name instead of the line.
- `p0-spike-power-settings.md` line 3 ("Needs Haik: only the optional B5") and line 345 ("Open: only the optional
  B5") are stale: B5 is dropped. Not named in m10, so not edited. Suggested: "Needs Haik: done (B5 dropped
  2026-11-03: in place stays)." and "Status: done."
- `p0-spike-power-settings.md` lines 378-380 (in its Result) still recommend a registry watch. The delta review (m6)
  asks for a note under it; deliverable 4 rules out edits to spike Results. Suggested note: "(Not adopted:
  `architecture.md` Runtime polls every 10 s, part 2 item 19; no fact shows that 10 s is too slow.)"
- "Sounds" is still listed as an output in `product.md` UX ("Tray icon, tray menu, notifications, sounds"),
  `architecture.md` lines 15 and 26, and `plan.md` Phase 5. After M2 I found no product sound left. Phase 5 or Haik
  decides whether the word stays.
- The power spike's `hold` mode reads a battery sleep of 0 as a 0 s timeout in its start lines. Throwaway spike, so
  not fixed (as m1 says); noted in `research.md`.

## Log

- 2026-11-03: card written by the orchestrator.
- 2026-11-03: worker started. Read the card, `plan.md` "How we work", and the delta review in full.
- 2026-11-03: read `product.md`, `architecture.md`, the proposals file parts 1 and 3, `research.md` power and lid
  sections. Read the M1 log (read only): start 13:13:03.950 idle 29 s, last tick 13:29:04.981 idle 990 s, no tick
  step over 6 s, 506 Idle Timeout 13:16:04.6, `gaps: none`, restore AC 10800 DC 600 lid 1/1. Its plan lines misread
  sleep.dc 0 as a 0 s timeout ("requests end about 300 s after the last input"); noted for `research.md`.
- 2026-11-03: read the lid and power card status lines and `00-facts.md`. Lid card header already says done; its
  Result line 265 and Checks line 360 are stale (m10). The power card's registry-watch line (m6) is in its Result, so
  deliverable 4 rules it out. Next: check the m1, m2 and m9 log lines read only, then edit.
- 2026-11-03: checked read only: every m1 log line (H1 has 744 ticks, all `[before-sleep]`; F lines say "Slept"; I
  line 729; C line 113 at the resume), m2 (requests log 10:58:45 and 10:59:15 list the spike, H run 2 has 61 hold
  ticks all 0x0; I 0x1 to the last tick), m9 (idle 1430 s and 945 s at the first tick after resume). `Get-WinEvent`:
  Kernel-Power 40 at 10:37:02.224 and 10:39:06.386, each 0.1 s before the 42; 105 AcOnline false then true at
  12:27:54.0 / 12:28:00.3 and 01:50:47.1 / 01:50:52.4.
- 2026-11-03: `product.md` edited: items 7, 1 (with M1), 3 (with M3 and m7's safe default), 6 (M2), 2 (floor and
  work end, inferences marked).
- 2026-11-03: `architecture.md` edited: items 2 (`SetSuspendState`), 5 (clock), 4 (HKCU copy, lid strategy).
  `research.md` edited: M1 result, m1 notes at each result, m2, m9, E release with the lid closed, Kernel-Power 40,
  night AC drop, B5 dropped, two stale "not tested" lines. Next: proposals file marks, lid card status lines.
- 2026-11-03: proposals file: part 3 items 1 to 8 marked applied, item 32 moot. Lid card Result and Checks status
  lines fixed. Acceptance checks run; Result written. Status done.

## Result

Status: done.

What was done: the delta review's section 1 and its minors are applied to `product.md`, `architecture.md` and
`research.md`, with the M1 to M3 answers. Each new line marks what was tested and what is inferred. Where the
review's text said "tested" for an inference, I marked it.

Section 1 items:

| Item | Outcome |
|---|---|
| 1 Battery timer | Applied, `product.md` Power behavior. No "Not tested" sentence; says tested with Never only, lid open. Added: a finite value only moves the point where requests end (Microsoft's text, not tested). Line ref replaced by the section name |
| 2 Hibernate | Applied: `product.md` work end and battery floor, `architecture.md` loop thread. "Apps run until the user's sleep timeout" marked inferred (not tested with the lid closed) |
| 3 Idle lock | Applied with the M3 fix (Keep screen on keeps its idle lock) and m7 |
| 4 In place | Applied, both `architecture.md` lines. The Control Panel page "applies to all plans" marked not tested. B5 dropped, noted in `research.md` |
| 5 Clock | Applied, `architecture.md` Runtime, plus the review's "Phase 1 picks the clock per duration" |
| 6 Sound | Applied with M2: no sound, a notification at the next sign-in. "Not heard" worded as "no test showed it was heard" |
| 7 Re-take | Applied; reason is the Windows fact; names the tested form (clear, then set on the same handle) |
| 8 Screen off | Confirmed, no edit |
| 32 | Moot (m8) |

Minors:

- m1: applied. Each misread noted next to its result in `research.md` (E was already noted; H run 1, F, I, C added,
  plus the M1 log's own misread). Spike code not changed.
- m2: applied, `research.md` "Resume" and "Battery rule".
- m3: applied with item 2.
- m4: applied with item 7; `research.md` "Resume" says "new before old" was not tested.
- m5: applied with items 1 and 2; the night AC drop added to `research.md`.
- m6: `architecture.md` part applied with item 4. The power card note is skipped (deliverable 4); see Found outside
  scope.
- m7: applied as a short safe default in `product.md`: lock until a test shows which value means Never.
- m8: applied (item 32 moot).
- m9: applied with item 5, and in `research.md` "Clock".
- m10: lid card Result and Checks status lines fixed (its header was already right). `plan.md` and `CLAUDE.md`: edits
  below, for the orchestrator.

Also added to `research.md`: in E, putting the lid action back with the lid closed did not sleep the PC; Kernel-Power
40 before both F hibernates (meaning not checked); two stale "not tested" lines updated (battery cure, Start > Sleep).

Files changed: `docs/product.md`, `docs/architecture.md`, `docs/research.md`, `docs/reviews/p0-close-proposals.md`
(part 3 marks only), `docs/tasks/p0-spike-lid-sleep.md` (two status lines), this card. Reads only: the M1, C, E, F,
H and I logs, the requests log, and `Get-WinEvent` for Kernel-Power 40 and 105. No spike run, no setting change.

Checks:

- Every section 1 item and every minor has an outcome above: pass.
- No edited line contradicts decisions 1 to 3 or M1 to M3: pass. Read the new Power behavior section in full, and
  searched the three docs for "time limit", "heat guard", "idle lock", "locks the PC", "Short sound" and the old item
  texts: no stale hit. One point for Phase 1: a finite battery timer value would act as a time limit on battery with
  no input, which decision 3 rules out. `product.md` now says so; Never fits.
- `CLAUDE.md` facts still true: pass. Test laptop values match the M1 restore read (AC 10800, DC 600, lid 1/1).

Edits for the orchestrator (out of my scope):

- `CLAUDE.md`, "Facts to keep in mind", add two lines (m10):
  - "On battery, Windows ends every power request about 5 minutes after the battery sleep timeout, lid open or closed.
    Battery sleep Never stopped it (tested 2026-11-03), so a hold must raise the battery timer."
  - "A user's own sleep (Start > Sleep) ends every request; after resume it is not honored until it is taken again."
- `docs/plan.md` Status: the line "Overnight keep-awake PID 11284 ..." (PIDs 33632 and 11944) and the lines from
  "`p0-close-apply` done; ..." to "(5) Haik signs off, Phase 1 starts." are stale. Replace both with: "- Morning
  steps, agent steps, delta review, M1 battery check and `p0-close-results` done (2026-11-03). No spike, holder,
  keep-awake or requests-log process left (lid card AFTER line, 13:08). Next: Haik signs off Phase 0, then Phase 1
  starts. Optional: a full review of the tray card's Result."
- `docs/phases/00-facts.md`: table row `p0-spike-power-settings`, "done by Haik (B5 optional)" to "done by Haik (B5
  dropped)"; row `p0-close-results` to "done"; Close, first box to "[x]" with "(cards `p0-close-apply` and
  `p0-close-results`)"; work log line "Renew by taking the new request before dropping the old one." append "(tested
  form: clear, then set on the same handle, H4; this order not tested)".

Open items for later phases (from the delta review; place them when Phase 1 and Phase 3 are planned):

- Phase 1: test which stored value means sign-in Never, on an account without Windows Hello (m7). Pick the battery
  timer value (Never fits decision 3).
- Phase 3 checklist: closed lid, no request, battery, past the 600 s sleep timeout (item 2); one sign-in delay other
  than "Every Time" (m7); re-take on battery; "new before old" if Phase 1 uses it; a change in Settings or the Control
  Panel lid page during a test with a copy is a real write to the user's plan (section 5).
