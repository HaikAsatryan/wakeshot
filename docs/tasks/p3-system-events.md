# System events: lid, AC and battery, active plan, suspend and resume, input idle

Phase: 3. Status: done. Needs Haik: no. Real laptop: no (notification registrations and reads only). Agent:
`wakeshot-worker`. Size: about 100K tokens.
Runs after: Haik's sign-off of the track plans; the contracts card. Parallel with: every other wave 1 card of this
track and any card of tracks 2, 4, 5, 6.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p3-system-events\target`.

## Goal

Fill `power::system` and `clock::input_idle_ms`: the power notifications become events posted through the event sink,
and the reads the hibernate thread needs exist.

## Read first

- `CLAUDE.md`; `docs/phases/03-windows-power.md` (track notes 1, 2, 9).
- `docs/architecture.md`: section 4 "Tray threads" (callbacks only post), section 5 "OS events" table, "Lock and
  hibernate" (the fresh lid registration), section 3 "Clocks" (input idle).
- `docs/research.md`: "Power settings spike" (setting notifications, the lid value is not the index), "Lid and sleep
  spike" (lid notification, resume, clock).
- `spikes/lid-sleep/src/events.rs` (`Subscriptions::start` and the callbacks), `sys.rs` (`idle_ms`, `power_status`).
  Port by reading and rewriting.
- The event sink trait and its fake from the contracts card (`EventSink`, `FakeEventSink`).

## Deliverables

`crates/wakeshot-windows/src/power/system.rs` and `crates/wakeshot-windows/src/clock.rs`:

- `Subscriptions::start(sink)`: `PowerSettingRegisterNotification` with `DEVICE_NOTIFY_CALLBACK` for
  `GUID_LIDSWITCH_STATE_CHANGE` (`Event::Lid`), `GUID_ACDC_POWER_SOURCE` and `GUID_BATTERY_PERCENTAGE_REMAINING`
  (`Event::Power`, keeping the last of each so both fields are filled; value 2 counts as battery, track note 9),
  `GUID_ACTIVE_POWERSCHEME` (`Event::ActivePlan(guid)`, a contract addition), and
  `PowerRegisterSuspendResumeNotification` (`PBT_APMSUSPEND`: `Suspending`; `PBT_APMRESUMEAUTOMATIC`: `Resumed`).
  Callbacks only decode and post. Drop unregisters all; check in Microsoft's docs whether a callback can still run
  after unregister and guard against it (record what the docs say).
- Payload decoders as pure functions, so tests need no hardware.
- `lid_once(timeout_ms) -> Option<bool>`: a fresh lid registration that waits for its first callback; `None` on
  timeout (the hibernate thread's last check).
- `power_status() -> Result<(bool, Option<u8>), WinError>` (`GetSystemPowerStatus`: on AC, battery percent).
- `clock::input_idle_ms()`: `GetLastInputInfo` against `GetTickCount`, with wrapping arithmetic (the tick wraps after
  49.7 days).

## Acceptance checks

Tests in `power::system::tests` and `clock::tests`:

- `decode_lid`, `decode_acdc` (0 AC, 1 and 2 battery), `decode_battery_pct`, `decode_plan_guid`,
  `decode_suspend_resume` (ignore `PBT_APMRESUMESUSPEND` and others).
- `real_registration_gives_first_values` (real, harmless; `#[ignore]`, it needs a lid that is open): within 2 s the
  fake sink gets a lid value (open), a power value that matches `power_status()`, and the active plan GUID.
- `real_lid_once_returns_open_here` (real; `#[ignore]`, same reason) and `lid_once_times_out` (an injected
  registration that never calls back).
- The two `real_` tests run here with `cargo test --locked -p wakeshot-windows -- --ignored power::system::` while the
  lid is open, and again in `p3-real-checks` step 2. GitHub runners have no lid, and a closed lid here would fail every
  worker's run (review M9).
- `input_idle_wraps` (pure) and `input_idle_reads` (real: a number, no error).

Commands: `rustfmt --edition 2024 --check <owned files>`; `cargo clippy --locked -p wakeshot-windows --all-targets` with
`-D warnings` for both targets; `cargo test --locked -p wakeshot-windows -- power::system:: clock::`. Record each
outcome.

