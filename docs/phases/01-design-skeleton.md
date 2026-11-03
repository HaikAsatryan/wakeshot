# Phase 1: Detailed design and skeleton

Goal: a detailed design Haik signs off, then a public repo with a Cargo workspace and CI that build and test, plus
the two Phase 0 leftovers that need the screen or sound: a tray menu that looks like Windows 11 and a lid-close sound
that is heard.

## Design summary for Haik

### Sign-off list for Haik

Read this list and "Values I set" below (values I chose without asking, each with its reason; change any) to sign
off. Reply "signed off", plus an answer to any question you want to change.

- **What the reviews found.** Five reviews (design, fixes, final check): 1 blocker, 14 major, 55 minor, all fixed or
  placed with a reason. Main risks: your saved power values lost after a failed read, write or plan switch, a closed
  laptop left running unlocked in a bag, a crash with nothing restoring, a public push with no last yes from you.
- **What changed.** A failed read or write never deletes your saved values: the file that holds them, the sign-in
  restore entry and the guard (a helper that restores if Wakeshot dies) stay until a restore really works (exceptions
  in Question 11). Your own change to a value Wakeshot overrides stops its writes on every plan until the work ends. A
  crash always ends the tray, so the guard restores. A closed laptop unplugged from the dock locks and chimes after
  5 s on battery (`product.md` updated). The secret scan runs before every commit and push, and checks itself.
- **Told, no answer needed.** Network names in the new tray menu pictures get cropped. "Lock at once, chime in
  parallel" is tested with the Windows sound, not yet with our chime (a Phase 3 check).
- **Going public.** Your 15:45 sign-off covers only the local build cards. `git init` and the first public push wait
  for this sign-off, then for one more yes from you right before them (Gate B below). A public push cannot be undone.
- **Questions 10 to 12** (answered 2026-11-03 18:09: the defaults below; details under "New questions"):
  - 10. Two users: while another user is on the PC's own screen, Wakeshot puts your settings back and keeps only the
    system power request; the same over Remote Desktop into your own session (likely; not tested). Beyond that, v1
    assumes one signed-in user at a time; the README says so. Risk accepted: an override left behind after a crash
    can keep another user's closed laptop running unlocked.
  - 11. Cases where your settings stay overridden longer: until Wakeshot next starts (the sign-in restore and then the
    tray fail), until a reinstall (an uninstall whose restore fails), or until you set them by hand (a damaged saved
    values file; until then a lid close gets no lock). Default: accept; the README lists them.
  - 12. A restore that fails for 2 minutes outside the tray starts the tray to warn you, even with start at login off
    or right after you picked Quit. That tray also works as usual: it overrides again at the next agent turn. Other
    option: it only retries and warns, then exits. Default: yes, works as usual.

### Full summary

Written by card `p1-detailed-design` (2026-11-03); both design reviews and Haik's answers worked in by
`p1-design-review-fixes`, both verify reviews by `p1-design-verify-fixes`. Haik signs it off before any product code.
Full design: `docs/architecture.md`.

What gets built:

- **Two programs, one install.** `wakeshot-tray.exe` sits in the tray and does all the work. `wakeshot.exe` is the
  command line. They talk only through small JSON files in a data folder outside the install folder, so an update or
  a `winget upgrade` never deletes them.
- **A decision core with no Windows code.** It gets events (an agent got busy, the lid closed, the PC woke up) with
  the time, and returns actions (hold, change a setting, lock, hibernate, update the icon). All timing rules are
  tested with a fake clock, and CI also tests it on Linux to prove it has no OS code.
- **A Windows layer runs the actions.** Nothing slow (files, power plan writes, hibernate) runs on the thread that
  owns the tray, so the tray never freezes.
- **Holding awake:** the two power requests Windows needs, plus a screen request only for "Keep screen on". After
  every wake-up they are dropped and taken again (the form tested in Phase 0).
- **Your power settings:** when a hold starts, Wakeshot sets the lid action to Do nothing and battery sleep to Never
  on your active plan, also on AC. Nothing else. Before each write it saves your value in a journal file; if that
  fails, it writes nothing. It puts back only values that still hold what it wrote; if you change one, yours wins on
  every plan until the work ends. One thread decides what is still Wakeshot's, in order, so a plan switch or a failed
  read never drops your saved values. While another user is on the PC's own screen, your values are back and only the
  power request stays.
