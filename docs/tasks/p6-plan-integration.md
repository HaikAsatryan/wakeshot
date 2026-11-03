# Plan the integration (Phase 6, second part)

Phase: 6. Status: done. Needs Haik: yes, at the end: the orchestrator relays the plan summary and its questions;
cards that depend on an unanswered question wait. Agent: `wakeshot-worker`. Size: about 140K tokens.
Runs after: Haik's sign-off of the track plans and the first push. Parallel with: `p1-close-contracts` and any
build card of any track (this card writes no code and builds nothing; it owns only the files below).
`p1-close-contracts` adds `Event::Updated { version }` and `Notice::Updated { version }` (Haik's yes to Question 7)
while this card runs: plan their wiring (the tray posts `Updated` once at start when Velopack restarted it).

## Goal

The five track plans leave the wiring for later (cross-track review M10). Plan it: wiring the tracks into the two
exes (`wakeshot-tray`, `wakeshot-cli`), the end-to-end tests, and Haik's manual checklist (`architecture.md` section
9). Split it into task cards with zero conflicts and no lost work, test-first, that start as soon as the track cards
they need are done.

## Read first

- `CLAUDE.md`; `docs/plan.md`: "How we work" (parallel, laptop and Git rules) and the note under the Phases table.
- `docs/architecture.md`: the process and thread layout, the tray loop, startup and shutdown order, section 9 (the
  manual checklist). Skim the rest for anything no track owns.
