# Integration: the update thread, `ApplyUpdate` and the notice after an update

Phase: 6. Status: ready. Needs Haik: no (uses his answers 7 and 9: a notice after an update, yes). Agent:
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

## Log

- 2026-11-03: card written by `p6-plan-integration`.

## Result

What was done, files changed, checks and outcomes, open items. Include how the local feed is set.
