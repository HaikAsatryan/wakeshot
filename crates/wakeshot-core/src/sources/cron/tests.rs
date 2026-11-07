use std::cell::Cell;

use proptest::prelude::*;

use super::*;

/// Yerevan, where the live tests ran: UTC+4, no daylight saving.
const YEREVAN: FixedOffset = FixedOffset(240);

fn cron(text: &str) -> Cron {
    Cron::parse(text).unwrap_or_else(|| panic!("{text:?} should parse"))
}

/// UTC ms of a UTC date and time.
fn utc(year: i64, month: u32, date: u32, hour: i64, minute: i64) -> i64 {
    days_from_civil(year, month, date) * DAY_MS + hour * HOUR_MS + minute * MINUTE_MS
}

/// UTC ms of a local date and time at a fixed offset.
fn at(zone: FixedOffset, year: i64, month: u32, date: u32, hour: i64, minute: i64) -> i64 {
    utc(year, month, date, hour, minute) - i64::from(zone.0) * MINUTE_MS
}

fn set(values: &[u32]) -> u64 {
    values.iter().fold(0, |set, v| set | (1 << v))
}

fn range(low: u32, high: u32) -> u64 {
    (low..=high).fold(0, |set, v| set | (1 << v))
}

/// A made-up zone with yearly daylight saving rules, so tests can cross changes in any year. Each change is on the
/// nth Sunday of a month at a local time (minutes after midnight, on the clock in effect before the change).
struct RuleZone {
    standard: i32,
    daylight: i32,
    start: (u32, i64, i64),
    end: (u32, i64, i64),
}

impl RuleZone {
    fn change(year: i64, (month, nth, minute): (u32, i64, i64), offset: i32) -> i64 {
        let first = days_from_civil(year, month, 1);
        let sunday = first + (7 - (first + 4).rem_euclid(7)) % 7 + 7 * (nth - 1);
        sunday * DAY_MS + minute * MINUTE_MS - i64::from(offset) * MINUTE_MS
    }

    fn starts(&self, year: i64) -> i64 {
        Self::change(year, self.start, self.standard)
    }

    fn ends(&self, year: i64) -> i64 {
        Self::change(year, self.end, self.daylight)
    }
}

impl LocalOffset for RuleZone {
    fn offset_minutes(&self, utc_ms: i64) -> i32 {
        let local = utc_ms + i64::from(self.standard) * MINUTE_MS;
        let year = civil_from_days(local.div_euclid(DAY_MS)).0;
        let (start, end) = (self.starts(year), self.ends(year));
        let daylight = if start < end {
            start <= utc_ms && utc_ms < end
        } else {
            utc_ms >= start || utc_ms < end
        };
        if daylight {
            self.daylight
        } else {
            self.standard
        }
    }
}

/// US Pacific rules since 2007: 02:00 on the second Sunday of March to 02:00 on the first Sunday of November.
const PACIFIC: RuleZone = RuleZone {
    standard: -480,
    daylight: -420,
    start: (3, 2, 120),
    end: (11, 1, 120),
};

/// Southern: daylight saving runs across the new year (Sydney's rules).
const SYDNEY: RuleZone = RuleZone {
    standard: 600,
    daylight: 660,
    start: (10, 1, 120),
    end: (4, 1, 180),
};

/// A 30 minute change (Lord Howe Island's rules).
const LORD_HOWE: RuleZone = RuleZone {
    standard: 630,
    daylight: 660,
    start: (10, 1, 120),
    end: (4, 1, 120),
};

/// Changes at midnight, so a whole first hour of a day is skipped or repeated (Chile's style).
const MIDNIGHT: RuleZone = RuleZone {
    standard: -240,
    daylight: -180,
    start: (9, 1, 0),
    end: (4, 1, 0),
};

/// Counts calls, to show a search stays short.
struct Counting<'a> {
    zone: &'a dyn LocalOffset,
    calls: Cell<u32>,
}

impl LocalOffset for Counting<'_> {
    fn offset_minutes(&self, utc_ms: i64) -> i32 {
        self.calls.set(self.calls.get() + 1);
        self.zone.offset_minutes(utc_ms)
    }
}

// Parser.

#[test]
fn every_syntax_form_parses() {
    let c = cron("*/15 0 1-5 1,6,12 1-5/2");
    assert_eq!(c.minutes, set(&[0, 15, 30, 45]));
    assert_eq!(c.hours, set(&[0]));
    assert_eq!(c.days, range(1, 5));
    assert_eq!(c.months, set(&[1, 6, 12]));
    assert_eq!(c.weekdays, set(&[1, 3, 5]));

    let c = cron("* * * * *");
    assert_eq!(
        (c.minutes, c.hours, c.days, c.months, c.weekdays),
        (
            range(0, 59),
            range(0, 23),
            range(1, 31),
            range(1, 12),
            range(0, 6)
        )
    );
    assert_eq!(cron("1-30/5 * * * *").minutes, set(&[1, 6, 11, 16, 21, 26]));
    assert_eq!(cron("1,15,30,15 * * * *").minutes, set(&[1, 15, 30]));
    assert_eq!(
        cron("1-10/3,*/20,59 * * * *").minutes,
        set(&[1, 4, 7, 10, 0, 20, 40, 59])
    );
    assert_eq!(cron("007 * * * *").minutes, set(&[7]));
    // A step past the range keeps only the first value, as in JS.
    assert_eq!(cron("*/99999999999999999999 * * * *").minutes, set(&[0]));
    assert_eq!(cron("5-50/60 * * * *").minutes, set(&[5]));
    assert_eq!(cron("* */6 * * *").hours, set(&[0, 6, 12, 18]));
    assert_eq!(cron("* * */10 * *").days, set(&[1, 11, 21, 31]));
    assert_eq!(cron("* * * */4 *").months, set(&[1, 5, 9]));
}

