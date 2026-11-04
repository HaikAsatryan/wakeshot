# Make the watch thread's tick test wait instead of sleep

Phase: 4 (sources). Status: done. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 30K tokens.
Written by the orchestrator (session `wakeshot-27`, 2026-11-04) from CI run 41816496469 (commit 83b38fd).
Runs after: `p4-watch-thread` (done). Parallel with: `p2-6-reset-gaps` (core only; this card owns no core file).
Real laptop: no.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p4-watch-tick-flake\target`.

## Goal

`watch::thread::tests::a_tick_posts_nothing_when_nothing_changed` (`crates/wakeshot-windows/src/watch/thread.rs`,
about line 1116) failed on the ARM64 CI runner: "the ticks ran: 3 checks before, 5 after". It calls `rig.tick()` three
times with a fixed 50 ms sleep each, then expects at least 3 more liveness checks. On a slow runner the third tick had
not run yet. The x64 job and the core passed on the same commit. Make the test wait for what it checks, so it does not
flake, without making it weaker.

## Deliverables

1. Wait until the liveness check count reaches `checks + 3` (the file's `until` helper or the same pattern, with its
   usual time limit), then assert that nothing new was posted (`rig.settled() == first`) and the rest as now.
2. Look through the other tests in the same file for the same fixed-sleep-then-count pattern; fix any in the same way.
   List them in the Result.

## Files this card owns

The `#[cfg(test)]` part of `crates/wakeshot-windows/src/watch/thread.rs`, and this card. No product code.

## Acceptance checks

- `cargo fmt --all --check`; clippy with and without `--all-targets` on x64, `-D warnings`.
- `cargo test -p wakeshot-windows --locked watch::thread` 20 times in a row (debug), also once with 8 copies at once
  (load), all pass. Then `cargo test --workspace --locked` once.
- No tray started; Haik's plan never written; build folder deleted at the end.

## Questions for Haik

None.

## Found outside scope

- `cargo test --workspace --locked` on 2026-11-04 failed in 3 `wakeshot-core` scenario tests
  (`scenarios::lid::a_hold_key_released_while_the_floor_short_apply_is_out_keeps_the_short_timeouts`,
  `scenarios::lid::a_restore_retry_that_works_while_the_short_timeouts_are_on_writes_them_again`,
  `scenarios::props::a_lost_journals_reset_over_a_running_holds_values_applies_the_hold_again`). This card touches no
  core file; most likely the `p2-6-reset-gaps` worker's core edits in progress (not verified). Not touched.
- `a_restore_request_posts_one_menu_restore` ticks, sleeps 300 ms and counts restores, so on a slow runner it can
  pass before its tick ran: that rig has no live session, so nothing shows the tick ran. It cannot flake, only pass
  for a weaker reason. Left as is: showing the tick would need a session in that rig, a change to what it tests.

## Log

- 2026-11-04: started; read the card, CLAUDE.md, "How we work" and the test part of `watch/thread.rs`.
- 2026-11-04: likely cause (read from the code, not verified on the ARM64 runner): `tick()` only sets a signal, and
  ticks that come while one runs count once, so on a slow runner two of the three 50 ms-apart ticks merged (each
  tick checks the one live session once: 3 checks before, 5 after, so two ticks ran). A single wait for `checks + 3`
  after three quick ticks could then hang, so the fix ticks once and waits for its check, three times.
- 2026-11-04: added `Rig::checks` and `Rig::tick_and_wait`; used them in the tick test and in the failed-post test's
  last tick (it slept 300 ms and compared, so a late tick passed it for the wrong reason). Building next.
- 2026-11-04: fmt and both clippy runs clean; 20 serial runs pass. Load run (8 copies): 7 pass, 1 fail in
  `a_restore_request_posts_one_menu_restore`: two `Menu(Restore)`. Cause: the test wrote the request with
  `fs::write`, so the busy thread read the empty file (counts anyway, by design) and then the full one with a new
  write time (counts again, by design). The CLI writes it with `store::write_atomic`. Fix in the test part:
  `Rig::write` writes data folder files atomically, used for holds, config and the restore request. Rerunning.
- 2026-11-04: fmt, both clippy runs clean; 20 serial runs and 3 load rounds of 8 copies all pass. Workspace run:
  every crate passes except 3 core scenario tests (see "Found outside scope"). Build folder deleted.

## Result

Done. All changes are in the `#[cfg(test)]` part of `crates/wakeshot-windows/src/watch/thread.rs`.

- `a_tick_posts_nothing_when_nothing_changed`: ticks once and waits (the file's `until`, 10 s limit) for the session
  check that tick makes, three times, then asserts `settled() == first` and the fed bytes as before. One tick at a
  time, since ticks that come while one runs count once: three quick ticks and one wait for `checks + 3` could hang.
  New `Rig::checks` and `Rig::tick_and_wait`.
- Same pattern found: `a_failed_post_is_logged_once_and_the_next_tick_posts_everything_again`, last tick (tick, sleep
  300 ms, compare). Now `tick_and_wait`, so it cannot pass before the tick ran. The other sleeps in the file come
  before "nothing more happened" asserts, where a sleep is right.
- The load check found a second flake: `a_restore_request_posts_one_menu_restore` posted `Menu(Restore)` twice
  (1 of 8 copies), because `fs::write` let the thread read the empty file, then the full one. The CLI writes it
  atomically, so `Rig::write` now writes data folder files (holds, config, restore request) with
  `store::write_atomic`, as Wakeshot's own writers do. Change it when: a test needs a torn write on purpose (then
  write it plainly in that test).

Checks: `cargo fmt --all --check` clean. Clippy `-p wakeshot-windows` x64 with and without `--all-targets`,
`-D warnings`: clean (only my crate, as other cards build at the same time). `cargo test -p wakeshot-windows --locked
watch::thread`: 20 of 20 serial runs pass; load, 8 copies of the test binary at once (run directly, so the copies do
not queue on cargo's lock), 3 rounds: 24 of 24 pass (before the atomic write fix: 7 of 8). `cargo test --workspace
--locked --no-fail-fast`: every crate passes except 3 `wakeshot-core` scenario tests, most likely the
`p2-6-reset-gaps` worker's core edits in progress (not verified); `wakeshot-windows` 563 passed. No tray started, no
power setting written, build folder deleted. Nothing new for `docs/research.md` (the orchestrator limited this card
to its own files; the cause above is a code reading, not verified on the ARM64 runner).

Open: rerun the workspace tests once `p2-6-reset-gaps` lands; the next CI run on ARM64 is the real proof.
