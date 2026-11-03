//! The battery floor: when to step back and when to hold again (decision 21).
//!
//! Owner: `p2-policy-power`.

/// The floor when the energy saver threshold gives none.
pub const DEFAULT_PCT: u8 = 20;

/// The floor from the energy saver threshold on battery. Settings stores Never as 0 and On battery as 100 (research,
/// "Power settings spike"); both give 20 (product, "Battery floor"), and so does any value Settings never writes.
pub fn floor_pct(saver_threshold_dc: u32) -> u8 {
    match u8::try_from(saver_threshold_dc) {
        Ok(pct @ 1..=99) => pct,
        _ => DEFAULT_PCT,
    }
}

/// Strictly below, so a battery at the floor still holds (phase 1, "Values I set"). An unknown percent is not below.
pub fn below(battery_pct: Option<u8>, floor_pct: u8) -> bool {
    battery_pct.is_some_and(|pct| pct < floor_pct)
}

/// Stepped back after a power reading. It starts on battery below the floor and ends only when AC comes back, so a
/// percent that reads one higher for a moment does not start the hold, its apply, notice and sound again. An unknown
/// percent never starts it, or a laptop whose percent cannot be read could never hold on battery; Wakeshot never
/// writes the critical battery action, so Windows keeps its own (whether a held request delays it: not verified).
/// Change it when a lowered threshold must end a step back without AC.
pub fn stepped_back(was: bool, ac: bool, battery_pct: Option<u8>, floor_pct: u8) -> bool {
    !ac && (was || below(battery_pct, floor_pct))
}

#[cfg(test)]
mod tests {
    use super::*;

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
    }
}
