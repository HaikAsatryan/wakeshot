# System events: lid, AC and battery, active plan, suspend and resume, input idle

Phase: 3. Status: ready. Needs Haik: no. Real laptop: no (notification registrations and reads only). Agent:
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

## Log

- 2026-11-03 17:00: card written by `p3-plan-windows-power`.

## Result

What was done, files changed, checks and outcomes, open items.
