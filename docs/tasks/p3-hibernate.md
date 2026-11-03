# Hibernate: the last check and the call

Phase: 3. Status: ready. Needs Haik: no. Real laptop: no (never calls the real `SetSuspendState`; the real hibernate
is integration checklist 7). Agent: `wakeshot-worker`. Size: about 90K tokens.
Runs after: `p3-system-events`. Parallel with: `p3-journal`, `p3-power-io` and any card of tracks 2, 4, 5, 6.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p3-hibernate\target`.

## Goal

Fill `power::hibernate`: the hibernate thread's run for `Command::Hibernate`: the last check, the "going to sleep"
sound, then the hibernate, each failure answered as `HibernateEnd`.

## Read first

- `CLAUDE.md`; `docs/phases/03-windows-power.md` (track notes 1, 2, 8).
- `docs/architecture.md`: section 5 "Lock and hibernate", section 3 "Clocks" (idle clamped at resume) and the
  countdown, section 4 "Tray threads" (hibernate thread).
- `docs/product.md`: "Power behavior", the work-end and battery floor points.
- `docs/research.md`: "Lid and sleep spike" (`SetSuspendState`, privilege, hibernate here, lid notification).
- `docs/reviews/p1-contracts-domain.md`: M5 (who plays the sound).
- `spikes/lid-sleep/src/power.rs` (`set_suspend_state`, `hibernate_allowed`), `sys.rs` (`enable_shutdown_privilege`).
- After the contracts card and `p3-system-events`: `power/system.rs` (`lid_once`, `power_status`), `clock.rs`
  (`input_idle_ms`), `sound.rs` (`SoundPlayer::play_and_wait`, a contract addition).

## Deliverables

`crates/wakeshot-windows/src/power/hibernate.rs`:

- `run_hibernate(reason, floor_pct, last_resume_mono_ms, deps, sound, log) -> HibernateEnd`. Checks in this order,
  each a `Skipped` on failure: lid closed (`lid_once(2000)`; `None` is `LidUnknown`, open is `LidOpen`); idle, the
  smaller of input idle and the time since the last resume, at least 5 minutes (`InputSeen`); for `Floor`, on battery
  (`OnAc`) and battery percent below the floor (`AboveFloor`). Then `sound.play_and_wait(Cue::GoingToSleep)` (the
  tray track sets the wait; the core never sends this cue, review M5), SeShutdownPrivilege, `IsPwrHibernateAllowed`
  (false: `NotAllowed`),
  `SetSuspendState(TRUE, FALSE, FALSE)` (error: `Failed`; returns after resume: `Resumed`). Each check logged.
- Trait `HibernateDeps` (lid, idle, mono now, power status, privilege, allowed, hibernate) with `Win32Hibernate`,
  `DryRunHibernate` (every step logged, never hibernates) and `FakeHibernate` (behind `test-support`).

## Acceptance checks

Tests in `power::hibernate::tests`, with `FakeHibernate` and `FakeSoundPlayer`:

- One test per skip: `lid_open_skips`, `lid_unknown_skips`, `input_seen_skips`, `recent_resume_counts_as_input`,
  `floor_on_ac_skips`, `floor_above_skips` (at the floor is not below it).
- `checks_run_in_order_and_stop_at_first_fail`, `sound_plays_only_when_all_checks_pass`, `not_allowed`,
  `call_failed`, `resumed`, `dry_run_never_hibernates`.
- Real, harmless: `privilege_enables` and `hibernate_allowed_reads` (record the value in the Log).
- Never: a test that calls `Win32Hibernate::hibernate`.

Commands: `rustfmt --edition 2024 --check <owned files>`; `cargo clippy --locked -p wakeshot-windows --all-targets` with
`-D warnings` for both targets; `cargo test --locked -p wakeshot-windows power::hibernate::`. Record each outcome.

- Check rule: `docs/plan.md` "How we work" step 2 and "Checks while other cards run" in this track's phase file. A
  failure in a file another card owns goes in the Log: wait and run again; if it is still there when this card's own
  work is done, list it under "Found outside scope". Never edit, format or `--fix` that file. Every cargo command takes
  `--locked`, and clippy runs once more without `--all-targets` (the libs as they ship; `CLAUDE.md` "Checks"). No
  `--workspace` or `--all` run here; the orchestrator runs those when it closes a wave.

## Out of scope

- The countdown and when to send `Hibernate` (core); the sound and how long to wait for it (tray track; until then
  `FakeSoundPlayer`); a real hibernate (integration checklist 7).
- Any file but `power/hibernate.rs`.

## Questions for Haik

Each with the worker's default.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-03 17:00: card written by `p3-plan-windows-power`.
- 2026-11-03 20:12, orchestrator, from `p2-policy-power`'s "Found outside scope" item 5: the `AboveFloor` check can
  call `policy::floor::below` (strictly below; an unknown percent is not below), so the core and the hibernate thread
  cannot drift apart. The idle check must match `policy::ending::clamped_idle_ms` and `IDLE_MS`.

## Result

What was done, files changed, checks and outcomes, open items.
