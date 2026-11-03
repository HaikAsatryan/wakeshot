# Cron next times in local time, and the PC's time zone

Phase: 4. Status: done. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 100K tokens.
Runs after: the contracts card (it creates `sources/cron.rs` and `localtime.rs`). Parallel with: every other card of
this track (`docs/phases/04-sources.md`, "Files each card owns") and every card of tracks 2, 3, 5, 6.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p4-cron\target`. Uses the real laptop: no (reads time zone data only;
never changes the PC's zone or clock).

## Goal

Give the Claude parser (and Copilot's, after v1) the next run time of a cron job, in the PC's local time with daylight saving,
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

None.

## Found outside scope

Problems seen but not fixed.

1. **`p4-claude-transcript` deliverable 2 names the wrong call.** It says "`Cron::next_after(last fire)` plus
   `claude_jitter_ms`". With Claude Code's real rule that is wrong for `*/5` jobs, which fire 285 s after the last
   fire, not near a 5 minute mark. It should call `ClaudeScheduler::DEFAULT.next_fire_ms(&ClaudeJob { .. })` (see
   Result). For a resumed session the live data fit `from` = the resume start (inference, `docs/research.md` "Cron
   checks").
2. **`docs/research/claude-code-signals.md` "Scheduler facts"** says a `*/N` job with a 5 minute period "runs 15 s
   early (4 min 45 s gap)". The exact rule is in `docs/research.md` "Cron checks": 285 s after the last fire or the
   creation; "15 s early" holds only from a 5 minute mark. Not my file.
3. **For the Claude parsing review: the jitter constants are a remote flag.** Claude Code reads them from
   `tengu_kairos_cron_config` and falls back to the built-in ones. If the flag ever makes jobs later than computed
   here, a hold ends 2 minutes (the core's margin) after the computed time and the PC may sleep before the job fires;
   the schema caps lateness at 30 minutes, and nothing here can hold forever. A safer reader rule (for example, hold
   to the run plus the 30 minute cap when the first `nextFireAt` does not match the computed one) is the transcript
   card's call.
4. **Card check too tight.** "The next fires in the log come 0.1 to 1 s after the computed times": the Phase 0 log has
   a fire 31 ms after (01:45:11.895 against 01:45:11.864), fixture 03 one 54 ms after. The tests use more than 0 and at
   most 1 s.
5. **Two copies of civil date math.** `holds::file` has private `days_from_civil` and `civil_from_days`; this card
   wrote its own, as the card said. A shared one could live in `time` later (contracts change). The Win32 error helper
   is also written three times (`log.rs`, `process.rs`, `localtime.rs`); a shared one in `error.rs` would do.

## Log

- 2026-11-03 16:53: card written by `p4-plan-sources`.
- 2026-11-03 21:20: worker started. Status in progress. Owned files: `sources/cron.rs`, `localtime.rs`, this card.
  Orchestrator note: Codex and Copilot moved to after v1; in v1 only the Claude Code source uses this API.
- 2026-11-03 21:36: read Claude Code 2.1.317's own cron code in `claude.exe` (read only): parser `d5`, search `FYt`,
  next fire `Zkn` (recurring) and `W7o` (one-shot), constants `jJ`, remote flag `tengu_kairos_cron_config`. Finding:
  the `*/5` rule is not "15 s early" on the clock; it fires `from + 285 s` (from = last fire or creation), which the
  live fixtures 09 and 10 show. So the card's `claude_jitter_ms(id, cron, run_utc, zone)` cannot express it; design
  change: one call that gives the whole next fire from `from`. Windows zone call cost measured (read only): about
  0.3 us per call within one year, about 23 us when the year changes.
- 2026-11-03 21:47: `sources/cron.rs` written: 28 unit tests pass (`cargo test --locked -p wakeshot-core --lib
  sources::cron`). Differential check in the scratch folder: Claude Code's own functions copied out of `claude.exe`,
  run in Node 26 with `TZ` set, against this module with the same zone offsets: 200,000 parse cases and 200,000
  search cases (next run, recurring fire, one-shot fire) in 10 zones, 0 mismatches. Next: `localtime.rs`.
- 2026-11-03 21:50: `localtime.rs` written: 6 tests pass. Windows zone data puts the 2026 Pacific changes on 8 March
  and 1 November at 02:00 local, as the card said. Clippy clean on both targets, with and without `--all-targets`.
- 2026-11-03 21:53: research notes added (`docs/research.md`, "Cron checks"). Every acceptance check run (Result).
  Scratch scripts and the build folder deleted. Status done.

## Result

Status: done. Needs Haik: no.

**What was done.** Both files copy Claude Code 2.1.317's own scheduler, read in its bundled code (`d5`, `FYt`,
`Zkn`, `W7o`; `docs/research.md`, "Cron checks"), not textbook cron, so a due time here is the one Claude Code picks.

- `crates/wakeshot-core/src/sources/cron.rs` (pure, no crate, 372 code lines without comments and blank lines; track
  note 1's trigger is about 400):
  - `Cron::parse(&str) -> Option<Cron>`: the card's syntax, with JS white space and Claude Code's set-based "restricted"
    test for the both-day-fields rule (`1-31` counts as `*`, `*/2` as restricted).
  - `Cron::next_after(utc_ms, &dyn LocalOffset) -> Option<i64>`: `FYt` for recurring jobs, step by step with JS
    `Date` setter semantics on local time. Skipped local times never run; a repeated one runs at its first pass. At
    most 527,040 steps (Claude Code's cap) and about 5 years: `0 0 31 2 *` gives `None` in under 10,000 zone calls.
  - `Cron::period_ms(at, zone)`, `trait LocalOffset`, `FixedOffset(i32)`, civil date math with tests.
  - `ClaudeScheduler` (the constants `jJ`, pub fields, `ClaudeScheduler::DEFAULT`) and
    `ClaudeScheduler::next_fire_ms(&ClaudeJob, zone) -> Option<i64>`, with
    `ClaudeJob { id: &str, cron: &Cron, recurring: bool, from_ms: i64 }`.
- `crates/wakeshot-windows/src/localtime.rs`: `SystemZone::current()` (re-reads the PC's zone at each call, about
  0.3 us, so a zone change applies at once), `SystemZone::named(&DYNAMIC_TIME_ZONE_INFORMATION)`, `impl LocalOffset`
  through `SystemTimeToTzSpecificLocalTimeEx` (that year's rules), and `take_error() -> Option<WinError>`: a failed
  call gives the zone's current offset and keeps the first error for the caller to log; nothing panics.

**Design change from the card, and why.** The card asked for `claude_jitter_ms(id, cron, run_utc, zone) -> i64`, added
to `next_after`. Claude Code's code shows the `*/5` rule is `from + 285 s` (from = the last fire, or the creation),
not "the 5 minute mark minus 15 s"; live fixtures 09 and 10 confirm it (285 s after creation, then 285 s after each
fire, off the 5 minute marks). A shift added to a run time cannot express that, so there is one call for the whole
next fire, recurring and one-shot alike. One-shots are in (the card said "no shift here") because a `/clear` carry
line has no `nextFireAt`, and `W7o` is 10 lines. The result is the first whole ms Claude Code's tick can fire at (the
ceiling of its float time). Change it when Claude Code's scheduler changes (`p4-live-claude` reruns the live checks).

**For `p4-claude-transcript` (the API it builds on).**

- Due time of an armed job without an exact time in the transcript: `ClaudeScheduler::DEFAULT.next_fire_ms(&ClaudeJob
  { id, cron: &cron, recurring, from_ms })`, where `from_ms` is the last `scheduled_task_fire` line's time for a
  recurring job that has fired, else the job's creation time (`session_cron_carry` gives `createdAt`); one-shots
  always use the creation time. After a resume the live data fit `from_ms` = the resume start (not verified in code).
- When the transcript has `nextFireAt` or `scheduledFor`, use it (ceil of the float); it already has the jitter.
- `Cron::parse` returns `None` for exactly what Claude Code rejects (200,000 random texts agree), so a `None` there
  means a new Claude Code syntax: track note 5.
- The zone: `wakeshot_windows::localtime::SystemZone::current()` as `Box<dyn LocalOffset + Send>` (it is also
  `Sync`); log `take_error()` now and then. A daily job's fire took 3.8 ms in a debug build with it.
- `ClaudeScheduler::recurring_max_age_ms` holds the 7 day expiry, so the constants live in one place.

**Files changed.** `crates/wakeshot-core/src/sources/cron.rs`, `crates/wakeshot-windows/src/localtime.rs`,
`docs/research.md` (new section "Cron checks", addition only), this card.

**Checks and outcomes.**

- `rustfmt --edition 2024 --check` on both owned files: clean.
- `cargo clippy --locked -p wakeshot-core -p wakeshot-windows --all-targets --target <t> -- -D warnings`, and the same
  without `--all-targets`, for `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`: clean, all four.
- `cargo test --locked -p wakeshot-core`: 475 unit tests and 10 integration tests pass. New, in `sources::cron::tests`
  (28): `every_syntax_form_parses`, `day_of_week_7_is_sunday`, `white_space_is_javascript_white_space`,
  `every_rejected_form_gives_none`, `the_minute_step_form_is_exact_text`,
  `civil_dates_at_month_year_and_leap_edges`, `wall_fields_before_1970`, `next_after_is_strictly_after`,
  `next_after_crosses_hour_day_month_and_year_ends` (29 February included), `next_after_is_local_time`,
  `either_day_field_matches_when_both_are_restricted` (`0 9 15 * 1`), `an_impossible_date_gives_none_fast`,
  `times_outside_the_js_range_give_none`, `period_is_the_gap_between_the_next_two_runs`,
  `the_made_up_zones_change_when_they_should`, `a_skipped_local_time_does_not_run`,
  `a_repeated_local_time_runs_once_at_its_first_pass`, `a_one_shot_runs_at_the_second_pass_too`,
  `the_job_fraction_reads_like_javascript_parse_int`, `live_test_3_next_fire_at_and_fires` (the `*/1` job: computed
  time is the ceiling of the logged `nextFireAt`; five fires 31 to 939 ms after the computed times),
  `live_fixtures_next_fire_times` (fixtures 03, 06, 09, 10, 11 to the ms), `live_one_shot_fires` (Phase 0 tests 5 and
  14: the loop fallback fired 489 ms after its computed time, 35.9 s early),
  `a_five_minute_step_fires_15_s_short_of_5_minutes_after_from`,
  `recurring_jitter_is_a_share_of_the_gap_capped_at_30_minutes`,
  `a_one_shot_on_00_or_30_runs_early_but_not_before_its_creation`, and properties `civil_dates_round_trip`,
  `next_after_is_the_first_match` (random crons and offsets against a day-by-day search: `next_after(t) > t` and
  nothing between matches), `next_after_near_daylight_saving_changes` (four made-up zones: US, southern, 30 minute,
  midnight changes; recurring and one-shot against a minute-by-minute search).
- `cargo test --locked -p wakeshot-windows localtime::`: 6 pass. New: `the_pc_zone_matches_get_time_zone_information`
  (this PC: "Caucasus Standard Time", +240), `pacific_changes_on_the_dates_in_windows_zone_data` (-480 and -420 one ms
  before and at each change), `other_years_follow_their_own_rules` (2006 and 2027 Pacific, Sydney),
  `cron_times_skip_and_repeat_with_windows_rules`, `a_daily_claude_job_with_the_pc_zone`,
  `a_failed_call_gives_the_current_offset_and_keeps_the_error`.
- `cargo tree --locked -p wakeshot-core -e normal`: only `serde` and `serde_json` with their own deps, as before; no
  new crate.
- Grep of `crates/` for `SetTimeZoneInformation`, `SetDynamicTimeZoneInformation`, `SetSystemTime`: none.
- Beyond the card: the differential check against Claude Code's own JS (Log 21:47): 0 mismatches in 400,000 cases.
- No failure in any other card's file came up in these runs.

**Open items.** Not verified (in research): that Bun (JavaScriptCore) resolves DST edges like Node did; what the
remote flag holds today; Windows zone data against IANA data for zones where they differ; what Windows returns with
automatic daylight saving off. The findings above for the transcript card and the parsing review. The scratch
build folder is deleted.
