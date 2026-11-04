//! `SystemZone`: the PC's zone with daylight saving, for cron times.
//!
//! Owner: `p4-cron`.

use std::fmt;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Arc, Mutex};

use wakeshot_core::sources::cron::LocalOffset;
use windows::Win32::Foundation::{FILETIME, GetLastError, SYSTEMTIME, WIN32_ERROR};
use windows::Win32::System::SystemServices::{TIME_ZONE_ID_DAYLIGHT, TIME_ZONE_ID_STANDARD};
use windows::Win32::System::Time::{
    DYNAMIC_TIME_ZONE_INFORMATION, FileTimeToSystemTime, GetDynamicTimeZoneInformation,
    SystemTimeToFileTime, SystemTimeToTzSpecificLocalTimeEx, TIME_ZONE_ID_INVALID,
};

use crate::log::{Level, Log};
use crate::{WinError, clock};

/// The zone serves the watch thread's sources.
const AREA: &str = "watch";

/// 100 ns ticks per millisecond and per minute.
const TICKS_PER_MS: i64 = 10_000;
const TICKS_PER_MINUTE: i64 = 60_000 * TICKS_PER_MS;

/// Milliseconds from 1601-01-01 (the FILETIME start) to 1970-01-01.
const FILETIME_TO_UNIX_MS: i64 = 11_644_473_600_000;

/// No Win32 error uses it: the mark of an offset Windows gave that no zone has (more than a day).
const NO_WIN32_CODE: u32 = u32::MAX;

/// A zone as the core's `LocalOffset`, so cron times follow daylight saving in any year (track note 2).
///
/// Owner: `p4-cron`. Each call converts with Windows' own rules for that year (`SystemTimeToTzSpecificLocalTimeEx`
/// with the zone's dynamic daylight saving data). A failed call gives the zone's offset as last read now instead,
/// and keeps the first error for `take_error`, so the caller can log it; nothing here panics. A zone made with
/// `current_logged` logs each new error itself, once: boxed into a source, nobody can call `take_error`.
pub struct SystemZone {
    /// `None`: the PC's zone, read again at each call (about 0.3 us), so a zone change applies at once.
    named: Option<DYNAMIC_TIME_ZONE_INFORMATION>,
    /// The offset now, the answer for a failed call; refreshed at each read of the PC's zone.
    fallback_min: AtomicI32,
    error: Mutex<Option<WinError>>,
    log: Option<ZoneLog>,
}

/// Where each new error goes, with the errors logged so far: the cron search reads the zone many times, so a failing
/// call would fill the log.
struct ZoneLog {
    to: Arc<dyn Log>,
    logged: Mutex<Vec<WinError>>,
}

impl fmt::Debug for SystemZone {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SystemZone")
            .field("named", &self.named)
            .field("fallback_min", &self.fallback_min)
            .field("error", &self.error)
            .field("logged", &self.log.is_some())
            .finish()
    }
}

impl SystemZone {
    /// The PC's zone, as Settings > Time & language sets it.
    pub fn current() -> Self {
        Self::current_with(None)
    }

    /// The PC's zone, logging each new error to `log` once (the tray's Claude source).
    pub fn current_logged(log: Arc<dyn Log>) -> Self {
        Self::current_with(Some(ZoneLog {
            to: log,
            logged: Mutex::new(Vec::new()),
        }))
    }

    fn current_with(log: Option<ZoneLog>) -> Self {
        let zone = Self {
            named: None,
            fallback_min: AtomicI32::new(0),
            error: Mutex::new(None),
            log,
        };
        if let Err(e) = zone.read_current() {
            zone.keep(e);
        }
        zone
    }

    /// A zone from its data, such as one entry of `EnumDynamicTimeZoneInformation`, so tests can use a zone other
    /// than the PC's.
    pub fn named(zone: &DYNAMIC_TIME_ZONE_INFORMATION) -> Self {
        let now = convert(zone, clock::now().wall_ms);
        let fallback = now.unwrap_or(-(zone.Bias + zone.StandardBias));
        let zone = Self {
            named: Some(*zone),
            fallback_min: AtomicI32::new(fallback),
            error: Mutex::new(None),
            log: None,
        };
        if let Err(e) = now {
            zone.keep(e);
        }
        zone
    }