#[test]
fn day_of_week_7_is_sunday() {
    assert_eq!(cron("0 0 * * 7").weekdays, set(&[0]));
    assert_eq!(cron("0 0 * * 0").weekdays, set(&[0]));
    assert_eq!(cron("0 0 * * 5-7").weekdays, set(&[5, 6, 0]));
    assert_eq!(cron("0 0 * * 7-7").weekdays, set(&[0]));
    assert_eq!(cron("0 0 * * 0-7").weekdays, range(0, 6));
    assert_eq!(cron("0 0 * * 1-7/2").weekdays, set(&[1, 3, 5, 0]));
    assert_eq!(cron("0 0 * * */2").weekdays, set(&[0, 2, 4, 6]));
}

#[test]
fn white_space_is_javascript_white_space() {
    for text in [
        "  0  9 * * *  ",
        "0\t9\n*\r* *",
        "\u{a0}0 9 * * *",
        "0\u{3000}9 * * *",
        "\u{feff}0 9 * * *",
        "0 9 * * *\u{2028}",
    ] {
        assert_eq!(
            Cron::parse(text).map(|c| c.hours),
            Some(set(&[9])),
            "{text:?}"
        );
    }
    // Rust white space but not JS white space.
    assert_eq!(Cron::parse("\u{85}0 9 * * *"), None);
}

#[test]
fn every_rejected_form_gives_none() {
    for text in [
        "",
        "   ",
        "* * * *",
        "* * * * * *",
        "0 9 * * MON",
        "0 9 * JAN *",
        "0 9 * * mon-fri",
        "0 9 L * *",
        "0 9 15W * *",
        "0 9 ? * *",
        "0 9 * * ?",
        "0 9 * * 5L",
        "0 9 * * 1#2",
        "@daily",
        "60 * * * *",
        "* 24 * * *",
        "* * 0 * *",
        "* * 32 * *",
        "* * * 0 *",
        "* * * 13 *",
        "* * * * 8",
        "* * * * 0-8",
        "*/0 * * * *",
        "*/ * * * *",
        "*5 * * * *",
        "*-5 * * * *",
        "**/5 * * * *",
        "5-1 * * * *",
        "1-5/0 * * * *",
        "1-5/ * * * *",
        "1- * * * *",
        "-1 * * * *",
        "1-2-3 * * * *",
        "1-2/3/4 * * * *",
        "1/5 * * * *",
        "1,,2 * * * *",
        "1, * * * *",
        ",1 * * * *",
        "+1 * * * *",
        "1.5 * * * *",
        "0x1 * * * *",
        "\u{661} * * * *",
        "99999999999 * * * *",
        "1-99999999999 * * * *",
    ] {
        assert_eq!(Cron::parse(text), None, "{text:?}");
    }
}

#[test]
fn the_minute_step_form_is_exact_text() {
    assert!(cron("*/5 * * * *").minute_step);
    assert!(cron("*/1 * * * *").minute_step);
    assert!(cron("*/005 * * * *").minute_step);
    for text in [
        " */5 * * * *",
        "*/5  * * * *",
        "*/5 * * * * ",
        "*/5\t* * * *",
        "0-59/5 * * * *",
        "* * * * *",
        "*/5 */1 * * *",
        "*/5,7 * * * *",
    ] {
        assert!(!cron(text).minute_step, "{text:?}");
    }
}

// Civil date math.

#[test]
fn civil_dates_at_month_year_and_leap_edges() {
    // Day numbers and weekdays from Python's `datetime` (0 is Sunday).
    for (y, m, d, days, weekday) in [
        (1970, 1, 1, 0, 4),
        (1969, 12, 31, -1, 3),
        (2000, 2, 29, 11_016, 2),
        (2000, 3, 1, 11_017, 3),
        (1900, 3, 1, -25_508, 4),
        (2024, 2, 29, 19_782, 4),
        (2026, 11, 3, 20_760, 2),
        (2027, 1, 1, 20_819, 5),
        (2028, 2, 29, 21_243, 2),
        (1600, 3, 1, -135_080, 3),
        (2100, 3, 1, 47_541, 1),
        (2400, 2, 29, 157_113, 2),
    ] {
        assert_eq!(days_from_civil(y, m, d), days, "{y}-{m}-{d}");
        assert_eq!(civil_from_days(days), (y, m, d), "{days}");
        assert_eq!(Wall::at(days * DAY_MS).weekday, weekday, "{y}-{m}-{d}");
    }
    // 1900 and 2100 have no 29 February: the day after the 28th is 1 March.
    assert_eq!(
        civil_from_days(days_from_civil(1900, 2, 28) + 1),
        (1900, 3, 1)
    );
    assert_eq!(
        civil_from_days(days_from_civil(2100, 2, 28) + 1),
        (2100, 3, 1)
    );
    assert_eq!(
        civil_from_days(days_from_civil(2028, 2, 28) + 1),
        (2028, 2, 29)
    );
    assert_eq!(
        civil_from_days(days_from_civil(2026, 12, 31) + 1),
        (2027, 1, 1)
    );
    assert_eq!(
        civil_from_days(days_from_civil(2026, 4, 30) + 1),
        (2026, 5, 1)
    );
    // The live log's 01:41:00 local (UTC+4) is 1793655660000.
    assert_eq!(utc(2026, 11, 2, 21, 41), 1_793_655_660_000);
}

