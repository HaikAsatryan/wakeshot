//! Claude's cron syntax and next run times in local time.
//!
//! The parser, the search and the jitter copy Claude Code's own scheduler (2.1.317: `d5`, `FYt`, `Zkn`, `W7o` in its
//! bundled code; `docs/research.md`, "Claude Code cron: parser, search and fire times"), not standard cron, so a due
//! time here is the one Claude Code picks. Local time comes in through `LocalOffset`: this module reads no clock and no
//! zone.

/// Local time minus UTC, in minutes, at an instant, so cron times can be local without OS calls in the core.
///
/// `wakeshot_windows::localtime::SystemZone` gives the PC's zone. The search assumes offsets within a day of UTC and
/// at most one change in any two days, as in every real zone; a zone that breaks this can get a wrong time near a
/// change, never a hang.
pub trait LocalOffset {
    fn offset_minutes(&self, utc_ms: i64) -> i32;
}

/// One offset at all times: for tests, and as the fallback when the PC's zone cannot be read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FixedOffset(pub i32);

impl LocalOffset for FixedOffset {
    fn offset_minutes(&self, _utc_ms: i64) -> i32 {
        self.0
    }
}

const MINUTE_MS: i64 = 60_000;
const HOUR_MS: i64 = 60 * MINUTE_MS;
const DAY_MS: i64 = 24 * HOUR_MS;

/// The range of a JS `Date`, 10^8 days each way: `FYt` finds nothing outside it.
const JS_DATE_LIMIT_MS: i64 = 100_000_000 * DAY_MS;

/// `FYt`'s own cap on steps (366 days of minutes), kept so no search here goes further than Claude Code's.
const SEARCH_STEPS: u32 = 527_040;

/// The search also stops about 5 years out: a job that far away never holds, and an impossible date such as
/// `0 0 31 2 *` then ends in about 200 steps instead of 527,040. Claude Code would still find a 29 February run up to
/// 8 years out around 2100; that is the only difference.
const SEARCH_SPAN_MS: i64 = 5 * 366 * DAY_MS;

/// Value ranges of minute, hour, day of month, month and day of week.
const FIELD_RANGES: [(u32, u32); 5] = [(0, 59), (0, 23), (1, 31), (1, 12), (0, 6)];

/// A parsed 5-field cron expression, as Claude Code's `CronCreate` takes it. Each field is a bit set (bit n is
/// value n).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cron {
    minutes: u64,
    hours: u64,
    days: u64,
    months: u64,
    weekdays: u64,
    /// The text is exactly `*/N * * * *`, the only form Claude Code's cache lead rule applies to
    /// (`ClaudeScheduler::next_fire_ms`).
    minute_step: bool,
}

impl Cron {
    /// Claude Code's `d5`: five fields split on white space, each a list of `*`, `*/n`, `a`, `a-b` or `a-b/n`. Names,
    /// `L`, `W` and `?` give `None`; day of week takes 0 to 7, 7 being Sunday as 0.
    pub fn parse(text: &str) -> Option<Cron> {
        let mut sets = [0u64; 5];
        let mut count = 0;
        for field in text.split(is_js_space).filter(|f| !f.is_empty()) {
            let &(min, max) = FIELD_RANGES.get(count)?;
            sets[count] = parse_field(field, min, max)?;
            count += 1;
        }
        if count != 5 {
            return None;
        }
        let [minutes, hours, days, months, weekdays] = sets;
        Some(Cron {
            minutes,
            hours,
            days,
            months,
            weekdays,
            minute_step: is_minute_step_form(text),
        })
    }

    /// The first matching local minute strictly after `utc_ms`, as UTC ms; `None` if there is none within about 5
    /// years. A local time skipped by a daylight saving change does not run; one that repeats runs at its first pass
    /// only. This is `FYt` for recurring jobs.
    pub fn next_after(&self, utc_ms: i64, zone: &dyn LocalOffset) -> Option<i64> {
        self.next_run(utc_ms, zone, false)
    }