    /// The first failed call since the last take, so a burst of failures is logged once.
    pub fn take_error(&self) -> Option<WinError> {
        self.error.lock().ok()?.take()
    }

    fn read_current(&self) -> Result<DYNAMIC_TIME_ZONE_INFORMATION, WinError> {
        let mut zone = DYNAMIC_TIME_ZONE_INFORMATION::default();
        // SAFETY: the pointer is to a live local.
        let id = unsafe { GetDynamicTimeZoneInformation(&mut zone) };
        let bias = match id {
            TIME_ZONE_ID_INVALID => {
                return Err(WinError {
                    call: "GetDynamicTimeZoneInformation",
                    // SAFETY: takes no arguments and only returns a value.
                    code: unsafe { GetLastError() }.0,
                });
            }
            TIME_ZONE_ID_DAYLIGHT => zone.Bias + zone.DaylightBias,
            TIME_ZONE_ID_STANDARD => zone.Bias + zone.StandardBias,
            // No daylight saving in this zone, or the user turned it off.
            _ => zone.Bias,
        };
        self.fallback_min.store(-bias, Ordering::Relaxed);
        Ok(zone)
    }

    fn keep(&self, error: WinError) {
        if let Ok(mut slot) = self.error.lock() {
            slot.get_or_insert(error);
        }
        let Some(ZoneLog { to: log, logged }) = &self.log else {
            return;
        };
        let new = logged.lock().is_ok_and(|mut seen| {
            let new = !seen.contains(&error);
            if new {
                seen.push(error);
            }
            new
        });
        if new {
            let text = format!(
                "the time zone does not read: {} failed with {}; cron times use the offset read last ({} min)",
                error.call,
                error.code,
                self.fallback_min.load(Ordering::Relaxed)
            );
            log.line(Level::Warn, AREA, &text);
        }
    }

    fn failed(&self, error: WinError) -> i32 {
        self.keep(error);
        self.fallback_min.load(Ordering::Relaxed)
    }
}

impl LocalOffset for SystemZone {
    fn offset_minutes(&self, utc_ms: i64) -> i32 {
        let zone = match self.named {
            Some(zone) => zone,
            None => match self.read_current() {
                Ok(zone) => zone,
                Err(e) => return self.failed(e),
            },
        };
        convert(&zone, utc_ms).unwrap_or_else(|e| self.failed(e))
    }
}

/// Local time minus UTC in minutes at `utc_ms`, by the zone's rules for that year.
fn convert(zone: &DYNAMIC_TIME_ZONE_INFORMATION, utc_ms: i64) -> Result<i32, WinError> {
    // A time before 1601 or past the FILETIME range becomes a value Windows rejects, so the error names a real call.
    let ticks = utc_ms
        .checked_add(FILETIME_TO_UNIX_MS)
        .and_then(|ms| ms.checked_mul(TICKS_PER_MS))
        .map_or(u64::MAX, |t| t as u64);
    let utc_file = FILETIME {
        dwLowDateTime: ticks as u32,
        dwHighDateTime: (ticks >> 32) as u32,
    };
    let mut utc = SYSTEMTIME::default();
    // SAFETY: both pointers are to live locals.
    unsafe { FileTimeToSystemTime(&utc_file, &mut utc) }
        .map_err(|e| win_error("FileTimeToSystemTime", &e))?;
    let mut local = SYSTEMTIME::default();
    // SAFETY: the zone is a live reference and both times are live locals.
    unsafe { SystemTimeToTzSpecificLocalTimeEx(Some(zone), &utc, &mut local) }
        .map_err(|e| win_error("SystemTimeToTzSpecificLocalTimeEx", &e))?;
    let mut local_file = FILETIME::default();
    // SAFETY: both pointers are to live locals.
    unsafe { SystemTimeToFileTime(&local, &mut local_file) }
        .map_err(|e| win_error("SystemTimeToFileTime", &e))?;
    let local_ticks =
        (u64::from(local_file.dwHighDateTime) << 32) | u64::from(local_file.dwLowDateTime);
    // Both are below 2^63: Windows rejects any FILETIME above that.
    let offset = (local_ticks as i64 - ticks as i64).div_euclid(TICKS_PER_MINUTE);
    // The core's search assumes offsets within a day (`LocalOffset`); real zones are within 14 hours.
    match i32::try_from(offset) {
        Ok(minutes) if minutes.abs() <= 24 * 60 => Ok(minutes),
        _ => Err(WinError {
            call: "SystemTimeToTzSpecificLocalTimeEx",
            code: NO_WIN32_CODE,
        }),
    }
}