#[test]
fn wall_fields_before_1970() {
    let w = Wall::at(-1);
    assert_eq!(
        (w.year, w.month, w.date, w.hour, w.minute),
        (1969, 12, 31, 23, 59)
    );
    assert_eq!(w.weekday, 3);
}

// Next run, no daylight saving.

#[test]
fn next_after_is_strictly_after() {
    let every = cron("* * * * *");
    let noon = at(YEREVAN, 2026, 11, 3, 12, 0);
    assert_eq!(every.next_after(noon, &YEREVAN), Some(noon + MINUTE_MS));
    assert_eq!(
        every.next_after(noon + 30_000, &YEREVAN),
        Some(noon + MINUTE_MS)
    );
    assert_eq!(every.next_after(noon - 1, &YEREVAN), Some(noon));
}

#[test]
fn next_after_crosses_hour_day_month_and_year_ends() {
    let z = YEREVAN;
    let cases = [
        ("0 * * * *", (2026, 11, 3, 12, 59), (2026, 11, 3, 13, 0)),
        ("0 0 * * *", (2026, 11, 3, 23, 59), (2026, 11, 4, 0, 0)),
        ("30 23 * * *", (2026, 11, 3, 23, 30), (2026, 11, 4, 23, 30)),
        ("0 0 1 * *", (2026, 1, 31, 12, 0), (2026, 2, 1, 0, 0)),
        ("0 0 1 * *", (2026, 4, 30, 23, 59), (2026, 5, 1, 0, 0)),
        ("0 0 1 1 *", (2026, 12, 31, 23, 59), (2027, 1, 1, 0, 0)),
        (
            "59 23 31 12 *",
            (2026, 12, 31, 23, 59),
            (2027, 12, 31, 23, 59),
        ),
        ("0 0 29 2 *", (2026, 11, 3, 0, 0), (2028, 2, 29, 0, 0)),
        ("0 0 29 2 *", (2028, 2, 29, 0, 0), (2032, 2, 29, 0, 0)),
        ("0 0 29 2 *", (2096, 2, 1, 0, 0), (2096, 2, 29, 0, 0)),
        ("0 12 28 2 *", (2100, 1, 1, 0, 0), (2100, 2, 28, 12, 0)),
        ("0 0 1 3 *", (2100, 2, 28, 0, 0), (2100, 3, 1, 0, 0)),
        ("*/5 * * * *", (2026, 11, 3, 12, 3), (2026, 11, 3, 12, 5)),
        ("0 9 * * 1-5", (2026, 11, 6, 9, 0), (2026, 11, 9, 9, 0)),
        ("0 9 * 12 *", (2026, 11, 3, 0, 0), (2026, 12, 1, 9, 0)),
    ];
    for (text, (y, mo, d, h, mi), (ny, nmo, nd, nh, nmi)) in cases {
        let from = at(z, y, mo, d, h, mi);
        assert_eq!(
            cron(text).next_after(from, &z),
            Some(at(z, ny, nmo, nd, nh, nmi)),
            "{text:?} after {y}-{mo}-{d} {h}:{mi}"
        );
    }
}

#[test]
fn next_after_is_local_time() {
    let daily = cron("0 9 * * *");
    // 10:00 in Yerevan is past 9:00, so the next run is tomorrow at 05:00 UTC.
    assert_eq!(
        daily.next_after(utc(2026, 11, 3, 6, 0), &YEREVAN),
        Some(utc(2026, 11, 4, 5, 0))
    );
    let pacific = FixedOffset(-480);
    assert_eq!(
        cron("0 0 * * *").next_after(utc(2026, 11, 3, 6, 0), &pacific),
        Some(utc(2026, 11, 3, 8, 0))
    );
    let nepal = FixedOffset(345);
    assert_eq!(
        cron("0 0 * * *").next_after(utc(2026, 11, 3, 0, 0), &nepal),
        Some(utc(2026, 11, 3, 18, 15))
    );
}

#[test]
fn either_day_field_matches_when_both_are_restricted() {
    let z = YEREVAN;
    let c = cron("0 9 15 * 1");
    // 2026-11-03 is a Tuesday: Monday the 9th, Sunday the 15th, Monday the 16th.
    let mut t = at(z, 2026, 11, 3, 0, 0);
    for date in [9, 15, 16, 23, 30] {
        t = c.next_after(t, &z).unwrap();
        assert_eq!(t, at(z, 2026, 11, date, 9, 0), "date {date}");
    }
    // One restricted field: only that one counts.
    let from = at(z, 2026, 11, 3, 0, 0);
    assert_eq!(
        cron("0 9 15 * *").next_after(from, &z),
        Some(at(z, 2026, 11, 15, 9, 0))
    );
    assert_eq!(
        cron("0 9 * * 1").next_after(from, &z),
        Some(at(z, 2026, 11, 9, 9, 0))
    );
    // A field whose set is full counts as `*`; `*/2` counts as restricted.
    assert_eq!(
        cron("0 9 1-31 * 1").next_after(from, &z),
        Some(at(z, 2026, 11, 9, 9, 0))
    );
    assert_eq!(
        cron("0 9 15 * 0-6").next_after(from, &z),
        Some(at(z, 2026, 11, 15, 9, 0))
    );
    assert_eq!(
        cron("0 9 */2 * 1").next_after(from, &z),
        Some(at(z, 2026, 11, 3, 9, 0))
    );
    assert_eq!(
        cron("0 9 */2 * 1").next_after(at(z, 2026, 11, 3, 9, 0), &z),
        Some(at(z, 2026, 11, 5, 9, 0))
    );
}

