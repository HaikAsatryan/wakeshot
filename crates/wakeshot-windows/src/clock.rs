//! The two clocks the core needs, and input idle (architecture section 3, "Clocks").
//!
//! Owner: power track (`p3-system-events`).

use wakeshot_core::time::{Clock, Now};
use windows::Win32::Foundation::{GetLastError, SetLastError, WIN32_ERROR};
use windows::Win32::System::SystemInformation::{GetSystemTimePreciseAsFileTime, GetTickCount};
use windows::Win32::System::WindowsProgramming::QueryUnbiasedInterruptTime;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};

use crate::WinError;

/// The real clock behind the core's `Clock` trait, so the loop takes any clock and tests use `FakeClock`.
///
/// Owner: power track (Phase 3).
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Now {
        now()
    }
}

/// 100 ns ticks per millisecond, the unit of both Windows clocks.
const TICKS_PER_MS: u64 = 10_000;

/// Milliseconds from 1601-01-01 (the FILETIME start) to 1970-01-01.
const FILETIME_TO_UNIX_MS: i64 = 11_644_473_600_000;

pub fn now() -> Now {
    // SAFETY: takes no arguments and only returns a value.
    let ft = unsafe { GetSystemTimePreciseAsFileTime() };
    let ticks = (u64::from(ft.dwHighDateTime) << 32) | u64::from(ft.dwLowDateTime);

    let mut unbiased = 0u64;
    // SAFETY: the pointer is to a live local. The call fails only for a null pointer (Microsoft docs), so its result
    // is not checked.
    let _ = unsafe { QueryUnbiasedInterruptTime(&mut unbiased) };

    Now {
        wall_ms: filetime_to_unix_ms(ticks),
        mono_ms: unbiased / TICKS_PER_MS,
    }
}

fn filetime_to_unix_ms(ticks: u64) -> i64 {
    // u64::MAX / 10_000 is below 2^51, so the cast never wraps.
    (ticks / TICKS_PER_MS) as i64 - FILETIME_TO_UNIX_MS
}

/// Milliseconds since this session's last input. It counts suspended time (research, "Clock"), so
/// the core clamps it at each resume (architecture section 3, "Clocks").
pub fn input_idle_ms() -> Result<u64, WinError> {
    let mut info = LASTINPUTINFO {
        cbSize: size_of::<LASTINPUTINFO>() as u32,
        dwTime: 0,
    };
    // Microsoft's page names no extended error for a failure, so a code of 0 means Windows gave none.
    // SAFETY: takes a value and returns nothing.
    unsafe { SetLastError(WIN32_ERROR(0)) };
    // SAFETY: `info` is a live local with `cbSize` set, as the call requires.
    if !unsafe { GetLastInputInfo(&mut info) }.as_bool() {
        return Err(WinError {
            call: "GetLastInputInfo",
            // SAFETY: takes no arguments and only returns a value.
            code: unsafe { GetLastError() }.0,
        });
    }
    // Read after the last input time, so an input between the two reads cannot land after `now`.
    // SAFETY: takes no arguments and only returns a value.
    let now = unsafe { GetTickCount() };
    Ok(idle_between(now, info.dwTime))
}

/// Both ticks are `GetTickCount` values, which wrap after 49.7 days, so the gap is taken modulo 2^32. Microsoft's
/// `GetLastInputInfo` page says the input tick need not grow (`SendInput` supplies its own), so it can lie just past
/// `now`; such a gap wraps to more than 2^31 ms (24.8 days) and counts as fresh input, 0. The cost: a real idle time
/// over 24.8 days also reads as 0, which at worst skips one hibernate.
fn idle_between(now_tick: u32, last_input_tick: u32) -> u64 {
    let gap = now_tick.wrapping_sub(last_input_tick);
    if gap > i32::MAX as u32 {
        0
    } else {
        u64::from(gap)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2026-01-01T00:00:00Z.
    const JAN_2026_MS: i64 = 1_767_225_600_000;

    #[test]
    fn now_reads_both_clocks() {
        let t = now();
        assert!(t.wall_ms > JAN_2026_MS, "wall_ms {}", t.wall_ms);
        assert!(t.mono_ms > 0);
    }

    #[test]
    fn system_clock_serves_the_core_trait() {
        let clock: &dyn Clock = &SystemClock;
        let (a, b) = (clock.now(), clock.now());
        assert!(a.wall_ms > JAN_2026_MS);
        assert!(b.mono_ms >= a.mono_ms);
    }

    #[test]
    fn filetime_converts_to_unix_ms() {
        assert_eq!(filetime_to_unix_ms(116_444_736_000_000_000), 0);
        assert_eq!(filetime_to_unix_ms(0), -FILETIME_TO_UNIX_MS);
    }

    #[test]
    fn input_idle_wraps() {
        assert_eq!(idle_between(1_000, 400), 600);
        assert_eq!(idle_between(7, 7), 0);
        // The tick wrapped between the input and now.
        assert_eq!(idle_between(5, u32::MAX - 10), 16);
        // The largest gap still read as idle: 2^31 - 1 ms.
        assert_eq!(idle_between(i32::MAX as u32, 0), i32::MAX as u64);
        // An input stamped just after now (SendInput's own tick).
        assert_eq!(idle_between(100, 103), 0);
        assert_eq!(idle_between(u32::MAX, 2), 0);
    }

    #[test]
    fn input_idle_reads() {
        let idle = input_idle_ms().unwrap();
        assert!(idle <= i32::MAX as u64, "{idle}");
    }
}
