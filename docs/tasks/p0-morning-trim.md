# Cut Haik's morning steps to the minimum

Phase: 0. Status: done. Needs Haik: no (he does the cut list later). Agent: `wakeshot-worker`.
Runs after: Haik's answers to decisions 1 to 3 (in part 1 of `docs/reviews/p0-close-proposals.md`).
Parallel with: `p0-close-recheck` (it edits only `docs/reviews/p0-close-proposals.md`). Not with any spike run.

## Goal

Haik asked for the minimum manual work (2026-11-03): he does only what a human must do. Today's list
(`docs/phases/00-morning-steps.md`) takes about 2 hours plus 35 minutes of optional steps. Cut it to the steps that
need his body or eyes and feed a decision that is still open.

## Read first

- `docs/phases/00-morning-steps.md` (all).
- `docs/reviews/p0-close-proposals.md`: part 1 (Haik's answers) and part 3 (which step feeds which finding, and the
  "No consumer" line).
- "Steps for Haik" and the reading guide in `docs/tasks/p0-spike-power-settings.md`, `docs/tasks/p0-spike-lid-sleep.md`
  and `docs/tasks/p0-spike-tray.md`.
- `docs/reviews/p0-spike-safety-verify.md`, only the parts on stopping early and restore, so the cut keeps them safe.

## Deliverables