- **Every way back to your settings:** work end, Quit, sign-out, next start, a small guard process that restores if
  the tray dies (a crash; End task is not tested yet: the guard starts outside the tray's group for it), a one-time
  entry that restores at your next sign-in, and the install, update and uninstall hooks. A restore that fails outside
  the tray is retried, then the tray starts to keep trying and warn you.
- **Lid closed while holding:** the PC locks at once and a short chime plays (no lock if your own lid action is Do
  nothing). The same when you unplug a closed laptop that then keeps running only because of Wakeshot, once it has
  been on battery for 5 s. At the next unlock a notice says how long it ran with the lid closed and why.
- **Work ends with the lid closed, or the battery reaches the floor:** your settings come back at once; then, after 5
  minutes with no input and a fresh check that the lid is still closed, a short sound and the laptop hibernates.
- **What holds:** Claude Code first (status files plus transcripts, only the listed fields, never prompts), then
  Codex and Copilot CLI, `wakeshot run`, and hold files any script can write. Every hold is tied to a live process or
  a due time. If an agent's file format changes, a notice says "update Wakeshot".
- **Notices** are tray balloons (Windows 11 shows them as normal banners, no setup). **Sounds** are Wakeshot's own
  chimes on the default device, or on the laptop speakers when the default is a screen's audio (`p1-lid-sound`).
  **How the menu is drawn** stays open behind a seam; `p1-tray-menu-look` picks (likely the classic menu with
  owner-drawn items and round corners).
- **Dependencies:** `windows` 0.62.2, `serde`, `serde_json`, `embed-resource`, `proptest` for tests, `velopack` in
  Phase 6. No async runtime. Static C runtime, so users need no Visual C++ install (checked by the workspace card).
- **CI:** GitHub Actions runs format, lint, tests and release builds on x64 and on a real ARM64 runner.
- **Not tested yet, and placed:** battery Never with the lid closed, the guard against Task Manager's End task, the
  sign-in restore entry, the restore at sign-out, the hibernate countdown, our chime with the lock at once, fast user
  switching, and more: a 16-item manual checklist, mostly for Phase 3 (architecture section 9).

Done under your ahead sign-off (15:45, local only): repo files, Cargo workspace, CI. Still to come: their review and
its fixes, then your yes to go public (Gate B), a secret scan of exactly what gets committed, and the first public
push (steps below).

### Values I set

Design values you have not seen as questions, each with why. Change any at sign-off.

- Claude timer margin, due time plus 2 minutes: fires come up to 1 s late and then a turn starts. It also ends a
  wakeup cancelled with Esc, so a cancelled wakeup keeps the PC awake up to 2 minutes.
- Safety tick every 30 s: re-checks input idle, due times, the plan and the liveness only the process list can see.
- Plan read every 10 s while overriding: how fast your own change to an overridden setting is seen. Windows has no
  documented change notice for these values.
- Sign-out: the tray waits up to 4 s for its restore. Windows ends an app that holds up the session end (exact limit
  not verified).
- Log: 1 MB, three files kept. Hold `reason` cut to 60 characters, to fit the tooltip.
- Battery floor: step back when the battery is below the threshold, as `product.md` says (the first draft said "at or
  under").
- A second signed-in user gets power requests only, no lid feature: power plans are machine-wide, so one tray owns
  them. That tray overrides only while its session is on the PC's own screen (Question 10 asks about both).
- Lid close: lock at once, chime in parallel. The lock did not stop the Windows sound at a tested lid close; our chime
  in this order is not tested yet (checklist 16).
- Unplugging a closed laptop that then runs only on Wakeshot's Do nothing: lock and chime once it has been on battery
  for 5 s, so a dock's short AC drop does not lock someone typing on an external screen. Someone who pulls only the
  charger while typing on an external screen with the lid closed is locked: the safe side.
- Every sound: the default device, unless it is a screen's audio; then the laptop speakers (your delegated call,
  picked by `p1-lid-sound`). Headphones stay as they are.
- Before it hibernates, Wakeshot waits at most 2 s for the lid state; an unknown lid never hibernates.
- A restore that fails outside the tray (guard, sign-in restore): retried every 10 s for 2 minutes, then the tray
  starts (Question 12). Each retry stops if another Wakeshot took the journal over. The uninstall hook retries within
  its 30 s limit.
- `wakeshot restore` with the tray running: the tray does it; the command waits up to 5 s for the answer.
- The guard starts with Explorer as its parent, so End task on Wakeshot's group should not end it (not tested).
- Journal: one file, no registry copy (the first draft had one); the atomic write covers a torn write.
- Two exe crates: from Phase 6 only the tray takes `velopack`.

### Questions for Haik (answered 2026-11-03 15:29)

1. **Which folders go public.** Everything, from the first push: docs, planning, reviews, spikes and their logs (Haik:
   "We need to go fully public"). Phase 8 removes internal files and squashes the history. Never public: secrets
   (tokens, keys, credentials, Claude's `.key` files, anything that grants access) and build output (`target/`). The
   privacy check became a secret scan of exactly what gets committed, before each push (steps below).
2. **Install and data folder names.** Pack id `HaikAsatryan.Wakeshot`, data folder `%LocalAppData%\Wakeshot` (the
   default). Whether Velopack accepts the dot: Phase 6 checks before the first release.
3. **Minutes with no input before Wakeshot hibernates a closed laptop:** 5 (the default).
4. **Command names:** the default set, now in `product.md`, UX.
5. **The crash gap:** accepted (the default); the README says so. Battery sleep Never is part of the gap: a closed
   laptop at the sign-in screen does not sleep, so its battery drains until someone signs in. Every path into the gap
   is in `architecture.md` section 5, "Gap left"; Question 11 asks about the paths found after your answer.
6. **The "sign-in Never" lock exception:** no test account; v1 always locks at lid close while holding (the default).
7. **"Update Wakeshot":** once per Wakeshot start for each agent, plus a log line (the default).
8. **Which actions get a notice:** the routine start and end of a hold show only in the icon and tooltip; a notice or a
   sound comes with everything else (the default). Which moments get a sound: `product.md` (Haik, 15:35).

Commit identity (general review M4): Haik's usual git identity stays. His Gmail address is already on his public
commits (`gh` search, 15:29), so no noreply address.

### New questions for Haik (from the review fixes)

Answered 2026-11-03 18:09 (Haik: all defaults), each as its default below.

9. **Worktrees in this repo.** Your global rule gives fan-out agents worktree isolation because many repos deploy on
   push. Here a push only runs CI, only the orchestrator runs git, and parallel cards write different files (their
   "Parallel with" lines). A worktree would also miss uncommitted work and ignored files, branch from the last push
   unless `worktree.baseRef` is `"head"`, and need a merge-back step (general review M1, from the Claude Code docs).
   Default: no worktree isolation in this repo, for workers or reviewers. Change it when an agent must run git, or two
   cards must edit one file at the same time. Before the first commit worktrees cannot work at all, so Phase 1 runs
   without them either way. **Decided (15:55)** by the orchestrator under Haik's delegation of parallelism: the
   default, with the conflict rules in `docs/plan.md` "Git".
10. **Two users on one PC.** Power plans are per machine; the journal and the sign-in restore are per user. Two cases:
    - Fast user switching. User A's agent holds; A switches to user B; B closes the lid and bags the laptop. A's Do
      nothing keeps it running, A's tray can lock only A's session, and B's tray reads Do nothing as B's own value: B's
      session stays unlocked in the bag. Default (safer, new): a tray overrides only while its session is on the PC's
      own screen; when another user takes it, it puts the values back and keeps only the system power request (no
      screen request, so B's screen still turns off and Windows locks B), and applies again when its user comes back.
      Remote Desktop into your own session most likely counts as leaving too, so over Remote Desktop you get requests
      only. If that restore fails, its warning shows in A's session, not to B; it is retried. Session notices not
      tested yet (Phase 3, checklist 11).
    - A's override left behind (A's session ended without a restore): B's tray reads Do nothing as B's own value: no
      lock, no countdown. Default: v1 assumes one signed-in user at a time and the README says so; a second user's tray
      holds requests only. The other option: an owner marker every user can read (for example in `%ProgramData%`;
      rights not verified). Change it when a user reports two accounts.
11. **More ways into the crash gap**, and how long each lasts. Until the next sign-in: an OS crash or a power cut;
    "Shut down anyway" before the sign-out restore ends; End task ending the tray and the guard together, if the
    guard's own parent does not keep them apart (not tested). Until Wakeshot next starts (a manual start, or the next
    sign-in with start at login on), because Windows deletes the one-time sign-in entry before it runs: the sign-in
    restore fails for 2 minutes and the tray it starts fails too, or the sign-in restore crashes. Until a reinstall,
    with no warning: an uninstall during a hold whose restore fails for its whole 30 s. Until you set the values by
    hand: a saved values file that reads but does not parse; it is moved aside with a warning, the log holds your old
    values, and until then a lid close gets no lock. Default: your "accept" covers these too, and the README lists
    them.
12. **A failed restore starts the tray.** When the guard or the sign-in restore cannot restore for 2 minutes, it starts
    the tray (even with start at login off) so the warning shows and the retries go on. Default: yes. Side effects: that
    tray also holds for agents as usual (Automatic is on by default), so with start at login off Wakeshot is back and
    overrides at the next agent turn; and after a Quit whose restore failed, Wakeshot comes back 2 minutes later. The
    other option: that tray only retries and warns, then exits. Without either, the override stays with no warning
    until you start Wakeshot.

## Task cards (`docs/tasks/`)

| Card | Needs Haik | Runs after | Status |
|---|---|---|---|
| `p1-tray-menu-research` | no | none | done (`docs/research/tray-menu-look.md`) |
| tray Result review (reviewer, no card) | no | tray card | done (`docs/reviews/p0-tray.md`): 0 blocker, 2 major, 12 minor |
| `p1-tray-review-fixes` | no | the two rows above | done |
| `p1-detailed-design` | yes: signs off the design before code | Phase 0 sign-off | done; waits for Haik's sign-off and answers |
| design reviews (two reviewers, no card) | no | `p1-detailed-design` | done: general (`docs/reviews/p1-design-general.md`: 1 blocker, 4 major, 13 minor); power and facts (`docs/reviews/p1-design-power.md`: 0 blocker, 6 major, 13 minor) |
| `p1-design-review-fixes` | only new judgment calls | general review; power review sent when done | done (15:51): 4 new questions (9 to 12) |
| design fix verify (two reviewers, no card) | no | `p1-design-review-fixes` | done (16:03): general 0/1/9, power 0/2/12 (blocker/major/minor) |
| `p1-design-verify-fixes` | no (new calls as questions) | both verify reviews | done (16:42, three rounds); sign-off list written |
| final design check (reviewer, no card) | no | `p1-design-verify-fixes` | done: round 1 (16:32) 0/1/8; round 2 (16:39) major and all 8 minors fixed, 2 new minors, "ready for Haik after one clause" (`docs/reviews/p1-design-final-check.md`); both minors sent to round 3 |
| `p1-repo-files` | no | `p1-design-review-fixes` (not the sign-off: its four files do not depend on the design) | done (15:58) |
| `p1-cargo-workspace` | no | design sign-off (Haik signed ahead 15:45, on condition the verify finds no new choices of his) | done (16:13) |
| `p1-contracts` | no | `p1-cargo-workspace` | done (16:37); 13 design corrections routed to `p1-design-verify-fixes` (round 3), correction 12 to the power track planner |
| Phase 1 reviews (two reviewers, no card) | no | `p1-contracts`, `p1-ci` | done (16:55): general 0/0/8 (`docs/reviews/p1-skeleton-general.md`), contracts domain 1/5/8 (`docs/reviews/p1-contracts-domain.md`) |
| `p1-contracts-split` | no | the five track plan cards, both Phase 1 reviews | done (17:57); its 10 card edits sent to `p1-track-plan-fixes` |
| track plan cards `p2-plan-core` to `p6-plan-ship` | yes: Haik signs off all five at once | `p1-contracts`, design final check | done (17:22): 63 cards, about 6.15M tokens |
| cross-track review (reviewer, no card) | no | the five plans | done (17:43): 2/10/19 (`docs/reviews/p2-6-cross-track.md`) |
| `p1-track-plan-fixes` | no | cross-track review; `p1-contracts-split`'s card edits | done (18:07); `docs/phases/02-06-signoff.md` has 12 questions with defaults |
| fixes verify (reviewer, no card) | no | `p1-contracts-split`, `p1-track-plan-fixes`, first push | done (18:30): 0/2/6 (`docs/reviews/p1-fixes-verify.md`) |
| `p1-close-contracts` | no | fixes verify | done (18:50): `Event::Updated`, `Notice::Updated`, owner lines, `architecture.md`, answers in 16 cards, all 8 verify findings fixed; `Cargo.lock` one line (`proptest` in the CLI) |
| close verify (reviewer, no card) | no | `p1-close-contracts` | done (19:05): 0/1/5 (`docs/reviews/p1-close-verify.md`); major and minors routed or fixed (19:08) |
| `p6-plan-integration` | yes: signs off the plan, two questions | first push | done (18:39): 11 cards, about 1.19M tokens (`docs/phases/06-integration.md`) |
| `p6-int-skeleton` | no | `p1-close-contracts`; Haik's sign-off (18:58) | done (card Result) |
| `p1-ci` | no | `p1-cargo-workspace` | done (16:25); CI green after the first push (18:15) |
| Phase 1 general review (reviewer, no card) | no | `p1-repo-files`, `p1-ci` | of the repo files, workspace and CI, before the first push |
| fixes for that review (card written from its findings) | no | Phase 1 general review | not written yet |
| `p1-privacy-check`, `p1-privacy-fixes` | | | dropped (Haik's answer 1, 15:29): the secret scan in the steps below replaces them |
| `p1-lid-sound` | yes: about 3 minutes at the end (two lid closes) | Phase 0 sign-off | done (15:31) |
| `p1-tray-menu-look` | yes: hands off the laptop while the worker uses the screen, then one look at pictures | `p1-tray-menu-research`, `p1-tray-review-fixes`; screen part after `p1-lid-sound` | done (17:32): Haik likes it; asks for an icon per menu item (Phase 5, `p5-menu-paint`) |

Screen and sound cards (`p1-lid-sound`, `p1-tray-menu-look`) never run at the same time, or next to another card
that uses the screen, sound or power settings.

Most parallel safe order (Haik, 2026-11-03: parallel whenever safe, full effort): `p1-tray-menu-look` runs beside
every card below (none of them uses the screen or writes its files). `p1-repo-files` starts right after
`p1-design-review-fixes`; `p1-cargo-workspace` starts at the sign-off, beside it; `p1-ci` follows the workspace; the
general review waits for both lines; its fix card follows; then Haik's yes (Gate B), the freeze and the first push
(steps below).

Size estimates, one worker's context: `p1-repo-files` about 60K tokens, `p1-cargo-workspace` about 120K (two targets,
`dumpbin`, a first toolchain download), `p1-ci` about 80K. `p1-design-review-fixes`: its Result gives what it used.
`p1-tray-menu-look` is sized in its own card (running; not changed here).

## Orchestrator steps (git and GitHub)

Written by card `p1-detailed-design`; reworked by `p1-design-review-fixes` for Haik's answer 1 (fully public, secrets
never) and by `p1-design-verify-fixes` (Gate B back, a scan before every commit). Workers and reviewers never run git.
The secret scan reads exactly what gets committed, nothing is committed before it, and no file changes between the
scan and the push. Haik's "go fully public" (15:29) decided what goes public; Gate B asks him when, because a public
push cannot be undone. No worktree isolation (Question 9).

