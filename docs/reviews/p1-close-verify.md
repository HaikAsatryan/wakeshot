# Phase 1 close: verify `p1-close-contracts`

Reviewer: `wakeshot-reviewer`, 2026-11-03, about 18:55 to 19:30. Read only. No build, no git, no registry or power use.

Brief: check the card's deliverables, the verify review's findings, the two `Updated` variants and their cards, the 12
answered questions, the owner lines (with the integration owner table), and the `Cargo.lock` change. Files of the four
running build cards (`p3-store`, `p3-log`, `p3-process`, `p2-ownership-restore`) are ignored where they changed.

Counts: 0 blocker, 1 major, 5 minor.

What I ran (scripts in the scratchpad, `p1-close-verify\`):

- `owners.py`: every `.rs` file under `crates/`, its `Owner:` paragraph, and whether each card named there exists and
  names the file.
- `order.py`: every "Runs after" of the p2 to p6 build cards (integration cards included), its closure, and whether
  every pair of cards named in one file's owner paragraph is ordered. Plus the integration hand-overs by hand.
- `text.py`: en and em dashes and lines over 120 characters in the changed files.
- `diff` of `Cargo.lock` against the worker's copy taken first (`scratchpad\p1-close-contracts\Cargo.lock.before`,
  18:31).
- Greps for "Question", "answer", "until", "only if", "still open", "is open" and similar over the 02 to 06 phase files
  and the p2 to p6 cards.

## Major

### Major 1. A tray that starts in a locked session shows the update notice at once, against Haik's answer 4

- Haik's answer 4 (`docs/phases/02-06-signoff.md:34`): notices raised while you are locked wait until you are back.
  The core builds it from session events only: `docs/tasks/p2-core-lid.md:49-51` (item 8a, "wait and go out at
  `Unlocked` or `ConsoleBack`"). The core learns "locked" only from `Session(Locked)`, which comes from
  `WM_WTSSESSION_CHANGE` on a change, never as a start state.
- At start the core gets `Started { .., on_console, .. }` (`crates/wakeshot-core/src/event.rs:35-38`, `:51-58`): the
  console state, but no lock state. No card reads it: `docs/tasks/p3-session.md:31` gives only `on_console()`, although
  its Read first (`:20`) names the spike's `session_locked`. Integration feeds `Started`, then `JournalCorrupt`, then
  `Updated` (`docs/phases/06-integration.md:141-142`; `docs/tasks/p6-int-dispatch.md:34`), and nothing for the lock.
- When the new notice fires: Velopack restarts the tray after an update (`docs/phases/06-ship.md:75`), and an update
  applies only when nothing holds, after a check every 12 hours of wall time
  (`docs/tasks/p6-update-schedule.md:30`). So the most likely restart is while the user is away, often locked. The
  same start happens when the guard gives up and starts the tray (`docs/tasks/p3-recovery.md:47`), where the tray's
  own startup restore can fail and raise `RestoreFailed`, the warning that says the lid may still be on Do nothing.
- What breaks: `Event::Updated` reaches a core that thinks the session is unlocked, so `Notify(Updated)` (and a
  start-time `RestoreFailed` or `JournalCorrupt`) goes out on the lock screen. Whether a balloon raised there is lost:
  not verified; `p1-lid-sound` Result item 4 notes toasts are hidden on the lock screen by default (not tested), which
  is why Haik's answer 4 exists. A missed `RestoreFailed` leaves the user unaware of an override.
- Fix, no contract shape change: `p3-session` (not started) adds `locked_now() -> Option<bool>`, a port of
  `spikes/lid-sleep/src/sys.rs:196-215` (`WTSQuerySessionInformationW`, `WTSSessionInfoEx`, `SessionFlags`; the feature
  `Win32_System_RemoteDesktop` is declared). Integration (`p6-int-startup`, `p6-int-tray-mode`, `p6-int-dispatch`)
  passes `Session(SessionChange::Locked)` in `first` right after `Started` when it returns `Some(true)`, before
  `JournalCorrupt` and `Updated`. One rely-on line in the `Event` doc (a contract file: `p6-int-skeleton` runs alone and
  can carry it). `p2-core-lid` adds a test: `Started`, `Session(Locked)`, `Updated` gives no notice until `Unlocked`.
  The other way, a `locked` field in `Started`, changes a contract type and needs a solo card before `p2-core-holds`.

## Minor

### Minor 1. The integration hand-overs are not in the files' owner lines

- `crates/wakeshot-tray/src/velo.rs:1-3`: "Owner: ship track (`p6-velopack-dependency`)". The integration table and
  hand-overs give it to `p6-int-update-wiring` next, then fixes by the e2e cards (`docs/phases/06-integration.md:38`,
  `:45`, `:210-211`).
- The seven CLI command files name only their p4 card (`args.rs`, `run.rs`, `keep_awake.rs`, `automatic.rs`,
  `status.rs`, `restore.rs`, `open_log.rs`, each at line 3), while `06-integration.md:45-47` lets `p6-int-dry-run-e2e`
  fix them.
- This card set the pattern for the same case ("fixes by `p4-live-claude`" in 13 files), so the files now say two
  things. `p6-int-skeleton` runs alone but its Out of scope bars every track file, so no card will add these lines.
- What breaks: nothing in the order (`order.py`: every hand-over pair is ordered). The text misleads:
  `p6-velopack-dependency` "owns it whole" and reads no hand-over; the `p6-int-dry-run-e2e` worker opens `args.rs`,
  sees only `p4-cli-args`, and may list a bug as outside scope instead of fixing it.
- Fix: one line per file, added to `p6-int-skeleton`'s deliverables (it runs alone): "then `p6-int-update-wiring`;
  fixes by the e2e cards" in `velo.rs`, "fixes by `p6-int-dry-run-e2e`" in the seven CLI files, and "fixes by the e2e
  cards" in the `app/*` owner lines it writes.

### Minor 2. The two `main.rs` files have no `Owner:` line

- `crates/wakeshot-cli/src/main.rs:1-2` and `crates/wakeshot-tray/src/main.rs:1-2` say "This dispatch is the contracts
  card's and stays as it is". `CLAUDE.md:11-12`, `docs/architecture.md:45` and both `lib.rs` now say every file's
  module doc names its owner in an `Owner:` line.
- The tray one also "stays as it is", but `p6-int-skeleton`, then `p6-int-update-wiring` edit it
  (`06-integration.md:37`). `p6-int-skeleton` deliverable 4 rewrites the tray file's doc line; the CLI file stays
  without one.
- What breaks: an owner grep like the verify review's `owners.py` reports both as unowned; until `p6-int-skeleton`
  runs, the tray file contradicts the integration plan.
- Fix: "Owner: contracts card." in the CLI `main.rs` (any solo card, for example `p6-int-skeleton`). The two
  `build.rs` and `build-resources.rs` carry `// Owner:` as plain comments: fine for build scripts.

### Minor 3. `product.md` does not record the 12 answers, and one line now contradicts answer 1

- `docs/plan.md:70-71` and `CLAUDE.md`: decisions live in `docs/product.md`. Only Question 11 got a line
  (`product.md:127`). The rest (Codex holds with no limit, Copilot waiting does not hold, chimes honor No sounds,
  notices wait while locked, the update notice, the data folder at uninstall, `NotifyIconSettings` left, the public
  pre-release) are only in the sign-off file, the phase files and the cards.
- `docs/product.md:50-51`: "waiting for approval holds at most 1 hour ... a product rule". Answer 1 says a Codex turn
  that waits for approval holds with no limit (`docs/phases/04-sources.md:193-198`).
- What breaks: Phase 8 removes internal planning (`plan.md:85`), so the decisions would survive only in the README and
  the code; a reader of `product.md` (the README card, `p5-text`) finds a 1 hour approval limit that Codex does not
  have, and no update notice.
- Fix (orchestrator, `product.md`): one short block "Answers to the track plan questions (Haik, 2026-11-03 18:09)" and
  "for Claude" in line 50. The card's deliverable 4 named only cards and phase files, so this is a gap in the brief,
  not the worker's slip.

### Minor 4. The Phase 1 file still calls Questions 10 to 12 open

- `docs/phases/01-design-skeleton.md:26` ("Open questions, each with the default I take") and `:144` ("Each with the
  default the design takes until you answer"). The answer is only in the work log (`:617`).
- `docs/tasks/p2-core-settings.md:26` and `docs/tasks/p3-recovery.md:22` send workers to those questions, and
  `architecture.md:7-8` says the questions stay in that file.
- What breaks: little; a worker reads "until you answer" next to a card header that says answered.
- Fix (orchestrator, its own file): "Answered 2026-11-03 18:09: the defaults" at both spots.

### Minor 5. Stale waits outside the 12 questions

- The menu look answer (Haik 17:32) is recorded only in `docs/tasks/p5-menu-paint.md:23-26`. Still waiting on it:
  `p5-menu-paint.md:3-4`, `docs/tasks/p5-menu-view.md:3-4` ("if his answer changes the pick ... stop and ask"),
  `docs/phases/05-tray-ux.md:126` ("Waiting on Haik") and its table rows `:185`, `:187`. `p5-menu-view` never says what
  he answered (pick stands, an icon per row).
- `docs/phases/04-sources.md:3-4`: "No card started", but `p4-claude-live-shapes` is done (`:109`).
- What breaks: the `p5-menu-view` worker may stop to ask, or not know rows get icons.
- Fix (orchestrator): the answer in `p5-menu-view` and the 05 file; drop "No card started".

## Checks that passed

### Deliverables

| # | Outcome | Evidence |
|---|---|---|
| 1 | Done | `event.rs:47-48`, `:88-90`: `Updated { version }` with the owner list (`:33`) and a rely-on bullet; `view.rs:90-91`, `:107`. `text.rs` is in the core and has one `notice(&Notice)` (`:82`), so no new signature; its `NoticeText` doc (`:31-32`) allows a quiet notice. No `match` in `crates/` lists every `Event` or `Notice` variant (grep). `Cargo` changes: only Major 1's (below). |
| 2 | Done | `text.rs:4` and `art.rs` name `p5-haik-check`; 13 files name their live card (`owners.py`). The card said "first doc line"; the worker used the `Owner:` line, which is right after verify Minor 2. |
| 3 | Done | `architecture.md:4` header, `:45` owner claim, `:97` `install` row, `:223-226` and `:304-305` the variants, `:616` restore routine name (was 612), `:819-820` `store::AutoStart`, `:949-951` test plan names. No `restore_from_journal`, `FakePower` or `PowerApi` left outside history (grep). |
| 4 | Done | Every spot the card and verify Minor 1 name, read: `p2-core-lid` 8a and 9a, `p2-core-settings:3-5`, `p3-recovery:3-4`, `:47`, `:93`, `p3-haik-checks:66`, `p4-cli-restore:3`, `:74-76`, `p5-sounds:3-5`, `p6-velopack-dependency:3-5` (step 0), `p6-pack-and-release:3-4`, `p6-readme:3-5`, `:41`, `:48`; phase files 02 note 18, Coverage rows; 03 notes 9 and table; 04 notes and Questions; 05 section 6; 06 Questions and order step 4. |
| 5 | Done | Table in the Result; checked one by one below. |
| 6 | Done | `docs/research.md:1307` "Close contracts checks". `GetCommandLineW` under `Win32_System_Environment` in the local `windows-0.62.2` source (`System/mod.rs:35`; `Environment/mod.rs:105`): confirmed. |

### Verify review findings (`docs/reviews/p1-fixes-verify.md`)

| Finding | Outcome |
|---|---|
| Major 1 | Fixed: `crates/wakeshot-windows/Cargo.toml:40`, `crates/wakeshot-cli/Cargo.toml:28-29`; `p4-cli-args.md:42-44`, `p4-cli-run.md:27` say so. |
| Major 2 | Fixed: `02-core.md` note 20 (scratch copy with `assets\`, own target, `--locked`, delete after); all six mutant cards point at it (`p2-ownership-props:65`, `p2-core-props:62`, `p2-ownership-restore:96-98`, `p2-ownership-apply:79-81`, `p2-core-settings:85-86`, `p2-core-lid:77-79`). |
| Minor 1 | Fixed (spots above); clang owned by `p6-velopack-dependency` step 0. |
| Minor 2 | Fixed: `Owner:` lines in all 17 files (`owners.py`); claims in both `lib.rs`, `architecture.md:45`; `CLAUDE.md:11-12` fixed by the orchestrator. Left: the two `main.rs` (Minor 2 here). |
| Minor 3 | Fixed: `p3-recovery.md:6-7`, `03-windows-power.md:132`. |
| Minor 4 | Fixed: `p5-tray-icon.md:9-10`. |
| Minor 5 | Fixed: `06-ship.md:25`, `p6-velopack-dependency.md` (`velo::feed()`). |
| Minor 6 | Fixed: `03-windows-power.md:69` has `--locked`; the wrong `text.rs` path in the card is listed in its Found outside scope. |
| Known open M10, m9, lines 612 and 943 | Closed: `p6-plan-integration` done; m9 is deliverable 1; both lines fixed. |

### The `Updated` variants and their cards

- Match `docs/tasks/p6-plan-ship.md:161-163` item 4 (both fields `String`, `Notice` in `view.rs`, the core answers
  with `Notify`, the tray words it) and `architecture.md:223-226`, `:304-305`.
- `p2-core-lid.md:53-60` (9a): `Updated` gives `Notify(Updated)` once, two named tests. `p5-text.md:44`, `:53-55`: the
  fourteen notices, `Updated` quiet, not a warning, no click. `p6-int-update-wiring.md:12-13`, `:30`, `:38`, `:45`:
  velo gives the restarted version, `Updated` comes after `Started` and `JournalCorrupt`, a test that it is never
  first. `p6-int-dispatch.md:34`, `:68`; `06-integration.md:141-142`; `p6-int-update-e2e.md:41`. The one gap is the lock
  state at start (Major 1).

### The 12 questions

Each answer is written where a card builds it, and no p2 to p6 card or 02 to 06 phase file still waits on one (grep;
the questions' own text in `02-06-signoff.md` is marked answered at `:27-28`):

| Item | Where it is built |
|---|---|
| 1, 2 | `04-sources.md:193-198`; `p4-codex-source.md:17`, `:34`; `p4-copilot-source.md:18`, `:37`; both live cards |
| 3 | `p5-sounds.md:3-5`, `:56-59`; `05-tray-ux.md:127-128`, `:189` |
| 4 | `p2-core-lid.md:49-51`; `02-core.md` note 18 and Coverage (gap at start: Major 1) |
| 5 | `p6-readme.md:46`; `architecture.md:827-829`; `p6-int-update-e2e.md:3` |
| 6 | `p6-hooks.md:3`, `:41`; `06-ship.md:142` |
| 7 | as above |
| 8 | `p6-velopack-dependency.md:3-5`, step 0; `06-ship.md:92`, `:179-180`; 02 note 20 |
| 9 | `p6-pack-and-release.md:3-4`, `:57-58`; `06-ship.md:144` |
| 10 | `p3-haik-checks.md:66-67` |
| 11 | `p2-core-settings.md:3-5`, `:66`; `product.md:127`; `p3-recovery.md:3-4`, `:93`; `p4-cli-restore.md:3`, `:74-76`; `p6-readme.md:43-44` |
| 12 | `p6-hooks.md:50`; `p6-readme.md:47-48` |

### Owner lines and file ownership

- Every `.rs` file under `crates/` has an `Owner:` line except the two `main.rs` (Minor 2). Every card named in one
  exists and names the file; `p4-live-claude` names its files by folder (`sources/claude/*`, `watch/*`), which covers
  them (`owners.py`; `04-sources.md:167`).
- No two cards that can run at once own one file: every pair named in one file's owner paragraph is ordered by "Runs
  after" (`order.py`), and so is every integration hand-over (`main.rs`: `p6-int-skeleton` then
  `p6-int-update-wiring`; `velo.rs`: `p6-velopack-dependency` then `p6-int-update-wiring` then the e2e cards, which run
  one after another; `tray_mode.rs`; `tests/e2e/mod.rs`; the CLI files before `p6-int-dry-run-e2e`).
  `p6-int-skeleton` and `p6-velopack-dependency` are not ordered, but both run alone and the second never edits
  `main.rs` (`p6-velopack-dependency.md:74`).
- `p6-int-skeleton` will add `app.rs` and five `app/*` files with owner lines (`p6-int-skeleton.md:22-37`) and
  `pub mod app;` in `wakeshot-windows/src/lib.rs`, whose owner is the contracts card: right, since it runs alone.

### `Cargo.lock`

- `diff` against the worker's 18:31 copy: one line, `548a549 > "proptest",` in the `wakeshot-cli` entry
  (`Cargo.lock:545-553`). `proptest 1.12.0` was already locked (`Cargo.lock:247-251`). The file's time is 18:32, so
  nothing wrote it after the card. That the 18:31 copy equals the committed lock was not checked (no git); it matches
  the verify review's line numbers (`wakeshot-cli` at 545-552, no `proptest`).

### Text

- No en or em dash in the changed files (`text.py`). Lines over 120 characters outside tables: `p3-recovery.md:58`,
  `p4-cli-run.md:4`, `architecture.md:371`, `:384`, `product.md:21`; none is in a spot this card says it changed, so
  most likely older (not verified without git).

## Noted, not counted

- `p6-int-skeleton` runs alone (`p6-int-skeleton.md:4-7`) and needs no track code, but wave 1 started first
  (`01-design-skeleton.md:667-669`), so it now waits for a full drain of the running cards.
- `view.rs:87-88`: "`ClosedLidReport` how long the PC ran with the lid closed (`mono_ms`)": the field is `ran_ms`,
  measured on the mono clock (`p2-core-lid.md:47`). Reads oddly; not wrong.
