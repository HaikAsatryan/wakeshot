# Phase 6: Ship (track)

Planned 2026-11-03 by card `docs/tasks/p6-plan-ship.md`. Haik signs off all five track plans at once. Integration
(wiring all tracks into the two exes, end-to-end tests, Haik's checklist) is planned later, not here.

## Goal

Wakeshot installs per user from one `Setup.exe` (and later from winget), starts at login, updates itself from GitHub
Releases only while nothing holds and nothing is overridden, and leaves the user's power settings, Run values and
Path as they were when it is uninstalled. v1 ships unsigned. A README and a GitHub Pages page say what Wakeshot does
and what it changes.

## Track notes

Only what `architecture.md` leaves open for this track. Where these notes change the design, the exact proposal is
in `p6-plan-ship`, Result, for the orchestrator to route.

### Where the code goes

| File | Holds |
|---|---|
| `crates/wakeshot-windows/src/install.rs` | the CLI folder in the user Path: add, remove, the Path text edit, the `WM_SETTINGCHANGE` broadcast. The start-at-login Run value is `store::AutoStart`'s (`p3-store`), its one writer (review m12) |
| `crates/wakeshot-windows/src/hooks.rs` | what each Velopack hook does, in order and within its time, over trait `HookEnv`; `RealHookEnv` calls `install`, `store::AutoStart` and the power track's `power::journal::restore_locked` (one attempt per call; the hooks keep their own retries and limits), RunOnce and log code |
| `crates/wakeshot-windows/src/update.rs` | when to check, download and apply an update, over trait `UpdateFeed`; posts `Event::UpdateReady` and `Outcome::Update` |
| `crates/wakeshot-tray/src/velo.rs` | the only file that names the `velopack` crate: `startup()` runs `VelopackApp` with the four hook callbacks (tray startup step 1) and keeps its signature (returns nothing); a second function, `velo::feed() -> impl UpdateFeed`, gives tray mode the feed (verify review Minor 5) |
| `packaging/` | `pack.ps1`, `smoke-test.ps1`, `e2e.ps1`, `winget/` |
| `.github/workflows/release.yml`, `pages.yml`; `site/` | release build, pack, smoke test, upload; the Pages page |

Why the hook and update logic sit in the lib, not the tray: they test without `velopack` and its 93 packages (x64,
since `p6-tray-size`), and a bin crate flags every item no `main` calls yet, which would fail `clippy -D warnings`
until integration.

### Package and install values

Haik's: pack id `HaikAsatryan.Wakeshot`, data folder `%LocalAppData%\Wakeshot`. Values I set (change any at
sign-off):

- Pack title `Wakeshot`, authors `Haik Asatryan`, main exe `wakeshot-tray.exe`, `wakeshot.exe` beside it in the same
  package. Channels `win-x64` and `win-arm64`, always with `-r` (without it `vpk` packs for x86).
- Start menu shortcut only (`--shortcuts StartMenuRoot`), no desktop shortcut: the tray starts at login anyway.
- Run value `HKCU\Software\Microsoft\Windows\CurrentVersion\Run\Wakeshot`: the quoted path of
  `current\wakeshot-tray.exe` if `p6-velopack-facts` shows that `current` keeps its path across updates, else the root
  stub. The user turns start at login off in Settings > Apps > Startup; Windows keeps that choice in its own key
  (`Explorer\StartupApproved\Run\Wakeshot`), which `Setup.exe` over the install leaves off while it writes the Run
  value again (`p6-startup-switch-check`: by the keys; the off value written by script, not the real switch; sign-in
  not verified). Uninstall left that value: `p6-uninstall-startup-approval` removes it.
- Path entry: the `current` folder, so `wakeshot` works in every new terminal.
- `velopack` pinned exactly (`=1.2.161`), the same version as the `vpk` that packs it.

### Hooks

`architecture.md` section 6 says what the hooks do. Order and times (values I set):