1. **Gate A.** Haik signs off the design after reading the sign-off list (top of "Design summary for Haik"). His
   ahead sign-off (15:45) allows only the local build cards: no repo, no push. His answers to Questions 1 to 8 came
   2026-11-03 15:29.
2. Run the cards in the order above: `p1-repo-files` and `p1-cargo-workspace`, then `p1-ci`, then the Phase 1
   general review, then its fix card.
3. **Gate B**, once step 2 has ended: Haik's yes to go public now. The orchestrator tells him in a few lines what goes
   public (the whole tree, scanned for secrets in step 5) and that a public push cannot be undone. Nothing below runs
   without his yes; until then other cards may run. His yes covers steps 4 to 9 while the scan finds nothing he must
   judge; a hit he is asked about (step 6) needs his yes again before the push.
   Then **freeze**: from here until the push (step 9) no card that writes to the repo runs. A card still running
   (`p1-tray-menu-look`) finishes or pauses first, and a paused card stays paused until the push. The freeze covers
   the orchestrator too: it writes notes only to its scratchpad and copies them into the work log after the push, and
   no process that writes into the repo runs (for example a spike logging to its `out/` folder). From `git init`
   until the push the branch has no upstream, so skip the `git pull --rebase` at the start of each orchestrator run
   (`SKILL.md`, Git) until step 9 is done.
4. `git init -b main` in `C:\Users\haika\code\wakeshot`. `git config user.name` and `user.email` must give Haik's usual
   identity (`HaikAsatryan` and his Gmail address). Write the local hook `.git/hooks/commit-msg` (never committed; LF
   line endings), which rejects AI lines in every later commit too:

   ```sh
   #!/bin/sh
   # Haik's rule: commits carry only his name, no AI trailer or mention.
   pat='^[[:space:]]*(co-authored-by|claude-session):|generated with|anthropic\.com|claude\.ai/code'
   if grep -qiE "$pat" "$1"; then
     echo "commit-msg: AI trailer or mention found" >&2
     exit 1
   fi
   ```

   Tested with GNU grep on sample messages (2026-11-03): it rejects a co-author trailer, a `Claude-Session:` trailer
   with or without leading spaces, a bare `https://claude.ai/code/session_...` line and "Generated with"; it passes
   "Add Claude Code status file parser" and "Initial commit".

   Update the `CLAUDE.md` status line ("no git repo yet" is then wrong).
5. **Stage and scan.** `git add -A`. Save `git ls-files -s` (mode, blob hash, path) to a file in the orchestrator's
   scratchpad. `git status --ignored --short` must show only `target/` folders and the other `.gitignore` paths as
   ignored. Run the secret scan below on the index. Self-test: the scan must find the known fine hits listed below
   (`live-tests.log`, `watch.rs`, the tray manifest). Zero hits means the scan is broken (a wrong flag, a bad pattern
   file, a regex difference): stop and fix the command; never commit on a scan that found nothing.
6. A hit that is a real secret: stop; a worker removes it (a new card, which lifts the freeze for that card only);
   then step 5 again. A hit that only names a pattern (a doc, this file): record it as fine in the scratchpad notes
   (into the work log after the push). A hit the orchestrator cannot judge: ask Haik.
7. First commit: `git commit -m "Initial commit"`. Check with `git log -1 --format="%an <%ae>%n%B"`: Haik's identity,
   no trailer.
8. `gh repo create HaikAsatryan/wakeshot --public --source . --remote origin` (no `--push`). `git remote -v` must show
   `git@github.com:HaikAsatryan/wakeshot.git` (this machine uses SSH remotes); if it shows HTTPS, `git remote set-url
   origin git@github.com:HaikAsatryan/wakeshot.git`.
9. **Gate C**, right before the push: `git status --porcelain` is empty, and `git ls-files -s` is the same as the file
   saved in step 5 (compare the two files). Any difference: steps 5 and 6 again, then commit the change with a
   message that says what changed (no second "Initial commit", no amend), then Gate C again. Then `git push -u origin
   main`. The freeze ends.
10. CI: `gh run list --limit 1`, then `gh run watch <id> --exit-status`. Record the run URL and result in the work
    log. A red job becomes a fix card. Tick "Repo public, CI green" when every job is green.
11. From then on: commit after each finished card, push after a phase passes review (`plan.md`). Before each commit,
    run the step 5 scan with its self-test on the index (paths and `git grep --cached`), so a secret never enters a
    commit. Before each push, also scan the commits the push sends (below), and make no commit between that scan and
    the push. A hit in a commit not yet pushed: never push. A worker removes the secret, and the orchestrator asks
    Haik before it rewrites the unpushed commits (a history rewrite is his call). Scanning only the index again is not
    enough: the old commit still holds the secret, and the push would publish it.

Secret scan (the same patterns before every commit and push; the orchestrator keeps them in a file in its
scratchpad):

- Paths. First push: `git ls-files`; later: `git log --name-only --format= origin/main..HEAD`. No path may match
  `\.(key|pem|pfx|p12)$`, `(^|/)id_(rsa|ed25519)`, `(^|/)\.env`, `credentials`, or build output (`(^|/)target/`,
  `\.(exe|dll|pdb)$`).
