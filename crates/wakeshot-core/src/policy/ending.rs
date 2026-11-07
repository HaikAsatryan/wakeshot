//! The countdown to sleep when work ends with the lid closed, or at a step back with the lid closed (decisions 21 and
//! 23: sleep, not hibernate).
//!
//! When the countdown is due the core sends `SleepCheck`: the hibernate thread checks again and plays the going to
//! sleep sound, then the core sets the 1-minute sleep and screen timeouts so Windows' own idle sleep takes the laptop.
//! `Hibernate` is the fallback [`FALLBACK_MS`] later. The thread's idle check must stay the same as
//! [`clamped_idle_ms`] and [`IDLE_MS`] here, so it skips only for input that came after the core's reading.

use crate::command::CountdownReason;
use crate::policy::lid::DO_NOTHING;
use crate::time::Deadline;

/// Input idle that makes the countdown due, so someone typing on an external screen with the lid closed is not put
/// to sleep.
pub const IDLE_MS: u64 = 5 * 60 * 1000;

/// How long the PC may stay awake with the short timeouts in force before the fallback hibernate, counted in run time
/// from the write and again from each resume: the same 5 minutes (product, work end). On the test laptop the sleep
/// came when the screen timer turned the screen off (one run, 2026-11-04), so with a 1-minute screen
/// timeout most likely about a minute after the write (inferred, not tested).
pub const FALLBACK_MS: u64 = 5 * 60 * 1000;

/// How far the wall clock must move past run time between a `Suspending` and the next tick for it to count as a
/// sleep. A sleep on the test laptop showed 227 s (research, "Idle sleep with the lid closed"), a suspend that did not
/// happen shows next to none, and 5 s stay clear of the wall clock's own small corrections.
pub const SLEPT_GAP_MS: i64 = 5_000;

/// Input idle with a resume counted as input: the smaller of the reported idle and the mono time since the last
/// `Resumed`. Input idle counts suspended time and mono time does not (research, "Clock"), but no suspend lies between
/// the last resume and now, so there the two count the same time.
pub fn clamped_idle_ms(
    reported_ms: u64,
    now_mono_ms: u64,
    last_resume_mono_ms: Option<u64>,
) -> u64 {
    match last_resume_mono_ms {
        Some(resumed) => reported_ms.min(now_mono_ms.saturating_sub(resumed)),
        None => reported_ms,
    }
}

/// Holding ended now: a countdown when the lid is closed and the user's own lid action (`policy::lid::user_action`,
/// read before the restore lands) is not Do nothing. With their own Do nothing, Wakeshot changed nothing that keeps
/// the laptop running, so it only releases its hold (product, work end).
pub fn starts_at_work_end(lid_closed: bool, user_lid_action: u32) -> bool {
    lid_closed && user_lid_action != DO_NOTHING
}

/// The reason to keep when a countdown starts while another may run. `WorkEnded` wins: nothing holds the laptop any
/// more, so AC back, which ends a `Floor` countdown, must not end this one.
pub fn merged(running: Option<CountdownReason>, starting: CountdownReason) -> CountdownReason {
    if running == Some(CountdownReason::WorkEnded) {
        CountdownReason::WorkEnded
    } else {
        starting
    }
}

/// What can end a countdown before it is due.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Cancel {
    /// Holding starts again: a hold counts and the PC is not stepped back. A hold that appears while stepped back
    /// does not start holding, so it leaves a `Floor` countdown running.
    HoldingStarts,
    LidOpened,
    AcBack,
}

/// Holding again or the lid opening ends every countdown; AC back ends only `Floor`, because work that ended still
/// needs the sleep on AC.
pub fn cancels(reason: CountdownReason, cancel: Cancel) -> bool {
    match cancel {
        Cancel::HoldingStarts | Cancel::LidOpened => true,
        Cancel::AcBack => reason == CountdownReason::Floor,
    }
}

/// The countdown at one input idle reading.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Check {
    /// Send `SleepCheck` now.
    Due,
    /// Not yet: the next check, when the countdown is due if no input comes.
    Wait(Deadline),
}