#[test]
fn an_impossible_date_gives_none_fast() {
    for text in ["0 0 31 2 *", "0 0 30 2 *", "0 0 31 4 *", "0 0 31 6,9,11 *"] {
        let zone = Counting {
            zone: &PACIFIC,
            calls: Cell::new(0),
        };
        assert_eq!(
            cron(text).next_after(utc(2026, 11, 3, 0, 0), &zone),
            None,
            "{text}"
        );
        assert!(
            zone.calls.get() < 10_000,
            "{text}: {} zone calls",
            zone.calls.get()
        );
    }
    // Around 2100, 29 February is 8 years apart, beyond the 5 year search.
    assert_eq!(
        cron("0 0 29 2 *").next_after(utc(2097, 1, 1, 0, 0), &YEREVAN),
        None
    );
}

#[test]
fn times_outside_the_js_range_give_none() {
    let every = cron("* * * * *");
    for t in [
        i64::MIN,
        i64::MAX,
        JS_DATE_LIMIT_MS,
        -JS_DATE_LIMIT_MS - 2 * MINUTE_MS,
    ] {
        assert_eq!(every.next_after(t, &YEREVAN), None, "{t}");
    }
    assert_eq!(
        every.next_after(JS_DATE_LIMIT_MS - MINUTE_MS, &YEREVAN),
        Some(JS_DATE_LIMIT_MS)
    );
    assert_eq!(every.next_after(-1, &FixedOffset(i32::MAX)), Some(0));
}

#[test]
fn period_is_the_gap_between_the_next_two_runs() {
    let z = YEREVAN;
    let now = at(z, 2026, 11, 3, 12, 2);
    assert_eq!(cron("*/5 * * * *").period_ms(now, &z), Some(5 * MINUTE_MS));
    assert_eq!(cron("* * * * *").period_ms(now, &z), Some(MINUTE_MS));
    assert_eq!(cron("0 * * * *").period_ms(now, &z), Some(HOUR_MS));
    assert_eq!(cron("0 9 * * *").period_ms(now, &z), Some(DAY_MS));
    // Friday 9:00, then Monday 9:00.
    assert_eq!(
        cron("0 9 * * 1-5").period_ms(at(z, 2026, 11, 6, 8, 0), &z),
        Some(3 * DAY_MS)
    );
    assert_eq!(
        cron("0,10 * * * *").period_ms(now, &z),
        Some(50 * MINUTE_MS)
    );
    assert_eq!(cron("0 0 31 2 *").period_ms(now, &z), None);
    assert_eq!(
        cron("0 0 29 2 *").period_ms(now, &z),
        Some(days_from_civil(2032, 2, 29) * DAY_MS - days_from_civil(2028, 2, 29) * DAY_MS)
    );
}

// Daylight saving, with made-up zones.

#[test]
fn the_made_up_zones_change_when_they_should() {
    // US Pacific 2026: 10:00 UTC on 8 March and 09:00 UTC on 1 November.
    assert_eq!(PACIFIC.starts(2026), utc(2026, 3, 8, 10, 0));
    assert_eq!(PACIFIC.ends(2026), utc(2026, 11, 1, 9, 0));
    assert_eq!(PACIFIC.offset_minutes(utc(2026, 3, 8, 10, 0) - 1), -480);
    assert_eq!(PACIFIC.offset_minutes(utc(2026, 3, 8, 10, 0)), -420);
    assert_eq!(PACIFIC.offset_minutes(utc(2026, 11, 1, 9, 0) - 1), -420);
    assert_eq!(PACIFIC.offset_minutes(utc(2026, 11, 1, 9, 0)), -480);
    assert_eq!(SYDNEY.offset_minutes(utc(2026, 1, 15, 0, 0)), 660);
    assert_eq!(SYDNEY.offset_minutes(utc(2026, 7, 15, 0, 0)), 600);
    assert_eq!(SYDNEY.offset_minutes(utc(2026, 12, 31, 23, 0)), 660);
}