- `docs/product.md`: what v1 must do, end to end.
- Every track phase file, the parts that hand work to integration: `docs/phases/02-core.md` ("Where other tracks
  meet this one", Coverage), `03-windows-power.md` ("Manual checklist" table rows marked integration),
  `04-sources.md` (the "Integration" paragraph under the order), `05-tray-ux.md` (section 5, "Integration notes"),
  `06-ship.md` and `docs/tasks/p6-plan-ship.md` (the table of what other tracks give, rows marked integration).
- `docs/reviews/p2-6-cross-track.md`: M10.
- `crates/`: the module layout; each file's first doc line names its owner card. `crates/wakeshot-tray/src/main.rs`
  and `crates/wakeshot-cli/src/main.rs`, and any `#[expect(dead_code, reason = "integration ...")]` in the code
  (grep for `integration`): each one is a wiring job.
- `docs/phases/02-06-signoff.md`: Haik's answers (all defaults, 2026-11-03 18:09).

## Deliverables

1. `docs/phases/06-integration.md`: goal, short notes (only what `architecture.md` leaves open), the card table
   (Card, Needs Haik, Laptop, Runs after, Status), the order, the manual checklist map (each section 9 item to the
   card or session that covers it, none left out), and a work log section. Design changes go in this card's Result
   as proposals (section and new text); `architecture.md` stays unchanged.
2. Task cards `docs/tasks/p6-int-*.md` from `docs/tasks/_template.md`. Each card:
   - fits about 150K tokens of one worker; split bigger work;
   - names the files and folders it owns. Integration owns the wiring files (`main.rs` of both exes and the tray loop
     files no track card owns). When a card must edit a file a track card owns, it runs after that card is done and
     says so ("takes over `<file>` after `<card>`"); list every such hand-over;
   - is test-first: the acceptance checks name the tests and the cargo commands (`CLAUDE.md` "Checks", with
     `--locked`; while other cards run, only its own package and files, as the phase notes say);
   - says its build folder: `CARGO_TARGET_DIR=<scratchpad>\<card name>\target`;
   - is marked when it touches the real laptop (power settings, screen, sound, installs, sign-out, Explorer restart,
     reboot) or needs Haik's hands, with his minutes. The orchestrator runs those one at a time across all tracks;
     sign-out, Explorer restart and reboot cards run with no other agent;
   - fills "Runs after" with real card names (track cards or other `p6-int-*` cards) and "Parallel with".
3. The last card is Haik's checklist session: exact steps he types or does, each with what he should see, grouped
   so his time is short; the minutes stated.
4. In this card's Result: any contract change integration needs (exact change), so the orchestrator can run one
   more contracts card alone before the first integration card; and the critical path from today's track plans to
   the end of integration (cards and tokens).
5. In this card's Result: "Questions for Haik", each with your default. Only product decisions; facts you find in
   the code and docs.

## Acceptance checks

- Every hand-off to integration in the five phase files and in `p6-plan-ship` is in a card (a table in the Result:
  source line, card).
- Every `architecture.md` section 9 item is mapped to a card or session.
- Every `#[expect(dead_code, reason = "integration ...")]` in `crates/` is named in a card.
- No two integration cards that can run at once own one file; no integration card owns a file a track card owns
  while that card can still run.
- Every "Runs after" name is a real card (check with a script; record it).
- No code written; `crates/`, `architecture.md`, `product.md` and the track phase files and cards unchanged.
- No em or en dashes; no new line over 120 characters outside tables.

## Out of scope

- Code; `crates/`; `Cargo.toml`; `docs/architecture.md`; `docs/product.md`; the track phase files and cards.
- Phase 7 (field test) and Phase 8 (launch cleanup).
- Git: never run it.

## Questions for Haik

Each with the worker's default. Neither blocks a card.

Answered 2026-11-03 18:58. Haik signs off the integration plan. 1: his dock with an external screen is always plugged
in, so block 6 is in, not optional. 2: "not sure needs to be tested": the default stands (not tested before v1).

1. **A USB-C dock for your checklist session.** Checklist 8 (dock AC drop, unplug with the lid closed) and the "not
   while typing on an external screen" part of 7 need a dock with an external screen and keyboard. Default: block 6
   of `p6-int-haik-checks` is optional (about 15 minutes); without a dock at hand those checks move to the Phase 7
   field test, before v1.0.
2. **Remote Desktop into your session** (the last part of checklist 11) needs a second PC. Default: not tested before
   v1; the README's "one signed-in user per PC" line stands, and Phase 7 runs it if a second PC is at hand.

## Found outside scope

Problems seen but not fixed.

1. `docs/phases/02-core.md` line 176 lists "the IO queue" as integration's; `io_queue.rs` is `p3-power-io`'s
   (integration only uses it). Stale wording.
2. `docs/phases/05-tray-ux.md` line 215 says checklist 16 is "Phase 3 or integration", and `architecture.md`
   section 9 gives Phase 3 for items 7, 8, 9, 10 and 16: both now map to integration cards (proposal 5 below).
3. `docs/tasks/p0-architecture-review.md` still says `Status: ready` although Phase 0 is signed off.
4. `p6-velopack-dependency` builds velo's feed as `GithubSource` with pre-releases off, so the Question 9 public
   pre-release can test the release workflow but never reaches an installed build's update check. Not a bug
   (intended); recorded so nobody plans an update test on it. The update path test uses a debug-only local feed.
5. Hosted CI runners and hidden windows: whether `app::window` tests pass on `windows-latest` and `windows-11-arm`:
   not verified (`p6-int-window` records it after the next push).

## Log

- 2026-11-03 18:14: card written by the orchestrator (cross-track review M10; Haik's 18:02 order: an integration
  plan card after the push).
- 2026-11-03: worker started; reading plan, architecture, the five phase files, M10, crates and sign-off answers.
- 2026-11-03: read architecture 1 to 4 and 8 to 9, the five phase files, M10, both `main.rs`, `sink.rs`,
  `io_queue.rs`. Found: no `#[expect(dead_code, reason = "integration ...")]` in `crates/` today (only the CLI's
  `command.rs` one, owned by `p4-cli-args`); `velo.rs` gets one later from `p6-velopack-dependency`. The tray bin
  declares only `Win32_Foundation` and calls Windows only through the lib, so the loop window and the real sink need
  a new `wakeshot-windows` module: a contract change. A read-only agent is extracting the track cards' APIs.
- 2026-11-03: agent extract back (APIs of 28 cards). Facts used: `PowerExecutor::run` returns events, no thread;
  `IoQueue::session_ending` puts the restore first itself; the Leftover owner check is the executor's; `TrayIcon` is
  loop-thread only and its menu runs a modal loop; velo's feed is `GithubSource` with pre-releases off, so the update
  e2e needs a local feed seam; `research.md` "`NIF_GUID` and the exe path": a moved exe keeps its GUID icon.
  Design picked: wiring in a new lib module `app` (window, dispatch, io_thread, startup, tray_mode), the bin thin.
  Writing the phase file and 11 cards now.
- 2026-11-03: `docs/phases/06-integration.md` written (notes, owner table, card table, order, checklist map). Next:
  the 11 cards, then the name and line checks by script.
- 2026-11-03: cards written: skeleton, io-thread, window, dispatch, startup, tray-mode, update-wiring. Next: the
  three e2e cards and Haik's checklist card.
- 2026-11-03: all 11 cards written (dry-run-e2e, real-e2e, update-e2e, haik-checks added). Haik's session: about 27
  minutes, plus about 15 optional with a dock. Next: scripts for names, lines, dashes and the critical path.
- 2026-11-03: `check_plan.py` (scratchpad `p6-plan-integration\`) run twice: every "Runs after" name is a real card,
  the phase table matches the cards, no dash, no long line; critical path 1.35M tokens. Result written. Done.

## Result

Done 2026-11-03. The integration plan: 11 cards, about 1.19M tokens; one contract change; Haik about 27 minutes
(18 of hands) plus 15 optional, in one session at the end.

### Files written (no other file changed)

- `docs/phases/06-integration.md` (new): goal, notes 1 to 9 (where the wiring lives and the owner table, the seams,
  the loop window, the dispatcher with its runner table, the IO thread, startup and shutdown order, test layers, the
  update test seam, checks while others run), card table, order, critical path, checklist map, work log.
- `docs/tasks/p6-int-skeleton.md`, `p6-int-io-thread.md`, `p6-int-window.md`, `p6-int-dispatch.md`,
  `p6-int-startup.md`, `p6-int-tray-mode.md`, `p6-int-update-wiring.md`, `p6-int-dry-run-e2e.md`,
  `p6-int-real-e2e.md`, `p6-int-update-e2e.md`, `p6-int-haik-checks.md` (new).
- This card (Status, Log, Questions, Found outside scope, Result).

### Contract change integration needs (run alone before the first integration card)

It is card `p6-int-skeleton` (Runs alone, after `p1-close-contracts`), exactly:

1. `crates/wakeshot-windows/src/lib.rs`: add `pub mod app;` as the first module line.
2. New `crates/wakeshot-windows/src/app.rs` (owner: `p6-int-skeleton`, frozen): `pub mod dispatch; pub mod io_thread;
   pub mod startup; pub mod tray_mode; pub mod window;`, `pub enum Flow { Go, Quit }`, `pub trait LoopHandler { fn
   event(&mut self, event: Event) -> Flow; fn session(&mut self, change: SessionChange) -> Flow; fn timer(&mut self)
   -> Flow; fn next_timer_ms(&self) -> u32; }`, and `pub struct Ack` (`new`, `set`, `reset`, `wait(timeout_ms) ->
   bool`, over `Arc<(Mutex<bool>, Condvar)>`, with tests).
3. New files with only an owner doc: `app/window.rs`, `app/dispatch.rs`, `app/io_thread.rs`, `app/startup.rs`; and
   `app/tray_mode.rs` with `pub fn run(dry_run: bool) -> ExitCode` returning `SUCCESS`.
4. `crates/wakeshot-tray/src/main.rs`: the `Mode::Tray | Mode::DryRun` arm calls
   `wakeshot_windows::app::tray_mode::run(dry_run)`; the doc line names the integration owners.
5. No `Cargo.toml`, `Cargo.lock` or `windows` feature change: the lib already declares `Win32_UI_WindowsAndMessaging`,
   `Win32_System_RemoteDesktop` and `Win32_System_Threading` (read in `crates/wakeshot-windows/Cargo.toml`).

No core contract change: `Started`, `InputIdle`, `Session`, `JournalCorrupt`, `UpdateReady` and `Done` cover the loop;
`Event::Updated` and `Notice::Updated` come from `p1-close-contracts`, and the core's handling of `Updated` is
`p2-core-lid`'s (its card, line 59). Wiring for it: the tray passes `Updated { version }` once at start, after
`Started`, when Velopack restarted it (`p6-int-update-wiring`, through `Dispatcher::new`'s `first` events).

### Critical path, from today's track plans to the end of integration

11 cards, about 1.35M tokens: `p2-policy-holds` (100K), `p2-core-holds` (150K), `p2-core-settings` (150K),
`p2-core-lid` (130K), `p6-int-dispatch` (140K), `p6-int-tray-mode` (140K), `p6-int-update-wiring` (90K),
`p6-int-dry-run-e2e` (130K), `p6-int-real-e2e` (130K), `p6-int-update-e2e` (120K), `p6-int-haik-checks` (70K).
Computed by the script below over every card's "Runs after" and size (cards with no size, such as
`p1-close-contracts`, count 0). Next longest into integration: the power chain (`p2-ownership-restore`,
`p2-ownership-apply`, `p3-power-io`, then `p6-int-io-thread`: 500K). The real pace: four integration laptop slots
(tray-mode Part B, dry-run e2e, real e2e, update e2e) and Haik's session, one at a time across all tracks, after the
tracks' own laptop cards (`p3-haik-checks`, `p5-haik-check`).

### Hand-offs to integration and the card that takes each

Line numbers as read on 2026-11-03, before `p1-close-contracts` edits those files.

| Source line | Hand-off | Card |
|---|---|---|
| `02-core.md` 154 | wire `Core` into the tray loop after `p2-core-lid` | `p6-int-dispatch` |
| `02-core.md` 176 | the loop, the 30 s safety tick timer, the IO queue, the end-of-session restore at the queue front, the command runner | `p6-int-window` (loop), `p6-int-dispatch` (timer, tick, end of session, routing), `p6-int-io-thread` (runner side) |
| `03-windows-power.md` 106 | the loop and its hidden window; the window's owner calls `session::decode` | `p6-int-window` |
| `03-windows-power.md` 107 | the real event sink | `p6-int-window` (`WindowSink`) |
| `03-windows-power.md` 107 to 108 | the IO thread loop: `IoQueue`, the executor, the file commands, `FinishRestoreRequest` | `p6-int-io-thread` |
| `03-windows-power.md` 108 | `Command::Log` | `p6-int-io-thread` |
| `03-windows-power.md` 109 | the settings mutex retry on each tick | `p6-int-dispatch` (with `p6-int-startup`'s `try_settings`) |
| `03-windows-power.md` 109 | tray startup order | `p6-int-startup`, `p6-int-tray-mode` |
| `03-windows-power.md` 109 to 110, 159, 161 | checklist 7, 8, 9, 10, 16 and the second-session part of 11 | `p6-int-haik-checks` (7, 8, 10, 11, 16), `p6-int-real-e2e` (9) |
| `03-windows-power.md` 93 to 95 (note 7) | the tray reports a corrupt journal left by others as `JournalCorrupt` and renames it `.seen` | `p6-int-startup` (`corrupt_left`), `p6-int-dispatch` (first events) |
| `04-sources.md` 185 | start the watch thread with the three sources and the real `process::Liveness` | `p6-int-tray-mode` |
| `04-sources.md` 186 | post its events after `Started` | `p6-int-dispatch` (`Started` first), `p6-int-tray-mode` (order) |
| `04-sources.md` 186 | delete hold files on `DeleteHold` | `p6-int-io-thread` |
| `04-sources.md` 186 to 187 | end-to-end checks of `run`, `keep-awake`, `status`, `restore` against a running tray | `p6-int-dry-run-e2e`; `restore` on real plans also `p6-int-real-e2e` |
| `05-tray-ux.md` 35 | the tray bin | `p6-int-skeleton` (`main.rs`), `p6-int-tray-mode` |
| `05-tray-ux.md` 49 | the balloon click hook opens the click through `shell.rs` | `p6-int-tray-mode` |
| `05-tray-ux.md` 114 to 115 | `TrayIcon::new(kind, OwnerDrawnMenu::new(sink))` at step 8; `Tray(view)` runs `icon.show`; `on_balloon_click` | `p6-int-tray-mode` (objects, `RealRunners`), `p6-int-dispatch` (routing) |
| `05-tray-ux.md` 117 | `BalloonNotifier::new(identity)` on the IO thread runs `Notify`, gives `Notified` | `p6-int-io-thread`, `p6-int-tray-mode` |
| `05-tray-ux.md` 118 to 119 | `Play` on the loop thread; the hibernate thread holds the player and calls `play_and_wait` | `p6-int-dispatch` (routing), `p6-int-tray-mode` (the hibernate thread with the player) |
| `05-tray-ux.md` 120, 214 | `OpenLog` off the loop thread with the log path from `store` | `p6-int-io-thread` |
| `05-tray-ux.md` 121 to 122, 217 | real-exe checks: balloon header, exe icon in Explorer and Start, the product GUID's `NotifyIconSettings` entry, plain-English log lines | `p6-int-dry-run-e2e` (header, Explorer icon, log), `p6-int-real-e2e` (product GUID entry, log), `p6-int-haik-checks` block 1 (Start entry, header) |
| `05-tray-ux.md` 215 | checklist 16 after `p5-sounds` | `p6-int-haik-checks` block 2 |
| `05-tray-ux.md` 217 | wiring into the tray exe | `p6-int-tray-mode` |
| `06-ship.md` 30 | a bin flags code no `main` calls until integration | phase file note 1 (wiring in the lib); `p6-int-update-wiring` removes `velo.rs`'s `expect` |
| `06-ship.md` 125 | start the update thread in tray mode; run `ApplyUpdate` and `Quit` in the loop | `p6-int-update-wiring` (thread, `apply_update`), `p6-int-dispatch` (routing, `Quit`) |
| `06-ship.md` 126 | the update path end to end (a pre-release or a local feed) | `p6-int-update-e2e` (local feed, phase file note 8) |
| `06-ship.md` 126 to 127; `p6-plan-ship.md` 231 | checklist 14: `winget upgrade` and uninstall during a real hold | `p6-int-update-e2e` (Setup.exe over the install, which is what `winget upgrade` runs, and the uninstall, each during a real hold); `winget` itself in Phase 8 |
| `p6-plan-ship.md` 170 to 171 | `velo.rs` items with `#[expect(dead_code, reason = "integration starts the update thread")]` | `p6-int-update-wiring` |
| `p6-plan-ship.md` 213 | the core's update rule, needed by integration; with Question 3, `Updated` gives the notice | core: `p2-core-lid`; wiring: `p6-int-update-wiring` |
| `p6-plan-ship.md` 218 | update thread; `ApplyUpdate` then `Quit`; the update notice; the update path end to end; checklist 14 | `p6-int-update-wiring`, `p6-int-update-e2e` |
| `p6-plan-integration.md` 7 to 8 | `Event::Updated` posted once at start when Velopack restarted the tray | `p6-int-update-wiring` |
| `p2-plan-core.md` 273 | sources' facts reach the core as agreed | `p6-int-dry-run-e2e` (a Claude Code session through the real watch thread) |
| `p2-plan-core.md` 274 | power runs `Log`, `FinishRestoreRequest`, `Leftover` (owner check); posts `JournalCorrupt`, `Started.first_run` | `p6-int-io-thread` (`Log`, `FinishRestoreRequest`, `Leftover` through the executor), `p6-int-startup` (`JournalCorrupt`, `first_run`) |
| `p2-plan-core.md` 285 | `Core` with every handler | `p6-int-dispatch` |
| `p3-plan-windows-power.md` 292 | `session::decode` for the loop window | `p6-int-window` |
| `p3-plan-windows-power.md` 295 | executor, `IoQueue`, startup, recovery modes, dry-run forms, harness | `p6-int-io-thread`, `p6-int-startup`, `p6-int-tray-mode` (dry-run forms, `TrayGuard`), `p6-int-real-e2e` (harness `end-session`) |
| `p4-plan-sources.md` 240 | `FinishRestoreRequest` end to end of `wakeshot restore` | `p6-int-io-thread`, `p6-int-dry-run-e2e`, `p6-int-real-e2e` |
| `p4-plan-sources.md` 245 to 247, 294 | `WatchThread` with the three sources, `SystemZone`, real liveness and sink after `Started`; `DeleteHold`; CLI end to end | `p6-int-tray-mode`, `p6-int-io-thread`, `p6-int-dry-run-e2e` |
| `p5-plan-tray-ux.md` 212 | the log path from `store` at integration | `p6-int-io-thread` (`OpenLog`) |
| `p2-6-cross-track.md` 299 to 307 (M10 list) | also: startup steps 1 to 8, single tray, panic hook, the 4 s end-of-session wait, the hibernate thread, the dry-run tray end to end | `p6-int-startup`, `p6-int-dispatch`, `p6-int-tray-mode`, `p6-int-dry-run-e2e` |

### Design changes proposed for `architecture.md` (section and new text)

1. Section 2, crate table, `wakeshot-tray` "Holds": replace "message loop and wiring only" with "and the tray-mode
   call into `wakeshot_windows::app::tray_mode::run`; no wiring logic". Why: phase file note 1.
2. Section 2, `wakeshot-windows` modules, new row `app` | "the tray mode's wiring: `window` (the loop's hidden
   top-level window, the pump, its one timer, `WindowSink`), `dispatch` (`Dispatcher`: the core in the loop, the
   runner for each command, the tick, end of session, quit), `io_thread` (the IO thread loop over `IoQueue`),
   `startup` (startup steps 3 to 7 and `Started`), `tray_mode` (the composition root)". The `sink` row: "The real sink
   comes with the loop (integration)" becomes "The real sink is `app::window::WindowSink`".
3. Section 4, "Tray threads", new bullet after the posting bullet: "The loop window handles `WM_CLOSE` (also
   `taskkill` without `/F`) as Quit from the menu. A handler call that re-enters (a runner that pumps messages, such
   as the modal menu) queues its event; the outer call feeds it to the core after its own batch."