    /// The gap between the next two runs after `at`, for rule 6 ("a period of 1 hour or less holds").
    pub fn period_ms(&self, at: i64, zone: &dyn LocalOffset) -> Option<i64> {
        let run = self.next_after(at, zone)?;
        Some(self.next_after(run, zone)? - run)
    }

    /// `FYt` step by step, with JS `Date` setters on local time, so even the daylight saving edges match Claude Code.
    /// One-shots (`oneShot: true`) also run at the second pass of a repeated time and walk a matching day minute by
    /// minute.
    fn next_run(&self, from_ms: i64, zone: &dyn LocalOffset, one_shot: bool) -> Option<i64> {
        let mut u = from_ms
            .div_euclid(MINUTE_MS)
            .checked_mul(MINUTE_MS)?
            .checked_add(MINUTE_MS)?;
        if !(-JS_DATE_LIMIT_MS..=JS_DATE_LIMIT_MS).contains(&u) {
            return None;
        }
        let end = u + SEARCH_SPAN_MS;
        for _ in 0..SEARCH_STEPS {
            if u > end {
                return None;
            }
            let w = wall(zone, u);
            let next = if to_instant(zone, w.local_ms) != u {
                if one_shot && self.matches(&w) {
                    return Some(u);
                }
                u + MINUTE_MS
            } else if !has(self.months, w.month) {
                let (year, month) = if w.month == 12 {
                    (w.year + 1, 1)
                } else {
                    (w.year, w.month + 1)
                };
                let first = to_instant(
                    zone,
                    days_from_civil(year, month, 1) * DAY_MS + w.time_of_day(),
                );
                to_instant(zone, wall(zone, first).midnight())
            } else if !self.day_matches(&w) {
                let tomorrow = to_instant(zone, w.local_ms + DAY_MS);
                to_instant(zone, wall(zone, tomorrow).midnight())
            } else if !has(self.hours, w.hour) || !has(self.minutes, w.minute) {
                if one_shot {
                    u + MINUTE_MS
                } else if !has(self.hours, w.hour) {
                    to_instant(zone, w.midnight() + i64::from(w.hour + 1) * HOUR_MS)
                } else {
                    to_instant(zone, w.local_ms + MINUTE_MS)
                }
            } else {
                return Some(u);
            };
            // Real zones always move forward here; the guard keeps a broken `LocalOffset` from looping in place.
            u = if next > u { next } else { u + MINUTE_MS };
        }
        None
    }

    fn matches(&self, w: &Wall) -> bool {
        has(self.months, w.month)
            && self.day_matches(w)
            && has(self.hours, w.hour)
            && has(self.minutes, w.minute)
    }

    /// With both day fields restricted, a day matches if either does (vixie cron). Claude Code calls a field
    /// unrestricted when its set holds every value, so `1-31` counts as `*` and `*/2` as restricted, where vixie cron
    /// looks at a leading `*`.
    fn day_matches(&self, w: &Wall) -> bool {
        let date = has(self.days, w.date);
        let weekday = has(self.weekdays, w.weekday);
        match (
            self.days.count_ones() == 31,
            self.weekdays.count_ones() == 7,
        ) {
            (true, true) => true,
            (true, false) => weekday,
            (false, true) => date,
            (false, false) => date || weekday,
        }
    }
}

/// Claude Code's `F` for one field. Its regexes are `^\*(?:\/(\d+))?$`, `^(\d+)-(\d+)(?:\/(\d+))?$` and `^\d+$`.
fn parse_field(field: &str, min: u32, max: u32) -> Option<u64> {
    let weekday = min == 0 && max == 6;
    let mut set = 0u64;
    for part in field.split(',') {
        if let Some(rest) = part.strip_prefix('*') {
            let step = if rest.is_empty() {
                1
            } else {
                number(rest.strip_prefix('/')?)?
            };
            if step < 1 {
                return None;
            }
            set |= steps(min, max, step);
        } else if let Some((low, rest)) = part.split_once('-') {
            let (high, step) = match rest.split_once('/') {
                Some((high, step)) => (high, number(step)?),
                None => (rest, 1),
            };
            let (low, high) = (number(low)?, number(high)?);
            let top = if weekday { 7 } else { max };
            if low > high || step < 1 || low < min || high > top {
                return None;
            }
            let mut range = steps(low, high, step);
            if weekday && range & (1 << 7) != 0 {
                range = (range & !(1 << 7)) | 1;
            }
            set |= range;
        } else {
            let mut value = number(part)?;
            if weekday && value == 7 {
                value = 0;
            }
            if value < min || value > max {
                return None;
            }
            set |= 1 << value;
        }
    }
    (set != 0).then_some(set)
}

