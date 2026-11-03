# Plan the Tray UX track (Phase 5)

Phase: 5. Status: done. Needs Haik: yes, at the end: he signs off all five track plans at once (the orchestrator
relays one summary). Agent: `wakeshot-worker`. Size: about 120K tokens.
Runs after: `p1-design-verify-fixes` and the final design check (`docs/reviews/p1-design-final-check.md`), and
`p1-contracts` (the module layout and types in `crates/`). Parallel with: the other four track plan cards
(`p2-plan-core`, `p3-plan-windows-power`, `p4-plan-sources`, `p5-plan-tray-ux`, `p6-plan-ship`; each owns only its
own phase file and card prefix) and the Phase 1 review (read only).

## Goal

Phases 2 to 6 run as parallel tracks (`docs/plan.md`, the note under the Phases table). Plan this track: Icons (design, light and dark, high contrast, every scale), menu, notifications, sounds, first run, log view.
Split it into task cards that can be built at the same time as the other four tracks with zero conflicts and no
lost work, test-first, against the contracts in `crates/`.

## Notes from the orchestrator (16:38)

- The shared `wakeshot-windows` modules `store`, `log` and `process` belong to the Windows power track (Phase 3),
  which builds them first. Do not own them; use their fakes until those cards are done.
- Haik's look at the tray menu pictures (`p1-tray-menu-look`) is still pending; plan the menu cards on the current
  pick (owner-drawn items plus the DWM round-corner call) and mark them as depending on his answer.
- `docs/architecture.md` is getting 13 type corrections from `p1-contracts` while you plan. Where it disagrees with
  `crates/`, the code is right.

## Read first

- `CLAUDE.md`; `docs/plan.md`: "How we work" (the parallel and Git rules) and the note under the Phases table.
- `docs/architecture.md`: section 4 (tray threads, tray startup), section 5 ("Tray, notices, sounds"), section 8 (the user-visible log); also `docs/research/tray-menu-look.md` and the Results of `p1-tray-menu-look` and `p1-lid-sound`. Skim the rest for anything your track owns.
- `docs/product.md`: the lines your track serves.
- `docs/research.md`: only the sections the design cites for your track.
- `crates/`: the module layout and contract types written by `p1-contracts` (its card Result says what is where).
- `docs/phases/01-design-skeleton.md`: "Values I set" and the answered questions. Questions 10 to 12 are open and
  Haik's: plan around both answers or mark the card that depends on them; do not decide them.

## Deliverables

1. `docs/phases/05-tray-ux.md`: goal, short track notes (only what `architecture.md` leaves open for this track),
   the card table (Card, Needs Haik, Runs after, Status), the order inside the track, and a work log section.
   `architecture.md` stays the design: any design change you need goes in your card's Result as a proposal, with the
   section and the new text.
2. Task cards `docs/tasks/p5-*.md` from `docs/tasks/_template.md`. Each card:
   - fits about 150K tokens of one worker; split bigger work;
   - names the files and folders it owns. Inside `crates/`, own whole module files. Never `lib.rs` or `main.rs`
     module lists, `Cargo.toml` or `Cargo.lock`: those change only through the contracts card the orchestrator runs
     alone, so list what you need under deliverable 3;
   - is test-first: the acceptance checks name the tests and the cargo commands (`CLAUDE.md` "Checks");
   - says its build folder: `CARGO_TARGET_DIR=<scratchpad>\<card name>\target`;
   - is marked when it touches the real laptop: power settings, screen, sound, installs, or needs Haik's hands.
     The orchestrator runs those one at a time across all tracks;
   - fills "Runs after" (inside this track, or another track's card by name) and "Parallel with".
3. In your card's Result: the contract changes this track needs (missing or wrong types, traits, module files,
   dependencies), each with the exact change, for one contracts card the orchestrator runs before the builds.
4. In your card's Result: what this track needs from other tracks and from when, and the fake it uses until then.

## Acceptance checks

- Every item of `architecture.md` and `product.md` this track owns is in a card, or named as integration (later).
- No two cards of this track own one file at the same time, and none owns a file another track's section names.
- Every card that needs the real laptop or Haik is marked as such.
- No code written; `crates/`, `architecture.md`, `product.md` and other tracks' files unchanged.