4. Section 4, "Tray startup", step 8 ends: "then `Started`, then `JournalCorrupt` when a corrupt journal was left by
   another process, then `Updated` when Velopack restarted the tray. After Quit: the IO thread runs what is queued
   (up to 10 s), the update and watch threads stop, the icon and the requests go, the mutexes are released."
5. Section 9, a paragraph after the Windows layer list: "Integration (Phase 6): wiring tests with fakes and the real
   core in CI; process tests of the real exes in dry run; the real tray on active plan copies; the update path on a
   real install with a local feed (debug builds only, `WAKESHOT_UPDATE_FEED`); Haik's checklist on an active plan
   copy." Checklist table, Phase column: items 7, 8, 9, 10 and 16 "3" become "6 (integration)"; item 11 "3" becomes
   "3; console part 6".

### Design picks (worker as principal designer), each with its trigger in the phase file

Wiring in a lib module `app`, not the bin (note 1); hidden top-level loop window with fixed class names, `WM_CLOSE` as
Quit (note 3); one timer, `InputIdle` before every `Tick`, re-entry queue (note 4); IO drain on quit capped at 10 s
(note 5); second start stays silent (note 4); `WAKESHOT_UPDATE_FEED` in debug builds only (note 8); integration plan
copies named `Wakeshot int *` (note 7); Haik's session on an active plan copy, so his own plan is never written;
console left and back tested with "Switch user" on one account (not verified that it gives `WTS_CONSOLE_DISCONNECT`
there; `p6-int-haik-checks` block 3 says to skip if it is missing); checklist 9 and 14 run by agents, no hands.

