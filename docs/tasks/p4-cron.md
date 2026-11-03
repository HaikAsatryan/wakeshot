# Cron next times in local time, and the PC's time zone

Phase: 4. Status: ready. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 100K tokens.
Runs after: the contracts card (it creates `sources/cron.rs` and `localtime.rs`). Parallel with: every other card of
this track (`docs/phases/04-sources.md`, "Files each card owns") and every card of tracks 2, 3, 5, 6.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p4-cron\target`. Uses the real laptop: no (reads time zone data only;
never changes the PC's zone or clock).

## Goal

Give the Claude and Copilot parsers the next run time of a cron job, in the PC's local time with daylight saving,
with Claude's jitter, without a date crate and without OS calls in the core.

## Read first

- `docs/phases/04-sources.md`: track notes 1, 2, 4, 5, 9.
- `docs/research.md`: "Sources track plan checks" (cron syntax, jitter, crates).
- `docs/research/claude-code-signals.md`: "Timers", "Scheduler facts" (jitter formula), live tests 3, 5, 14.
- `spikes/claude-watch/fixtures/live-tests.log` lines 145 to 146 and 178 to 189 (a `*/1` job: `nextFireAt` and fires).

## Deliverables

1. `crates/wakeshot-core/src/sources/cron.rs` (pure):
   - `Cron::parse(&str) -> Option<Cron>`: 5 fields; `*`, numbers, `*/n`, `a-b`, `a-b/n`, lists of these; day of week
     0 to 7 (0 and 7 Sunday); no names, `L`, `W`, `?` (they give `None`). Vixie rule: when both day fields are
     restricted, a day matches if either matches.
   - `trait LocalOffset { fn offset_minutes(&self, utc_ms: i64) -> i32; }` and `FixedOffset(i32)` for tests and as a
     fallback.
   - `Cron::next_after(&self, utc_ms: i64, zone: &dyn LocalOffset) -> Option<i64>`: the first matching local minute
     strictly after `utc_ms`, as UTC ms. A local time that does not exist (spring forward) is skipped; one that exists
     twice (fall back) fires once, at the first. Bounded search (at most about 5 years), so an impossible date such as
     `0 0 31 2 *` gives `None` without looping.
   - `Cron::period_ms(&self, at: i64, zone) -> Option<i64>`: the gap between the next two runs after `at`, for rule 6
     ("a period of 1 hour or less holds").
   - `claude_jitter_ms(id: &str, cron, run_utc, zone) -> i64` for recurring jobs: late by
     `min(frac(id) * 0.5 * gap to the next run, 30 min)`, `frac(id) = parseInt(id[0..8], 16) / 2^32`; `*/5 * * * *`
     exactly: 15 s early. One-shots: no shift here (their exact time comes from the transcript).
   - Civil date math (days from civil and back) written here, with tests at month, year and leap year edges.
2. `crates/wakeshot-windows/src/localtime.rs`: `SystemZone` implementing `LocalOffset` for the PC's current zone with
   its daylight saving rules for any year (`GetDynamicTimeZoneInformation`, `SystemTimeToTzSpecificLocalTimeEx`), and
   `SystemZone::named(&DYNAMIC_TIME_ZONE_INFORMATION)` so tests can use another zone. A failed call gives the zone's
   current bias (logged by the caller), never a panic. Feature `Win32_System_Time` comes from the contracts card.

Tests first:

- Core: every syntax form and every rejected form; `next_after` across minute, hour, day, month and year ends and
  29 February; the vixie OR rule (`0 9 15 * 1`); `0 0 31 2 *` gives `None` fast; a made-up zone with a spring and a fall
  change (a test `LocalOffset`) for skipped and doubled local times; the live-test check: for the `*/1` job of live
  test 3, `next_after` plus `claude_jitter_ms` equals the logged `nextFireAt` to the millisecond, and the next fires in
  the log come 0.1 to 1 s after the computed times; `*/5` is 15 s early; a daily job's jitter is capped at 30 minutes;
  a `proptest` property: `next_after(t) > t` and nothing between `t` and it matches.
- Windows: `SystemZone` for the PC's zone equals the offset `GetTimeZoneInformation` reports now; for "Pacific
  Standard Time" (found with `EnumDynamicTimeZoneInformation`, read only) the offsets on 2026-03-08 and 2026-11-01 just
  before and after the change are -480 and -420 minutes as Microsoft's zone data says (check the dates in the zone
  data, do not trust this card's dates).

## Acceptance checks

- `rustfmt --edition 2024 --check <owned files>`; `cargo clippy --locked -p wakeshot-core -p wakeshot-windows
  --all-targets --target x86_64-pc-windows-msvc -- -D warnings` and the same for `aarch64-pc-windows-msvc`; `cargo test
  --locked -p wakeshot-core` and `cargo test --locked -p wakeshot-windows localtime::`: clean as the check rule below
  says, new tests named in the Result.
- `cargo tree -p wakeshot-core -e normal` lists no new crate.
- No test changes the PC's time zone or clock (grep for `SetTimeZoneInformation`, `SetDynamicTimeZoneInformation`,
  `SetSystemTime`: none).
- Check rule: `docs/plan.md` "How we work" step 2 and "Checks while other cards run" in this track's phase file. A
  failure in a file another card owns goes in the Log: wait and run again; if it is still there when this card's own
  work is done, list it under "Found outside scope". Never edit, format or `--fix` that file. Every cargo command takes
  `--locked`, and clippy runs once more without `--all-targets` (the libs as they ship; `CLAUDE.md` "Checks"). No
  `--workspace` or `--all` run here; the orchestrator runs those when it closes a wave.

## Out of scope

- Reading transcripts or Copilot events (`p4-claude-transcript`, `p4-copilot-source`). Deciding what holds (core
  track). Any file not listed above.

## Questions for Haik

Each with the worker's default.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-03 16:53: card written by `p4-plan-sources`.

## Result

What was done, files changed, checks and outcomes, open items.