- Check rule: `docs/plan.md` "How we work" step 2 and "Checks while other cards run" in this track's phase file. A
  failure in a file another card owns goes in the Log: wait and run again; if it is still there when this card's own
  work is done, list it under "Found outside scope". Never edit, format or `--fix` that file. Every cargo command takes
  `--locked`, and clippy runs once more without `--all-targets` (the libs as they ship; `CLAUDE.md` "Checks"). No
  `--workspace` or `--all` run here; the orchestrator runs those when it closes a wave.

## Out of scope

- Session notices (`p3-session`); what the core does with the events; the real event sink (integration).
- Any file but `power/system.rs` and `clock.rs`.

## Questions for Haik

Each with the worker's default.

## Found outside scope

Problems seen but not fixed.

1. **A lost system event is not mended by the tick.** `EventSink`'s contract says a failed post is logged and "the
   next safety tick recovers". The tick reads input idle, deadlines, the plan and liveness, but no lid, power source
   or suspend state, so a lost `Lid`, `Power`, `Suspending` or `Resumed` stays lost until Windows sends the next one.
   A post fails only when the loop's queue is full or gone, so this is rare. This card counts the losses
   (`Subscriptions::lost_posts()`); integration (`p6-int-tray-mode` or `p6-int-dispatch`) should log a change on each
   tick. If a lost lid event must be mended, the tick would need a fresh lid read (`lid_once`), a core or integration
   decision.