### Checks

- Every hand-off to integration in the five phase files and `p6-plan-ship` is in a card: pass (table above).
- Every section 9 item is mapped: pass (phase file, "Manual checklist map", rows 1 to 16).
- Every `#[expect(dead_code, reason = "integration ...")]` in `crates/`: none exists today (grep of `expect(` in
  `crates/`: only `wakeshot-cli/src/command.rs`, reason "p4-cli-args removes this attribute", not integration's).
  The one `p6-velopack-dependency` will add in `velo.rs` is named in `p6-int-update-wiring`: pass.
- No two integration cards that can run at once own one file: pass. Only `p6-int-io-thread`, `p6-int-window`,
  `p6-int-dispatch` and `p6-int-startup` can run at once, and each owns one different `app/*` file; every later card
  runs after the card that owned its files (hand-over list in the phase file).
- No integration card owns a track file while that track card can still run: pass. `velo.rs` after
  `p6-velopack-dependency`; the CLI command files after `p4-cli-args`, `p4-cli-run`, `p4-cli-commands`,
  `p4-cli-restore`; `lib.rs` and the tray's `main.rs` by `p6-int-skeleton`, run alone.
- Every "Runs after" name is a real card: pass. Script
  `<scratchpad>\p6-plan-integration\check_plan.py` (run `python -I check_plan.py C:\Users\haika\code\wakeshot`): all
  names in the 11 cards and in the phase file's table exist in `docs/tasks/`, and the table matches each card.
- No code written; `crates/`, `architecture.md`, `product.md`, the track phase files and cards unchanged by this card:
  pass by this card's own edits (only the files listed above); not checked with git (never run).
- No em or en dashes; no new line over 120 characters outside tables: pass (the same script).

### Open items

- The two questions above (neither blocks a card).
- The orchestrator routes the five design proposals and the stale lines under "Found outside scope".
- `p1-close-contracts` is editing the phase files and cards now; if it changes a "Runs after" or a card name used here,
  re-run the script.
