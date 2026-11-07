//! Hold files to holds: one parsed hold file, Automatic and `Now`, to whether it counts now, whether to delete the
//! file, and when that next changes on its own (architecture section 6; decisions 10, 11, 31 and 32). Also the Keep
//! awake choice the menu shows, read back from `keep-awake.json`.

use crate::holds::{HoldFacts, HoldKind};
use crate::process::Alive;
use crate::time::{Deadline, Now};
use crate::view::KeepAwake;

/// Whether a hold of this kind may count under the Automatic setting. Automatic watches agents and `wakeshot run`;
/// Keep awake works whether it is on or off (product, UX). The one place this rule lives, for agent sessions too.
pub fn kind_counts(kind: HoldKind, automatic: bool) -> bool {
    automatic || kind == HoldKind::KeepAwake
}

/// One hold file's answer at one `Now`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HoldAnswer {
    /// Whether the file holds the PC awake now.
    pub counts: bool,
    pub kind: HoldKind,
    /// The file's `reason`, already cut to 60 characters by the parser; `None` when the file has none, and the
    /// tooltip then names the kind (`present`).
    pub label: Option<String>,
    /// The display is wanted too: only a counting Keep awake hold with Keep screen on.
    pub screen: bool,
    /// Delete the file: its process is Gone. Never on `Unknown`, and whatever Automatic or `expires` say.
    pub delete: bool,
    /// The next time `counts` changes if the facts stay the same: `expires`, while it counts. Always after `now`.
    pub next: Option<Deadline>,
    /// The last liveness answer that was `Yes` or `Gone`, to pass back as `last_alive` next time.
    pub alive: Alive,
}

/// The hold file rules (architecture section 6):
///
/// - Counts while its process is alive and before `expires` (at `expires` it stops), and only if
///   [`kind_counts`] allows its kind. An expired file stays on disk while its process lives.
/// - Liveness `Gone`: no count and a delete. `Unknown`: `last_alive` stands (the last `Yes` or `Gone`; `Unknown` when
///   there was none, which counts nothing), and nothing is deleted.
/// - `screen` counts only for `KeepAwake`.
pub fn decide(facts: &HoldFacts, last_alive: Alive, automatic: bool, now: Now) -> HoldAnswer {
    let alive = match facts.alive {
        Alive::Unknown => last_alive,
        known => known,
    };
    let before_expires = facts
        .expires_wall_ms
        .is_none_or(|expires| now.wall_ms < expires);
    let counts = alive == Alive::Yes && before_expires && kind_counts(facts.kind, automatic);
    HoldAnswer {
        counts,
        kind: facts.kind,
        label: facts.reason.clone(),
        screen: counts && facts.kind == HoldKind::KeepAwake && facts.screen,
        delete: facts.alive == Alive::Gone,
        next: facts.expires_wall_ms.filter(|_| counts).map(Deadline::Wall),
        alive,
    }
}

/// The Keep awake choice now, and when it next changes on its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeepAwakeNow {
    /// `Off`, `UntilOff`, or `Minutes` with the minutes left rounded up: 30 right after a 30-minute choice, 1 in the
    /// last minute. Which menu item to check for it is `present`'s call.
    pub choice: KeepAwake,
    /// The next minute boundary while minutes are left, then `expires`; `None` for `Off` and `UntilOff`.
    pub next: Option<Deadline>,
}