- Content. First push: `git grep --cached -n --text -E -f <patterns file>` (searches the staged blobs, binary ones
  too: `-I` would skip spike logs that git sees as binary, such as
  `spikes/power-settings/out/hold-limited-20261103-021916.log`, found by `p1-repo-files`). Before each later commit:
  the same. Before each later push: the added lines of `git log -p origin/main..HEAD`. One pattern per line:

  ```text
  gh[pousr]_[A-Za-z0-9]{36}
  github_pat_[A-Za-z0-9_]{40,}
  sk-ant-[A-Za-z0-9_-]{20,}
  sk-(proj-)?[A-Za-z0-9_-]{32,}
  AKIA[0-9A-Z]{16}
  xox[abprs]-[A-Za-z0-9-]{10,}
  -----BEGIN [A-Z ]*PRIVATE KEY-----
  eyJ[A-Za-z0-9_-]{10,}\.eyJ[A-Za-z0-9_-]{10,}
  (AccountKey|SharedAccessSignature|[Pp]assword)=[^;[:space:]]{4,}
  "(accessToken|refreshToken|bridgeSessionId)"[[:space:]]*:[[:space:]]*"[^"]{8,}"
  ```

  Then once more with `-i` and this one pattern, for keys assigned in code or config:
  `(api[_-]?key|secret|token|passwd|password)["']?[[:space:]]*[:=][[:space:]]*["'][A-Za-z0-9_./+=-]{16,}["']`.
- What each catches: GitHub tokens; Anthropic keys and Claude Code sign-in tokens; OpenAI keys; AWS keys; Slack
  tokens; private keys; signed web tokens; connection strings; Claude Code account fields and the Remote Control
  session id (whether that id alone grants access: not verified, so a value found is removed).
- Known fine hits (dry run of these patterns with GNU `grep -rnIE` over the working tree, not `git grep`, 2026-11-03
  15:48): `spikes/claude-watch/fixtures/live-tests.log` (redacted values `<str 28>` and `session_BRIDGE01` to `03`),
  `spikes/claude-watch/src/watch.rs` (test value `session_x`), `spikes/tray/tray-spike.exe.manifest` (the public
  Common Controls `publicKeyToken`). No path hit. The step 5 self-test needs them; if they are ever removed (Phase 8),
  keep one file with a fake value the patterns hit.
- Images and other binary files are not scanned. A picture could still show a secret on screen; the pictures come
  from the tray spikes and the menu card. The menu card's crop step (deliverable 7) covers three old pictures only.
  Its new pictures (deliverable 1: the Wi-Fi quick settings flyout, Chrome, Telegram) can show network and account
  names: not secrets under answer 1, but personal, so the orchestrator adds a crop line to that deliverable before
  the pictures are taken or kept.

## Close

- [x] Haik signs off the design (2026-11-03 16:44, "signed off" on the sign-off list; Questions 10 to 12 at the
  defaults, as the list asks).
- [x] All cards done; reviews (general and domain) done and fixed (2026-11-03 19:08; `p6-int-skeleton` is an
  integration card and runs after wave 1).
- [x] Repo public, CI green on x64 and ARM64 (2026-11-03 18:15, run 41651223828).
- [x] Haik signs off Phase 1 (2026-11-03 19:13, "ok signing off").

## Work log

- 2026-11-03 14:35: Phase file created after Haik's Phase 0 sign-off. Repo owner `HaikAsatryan` (Haik). Haik's
  parallel session still runs two read-only agents (tray review, tray menu research); he tells the orchestrator
  when they finish. Started in parallel: `p1-detailed-design` (docs only) and `p1-lid-sound` (sound, lid spike,
  needs Haik about 3 minutes at the end).
- 2026-11-03 14:59: Haik: his parallel session finished. Tray review: 0 blocker, 2 major (the orange toast icon is
  not the spike's PNG: R and B swapped after it, cause unknown; leftovers the end check missed: a balloon key, 4
  stored spike toasts until 2026-11-06, two `NotifyIconSettings` entries, a `guidmove` exe copy), 12 minor, with
  exact edits. Menu research pick: owner-drawn items in the classic menu plus a DWM round-corner call; menu follows
  the Windows mode (decided under Haik's delegation of design calls); leftovers removed at the end of
  `p1-tray-menu-look`. Card `p1-tray-review-fixes` started (docs only). Design worker told the research is final.
- 2026-11-03 15:03: Card `p1-lid-sound` blocked on Haik's two lid closes (steps in the card). So far: System Sounds
  100% and not muted; `SND_SYSTEM` plays on the default device (explains D, not I); sounds start 0.3 to 0.8 s after
  the call, so in I the lock came first. Proposed: own chime through WASAPI on the default device, or the speakers
  when the default is a screen's audio; the lock waits for it. Loopback saw it with the lid open. Haik's volume,
  device, scheme and power values unchanged at 15:02. Orchestrator check 15:03: Balanced active, no power requests,
  no spike process, on battery 91%.
- 2026-11-03 15:07: Card `p1-tray-review-fixes` done: 19 review edits (14 as written, 5 changed with reasons in its
  Log), both optional edits, research link, menu theme line in `product.md`, `p1-tray-menu-look` updated. Orchestrator:
  copied the tray worker's old scratchpad (191 files, 8.3 MB, no `.key` files) to
  `~/.claude/wakeshot-keep/2026-11-03-tray-scratchpad/` so a temp cleanup cannot lose review finding 13's evidence;
  added the crop step to `p1-tray-menu-look` (deliverable 7); fixed the stale `research.md:485` pointer in
  `product.md`; sent the design worker the stale `architecture.md` pointers and the review points with no numbered
  edit (log wall time, balloon events, outside click, icon reload, toast `DisplayName`).
- 2026-11-03 15:11: Verify review of the tray fixes (`docs/reviews/p0-tray-verify.md`): fixed; 0 blocker, 0 major,
  5 minor doc cleanups. They go to `p1-tray-menu-look` (read first), which edits the same tray docs; minor 1 matters
  for it (the old dark menu recipe follows the app mode, product.md now says Windows mode).
- 2026-11-03 15:16: Card `p1-detailed-design` done (`architecture.md` 643 lines, 8 questions for Haik, cards
  `p1-repo-files`, `p1-cargo-workspace`, `p1-ci`, `p1-privacy-check`, `p1-privacy-fixes`, 11 git steps with three
  gates). It used about 434K tokens, far over the 150K card size. Two design reviews started before Haik's sign-off
  (power and facts; general). Questions relayed to Haik. Its outside-scope notes wait for the review fix worker:
  `product.md` battery value and hibernate minutes (after Haik's answers), signals rule 11 marked dropped, five tray
  facts from `docs/reviews/p0-tray.md` into `research.md`, question pointers in `architecture.md` replaced by answers.
- 2026-11-03 15:29: Haik's lid closes for `p1-lid-sound`: both work. Close 1 (laptop alone): Windows sound, then our
  chime. Close 2 (DELL in): our chime, then the Windows sound. DELL out now; Haik cannot reach it for about 15
  minutes. Test sounds 14:56 to 15:00: heard at least once, details not remembered. Worker resumed to record them.
- 2026-11-03 15:29: General design review: 1 blocker (files can change after the privacy check and still go public),
  4 major (worktree isolation vs the git flow; Question 1 kept the Claude fixture log local; privacy card about 2.5x
  one worker; commit email never asked), 13 minor. Haik's answers: Question 1 "fully public" (his words in
  `p1-design-review-fixes`), with a separate cleanup phase before launch (now Phase 8 in `plan.md`); Questions 2 to 8
  defaults. Commit identity: his usual one; the Gmail address is already on 6,683 of his public commits (`gh`
  search). New standing rule in `plan.md`: parallel agents whenever safe, full effort. Started in parallel:
  `p1-design-review-fixes` (docs) and `p1-tray-menu-look` (spike code first; blocks before any screen step).
- 2026-11-03 15:31: Card `p1-lid-sound` done, all three checks pass. Close 1 repeated I and was heard, so neither
  the lock nor the closed lid cuts sound; I's silence stays unexplained (most likely not noticed, a guess). Method:
  own chime through WASAPI on the default device, or the speakers when the default is display audio; the lock waits
  until the stream runs (at most 1 s). End state 15:30 matches the 14:40 BEFORE line (DELL out). Sent to the design
  fix worker for the sound seam. Its list of other sound moments (Phase 5) relayed to Haik for his word.
- 2026-11-03 15:34: Power design review: 0 blocker, 6 major, 13 minor. Majors: a plan switch or an in-flight read
  can drop journal entries while our values stay; read and apply errors count as the user's or as done and delete
  the journal; the panic hook restore races the IO thread; the guard is the tray's child again (Phase 0 said
  detached); a hold file with no `procStart` can hold forever on a reused pid; `Apply` can undo the user's plan
  switch. Minor 18: the lock does not cut sound (close 1), so no wait before the lock; the orchestrator told the
  fix worker to lock at once and play the sound in parallel. Review sent to `p1-design-review-fixes`.
