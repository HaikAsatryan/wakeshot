//! The Keep screen on idle lock (decision 18).
//!
//! Windows locks by itself when the screen turns off for idle, also while a request is held (research, "Lock"). With
//! the display request on the screen never turns off, so Wakeshot locks instead, once input idle passes the user's
//! own sleep timeout for the current power source. The core calls [`sleep_timeout_ms`] and [`idle_lock`] at each
//! `InputIdle`, and on a lock sends `Lock` and `Notify(IdleLocked)` (track note 11).
//!
//! Owner: `p2-policy-power`.

use crate::policy::lid::user_value;
use crate::settings::{JournalEntry, PlanValues, SettingKey};
use crate::time::Deadline;

/// The user's own sleep timeout for a power source in ms, or `None` when it is Never (0): then Windows would never
/// sleep or lock for idle either. On battery it is the journal's `original` while we own the key, since our value is
/// Never; on AC it is always the live value, because Wakeshot never writes AC sleep (decision 17).
pub fn sleep_timeout_ms(plan: &PlanValues, ac: bool, journal: &[JournalEntry]) -> Option<u64> {
    let secs = if ac {
        plan.sleep_ac
    } else {
        user_value(journal, plan.active, SettingKey::SleepDc, plan.sleep_dc)
    };
    (secs != 0).then(|| u64::from(secs) * 1000)
}

/// What the idle lock looks at, at one input idle reading.
///
/// Owner: core track (Phase 2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IdleFacts {
    /// The display request is on. Without it the screen turns off and Windows locks by itself.
    pub display_on: bool,
    /// The session is locked already, so a lock and its notice would only mislead.
    pub session_locked: bool,
    /// Input idle clamped at resume (`policy::ending::clamped_idle_ms`), so a resume counts as input.
    pub idle_ms: u64,
    /// From [`sleep_timeout_ms`].
    pub timeout_ms: Option<u64>,
    pub now_mono_ms: u64,
}

/// The idle lock after one reading.
///
/// Owner: core track (Phase 2) keeps `locked_at_mono_ms` for the next reading and sets a deadline at `next_check`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IdleLock {
    /// Send `Lock` and `Notify(IdleLocked)` now.
    pub lock: bool,
    /// When we locked, while that idle period lasts: no second lock until input starts a new one.
    pub locked_at_mono_ms: Option<u64>,
    /// When the lock is due if no input comes, so the lock is not up to a safety tick late.
    pub next_check: Option<Deadline>,
}