## Out of scope

- Code; `crates/`; `Cargo.toml`; `docs/architecture.md`; `docs/product.md`; other tracks' phase files and cards.
- Integration cards (wiring the tracks into the two exes): planned after the tracks.
- Git.

## Questions for Haik

Each with the worker's default.

1. **Chimes and the Windows "No sounds" scheme** (open item of `p1-lid-sound`). When your sound scheme is "No sounds",
   should Wakeshot still play its three chimes? Default: no, it honors the scheme: no chime, and the log says it was
   skipped. The Volume mixer can also turn Wakeshot down or off on its own. The other option: always play, since the
   lid-close chime is a safety hint. Only `p5-sounds` depends on it; it takes the default if no answer has come.

Not questions: Questions 10 to 12 (`phases/01-design-skeleton.md`) change no card of this track. Your one look at the
menu pictures (`p1-tray-menu-look`) is still pending; the two menu cards follow it.

## Found outside scope

Problems seen but not fixed.

- Review B1 suggests a `tray.rs` mode file in the tray bin "for Phase 5". This track needs none: its on-screen checks
  run as examples in `wakeshot-windows/examples/`, and the tray mode is wiring (integration), as the orchestrator
  said. The contracts card can leave that file to integration.
- Phase 2's `Notify(IdleLocked)` (its track note 11) is raised when the screen locks for idle, so its balloon shows
  while the session is locked, the case where a balloon may be lost (balloons are not stored, research "Balloon").
  Proposal D5 below covers it.
- `crates/wakeshot-tray/build.rs` line 15: "the tray needs Common Controls 6 (`LoadIconMetric`)". With icons drawn at
  run time (proposal D4), the manifest stays for per-monitor v2 DPI and `supportedOS`, not for `LoadIconMetric`. Fix
  the comment in the contracts card or later.