- 2026-11-03 15:36: Haik: sound moments "default" (sent to the fix worker for `product.md`). "Full effort" means
  xhigh, never max (in `plan.md`). Do not touch his global rules yet. No agent of this session ran at max.
- 2026-11-03 15:38: Haik agrees to the sign-off flow: after `p1-design-review-fixes` and its verify review, the
  orchestrator sends him a short list (findings, what changed, any new choices of his); he signs off from that list.
- 2026-11-03 15:43: Haik leaves the laptop for an hour or more on battery (87%, Windows estimate 379 min) and asked
  the orchestrator to keep working. Orchestrator started `power-settings-spike limited hold --sleep-dc 0 --minutes
  150 --allow-ac` at Medium (PID 15652, copy d4adc592 "Wakeshot spike battery test" active, SYSTEM and EXECUTION
  held; output in the orchestrator scratchpad `away-hold.out.txt`). It restores Balanced by itself at about 18:13.
  The lid must stay open (lid action Sleep stays on the copy). Tray worker told; no screen step until Haik decides.
- 2026-11-03 15:45: Haik: (1) the laptop is in a private place, so keep the screen on and unlocked while he is away;
  (2) he signs off the design ahead: if the verify review of `p1-design-review-fixes` finds no new choices that are
  his, the build cards start (local only: no repo, no push), and he reads the sign-off list when back. Orchestrator
  holds DISPLAY (pwsh PID 13064, scratchpad `away-screen-holder.ps1`, until about 18:13, stops under 25% battery).
  Tray worker told it may do its screen part without waiting (open menus and take pictures only, never click items).
- 2026-11-03 15:52: Card `p1-design-review-fixes` done: general 11 fixed, 6 another way, 1 partly; power 16 fixed, 2
  another way, 1 became Question 10; overlaps fixed once; privacy cards dropped (secret scan in the git steps); new
  Questions 9 to 12 for Haik. Two verify reviewers started in parallel; the general one also says whether any open
  question changes the build cards (Haik's condition for his early sign-off). `p1-repo-files` started (it needs no
  sign-off). Haik asked why Phases 2 to 6 cannot run in parallel: they can, once the contracts exist as code.
  `plan.md` now runs Phases 2 to 6 as parallel tracks with one plan step and one sign-off; new card `p1-contracts`
  after the workspace, in parallel with `p1-ci`.
- 2026-11-03 15:55: Haik left the laptop. His words: "as a principal architect and project manager you will handle
  parallelism ... In the end we should have 0 conflicts and quality work with no lost work due to conflicts." So the
  orchestrator decided Question 9 (no worktrees; file ownership rules in `plan.md` "Git"). Questions 10 to 12 are
  product calls and stay open for him.
- 2026-11-03 15:58: Card `p1-repo-files` done: `LICENSE` (SPDX MIT, "Copyright (c) 2026 Haik Asatryan"), 17-line
  `README.md`, `.gitignore` (checked without git: only the four `target` folders ignored, 292 files stay),
  `.gitattributes` (six spike logs with lone CR or NUL kept as binary). Its notes: the secret scan's `git grep -I`
  would skip a spike log git sees as binary, so the orchestrator changed the scan step to `--text`; `target/` would
  also ignore a source folder named `target` (none exists); the four spike `.gitignore` files are now redundant
  (left for Phase 8 cleanup).
- 2026-11-03 16:01: General verify (`docs/reviews/p1-design-general-verify.md`): 0 blocker, 1 major, 9 minor; all
  18 general findings closed. Point 4: Questions 9 to 12 change nothing in the three build cards. Major: the fixes
  dropped Haik's last yes before the first public push (only in the steps' intro; Gate A reads as passed); it goes
  into his sign-off list, and `git init` and the push wait for him. Minors (freeze vs the orchestrator's own log
  writes and running spike logs; no way out for a secret in a local commit; no self-test of the scan) wait for one
  docs fix worker together with the power verify's findings. `p1-cargo-workspace` started (local only), under
  Haik's early sign-off: the condition holds for the build cards.
- 2026-11-03 16:06: Power verify (`docs/reviews/p1-design-power-verify.md`): 0 blocker, 2 major, 12 minor; 15 of 19
  findings fixed as stated. Majors: a failed plan-list or active-plan read looks like "plan gone" and deletes the
  journal, RunOnce and guard (original finding 2 again); Question 10 leaves out fast user switching. Lock and sound
  now right (lock at once, chime in parallel). Card `p1-design-verify-fixes` started (docs only; owns
  `architecture.md`, `product.md` and the phase file's design sections; writes Haik's sign-off list). Battery 80%
  at 16:06 (87% at 15:42), so about 44% when the away hold ends at 18:13.