/// Due at [`IDLE_MS`] of clamped idle. Input does not end the countdown; it only moves the next check.
pub fn check(clamped_idle_ms: u64, now_mono_ms: u64) -> Check {
    if clamped_idle_ms >= IDLE_MS {
        Check::Due
    } else {
        Check::Wait(Deadline::Mono(
            now_mono_ms.saturating_add(IDLE_MS - clamped_idle_ms),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use CountdownReason::{Floor, WorkEnded};

    const T0: u64 = 7_200_000;
    const MINUTE: u64 = 60_000;

    #[test]
    fn idle_just_under_five_minutes_is_not_due() {
        assert_eq!(check(IDLE_MS - 1, T0), Check::Wait(Deadline::Mono(T0 + 1)));
    }

    #[test]
    fn idle_at_five_minutes_is_due() {
        assert_eq!(check(IDLE_MS, T0), Check::Due);
        assert_eq!(check(IDLE_MS + 1, T0), Check::Due);
    }

    #[test]
    fn no_idle_waits_the_whole_five_minutes() {
        assert_eq!(check(0, T0), Check::Wait(Deadline::Mono(T0 + IDLE_MS)));
    }

    #[test]
    fn a_resume_inside_the_idle_period_clamps_idle() {
        let idle = clamped_idle_ms(20 * MINUTE, T0, Some(T0 - MINUTE));
        assert_eq!(idle, MINUTE);
        assert_eq!(
            check(idle, T0),
            Check::Wait(Deadline::Mono(T0 + IDLE_MS - MINUTE))
        );
    }

    #[test]
    fn input_after_the_resume_counts_as_reported() {
        assert_eq!(clamped_idle_ms(30_000, T0, Some(T0 - 60 * MINUTE)), 30_000);
    }

    #[test]
    fn no_resume_yet_uses_the_reported_idle() {
        assert_eq!(clamped_idle_ms(20 * MINUTE, T0, None), 20 * MINUTE);
    }

    #[test]
    fn a_resume_at_now_means_no_idle() {
        assert_eq!(clamped_idle_ms(20 * MINUTE, T0, Some(T0)), 0);
        assert_eq!(clamped_idle_ms(20 * MINUTE, T0, Some(T0 + 1)), 0);
    }

    #[test]
    fn input_during_the_countdown_moves_the_check() {
        assert_eq!(
            check(4 * MINUTE, T0),
            Check::Wait(Deadline::Mono(T0 + MINUTE))
        );
        let later = T0 + MINUTE;
        assert_eq!(
            check(10_000, later),
            Check::Wait(Deadline::Mono(later + IDLE_MS - 10_000))
        );
    }

    #[test]
    fn work_end_countdown_needs_the_lid_closed_and_the_users_own_action() {
        assert!(!starts_at_work_end(true, DO_NOTHING));
        for own in [1, 2, 3] {
            assert!(starts_at_work_end(true, own), "{own}");
            assert!(!starts_at_work_end(false, own), "{own}");
        }
        assert!(!starts_at_work_end(false, DO_NOTHING));
    }

    #[test]
    fn holding_again_or_the_lid_opening_cancels_both_reasons() {
        for reason in [WorkEnded, Floor] {
            assert!(cancels(reason, Cancel::HoldingStarts), "{reason:?}");
            assert!(cancels(reason, Cancel::LidOpened), "{reason:?}");
        }
    }

    #[test]
    fn ac_back_cancels_only_the_floor_countdown() {
        assert!(cancels(Floor, Cancel::AcBack));
        assert!(!cancels(WorkEnded, Cancel::AcBack));
    }

    #[test]
    fn work_ended_wins_over_floor() {
        assert_eq!(merged(None, Floor), Floor);
        assert_eq!(merged(None, WorkEnded), WorkEnded);
        assert_eq!(merged(Some(Floor), WorkEnded), WorkEnded);
        assert_eq!(merged(Some(WorkEnded), Floor), WorkEnded);
        assert_eq!(merged(Some(Floor), Floor), Floor);
    }
}