1. A tag for every step and sub-step, in a table in this card's Result: **Haik** (needs a human: lid, charger, cable,
   power button, a real mouse click, his own eyes on something an agent cannot capture), **Agent** (an agent can do
   it: registry theme switch, screenshots read back as images, no-input waits, commands, log reads), or **Drop** (no
   open decision needs it after Haik's answers: heat data, wake timers, the older keep-awake call, the time limit).
   One short reason each.
2. `docs/phases/00-morning-steps.md` rewritten to only the **Haik** steps, in a safe order, with the safety parts
   kept: "Stopping early", restore, `checkpower` after each power block, the Final check. Keep the step letters so
   the cards' reading guides still match; say where a letter is gone. Fewest windows, fewest commands to type.
3. The keep-awake handshake at C2 and D1 goes away: the orchestrator stops the keep-awake (PID 11284, elevated) right
   before Haik starts. Say this in step 0 as an orchestrator line.
4. Each spike card: a short "Moved to agent" list of the **Agent** steps, with the order they must run in relative to
   Haik's steps (before, after, or no matter). Do not run them.
5. New time estimate for Haik at the top of the list.

## Acceptance checks

- Every step in today's list appears in the tag table.
- No **Haik** step is one an agent can do from this laptop.
- No step that feeds a "Still needed" item of part 3 is dropped. If `p0-close-recheck` has not tagged part 3 yet, use
  part 1 and part 3 as written and list your own reading in the Log.
- Every power block in the new list still ends with `checkpower`, and "Stopping early" still works for each block.
- The spike tool code is unchanged. Changing a tool is out of scope; if a cut needs a tool change, list it as a
  question.

## Out of scope

- Running any spike tool, any power setting change, any process stop.
- `docs/reviews/p0-close-proposals.md` (another worker edits it).

## Questions for Haik

Each with the worker's default. None blocks the list; it works with the defaults.

1. **T10, the Explorer restart.** An agent could do it, but your rule says agents end only processes they started,
   and it closes your open File Explorer windows. May an agent restart Explorer once for this test? Default: no, T10
   stays yours (1 minute, the first step).
2. **Commands in your blocks.** You paste 7 run commands (D4, E2, F1, F2, H1, C4, I2, plus F3 and H4 only if needed)
   and one `checkpower` after each block. An agent could start each run instead, but each would need a chat message
   from you and its answer at that moment (the handshake this card removes), and a live agent session. Default: you
   paste them.
3. **Two more values in `checkpower`** (a tool change in `spikes/morning-shell.ps1`, so not done here):
   `DelayLockInterval` and the battery Power mode, which the agents' Settings tour can touch and `checkpower` does not
   read today. Default: no change; the agents check both themselves, and your fallback after the Final check has one
   extra line for `DelayLockInterval`.

## Found outside scope

- `docs/plan.md:90` still tells the orchestrator to stop the keep-awake at C2 or D1 when Haik says so. Now it is
  step 0, before Haik starts.
- `docs/phases/00-facts.md:16-19` map the spike cards to "blocks A, B, C", "block T" and "blocks D to K, after A to
  C". Now: power = C and optional B5 (A and B by agents); tray = T10 and T13; lid = D to I in the order D, E, F, H,
  then C and I.
- The orchestrator's chat to Haik at 09:37 (seen in my screenshot) says he will double-click `morning-shell.cmd` and
  that step 0 says so. The new step 0 has the orchestrator open it for him.
- Haik's volume is 24% (read 09:45); the old C1 wanted about 50% for the lid-close sound. Step 0 now has the
  orchestrator raise it and put it back.

## Log

- 2026-11-03: card written by the orchestrator.
- 2026-11-03: worker started. Read CLAUDE.md, plan "How we work", the card, `docs/phases/00-morning-steps.md`.
- 2026-11-03: read `p0-close-proposals.md` parts 1 and 3 (part 3 has no "Still needed" tags yet, so I use it as
  written), the three spike cards' steps and reading guides, and the stop and restore parts of
  `p0-spike-safety-verify.md`. Next: check what an agent can do here (Medium launch, screenshot, theme, tray code).
- 2026-11-03: my reading of part 3 (no tags yet): items 1 to 8 all still needed after Haik's answers. Decision 3
  drops the lid-closed time limit and decision 2 drops the heat guard, so nothing reads the external screen count or
  closed-lid heat any more. Step D still settles trap 12 (charger and dock unplugged with the lid closed), which
  decision 3's "a closed laptop holds as long as the work runs" depends on, and item 6 names it. "No consumer": G, I3,
  C7. Block T: results go to `p0-spike-tray` (Phase 5 inputs), so I keep its open questions but move what an agent
  can do. Tray code read: interactive mode captures icon and menus by itself on the `ImmersiveColorSet` broadcast and
  quits on `WM_CLOSE`. The lid spike's lid-close sound is the same `SystemNotification` alias as T5's first sound.
- 2026-11-03 09:37: read-only probe (script `probe.ps1` in my scratchpad): a screenshot of the screen saved and read
  back as an image (session unlocked, Haik at the orchestrator chat); UI Automation loads in pwsh 7 and finds the
  taskbar; theme now Dark (`AppsUseLightTheme` 0, `SystemUsesLightTheme` 0); on AC, 100%; `powercfg /requests`
  shows only the keep-awake (SYSTEM, PID 11284, command line `keep-awake.ps1`); the tray spike's
  `NotifyIconSettings` key has no `IsPromoted`; `SystemNotification` plays `Windows Background.wav`. The tray spike
  holds no power request (no `PowerSetRequest` or `SetThreadExecutionState` in `spikes/tray/src`) and its logs append.
  `morning-shell.log` shows agents opened the morning shell through `explorer.exe` at 03:41, 03:56, 04:26 and 04:34:
  Medium every time, in Windows Terminal since 04:26.
- 2026-11-03: `p0-close-recheck` has now tagged part 3: items 1 to 8 "Still needed"; moot: D3 heat run, the heat
  value from I, the screen count and "did the DELL stay on" (D5); D6 and D7 still needed (trap 12). Same as my
  reading above.
- 2026-11-03 09:45: Lenovo Vantage is installed (`E046963F.LenovoCompanion` 10.2608.62.0, Start app id
  `E046963F.LenovoCompanion_k1h2ywk1493x8!App`). Microsoft Learn "Customize the Windows performance power slider"
  (read today) names `ded574b5-45a0-4f42-8737-46345c09c238` "Best Performance", so A6's label check is not needed.
  Volume read through Core Audio from this session (`volume-read.ps1` in my scratchpad, read only): 24%, not muted.
  Not tried: driving Settings or Vantage with UI Automation, a registry theme switch, synthetic input. I did not
  switch the theme: Haik was at the screen, and a flip to Light would have been in his face.
- 2026-11-03: plan: Haik keeps T10, D, E, F, H, C, I (C and I back to back on one battery run), the sound notes, the
  Final check, optional B5 and T13. Agents take 0, A, B (but B5), the rest of T, all log reads and notes. Drop D2, D3,
  "DELL stayed on", the C2 and D1 handshakes, K1, T5, G, I3, C7. Writing the new list next.
- 2026-11-03: `docs/phases/00-morning-steps.md` rewritten (about 80 minutes, about 30 hands-on). Next: "Moved to
  agent" in the three spike cards, `research.md`, then the tag table in Result.
- 2026-11-03 09:50: "Moved to agent" written in the three spike cards; `research.md` section "Agent reach on this
  laptop" added; tag table, questions and outside-scope notes written. Acceptance checks run (Result). Done.
- 2026-11-03 10:10: round 2. Review `docs/reviews/p0-morning-trim-safety.md`: not safe to start, 5 majors, 8 minors,
  all doc edits. Fixing all in the list and the three "Moved to agent" lists. Key change for major 1: the agents' A
  tour now runs on the `copy-test --keep` copy (B1 first), so Settings writes land on a throwaway plan that
  `restoreall` removes; only `DelayLockInterval` (HKCU) and the AC Power mode overlay are real, and both get their own
  checks. Spike code checked for this: `watch` is not a write mode (no refusal with a copy active, `main.rs:144-160`)
  and snapshots the active plan plus `DelayLockInterval` and both overlays (`watch.rs:50-104`).
- 2026-11-03 10:40: round 2 done. List, three "Moved to agent" lists, tag rows, Result ("Round 2" table), research.md
  volume note updated. New question 3 (`checkpower` reading two more values; default no). No dashes; no spike file
  touched. Haik's time unchanged (about 80 minutes, about 30 hands-on); agents' screen part after him about 60.

## Result

Status: done. Haik's list went from about 2 hours plus 35 minutes optional to **about 80 minutes, about 30 of them
hands-on** (23 minutes he may walk away in C and H; 26 minutes near the closed laptop in E and I). Optional: B5 (5
minutes), T13 (1 minute).

### Tags

Haik = needs a human; Agent = an agent can do it (B = before Haik starts, A = after his Final check, never during his
blocks); Drop = no open decision needs it after Haik's answers. Part 3 tags are from `p0-close-recheck`.

| Old step | Tag | Reason |
| --- | --- | --- |
| Stopping early 1 to 5 | Haik (kept) | Safety path for when he stops; an agent may be gone |
| "To stop any run", "If the PC restarted" | Haik (kept) | Same safety path |
| Manual restore 1 to 7 | Haik (kept) | Fallback when `restoreall` fails and no agent is there; step 5 now says Best performance |
| 0.1 checks and BEFORE lines | Agent, B | Reads and card writes |
| 0.2 open the morning shell | Agent, B | `explorer.exe morning-shell.cmd` gives Medium in Windows Terminal (`morning-shell.log`, 4 agent launches) |
| 0.3 check token and ALL OK | Agent, B | `morning-shell.log` has the integrity; the agent reads the values itself |
| 0.4 use this window | Haik | Instruction only |
| A intro (charger in) | Agent, A | Precondition the agent checks |
| A1 `spike watch` | Agent, A | Command with a Medium token |
| A2 battery sleep in Settings | Agent, A | Settings through UI Automation (not tried), on the B1 throwaway copy, put back at once |
| A3 energy saver Never, Always | Agent, A | Same; feeds the battery floor rule (`product.md:56`) |
| A4 lid on battery in Settings | Agent, A | Same |
| A5 sign-in option | Agent, A | Same; feeds "no lock if Never" (`product.md:53`); a UAC or PIN prompt would send it back to Haik |
| A6 Power mode | Agent, A | Label known from Microsoft Learn (Best performance); change and back through UI Automation |
| A7 end the watch | Agent, A | Let it end by itself, or Ctrl+C with `GenerateConsoleCtrlEvent` (tested, `research.md`) |
| A8 read lines, fix STILL CHANGED | Agent, A | Log read; the elevated agent can write every value, CONSOLELOCK too |
| A write-down | Agent, A | Recorded in the power card |
| B1 `copy-test --keep` | Agent, A | Command; runs first, so the A tour writes land on the copy |
| B2 `checkpower` 3 red lines | Agent, A | Command |
| B3 Power mode with a copy active | Agent, A | Done together with A6; UI Automation read plus screenshot; feeds part 3 item 4 |
| B4 Lenovo Vantage screenshots | Agent, A | App launch, UI Automation Invoke on named items, toggles read before and after (not tried); feeds item 4 |
| B5 restart with the copy | Haik, optional, last | Sign-in after a restart; it also ends the agent session; item 4 lists it |
| B6 `spike restore`, B7 `checkpower` | Agent, A | Commands |
| C1 close media apps, volume | Haik once in step 0, plus Agent, B | Only a human may close his apps (Chrome held requests at 09:50); the orchestrator checks `powercfg /requests`, logs it every 30 s, and checks the Speakers volume by endpoint |
| C2 keep-awake handshake | Agent, B | Orchestrator stops PID 11284 before Haik starts (deliverable 3) |
| C3 unplug the charger | Haik | Charger |
| C4 `spike hold` | Haik | Starts the run right after his unplug; see question 2 |
| C5 hands off 15 minutes | Haik | He must not touch it (may walk away) |
| C6 wake, sign in | Haik | Sign-in |
| C7 `checkpower` | Haik (kept) | Safety check per block; charger now stays out for I |
| C8 orchestrator check | Agent, A | After the last agent end check, values read live |
| C write-down (log name) | Agent, A | Newest `out\hold-*.log` |
| D intro (hard desk, never leave) | Haik (kept) | Safety |
| D1 handshake | Agent, B | Step 0 |
| D2 `restoreall`, `checkpower` | Drop | Step 0 checks the same; the lid tool refuses a leftover copy or journal |
| D3 heat run | Drop | Decision 2: no heat guard (recheck: moot) |
| D4 command | Haik | Starts the run he acts on; see question 2 |
| D5 close lid, sound | Haik | Lid, ears; item 6 |
| D5 "did the DELL stay on" | Drop | No rule counts screens any more (recheck: moot) |
| D6 unplug the DELL | Haik | Cable; trap 12, still needed |
| D7 charger out and in | Haik | Charger; trap 12 |
| D8 open lid, DONE, hand on bottom | Haik | Lid; hand check stays for test safety |
| D write-down | Haik (sound only) | Ears |
| E1 charger in, DELL out | Haik | Cables (already so after D) |
| E2 command | Haik | See question 2 |
| E3 close lid, listen | Haik | Lid, ears; items 6, 7 |
| E4 4 minutes closed, stay near | Haik | Lid; safety |
| E5 open, wake if black | Haik | Lid, key or power button |
| E5 "note what you see" | Agent, A | `locked` field and display events in the log |
| E6 sign in, DONE, hand on bottom | Haik | Sign-in; safety |
| E write-down | Haik (sound only) | Ears |
| F1 sleep call, power button | Haik | Power button, sign-in; item 2 |
| F2 hibernate call, power button | Haik | Same |
| F3 screen-off (if F1 did not sleep) | Haik | Key, sign-in |
| F write-down | Agent, A | RESULT lines |
| H1 command, Start > Power > Sleep | Haik | A manual sleep is the test; wake needs the power button; items 3, 7 |
| H2 power button, sign in | Haik | Same |
| H3 hands off 8 minutes, key, sign in | Haik | Same |
| H4 `--retake` if needed | Haik | Same |
| H write-down | Agent, A | Log |
| I1 unplug the charger | Haik (changed) | Now: check the charger is still out; the lid tool does not refuse on AC |
| I2 command | Haik | See question 2 |
| I3 close lid 22 minutes, stay near | Haik | Lid; items 1, 5 to 8 |
| I4 open, sign in, hand on bottom | Haik | Sign-in; safety |
| I4 "note what you see" | Agent, A | Log |
| I5 charger and DELL back in | Haik | Cables |
| I write-down | Haik (sound only); heat note Drop | Heat fed only the heat guard |
| J1/K1 `restoreall`, `checkpower` | Drop | I6 ran `checkpower`; the Final check runs both |
| J2 tell the orchestrator | Haik | Only he heard the sounds; D's from the DELL, the laptop or none |
| K3 orchestrator check | Agent, A | After the last agent end check, values read live |
| T intro | Haik (short note) | Tells him the tray circle is the spike |
| T1 start `trayspike` | Agent, B | Medium launch through Explorer |
| T2 where it landed, drag to taskbar | Agent, B | Log rect; `IsPromoted` on its own key or a synthetic drag (not tried) |
| T3 tooltip, menu, submenu | Agent, A | Synthetic hover and right-click, screenshots; a real right-click was already done in the `--idle 60` run |
| T4 balloon, toasts, Win+N | Agent, A | Menu items, screenshots, synthetic Win+N (not tried) |
| T5 sounds by ear | Drop | Both `PlaySound` TRUE; the alias is heard at lid close in D, E, I; the WAV is a placeholder for Phase 5 |
| T6 Automatic | Agent, A | Screenshot of the icon |
| T7 Light, hands off | Agent, A | Settings through UI Automation, else registry plus `ImmersiveColorSet` broadcast (not tried) |
| T8 check icon and menu | Agent, A | The spike's own captures, read as images |
| T9 back to Dark, check | Agent, A | Same; both registry values read back 0 |
| T10 Explorer restart | Haik | His rule forbids an agent ending Explorer; see question 1 |
| T11 high contrast (optional) | Drop | Optional; turning it off is not verified to bring back `Custom.theme`; Phase 5 |
| T12 Quit | Agent, A | `WM_CLOSE` to `WakeshotTraySpike` |
| T13 do the pictures look native | Haik, optional, any time | His taste; tray card acceptance "Haik has seen them" |
| T write-down, token check | Agent, A | Log reads |
| G1 to G4 wake timer | Drop | Part 3 "No consumer" |
| I3 wake timer on battery | Drop | Same |
| Optional older keep-awake run (old optional "C7") | Drop | Same |
| Final 1 `restoreall`, `checkpower` | Haik (kept) | The card keeps the Final check |
| Final 2 orchestrator check | Agent, A | Quick check, then the agent steps one at a time, end check, AFTER lines, "agent steps done" |
| Final 3 close the morning shell | Agent, A | The orchestrator opened it and closes it |
| New: wait for "agent steps done"; fallback after 90 minutes | Haik | Only if no message comes: `restoreall`, `checkpower`, one `DelayLockInterval` line |

Agent steps that must run before Haik starts: 0.1 checks and BEFORE lines, T1 and T2 (tray spike started and on the
taskbar), battery and the Speakers volume, Haik closing media apps, keep-awake stop with `powercfg /requests` and
`limited read`, the 30 s requests log, opening the morning shell, then "go". Everything else of the agents runs after
his Final check, one agent on the screen at a time: power B and A, then tray; lid log reads alongside.

### Files changed

- `docs/phases/00-morning-steps.md`: rewritten to the Haik steps, new order T10, D, E, F, H, C, I, step 0 as
  orchestrator lines (keep-awake stop included), `checkpower` at the end of every block, "Letters that moved or are
  gone" table, new time table.
- `docs/tasks/p0-spike-power-settings.md`, `p0-spike-lid-sleep.md`, `p0-spike-tray.md`: "Moved to agent" lists with
  before and after order; the stale block lists in "Steps for Haik" corrected.
- `docs/research.md`: new section "Agent reach on this laptop".
- This card.

### Acceptance checks

- Every step in today's list is in the tag table: done (all 0 to Final steps, sub-steps, write-downs and optional
  steps).