| Hook | Velopack limit | Steps, in order |
|---|---|---|
| install (also Setup.exe over an install, which is what `winget upgrade` runs) | 30 s | write the Run value, the Path entry and the notice app id key (`HKCU\Software\Classes\AppUserModelId\velopack.HaikAsatryan.Wakeshot`: `DisplayName` and `IconUri`, the notice header's name and picture); then the restore routine if a journal exists and its owner is Gone, again every 2 s until 25 s |
| obsolete (old version, before an update) | 15 s | log only: updates apply only when nothing is overridden, and the updated hook restores anyway |
| updated (new version) | 15 s | the Path entry (no change when `current` keeps its path) and the notice app id key, so an update from a version without it gets it; then restore as at install, until 10 s |
| uninstall | 30 s | remove the Run value, its `StartupApproved` switch value, the RunOnce value, the Path entry and the notice app id key; then restore with retries until 25 s; a journal that does not parse gets the lost-journal reset instead; then the data folder (Question 2) |

- Registry first, restore last: a restore call that hangs is killed at the limit, and must not leave a Run value or a
  Path entry pointing into a deleted folder. This changes the order in `architecture.md` section 6 ("restores ...
  then removes"); proposal in `p6-plan-ship`, Result.
- An owner that is alive or Unknown at a hook: no restore, a log line (an installed tray is always killed first, so
  this is a dev build running from elsewhere).
- Hooks show no UI, never start the tray (Velopack kills every process from the install folder after each hook) and
  never panic: each failure is a log line with process `hook`.

### Updates

- The update thread runs only in tray mode (not guard, restore or dry run) and only when Velopack finds an install; a
  build run from `target\` has no updates.
- First check 5 minutes after start, so sign-in is not slowed; then every 12 hours of wall time, so a PC that slept
  past a due check checks soon after it wakes. Download at once, then `Event::UpdateReady { version }`. A failed
  check or download: a log line, tried again at the next check, no notice. Values I set.
- The core applies only when no hold counts; no hold file other than Keep awake names a live or Unknown process (so
  an update never kills a `wakeshot run` wrapper, also with Automatic off); nothing may be owned; no countdown, work
  end or hibernate is pending; and the session is not ending. An expired Keep awake file does not count: it always
  names this tray, which quits for the update (`core/update.rs`). Then `Command::ApplyUpdate`: the update thread
  calls `wait_exit_then_apply_updates(silent, restart)`, answers `Outcome::Update`, and on success the core takes the
  menu's Quit path (requests off, restore, Keep awake file deleted, `Quit`). Velopack waits at most 60 s for the exit
  (research, "Ship track planning checks"), runs the obsolete and updated hooks, and starts the new tray. A tray that
  has not quit 60 s after `wait_exit_then_apply_updates` is not waited for: Update.exe runs the obsolete hook while
  the old tray still runs, kills it (exit code 1), applies, runs the updated hook and starts the new tray (tested,
  research "Velopack checks for Wakeshot", point 4). So the quit must finish within 60 s; a tray killed with values
  still overridden is covered by the updated hook's restore.
- After an update, one notice (Question 3: yes, Haik 2026-11-03 18:09): the tray posts `Event::Updated { version }` once
  at start, the core answers `Notify(Notice::Updated { version })`, the tray track words it (`p5-text`).
- Network: the GitHub API without a token (60 requests an hour per IP); two checks a day. No other network call.

### Release

- `release.yml` on a `v*` tag: the tag must equal `[workspace.package] version`; build and test each CPU on its own
  runner, with the same checks as `ci.yml`; `pack.ps1` per CPU; `smoke-test.ps1` on each runner (silent install,
  checks, silent uninstall, checks); one upload job, the only one with `contents: write`, runs `vpk upload github`.
  Deltas come from the last release (`vpk download github` before the pack). A manual run packs and smoke-tests
  without uploading.
- A version with a pre-release part (`X.Y.Z-rc.N`) becomes a GitHub pre-release and never goes to winget. So the first
  public run (Question 5) is version `0.1.0-rc.1`, tag `v0.1.0-rc.1`. Users' update checks skip pre-releases
  (`velo.rs`, `prerelease: false`; `p6-velopack-dependency`).
- The winget job runs only when the repository variable `WINGET_SUBMIT` is set and the secret `WINGET_TOKEN` exists
  (`wingetcreate`, our files unchanged). Both are Haik's to set.
- Signing: none in v1 (decision 44). The workflow marks the two places SignPath goes later: the exes before
  `vpk pack`, `Setup.exe` after it. Applying to SignPath waits for real users (`research/distribution.md`, section 3).
- ARM64 builds need clang once `velopack` is in (research, "Ship track planning checks"). Both runner images have
  LLVM; this laptop has none, and Haik said yes to adding it (Question 4): `p6-velopack-dependency` step 0.

### Checks while other cards run

From the cross-track review (B1); the rule is `docs/plan.md` "How we work" step 2. Every card of this track checks only
its own package and files:
- format: `rustfmt --edition 2024 --check <owned files>`. A bare `rustfmt` uses the 2015 edition (the repo has no
  `rustfmt.toml`) and also checks the child modules a file declares (both checked 2026-11-03, `docs/research.md` "Track
  plan fixes checks"), so a diff in another card's child file is not this card's. Format only owned files (`rustfmt
  --edition 2024 <file>`), never a file whose child modules another card owns.
- lint: `cargo clippy --locked -p <own package> --all-targets --target <t> -- -D warnings` and the same without
  `--all-targets` (the libs as they ship, without `test-support`; `CLAUDE.md` "Checks"), for both targets: nothing in
  the owned files.
- test: `cargo test --locked -p <own package> <filter>`: the card's own tests pass. Several filters go after `--`
  (`cargo test --locked -p wakeshot-windows -- a:: b::`); two filters before `--` are a cargo error.
- Every cargo command takes `--locked`: it fails at once if a build would change `Cargo.lock`, which only
  `p6-velopack-dependency` changes, alone.
- A failure in a file another card owns goes in the Log: wait and run again; never edit, format or `--fix` that file.
  Never run `cargo fmt` without `--check`, `cargo fix` or `cargo clippy --fix`.
- Every save of an owned file compiles (`todo!()` bodies are fine): one compile error fails every `-p` run of that crate
  and of the crates that use it.
- No card runs `--workspace` or `--all` as a done check; the orchestrator runs the whole-workspace check when it closes
  a wave.
- Once `p6-velopack-dependency` is done, ARM64 builds of `wakeshot-tray` need clang (only the tray depends on
  `velopack`). Haik said yes to the Build Tools clang component (sign-off item 8, Haik 2026-11-03 18:09);
  `p6-velopack-dependency` step 0 has Haik install it, elevated. Without it the tray's ARM64 builds are CI only
  (review M5).
- Delete the card's own target folder when the card is done (review m14).

`p6-velopack-dependency` is the one exception: it runs alone, so its whole-workspace runs are right.

### Not in this track

- The winget submission (steps in `packaging/winget/README.md`), the first public release and the repo cleanup:
  Phase 8. scoop: later (an own bucket; its
  uninstall runs no Velopack hook, so it needs a `pre_uninstall` restore). Microsoft Store: Phase 8, if adding it and
  signing are free and the submission is automated (`product.md`; facts from `p6-launch-research`, card `p8-store`).
- Integration: start the update thread in tray mode and run `ApplyUpdate` and `Quit` in the loop; the update path end
  to end (a GitHub pre-release, Question 5: yes, or a local feed); checklist 14 in `architecture.md` section 9 (`winget
  upgrade` and uninstall during a real hold), which needs Haik.

## Task cards (`docs/tasks/`)

Real laptop means the card installs software or changes real user state on the test laptop. The orchestrator runs
those one at a time across all tracks.

| Card | Needs Haik | Runs after | Status |
|---|---|---|---|
| `p6-velopack-facts` | no; real laptop: installs and uninstalls a throwaway app | Haik's sign-off of the track plans; the contracts card (it builds the repo's release exes; review m10) | done (22:12): 11 facts checked; nothing left installed |
| `p6-login-and-path` | no | the contracts card; `p3-store` (test key helper) | done (committed 01:42): 21 tests; the real Path and Run unchanged by every run |
| `p6-update-schedule` | no | the contracts card | done (committed 01:42): 17 tests |
| `p6-hooks` | no (Question 2: the default, Haik 2026-11-03 18:09) | `p6-login-and-path`; `p3-store`, `p3-log`, `p3-process`, `p3-journal`, `p3-power-plan`; core `p2-ownership-restore` (review M1) | done (committed 02:05): 17 tests; real Run, RunOnce and Path unchanged |
| `p6-velopack-dependency` | yes, once: the elevated clang install of its step 0 (Question 4: yes); runs alone on the whole laptop: no other card and no cargo command (review M5) | `p6-hooks`, `p6-update-schedule`, `p6-velopack-facts` | done (committed 03:32): velopack 1.2.161; x64 checks and 1,260 tests pass; step 0 (clang) left for Haik, ARM64 tray: CI only |
| `p6-pack-and-release` | no (Question 5: yes, the first real run is a public pre-release) | `p6-velopack-facts` | done (committed 02:25): x64 and ARM64 packed for real into the scratch folder; nothing installed |
| `p6-install-e2e` | no; real laptop: installs, the real Run value and Path, a throwaway plan copy | `p6-velopack-dependency`, `p6-pack-and-release`; `p3-power-plan` (its first test shows a write to an inactive copy leaves the real plan alone) | done (committed 03:57): `packaging/e2e.ps1`, 81 of 81 checks twice at Medium; AFTER equals BEFORE |
| `p6-launch-research` | yes: reads its Result (rival pick, Store recommendation) before the README, winget and Store cards are rewritten; no installs | none | done: its Result waits for Haik's read |
| `p6-winget-manifest` | no | `p6-pack-and-release`; `p6-launch-research` (the winget rules for an app that updates itself) | done (committed 02:25): templates and `fill.ps1`; `winget validate` passed on the local packs |
| `p6-readme` | yes: a read at the Phase 6 sign-off; the answers it states (Phase 1 Questions 10 to 12, this track's 1 to 3, sign-off item 12) are in, all defaults | `p6-pack-and-release`, `p6-install-e2e`; the sources track's `p4-hold-files`, `p4-cli-args`, `p4-live-claude`; the tray track's `p5-haik-check` (review m2); `p6-launch-research` (its scope grows from that Result, Haik 2026-11-04); `p6-media` | blocked: draft done 04:22 (474 lines, 10 media spots, product words from the code as it is); waits for `p5-haik-check` and `p6-media` |
| `p6-media` | yes: watches the video and the loop and says yes or what to change; about 10 min off the PC for the captures | `p6-launch-research` (tool, storyboard, shot list); `p5-haik-check` (final icon); integration's `p6-int-real-e2e` (the real app) | blocked: Part A done (14:08: loops, the video pipeline with stand-ins, Remotion inside `media/`); Part B needs the laptop idle, captures and Haik; Haik (2026-11-04): Part B waits until his design review of the final software |
| `p6-pages` | yes: one look before it goes live at v1 | `p6-readme`; the tray track's `p5-icon-art` and `p5-haik-check` (Haik's icon pick rewrites `assets/wakeshot.ico`; review m3) | blocked: draft done (11:50); 9 marked spots wait for other cards |
| `p6-startup-switch-check` | no | `p6-install-e2e` | done (06:23: the off choice survives Setup over the install by the keys; uninstall leaves the StartupApproved value) |
| `p6-tray-size` | no (stop 2, a dependency change, waits for the orchestrator) | `p6-int-update-wiring` | done (07:01: release tray 3.95 to 2.71 MB, CLI 0.67 to 0.56 MB) |
| `p6-uninstall-startup-approval` | no (orchestrator decision for his review) | `p6-startup-switch-check` | done (06:45: 1287 workspace tests; `e2e.ps1` 88 of 88) |
| `p6-small-fixes` | no | `p6-tray-size` | done (07:08) |
| `p6-real-install-guard` | no | `p2-6-runonce-keep` | later (18:43): before Haik installs the first release; real-settings tests refuse to run beside an installed Wakeshot |

Size estimates, one worker's context: facts 100K, login and path 70K, update schedule 70K, hooks 100K, velopack
dependency 110K, pack and release 110K, install e2e 130K, winget 60K, README 70K, Pages 60K. About 880K in all.
Wave 1 of the other tracks peaked at 2 to 3 times such estimates (`docs/phases/02-06-lighter.md`).

Bundles (one worker, both cards in order; `docs/phases/02-06-lighter.md`): `p6-login-and-path` +
`p6-update-schedule`; `p6-pack-and-release` + `p6-winget-manifest`; `p6-readme` + `p6-pages`. No separate review for
this track: `p6-hooks` calls the reviewed `restore_locked`, and `p6-install-e2e` and the integration e2e cards run the
rest for real.

### Files each card owns

Every code file below exists after the contracts card (module lines, the `velo::startup()` call in the tray's
`main.rs`, the new enum variants), so no card of this track edits a module list, a `main.rs` or a shared enum.

| Card | Owns |
|---|---|
| `p6-velopack-facts` | a section in `docs/research.md` (additions only); a scratch crate outside the repo |
| `p6-login-and-path` | `crates/wakeshot-windows/src/install.rs` |
| `p6-update-schedule` | `crates/wakeshot-windows/src/update.rs` |
| `p6-hooks` | `crates/wakeshot-windows/src/hooks.rs` |
| `p6-velopack-dependency` | root `Cargo.toml`, `Cargo.lock`, `crates/wakeshot-tray/Cargo.toml` (run alone); `crates/wakeshot-tray/src/velo.rs`; `.github/workflows/ci.yml` only if the ARM64 job needs a clang step |
| `p6-pack-and-release` | `packaging/pack.ps1`, `packaging/smoke-test.ps1`, `packaging/README.md`, `.github/workflows/release.yml` |
| `p6-install-e2e` | `packaging/e2e.ps1`; fixes in `packaging/smoke-test.ps1` (after `p6-pack-and-release`) |
| `p6-launch-research` | `docs/research/launch.md`; additions to `docs/research.md` |
| `p6-winget-manifest` | `packaging/winget/` |
| `p6-readme` | `README.md` |
| `p6-media` | `media/` (outside the Cargo workspace); additions to `docs/research.md` |
| `p6-pages` | `site/`, `.github/workflows/pages.yml` |

### Order inside the track

1. After Haik's sign-off and the contracts card: `p6-velopack-facts` (real laptop, no power settings),
   `p6-update-schedule`; `p6-login-and-path` once `p3-store` is done. In parallel with each other and with any other
   track's card.
2. After the facts: `p6-pack-and-release` (it packs into the scratch folder, never installs).
3. When `p3-store`, `p3-log`, `p3-process`, `p3-journal`, `p3-power-plan` and the core's `p2-ownership-restore` are
   done: `p6-hooks`.
4. Haik said yes to Question 4: `p6-velopack-dependency` step 0 has him install the clang component, elevated, with
   no cargo command running (review M5), before its first edit.
5. `p6-velopack-dependency`, alone on the whole laptop (no other card, no cargo command), at a quiet point: from then
   on every build of the tray compiles `velopack` once per target folder, and local ARM64 checks of the tray need
   clang.
6. `p6-install-e2e` (real laptop; never beside a card that uses the dry-run data folder), then `p6-winget-manifest`
   (it can also run right after step 2).
7. Last: `p6-media` (once the real app works end to end), then `p6-readme`, then `p6-pages`.
   `p6-launch-research` runs any time before them (no build, no installs) and its Result goes to Haik first.

## Questions for Haik

Answered 2026-11-03 18:09, all with the defaults (`docs/phases/02-06-signoff.md` items 5 to 9 and 12): (1) Wakeshot
is off after `winget upgrade` until the next sign-in, and the README says so; (2) the data folder at uninstall is
deleted after a good restore, and after a failed one the journal and logs stay; (3) yes, a notice after an update:
`p1-close-contracts` added `Event::Updated` and `Notice::Updated` (review m9); (4) yes, the Build Tools clang
component on this laptop (`p6-velopack-dependency` step 0); (5) yes, a public pre-release tests the release workflow,
and Phase 8 deletes it. Item 12: the uninstall leaves `NotifyIconSettings`, and the README says so.

## Work log

- 2026-11-03 16:59: track planned by `p6-plan-ship`: 10 cards, about 880K tokens. Waits for Haik's sign-off of the
  five track plans and the contracts card.
- 2026-11-03: Haik signed off the five track plans with every default (18:09); `p1-close-contracts` wrote the answers
  into this file and the cards, added `Event::Updated` and `Notice::Updated`, gave the clang install an owner
  (`p6-velopack-dependency` step 0) and settled `velo::feed()` (verify review Minor 5).
- 2026-11-03 22:16: `p6-velopack-facts` done and committed. For `p6-velopack-dependency`: its Result recommends
  `set_auto_apply_on_startup(false)` in `velo.rs` and a 10 s inner limit for the updated hook (Velopack gives 15 s).
  Open, for a docs pass: `architecture.md` section 6 still calls two now-checked facts unverified; this file's Updates
  part should say Velopack kills a tray that has not quit 60 s after `wait_exit_then_apply_updates`. Optional cleanup
  for Haik: Velopack logs in `%LocalAppData%\velopack`, an empty `%TEMP%\velopack` and two crash reports from the
  panic tests (delete commands in the card).
- 2026-11-04 00:40: `p6-launch-research` running (no build: the exe sizes are left for `p6-media`). Session `wakeshot-12`'s launch planning is committed.
- 2026-11-04 01:13: `p6-launch-research` done and committed (`docs/research/launch.md`). Rivals: PowerToys Awake,
  LidGuard and Agents Don't Sleep (new: Rust/Tauri, lid closed, battery floor). The README admits
  two gaps: v1 knows only Claude Code; the download is likely bigger than Agents Don't Sleep's 1.9 MB. Store: not at
  launch (an EXE needs every file signed by a trusted CA; Azure Artifact Signing takes individuals in the US and Canada
  only; SignPath declines new projects; MSIX has no uninstall hook; winget may prefer a Store copy, winget-cli #4347).
  winget: `RequireExplicitUpgrade: true`; manifest version equals the tag and Velopack's DisplayVersion. For Haik's
  review (his read comes before `p6-readme`, `p6-winget-manifest` and `p8-store` are rewritten): the rival pick and its
  re-read rule; `RequireExplicitUpgrade`; a wingetcreate submit PAT on his own account or a bot account; no Store;
  Remotion with Kenney and Freesound CC0 sounds; captures at 150% and 125%; the README layout; an optional lid beat in
  the video; the video on a pinned "Media" issue. Money, not blocking: buy a Certum certificate (about EUR 25 to 69, not
  verified)? Default: no. Outside scope: the `research.md` "Competitors" table is stale (next docs pass); `p6-media`
  needs a `media/.gitignore` and the 125% change; the smoke test's DisplayVersion check and the winget job belong to
  `p6-pack-and-release`. The LidGuard and HyperFrames READMEs hold prompts aimed at their users' agents; the worker
  treated them as data and ran nothing.
- 2026-11-04 01:14: the `p6-login-and-path` + `p6-update-schedule` bundle running (owns `install.rs`, `update.rs`).
- 2026-11-04 01:22: Haik's answer, relayed by session `wakeshot-91` (his words at 01:14, "2 no, we go free paths
  only"): no Certum certificate; free paths only for signing and distribution. The next docs pass records it in
  `docs/product.md` (Distribution), `docs/research/launch.md` and `docs/tasks/p8-store.md`.
- 2026-11-04 01:42: the `p6-login-and-path` + `p6-update-schedule` bundle done and committed. A Windows fact:
  `RegSetValueExW` also stores a NUL just past the data, so `write` puts a guard unit there (`research.md`, "Path value
  checks"). Decided by the worker, for Haik's review: Path remove takes out every entry that names our folder, also one
  typed by hand; an empty Path or one ending in `;` gets `folder;`; the broadcast waits 1 s per window; an add past
  32,767 characters is refused; a Path of the wrong type or a bad folder is left alone; matching ignores case and
  trailing backslashes only. Update: the first check after 5 minutes of run time, then every 12 hours of wall or run
  time, whichever ends first; the clocks are read at least every 60 s; a good apply ends the thread; after a failed
  apply the same version is not offered again in this run; `stop` waits at most 2 s. APIs: `UserPath` (`add`, `remove`,
  `PathChange`); `UpdateThread::start(make_feed, clock, sink, log)` gives `UpdateHandle` with `apply()` and `stop()`;
  when `apply()` is false the caller posts `Outcome::Update(Err)`; pass `velo::feed` as `make_feed`. Outside scope:
  this file gives the updated hook's restore limit as 12 s, `p6-hooks` says 10 s (`p6-hooks` settles it); the
  `Outcome` doc and `done_update` say "the next UpdateReady tries again", but `UpdateReady` comes at most once per
  version. Optional: `pub use crate::update::FakeUpdateFeed;` in `testing.rs`. `p6-hooks` running.
- 2026-11-04 02:02: the `p6-pack-and-release` + `p6-winget-manifest` bundle running (packs x64 into its scratch folder only; never a tag, an upload or a winget submit).
- 2026-11-04 02:05 (orchestrator, session `wakeshot-46` after the handover from `wakeshot-9d` at 02:03): `p6-hooks` done
  and committed. The updated hook's restore limit is 10 s (matches `p6-velopack-facts` and architecture.md section 5
  step 7); the Hooks table above now says 10 s. Decided by the worker, for Haik's review (card Result): an owner in
  state Unknown gets no restore; a journal.json that does not parse stays in place; an owner still running at
  uninstall keeps journal and logs; the updated hook never writes the Run value. Next for `p6-velopack-dependency`:
  the four `velo.rs` callbacks (exact lines in the `p6-hooks` card). Docs pass round 3 gains: architecture.md
  section 6 (old uninstall order) and section 5 ("the hooks are not built yet").
- 2026-11-04 02:25 (orchestrator, session `wakeshot-46`): the `p6-pack-and-release` + `p6-winget-manifest` bundle done
  and committed. Packed for real: x64 `Setup.exe` 10.5 MB (full package 3.0 MB), ARM64 8.8 MB (2.9 MB); a 0.1.1 delta
  against 0.1.0 is 9 KB. The smoke test ran only against a harmless stand-in: nothing installed. `winget validate`
  passed on the filled folder and failed on a broken copy. Not run: `actionlint` (auto mode blocked its download;
  Haik's item: he runs it or allows it). The "Release" notes above gain the pre-release rule and the winget switch.
  Decided by the worker, for Haik's review: `X.Y.Z-rc.N` is a GitHub pre-release and never goes to winget; no cache in
  the release workflow; release files upload before the smoke test, so a failed test keeps the packs; `wingetcreate`
  over Komac; an InstallationNotes line; no MinimumOSVersion; the repo is the homepage until Pages is live; winget
  description and tags are drafts. Question for Haik, not blocking: whose account holds the winget token (default:
  his own). Before the first tag: `actionlint` on `release.yml`; push, then the manual dry run from Actions (its smoke
  test fails until `p6-velopack-dependency` wires `VelopackApp`); then version `0.1.0-rc.1`, `cargo update
  --workspace`, commit, tag `v0.1.0-rc.1`, push.
- 2026-11-04 02:30 (orchestrator, session `wakeshot-46`): orchestrator decision, for Haik's review:
  `p6-velopack-dependency` runs without step 0 while Haik is away, alone on the whole laptop as its header says, as
  soon as the running cards end. Its own fallback applies: ARM64 builds of `wakeshot-tray` are checked by CI only (the
  runner image has LLVM) until Haik installs clang; step 0 stays his, and every later card records "ARM64 tray: CI
  only" until then. Why: it unblocks `p6-int-update-wiring` and `p6-install-e2e` overnight.
- 2026-11-04 02:39 (orchestrator, session `wakeshot-46`): CI green on 8ff7fa5 (Linux core, x64, ARM64). Manual
  dry run of `release.yml` (run 41731897399): build, pack, silent install and uninstall pass on both CPUs; only the 3
  hook checks fail, as expected until `p6-velopack-dependency` (details in the `p6-pack-and-release` Log).
- 2026-11-04 03:32 (orchestrator, session `wakeshot-46`): `p6-velopack-dependency` done and committed, run alone.
  velopack 1.2.161 (114 packages besides itself on x64, 117 on ARM64; `Cargo.lock` 80 to 181 packages, no locked
  version changed). `velo.rs`: `startup()` turns auto apply off and runs the four hook callbacks; `feed()` uses
  `GithubSource` with `prerelease: false`. x64: fmt, clippy both forms, 1,260 workspace tests, release build, CRT check;
  tray exe 1.64 MB to 2.03 MB; the dry-run smoke test still passes. ARM64: core, windows and cli pass clippy and
  release (no velopack or `ring` under them); the tray fails locally for want of clang (`ring`), so CI only. Step 0 for
  Haik (exact steps in the card): one elevated `setup.exe modify ... --add
  Microsoft.VisualStudio.Component.VC.Llvm.Clang`, then vswhere and `clang --version`. Decided by the worker, for
  Haik's review: no auto apply at startup; no offline fallback to a downloaded package; the feed keeps the last
  downloaded update; no pre-release switch in `feed()`; his user Path is not changed by the clang step. Next:
  `p6-int-update-wiring`, then the release dry run again.
- 2026-11-04 03:39 (orchestrator, session `wakeshot-46`): CI green on cd3d498. The release dry run on it (run
  41739978975) passed on both CPUs, all smoke checks included: the real hooks run through Velopack on the runners.
  Left before the first tag: `actionlint` (Haik), then version `0.1.0-rc.1` and its tag (Question 5).
- 2026-11-04 03:57 (orchestrator, session `wakeshot-46`): `p6-install-e2e` done and committed. `packaging/e2e.ps1`
  ran the whole card twice at Medium (03:48 and 03:52), 81 of 81 checks each: silent install, Setup over the install
  (the install hook put the plan copy's lid value back and removed the journal), the update hooks (restored; `current`
  kept its path), uninstall during a hold (restored; Run, RunOnce, Path entry, data and install folders, uninstall key,
  shortcut and temp folder gone). AFTER equals BEFORE by the script and by an elevated read: Run (7 values) and RunOnce
  bytes, Path (REG_EXPAND_SZ, same bytes), Balanced the only and active plan, lid 1/1, sleep 18000/18000, energy saver
  DC 30, the full `powercfg /qh` text. The hooks wrote only the never-active copy. Sizes: `Setup.exe` 11.3 MB, full
  package 3.8 MB, delta 9 KB; install 3.8 s, update 1 to 1.5 s, uninstall 0.4 s. Packs built from cd3d498, so the
  update wiring is not covered: run `e2e.ps1` again after `p6-int-wiring-fixes` (build, then about 40 s). Decided by
  the worker, for Haik's review: the script packs both versions itself; it refuses to run elevated unless
  `-AllowElevated`; its end puts BEFORE values back if they differ (never fired); the log shows Run and Path only as
  short hashes (the card is public); the journal owner is an ended `PING.EXE` it started. Small items for the rerun:
  `pack.ps1` help needs a blank line under `#Requires`; `packaging/README.md` should list `e2e.ps1`. Velopack's own
  logs stay in `%LocalAppData%\velopack` (deleting them is optional, Haik's call). The worker's note that `CLAUDE.md`
  still says 3 h and 4 h is wrong: it says 5 h (checked 03:57).
- 2026-11-04 04:02 (orchestrator `wakeshot-12`, decision for Haik's review): `p6-readme` started as a draft before
  its Runs after `p5-haik-check` and `p6-media` are done (both need Haik). Why: it needs no laptop, the night's laptop
  lane is serial, and everything else it states is done. The draft takes the product text from the code as it is
  and leaves one marked comment per media spot; its Result lists the lines to check after Haik's pick and the media.
- 2026-11-04 04:22: `p6-readme` draft done (worker, 04:22): `README.md` rewritten whole; links 23 of 23 and external
  links 19 of 19 good, help block equal to `args.rs`, top 13 lines. Left: speed and media (`p6-media`), product words
  (`p5-haik-check`), SmartScreen wording (first pre-release). 12 worker decisions for Haik's review and 6 found
  outside scope (stale doc lines in `architecture.md` section 5 and `product.md` Distribution; 30 s against 25 s in
  Phase 1 Question 11) in the card; the doc lines go to the next docs pass.
- 2026-11-04 04:30: `p6-install-e2e` rerun started (a follow-up worker, alone on the laptop): `packaging/e2e.ps1` on a
  release x64 build of 7b6d076, so the update wiring and the wiring fixes are covered; plus the `pack.ps1` help blank
  line and `e2e.ps1` in `packaging/README.md`. Recorded in its card under "Rerun (2026-11-04)". Pushed 7b6d076 at 04:30.
- 2026-11-04 04:37: `p6-install-e2e` rerun done: 81 of 81 checks on a release x64 build of 7b6d076, AFTER equals BEFORE
  (each value, plus the worker's own elevated read), all 15 PIDs gone. The tray is now 3.95 MB (the HTTPS stack), so
  `Setup.exe` is 12.4 MB; first install 7.7 s (was 3.8 s; the hook's own timer says 0.1 s; cause not verified). `pack.ps1`
  help shows in `Get-Help`; `packaging/README.md` lists `e2e.ps1`. 3 worker decisions for Haik's review in the card.
  The worker's note that `CLAUDE.md` still gives 3 h and 4 h: the file says 5 h (line 48, since 01:39), but the copy
  injected into new sessions still shows the old line (seen by `wakeshot-12` at 04:00; cause not verified).
- 2026-11-04 06:12: docs pass round 4 done (`p2-6-docs-pass`): `architecture.md` sections 3 to 6 and 9 match tonight's
  cards (one line states the unread-journal lock guess and marks it open for Haik); `product.md` pack id tested;
  fixtures renamed `user_values` in 11 files (names and comments only); `claude-code-signals.md` closing note says
  `procStart`. Workspace tests and both clippy forms pass. Open from it: start at login after Setup over the install
  (not verified), SmartScreen text, the unlocked-session checks, the tray exe at 3.95 MB (was 2.0).
- 2026-11-04 06:13 (orchestrator `wakeshot-12`): two new cards from docs pass round 4 and the `p6-readme` draft:
  `p6-startup-switch-check` (does start at login stay off after `Setup.exe` over the install, and what uninstall
  leaves of the switch; alone on the laptop) and `p6-tray-size` (why the release tray grew from 2.0 to 3.95 MB;
  stop 1 read only, a dependency change only after the orchestrator's go).
- 2026-11-04 06:23: `p6-startup-switch-check` done (one install cycle at Medium, 06:19): with the off value written
  (`03` and a FILETIME; community sources, no Microsoft doc; not the real switch, the session was locked), `Setup.exe`
  over the install wrote Run again and left the off value as it was. Uninstall removed Run but left
  `StartupApproved\Run\Wakeshot` (deleted by the worker; AFTER equals BEFORE). 24 of 25 checks; the fail was the
  worker's own check (a write of the same data does not move the key time). Orchestrator decision, for Haik's
  review: uninstall removes that value too (an uninstall leaves nothing; a reinstall starts as on a new PC):
  new card `p6-uninstall-startup-approval`.
- 2026-11-04 06:24 (orchestrator `wakeshot-12`): `p6-uninstall-startup-approval` started (store `AutoStart`, the
  uninstall hook, `e2e.ps1` step 7, one `e2e.ps1` run alone on the laptop), beside `p6-tray-size` stop 1.
- 2026-11-04 06:35: `p6-tray-size` stop 1 done (read only): the 1.9 MB come from velopack's own dependencies, not its
  features (`file-logging`, `public-utils`, both off): `ureq` with default features (rustls, ring, the root list,
  about 1.09 MB), the HTTP client (about 213 KB), `url` and through `idna` ICU (about 133 KB). Measured: 3,953,152
  bytes now; `opt-level = "z"` 2,827,776; the IDNA stub (`idna_adapter = "=1.0.0"`) 3,820,544; both 2,705,920
  (32 % smaller), nothing the app uses lost. Orchestrator decision, for Haik's review: go for stop 2 with both,
  plus one release smoke run, alone among builders after `p6-uninstall-startup-approval`. For Haik: a stronger
  option, Windows' own TLS (SChannel) through a patched copy of velopack kept in the repo: 1,865,728 bytes, and
  the ARM64 tray builds without clang (checked by the worker; an HTTPS check against GitHub passed). It means
  keeping a patched dependency; his call.
- 2026-11-04 06:45: `p6-uninstall-startup-approval` done: `AutoStart::remove_approval` deletes
  `Explorer\StartupApproved\Run\Wakeshot` (missing is not an error); only `on_uninstall` calls it, right after the
  Run value (a fake and a real test show install and update never touch it). `e2e.ps1` seeds the off form before
  the uninstall: 88 of 88 at Medium, the laptop as before. 5 worker decisions for Haik's review in the card. Hooks
  table uninstall row updated here. Left for the next docs pass: `architecture.md` section 6 "Registry values
  Wakeshot writes" and the `store.rs` module doc could name the switch value; the README registry and "What stays"
  lists wait for the README finish.
- 2026-11-04 07:01: `p6-tray-size` done: `opt-level = "z"` and the `idna_adapter = "=1.0.0"` pin (root workspace
  dependency; the tray's `Cargo.toml` carries it, since only a dependency takes effect). One resolve, no `cargo
  update`: 181 to 160 lock packages (only `idna_adapter` and 21 ICU crates). Release x64: tray 3,954,176 to 2,706,432
  bytes, CLI 668,672 to 557,056. fmt, clippy (x64 both forms, ARM64 core, windows, cli), 1287 tests, release build,
  CRT check, debug smoke pass; the tray's ARM64 in CI only. Worker slip: an extra `cargo test --release` ran the CLI
  `run.rs` tests against the real `%LocalAppData%\Wakeshot` (a release exe ignores `WAKESHOT_DATA_DIR`); the
  folder did not exist before tonight's runs (install cards' BEFORE), so the worker copied it to scratch and deleted
  it; no tray ran, no power setting touched. Left: that test's release guard, the `architecture.md` section 11
  dependency rows, the velopack package counts in phase files (113 to 93), panic paths in local release builds
  (CI builds the shipped exes; `trim-paths` still unstable).
- 2026-11-04 07:02 (orchestrator `wakeshot-12`): new card `p6-small-fixes` (a release guard for the CLI run tests,
  one shared `taskbar_dpi`, three doc lines); line 29's velopack package count set to 93 here.
- 2026-11-04 07:08: `p6-small-fixes` done: the CLI `run.rs` tests skip in a release build (no other test file lacked
  the guard); one `taskbar_dpi` (`tray/icon.rs`, used by `notify.rs`); `store.rs` module doc and `architecture.md`
  sections 6 and 11 (the switch value, `idna_adapter`, 160 lock packages). fmt, clippy, debug workspace tests pass.
  2 worker decisions for Haik's review in the card.
- 2026-11-04 07:16: CI green on 2bcfd21 (all three jobs). Release dry run on 2bcfd21 (run 41765357171, by hand: packs
  and smoke-tests, uploads nothing): Version, x64 and ARM64 build, pack and smoke test passed; publish and winget
  skipped as a dry run should. The size change packs on both CPUs.
- 2026-11-04 09:56 (orchestrator `wakeshot-12`): Haik's answers to the open calls (his words in quotes): the update
  check keeps rustls with `ring` ("do as you think will be perfect"; option A of `p6-tray-size`): no patched
  velopack, no clang install, the ARM64 tray stays CI only. Change it when a user reports a failed update behind a
  company network, or velopack ships a TLS option. An upstream issue asking velopack for a native TLS feature is
  drafted; it is posted only after Haik reads the text. The winget token is on his own account ("My account is
  ok"): a classic token with `public_repo`, set at Phase 8 (orchestrator default: a 1-year expiry). Velopack's
  test logs in `%LocalAppData%\velopack` go (his "ok"; auto mode refused the delete to the orchestrator, so Haik
  runs it). The icon pick moves to a separate branding session of Haik's (brand colors first, then the icon states).
- 2026-11-04 10:13 (orchestrator `wakeshot-12`): Haik deleted `%LocalAppData%\velopack` and ran actionlint 1.7.12 on
  `release.yml` himself (auto mode refused both to the orchestrator): no findings. The velopack issue is open as
  velopack/velopack#1146 (an opt-in native TLS feature; #76, the system proxy issue, already names the trust store
  side). `p6-int-haik-checks` gains two steps when `p2-6-sleep-route` is done: the lid test of the sleep route,
  and Haik turns off Wakeshot under Settings > Apps > Startup, then signs out and in once (the real switch).
- 2026-11-04 11:33 (orchestrator `wakeshot-12`, decision for Haik's review): `p6-pages` started as a draft before
  its Runs after (`p6-readme` final, `p5-haik-check`) are done, as `p6-readme` was: the brand is final
  (`docs/brand.md`), so the page is built now with marked spots for media, menu words, release and winget lines,
  and the sleep wording (checked after `p2-6-sleep-route`). Running beside it: `p2-6-sleep-route`, docs pass round 5,
  `p5-haik-check` Part A (built on an export of f47391d, since the tree does not compile mid-change).
- 2026-11-04 11:50: `p6-pages` draft done: one page in the brand colours, light and dark, no scripts or trackers;
  `site/` (`index.html`, `style.css`, `wakeshot.svg` drawn from `art.rs`, `favicon.ico`, `check_page.py`) and
  `pages.yml` (after a finished Release run for stable tags, or by hand; not on `release: published`, since a
  `GITHUB_TOKEN` event starts no workflow; its `--final` check fails until the 9 spots are filled). actionlint 0
  errors; WCAG AA for every text colour pair. 10 worker decisions for Haik's review. Waits: `p6-media`,
  `p5-haik-check` words, `p2-6-sleep-route` wording, README comparison, SmartScreen words, winget line.
- 2026-11-04 11:52: `p6-readme` round 2 done (Haik's ask: compare only with very popular tools, on features): PowerToys
  Awake (140,521 stars), Caffeine (367,663 Chocolatey downloads), Don't Sleep (38,324 plus 20,340 portable); 13 rows,
  the agent and lid rows in bold; LidGuard and Agents Don't Sleep left the README. Lines wrong by
  `product.md` fixed (sleep, not hibernate; the lost journal; notice titles), two marked to check after
  `p2-6-sleep-route`. 10 worker decisions for Haik's review; Haik reads the table. `product.md` line 3 status updated.
- 2026-11-04 13:38 (orchestrator `wakeshot-12`, decision for Haik's review): `p6-media` Part A started before
  `p5-haik-check`: the icon loop, the video pipeline with stand-in screen pictures, `media/README.md` and
  `CREDITS.md`, the speed-number script. Waits: speed numbers (installed release, idle laptop), final captures
  (screen slot), the Claude Code session captures, Haik's look. `p6-int-haik-checks` got its 2026-11-04 changes
  (sleep, not hibernate; the dim timeout; a Startup switch block with one sign-out) and now runs after
  `p2-6-sleep-route-fixes`.
- 2026-11-04 14:08: `p6-media` Part A done: `media/` (Remotion 4.0.540 installed inside it only): the Sunrise ported
  from `art.rs` (0 pixel difference), icon loops light and dark (5 s GIFs), the 30 s video pipeline on the storyboard
  with stand-in pictures marked DRAFT (the final render refuses stand-ins and missing numbers), sounds made in code
  (Wakeshot's chimes from `chime.rs`), `README.md`, `CREDITS.md`, `measure-speed.ps1` (not run). 10 worker decisions
  for Haik's review. Part B: speed numbers (from a plain PowerShell window, no busy Claude Code session), final
  captures at 150% and 125%, a real Claude Code session, Haik's look.