#[test]
fn a_skipped_local_time_does_not_run() {
    // 2026-03-08 has no 02:00 to 02:59 in the US Pacific zone.
    let before = utc(2026, 3, 8, 8, 0); // 00:00 PST
    assert_eq!(
        cron("30 2 * * *").next_after(before, &PACIFIC),
        Some(utc(2026, 3, 9, 9, 30)) // 02:30 PDT the next day
    );
    assert_eq!(
        cron("0 2 * * *").next_after(before, &PACIFIC),
        Some(utc(2026, 3, 9, 9, 0))
    );
    assert_eq!(
        cron("0 3 * * *").next_after(before, &PACIFIC),
        Some(utc(2026, 3, 8, 10, 0)) // 03:00 PDT, the change itself
    );
    // Every 30 minutes: 01:30 PST, then 03:00 PDT.
    let c = cron("*/30 * * * *");
    assert_eq!(
        c.next_after(utc(2026, 3, 8, 9, 15), &PACIFIC),
        Some(utc(2026, 3, 8, 9, 30))
    );
    assert_eq!(
        c.next_after(utc(2026, 3, 8, 9, 30), &PACIFIC),
        Some(utc(2026, 3, 8, 10, 0))
    );
    // A 30 minute change: 2026-10-04 at Lord Howe skips 02:00 to 02:29.
    let before = utc(2026, 10, 3, 13, 0); // 23:30 the day before
    assert_eq!(
        cron("15 2 * * *").next_after(before, &LORD_HOWE),
        Some(utc(2026, 10, 4, 15, 15)) // the next day, 02:15 at +11:00
    );
    assert_eq!(
        cron("45 2 * * *").next_after(before, &LORD_HOWE),
        Some(utc(2026, 10, 3, 15, 45)) // 02:45 at +11:00
    );
    // A change at midnight: 2026-09-06 starts at 01:00, so a midnight job skips that day.
    assert_eq!(
        cron("0 0 * * *").next_after(utc(2026, 9, 5, 16, 0), &MIDNIGHT),
        Some(utc(2026, 9, 7, 3, 0))
    );
}

#[test]
fn a_repeated_local_time_runs_once_at_its_first_pass() {
    // 2026-11-01 runs 01:00 to 01:59 twice in the US Pacific zone: PDT, then PST.
    let c = cron("30 1 * * *");
    let first = c.next_after(utc(2026, 11, 1, 0, 0), &PACIFIC);
    assert_eq!(first, Some(utc(2026, 11, 1, 8, 30))); // 01:30 PDT
    assert_eq!(
        c.next_after(first.unwrap(), &PACIFIC),
        Some(utc(2026, 11, 2, 9, 30)) // 01:30 PST the next day, not 09:30 UTC today
    );
    // Every 30 minutes: 01:00 and 01:30 PDT, then 02:00 PST.
    let c = cron("*/30 * * * *");
    assert_eq!(
        c.next_after(utc(2026, 11, 1, 8, 0), &PACIFIC),
        Some(utc(2026, 11, 1, 8, 30))
    );
    assert_eq!(
        c.next_after(utc(2026, 11, 1, 8, 30), &PACIFIC),
        Some(utc(2026, 11, 1, 10, 0))
    );
    // Starting inside the second pass.
    assert_eq!(
        c.next_after(utc(2026, 11, 1, 9, 10), &PACIFIC),
        Some(utc(2026, 11, 1, 10, 0))
    );
    // A change at midnight: 2026-04-05 00:00 goes back to 23:00 on the 4th, so 23:30 on the 4th repeats.
    let c = cron("30 23 * * *");
    let first = c.next_after(utc(2026, 4, 4, 16, 0), &MIDNIGHT);
    assert_eq!(first, Some(utc(2026, 4, 5, 2, 30))); // 23:30 at -03:00
    assert_eq!(
        c.next_after(first.unwrap(), &MIDNIGHT),
        Some(utc(2026, 4, 6, 3, 30))
    );
    assert_eq!(
        cron("0 0 * * *").next_after(utc(2026, 4, 4, 16, 0), &MIDNIGHT),
        Some(utc(2026, 4, 5, 4, 0))
    );
}

#[test]
fn a_one_shot_runs_at_the_second_pass_too() {
    let c = cron("30 1 1 11 *");
    // Made at 01:45 PDT, after the first 01:30: it runs at 01:30 PST.
    assert_eq!(
        c.next_run(utc(2026, 11, 1, 8, 45), &PACIFIC, true),
        Some(utc(2026, 11, 1, 9, 30))
    );
    // A recurring job made then waits a year.
    assert_eq!(
        c.next_after(utc(2026, 11, 1, 8, 45), &PACIFIC),
        Some(utc(2027, 11, 1, 8, 30))
    );
    // A skipped time: a one-shot on 2026-03-08 02:30 waits a year too.
    assert_eq!(
        cron("30 2 8 3 *").next_run(utc(2026, 3, 1, 0, 0), &PACIFIC, true),
        Some(utc(2027, 3, 8, 10, 30))
    );
}

// Claude Code's jitter.

fn job<'a>(id: &'a str, cron: &'a Cron, recurring: bool, from_ms: i64) -> ClaudeJob<'a> {
    ClaudeJob {
        id,
        cron,
        recurring,
        from_ms,
    }
}

fn fire(id: &str, text: &str, recurring: bool, from_ms: i64) -> i64 {
    let c = cron(text);
    ClaudeScheduler::DEFAULT
        .next_fire_ms(&job(id, &c, recurring, from_ms), &YEREVAN)
        .unwrap()
}

