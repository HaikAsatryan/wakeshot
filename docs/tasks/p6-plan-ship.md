# Plan the Ship track (Phase 6)

Phase: 6. Status: done. Needs Haik: yes, at the end: he signs off all five track plans at once (the orchestrator
relays one summary). Agent: `wakeshot-worker`. Size: about 120K tokens.
Runs after: `p1-design-verify-fixes` and the final design check (`docs/reviews/p1-design-final-check.md`), and
`p1-contracts` (the module layout and types in `crates/`). Parallel with: the other four track plan cards
(`p2-plan-core`, `p3-plan-windows-power`, `p4-plan-sources`, `p5-plan-tray-ux`, `p6-plan-ship`; each owns only its
own phase file and card prefix) and the Phase 1 review (read only).

## Goal

Phases 2 to 6 run as parallel tracks (`docs/plan.md`, the note under the Phases table). Plan this track: Velopack installer and updates, start at login, uninstall restore, signing, winget manifest, README, GitHub Pages. Integration (wiring the tracks into the two exes) is planned later, not here.
Split it into task cards that can be built at the same time as the other four tracks with zero conflicts and no
lost work, test-first, against the contracts in `crates/`.

## Read first

- `CLAUDE.md`; `docs/plan.md`: "How we work" (the parallel and Git rules) and the note under the Phases table.
- `docs/architecture.md`: section 10 (CI and build targets), the install, update and uninstall hooks wherever the design names them, section 11 (dependencies: `velopack`); also `docs/research/distribution.md`. Skim the rest for anything your track owns.
- `docs/product.md`: the lines your track serves.
- `docs/research.md`: only the sections the design cites for your track.
- `crates/`: the module layout and contract types written by `p1-contracts` (its card Result says what is where).
- `docs/phases/01-design-skeleton.md`: "Values I set" and the answered questions. Questions 10 to 12 are open and
  Haik's: plan around both answers or mark the card that depends on them; do not decide them.

## Deliverables

1. `docs/phases/06-ship.md`: goal, short track notes (only what `architecture.md` leaves open for this track),
   the card table (Card, Needs Haik, Runs after, Status), the order inside the track, and a work log section.
   `architecture.md` stays the design: any design change you need goes in your card's Result as a proposal, with the
   section and the new text.
2. Task cards `docs/tasks/p6-*.md` from `docs/tasks/_template.md`. Each card:
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

Each with the worker's default. None blocks the plan; the cards take the default until he answers.

1. **Wakeshot is off after `winget upgrade` until the next sign-in or a manual start.** winget runs Velopack's
   `Setup.exe --silent`, which does not start the app, and a hook cannot start it (Velopack kills every process from
   the install folder after each hook). Agent work in that window gets no hold, so the PC may sleep. Wakeshot's own
   updates do restart it; this is only the winget path. Default: accept; the README says so. Other: the install hook
   leaves a delayed starter outside the install folder (not tested, fragile).
2. **The data folder at uninstall** (`%LocalAppData%\Wakeshot`: settings, hold files, logs, journal). Default: after a
   successful restore, delete all of it; after a failed restore, keep only `journal.json` and the logs, so a reinstall
   restores and the log has the old values. Other: keep the folder, so Automatic survives a reinstall.
3. **A notice after Wakeshot updated itself** ("Wakeshot updated to 1.1.0"), no sound. Default: yes, by decision 0 (a
   notice for every action Wakeshot takes on its own). It adds one notice type and one event. Other: the icon only.
4. **Clang on this laptop.** `velopack` brings `ring`, which needs clang to build for ARM64 (research, "Ship track
   planning checks"); this laptop has none. After `p6-velopack-dependency`, every local ARM64 check of every track
   fails without it. Default: before that card, the orchestrator's elevated session adds the Build Tools component
   "C++ Clang Compiler for Windows" (`Microsoft.VisualStudio.Component.VC.Llvm.Clang`; that this ID and its size are
   the same in Build Tools 2026: not verified), and the card sets clang's folder for its builds without changing your
   PATH. Other: install nothing; local ARM64 checks skip the tray, and CI covers ARM64.
5. **A public test release.** The release workflow's first real run publishes a GitHub pre-release (for example
   `v0.1.0-rc.1`) with installers of an unfinished build; the orchestrator pushes the tag after the Phase 6 review
   push. Velopack's update check skips pre-releases. Default: yes; Phase 8 deletes the test releases and tags. Other:
   only the manual dry run (no release) until v1, so the upload step is first tried at v1.

Phase 1 Questions 10 to 12 touch only `p6-readme`, which states the answers; it waits for them.