- No Haik step is one an agent can do: met with two named exceptions. T10 is agent-doable but blocked by Haik's
  process rule (question 1). The run commands are agent-doable but need his moment (question 2). The kept safety
  parts (stop rule, manual restore, `checkpower` per block, Final check) stay his because the card keeps them.
- No step that feeds a "Still needed" item is dropped: checked against the recheck's tags (items 1 to 8). Dropped
  steps feed only moot parts (D3, D5 screen count, heat note) or "No consumer" (G, I3, the older keep-awake run), or
  repeat a check (D2, K1), or the tray card only (T5, T11).
- Every power block ends with `checkpower`: D9, E7, F4, H5, C7, I6, then the Final check. "Stopping early" applies to
  each: Ctrl+C, `restoreall`, `checkpower`. The agents' B and A end with `restore` and an end check, and have their
  own stop rule.
- Spike tool code unchanged: no file under `spikes/` outside logs and `target` changed in the last 3 hours (checked
  09:48).

### Open items

- Not tried by me (the card forbids it or it would have flipped Haik's screen): UI Automation in Settings and Vantage,
  the theme switch, synthetic input, `IsPromoted` applied live, a volume write. Each agent step that fails goes back to
  Haik as a short optional step; the cards say so.
- After the Final check Haik leaves the laptop on AC and unlocked for about an hour for the agents, and does not
  unplug it or close the lid until "agent steps done" (the list says so).
- Round 1 safety review: `docs/reviews/p0-morning-trim-safety.md` (not safe to start, 5 majors, 8 minors). Round 2
  fixes are below; a recheck by a reviewer is the next step.

### Round 2: safety review fixes (2026-11-03, about 10:10 to 10:40)

Every finding fixed in the list and the three "Moved to agent" lists; no spike run, no setting change, no process
stop, no tool code change.

| Finding | Fix |
| --- | --- |
| 1. Agents change the real plan after Haik is done, no end check | Power card: B1 runs first, so the A tour writes land on a throwaway copy that `restoreall` removes; only `DelayLockInterval` and the AC overlay are real, and both are checked. Each change is put back at once. A stop rule for agents (also for a new session that finds "started" without "done"). AFTER lines after the last end check, values read live. Haik waits for "agent steps done" before he unplugs or closes the lid; fallback after 90 minutes with `restoreall`, `checkpower` and one `DelayLockInterval` line. |
| 2. Tour content lost | A2 to A6 written into the power card: page, control, test value, put-back, plus the old put-back table for `STILL CHANGED`. Battery Power mode: never touched; `ActiveOverlayDcPowerScheme` and `DelayLockInterval` read before and after. |
| 3. Power and tray agents on one screen | One agent on the screen at a time, in both cards and in the Final check: power B and A, then tray; lid log reads alongside. |
| 4. Display request without a cap | One holder per agent step, in the `keep-awake.ps1` pattern: ends itself after 45 minutes or on battery, PID file, stopped by PID; the end check's `powercfg /requests` must not list it. |
| 5. Power requests checked once | Step 0: Haik closes media apps once (Chrome held DISPLAY and EXECUTION). The orchestrator logs `powercfg /requests` every 30 s (read only, capped at 3 hours, stopped at the Final check); a spoiled block from D to I is redone alone. Reminder lines at the top of D and C. After a break, the orchestrator repeats the step 0 checks and says "go". |
| 6. Volume on the DELL | Read and write by endpoint ID; Speakers 67%, DELL 24%; change nothing unless the Speakers are under 50%. J2 asks whether D's sound came from the DELL, the laptop, or nothing. |
| 7. "Stop" on heat unclear | One "Too hot" line in D, used by D8, E6 and I4: lid open, `checkpower`, let it cool, tell the orchestrator. |
| 8. I on AC unnoticed | I1: check the battery icon shows no plug. |
| 9. B5 after the shell is closed | B5 step 0: double-click `morning-shell.cmd`; "About this list" names all three waits. |
| 10. Workers not checked | Step 0 check 1: no worker or reviewer agent running. |
| 11. No lock check | Both cards: check `LogonUI` before every UI Automation action or synthetic input; on a lock, put back and wait. |
| 12. Small leftovers | T12 deletes `IsPromoted`; T11 dropped; Vantage only through UI Automation Invoke on named items, toggles read before and after. |
| 13. Naming drift | The dropped optional run is "the older keep-awake run (old optional C7)"; C7 now means only the kept `checkpower` step. Lid card says D4 to D9. |

Haik's time is unchanged: about 80 minutes, about 30 hands-on (step 0 now has 1 minute for closing media apps). The
agents' screen part after his Final check grew from about 40 to about 60 minutes, because they now run one at a
time.
