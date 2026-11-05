# Screen session extras: Settings icon, test leftovers, typing rerun, floor grace check

Phase: 6 (integration checks). Status: ready. Needs Haik: yes, about 20 minutes at the screen, in the same session as
`p6-notice-app-icon` Part B (dock in, lid open, unlocked). Agent: `wakeshot-worker`. Size: about 80K tokens on top of
`p6-notice-app-icon` Part B. Written by the orchestrator, 2026-11-05, from `p6-int-haik-checks` "Found outside
scope" 4, 9 and 10 and `p2-6-floor-grace` "Found outside scope" 3. Haik's answers (2026-11-05): yes to deleting the
key and the leftovers, yes to both reruns.
Runs after: none; runs as a bundle with `p6-notice-app-icon` Part B (one worker, the order below). Parallel with:
nothing that builds, changes power settings or edits `packaging/`.
Real laptop: yes: the plan copy of `p6-notice-app-icon` Part B (step 3), a test install, Haik's own values never
written.

## Goal

Use the one screen session for four small leftovers of the integration checks: a fresh Settings icon, an empty
`%TEMP%`, a real typing test with the lid closed, and Haik's check of the 30 s floor grace.

## Read first

- `docs/tasks/p6-notice-app-icon.md`, Part B (the frame of this session; its steps 1 to 10 stay as written).
- `docs/tasks/p6-int-haik-checks.md`: "Found outside scope" 4, 9, 10; Part A step 5 (the `floor-on`, `floor-off`
  scripts); Part B blocks 4 and 6 (the original steps).
- `docs/tasks/p2-6-floor-grace.md`: Goal, "Found outside scope" 3; `docs/product.md` "Battery floor".
- `CLAUDE.md`: Test laptop, and the facts on elevation.

## Deliverables

Order inside the bundle:

1. `p6-notice-app-icon` steps 1 and 2 (guard, BEFORE). Add to BEFORE: the values of
   `HKCU\Control Panel\NotifyIconSettings\2334163346184210159`, and the list of `%TEMP%\wakeshot-*` folders and
   `%TEMP%\velopack` with their times.
2. Finding 10: delete the ten `%TEMP%\wakeshot-*` folders dated 2026-11-04 about 04:16 and the `%TEMP%\velopack`
   folder dated 2026-11-04 about 06:37. Anything with another date, or anything under `%TEMP%\claude`: leave it and
   list it. Record what went.
3. Finding 4: `reg export` the key above into the scratch folder, then delete it. This must happen before the tray's
   first start in step 6 of `p6-notice-app-icon`, so Windows takes a fresh snapshot at the icon's first add.
4. Before any cargo command: if the Visual Studio Installer runs (Haik is adding clang now), wait until its process
   ends (check every 30 s, up to 40 minutes; then stop and say so). Then `p6-notice-app-icon` steps 3 to 5 as written.
   Also write in the scratch folder `floor-on.cmd` and `floor-off.cmd` for the plan copy, as `p6-int-haik-checks`
   Part A step 5 did. Pick a threshold that puts the floor in force at the current charge, by the rule in
   `wakeshot-core` `policy/floor.rs` and the values Windows accepts; on the copy only. If no value does at 99 or 100%,
   the script says so and waits on battery as before. `floor-off` puts the copy's value back. Run each once and read
   the value back, then set it back.
5. Set Status `blocked` with Haik's steps for the first part (`p6-notice-app-icon` steps 6 and 7), and stop. The
   orchestrator relays and sends Haik's answers back.
6. `p6-notice-app-icon` steps 8 and 9 (the second start). Then, with that tray running, block for Haik's second part,
   in this order:
   - **Settings look:** Settings > Personalization > Taskbar > Other system tray icons. Say what picture the
     Wakeshot entry has: the Sunrise (violet square, gold sun), the coffee cup, or something else. Then read the key
     again and keep its new `IconSnapshot` as PNG, as `p6-int-haik-checks` Part C did.
   - **Typing rerun (finding 9, about 10 minutes):** block 6 step 3 as written, with these changes: open a new
     Terminal window first (the install added the Path entry), hover over the icon first and wait until it says
     "Not holding" (a working Claude Code session holds it), and type for 7 full minutes after the lid close by the
     clock. Expected: no "going to sleep" sound, no sleep, the external screen stays on.
   - **Floor grace check (about 12 minutes):** write the exact steps from `p2-6-floor-grace` "Found outside scope" 3:
     on the dock and under the floor (`floor-on`), Keep awake > 30 minutes; a dock drop under 2 s gives nothing; a pull
     left out gives the battery chime and the "Battery low" notice once, at about 30 s; back in, holding again with no
     sound; then lid closed (lid chime, lock), pull the dock and leave it out with the lid closed for 7 minutes, touch
     nothing, and note when the battery chime comes and whether and when Windows sleeps (the open fact: at the
     restore, or after the 5 minutes). Then `floor-off`, Keep awake > Off, dock in.
   Each step says what to do, what to see and hear, and what to report, as `p6-int-haik-checks` Part B does.
7. After Haik's answers: read the tray log and the Kernel-Power events for the two reruns (as `p6-int-haik-checks`
   Part C did), then `p6-notice-app-icon` step 10 (uninstall, plan back, AFTER equals BEFORE).
8. Facts in `docs/research.md` "Integration checklist with Haik" (additions only, above the "Battery floor grace
   checks" heading if that one is still last; `p2-6-floor-grace` "Found outside scope" 2). Results in this card and
   in `p6-notice-app-icon`.

## Acceptance checks

- The leftovers and the key are gone, and nothing else was deleted.
- The typing rerun and the floor check each have Haik's words and the log's times.
- AFTER equals BEFORE: the active plan, Balanced's lid and sleep values and its `/qh` hash; no install traces.

## Out of scope

- Any code change. A result that differs from the expected one goes to "Found outside scope" with the evidence; the
  orchestrator decides.
- Haik's own power values: never written.

## Questions for Haik

None yet.

## Found outside scope

## Log

- 2026-11-05: card written by the orchestrator.

## Result