/// The Win32 error when the HRESULT wraps one, so codes read the same as Microsoft's error tables.
fn win_error(call: &'static str, error: &windows::core::Error) -> WinError {
    WinError {
        call,
        code: WIN32_ERROR::from_error(error).map_or(error.code().0 as u32, |e| e.0),
    }
}

#[cfg(test)]
mod tests {
    use wakeshot_core::sources::cron::{ClaudeJob, ClaudeScheduler, Cron, FixedOffset};
    use windows::Win32::Foundation::{ERROR_INVALID_PARAMETER, ERROR_NO_MORE_ITEMS};
    use windows::Win32::System::Time::{
        EnumDynamicTimeZoneInformation, GetTimeZoneInformation, GetTimeZoneInformationForYear,
        TIME_ZONE_INFORMATION,
    };

    use super::*;
    use crate::log::MemoryLog;

    const MINUTE_MS: i64 = 60_000;
    const DAY_MS: i64 = 24 * 60 * MINUTE_MS;

    fn text(units: &[u16]) -> String {
        let end = units.iter().position(|&u| u == 0).unwrap_or(units.len());
        String::from_utf16_lossy(&units[..end])
    }

    /// Reads Windows' zone list only; nothing here changes the PC's zone or clock.
    fn zone_named(key: &str) -> DYNAMIC_TIME_ZONE_INFORMATION {
        for index in 0.. {
            let mut zone = DYNAMIC_TIME_ZONE_INFORMATION::default();
            // SAFETY: the pointer is to a live local.
            let result = unsafe { EnumDynamicTimeZoneInformation(index, &mut zone) };
            assert_ne!(result, ERROR_NO_MORE_ITEMS.0, "no zone {key:?}");
            assert_eq!(result, 0, "EnumDynamicTimeZoneInformation({index})");
            if text(&zone.TimeZoneKeyName) == key {
                return zone;
            }
        }
        unreachable!()
    }

    /// Unix ms of a SYSTEMTIME read as UTC.
    fn ms_of(time: &SYSTEMTIME) -> i64 {
        let mut file = FILETIME::default();
        // SAFETY: both pointers are to live locals.
        unsafe { SystemTimeToFileTime(time, &mut file) }.unwrap();
        let ticks = (u64::from(file.dwHighDateTime) << 32) | u64::from(file.dwLowDateTime);
        ticks as i64 / TICKS_PER_MS - FILETIME_TO_UNIX_MS
    }

    fn utc(year: u16, month: u16, day: u16, hour: u16, minute: u16) -> i64 {
        ms_of(&SYSTEMTIME {
            wYear: year,
            wMonth: month,
            wDay: day,
            wHour: hour,
            wMinute: minute,
            ..Default::default()
        })
    }

    /// The local date and time of a yearly change rule (`wYear` 0): the `wDay`-th `wDayOfWeek` of `wMonth`, 5 being
    /// the last, at `wHour:wMinute`, worked out by Windows' own calendar.
    fn change_in(year: u16, rule: &SYSTEMTIME) -> SYSTEMTIME {
        assert_eq!(rule.wYear, 0, "a fixed-date rule");
        let mut time = SYSTEMTIME {
            wYear: year,
            wMonth: rule.wMonth,
            wDay: 1,
            wHour: rule.wHour,
            wMinute: rule.wMinute,
            ..Default::default()
        };
        // Day of week of the 1st, by a round trip through FILETIME.
        let mut file = FILETIME::default();
        let mut first = SYSTEMTIME::default();
        // SAFETY: all pointers are to live locals.
        unsafe { SystemTimeToFileTime(&time, &mut file) }.unwrap();
        // SAFETY: as above.
        unsafe { FileTimeToSystemTime(&file, &mut first) }.unwrap();
        time.wDay = 1 + (rule.wDayOfWeek + 7 - first.wDayOfWeek) % 7 + 7 * (rule.wDay - 1);
        // "Last": step back a week while the date does not exist.
        // SAFETY: both pointers are to live locals.
        while unsafe { SystemTimeToFileTime(&time, &mut file) }.is_err() {
            time.wDay -= 7;
        }
        time
    }