/// Lock once per idle period. Input after our lock starts a new period; the reading shows it as an idle shorter than
/// the time since the lock, since without input idle would have grown by all of that time.
pub fn idle_lock(locked_at_mono_ms: Option<u64>, facts: IdleFacts) -> IdleLock {
    let now = facts.now_mono_ms;
    let locked_at = locked_at_mono_ms.filter(|&at| facts.idle_ms >= now.saturating_sub(at));
    let quiet = IdleLock {
        lock: false,
        locked_at_mono_ms: locked_at,
        next_check: None,
    };
    let Some(timeout) = facts.timeout_ms else {
        return quiet;
    };
    if !facts.display_on || facts.session_locked || locked_at.is_some() {
        return quiet;
    }
    if facts.idle_ms >= timeout {
        IdleLock {
            lock: true,
            locked_at_mono_ms: Some(now),
            next_check: None,
        }
    } else {
        IdleLock {
            next_check: Some(Deadline::Mono(now.saturating_add(timeout - facts.idle_ms))),
            ..quiet
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::ending::clamped_idle_ms;
    use crate::settings::Guid;

    const BALANCED: Guid = Guid(0x381b4222_f694_41f0_9685_ff5bb260df2e);
    const OTHER: Guid = Guid(0x8c5e7fda_e8bf_4a96_9a85_a6e23a8c635c);
    const TEN_MIN: u64 = 600_000;
    const T0: u64 = 7_200_000;

    fn plan(sleep_ac: u32, sleep_dc: u32) -> PlanValues {
        PlanValues {
            active: BALANCED,
            lid_ac: 0,
            lid_dc: 0,
            sleep_ac,
            sleep_dc,
            saver_threshold_dc: 20,
        }
    }

    fn sleep_entry(original: u32) -> JournalEntry {
        JournalEntry {
            plan: BALANCED,
            key: SettingKey::SleepDc,
            original,
            written: 0,
            restored: false,
        }
    }

    fn facts(idle_ms: u64, now_mono_ms: u64) -> IdleFacts {
        IdleFacts {
            display_on: true,
            session_locked: false,
            idle_ms,
            timeout_ms: Some(TEN_MIN),
            now_mono_ms,
        }
    }

    #[test]
    fn timeout_on_ac_is_the_live_value() {
        let j = [sleep_entry(600)];
        assert_eq!(
            sleep_timeout_ms(&plan(10_800, 0), true, &j),
            Some(10_800_000)
        );
    }

    #[test]
    fn timeout_on_battery_is_the_original_while_ours() {
        let j = [sleep_entry(600)];
        assert_eq!(sleep_timeout_ms(&plan(10_800, 0), false, &j), Some(TEN_MIN));
    }

    #[test]
    fn timeout_on_battery_is_the_live_value_when_not_ours() {
        assert_eq!(
            sleep_timeout_ms(&plan(10_800, 900), false, &[]),
            Some(900_000)
        );
        let changed = [sleep_entry(600)];
        assert_eq!(
            sleep_timeout_ms(&plan(10_800, 900), false, &changed),
            Some(900_000)
        );
        let elsewhere = [JournalEntry {
            plan: OTHER,
            ..sleep_entry(600)
        }];
        assert_eq!(sleep_timeout_ms(&plan(10_800, 0), false, &elsewhere), None);
    }

    #[test]
    fn keep_screen_on_with_sleep_timeout_never_never_locks() {
        let p = plan(0, 0);
        assert_eq!(sleep_timeout_ms(&p, true, &[]), None);
        assert_eq!(sleep_timeout_ms(&p, false, &[sleep_entry(0)]), None);
        let never = IdleFacts {
            timeout_ms: None,
            ..facts(u64::MAX / 2, T0)
        };
        assert_eq!(
            idle_lock(None, never),
            IdleLock {
                lock: false,
                locked_at_mono_ms: None,
                next_check: None
            }
        );
    }

    #[test]
    fn locks_at_the_timeout_not_before() {
        let before = idle_lock(None, facts(TEN_MIN - 1, T0));
        assert!(!before.lock);
        assert_eq!(before.locked_at_mono_ms, None);
        assert_eq!(before.next_check, Some(Deadline::Mono(T0 + 1)));

        let at = idle_lock(None, facts(TEN_MIN, T0));
        assert!(at.lock);
        assert_eq!(at.locked_at_mono_ms, Some(T0));
        assert_eq!(at.next_check, None);
    }

    #[test]
    fn one_lock_per_idle_period() {
        let first = idle_lock(None, facts(TEN_MIN, T0));
        assert!(first.lock);
        let mut locked_at = first.locked_at_mono_ms;
        for step in 1..=20 {
            let later = T0 + step * 30_000;
            let again = idle_lock(locked_at, facts(TEN_MIN + step * 30_000, later));
            assert!(!again.lock, "step {step}");
            assert_eq!(again.locked_at_mono_ms, Some(T0), "step {step}");
            locked_at = again.locked_at_mono_ms;
        }
    }

    #[test]
    fn input_after_the_lock_starts_a_new_period() {
        let first = idle_lock(None, facts(TEN_MIN, T0));
        let back = idle_lock(first.locked_at_mono_ms, facts(10_000, T0 + 60_000));
        assert!(!back.lock);
        assert_eq!(back.locked_at_mono_ms, None);
        assert_eq!(
            back.next_check,
            Some(Deadline::Mono(T0 + 60_000 + TEN_MIN - 10_000))
        );

        let gone_again = T0 + 60_000 + TEN_MIN - 10_000;
        let second = idle_lock(back.locked_at_mono_ms, facts(TEN_MIN, gone_again));
        assert!(second.lock);
        assert_eq!(second.locked_at_mono_ms, Some(gone_again));
    }

    #[test]
    fn a_resume_after_the_lock_starts_a_new_period() {
        let first = idle_lock(None, facts(TEN_MIN, T0));
        let resumed = T0 + 30_000;
        let now = resumed + 5_000;
        let idle = clamped_idle_ms(TEN_MIN + 3_600_000, now, Some(resumed));
        let after = idle_lock(first.locked_at_mono_ms, facts(idle, now));
        assert!(!after.lock);
        assert_eq!(after.locked_at_mono_ms, None);
    }

    #[test]
    fn no_lock_without_the_display_request() {
        let off = IdleFacts {
            display_on: false,
            ..facts(TEN_MIN * 3, T0)
        };
        let r = idle_lock(None, off);
        assert!(!r.lock);
        assert_eq!(r.next_check, None);
    }

    #[test]
    fn no_lock_while_the_session_is_locked() {
        let locked = IdleFacts {
            session_locked: true,
            ..facts(TEN_MIN * 3, T0)
        };
        let r = idle_lock(None, locked);
        assert!(!r.lock);
        assert_eq!(r.locked_at_mono_ms, None);
        assert_eq!(r.next_check, None);
    }
}
