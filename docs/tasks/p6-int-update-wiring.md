# Integration: the update thread, `ApplyUpdate` and the notice after an update

Phase: 6. Status: done. Needs Haik: no (uses his answers 7 and 9: a notice after an update, yes). Agent:
`wakeshot-worker`. Size: about 90K tokens.
Real laptop: no. It builds only; it never runs a Velopack hook or an installed update.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p6-int-update-wiring\target`.
Runs after: `p6-int-tray-mode`, `p6-velopack-dependency` (takes over `velo.rs`), `p6-update-schedule`.
Parallel with: any card that owns none of the files below (not the e2e cards: they may fix `tray_mode.rs`).

## Goal

Connect the update thread in tray mode: `velo.rs` gives the feed and the version Velopack restarted the tray with,
`tray_mode` starts the thread when installed, `ApplyUpdate` reaches it, and `Event::Updated { version }` goes to the
core once at start after an update. Add the debug-only local feed the update path test needs.

## Read first

- `docs/phases/06-integration.md`: notes 1, 6 and 8.
- `docs/phases/06-ship.md`, "Updates"; the Results of `p6-update-schedule` (`UpdateFeed`, the thread body, how an
  apply request reaches it, how it stops), `p6-velopack-dependency` (what `velo.rs` holds, its
  `#[expect(dead_code, reason = "integration starts the update thread")]` items), `p1-close-contracts`
  (`Event::Updated`, `Notice::Updated`), `p2-core-lid` (the core's update rule and `Updated`).
- docs.rs for `velopack` 1.2.161: `UpdateManager`, its sources (a local folder source), `VelopackApp`'s restarted
  callback. Record what you find in `docs/research.md`.

## Deliverables

1. `crates/wakeshot-tray/src/velo.rs` (taken over from `p6-velopack-dependency`; keep `startup()`'s signature):
   - remove every `#[expect(dead_code, reason = "integration ...")]` by using those items;
   - the version Velopack restarted the tray with, if any, for `Event::Updated`;
   - in debug builds only (`cfg(debug_assertions)`): `WAKESHOT_UPDATE_FEED=<folder>` makes the feed a local folder
     source; release builds never read it (phase file note 8). If velopack has no local folder source, pick the next
     best seam yourself (a technical choice), with its reason and a trigger in the Result.
2. `crates/wakeshot-tray/src/main.rs` (taken over from `p6-int-skeleton`): pass velo's feed and restarted version to
   `tray_mode::run` in tray mode only (never in dry run, guard or restore mode).
3. `crates/wakeshot-windows/src/app/tray_mode.rs` (taken over from `p6-int-tray-mode`): `run` takes the update parts;
   starts the update thread only when a feed is given; `apply_update` hands the request to it; the thread is stopped
   in shutdown step 9; `Updated { version }` is passed in `first` after `Started` (and after `Session(Locked)` and `JournalCorrupt`).
4. `docs/research.md`: the velopack facts checked, with how.

## Acceptance checks

- Tests first: in `app::tray_mode::tests`, with a fake `UpdateFeed` (from `p6-update-schedule`'s test support): a feed
  given starts the thread and an `apply_update` reaches it once; no feed starts nothing and `apply_update` answers
  `Err`; `Updated` is the second or third event fed, never the first. In `velo.rs` (the bin's unit tests): with
  `WAKESHOT_UPDATE_FEED` set, a debug build picks the local folder source.
- `rustfmt --edition 2024 --check` on the three owned files.
- `cargo clippy --locked -p wakeshot-tray --all-targets --target <t> -- -D warnings` and without `--all-targets`;
  the same for `-p wakeshot-windows`; both targets (ARM64 of the tray needs clang: Haik's answer 8; if it is missing,
  CI only, recorded).
- `cargo test --locked -p wakeshot-windows -- app::tray_mode::` and `cargo test --locked -p wakeshot-tray` pass.
- `cargo build --release --locked -p wakeshot-tray --target x86_64-pc-windows-msvc`; the release exe holds no
  `WAKESHOT_UPDATE_FEED` string (`findstr`).
- Build folder deleted at the end.

## Out of scope

- The update rule (core), the schedule (`update.rs`), the hooks; a real install or update (`p6-int-update-e2e`).

## Questions for Haik

None.

## Found outside scope

Problems seen but not fixed.

1. `docs/phases/06-integration.md` note 6 (read only for this card) says the update thread starts after
   `IoThread::start`; it now starts right after the watch thread, before `Dispatcher::new` (decision 1, the module doc
   of `tray_mode.rs`). The shutdown order in step 9 is unchanged.
2. This card's acceptance check says `Updated` is "the second or third event fed"; with `Session(Locked)` and
   `JournalCorrupt` both, it is the fourth. The test checks it comes after all of them and never first.
3. For `p6-int-dry-run-e2e`: the local feed cannot be tested through a dry run. A dry run never starts the update
   thread (two gates), and the feed needs an install (Update.exe beside the exe). It works only for an installed
   debug build in tray mode, which is `p6-int-update-e2e`'s ground.
4. For `p6-install-e2e` (running now): any packed build made from the code after 03:42 starts the update thread. An
   installed tray left running more than 5 minutes calls the GitHub API once (no token); the repo has no release
   (`gh release list`, 03:52), so nothing is downloaded. What `GithubSource` answers for a repo with no release
   (`RemoteIsEmpty` or an error line): not verified.
5. For whoever quotes exe sizes (`p6-media`, `p6-launch-research`, README): the x64 release tray is now 3,947,520
   bytes, not 2,026,496 (`p6-velopack-dependency`, `p6-install-e2e` line 91): the HTTPS stack is linked in now.
6. While I edited (about 03:41 to 03:42) the tree did not compile for about a minute: `tray_mode::run` had its new
   signature before `main.rs` was changed. A build by another card in that minute would have failed on it.

## Log

- 2026-11-03: card written by `p6-plan-integration`.
- 2026-11-04 03:34: started by `wakeshot-worker`; reading the card, the phase notes and the Results it names.
- 2026-11-04 03:40: read integration notes 1, 6, 8, ship "Updates", the Results of `p6-update-schedule`,
  `p6-velopack-dependency`, `p6-int-tray-mode`, the update parts of `p1-close-contracts` and `p2-core-lid`, and the
  code (`velo.rs`, tray `main.rs`, `tray_mode.rs`, `update.rs`, `dispatch.rs`). velopack 1.2.161 read in the cargo
  registry and on docs.rs: `sources::FileSource::new(path)` exists (local folder source),
  `on_restarted(FnOnce(Version))` fires inside `run()` after it removes `VELOPACK_RESTART`,
  `locator::auto_locate_app_manifest` is public. Plan: a
  `OnceLock` for the restarted version, `velo::updates()` gives `Updates { feed, restarted }`, the update thread starts
  with the watch thread (before `Dispatcher::new`) so `RealRunners` owns its handle.
- 2026-11-04 03:44: code and tests written in one pass (`tray_mode.rs`: `Updates`, `MakeFeed`, `Updater`, `AnyFeed`,
  `first_events`; `velo.rs`: `RESTARTED`, `updates()`, `installed()`, `Source`; `main.rs`: the Tray and DryRun arms).
  rustfmt on the three files (only they changed, by write time).
- 2026-11-04 03:48: x64 clippy (both packages, both forms) clean; `app::tray_mode` 15 passed; tray tests pass (velo 7
  of 8 in `main.rs`). ARM64 clippy of `wakeshot-windows` both forms clean; ARM64 tray fails on `ring` (no clang).
  Release x64 tray built; `findstr WAKESHOT_UPDATE_FEED` finds nothing in it (found in the debug test exe). The release
  tray grew to 3,947,520 bytes: `feed()` is no longer dead code, so LTO keeps the HTTPS stack (`ureq`, `rustls`,
  `ring`; their strings are in the exe now).
- 2026-11-04 03:49: no Wakeshot process ran before; 16 `NotifyIconSettings` ids listed. Smoke run 1 (03:48:54 to
  03:48:57): passed, tray pid 37940, exit code 0, no update line in the dry run's log (by design). One new entry,
  id `15983757285637097514` (only `IconSnapshot` in it, as the velopack card saw), exported to the scratchpad and
  deleted 03:49:34; no AUMID key.
- 2026-11-04 03:50: `docs/research.md` "Update wiring checks" added; `architecture.md` section 11 velopack line fixed.
- 2026-11-04 03:52: the card's exact test commands (host target) pass: 15 and 8 + 5 (2 ignored) + 0 (1 ignored). x64
  clippy and rustfmt rerun after the format: clean. ARM64 tray clippy `--all-targets`: exit 101, `ring`. Icon entry
  and AUMID key still gone at 03:52:35; pid 37940 gone; no smoke temp folder. Build folder deleted, checked gone.
  `gh release list -R HaikAsatryan/wakeshot`: no release yet.
- 2026-11-04 03:53: Result written; status done.

## Result

**Done 2026-11-04 03:53.** No power setting, no install, no hook, no Run or RunOnce value touched.

### Built

- `crates/wakeshot-windows/src/app/tray_mode.rs`:
  - `pub type MakeFeed = Box<dyn FnOnce() -> Box<dyn UpdateFeed> + Send>` and `pub struct Updates { pub feed:
    Option<MakeFeed>, pub restarted: Option<String> }` (`Default`: no feed, no restart).
  - `run(dry_run, updates)`. The dry run drops `updates` even if given; the real tray logs "Wakeshot is not installed,
    so it does not check for updates" when no feed came.
  - `Updater` (in `RealRunners`): starts `UpdateThread::start(move || AnyFeed(make()), SystemClock, sink, log)` when a
    feed is given (a failed start is one Warn line, not fatal); `apply()` hands `ApplyUpdate` to the handle, and when
    no thread runs or it has ended (`apply()` false) posts `Done(Update(Err(no update thread, 50)))` with one Warn line;
    `stop()` first in `shut_down`, right after `IoThread::finish`. `AnyFeed` adapts the boxed feed to the generic
    `start`. `answer_update` and `UPDATE_NOT_WIRED` are gone (now `NO_UPDATE_THREAD`).
  - `first_events(started, locked, corrupt, updated)`: `Started`, `Session(Locked)`, `JournalCorrupt`, `Updated`; one
    Info line "Velopack restarted the tray after the update to <v>".
- `crates/wakeshot-tray/src/velo.rs`: the `expect(dead_code)` is gone (`feed()` is used). `startup()` keeps its
  signature and adds `on_restarted`, which keeps the version in `static RESTARTED: OnceLock<String>`.
  `updates()` gives the feed only when `installed()` (`locator::auto_locate_app_manifest(FromCurrentExe)` is not
  `NotInstalled`) and the restarted version. `Source::pick` chooses GitHub, or in debug builds only the folder in
  `WAKESHOT_UPDATE_FEED` (`FileSource`); `feed()` uses `UpdateManager::new_boxed`.
- `crates/wakeshot-tray/src/main.rs`: `Mode::Tray` calls `tray_mode::run(false, velo::updates())`; `Mode::DryRun`
  calls `run(true, Updates::default())`. Guard and restore mode never see it.
- `docs/architecture.md` section 11: the velopack row (declared, pinned, tray only, clang for ARM64).
- `docs/research.md`: new section "Update wiring checks".

### How the local feed is set (for `p6-int-update-e2e`)

1. Build the debug exes (`cargo build`, no `--release`) and pack them with `vpk`, as `packaging/pack.ps1` does, into
   an install; a release build never reads the variable.
2. Make a feed folder holding a newer version's `vpk pack` output: `releases.<channel>.json` (for x64
   `releases.win-x64.json`) and its `.nupkg` files.
3. Start the installed tray (tray mode, not `--dry-run`) with `WAKESHOT_UPDATE_FEED=<feed folder>` in its
   environment. An empty value counts as unset. The first check runs after 5 minutes of run time.
4. Not verified: that the tray Update.exe restarts still has the variable (likely, Update.exe is the tray's child).

### Checks (build folder `<scratchpad>\p6-int-update-wiring\target`, all `--locked`, no `RUSTFLAGS`)

| Check | Outcome |
|---|---|
| `rustfmt --edition 2024 --check` on the three owned files | exit 0 (after one `rustfmt` of them) |
| `cargo clippy -p wakeshot-tray --all-targets --target x86_64-pc-windows-msvc -- -D warnings`, and without `--all-targets` | clean, both |
| the same for `-p wakeshot-windows`, x64 | clean, both |
| the same for `-p wakeshot-windows`, `aarch64-pc-windows-msvc` | clean, both |
| the same for `-p wakeshot-tray`, ARM64 | **ARM64 tray: CI only**: exit 101, `error: failed to run custom build command for ring v0.17.14`, cc-rs `failed to find tool "clang": program not found` (no clang on this laptop) |
| `cargo test --locked -p wakeshot-windows -- app::tray_mode::` | 15 passed (also with `--target x86_64-pc-windows-msvc --lib`) |
| `cargo test --locked -p wakeshot-tray` | `main.rs` 8 passed; `recovery.rs` 5 passed, 2 ignored; `smoke.rs` 1 ignored |
| `cargo build --release --locked -p wakeshot-tray --target x86_64-pc-windows-msvc` | exit 0; exe 3,947,520 bytes |
| `findstr /c:"WAKESHOT_UPDATE_FEED"` on the release exe | not found (exit 1); found in the debug test exe (control) |
| Smoke: `cargo test --locked -p wakeshot-tray --target x86_64-pc-windows-msvc --test smoke -- --ignored --test-threads=1` | 1 passed (2.5 s), tray pid 37940, exit code 0; output `<scratchpad>\p6-int-update-wiring\smoke-run1.txt` |
| Dry-run `NotifyIconSettings` entry | one new id `15983757285637097514`, exported to `<scratchpad>\p6-int-update-wiring\notifyicon-15983757285637097514.reg`, deleted 03:49:34, still gone with no AUMID key at 03:52:35 |
| Build folder | deleted, checked gone |

New unit tests: `a_feed_starts_the_thread_and_apply_update_reaches_it_once`,
`no_feed_starts_nothing_and_apply_update_answers_err`, `an_update_thread_that_ended_gets_err_from_the_loop`,
`a_lost_update_answer_is_logged`, `updated_comes_after_started_locked_and_journal_corrupt_never_first` (tray mode);
`a_build_folder_exe_gets_no_update_thread_and_no_updated_notice`,
`with_the_local_feed_set_a_debug_build_reads_that_folder`, `without_the_local_feed_the_feed_is_github` (velo).

### Decided by the worker, for Haik's review

None of these changes what the laptop does when no work is going on.

| # | Choice | Why | Change it when |
|---|---|---|---|
| 1 | The update thread starts right after the watch thread, before `Dispatcher::new`, not after `IoThread::start` (note 6) | `RealRunners` owns the handle, so no slot shared between `serve` and the runners; the thread posts nothing for 5 minutes and no apply comes before its own `UpdateReady`; it still stops right after `IoThread::finish` | Something in step 8 must run before the first check can start |
| 2 | The install check runs in `velo::updates()` on the main thread (`auto_locate_app_manifest`), before `tray_mode::run` | "Installed only" without a thread that waits 5 minutes to learn it; the check is a few file reads | A start-time trace shows it |
| 3 | Only `NotInstalled` means no install; a broken install still runs the thread | Its checks then log the real error every 12 hours, which says what broke | The log fills with one broken install's lines |
| 4 | The dry run drops `Updates` in `tray_mode::run` as well as in `main` | A dry run from an installed copy must never end that copy's tray or run its hooks; two gates cost one line | Never likely |
| 5 | `ApplyUpdate` with no thread answers `Update(Err(no update thread, 50))` and one Warn line | The core logs it and waits for the next `UpdateReady`, never stuck | A log shows it often (then a bug in the thread or the core) |
| 6 | The velo test reads the variable through `Source::pick`'s reader, not `std::env::set_var` | `set_var` is `unsafe` in edition 2024: other test threads read the environment (`feed()`) at the same time | Never likely |
| 7 | Two new Info lines: "Wakeshot is installed, so it checks for updates" (or "not installed ...") and "Velopack restarted the tray after the update to <v>" | One line each per run; the update e2e and a field log can tell the path taken | Never likely |

### Open items

- ARM64 tray: CI only until Haik installs clang (`p6-velopack-dependency` step 0).
- For the orchestrator: see "Found outside scope".