    #[test]
    fn the_pc_zone_matches_get_time_zone_information() {
        let mut info = TIME_ZONE_INFORMATION::default();
        // SAFETY: the pointer is to a live local.
        let id = unsafe { GetTimeZoneInformation(&mut info) };
        let bias = match id {
            TIME_ZONE_ID_DAYLIGHT => info.Bias + info.DaylightBias,
            TIME_ZONE_ID_STANDARD => info.Bias + info.StandardBias,
            _ => info.Bias,
        };
        let zone = SystemZone::current();
        assert_eq!(
            zone.offset_minutes(clock::now().wall_ms),
            -bias,
            "zone id {id}"
        );
        assert_eq!(zone.take_error(), None);
        let mut dynamic = DYNAMIC_TIME_ZONE_INFORMATION::default();
        // SAFETY: the pointer is to a live local.
        unsafe { GetDynamicTimeZoneInformation(&mut dynamic) };
        println!(
            "PC zone {:?}, offset {}",
            text(&dynamic.TimeZoneKeyName),
            -bias
        );
    }

    #[test]
    fn pacific_changes_on_the_dates_in_windows_zone_data() {
        let pacific = zone_named("Pacific Standard Time");
        let mut rules = TIME_ZONE_INFORMATION::default();
        // SAFETY: the zone and the result are live locals.
        unsafe { GetTimeZoneInformationForYear(2026, Some(&pacific), &mut rules) }.unwrap();
        let spring = change_in(2026, &rules.DaylightDate);
        let fall = change_in(2026, &rules.StandardDate);
        assert_eq!((spring.wMonth, spring.wDay, spring.wHour), (3, 8, 2));
        assert_eq!((fall.wMonth, fall.wDay, fall.wHour), (11, 1, 2));
        // Each change time is on the clock in effect before it.
        let spring_utc = ms_of(&spring) + i64::from(rules.Bias + rules.StandardBias) * MINUTE_MS;
        let fall_utc = ms_of(&fall) + i64::from(rules.Bias + rules.DaylightBias) * MINUTE_MS;
        assert_eq!(spring_utc, utc(2026, 3, 8, 10, 0));
        assert_eq!(fall_utc, utc(2026, 11, 1, 9, 0));

        let zone = SystemZone::named(&pacific);
        for (change, before, after) in [(spring_utc, -480, -420), (fall_utc, -420, -480)] {
            assert_eq!(zone.offset_minutes(change - MINUTE_MS), before);
            assert_eq!(zone.offset_minutes(change - 1), before);
            assert_eq!(zone.offset_minutes(change), after);
            assert_eq!(zone.offset_minutes(change + MINUTE_MS), after);
        }
        assert_eq!(zone.take_error(), None);
    }

    #[test]
    fn other_years_follow_their_own_rules() {
        let zone = SystemZone::named(&zone_named("Pacific Standard Time"));
        // Before 2007 US daylight saving started on the first Sunday of April, so mid-March 2006 was still
        // standard time, and 2027 starts on 14 March.
        assert_eq!(zone.offset_minutes(utc(2006, 3, 15, 12, 0)), -480);
        assert_eq!(zone.offset_minutes(utc(2006, 4, 2, 10, 0) - 1), -480);
        assert_eq!(zone.offset_minutes(utc(2006, 4, 2, 10, 0)), -420);
        assert_eq!(zone.offset_minutes(utc(2027, 3, 14, 10, 0) - 1), -480);
        assert_eq!(zone.offset_minutes(utc(2027, 3, 14, 10, 0)), -420);
        // A southern zone: daylight saving across the new year.
        let sydney = SystemZone::named(&zone_named("AUS Eastern Standard Time"));
        assert_eq!(sydney.offset_minutes(utc(2027, 1, 15, 0, 0)), 660);
        assert_eq!(sydney.offset_minutes(utc(2027, 7, 15, 0, 0)), 600);
        assert_eq!(zone.take_error(), None);
        assert_eq!(sydney.take_error(), None);
    }