- `windows` 0.62.2 has no `NIN_KEYSELECT`, and the owner-draw structs are in `Win32::UI::Controls` (research "Tray UX
  track plan checks"): told to `p5-tray-icon` and `p5-menu-paint` in their cards.

## Log

- 2026-11-03 16:27: card written by the orchestrator (Phases 2 to 6 as parallel tracks, Haik's ask).
- 2026-11-03 16:39: worker started; reading the design, crates and the Phase 1 cards.
- 2026-11-03 16:54: read architecture sections 2 to 5 and 8, product UX, the menu look and lid sound cards, the
  tray research, `crates/` (view, tray, notify, sound, testing, tray bin). Scratch build (outside the repo, own target
  folder): every Windows API the track plans to call compiles with the 32 declared `windows` features on x64 and ARM64,
  and `tray.rs` with `pub mod art;` resolves to `tray/art.rs`. Orchestrator relayed the contracts domain review
  (B1 split by owner, M1 event sink and window hooks, M5 going-to-sleep sound): taken into the plan.
- 2026-11-03 17:00: read the other planners' phase files for overlaps. Phase 2 already plans `text.rs` (strings, this
  track) apart from `present.rs` (structure, Phase 2) and `Started.first_run`: aligned, my first-run proposal dropped.
  Phase 4 owns `wakeshot-windows/src/shell.rs` (`open_with_default_app`): this track uses it and owns no shell file.
  Writing the phase file and nine cards.
- 2026-11-03 17:16: phase file, nine cards and the research section written; acceptance checks run (Result). Status
  done.

## Result

Done 2026-11-03 17:16. Docs only; no code in the repo. One scratch build outside the repo checked facts the plan
rests on (research, "Tray UX track plan checks").

### Files

- New `docs/phases/05-tray-ux.md`: goal, track notes (file owners; how the parts fit; ten design picks with reasons
  and triggers; rules for laptop cards; integration notes; what waits on Haik), where other tracks meet this one, card
  table, order, coverage table, work log.
- Nine new cards: `p5-theme`, `p5-icon-art`, `p5-text`, `p5-menu-paint`, `p5-tray-icon`, `p5-menu-view`, `p5-notices`,
  `p5-sounds`, `p5-haik-check` (`docs/tasks/`).
- `docs/research.md`: new section "Tray UX track plan checks".
- This card.

### Cards

| Card | Owns | Laptop | Needs Haik | Size |
|---|---|---|---|---|
| `p5-theme` | `tray/theme.rs` | no | no | 40K |
| `p5-icon-art` | `tray/art.rs`, `assets/wakeshot.ico`, `img/ux/icon-*` | no | one look at the sheet, not blocking | 120K |
| `p5-text` | core `text.rs` | no | one read of the text list, not blocking | 80K |
| `p5-menu-paint` | `tray/menu_paint.rs`, `img/ux/menu-render-*` | no | his menu look answer | 110K |
| `p5-tray-icon` | `tray/icon.rs`, `examples/ux_icon.rs`, `img/ux/taskbar-*` | screen; light/dark switch | no | 140K |
| `p5-menu-view` | `tray/menu.rs`, `examples/ux_menu.rs`, `img/ux/menu-screen-*` | screen; light/dark switch | his menu look answer | 130K |
| `p5-notices` | `notify.rs`, `examples/ux_notices.rs`, `img/ux/notice-*` | screen; Do not disturb switch | no | 110K |
| `p5-sounds` | `sound.rs` and `sound/*`, `examples/ux_sounds.rs` | sound | Question 1 (default taken) | 120K |
| `p5-haik-check` | `examples/ux_all.rs`, `img/ux/check-*`; afterwards his icon pick and wording | screen, sound | yes, about 25 minutes | 80K |

Total about 930K tokens. Each laptop card does its code first (Part A, no laptop), then blocks for the screen or sound
slot (Part B), so slot time is short. Order: `p5-theme`, `p5-icon-art`, `p5-text` and Part A of `p5-sounds` and
`p5-notices` start together after the contracts card; then `p5-menu-paint` and `p5-tray-icon`; then the laptop parts
one at a time; `p5-haik-check` last.

### Contract changes this track needs (deliverable 3)

For the one contracts change card the orchestrator runs alone, before any build. Eight changes; no new crate, no new
`windows` feature (checked by the scratch build), no file in `crates/wakeshot-tray/src`.

1. **Strings apart from structure (review B1).** New `crates/wakeshot-core/src/text.rs`, `pub mod text;` in the core
   `lib.rs`, owner tray track (`p5-text`), stub bodies with the product sketch's words. Phase 2 plans the same file
   (`docs/phases/02-core.md` notes 13, 16; `p2-present` calls `text::hold_group`, `status_line`, `tooltip`). Proposed
   shapes; merge with `p2-plan-core`'s list, and where they differ Phase 2's call sites decide the shape:

   ```rust
   pub const TOOLTIP_MAX_UTF16: usize = 127; // szTip[128] less the end mark
   pub fn action(command: MenuCommand) -> String; // menu item text with its & access key
   pub fn keep_awake_menu() -> String; // the submenu's own text
   pub fn hold_group(kind: HoldKind, label: &str, count: u32) -> String;
   pub fn keep_awake_group(left: KeepAwake) -> String; // Minutes(n): n minutes left
   pub fn more_groups(n: u32) -> String; // "and 2 more"
   pub fn status_line(status: &Status, groups: &[String]) -> String;
   pub fn tooltip(status: &Status, groups: &[String]) -> String;
   pub fn dry_run(tooltip: &str) -> String;
   pub fn notice(notice: &Notice) -> NoticeText;
   pub struct Status { pub icon: IconState, pub stepped_back: bool, pub requests_only: bool, pub screen_on: bool }
   pub struct NoticeText { pub title: String, pub body: String, pub warning: bool, pub silent: bool,
                           pub click: NoticeClick }
   pub enum NoticeClick { Nothing, OpenTaskbarSettings, OpenLog }
   ```

   `view.rs` docs: the module doc points at `text` for all words; `MenuItem`: "a text may hold `&` before its access
   key; `&&` shows one `&` (the Win32 rule)"; `Notice`: "worded by the tray track in `text::notice`".
2. **Tray module split.** `crates/wakeshot-windows/src/tray.rs` declares `pub mod art; pub mod icon; pub mod menu; pub
   mod menu_paint; pub mod theme;` with five stub files under `tray/` (one doc line naming the owner card), and adds
   `pub struct IconIdentity { pub hwnd: isize, pub guid: u128 }` (`Clone`, `Copy`, `Debug`, `PartialEq`, `Eq`): the
   notify icon's identity, so `BalloonNotifier` on the IO thread modifies the icon `TrayIcon` owns on the loop
   thread. Module doc: "`TrayIcon` owns its own hidden window on the loop thread; no tray message reaches the loop's
   window." After this, no card edits `tray.rs`.
3. **Event sink (review M1).** The `EventSink` the review asks for, with its fake, in `wakeshot-windows`. This track
   posts `Event::Menu` (menu) and `Event::Done(Outcome::Sound)` (player) through it. This track needs **no** hook in
   the loop's window procedure (change 2).
4. **Sound seam (review M5).** `pub trait SoundPlayer: Send + Sync { fn play(&self, cue: Cue); fn play_and_wait(&self,
   cue: Cue) -> Result<SoundEnd, WinError>; }`. Doc: "`play` never blocks; its result comes back as
   `Event::Done(Outcome::Sound)` through the event sink. `play_and_wait` blocks until the cue played out or 3 s passed;
   only the hibernate thread calls it, after its last check." `Command::Hibernate` doc: "the hibernate thread plays
   `Cue::GoingToSleep`; the core never sends `Play(GoingToSleep)`" (Phase 2 note 4 agrees). `FakeSoundPlayer`: a
   `Mutex` record (so it is `Sync`), an optional sink that posts `Done(Sound(Ok(Played)))`, a scripted error for
   `play_and_wait`; moved into `sound.rs` behind `test-support` and re-exported from `testing.rs`, its API frozen.
5. **Sound outcome.** `Outcome::Sound(Result<SoundEnd, Failure>)` with `pub enum SoundEnd { Played, Skipped }`
   (`Skipped`: the user's scheme is No sounds). If Haik answers Question 1 "always play", `Skipped` is never produced;
   no second contract round.
6. **Notifier seam.** `pub trait Notifier: Send` (it moves to the IO thread). `FakeNotifier` moved into `notify.rs`
   behind `test-support`, re-exported from `testing.rs`, API frozen.
7. **`Notice::StepBack { floor_pct: u8 }`**, so the notice can say "under 20%". Phase 2 raises it.
8. **App icon resource.** New `assets/wakeshot.ico` (placeholder: any valid ICO, for example the tray spike's ring) and,
   in both `crates/wakeshot-tray/build.rs` and `crates/wakeshot-cli/build.rs`, a `1 ICON "<path>"` line in the
   generated `.rc` plus `cargo:rerun-if-changed` for it. `p5-icon-art` owns the `.ico` afterwards.

Phase 2's new notice variants (corrupt journal, idle lock) need nothing from this track: `p5-text` words every variant
through one exhaustive match.

### What this track needs from other tracks, and its fakes (deliverable 4)

| From | What | When | Until then |
|---|---|---|---|
| Contracts card | changes 1 to 8 | before any p5 card | nothing starts before it |
| Phase 2 | `present.rs` builds real `TrayView`s from `text.rs` | `p5-haik-check` can use it; not required | demos build views by hand from `text.rs` |
| Phase 2 | D5 below: notices raised while the session is locked or off the console wait until it is back | before its notice logic is built | `p5-haik-check` tests a locked balloon |
| Phase 4 | `shell.rs` `open_with_default_app` (`p4-cli-commands`) for Open log and the first-run click. Requests to that card: accept a URI too (`ms-settings:taskbar`); call `AllowSetForegroundWindow(ASFW_ANY)` before opening, so the app opened from the tray menu comes to the front; initialize COM apartment-threaded (ShellExecuteEx docs) on its own short thread, so it never blocks the IO thread | before `p5-haik-check` | demos print the click action |
| Phase 3 | nothing at build time; the log path from `store` at integration | integration | demos use a sample log file |

What other tracks need from this one: Phase 3's hibernate thread uses `play_and_wait` (`FakeSoundPlayer` until
`p5-sounds`), and checklist 16 (chime at a real lid close) needs `p5-sounds` done. Phase 6 uses `assets/wakeshot.ico`
(placeholder until `p5-icon-art`) for the installer and `p6-pages`, and the three icon GUIDs (`p5-tray-icon` Result)
if uninstall cleans the icon's `NotifyIconSettings` entry. Integration: phase file track note 5.

### Design proposals for `architecture.md` (not edited; the orchestrator routes them)

- D1. Section 2, core modules: new row `text` | "every user-facing string: menu, status line, tooltip pieces, notices
  (`NoticeText`, `NoticeClick`); tray track". Windows `tray` row: "`TrayIcon` (its own hidden window: add, modify,
  re-add on `TaskbarCreated`, clicks, theme and display changes), `OwnerDrawnMenu` (`MenuView`), `IconIdentity`,
  theme reads, icon art drawn in code (`tray::art`)".
- D2. Section 2, "Boundary traits": the `SoundPlayer` line of change 4; after the device rule paragraph: "The chimes
  are made in code; no WAV resource."
- D3. Section 4, "Tray threads", Loop row, Does: "the loop's hidden window (events, session and power messages),
  `TrayIcon`'s hidden window (icon messages, `TaskbarCreated`, display and theme changes) and the menu's owner window;
  timers; the core; ...".
- D4. Section 5, "Tray, notices, sounds", Icon bullet: replace "The exe has a Common Controls 6 manifest (needed for
  `LoadIconMetric`) and per-monitor v2 DPI (tested)." with "Icons are drawn at run time from `tray::art` at
  `SM_CXSMICON` for the taskbar's DPI (`CreateIconIndirect`): no icon resources, no `LoadIconMetric`. The exe keeps its
  manifest (per-monitor v2 DPI, tested). The app icon is `assets/wakeshot.ico`, from the same art." Menu bullet, add:
  "Left click, right click and keyboard select open it." Notices bullet, add: "Title, body, warning, sound and click
  come from `text::notice`; a notice that comes with a chime is silent (`NIIF_NOSOUND`). A click runs the last
  balloon's action: first run opens `ms-settings:taskbar`; a failed restore or a corrupt journal opens the log."
- D5. Section 3, after "Closed-lid report": "Notices raised while this session is locked or off the console wait until
  it is unlocked or back. Balloons are not stored (research, 'Balloon'), so one shown on the lock screen may be lost.
  Change it when `p5-haik-check` shows a balloon raised while locked still reaches the user." Covers Phase 2's
  `IdleLocked`, which is raised exactly as the session locks.
- D6. Section 5, "Lock and hibernate": "(Phase 5 sets how long the thread waits for it)" becomes "with
  `SoundPlayer::play_and_wait`, at most 3 s".
- D7. Section 3, decisions table row 36: "`Notice::FirstRun` from `Started.first_run`; the balloon says how to pin the
  icon and its click opens the taskbar settings page".

### Acceptance checks

- Every item of `architecture.md` and `product.md` this track owns is in a card or named as integration: pass
  (phase file, "Coverage": icon states and art, add and re-add, theme, menu, wording, notices, first run, sounds, the
  hibernate wait, checklist 13; Open log is Phase 4's `shell.rs` plus integration; checklist 16 Phase 3 or integration
  after `p5-sounds`; toasts not in v1).
- No two cards of this track own one file at the same time, and none owns a file another track's section names: pass.
  Checked against `docs/phases/02-core.md` (`text.rs` is this track's there too), `03-windows-power.md` (it excludes
  `tray.rs`, `notify.rs`, `sound.rs`), `04-sources.md` (`shell.rs` is Phase 4's; this track owns no shell file) and
  `06-ship.md` (no icon, `assets` or `build.rs` claim). Pictures use per-card prefixes; `p5-haik-check` takes
  `art.rs`, the `.ico` and `text.rs` only after their cards are done.
- Every card that needs the real laptop or Haik is marked: pass (`p5-tray-icon`, `p5-menu-view`, `p5-notices` screen;
  `p5-sounds` sound; `p5-haik-check` screen, sound and Haik; settings switches named).
- No code written; `crates/`, `architecture.md`, `product.md` and other tracks' files unchanged: pass. The scratch
  crate lives in the scratchpad, outside the repo.

### Open items

- Question 1 for Haik (default: honor No sounds). His menu look answer. His look at the icon sheet and read of the
  text list (batched, not blocking).
- The `text.rs` shapes must be merged with `p2-plan-core`'s list in the cross-track check.
- Phase 4's `shell.rs` requests (table above) need that planner's or the orchestrator's yes.