- 2026-11-03 16:14: Card `p1-cargo-workspace` done: root `Cargo.toml` (spikes excluded), `Cargo.lock`,
  `rust-toolchain.toml` 1.99.1, `.cargo/config.toml` (static CRT), four crates; fmt and clippy clean on both targets,
  5 tests, x64 and ARM64 release builds, no VC runtime imports, tray manifest has Common Controls 6 and
  PerMonitorV2; a tray spike build with the new root config works (its own target folder). Found: any `RUSTFLAGS`
  drops the static CRT (now in `CLAUDE.md`; `p1-ci` told). Two `architecture.md` notes sent to the verify fix
  worker. rust-analyzer already in the 1.99.1 toolchain (checked; `~/.cargo/bin` is not on this session's PATH).
  Started in parallel: `p1-contracts` and `p1-ci`, each owning its own files.
- 2026-11-03 16:21: Haik plugged in the charger and is away again; he asked for his power settings back. Orchestrator
  started a SYSTEM request first (pwsh PID 4740, scratchpad `away-system-holder.ps1`, 4 hours, until about 20:20;
  stops on battery under 25%), waited for AC (16:20:42, still on after 20 s), stopped the spike hold by PID 15652
  and ran `power-settings-spike limited restore` (Balanced active, copy d4adc592 deleted, journal removed). Checked
  16:21: only Balanced, lid 1/1 (`/qh`), sleep 10800/600, no journal. DISPLAY holder PID 13064 stays until about
  18:13 for the tray card. Battery 75% at 16:20.
- 2026-11-03 16:22: Card `p1-design-verify-fixes` done: power verify 14 of 14 (minor 8 another way: the journal is
  written last in Apply), general verify 6 fixed, 1 another way, 2 in part, 1 not its file; Gate B (Haik's yes right
  before `git init`, through the first push) back as step 3; sign-off list (25 lines) at the top of the summary; no
  new questions. Relayed its boundary type changes to `p1-contracts`. Orchestrator cleanups from its notes: skill
  lines on worktrees and `git pull --rebase` fixed, superseded note in `p1-lid-sound`, secret file lines added to
  `.gitignore` (no repo file matches them, checked). Final narrow check of the three verify majors and the list
  started.
- 2026-11-03 16:22: Card `p1-tray-menu-look` blocked only on Haik's one look. Pick stays: owner-drawn items plus the
  DWM round-corner call, now 8 epx corners, 14 epx Segoe UI Variable, 33 epx rows (Explorer 32); without the DWM call
  the menu is square. Windows' own tray menus follow the Windows mode (decision 3 holds). 36 crops
  `docs/research/img/tray-look-*`, private parts greyed (Wi-Fi name, Chrome extension name; checked against the
  orchestrator's crop rule), raw shots in its scratchpad. Spike leftovers deleted and still gone a minute later. Not
  tested: Explorer restart (simulated), real high contrast, 100% scale. Worker error: it overwrote the Phase 0
  picture `tray-icon-add1-dark-taskbar.png`; no copy of the original exists (searched the keep folder and every
  Claude temp folder). Orchestrator stopped its own DISPLAY holder (PID 13064) at 16:22; SYSTEM holder PID 4740
  stays.
- 2026-11-03 16:25: Card `p1-ci` done: `.github/workflows/ci.yml` (fmt on x64; clippy, tests and release builds on
  x64 and ARM64 with `--target`; exes uploaded 7 days; a Linux job tests the core alone; a `dumpbin` step fails on
  VC runtime imports, shown failing with `RUSTFLAGS` set). `actionlint` 0 errors; all commands pass locally on x64,
  clippy and build on ARM64; Linux not run here. Not verified until the first run: `dumpbin` on the runners and the
  `windows-11-arm` image. Its `architecture.md` section 10 wording waits for one docs pass with `p1-contracts`'
  corrections. Found: all agents of a session share one scratchpad, so its first build landed in another card's
  `scratchpad\target` (nothing broke); `plan.md` now names build folders after the card.
- 2026-11-03 16:27: Orchestrator wrote the five track plan cards (`p2-plan-core`, `p3-plan-windows-power`,
  `p4-plan-sources`, `p5-plan-tray-ux`, `p6-plan-ship`), each owning only its own phase file and card prefix. They
  start in parallel once `p1-contracts` and the final design check are done (stable module layout and design). After
  them: one cross-track check (file ownership collisions, laptop cards marked) and one contracts change card run
  alone, then Haik signs off all five plans at once. Track builds start after the first push, so each card can be
  committed by path.
- 2026-11-03 16:32: Final design check: not ready, 0 blocker, 1 major, 8 minor; Gate B passes. Major: an `Apply`
  that takes over an entry after a failed restore clears its `restored` mark before it writes, so a stop or failed
  write between them drops the entry and deletes the journal, RunOnce and guard while Do nothing stays. Minors
  include the short forms of Questions 10 (Remote Desktop, the accepted risk) and 12 (side effects, the other
  option). `p1-design-verify-fixes` reopened (same worker) with these and `p1-ci`'s section 10 wording. This is the
  third ordering bug found on paper in the restore paths, so the Core and Windows track plan cards now require a
  stop-point test (every step of Apply and restore) and a property test over journal entry states.
- 2026-11-03 16:36: `p1-design-verify-fixes` round 2 done: all 9 final-check findings fixed (taken-over entries keep
  `restored` until `PowerSetActiveScheme` succeeds; any failed write or call reruns the restore routine), `p1-ci`'s
  section 10 wording in, sign-off list 26 lines, Question 11 also covers a damaged journal, no new questions.
  `ApplyError::Write` now means the restore routine ran (relayed to `p1-contracts`). Final-check reviewer resumed to
  re-check round 2.
- 2026-11-03 16:38: Card `p1-contracts` done: boundary types of sections 2, 3, 6, 7 compile with no logic; `Clock`
  and a stub `Core`; `PowerApi`, `MenuView`, `Notifier`, `SoundPlayer`; one fake per boundary behind a
  `test-support` feature; all dependencies declared up front except `velopack` (113 packages; a solo Phase 6 card);
  fmt and clippy clean on both targets, 25 tests, core has no `windows` crate in its normal tree. Routed: 13
  `architecture.md` corrections (all but 12) to `p1-design-verify-fixes` round 3; correction 12 (journal entries
  with an unknown key) and the `state.json` before `restore-request.json` order to the Windows power planner; the
  shared `store`, `log`, `process` modules given to Phase 3 (built first, fakes for 4 and 5). `CLAUDE.md` names the
  contracts. Started in parallel (no shared files): five track planners, two Phase 1 reviews (general; contracts
  domain), the design correction round. The final-check re-check is still running.
- 2026-11-03 16:39: Final check round 2: the major is fixed (reviewer walked every stop point on the current text),
  all 8 minors fixed, 2 new minors: the sign-off list's Question 11 line must add "until then a lid close gets no
  lock"; after a failed apply the retry waits for an apply that never comes (late restore, nothing dropped). Both
  sent to `p1-design-verify-fixes` round 3.
- 2026-11-03 16:42: `p1-design-verify-fixes` round 3 done: `p1-contracts` corrections 1 to 11 and 13 in
  `architecture.md` (checked against `crates/`), no restore step text changed; Question 11 line has "until then a
  lid close gets no lock"; after a failed apply the restore retry covers every entry. Open, sent to the power track
  planner: a late restore (nothing dropped) when a key released on another plan has a failed-restore entry on plan
  P and the hold switches back to P; fix by a `Restore` with keys (contract) or by `Apply` restoring entries outside
  its writes. The design is ready for Haik's sign-off list.
- 2026-11-03 16:44: Haik: "signed off" (Gate A). The list asked for "signed off" plus an answer to any question he
  wants changed, so Questions 10 to 12 take the defaults. Gate B (his yes to go public, right before `git init`)
  comes after the Phase 1 general review's fix card and after the five track planners finish, because the freeze
  from Gate B to the push stops every card that writes to the repo. His tray picture look is still open; if it comes
  during the freeze, the orchestrator records it after the push.
- 2026-11-03 16:52: Contracts domain review (`docs/reviews/p1-contracts-domain.md`): 1 blocker, 5 major, 8 minor;
  25 tests pass, core has no `windows` crate on any target. Blocker: Phases 2 to 5 would each edit shared contract
  files (`holds.rs`, `settings.rs`, `view.rs`, the `wakeshot-windows` module list, the tray `main.rs`). Majors
  include: no type for threads to post events to the loop; the stop-point test cannot be written yet (no apply and
  restore signatures, no journal, RunOnce and guard seams; `FakePower` cannot tell stored from in-force values or
  stop mid-routine). Plan: one contracts change card, run alone after the planners, that declares every module,
  splits shared files by owner and adds the seams, merging this review, the planners' contract changes and the
  general review's code findings. All five planners told to plan on the split files.
- 2026-11-03 16:55: Phase 1 general review (`docs/reviews/p1-skeleton-general.md`): 0 blocker, 0 major, 8 minor;
  fmt, clippy, 25 tests and release builds pass on x64 and ARM64, no VC runtime. Top minors: `--locked` missing in
  the `CLAUDE.md` check commands; clippy with `--all-targets` enables `test-support`, so the shipped libraries are
  never linted alone; the Linux job does not prove the core has no `windows` dependency (`cargo tree` step). Card
  `p1-contracts-split` written: runs alone after the five planners; merges both Phase 1 reviews and the planners'
  contract changes.
- 2026-11-03 17:00: `p6-plan-ship` done (`docs/phases/06-ship.md`): 10 cards, about 880K tokens. Real laptop:
  `p6-velopack-facts`, `p6-install-e2e`. Needs Haik: `p6-readme`, `p6-pages`, `p6-velopack-dependency` (its Q4
  first; runs alone), `p6-pack-and-release` (its Q5). 5 contract changes (update variants in `Event`, `Command`,
  `Outcome`; module files `install.rs`, `hooks.rs`, `update.rs`, `velo.rs`; a `velo::startup()` call in the tray
  `main.rs`), one only if Haik says yes to its Q3. Cross-track: needs `p3-store`, `p3-log`, `p3-process`,
  `p3-restore`, `p3-power-plan`, p4 docs, the tray icon. Risk: `ring` needs clang for ARM64, none on this laptop
  (Q4). Its questions go into the one track sign-off.
- 2026-11-03 17:02: `p4-plan-sources` done (`docs/phases/04-sources.md`): 19 cards, about 1.7M tokens (50K to 140K
  each), no new crate (own cron and RFC 3339 code). Real laptop or Haik: `p4-claude-live-shapes`, `p4-live-claude`
  (Claude test sessions, no settings; optional 3-minute VS Code check), `p4-live-codex` and `p4-live-copilot`
  (Haik about 20 minutes each, may install the CLI). 12 contract changes (split `holds/file.rs`, source and watch
  modules, `Names`, `FindTail`, `Fed::RetryOnce`, CLI `main.rs` and `command.rs` frozen with one file per command).
  Cross-track: needs `p3-store`, `p3-process` (plus `start_of_pid` and `list`), `p3-log`; gives `format_expires` to
  the core and `shell.rs` to the tray. 2 questions for Haik (Codex and Copilot waiting).
- 2026-11-03 17:05: `p2-plan-core` done (`docs/phases/02-core.md`): 10 cards in five waves, about 1.21M tokens; no
  real laptop, no Haik hands. 10 contract changes (module files, seams `PlanIo` and `SafetyNet`, routine
  signatures, `first_run`, `Log`, `FinishRestoreRequest`, `Leftover`, `JournalCorrupt`, `IdleLocked`, doc fixes).
  Cross-track: `p2-core-holds` after `p4-hold-files`; wording from the tray track's `text.rs`. Conflict found: the
  power planner also builds apply, restore, check and their property test in `wakeshot-windows`. Orchestrator
  decision (Haik delegated architecture and parallelism): the routines and their property test live in the core,
  generic over `PlanIo` and `SafetyNet`; the Windows track implements both seams, runs the stop-point test on the
  real implementations and owns the real-laptop checks. Sent to the power planner while it still runs.
- 2026-11-03 17:08: `p5-plan-tray-ux` done (`docs/phases/05-tray-ux.md`): 9 cards, about 930K tokens. Screen:
  `p5-tray-icon`, `p5-menu-view`, `p5-notices`; sound: `p5-sounds`; both: `p5-haik-check` (Haik about 25 minutes);
  each does code first, then blocks for its slot. Both menu cards wait on Haik's menu-look answer; `p5-sounds` asks
  whether chimes honor the "No sounds" scheme (default yes). 8 contract changes (`text.rs` split from `view.rs`,
  `tray.rs` child modules and an icon identity type, a waitable sound play for the hibernate thread, the app icon in
  both `build.rs`). Cross-track: `text.rs` signatures merge with Phase 2's list; uses Phase 4's `shell.rs`; proposes
  that the core holds notices while the session is locked.
- 2026-11-03 17:22: `p3-plan-windows-power` done (`docs/phases/03-windows-power.md`): 15 cards, about 1.43M tokens;
  first wave runs seven cards in parallel, starting with the shared `p3-store`, `p3-log`, `p3-process`. Real laptop,
  one at a time: `p3-power-plan` (copy, never active), `p3-stop-points`, `p3-real-checks` (50 cycles on an active
  copy). `p3-haik-checks`: about 60 minutes elapsed, 15 of Haik's hands. 10 contract changes; three differ from the
  core's text (`PlanIo::values()`, locked entry points holding the journal lock, `Event::ActivePlan`). Correction
  12: an unknown-key entry stays untouched; the journal counts as empty without known entries (RunOnce and guard go,
  the file stays for the newer version). Its `p3-restore` was dropped after the 17:05 decision, but the sources and
  ship plans still name it. All five plans: 63 cards, about 6.15M tokens. Started in parallel: `p1-contracts-split`
  (alone on `crates/`, `Cargo.*`, `.github/`, `CLAUDE.md`, `architecture.md`) and a cross-track review of the five
  plans (`docs/reviews/p2-6-cross-track.md`, read only).
- 2026-11-03 17:32: Haik's look at the tray pictures: "I like your new ui, you might need some icons for each row as
  examples are using. Everything else is good." `p1-tray-menu-look` done. `product.md` UX line and `p5-menu-paint`
  updated: the pick stands and every menu item gets a small icon (Phase 5).
- 2026-11-03 17:43: Cross-track review (`docs/reviews/p2-6-cross-track.md`): 2 blockers, 10 majors, 19 minors; no
  cycles, no file with two owners at once. B1: p2, p4 and p6 cards require whole-crate or workspace checks while
  others run (stalls, formatting other cards' files); `plan.md` now says each card checks only its own package and
  files, the workspace check runs when a wave closes. B2: `p3-haik-checks` signs out while agents may run; `plan.md`
  now says sign-out, Explorer restart or reboot cards run alone. M1: stale `p3-restore` links in `p4-cli-restore`,
  `p6-hooks` and the 04 and 06 phase files. Totals: 63 cards, 6.17M tokens; Haik about 60 (p3), 40 (p4), 25 (p5),
  about 10 (p6, not verified) minutes; 18 laptop sessions (3 were unmarked); critical path about 730K over 8 cards
  after the README fix. Integration has no plan card yet. Card `p1-track-plan-fixes` started (owns the 02 to 06
  phase files and all p2 to p6 cards; writes `docs/phases/02-06-signoff.md` for Haik).
- 2026-11-03 17:45: Haik suspected the laptop slept. Checked: Kernel-Power 506/507 show five display-off Modern
  Standby windows since 16:27 (idle timeout), the longest 16:51 to 17:12, but the orchestrator's SYSTEM request
  (PID 4740) stayed listed, and agents reported and the orchestrator ran commands inside that window, so work did
  not pause. Power source changes at 16:43 and 17:30 (most likely the DELL cable); on AC, 93%.
- 2026-11-03 17:57: Card `p1-contracts-split` done: every module the plans name exists and is declared, each file
  names its one owner in its first doc line, no file claimed by two cards; fmt, clippy with and without
  `--all-targets` on x64 and ARM64, 36 tests, both release builds, icon in all four exes, `Cargo.lock` unchanged.
  All planners' contract changes applied except ship item 4 (waits on Haik's ship Question 3) and `velopack`. The
  Windows "differs" items merged (`PlanIo::values()`, locked entry points as stubs in `power/journal.rs`,
  `Event::ActivePlan(Guid)`; `PowerApi` and `FakePower` removed, `FakePlans` is the one plan fake). Both Phase 1
  reviews' findings fixed; `CLAUDE.md`, CI and `architecture.md` sections 2 and 3 match the code. Its 10 card edits
  and owner table sent to `p1-track-plan-fixes` (still running). Orchestrator session stops here for a context
  clear; its only agent still running is `p1-track-plan-fixes`.
- 2026-11-03 18:02: Haik: "I signe off and go public is yes". (1) Gate B: his yes to go public is given; it covers
  git steps 4 to 9 while the scan finds nothing he must judge. The freeze starts once `p1-track-plan-fixes` ends
  (no card may write to the repo from `git init` to the push); the cross-track check of its fixes runs after the
  push. (2) He signs off the five track plans ahead, before `docs/phases/02-06-signoff.md` exists. The planners'
  product questions in that file still go to him with their defaults (never take a yes he has not seen); cards that
  do not depend on them may start after the push. The orchestrator waits for `p1-track-plan-fixes`, then stops for
  a clear; the next session runs the git steps.
- 2026-11-03 18:07: Card `p1-track-plan-fixes` done: B1 (each phase file has a "Checks while other cards run" note;
  cards check only their own files and package, `--locked`, never `--workspace`), B2 (`p3-haik-checks` runs with no
  other agent or cargo command), all 10 `p1-contracts-split` card edits in; every "Runs after" names a real card.
  Not done: M10 (integration plan, out of its scope), m9 (waits on ship Question 3). `docs/phases/02-06-signoff.md`:
  42 lines, 12 questions with defaults, critical path 730K over 8 cards; relayed to Haik. Orchestrator added the
  build cap (at most four building cards at once) to `plan.md`. Open, for the next contracts change: owner doc lines
  for `text.rs`, `art.rs` (hand-over to `p5-haik-check`) and the files the `p4-live-*` cards fix; stale
  `architecture.md` lines (612 `restore_from_journal`, 943 the test plan name, no `store::AutoStart` line). Not
  verified: whether a GitHub Windows runner lets `p3-session` create a window in `cargo test` (first CI run shows).
  No agent of this session runs now. Orchestrator stops for a clear.
- 2026-11-03 18:09: New orchestrator session. Haik: "Answer to all questions are default!": all 12 questions in
  `docs/phases/02-06-signoff.md` take their defaults (recorded there). Git steps 4 to 10 start now; no agent runs.
- 2026-11-03 18:11: Git steps 4 to 9 done. `git init -b main`; identity `HaikAsatryan <haik.asatryan.95@gmail.com>`.
  The `commit-msg` hook was not written: the Claude Code auto mode classifier denied writing into `.git/hooks`
  ("unauthorized persistence"); the orchestrator checks each commit message with `git log -1` instead. Step 5: 489
  files staged; ignored paths only the four `spikes/*/target/`. Scan (scratchpad `secret-scan.sh`,
  `secret-patterns.txt`): no path hit; content hits all known fine: `spikes/claude-watch/fixtures/live-tests.log`
  (redacted `<str 28>` and `session_BRIDGE01` to `03`; two long lines read in full: placeholder ids and paths only),
  `spikes/claude-watch/src/watch.rs:523` (`session_x`), the public Common Controls `publicKeyToken` in
  `spikes/tray/tray-spike.exe.manifest` and, new, `crates/wakeshot-tray/wakeshot-tray.exe.manifest`. Self-test
  passed. Commit "Initial commit", author and committer Haik, no trailer. `gh repo create HaikAsatryan/wakeshot
  --public`; remote `git@github.com:HaikAsatryan/wakeshot.git`. Gate C: tree clean, index equal to the step 5 copy.
  Pushed `main`. CI run 41651223828 started.
- 2026-11-03 18:13: Reviewer started on the fixes verify (contracts split against `p1-contracts-domain.md` and
  `p1-skeleton-general.md`, plan fixes against `p2-6-cross-track.md`). Card `p1-close-contracts` written (Question 7
  yes needs `Event::Updated` and `Notice::Updated`; it also takes the open items of `p1-track-plan-fixes` and the
  verify findings). `p4-claude-live-shapes` started early (agent-session lane, no build, own files only).
- 2026-11-03 18:15: Card `p6-plan-integration` written and started (planning only, no build; owns
  `docs/phases/06-integration.md` and new `p6-int-*` cards). Its plan goes to Haik for sign-off when done.
- 2026-11-03 18:15: Git step 10: CI run https://github.com/HaikAsatryan/wakeshot/actions/runs/41651223828 green,
  all three jobs (`core-linux`; `windows` on `windows-latest` x64 and `windows-11-arm` ARM64). Still not shown: whether
  a GitHub runner lets `p3-session` create a window in `cargo test` (that code does not exist yet; its first CI run
  after the card shows it).
- 2026-11-03 18:30: Fixes verify done: 0 blocker, 2 major, 6 minor. M1: `p4-cli-args` needs `proptest` in the CLI
  (a `Cargo.lock` change) and `p4-cli-run` needs the `Win32_System_Environment` feature; M2: the two property cards
  of the core plant mutants in files they do not own while other cards build (fix: a scratch copy). Top minor:
  `p2-core-lid` 8a still waits on an answer Haik gave. Known open, not counted: M10 (integration plan running), m9
  (the `Updated` variants). The reviewer ran `git --version` once (no repo operation). `p1-close-contracts` widened
  to allow the M1 `Cargo` change and started 18:31; `p4-claude-live-shapes` and `p6-plan-integration` (no builds)
  run beside it.
- 2026-11-03 18:39: Card `p6-plan-integration` done: `docs/phases/06-integration.md`, 11 cards (`p6-int-*`), about
  1.19M tokens; Haik about 27 minutes (18 of hands) plus about 15 optional with a dock, one whole-laptop session on a
  throwaway active plan copy. Critical path 11 cards, about 1.35M tokens, through the core chain (`p2-policy-holds`
  to `p2-core-lid`), then dispatch, tray mode, update wiring, the three e2e cards and Haik's checks. One contracts
  change: `p6-int-skeleton` (runs alone; `pub mod app;`, the seams, empty `app/*` files, the tray-mode call; no Cargo
  change). Orchestrator added to that card the five `architecture.md` proposals and three stale lines from the plan's
  Result, and runs it right after `p1-close-contracts`, before the track builds. Two questions relayed to Haik (dock;
  Remote Desktop), neither blocks a card.
- 2026-11-03 18:47: Card `p4-claude-live-shapes` done (Claude Code 2.1.317, 13 cases, 27 redacted fixtures; resume
  keeps `sessionId` and transcript, `CronCreate` jobs come back, `ScheduleWakeup` wakeups do not; SDK resume after a
  kill writes `stopped`, which rule 3 must close on). Its "Found outside scope": older lines of the signals doc that
  this contradicts (route to `p4-live-claude`); 13 dead trusted-folder entries in `~/.claude.json` and three Phase 0
  `cctest` folders in `%LOCALAPPDATA%\Temp\claude` (told Haik; left).
- 2026-11-03 18:50: Card `p1-close-contracts` done: the two `Updated` variants, owner lines in about 35 files,
  `architecture.md` spots plus 4 stale lines, `product.md` 1 line, Haik's answers in 16 cards and the 02 to 06 phase
  files (no card waits on a question), clang install as step 0 of `p6-velopack-dependency`, all 8 verify findings
  fixed. `Cargo.lock`: one line, `"proptest"` in the `wakeshot-cli` entry. Whole-workspace fmt, 4 clippy runs, 36
  tests, both release builds green with `--locked`. The worker ran `git diff --no-index` once by mistake (usage
  error, no change). `CLAUDE.md` owner line fixed by the orchestrator (an `Owner:` line in the module doc).
  Commits: live shapes, the integration plan, then this card with the verify review.
- 2026-11-03 18:52: Track builds, wave 1 started (four building cards): `p3-store`, `p3-log`, `p3-process`,
  `p2-ownership-restore`. Beside them a reviewer checks `p1-close-contracts` (no build; `docs/reviews/p1-close-verify.md`).
  `p6-int-skeleton` waits for Haik's sign-off of the integration plan (asked 18:40, no answer yet).
- 2026-11-03 18:58: Haik: "Signing off + dock station is always plugged in. As now. RemoteDesktop not sure needs to
  be tested". Integration plan signed off; the dock block of `p6-int-haik-checks` is in; Remote Desktop not tested
  before v1 (default). Recorded in `p6-plan-integration`, `06-integration.md`, `p6-int-haik-checks` and `CLAUDE.md`
  (test laptop). Order: no new build card starts as wave 1 cards end; when all four are done, `p6-int-skeleton`
  runs alone (about 50K), then wave 2.
- 2026-11-03 19:08: Close verify done (`docs/reviews/p1-close-verify.md`): 0 blocker, 1 major, 5 minor; all
  deliverables and the 8 earlier findings confirmed. Major 1 (a tray that starts while locked shows notices at once,
  against answer 4) routed with no contract shape change: `p3-session` adds `locked_now()`, `p6-int-startup` adds
  `locked_at_start()`, `p6-int-dispatch`, `p6-int-update-wiring` and `06-integration.md` note 6 feed
  `Session(Locked)` right after `Started`, `p2-core-lid` 8a adds the test, `p6-int-skeleton` deliverable 9 adds the
  rely-on line in the `Event` doc. Minors 1 and 2 (owner lines) added to `p6-int-skeleton`; minors 3 to 5 fixed by
  the orchestrator (`product.md` answers block and the Claude-only approval limit; Questions 10 to 12 marked
  answered here; menu look answer in `p5-menu-paint`, `p5-menu-view` and `05-tray-ux.md`; `04-sources.md` status).
  Phase 1 review passes with these routed; push next.
- 2026-11-03 19:09: Pushed 4 commits (`bd655de..0f9960a`) after the push scan (one fine hit: a card naming the
  placeholder); CI run 41660862166. Asked Haik to sign off Phase 1.
- 2026-11-03 19:10: Card `p3-log` done (8 tests; 4 writers, 2000 lines each, 7 runs, no mixed line; offset `+04:00`;
  roll fails only while another app holds a log file, no line lost). Committed by path. Its stale `architecture.md`
  section 8 line goes to `p6-int-skeleton` deliverable 10. No new build card starts until wave 1 ends.
- 2026-11-03 19:13: Card `p3-process` done (22 tests, 19 pass, 3 ignored and run once; the guard's parent is
  Explorer, an elevated runner gets a Medium child, a Medium process may also use Explorer as parent). Committed by
  path. Its stale `architecture.md` section 5 line and a `research.md` line go to `p6-int-skeleton` deliverable 10.
  Its note that agents run High with `SeDebugPrivilege` (checked by the orchestrator: true) went into `CLAUDE.md`.
  Its proposal (`Spawned` says why it fell back): left, as its default says.
- 2026-11-03 19:13: CI run 41660862166 green (all three jobs). Haik: "ok signing off": Phase 1 signed off. Card
  `p2-ownership-restore` done (45 ownership tests, 63 in the crate; the four mutants each fail a test). Its notes:
  restore step 5 text and the 03 note 4 line go to `p6-int-skeleton` deliverable 10; the empty journal on a failed
  load goes to `p2-core-settings`; the stop after the journal delete goes to `p3-stop-points`. `cargo` is not on
  `PATH` in agent shells (checked): `CLAUDE.md` names its full path. Wave 1 waits only for `p3-store`.
  Later work is tracked in the phase files 02 to 06 and `06-integration.md`.
- 2026-11-03 19:15: `p3-store` done (15 tests, five runs; `write_atomic` falls back to a rename by handle on error 5;
  `HeldMutex` not `Send`). Wave 1 committed (3 commits, tree clean). `p6-int-skeleton` started alone. New SYSTEM
  request from this session (pwsh PID 12276) until about 00:15, so agents keep working past 20:20. Last line of this
  file's log: from now on each track's phase file has its own.