fn steps(low: u32, high: u32, step: u32) -> u64 {
    let step = usize::try_from(step).unwrap_or(usize::MAX);
    (low..=high).step_by(step).fold(0, |set, v| set | (1 << v))
}

/// ASCII digits only, like JS `\d`. A value past `u32` saturates, which compares the same as JS's large doubles.
fn number(text: &str) -> Option<u32> {
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some(text.bytes().fold(0u32, |n, b| {
        n.saturating_mul(10).saturating_add(u32::from(b - b'0'))
    }))
}

/// Claude Code tests the raw text with `^\*\/\d+ \* \* \* \*$`, so spacing matters here, unlike in `parse`.
fn is_minute_step_form(text: &str) -> bool {
    let Some(rest) = text.strip_prefix("*/") else {
        return false;
    };
    let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
    digits > 0 && &rest[digits..] == " * * * *"
}

/// JS white space (`\s`, `trim`, `parseInt`), which differs from Rust's: it has U+FEFF and lacks U+0085.
fn is_js_space(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202f}'
                | '\u{205f}'
                | '\u{3000}'
                | '\u{feff}'
    )
}

fn has(set: u64, value: u32) -> bool {
    value < 64 && (set >> value) & 1 == 1
}

/// Claude Code's scheduler constants (`jJ` in 2.1.317) and its rule for a job's next fire (`wRt`).
///
/// Claude Code reads these from its remote flag `tengu_kairos_cron_config` and keeps the built-in values only when the
/// flag is missing or fails its schema, so they can change without a release. The schema bounds them: `recurring_frac`
/// 0 to 1, `recurring_cap_ms`, `one_shot_max_ms` and `one_shot_floor_ms` at most 30 minutes, `one_shot_minute_mod` 1
/// to 60, `cache_lead_ms` at most 1 minute, `recurring_max_age_ms` at most 30 days.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClaudeScheduler {
    pub recurring_frac: f64,
    pub recurring_cap_ms: i64,
    pub one_shot_max_ms: i64,
    pub one_shot_floor_ms: i64,
    pub one_shot_minute_mod: u32,
    /// Not used here: the transcript reader drops a recurring job this long after its creation, after one more fire.
    pub recurring_max_age_ms: i64,
    pub cache_lead_ms: i64,
}

/// One Claude Code cron job, as its transcript lines give it.
///
/// `from_ms` is when Claude Code last fired a recurring job (the time of its `scheduled_task_fire` line, a few ms after
/// its tick, so a `*/5` time comes out a few ms late), or when the job was created if it has not fired yet; for a
/// one-shot, always its creation time.
#[derive(Clone, Copy, Debug)]
pub struct ClaudeJob<'a> {
    pub id: &'a str,
    pub cron: &'a Cron,
    pub recurring: bool,
    pub from_ms: i64,
}

/// `ylr` in Claude Code: the gap the cache lead rule looks for.
const CACHE_WINDOW_MS: i64 = 5 * MINUTE_MS;

/// The largest `recurring_cap_ms` the flag's schema allows; the built-in value is the same.
const SCHEMA_MAX_CAP_MS: i64 = 30 * MINUTE_MS;

impl ClaudeScheduler {
    pub const DEFAULT: Self = Self {
        recurring_frac: 0.5,
        recurring_cap_ms: 30 * MINUTE_MS,
        one_shot_max_ms: 90_000,
        one_shot_floor_ms: 0,
        one_shot_minute_mod: 30,
        recurring_max_age_ms: 7 * DAY_MS,
        cache_lead_ms: 15_000,
    };

