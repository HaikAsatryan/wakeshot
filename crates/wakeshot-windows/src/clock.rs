//! The two clocks the core needs (architecture section 3, "Clocks").

use wakeshot_core::time::{Clock, Now};
use windows::Win32::System::SystemInformation::GetSystemTimePreciseAsFileTime;
use windows::Win32::System::WindowsProgramming::QueryUnbiasedInterruptTime;

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
}