#[test]
fn the_job_fraction_reads_like_javascript_parse_int() {
    assert_eq!(claude_frac("653b6114"), 1_698_390_292.0 / 4_294_967_296.0);
    assert_eq!(claude_frac("653b6114ffff"), claude_frac("653b6114"));
    assert_eq!(claude_frac("ffffffff"), 4_294_967_295.0 / 4_294_967_296.0);
    assert_eq!(claude_frac("00000000"), 0.0);
    assert_eq!(claude_frac("12zz"), 18.0 / 4_294_967_296.0);
    assert_eq!(claude_frac(" -1f"), -31.0 / 4_294_967_296.0);
    assert_eq!(claude_frac("+1f"), 31.0 / 4_294_967_296.0);
    assert_eq!(claude_frac("0x1f"), 31.0 / 4_294_967_296.0);
    assert_eq!(claude_frac("0X"), 0.0);
    assert_eq!(claude_frac(""), 0.0);
    assert_eq!(claude_frac("zz"), 0.0);
    assert_eq!(claude_frac("routine:abc"), 0.0);
    // `slice(0, 8)` counts UTF-16 units, not bytes: U+FEFF is one unit (three bytes), so seven digits remain.
    assert_eq!(
        claude_frac("\u{feff}fffffff1"),
        268_435_455.0 / 4_294_967_296.0
    );
}

/// Live test 3 of 2026-11-03 (Claude Code 2.1.316; `research/claude-code-signals.md`, "Live test results"; its log
/// is not kept): a `*/1` job, id `653b6114`, made 01:40:44.550 local (UTC+4) with `nextFireAt`
/// 1793655671863.1191, then fires at the times below.
#[test]
fn live_test_3_next_fire_at_and_fires() {
    let created = 1_793_655_644_550;
    let next = fire("653b6114", "*/1 * * * *", true, created);
    let logged = 1_793_655_671_863.119_1_f64;
    assert_eq!(next, logged.ceil() as i64);
    assert!(
        (0.0..1.0).contains(&(next as f64 - logged)),
        "{next} vs {logged}"
    );
    let fires = [
        1_793_655_672_615, // 01:41:12.615
        1_793_655_732_666, // 01:42:12.666
        1_793_655_792_719, // 01:43:12.719
        1_793_655_852_803, // 01:44:12.803
        1_793_655_911_895, // 01:45:11.895, after a `/clear`
    ];
    let mut due = next;
    for fired in fires {
        let late = fired - due;
        assert!(late > 0 && late <= 1000, "fired {late} ms after {due}");
        due = fire("653b6114", "*/1 * * * *", true, fired);
    }
}

/// The live fixtures (`tests/fixtures/claude/live/`, Claude Code 2.1.317): `nextFireAt` from each `CronCreate`
/// result and `CronList`, and the fires that followed. The results carry no creation time; it is the result line's
/// time or a few ms before it.
#[test]
fn live_fixtures_next_fire_times() {
    // 03-loop-1m: `bd9c33c4` made by 14:19:24.194Z, `nextFireAt` 1793715622219.9412; fires 14:20:22.274Z and
    // 14:21:22.327Z.
    let next = fire("bd9c33c4", "*/1 * * * *", true, 1_793_715_564_194);
    assert_eq!(next, 1_793_715_622_220);
    assert_eq!(1_793_715_622_274 - next, 54);
    let next = fire("bd9c33c4", "*/1 * * * *", true, 1_793_715_622_274);
    assert_eq!(1_793_715_682_327 - next, 107);

    // 06-clear-during-loop: `f7152559`, `nextFireAt` 1793715688954.9924; fires 14:21:29.172Z and 14:22:29.237Z;
    // `CronList` after the second fire: 1793715808954.9924.
    assert_eq!(
        fire("f7152559", "*/1 * * * *", true, 1_793_715_621_075),
        1_793_715_688_955
    );
    let next = fire("f7152559", "*/1 * * * *", true, 1_793_715_689_172);
    assert_eq!(1_793_715_749_237 - next, 282);
    assert_eq!(
        fire("f7152559", "*/1 * * * *", true, 1_793_715_749_237),
        1_793_715_808_955
    );

    // 09-resume-clean and 10-resume-kill, `*/5`: 285 s after creation, not on the 5 minute marks. Created 4 ms
    // before the result lines (14:23:26.767Z, 14:23:30.168Z). After the resume, `CronList` gave 285 s after the
    // fire ticks, 2 ms and 1 ms before the fire lines (14:29:41.353Z, 14:29:43.135Z).
    assert_eq!(
        fire("f9471e8f", "*/5 * * * *", true, 1_793_715_806_763),
        1_793_716_091_763
    );
    assert_eq!(
        fire("1e451e62", "*/5 * * * *", true, 1_793_715_810_164),
        1_793_716_095_164
    );
    assert_eq!(
        fire("f9471e8f", "*/5 * * * *", true, 1_793_716_181_351),
        1_793_716_466_351
    );
    assert_eq!(
        fire("1e451e62", "*/5 * * * *", true, 1_793_716_183_134),
        1_793_716_468_134
    );
    // Their next fires: 14:34:26.659Z and 14:34:28.499Z.
    assert_eq!(
        1_793_716_466_659 - fire("f9471e8f", "*/5 * * * *", true, 1_793_716_181_353),
        306
    );
    assert_eq!(
        1_793_716_468_499 - fire("1e451e62", "*/5 * * * *", true, 1_793_716_183_135),
        364
    );

    // 11-resume-one-shot: a daily `0 9 * * *` job, capped at 30 minutes late: 09:30 local.
    assert_eq!(
        fire("2d41aaae", "0 9 * * *", true, 1_793_716_256_016),
        1_793_770_200_000
    );
    // One-shots off :00 and :30 run on the minute; they fired 754 ms and 849 ms after.
    assert_eq!(
        fire("cb8c0135", "34 18 3 11 *", false, 1_793_716_255_966),
        1_793_716_440_000
    );
    assert_eq!(
        fire("12d31851", "23 18 3 11 *", false, 1_793_715_656_683),
        1_793_715_780_000
    );
}