    /// When Claude Code fires the job next, in UTC ms, jitter included: the first whole ms its 1 s tick can fire it
    /// (it fires once `Date.now()` reaches its float time). `None` when the cron has no run within about 5 years.
    pub fn next_fire_ms(&self, job: &ClaudeJob, zone: &dyn LocalOffset) -> Option<i64> {
        if job.recurring {
            self.next_recurring(job, zone)
        } else {
            self.next_one_shot(job, zone)
        }
    }

    /// `Zkn`. Late by a share of the gap to the run after, so jobs on one cron spread out. A `*/N * * * *` job whose
    /// gap is 5 minutes instead fires `cache_lead_ms` short of 5 minutes after `from_ms`, wherever that falls on the
    /// clock (live fixtures 09 and 10: 285 s after creation and after each fire). By its name the rule keeps the
    /// prompt cache warm (not verified).
    fn next_recurring(&self, job: &ClaudeJob, zone: &dyn LocalOffset) -> Option<i64> {
        let run = job.cron.next_run(job.from_ms, zone, false)?;
        let Some(after) = job.cron.next_run(run, zone, false) else {
            return Some(run);
        };
        let gap = after - run;
        let lead = self.cache_lead_ms;
        if job.cron.minute_step
            && lead > 0
            && lead < gap
            && gap >= CACHE_WINDOW_MS
            && gap - lead < CACHE_WINDOW_MS
        {
            return job.from_ms.checked_add(gap - lead);
        }
        let late = (claude_frac(job.id) * self.recurring_frac * gap as f64)
            .min(self.recurring_cap_ms as f64);
        Some(ceil_ms(run as f64 + late))
    }

    /// The latest time Claude Code can fire a recurring job with any flag value its schema allows: the largest share of
    /// the gap with the largest cap, and for a `*/5` job also the `cache_lead_ms` rule, whose time is before `from_ms`
    /// plus 5 minutes for any lead above 0. Earlier fires need nothing: the shortest is the run itself, at most
    /// `recurring_cap_ms` before the built-in time, and the core holds from an hour before.
    pub fn latest_recurring_ms(job: &ClaudeJob, zone: &dyn LocalOffset) -> Option<i64> {
        let latest = Self {
            recurring_frac: 1.0,
            recurring_cap_ms: SCHEMA_MAX_CAP_MS,
            cache_lead_ms: 0,
            ..Self::DEFAULT
        };
        let share = latest.next_recurring(job, zone)?;
        let run = job.cron.next_run(job.from_ms, zone, false)?;
        let lead_rule = job.cron.minute_step
            && job.cron.next_run(run, zone, false).map(|after| after - run)
                == Some(CACHE_WINDOW_MS);
        Some(if lead_rule {
            share.max(job.from_ms.saturating_add(CACHE_WINDOW_MS))
        } else {
            share
        })
    }

    /// `W7o`. A one-shot on a minute divisible by `one_shot_minute_mod` (:00 and :30) runs early by up to
    /// `one_shot_max_ms`, never before its creation.
    fn next_one_shot(&self, job: &ClaudeJob, zone: &dyn LocalOffset) -> Option<i64> {
        let run = job.cron.next_run(job.from_ms, zone, true)?;
        if wall(zone, run).minute.checked_rem(self.one_shot_minute_mod) != Some(0) {
            return Some(run);
        }
        let early = self.one_shot_floor_ms as f64
            + claude_frac(job.id) * (self.one_shot_max_ms - self.one_shot_floor_ms) as f64;
        Some(ceil_ms((run as f64 - early).max(job.from_ms as f64)))
    }
}

/// Times stay within the JS `Date` range, so the cast neither wraps nor saturates.
fn ceil_ms(ms: f64) -> i64 {
    ms.ceil() as i64
}

