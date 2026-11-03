# Hibernate: the last check and the call

Phase: 3. Status: done. Needs Haik: no. Real laptop: no (never calls the real `SetSuspendState`; the real hibernate
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

None.

## Found outside scope

Problems seen but not fixed.

1. `docs/architecture.md` section 5 "Lock and hibernate" (and this card's Deliverables) list the order "sound,
   SeShutdownPrivilege, `IsPwrHibernateAllowed`, `SetSuspendState`". The code runs the privilege and the allowed check
   before the sound (Result, choice 1). That line needs the new order; not my file.
2. `testing.rs` (contracts) does not re-export `FakeHibernate`. Other crates reach it as
   `wakeshot_windows::power::hibernate::FakeHibernate` with `test-support`. `FakeRequests` and `FakeLocker` are not
   re-exported either, so nothing is broken; a one-line `pub use` would match the module doc ("gathers them in one
   place").
3. For `p5-sounds`: the last lid and idle check comes before the going to sleep sound, and `play_and_wait` may wait up
   to 3 s. A lid opened during the chime still hibernates. A short cue keeps that window small.
4. For `p6-int-tray-mode`: the `cfg!(test)` guard in `Win32Hibernate::hibernate` covers only this crate's unit tests.
   A test in another crate that runs `run_hibernate` with `Win32Hibernate` would really hibernate: use
   `DryRunHibernate` or `FakeHibernate` there.
5. No other card's file failed a build or a check during this card.

## Log

- 2026-11-03 17:00: card written by `p3-plan-windows-power`.
- 2026-11-03 20:12, orchestrator, from `p2-policy-power`'s "Found outside scope" item 5: the `AboveFloor` check can
  call `policy::floor::below` (strictly below; an unknown percent is not below), so the core and the hibernate thread
  cannot drift apart. The idle check must match `policy::ending::clamped_idle_ms` and `IDLE_MS`.
- 2026-11-03 21:06, worker: started. Status in progress. Reading the card's sources.
- 2026-11-03 21:12: sources read (phase notes 1, 2, 8; architecture 3, 4, 5; product; research; review M5; spike;
  `system.rs`, `clock.rs`, `sound.rs`, `policy::floor`, `policy::ending`). Microsoft pages re-read for
  `SetSuspendState`, `IsPwrHibernateAllowed` and `AdjustTokenPrivileges`. Design: privilege and allowed move ahead of
  the sound (see Result, choices). Writing `hibernate.rs`.
- 2026-11-03 21:17: `hibernate.rs` written; 21 tests pass. Real reads on this laptop (elevated, lid open, AC):
  `IsPwrHibernateAllowed` true, `GetPwrCapabilities` SystemS4 true, HiberFilePresent true; SeShutdownPrivilege off,
  then enabled, read back enabled. No test calls `Win32Hibernate::hibernate`; a test build panics there instead of
  hibernating. Running the acceptance checks.
- 2026-11-03 21:18: every acceptance check passed (Result). `docs/research.md` "Hibernate thread checks" added. Build
  folder deleted. Done.

## Result

**Status: done.** Not run on the real hibernate; no test calls `SetSuspendState`.

### Files changed

- `crates/wakeshot-windows/src/power/hibernate.rs` (owned).
- `docs/research.md`: "Hibernate thread checks" added at the end (addition only).
- This card.

### API (for `p6-int-tray-mode`)

All in `wakeshot_windows::power::hibernate`. Log area `hibernate`.

- `pub fn run_hibernate(reason: CountdownReason, floor_pct: u8, last_resume_mono_ms: Option<u64>, deps: &dyn
  HibernateDeps, sound: &dyn SoundPlayer, log: &dyn Log) -> HibernateEnd`. The three values are the fields of
  `Command::Hibernate`. It blocks: up to 2 s for the lid, up to 3 s for the sound, and on success through the whole
  hibernate until resume. It never panics on a Windows result. It has no guard against two runs at once: "one at a
  time" is the dispatcher's (`06-integration.md` note 4).
- Steps in order, each logged, the first failure ends the run:

  | Step | Fails as |
  |---|---|
  | lid closed (`lid_once(LID_WAIT_MS)`, 2000 ms) | open `Skipped(LidOpen)`; no answer `Skipped(LidUnknown)` |
  | idle: `ending::clamped_idle_ms(reported, mono now, last resume)`, due by `ending::check` (5 min) | `Skipped(InputSeen)`, also when the read fails |
  | `Floor` only: on battery | `Skipped(OnAc)`, also when the read fails |
  | `Floor` only: `floor::below(pct, floor_pct)` | `Skipped(AboveFloor)`, also at the floor and for an unknown percent |
  | SeShutdownPrivilege | `Failed(AdjustTokenPrivileges 1300 or the failed call)` |
  | `IsPwrHibernateAllowed` | `NotAllowed` |
  | `sound.play_and_wait(Cue::GoingToSleep)` | never: a failure is logged and the run goes on |
  | `SetSuspendState(TRUE, FALSE, FALSE)` | error `Failed(SetSuspendState, code)`; return `Resumed` |

- `pub trait HibernateDeps: Send + Sync`: `lid_closed() -> Option<bool>`, `input_idle_ms() -> Result<u64, WinError>`,
  `mono_now_ms() -> u64`, `power_status() -> Result<(bool, Option<u8>), WinError>`, `enable_shutdown_privilege() ->
  Result<(), WinError>`, `hibernate_allowed() -> bool`, `hibernate() -> Result<(), WinError>`. All take `&self`.
- `Win32Hibernate` (unit struct, `Copy`, `Default`): `system::lid_once`, `clock::input_idle_ms`, `clock::now().mono_ms`,
  `system::power_status`, the token calls, `IsPwrHibernateAllowed`, `SetSuspendState`.
- `DryRunHibernate<D = Win32Hibernate>`: `DryRunHibernate::new(log: Arc<dyn Log>)` over the real reads;
  `DryRunHibernate::over(reads, log)` over any. Reads pass through; the privilege and the call only log `dry run: ...`
  and return `Ok`, so a run whose checks pass answers `Resumed` at once.
- Behind `test-support`: `FakeHibernate` (pub fields `lid`, `idle_ms`, `mono_ms`, `power`, `privilege`, `allowed`,
  `hibernate`, `calls`; `Default` passes every check; `calls() -> Vec<HibernateCall>`) and `HibernateCall { Lid,
  InputIdle, MonoNow, PowerStatus, Privilege, Allowed, Hibernate }`.
- Wiring: keep one `Arc<dyn HibernateDeps>` (`Win32Hibernate` or `DryRunHibernate::new(log.clone())`) with the
  `Arc<dyn SoundPlayer>` and `Arc<dyn Log>`; per command, `thread::spawn(move || run_hibernate(reason, floor_pct,
  last_resume_mono_ms, &*deps, &*sound, &*log))` and post `Event::Done(Outcome::Hibernate(end))`
  (`deps_go_to_another_thread` does exactly this with a fake).

### Choices (why; change it when)

1. **Privilege and `IsPwrHibernateAllowed` run before the sound**, not after it as the card and architecture list.
   Why: with hibernate turned off (`powercfg /h off`, common to save disk), the listed order plays "going to sleep"
   and then shows a failure notice, the chime without a sleep that review M5 names as a fault. Both calls are fast and
   change nothing outside this process, so the chime still comes right before the call. Change it when a step must
   follow the sound. Architecture section 5 needs the new order (Found outside scope 1).
2. **Every unknown is a skip**: a failed idle read is `InputSeen`, a failed power read `OnAc` (as `power_status` reads
   an unknown source), an unknown percent `AboveFloor` (`floor::below`). The same rule as the unknown lid. Change it
   when a PC is seen whose read always fails, so it could never hibernate.
3. **A failed privilege is `Failed` at once**, `ERROR_NOT_ALL_ASSIGNED` (1300) included, instead of calling
   `SetSuspendState` anyway: the page says the call needs it, and the log then names the real cause. Change it when
   `SetSuspendState` is seen to work without it.
4. **A failed or skipped sound never stops the hibernate**: the settings are already restored, so stopping would
   leave a closed laptop running in a bag.
5. **The idle check calls `ending::check(..) == Due`**, not its own `>=`, so it cannot drift from the core.
6. **`WorkEnded` never reads the power source**: work that ended needs the hibernate on AC too (`ending::cancels`).
7. **The dry run reads real values and logs the two calls that change something** (track note 8). It is generic over
   its reads so `dry_run_never_hibernates` can pass every check and still see no privilege call and no hibernate.
8. **`IsPwrHibernateAllowed` is kept** (card, architecture, tested here), though its page now says it "may be altered
   or unavailable" and to use `GetPwrCapabilities`. `hibernate_allowed_reads` checks allowed implies `SystemS4`.
   Change it when the call goes away or disagrees with `GetPwrCapabilities` (`SystemS4` and `HiberFilePresent`).
9. **A test build cannot hibernate**: `Win32Hibernate::hibernate` panics under `cfg!(test)`. Shipping builds are
   unchanged.
10. **`bWakeupEventsDisabled` FALSE**, as the card says: which wakes TRUE would turn off, the power button among them
    or not, is not verified.

### Acceptance checks (2026-11-03 21:13 to 21:18, own build folder, every cargo command with `--locked`)

| Check | Outcome |
|---|---|
| `rustfmt --edition 2024 --check crates/wakeshot-windows/src/power/hibernate.rs` | exit 0 |
| `cargo clippy --locked -p wakeshot-windows --all-targets --target <t> -- -D warnings`, x64 and ARM64 | exit 0, both |
| the same without `--all-targets`, x64 and ARM64 | exit 0, both |
| extra: the same with `--features test-support`, x64 | exit 0 |
| `cargo test --locked -p wakeshot-windows power::hibernate::` | 21 passed, 0 failed |
| extra: `cargo test --locked -p wakeshot-windows` | 225 passed, 13 ignored, 0 failed |
| no test calls `Win32Hibernate::hibernate` | grep: the only `.hibernate()` call is `run_hibernate`'s `deps.hibernate()`; tests pass `FakeHibernate` or `DryRunHibernate` over it |

Tests: the card's `lid_open_skips`, `lid_unknown_skips`, `input_seen_skips` (also exactly 5 minutes passes),
`recent_resume_counts_as_input` (also a resume 5 minutes ago passes, one stamped after now skips),
`floor_on_ac_skips` (also a failed read), `floor_above_skips` (at, above, 100%, unknown; one below passes),
`checks_run_in_order_and_stop_at_first_fail`, `sound_plays_only_when_all_checks_pass` (the sound comes after the six
earlier calls and right before the call), `not_allowed`, `call_failed`, `resumed`, `dry_run_never_hibernates`,
`privilege_enables`, `hibernate_allowed_reads`. Added: `idle_read_failure_counts_as_input`,
`work_ended_never_reads_the_power_source`, `sound_failure_still_hibernates`, `privilege_failure_fails`,
`each_step_is_logged` (every line, exact text), `deps_go_to_another_thread`, `win32_errors_keep_their_code`.

Real values here (elevated, lid open, AC): `IsPwrHibernateAllowed` true, `SystemS4` true, `HiberFilePresent` true;
SeShutdownPrivilege turned off, then enabled by `enable_shutdown_privilege`, read back enabled.

### Open items

- Integration checklist 7 runs the real hibernate. Whether `SetSuspendState` can return TRUE without hibernating:
  not verified (research, "Hibernate thread checks").
- The Medium token enable was not re-run here; the lid spike showed it works from a process started by Explorer.
- Found outside scope 1 to 4 for the orchestrator, `p5-sounds` and `p6-int-tray-mode`.
- This card's code is in the power safety review scope (`docs/phases/02-06-lighter.md`, "Reviews").