2. **`LoopRunners::input_idle_ms() -> u64`** (`p6-int-dispatch` deliverable 1) has no error path, while
   `clock::input_idle_ms()` returns `Result<u64, WinError>` (this crate's rule for every Windows call). The runner
   must choose a value for a failed read; 0 is the safe one (counts as input, so no hibernate), and the error should
   be logged.

## Log

- 2026-11-03 17:00: card written by `p3-plan-windows-power`.
- 2026-11-03 20:30: worker started; status running. Owns `power/system.rs`, `clock.rs` and this card only.
- 2026-11-03 20:45: read the card's sources and Microsoft's pages (register, unregister, callback routine,
  `SYSTEM_POWER_STATUS`, `GetLastInputInfo`, Power Setting GUIDs, `PBT_APMRESUMEAUTOMATIC`). The unregister pages say
  nothing about a callback during or after unregister, so callbacks find their receiver by id in a table (design
  below). Warm build of `wakeshot-windows` tests: ok, 37 s.
- 2026-11-03 21:00: `power/system.rs` and `clock.rs` written and formatted; `cargo test -- power::system:: clock::`:
  26 passed, 2 ignored. Real tests (lid open): 2 passed.
- 2026-11-03 21:05: probe test (temporary, removed): the first lid callback comes on another thread after register
  returns; `PowerSettingUnregisterNotification` waited 300 ms for a callback that slept 300 ms. Recorded in
  `docs/research.md` "System events checks"; the design does not rely on either.
- 2026-11-03 21:10: first clippy round failed in `session.rs:141` (`p3-session`'s file, "unnecessary closure used with
  `bool::then`"); nothing in this card's files. Rerun minutes later: all four clippy runs clean, the `session.rs`
  error gone.
- 2026-11-03 21:20: all checks rerun after the last comment edits; build folder deleted. Status done.

## Decisions

Each with its reason and when to change it.

1. **Callbacks find their receiver by id in a static table, never through a pointer.** The context value is an id.
   Drop unregisters each handle without holding the table lock, removes the entry, then waits until no callback still
   holds it. Why: Microsoft's pages say nothing about callbacks during or after unregister, so this is safe whatever
   Windows does; the lock rule avoids a deadlock, since an unregister waited for a running callback here. Change it
   when: never needed for safety; a plain context pointer is fine only if Microsoft documents that no callback runs
   after unregister returns.
2. **`DEVICE_NOTIFY_SUBSCRIBE_PARAMETERS` is leaked**, 16 bytes per registration: 5 per tray run, 1 per `lid_once`.
   Why: not documented whether Windows copies it. Change it when: Microsoft documents a copy, or `lid_once` runs often
   enough for the bytes to matter (it runs once per hibernate attempt).
3. **A failed registration does not fail `start`.** It is listed in `failures()` and the rest keep posting, so a
   failed plan notification never costs the lid. Change it when: the core must act on a missing notification (for
   example, treat a missing lid registration as "lid unknown").
4. **`Event::Power` waits for the power source**, then each post carries the last value of the other field; posts
   happen under the power lock, so two callback threads cannot post readings out of order. Change it when: the core
   wants the battery percent before the source is known.
5. **Failed posts are counted, not logged** (`lost_posts()`): a callback must only post, and the log writer does file
   I/O. Change it when: the log gets a non-blocking path a callback may use.
6. **`lid_once` returns the first value** of its fresh registration, `Some(true)` for closed (the same sense as
   `Event::Lid { closed }`); a failed registration or a timeout is `None`. `timeout_ms` is `u64`. Change it when: a
   hibernate ever runs on a lid opened during the check (then wait a few ms for a later value).
7. **`power_status`**: `ACLineStatus` 255 (unknown) counts as AC, so an unknown source never lets the battery floor
   hibernate; the percent is `None` when `BatteryFlag` has bit 128 (no battery, or unknown) or the percent is over
   100. Change it when: a caller needs "unknown" kept apart (then return an enum).
8. **`input_idle_ms` reads the last input before the tick** and counts a gap over 2^31 ms as 0, because Microsoft
   says the input tick need not grow and `SendInput` supplies its own. Cost: a real idle over 24.8 days reads as 0.
   Change it when: idle times that long must count.
9. **Decoders take exactly 4 bytes for a DWORD and 16 for the GUID**; an undocumented value gives no event. Change it
   when: Windows is seen sending a longer payload.
10. **The registration seam (`Registrar`) is private**, for this module's tests only. Change it when: another crate
    must fake the subscriptions (then a `test-support` fake, through the orchestrator for `testing.rs`).

## Result

**Status: done.**

Done:

- `crates/wakeshot-windows/src/power/system.rs`: `Subscriptions::start(sink: Arc<dyn EventSink>) -> Subscriptions`
  (lid, power source, battery percent, active plan, suspend and resume; `failures() -> &[WinError]`,
  `lost_posts() -> u64`; drop unregisters all, and nothing posts after it returns); `lid_once(timeout_ms: u64) ->
  Option<bool>` (`Some(true)` closed, `Some(false)` open, `None` unknown); `power_status() -> Result<(bool,
  Option<u8>), WinError>` (on AC, battery percent); pure decoders for each payload; the callbacks only decode and post.
- `crates/wakeshot-windows/src/clock.rs`: `input_idle_ms() -> Result<u64, WinError>` (`GetLastInputInfo` against
  `GetTickCount`, wrapping; a last input just after now is 0).
- `docs/research.md`: new section "System events checks" (what the pages say, the probe, first values).

For `p3-hibernate`: call `power::system::lid_once(2000)`, `power::system::power_status()` and
`clock::input_idle_ms()`. Here `lid_once` answers in well under 1 ms; with no lid it waits the whole timeout.

Checks (all on this laptop, elevated, lid open, AC):

| Check | Outcome |
|---|---|
| `rustfmt --edition 2024 --check` on both owned files | clean |
| `cargo clippy --locked -p wakeshot-windows --all-targets --target <t> -- -D warnings`, both targets | clean (first round blocked by `session.rs`, see Log) |
| same without `--all-targets`, both targets | clean |
| `cargo test --locked -p wakeshot-windows -- power::system:: clock::` | 26 passed, 2 ignored |
| `cargo test --locked -p wakeshot-windows -- --ignored --test-threads=1 power::system::` | 2 passed (`real_registration_gives_first_values`, `real_lid_once_returns_open_here`) |

The card's named tests all pass: `decode_lid`, `decode_acdc`, `decode_battery_pct`, `decode_plan_guid`,
`decode_suspend_resume`, `lid_once_times_out`, `input_idle_wraps`, `input_idle_reads`, and the two `real_` tests.
Added: routing by GUID, the payload block reader, every-watch-in-order, each event, the power merge, no battery
reading, a failed registration, a failed post, drop with later callbacks, drop during a running callback, `lid_once`
with a value before register returns, from another thread, and with a failed registration, `power_status` flags and a
real read.

Open items:

- `p3-real-checks` step 2 reruns the two `real_` tests.
- `input_idle_reads` and `power_status_reads` run in plain `cargo test`, as the card asks; whether `GetLastInputInfo`
  succeeds on a GitHub runner is not verified.
- Found outside scope 1 and 2 are for integration.