/// Live tests 5 and 14 of 2026-11-03 (Claude Code 2.1.316; the log is not kept): a wakeup on `39 1 * * *` fired
/// 391 ms after 01:39:00, and the loop fallback one-shot `6633aef5` on `0 2 * * *` fired at 01:59:24.559, 35.9 s
/// early.
#[test]
fn live_one_shot_fires() {
    let due = fire("52d5f0e6", "39 1 * * *", false, 1_793_655_346_095);
    assert_eq!(due, 1_793_655_540_000);
    let due = fire("6633aef5", "0 2 * * *", false, 1_793_655_543_498);
    assert_eq!(
        due,
        (1_793_656_800_000.0 - 35_930.351_384_449_75_f64).ceil() as i64
    );
    assert_eq!(1_793_656_764_559 - due, 489);
}

#[test]
fn a_five_minute_step_fires_15_s_short_of_5_minutes_after_from() {
    let z = YEREVAN;
    let c = cron("*/5 * * * *");
    let s = ClaudeScheduler::DEFAULT;
    // From a 5 minute mark, that is 15 s before the next mark.
    let mark = at(z, 2026, 11, 3, 12, 0);
    assert_eq!(
        s.next_fire_ms(&job("ffffffff", &c, true, mark), &z),
        Some(mark + 285_000)
    );
    // From anywhere else, it does not land on a mark.
    let off_mark = mark + 3 * MINUTE_MS + 10_000;
    assert_eq!(
        s.next_fire_ms(&job("00000000", &c, true, off_mark), &z),
        Some(off_mark + 285_000)
    );
    // Other forms and other gaps get the usual jitter.
    let late_half = |text: &str, from: i64| {
        let c = cron(text);
        s.next_fire_ms(&job("80000000", &c, true, from), &z)
            .unwrap()
    };
    assert_eq!(
        late_half("0-59/5 * * * *", mark),
        mark + 5 * MINUTE_MS + 75_000
    );
    assert_eq!(
        late_half("*/6 * * * *", mark),
        mark + 6 * MINUTE_MS + 90_000
    );
    assert_eq!(
        late_half("*/4 * * * *", mark),
        mark + 4 * MINUTE_MS + 60_000
    );
    // No cache lead when the flag sets it to 0.
    let no_lead = ClaudeScheduler {
        cache_lead_ms: 0,
        ..ClaudeScheduler::DEFAULT
    };
    assert_eq!(
        no_lead.next_fire_ms(&job("80000000", &c, true, mark), &z),
        Some(mark + 5 * MINUTE_MS + 75_000)
    );
}

#[test]
fn recurring_jitter_is_a_share_of_the_gap_capped_at_30_minutes() {
    let z = YEREVAN;
    let nine = at(z, 2026, 11, 4, 9, 0);
    let from = at(z, 2026, 11, 3, 12, 0);
    // Daily: half of a day is far over the cap.
    assert_eq!(
        fire("ffffffff", "0 9 * * *", true, from),
        nine + 30 * MINUTE_MS
    );
    assert_eq!(fire("00000000", "0 9 * * *", true, from), nine);
    // 1/64 of half a day is 675 s, under the cap.
    assert_eq!(fire("04000000", "0 9 * * *", true, from), nine + 675_000);
    // Hourly: a quarter of an hour for a fraction of 1/2.
    assert_eq!(
        fire("80000000", "0 * * * *", true, from),
        from + HOUR_MS + 15 * MINUTE_MS
    );
    // A job with no second run in 5 years gets no jitter.
    let leap = utc(2096, 2, 29, 0, 0) - 240 * MINUTE_MS;
    assert_eq!(
        fire("ffffffff", "0 0 29 2 *", true, utc(2095, 6, 1, 0, 0)),
        leap
    );
    let never = cron("0 0 31 2 *");
    assert_eq!(
        ClaudeScheduler::DEFAULT.next_fire_ms(&job("ffffffff", &never, true, from), &z),
        None
    );
}

#[test]
fn a_one_shot_on_00_or_30_runs_early_but_not_before_its_creation() {
    let z = YEREVAN;
    let noon = at(z, 2026, 11, 3, 12, 0);
    let made = at(z, 2026, 11, 3, 9, 0);
    assert_eq!(fire("80000000", "0 12 3 11 *", false, made), noon - 45_000);
    assert_eq!(
        fire("80000000", "30 12 3 11 *", false, made),
        noon + 30 * MINUTE_MS - 45_000
    );
    assert_eq!(
        fire("80000000", "15 12 3 11 *", false, made),
        noon + 15 * MINUTE_MS
    );
    // Made 10 s before noon: the early time has passed, so it runs at once.
    assert_eq!(
        fire("80000000", "0 12 3 11 *", false, noon - 10_000),
        noon - 10_000
    );
    // `one_shot_minute_mod` 0 is outside the flag's schema; it means no early run, as JS's `x % 0` is NaN.
    let c = cron("0 12 3 11 *");
    let no_mod = ClaudeScheduler {
        one_shot_minute_mod: 0,
        ..ClaudeScheduler::DEFAULT
    };
    assert_eq!(
        no_mod.next_fire_ms(&job("80000000", &c, false, made), &z),
        Some(noon)
    );
    let never = cron("0 0 31 2 *");
    assert_eq!(
        ClaudeScheduler::DEFAULT.next_fire_ms(&job("80000000", &never, false, made), &z),
        None
    );
}

