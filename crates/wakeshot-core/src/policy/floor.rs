//! The battery floor: when to step back and when to hold again (decision 21).

use crate::policy::lid::SOURCE_SETTLE_MS;

/// The floor when the energy saver threshold gives none.
pub const DEFAULT_PCT: u8 = 20;

/// How long the PC must have been on battery before a step back under the floor. A USB-C dock drops AC for a few
/// seconds (4.3 to 9.65 s on the test laptop's dock), and each drop under the floor gave a step back with its restore,
/// chime and notice, the notice still on screen after AC came back. The same 30 s as the closed-lid lock after an
/// unplug, so both fall due in one settle pass. The cost: on battery under the floor a hold keeps the PC awake 30 s
/// longer.
pub const GRACE_MS: u64 = SOURCE_SETTLE_MS;

/// The floor from the energy saver threshold on battery. Settings stores Never as 0 and On battery as 100 (research,
/// "Power settings spike"); both give 20 (product, "Battery floor"), and so does any value Settings never writes.
pub fn floor_pct(saver_threshold_dc: u32) -> u8 {
    match u8::try_from(saver_threshold_dc) {
        Ok(pct @ 1..=99) => pct,
        _ => DEFAULT_PCT,
    }
}

/// Strictly below, so a battery at the floor still holds (product, "Battery floor"). An unknown percent is not below.
pub fn below(battery_pct: Option<u8>, floor_pct: u8) -> bool {
    battery_pct.is_some_and(|pct| pct < floor_pct)
}

/// The floor's answer after a power reading.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Floor {
    /// Holds hold.
    Holds,
    /// On battery under the floor, but not yet for [`GRACE_MS`]: holds hold until this run time, then the step back,
    /// unless AC comes back first.
    Grace { due_mono_ms: u64 },
    /// No hold keeps the PC awake until AC.
    SteppedBack,
}

/// Where the grace counts from after an AC loss at `now_mono_ms` (run time, so a sleep in between does not count): the
/// loss itself when the AC before it lasted [`SOURCE_SETTLE_MS`], or AC since the first reading (`ac_since_mono`
/// `None`); else `kept`, the start of the run on battery that AC came back into for less than that. So a charger that
/// flaps, on battery under 30 s at a time with short AC between, still steps back 30 s after its first loss, while a
/// dock's short drop between long stretches of AC waits 30 s each time. `kept` is `None` for a tray that started on
/// battery and saw no AC that lasted: it steps back at once, as at its start. The cost under a flapping charger: each
/// blip of AC ends the step back, and the next loss steps back again at once, with its sound and notice. Change it
/// when a step back should end only once AC has lasted 30 s too.
pub fn battery_run_start(
    kept: Option<u64>,
    ac_since_mono: Option<u64>,
    now_mono_ms: u64,
) -> Option<u64> {
    let ac_lasted =
        ac_since_mono.is_none_or(|at| now_mono_ms >= at.saturating_add(SOURCE_SETTLE_MS));
    if ac_lasted { Some(now_mono_ms) } else { kept }
}