/// The Keep awake choice for the menu and the tooltip, from `keep-awake.json`'s facts (`None`: no such file) and its
/// last liveness answer. It follows the file alone, so a choice made from the CLI shows the same way. A file of
/// another kind reads as `Off`.
pub fn keep_awake(facts: Option<&HoldFacts>, last_alive: Alive, now: Now) -> KeepAwakeNow {
    const MINUTE_MS: u64 = 60_000;
    let off = KeepAwakeNow {
        choice: KeepAwake::Off,
        next: None,
    };
    let Some(facts) = facts.filter(|f| f.kind == HoldKind::KeepAwake) else {
        return off;
    };
    if !decide(facts, last_alive, true, now).counts {
        return off;
    }
    let Some(expires) = facts.expires_wall_ms else {
        return KeepAwakeNow {
            choice: KeepAwake::UntilOff,
            next: None,
        };
    };
    let left = u64::try_from(i128::from(expires) - i128::from(now.wall_ms)).ok();
    let Some(left) = left.filter(|&ms| ms > 0) else {
        return off;
    };
    let minutes = left.div_ceil(MINUTE_MS);
    let to_next_minute = left - (minutes - 1) * MINUTE_MS;
    KeepAwakeNow {
        choice: KeepAwake::Minutes(u32::try_from(minutes).unwrap_or(u32::MAX)),
        next: Some(Deadline::Wall(
            now.wall_ms.saturating_add_unsigned(to_next_minute),
        )),
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::holds::HoldId;

    const SEC: i64 = 1000;
    const MIN: i64 = 60 * SEC;
    const HOUR: i64 = 60 * MIN;
    /// 2026-11-03 00:00 UTC.
    const T: i64 = 1_793_750_400_000;
    const NOW: Now = Now {
        wall_ms: T,
        mono_ms: 36_000_000,
    };
    const KINDS: [HoldKind; 4] = [
        HoldKind::Agent,
        HoldKind::Run,
        HoldKind::Script,
        HoldKind::KeepAwake,
    ];

    fn at(wall_ms: i64) -> Now {
        Now { wall_ms, ..NOW }
    }

    fn hold(kind: HoldKind, alive: Alive) -> HoldFacts {
        let name = match kind {
            HoldKind::Agent => "agent.json",
            HoldKind::Run => "run-4242.json",
            HoldKind::Script => "nightly-backup.json",
            HoldKind::KeepAwake => "keep-awake.json",
        };
        HoldFacts {
            id: HoldId(name.to_owned()),
            kind,
            alive,
            expires_wall_ms: None,
            reason: None,
            screen: false,
        }
    }

    fn expiring(kind: HoldKind, expires: i64) -> HoldFacts {
        HoldFacts {
            expires_wall_ms: Some(expires),
            ..hold(kind, Alive::Yes)
        }
    }

    fn screen_on() -> HoldFacts {
        HoldFacts {
            screen: true,
            ..hold(HoldKind::KeepAwake, Alive::Yes)
        }
    }

    /// A file seen for the first time, with Automatic on.
    fn first(facts: &HoldFacts, now: Now) -> HoldAnswer {
        decide(facts, Alive::Unknown, true, now)
    }

    // Kinds and Automatic.

    #[test]
    fn kind_counts_follows_automatic() {
        for kind in KINDS {
            assert!(kind_counts(kind, true), "{kind:?}");
            assert_eq!(
                kind_counts(kind, false),
                kind == HoldKind::KeepAwake,
                "{kind:?}"
            );
        }
    }

    #[test]
    fn each_live_kind_counts_with_automatic_on() {
        for kind in KINDS {
            let a = first(&hold(kind, Alive::Yes), NOW);
            assert!(a.counts, "{kind:?}");
            assert_eq!(a.kind, kind);
            assert!(!a.delete, "{kind:?}");
            assert_eq!(a.next, None, "{kind:?}");
        }
    }

    #[test]
    fn automatic_off_stops_run() {
        let a = decide(&expiring(HoldKind::Run, T + HOUR), Alive::Yes, false, NOW);
        assert!(!a.counts);
        assert_eq!(a.next, None);
        assert!(!a.delete);
    }

    #[test]
    fn automatic_off_stops_script() {
        let a = decide(&hold(HoldKind::Script, Alive::Yes), Alive::Yes, false, NOW);
        assert!(!a.counts);
    }

    #[test]
    fn automatic_off_keeps_keep_awake() {
        let a = decide(
            &expiring(HoldKind::KeepAwake, T + HOUR),
            Alive::Yes,
            false,
            NOW,
        );
        assert!(a.counts);
        assert_eq!(a.next, Some(Deadline::Wall(T + HOUR)));
    }

    // Liveness.

    #[test]
    fn yes_is_remembered() {
        assert_eq!(
            first(&hold(HoldKind::Run, Alive::Yes), NOW).alive,
            Alive::Yes
        );
    }

    #[test]
    fn gone_gives_a_delete_and_no_count() {
        for kind in KINDS {
            for automatic in [true, false] {
                let a = decide(&hold(kind, Alive::Gone), Alive::Yes, automatic, NOW);
                assert!(!a.counts, "{kind:?} {automatic}");
                assert!(a.delete, "{kind:?} {automatic}");
                assert_eq!(a.next, None, "{kind:?} {automatic}");
                assert_eq!(a.alive, Alive::Gone);
            }
        }
    }

    #[test]
    fn gone_after_expires_still_gives_a_delete() {
        let facts = HoldFacts {
            alive: Alive::Gone,
            ..expiring(HoldKind::Script, T - HOUR)
        };
        assert!(first(&facts, NOW).delete);
    }

    #[test]
    fn unknown_keeps_the_last_yes() {
        let a = decide(&expiring(HoldKind::Run, T + HOUR), Alive::Yes, true, NOW);
        let facts = HoldFacts {
            alive: Alive::Unknown,
            ..expiring(HoldKind::Run, T + HOUR)
        };
        let b = decide(&facts, a.alive, true, at(T + MIN));
        assert!(b.counts);
        assert!(!b.delete);
        assert_eq!(b.alive, Alive::Yes);
        assert_eq!(b.next, Some(Deadline::Wall(T + HOUR)));
    }

    #[test]
    fn unknown_keeps_the_last_gone_and_deletes_nothing() {
        let a = decide(&hold(HoldKind::Run, Alive::Unknown), Alive::Gone, true, NOW);
        assert!(!a.counts);
        assert!(!a.delete);
        assert_eq!(a.alive, Alive::Gone);
    }

    #[test]
    fn unknown_before_any_answer_counts_nothing_and_deletes_nothing() {
        let a = first(&hold(HoldKind::Run, Alive::Unknown), NOW);
        assert!(!a.counts);
        assert!(!a.delete);
        assert_eq!(a.alive, Alive::Unknown);
        assert_eq!(a.next, None);
    }

    // Expiry.

    #[test]
    fn no_expires_counts_with_no_deadline() {
        let a = first(&hold(HoldKind::Script, Alive::Yes), NOW);
        assert!(a.counts);
        assert_eq!(a.next, None);
    }

    #[test]
    fn counts_before_expires_with_its_deadline() {
        let a = first(&expiring(HoldKind::Script, T + 30 * MIN), NOW);
        assert!(a.counts);
        assert_eq!(a.next, Some(Deadline::Wall(T + 30 * MIN)));
    }

    #[test]
    fn counts_just_before_expires() {
        let a = first(&expiring(HoldKind::Run, T), at(T - 1));
        assert!(a.counts);
        assert_eq!(a.next, Some(Deadline::Wall(T)));
    }

    #[test]
    fn stops_exactly_at_expires() {
        let a = first(&expiring(HoldKind::Run, T), at(T));
        assert!(!a.counts);
        assert_eq!(a.next, None);
        assert!(!a.delete);
    }

    #[test]
    fn expired_file_is_kept_while_its_process_lives() {
        let a = first(&expiring(HoldKind::Script, T - HOUR), NOW);
        assert!(!a.counts);
        assert!(!a.delete);
        assert_eq!(a.next, None);
    }

    #[test]
    fn wall_jump_forward_past_expires_ends_the_hold() {
        let facts = expiring(HoldKind::KeepAwake, T + 30 * MIN);
        assert!(first(&facts, NOW).counts);
        let a = first(&facts, at(T + 5 * HOUR));
        assert!(!a.counts);
        assert_eq!(a.next, None);
    }

    #[test]
    fn wall_jump_back_keeps_the_deadline_at_expires() {
        let a = first(
            &expiring(HoldKind::KeepAwake, T + 30 * MIN),
            at(T - 3 * HOUR),
        );
        assert!(a.counts);
        assert_eq!(a.next, Some(Deadline::Wall(T + 30 * MIN)));
    }

    // Screen and label.

    #[test]
    fn screen_counts_for_keep_awake() {
        assert!(first(&screen_on(), NOW).screen);
    }

    #[test]
    fn screen_holds_with_automatic_off() {
        assert!(decide(&screen_on(), Alive::Yes, false, NOW).screen);
    }

    #[test]
    fn screen_is_ignored_outside_keep_awake() {
        for kind in [HoldKind::Agent, HoldKind::Run, HoldKind::Script] {
            let facts = HoldFacts {
                screen: true,
                ..hold(kind, Alive::Yes)
            };
            let a = first(&facts, NOW);
            assert!(a.counts, "{kind:?}");
            assert!(!a.screen, "{kind:?}");
        }
    }

    #[test]
    fn screen_ends_with_the_hold() {
        let expired = HoldFacts {
            expires_wall_ms: Some(T),
            ..screen_on()
        };
        assert!(!first(&expired, NOW).screen);
        let gone = HoldFacts {
            alive: Alive::Gone,
            ..screen_on()
        };
        assert!(!first(&gone, NOW).screen);
    }

    #[test]
    fn label_is_the_reason() {
        let facts = HoldFacts {
            reason: Some("nightly backup".to_owned()),
            ..hold(HoldKind::Script, Alive::Yes)
        };
        assert_eq!(first(&facts, NOW).label.as_deref(), Some("nightly backup"));
        assert_eq!(first(&hold(HoldKind::Run, Alive::Yes), NOW).label, None);
    }

    // Keep awake choice.

    fn keep_awake_until(expires: i64) -> HoldFacts {
        expiring(HoldKind::KeepAwake, expires)
    }

    #[test]
    fn keep_awake_is_off_without_a_file() {
        let k = keep_awake(None, Alive::Unknown, NOW);
        assert_eq!(k.choice, KeepAwake::Off);
        assert_eq!(k.next, None);
    }

    #[test]
    fn keep_awake_until_turned_off() {
        let k = keep_awake(
            Some(&hold(HoldKind::KeepAwake, Alive::Yes)),
            Alive::Yes,
            NOW,
        );
        assert_eq!(k.choice, KeepAwake::UntilOff);
        assert_eq!(k.next, None);
    }

    #[test]
    fn keep_awake_reads_its_full_minutes_at_the_start() {
        let k = keep_awake(Some(&keep_awake_until(T + 30 * MIN)), Alive::Yes, NOW);
        assert_eq!(k.choice, KeepAwake::Minutes(30));
        assert_eq!(k.next, Some(Deadline::Wall(T + MIN)));
    }

    #[test]
    fn keep_awake_rounds_minutes_left_up() {
        let k = keep_awake(Some(&keep_awake_until(T + 30 * MIN)), Alive::Yes, at(T + 1));
        assert_eq!(k.choice, KeepAwake::Minutes(30));
        assert_eq!(k.next, Some(Deadline::Wall(T + MIN)));
        let k = keep_awake(
            Some(&keep_awake_until(T + 30 * MIN)),
            Alive::Yes,
            at(T + MIN),
        );
        assert_eq!(k.choice, KeepAwake::Minutes(29));
        assert_eq!(k.next, Some(Deadline::Wall(T + 2 * MIN)));
    }

    #[test]
    fn keep_awake_last_minute_reads_one_until_expires() {
        let k = keep_awake(Some(&keep_awake_until(T)), Alive::Yes, at(T - 30 * SEC));
        assert_eq!(k.choice, KeepAwake::Minutes(1));
        assert_eq!(k.next, Some(Deadline::Wall(T)));
    }

    #[test]
    fn keep_awake_is_off_at_expires() {
        let k = keep_awake(Some(&keep_awake_until(T)), Alive::Yes, at(T));
        assert_eq!(k.choice, KeepAwake::Off);
        assert_eq!(k.next, None);
    }

    #[test]
    fn keep_awake_is_off_when_its_tray_is_gone() {
        let k = keep_awake(
            Some(&hold(HoldKind::KeepAwake, Alive::Gone)),
            Alive::Yes,
            NOW,
        );
        assert_eq!(k.choice, KeepAwake::Off);
    }

    #[test]
    fn keep_awake_unknown_keeps_the_last_answer() {
        let facts = HoldFacts {
            alive: Alive::Unknown,
            ..keep_awake_until(T + HOUR)
        };
        assert_eq!(
            keep_awake(Some(&facts), Alive::Yes, NOW).choice,
            KeepAwake::Minutes(60)
        );
        assert_eq!(
            keep_awake(Some(&facts), Alive::Gone, NOW).choice,
            KeepAwake::Off
        );
        assert_eq!(
            keep_awake(Some(&facts), Alive::Unknown, NOW).choice,
            KeepAwake::Off
        );
    }

    #[test]
    fn keep_awake_ignores_other_kinds() {
        let k = keep_awake(Some(&expiring(HoldKind::Run, T + HOUR)), Alive::Yes, NOW);
        assert_eq!(k.choice, KeepAwake::Off);
        assert_eq!(k.next, None);
    }

    #[test]
    fn extreme_times_do_not_overflow() {
        for expires in [i64::MIN, i64::MAX, 0] {
            for wall_ms in [i64::MIN, i64::MAX, 0] {
                let now = at(wall_ms);
                let facts = keep_awake_until(expires);
                let _ = first(&facts, now);
                let _ = keep_awake(Some(&facts), Alive::Yes, now);
            }
        }
    }

    // The deadlines are exact: the answer stays the same until each and changes at it.

    fn wall_of(d: Deadline) -> i64 {
        match d {
            Deadline::Wall(w) => w,
            Deadline::Mono(m) => panic!("hold files use wall time only, got Mono({m})"),
        }
    }

    fn alive() -> impl Strategy<Value = Alive> {
        prop_oneof![Just(Alive::Yes), Just(Alive::Gone), Just(Alive::Unknown)]
    }

    fn facts() -> impl Strategy<Value = HoldFacts> {
        (
            proptest::sample::select(KINDS.to_vec()),
            alive(),
            proptest::option::of(-3 * HOUR..5 * HOUR),
            any::<bool>(),
        )
            .prop_map(|(kind, alive, expires, screen)| HoldFacts {
                expires_wall_ms: expires.map(|e| T + e),
                screen: screen && kind == HoldKind::KeepAwake,
                ..hold(kind, alive)
            })
    }

    proptest! {
        #[test]
        fn hold_deadline_is_where_the_count_changes(
            facts in facts(),
            last in alive(),
            automatic in any::<bool>(),
            pick in any::<u64>(),
            far in 0..10 * HOUR,
        ) {
            let a = decide(&facts, last, automatic, NOW);
            prop_assert_eq!(a.delete, facts.alive == Alive::Gone);
            prop_assert!(!a.screen || (a.counts && a.kind == HoldKind::KeepAwake));
            match a.next {
                Some(d) => {
                    let gap = wall_of(d) - T;
                    prop_assert!(gap > 0);
                    let before = i64::try_from(pick % gap.unsigned_abs()).unwrap();
                    prop_assert_eq!(decide(&facts, a.alive, automatic, at(T + before)).counts, a.counts);
                    prop_assert_ne!(decide(&facts, a.alive, automatic, at(T + gap)).counts, a.counts);
                }
                None => {
                    prop_assert_eq!(decide(&facts, a.alive, automatic, at(T + far)).counts, a.counts);
                }
            }
        }

        #[test]
        fn keep_awake_deadline_is_where_the_choice_changes(
            facts in facts(),
            last in alive(),
            pick in any::<u64>(),
            far in 0..10 * HOUR,
        ) {
            let k = keep_awake(Some(&facts), last, NOW);
            let last = decide(&facts, last, true, NOW).alive;
            match k.next {
                Some(d) => {
                    let gap = wall_of(d) - T;
                    prop_assert!(gap > 0 && gap <= MIN);
                    let before = i64::try_from(pick % gap.unsigned_abs()).unwrap();
                    prop_assert_eq!(keep_awake(Some(&facts), last, at(T + before)).choice, k.choice);
                    prop_assert_ne!(keep_awake(Some(&facts), last, at(T + gap)).choice, k.choice);
                }
                None => {
                    prop_assert_eq!(keep_awake(Some(&facts), last, at(T + far)).choice, k.choice);
                }
            }
        }
    }
}