// Properties.

/// One cron field as text, from every form the parser takes.
fn field(min: u32, max: u32, top: u32) -> impl Strategy<Value = String> {
    let part = prop_oneof![
        Just("*".to_string()),
        (1..=max + 1).prop_map(|step| format!("*/{step}")),
        (min..=top).prop_map(|v| v.to_string()),
        (min..=top, min..=top).prop_map(|(a, b)| format!("{}-{}", a.min(b), a.max(b))),
        (min..=top, min..=top, 1..=max + 1).prop_map(|(a, b, step)| format!(
            "{}-{}/{step}",
            a.min(b),
            a.max(b)
        )),
    ];
    prop::collection::vec(part, 1..=3).prop_map(|parts| parts.join(","))
}

fn any_cron() -> impl Strategy<Value = String> {
    (
        field(0, 59, 59),
        field(0, 23, 23),
        field(1, 31, 31),
        field(1, 12, 12),
        field(0, 6, 7),
    )
        .prop_map(|(mi, h, d, mo, w)| format!("{mi} {h} {d} {mo} {w}"))
}

/// The first matching minute strictly after `t` at a fixed offset, found day by day without the search's jumps.
fn brute_next(c: &Cron, t: i64, zone: FixedOffset) -> Option<i64> {
    let offset = i64::from(zone.0) * MINUTE_MS;
    let start = (t.div_euclid(MINUTE_MS) + 1) * MINUTE_MS;
    let end = start + SEARCH_SPAN_MS;
    let mut day = (start + offset).div_euclid(DAY_MS);
    while day * DAY_MS - offset <= end {
        let w = Wall::at(day * DAY_MS);
        if has(c.months, w.month) && c.day_matches(&w) {
            for minute in 0..1440 {
                let u = day * DAY_MS + minute * MINUTE_MS - offset;
                if u >= start && c.matches(&Wall::at(u + offset)) {
                    return (u <= end).then_some(u);
                }
            }
        }
        day += 1;
    }
    None
}

/// The first matching minute strictly after `t`, minute by minute; `later_too` also takes the second pass of a
/// repeated local time (one-shots).
fn brute_next_in(c: &Cron, t: i64, zone: &RuleZone, later_too: bool) -> Option<i64> {
    let mut u = (t.div_euclid(MINUTE_MS) + 1) * MINUTE_MS;
    for _ in 0..3 * 1440 {
        let offset = zone.offset_minutes(u);
        let local = u + i64::from(offset) * MINUTE_MS;
        let later = [zone.standard, zone.daylight]
            .into_iter()
            .any(|o| o > offset && zone.offset_minutes(local - i64::from(o) * MINUTE_MS) == o);
        if (later_too || !later) && c.matches(&Wall::at(local)) {
            return Some(u);
        }
        u += MINUTE_MS;
    }
    None
}

const ZONES: [&RuleZone; 4] = [&PACIFIC, &SYDNEY, &LORD_HOWE, &MIDNIGHT];

proptest! {
    #[test]
    fn civil_dates_round_trip(days in -100_000_000_i64..=100_000_000) {
        let (y, m, d) = civil_from_days(days);
        prop_assert!((1..=12).contains(&m) && (1..=31).contains(&d));
        prop_assert_eq!(days_from_civil(y, m, d), days);
    }

    /// `next_after(t) > t`, it matches, and nothing between `t` and it matches.
    #[test]
    fn next_after_is_the_first_match(
        text in any_cron(),
        t in utc(2000, 1, 1, 0, 0)..utc(2090, 1, 1, 0, 0),
        offset in -720_i32..=840,
    ) {
        let c = cron(&text);
        let zone = FixedOffset(offset);
        let got = c.next_after(t, &zone);
        prop_assert_eq!(got, brute_next(&c, t, zone), "{}", text);
        if let Some(run) = got {
            prop_assert!(run > t);
        }
    }

    /// Near a daylight saving change, recurring jobs skip missing times and the second pass of repeated ones, and
    /// one-shots take the second pass too.
    #[test]
    fn next_after_near_daylight_saving_changes(
        minutes in field(0, 59, 59),
        hours in field(0, 23, 23),
        zone in 0..ZONES.len(),
        year in 2020_i64..2040,
        spring in any::<bool>(),
        shift in -6 * HOUR_MS..6 * HOUR_MS,
    ) {
        let zone = ZONES[zone];
        let text = format!("{minutes} {hours} * * *");
        let c = cron(&text);
        let change = if spring { zone.starts(year) } else { zone.ends(year) };
        let t = change + shift;
        prop_assert_eq!(c.next_after(t, zone), brute_next_in(&c, t, zone, false), "{}", text);
        prop_assert_eq!(c.next_run(t, zone, true), brute_next_in(&c, t, zone, true), "{}", text);
    }
}