/// The floor after a power reading. `was`: stepped back before it. `on_battery_since_mono`: where the grace counts
/// from ([`battery_run_start`]); `None` when the core saw no AC loss after AC that lasted, as for a tray that starts
/// on battery, which steps back at once: there is no AC drop to wait out, and a hold that ran for the grace would
/// apply settings only to take them back with a chime 30 s later.
///
/// A step back starts on battery below the floor and ends only when AC comes back, so a percent that reads one higher
/// for a moment does not start the hold, its apply, notice and sound again. An unknown percent never starts it, or a
/// laptop whose percent cannot be read could never hold on battery; Wakeshot never writes the critical battery action,
/// so Windows keeps its own (whether a held request delays it: not verified). Change it when a lowered threshold must
/// end a step back without AC.
pub fn decide(
    was: bool,
    ac: bool,
    battery_pct: Option<u8>,
    floor_pct: u8,
    on_battery_since_mono: Option<u64>,
    now_mono_ms: u64,
) -> Floor {
    if ac {
        return Floor::Holds;
    }
    if was {
        return Floor::SteppedBack;
    }
    if !below(battery_pct, floor_pct) {
        return Floor::Holds;
    }
    match on_battery_since_mono.map(|at| at.saturating_add(GRACE_MS)) {
        Some(due_mono_ms) if now_mono_ms < due_mono_ms => Floor::Grace { due_mono_ms },
        _ => Floor::SteppedBack,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const T0: u64 = 7_200_000;

    /// No AC loss seen, so no grace: the step back rule alone.
    fn stepped_back(was: bool, ac: bool, battery_pct: Option<u8>, floor_pct: u8) -> bool {
        decide(was, ac, battery_pct, floor_pct, None, T0) == Floor::SteppedBack
    }

    #[test]
    fn floor_is_20_when_the_threshold_is_never_or_on_battery() {
        assert_eq!(floor_pct(0), 20);
        assert_eq!(floor_pct(100), 20);
    }

    #[test]
    fn floor_follows_the_threshold_in_between() {
        for t in [1, 10, 19, 20, 21, 50, 99] {
            assert_eq!(u32::from(floor_pct(t)), t);
        }
    }

    #[test]
    fn floor_is_20_for_values_settings_never_writes() {
        for t in [101, 255, 256, u32::MAX] {
            assert_eq!(floor_pct(t), DEFAULT_PCT, "{t}");
        }
    }

    #[test]
    fn below_the_floor_is_strict() {
        assert!(below(Some(19), 20));
        assert!(!below(Some(20), 20));
        assert!(!below(Some(21), 20));
        assert!(!below(None, 20));
    }

    #[test]
    fn steps_back_on_battery_below_the_floor() {
        assert!(stepped_back(false, false, Some(19), 20));
        assert!(!stepped_back(false, false, Some(20), 20));
        assert!(!stepped_back(false, false, Some(21), 20));
    }

    #[test]
    fn on_ac_below_the_floor_is_no_step_back() {
        assert!(!stepped_back(false, true, Some(5), 20));
    }

    #[test]
    fn ac_back_ends_the_step_back() {
        assert!(!stepped_back(true, true, Some(10), 20));
        assert!(!stepped_back(true, true, None, 20));
    }

    #[test]
    fn a_step_back_lasts_on_battery_until_ac() {
        assert!(stepped_back(true, false, Some(19), 20));
        assert!(stepped_back(true, false, Some(21), 20));
        assert!(stepped_back(true, false, None, 20));
    }

    #[test]
    fn an_unknown_percent_never_steps_back() {
        assert!(!stepped_back(false, false, None, 20));
        assert_eq!(decide(false, false, None, 20, Some(T0), T0), Floor::Holds);
    }

    #[test]
    fn the_grace_is_the_30_s_of_the_closed_lid_lock() {
        assert_eq!(GRACE_MS, 30_000);
        assert_eq!(GRACE_MS, SOURCE_SETTLE_MS);
    }

    #[test]
    fn under_the_floor_right_after_an_ac_loss_holds_until_30_s_on_battery() {
        let due = T0 + GRACE_MS;
        for now in [T0, T0 + 1, T0 + 5_000, due - 1] {
            assert_eq!(
                decide(false, false, Some(19), 20, Some(T0), now),
                Floor::Grace { due_mono_ms: due },
                "{now}"
            );
        }
        assert_eq!(
            decide(false, false, Some(19), 20, Some(T0), due),
            Floor::SteppedBack
        );
    }

    #[test]
    fn a_percent_that_falls_under_the_floor_after_30_s_on_battery_steps_back_at_once() {
        let now = T0 + GRACE_MS + 60_000;
        assert_eq!(
            decide(false, false, Some(19), 20, Some(T0), now),
            Floor::SteppedBack
        );
    }

    #[test]
    fn a_percent_that_falls_under_the_floor_inside_the_grace_waits_for_its_end() {
        assert_eq!(
            decide(false, false, Some(19), 20, Some(T0), T0 + 10_000),
            Floor::Grace {
                due_mono_ms: T0 + GRACE_MS
            }
        );
    }

    #[test]
    fn no_ac_loss_seen_steps_back_at_once() {
        assert_eq!(
            decide(false, false, Some(19), 20, None, T0),
            Floor::SteppedBack
        );
    }

    #[test]
    fn ac_back_inside_the_grace_holds() {
        assert_eq!(
            decide(false, true, Some(19), 20, Some(T0), T0 + 5_000),
            Floor::Holds
        );
    }

    #[test]
    fn a_loss_after_ac_that_lasted_30_s_starts_a_new_run() {
        let loss = T0 + 10 * 60_000;
        let ac_since = loss - SOURCE_SETTLE_MS;
        assert_eq!(
            battery_run_start(Some(T0), Some(ac_since), loss),
            Some(loss)
        );
        assert_eq!(battery_run_start(None, Some(ac_since), loss), Some(loss));
    }

    #[test]
    fn a_loss_after_ac_since_the_first_reading_starts_a_run() {
        assert_eq!(battery_run_start(None, None, T0), Some(T0));
    }

    #[test]
    fn a_loss_after_ac_of_under_30_s_keeps_the_run_of_the_first_loss() {
        // Lost at T0, back at T0 + 5 s, lost again at T0 + 10 s: the grace still ends 30 s after T0.
        let loss = T0 + 10_000;
        let kept = battery_run_start(Some(T0), Some(T0 + 5_000), loss);
        assert_eq!(kept, Some(T0));
        assert_eq!(
            battery_run_start(Some(T0), Some(loss - SOURCE_SETTLE_MS + 1), loss),
            Some(T0)
        );
        assert_eq!(
            decide(false, false, Some(19), 20, kept, T0 + GRACE_MS),
            Floor::SteppedBack
        );
    }

    #[test]
    fn a_tray_that_started_on_battery_keeps_stepping_back_at_once_after_a_short_ac_blip() {
        let kept = battery_run_start(None, Some(T0), T0 + 5_000);
        assert_eq!(kept, None);
        assert_eq!(
            decide(false, false, Some(19), 20, kept, T0 + 5_000),
            Floor::SteppedBack
        );
    }

    #[test]
    fn a_step_back_already_in_force_is_no_grace() {
        assert_eq!(
            decide(true, false, Some(19), 20, Some(T0), T0 + 1),
            Floor::SteppedBack
        );
    }

    #[test]
    fn at_or_above_the_floor_in_the_grace_holds() {
        assert_eq!(
            decide(false, false, Some(20), 20, Some(T0), T0 + 1),
            Floor::Holds
        );
    }

    #[test]
    fn a_far_future_since_does_not_overflow() {
        assert_eq!(
            decide(false, false, Some(19), 20, Some(u64::MAX), 0),
            Floor::Grace {
                due_mono_ms: u64::MAX
            }
        );
    }
}