## Found outside scope

Problems seen but not fixed.

- `docs/phases/02-core.md` says the hooks call `ownership::restore`. In this plan they call the power track's
  `restore_from_journal()` (`p3-restore`), which uses it; the order of cards is the same either way. No change needed
  unless the core track meant a direct call.
- Phase 8's cleanup removes internal docs; `docs/hold-files.md` (sources track) is a user doc the README links to,
  so it must stay.
- Every card of every track that builds the tray after `p6-velopack-dependency` compiles `velopack`'s 113 packages
  once per target folder (a few minutes each, not measured). The orchestrator may want that card late, when most
  tray builds are done.

## Log

- 2026-11-03 16:27: card written by the orchestrator (Phases 2 to 6 as parallel tracks, Haik's ask).
- 2026-11-03 16:39: worker started. Reading CLAUDE.md, plan.md, architecture.md, distribution.md, crates/.
- 2026-11-03 16:48: read done. Checked (for research.md): `ring` needs clang for ARM64 and this laptop has none;
  both runner images have LLVM and .NET 10; velopack `UpdateManager` API and its 60 s exit wait. Writing the phase
  file and cards next.
- 2026-11-03 16:53: orchestrator note (contracts domain review blocker): one contracts card, run alone, declares every
  module and splits shared files by owner before any build. So no ship card edits `main.rs`, `lib.rs` or a shared
  enum: the contracts card adds the ship modules, the `velo::startup()` call in the tray's `main.rs` and the new
  variants; ship cards own whole files only. The velopack card still edits the Cargo files, run alone.
- 2026-11-03 16:59: phase file and 10 cards written; power track card names (`p3-store`, `p3-log`, `p3-process`,
  `p3-restore`, `p3-power-plan`) taken from `docs/phases/03-windows-power.md`. The tray track's phase file did not
  exist yet, so its file list is not compared.
- 2026-11-03 17:00: acceptance checks run (Result, "Checks"); no em dash or non-ASCII in the new files. Status done.
- 2026-11-03 18:03 (`p1-track-plan-fixes`): history above is kept as written. `p3-restore`, `p3-apply` and
  `p3-power-props` were dropped at 17:17; read `p3-restore` and `restore_from_journal` as `p3-journal`
  (`power::journal::restore_locked`) plus the core's `p2-ownership-restore`. `PowerApi` and `FakePower` are gone
  (`p1-contracts-split`): `FakePlans` is the one plan fake. The Run value's one writer is `store::AutoStart`
  (cross-track review m12). The cards and phase files carry the current text.

## Result

Done 2026-11-03. The ship track is planned in 10 cards, about 880K tokens of worker context in all. No code written.

### Files

- New: `docs/phases/06-ship.md` (goal, track notes, card table, file owners, order, work log).
- New cards: `docs/tasks/p6-velopack-facts.md`, `p6-login-and-path.md`, `p6-update-schedule.md`, `p6-hooks.md`,
  `p6-velopack-dependency.md`, `p6-pack-and-release.md`, `p6-install-e2e.md`, `p6-winget-manifest.md`,
  `p6-readme.md`, `p6-pages.md`.
- `docs/research.md`: new section "Ship track planning checks" (`ring` needs clang for ARM64, this laptop has none,
  both runner images have LLVM, the `velopack` manifest, the `UpdateManager` API and its 60 s exit wait, `vpk` and
  `winget` versions here).
- This card.

### Cards

| Card | Size | Needs Haik | Real laptop |
|---|---|---|---|
| `p6-velopack-facts` | 100K | no | yes: installs a throwaway app |
| `p6-login-and-path` | 70K | no | no |
| `p6-update-schedule` | 70K | no | no |
| `p6-hooks` | 100K | no (takes Question 2's answer) | no |
| `p6-velopack-dependency` | 110K | Question 4 answered first; runs alone for the Cargo files | no |
| `p6-pack-and-release` | 110K | Question 5 before the first real run | no |
| `p6-install-e2e` | 130K | no | yes: installs, the real Run value and Path, a plan copy |
| `p6-winget-manifest` | 60K | no | no |
| `p6-readme` | 70K | yes: the answers it states; a read at sign-off | no |
| `p6-pages` | 60K | yes: one look before v1 | no |

### Contract changes for the contracts card (run alone, before any build)

1. `crates/wakeshot-core/src/event.rs`: new variant `Event::UpdateReady { version: String }`. Doc: owner ship track
   (Phase 6) posts it from the update thread once an update is downloaded, at most once per version per run; the core
   track decides when to apply.
2. `crates/wakeshot-core/src/command.rs`: new variant `Command::ApplyUpdate`. Doc: the core issues it only when no
   hold counts, no hold file names a live process (any kind, also with Automatic off, so an update never kills a
   `wakeshot run` wrapper) and no key is owned; the ship track's update thread runs it.
3. `crates/wakeshot-core/src/command.rs`: new variant `Outcome::Update(Result<(), Failure>)`. Doc: `Ok` means
   Velopack's updater now waits (at most 60 s) for this process to exit, so the core issues `Quit`; `Err` means
   nothing changed and the next `UpdateReady` tries again.
4. Only if Haik says yes to Question 3: `Event::Updated { version: String }` (posted once at start when Velopack
   restarted the tray after an update) and `Notice::Updated { version: String }` in `crates/wakeshot-core/src/view.rs`
   (the core answers with `Notify`; the tray track words it).
5. Module files, each with only a module doc comment, so every ship card owns a whole file:
   - `crates/wakeshot-windows/src/install.rs`, `hooks.rs`, `update.rs`, with `pub mod install;`, `pub mod hooks;`,
     `pub mod update;` in `crates/wakeshot-windows/src/lib.rs`;
   - `crates/wakeshot-tray/src/velo.rs` with `pub fn startup() {}` (doc: tray startup step 1; Velopack runs a hook
     and exits the process before anything else starts), and in `crates/wakeshot-tray/src/main.rs` `mod velo;` plus
     `velo::startup();` as the first statement of `main`. From then on `p6-velopack-dependency` owns `velo.rs` and
     keeps that signature; what `velo.rs` adds for integration (the update feed, the restarted version) carries
     `#[expect(dead_code, reason = "integration starts the update thread")]`, which fails clippy once integration uses
     it, so it cannot be forgotten.
6. No new `windows` features: the registry calls and `SendMessageTimeoutW` sit under the declared
   `Win32_System_Registry` and `Win32_UI_WindowsAndMessaging` (from the `windows` crate's module layout; not compiled
   yet).
7. `velopack = "=1.2.161"` is not in the contracts card: `p6-velopack-dependency` adds it, run alone (orchestrator's
   instruction, research "Contracts build").

### Design changes proposed for `architecture.md` (routed by the orchestrator)

1. Section 2, crate table, `wakeshot-tray` "Holds": "Velopack hooks" becomes "the Velopack wiring (`velo.rs`, the only
   file that names `velopack`)". Windows modules: the `store` row becomes "data folder paths, atomic file write, the
   RunOnce value"; new rows `install` | "the Run value (start at login) and the CLI folder in the user Path"; `hooks` |
   "what each Velopack hook does, in order and within its time, over trait `HookEnv`"; `update` | "update check,
   download and apply over trait `UpdateFeed`, which `velo.rs` implements". Why: the logic tests without `velopack`,
   and a bin crate flags code no `main` calls yet.
2. Section 3: the three new variants above (four more with Question 3). Episode flow, one line: "An update applies
   only when no hold counts, no hold file names a live process and nothing is owned; after `Update(Ok)` the core
   issues `Quit`." Section 9, core property tests, add: "no `ApplyUpdate` while a hold counts, a hold file names a live
   process or a key is owned".
3. Section 4, tray threads, new row: Update | "in tray mode only, when installed: checks GitHub Releases 5 minutes
   after start, then every 12 hours of wall time; downloads; posts `UpdateReady`; runs `ApplyUpdate`" | "power calls,
   Wakeshot's own files".
4. Section 6, "Velopack hooks": replace "Uninstall (30 s limit) restores, retrying within the limit, then removes Run,
   RunOnce and the Path entry" with "Uninstall (30 s limit) first removes Run, RunOnce and the Path entry, then
   restores, retrying until 25 s, so a restore that hangs until the limit cannot leave them behind". Add: "Install
   and updated write the Run value and the Path entry first, then restore (retries until 25 s and 12 s); the obsolete
   hook only logs." Add the data folder rule from Question 2.
5. Section 6, last sentence "Open for Phase 6: `winget upgrade` ...": replace with Haik's answer to Question 1. The two
   "not verified" points of that section (the dot in the pack id, the `current` path) get `p6-velopack-facts`'
   answers when it is done.
6. Section 10: "Velopack packing and releases: Phase 6." becomes "`release.yml` on a `v*` tag: build and test per CPU,
   `packaging/pack.ps1` (channels `win-x64`, `win-arm64`), a silent install and uninstall smoke test on each runner,
   then one job with `contents: write` runs `vpk upload github`. ARM64 builds need clang once `velopack` is in
   (`ring`)."
7. Section 11, `velopack` row: version "`=1.2.161`, exact, the same as `vpk`"; why: add "brings `ring`, which needs
   clang for ARM64".

### What this track needs from other tracks

| From | What | Needed by | Until then |
|---|---|---|---|
| Core (Phase 2) | The update rule: on `UpdateReady`, issue `ApplyUpdate` once no hold counts, no hold file names a live process and nothing is owned; `Update(Ok)` gives `Quit`; a property test for it. With Question 3: `Updated` gives `Notify(Notice::Updated)`. Not in `docs/phases/02-core.md` yet | integration | `p6-update-schedule` tests drive the apply directly |
| Power (Phase 3) | `p3-store`: the data folder path and a "remove the RunOnce value" call; `p3-log`: `LogWriter` with process `hook`; `p3-process`: liveness; `p3-restore`: `restore_from_journal()` as one attempt per call (the hooks own their retries and time limits). The Run value stays out of `store` (start at login is `install.rs`) | `p6-hooks` | the `HookEnv` fake |
| Power (Phase 3) | `p3-power-plan`'s first test: a write to an inactive plan copy leaves the real plan alone | `p6-install-e2e` | none; the e2e card waits |
| Sources (Phase 4) | `docs/hold-files.md` (`p4-hold-files`); the CLI help (`p4-cli-args`); a credit line if any rival code is ported (none planned); `wakeshot.exe` with no arguments keeps exiting 0 (winget validation) | `p6-readme`, Phase 8 | none; the README waits |
| Tray (Phase 5) | The app icon as an `.ico` at a stable path (for `vpk pack --icon`: `Setup.exe`, the Start menu shortcut, the Apps list); one picture of the icon or menu; with Question 3, the wording of `Notice::Updated`. First run detection must not use Velopack's `on_first_run`: silent installs, as winget's, never start the app (Phase 0 trial) | `p6-pack-and-release` (icon, optional), `p6-pages` | `vpk`'s default icon; no picture |
| Integration | Start the update thread in tray mode; run `ApplyUpdate`, then `Quit`, in the loop; the update notice; the update path end to end; checklist 14 with Haik | after the tracks | none |
| All tracks | After `p6-velopack-dependency`, local ARM64 checks need clang (Question 4), and each target folder compiles `velopack` once | from that card on | none |

### Checks

- Every item this track owns is in a card or named as integration or a later phase: decisions 28 and 31
  (`p6-hooks`, `p6-login-and-path`, `p6-install-e2e`); 41 (`p6-readme`, `p6-pages`; cleanup and squash Phase 8); 42
  (`p6-velopack-facts`, `p6-update-schedule`, `p6-velopack-dependency`, `p6-hooks`; the idle rule in the core, the
  loop in integration); 43 (`p6-winget-manifest`; submission Phase 8; scoop later; Store not planned); 44
  (`p6-pack-and-release` marks the signing spots; SignPath after users exist); 9's credit line (`p6-readme`);
  section 4 startup step 1 (contracts card call, `p6-velopack-dependency`); section 5 restore step 7 and the
  uninstall line of "Gap left" (`p6-hooks`, `p6-readme`); section 6 pack id, `current` path, hooks, registry values,
  data folder, the winget open point (`p6-velopack-facts`, `p6-hooks`, Question 1, Question 2); section 8 log process
  `hook` (`p6-hooks`); section 9 checklist 14 (integration); sections 10 and 11 (`p6-pack-and-release`,
  `p6-velopack-dependency`); `product.md` Distribution and "No telemetry" (README, update notes). Pass.
- No two cards own one file at the same time: the owner table in the phase file; `packaging/smoke-test.ps1` moves to
  `p6-install-e2e` only after `p6-pack-and-release` is done. No file overlaps the power or sources tracks' owner
  tables (`docs/phases/03-windows-power.md`, `04-sources.md`) or the core track's file; the tray track's file did not
  exist yet at 16:59. `ci.yml` is touched only by `p6-velopack-dependency`, only if needed, after the Phase 1 review
  fix card. Pass, with the tray track still to compare.
- Every card that needs the real laptop or Haik is marked (table above). Pass.
- No code written; `crates/`, `Cargo.toml`, `architecture.md`, `product.md`, `plan.md`, the Phase 1 file and other
  tracks' files unchanged by this card. Pass.

### Open items

- Questions 1 to 5 above; Phase 1 Questions 10 to 12 for the README.
- The orchestrator routes the contract changes and design changes above, and compares the owner table with the tray
  track's when its phase file exists.