    #[test]
    fn cron_times_skip_and_repeat_with_windows_rules() {
        let zone = SystemZone::named(&zone_named("Pacific Standard Time"));
        let spring = Cron::parse("30 2 * * *").unwrap();
        assert_eq!(
            spring.next_after(utc(2026, 3, 8, 8, 0), &zone),
            Some(utc(2026, 3, 9, 9, 30))
        );
        let fall = Cron::parse("30 1 * * *").unwrap();
        let first = fall.next_after(utc(2026, 11, 1, 0, 0), &zone);
        assert_eq!(first, Some(utc(2026, 11, 1, 8, 30)));
        assert_eq!(
            fall.next_after(first.unwrap(), &zone),
            Some(utc(2026, 11, 2, 9, 30))
        );
        assert_eq!(zone.take_error(), None);
    }

    #[test]
    fn a_daily_claude_job_with_the_pc_zone() {
        let zone = SystemZone::current();
        let now = clock::now().wall_ms;
        let cron = Cron::parse("0 9 * * *").unwrap();
        let job = ClaudeJob {
            id: "2d41aaae",
            cron: &cron,
            recurring: true,
            from_ms: now,
        };
        let started = std::time::Instant::now();
        let due = ClaudeScheduler::DEFAULT.next_fire_ms(&job, &zone);
        let took = started.elapsed();
        let offset = zone.offset_minutes(now);
        // Without a change in the next two days, a fixed offset gives the same time.
        if zone.offset_minutes(now + 2 * DAY_MS) == offset {
            let fixed = ClaudeScheduler::DEFAULT.next_fire_ms(&job, &FixedOffset(offset));
            assert_eq!(due, fixed);
        }
        assert!(due.is_some_and(|d| d > now && d <= now + DAY_MS + 30 * MINUTE_MS));
        assert_eq!(zone.take_error(), None);
        println!("daily job due {due:?}, search took {took:?}");
    }

    #[test]
    fn a_failed_call_gives_the_current_offset_and_keeps_the_error() {
        let zone = SystemZone::current();
        let now = zone.offset_minutes(clock::now().wall_ms);
        for bad in [i64::MIN, i64::MAX, -FILETIME_TO_UNIX_MS - 1] {
            assert_eq!(zone.offset_minutes(bad), now, "{bad}");
        }
        assert_eq!(
            zone.take_error(),
            Some(WinError {
                call: "FileTimeToSystemTime",
                code: ERROR_INVALID_PARAMETER.0,
            })
        );
        assert_eq!(zone.take_error(), None);
        let pacific = SystemZone::named(&zone_named("Pacific Standard Time"));
        let pacific_now = pacific.offset_minutes(clock::now().wall_ms);
        assert_eq!(pacific.offset_minutes(i64::MIN), pacific_now);
        assert!(pacific.take_error().is_some());
    }

    #[test]
    fn a_logged_zone_logs_each_new_error_once() {
        let log = Arc::new(MemoryLog::default());
        let zone = SystemZone::current_logged(log.clone());
        let now = zone.offset_minutes(clock::now().wall_ms);
        assert!(log.lines.lock().unwrap().is_empty(), "the PC's zone reads");
        for bad in [i64::MIN, i64::MAX, -FILETIME_TO_UNIX_MS - 1, i64::MIN] {
            assert_eq!(zone.offset_minutes(bad), now, "{bad}");
        }
        let lines = log.lines.lock().unwrap().clone();
        assert_eq!(lines.len(), 1, "one error, many reads: {lines:?}");
        let (level, area, text) = &lines[0];
        assert_eq!((*level, *area), (Level::Warn, AREA));
        assert_eq!(
            *text,
            format!(
                "the time zone does not read: FileTimeToSystemTime failed with 87; cron times use the offset read \
                 last ({now} min)"
            )
        );
        // `take_error` still works beside the log.
        assert!(zone.take_error().is_some());

        // Another call or code is new, and logged once too; one already logged stays quiet.
        let other = WinError {
            call: "GetDynamicTimeZoneInformation",
            code: 5,
        };
        zone.keep(other);
        zone.keep(other);
        zone.keep(WinError {
            call: "FileTimeToSystemTime",
            code: ERROR_INVALID_PARAMETER.0,
        });
        let lines = log.lines.lock().unwrap().clone();
        assert_eq!(lines.len(), 2, "{lines:?}");
        assert!(
            lines[1]
                .2
                .contains("GetDynamicTimeZoneInformation failed with 5"),
            "{lines:?}"
        );

        // An unlogged zone writes nowhere.
        let quiet = SystemZone::current();
        assert_eq!(quiet.offset_minutes(i64::MIN), now);
        assert!(quiet.take_error().is_some());
    }
}