/// Claude Code's `T`: `parseInt(id.slice(0, 8), 16) / 2^32`, or 0 when that is not a number. Ids are 8 hex digits,
/// but the JS rules are kept for any other text: `slice` counts UTF-16 units, `parseInt` skips leading white space,
/// takes a sign and a `0x`, then reads hex digits up to the first other character.
fn claude_frac(id: &str) -> f64 {
    let mut units = 0;
    let head_len = id
        .char_indices()
        .take_while(|&(_, c)| {
            units += c.len_utf16();
            units <= 8
        })
        .last()
        .map_or(0, |(i, c)| i + c.len_utf8());
    let text = id[..head_len].trim_start_matches(is_js_space);
    let (negative, text) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text.strip_prefix('+').unwrap_or(text)),
    };
    let text = text
        .strip_prefix("0x")
        .or_else(|| text.strip_prefix("0X"))
        .unwrap_or(text);
    let digits = text.bytes().take_while(u8::is_ascii_hexdigit).count();
    // At most 8 hex digits fit the head, so the value is below 2^32.
    let Ok(value) = u64::from_str_radix(&text[..digits], 16) else {
        return 0.0;
    };
    let frac = value as f64 / 4_294_967_296.0;
    if negative { -frac } else { frac }
}

/// A local time split into its fields, as JS `Date` getters give them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Wall {
    local_ms: i64,
    /// Days since 1970-01-01 in local time.
    day: i64,
    year: i64,
    month: u32,
    date: u32,
    hour: u32,
    minute: u32,
    /// 0 is Sunday.
    weekday: u32,
}

impl Wall {
    fn at(local_ms: i64) -> Wall {
        let day = local_ms.div_euclid(DAY_MS);
        let time = local_ms.rem_euclid(DAY_MS);
        let (year, month, date) = civil_from_days(day);
        Wall {
            local_ms,
            day,
            year,
            month,
            date,
            hour: (time / HOUR_MS) as u32,
            minute: (time % HOUR_MS / MINUTE_MS) as u32,
            // 1970-01-01 was a Thursday.
            weekday: (day + 4).rem_euclid(7) as u32,
        }
    }

    fn midnight(&self) -> i64 {
        self.day * DAY_MS
    }

    fn time_of_day(&self) -> i64 {
        self.local_ms - self.midnight()
    }
}

fn wall(zone: &dyn LocalOffset, utc_ms: i64) -> Wall {
    Wall::at(utc_ms + i64::from(zone.offset_minutes(utc_ms)) * MINUTE_MS)
}

/// The instant a JS `Date` setter picks for a local time (ECMAScript `UTC(t)`): the first pass when the time repeats;
/// for a time a change skipped, the offset from before the change, which lands as far past the change as the time
/// was into the gap.
fn to_instant(zone: &dyn LocalOffset, local_ms: i64) -> i64 {
    // An instant with this local time lies within a day of it, so the offsets a day before and a day after are the
    // only ones it can have.
    let before = zone.offset_minutes(local_ms - DAY_MS);
    let after = zone.offset_minutes(local_ms + DAY_MS);
    let fits = |offset: i32| {
        let t = local_ms - i64::from(offset) * MINUTE_MS;
        (zone.offset_minutes(t) == offset).then_some(t)
    };
    let first = fits(before);
    let second = if after == before { None } else { fits(after) };
    match (first, second) {
        (Some(a), Some(b)) => a.min(b),
        (Some(t), None) | (None, Some(t)) => t,
        (None, None) => local_ms - i64::from(before) * MINUTE_MS,
    }
}

/// Days since 1970-01-01 of a proleptic Gregorian date (Howard Hinnant's `days_from_civil`).
fn days_from_civil(year: i64, month: u32, date: u32) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let month = i64::from(month);
    let day_of_year =
        (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + i64::from(date) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// The date of a day number from `days_from_civil`, as (year, month 1 to 12, day 1 to 31).
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let days = days + 719_468;
    let era = days.div_euclid(146_097);
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let date = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month as u32, date as u32)
}

#[cfg(test)]
mod tests;
