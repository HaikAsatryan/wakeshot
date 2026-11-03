//! Agent sessions to holds: one session's facts, its liveness answer and what the core remembers about it, to whether
//! it holds now and when that next changes on its own (architecture section 7; decisions 5 to 8, 12 and 37). Parsers
//! report facts only; every hold rule for agents is here.
//!
//! Owner: `p2-policy-holds`.

use crate::process::Alive;
use crate::sources::{Activity, FormatHealth, SessionFacts};
use crate::time::{Deadline, Now};

/// How long before its due time a timer holds (decision 6).
pub const TIMER_LEAD_MS: i64 = 60 * 60 * 1000;

/// How long after its due time a timer still holds. Fires come up to 1 s late and the turn then shows busy; the
/// margin also ends a wakeup cancelled with Esc, which writes nothing (architecture section 7).
pub const TIMER_MARGIN_MS: i64 = 2 * 60 * 1000;

/// The longest a waiting session with Remote Control holds (decision 7).
pub const WAITING_LIMIT_MS: i64 = 60 * 60 * 1000;

/// What the core keeps about one session between snapshots and ticks. A new session starts from `default()`; the
/// caller stores the `memory` of each answer and drops it when the session leaves the snapshot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SessionMemory {
    /// The last liveness answer that was `Yes` or `Gone`; `Unknown` until the first one.
    pub alive: Alive,
    /// When the core first saw the current wait; `None` while the session is not waiting.
    pub waiting: Option<WaitSeen>,
}

impl Default for SessionMemory {
    fn default() -> Self {
        Self {
            alive: Alive::Unknown,
            waiting: None,
        }
    }
}

/// The first sight of one wait, on both clocks. The waiting hour runs on the wall clock (a sleep past it ends the
/// hold at resume); the run time since this sight also caps it, so a wall clock set back never stretches it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WaitSeen {
    /// The wait's own `since_wall_ms` at that sight; a different one later is a new wait with a new hour.
    pub since_wall_ms: Option<i64>,
    pub wall_ms: i64,
    pub mono_ms: u64,
}

/// One session's answer at one `Now`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SessionAnswer {
    /// Whether the session holds the PC awake now. Automatic is not applied here: with Automatic off the caller
    /// counts no agent session (`hold_files::kind_counts(HoldKind::Agent, false)`).
    pub holds: bool,
    /// The source's label, such as "Claude Code", for the tooltip groups and `Notice::UpdateWakeshot`.
    pub label: &'static str,
    /// The format is not fully known, so the caller shows "update Wakeshot" (once per start per agent). Wanted
    /// whatever the liveness, since an unreadable file may give no process to check.
    pub update_notice: bool,
    /// The next time `holds` changes if the facts stay the same; `None` when only new facts change it. Always after
    /// `now` on its clock.
    pub next: Option<Deadline>,
    /// What to keep for the next call.
    pub memory: SessionMemory,
}

/// The hold rules for one session (track note 12; architecture section 7):
///
/// - Liveness `Gone`: no hold. `Unknown`: the last `Yes` or `Gone` in `memory` stands; none yet: no hold.
/// - `Working`, or `open_tasks > 0`: holds while the process lives, no time limit (decisions 5 and 8).
/// - A timer holds from its due time less [`TIMER_LEAD_MS`] until its due time plus [`TIMER_MARGIN_MS`] (decision 6).
/// - `Waiting` holds only with `remote`, until `since_wall_ms` (or the core's first sight of the wait) plus
///   [`WAITING_LIMIT_MS`], and at most that long in run time after the first sight (decision 7).
/// - `FormatHealth::Unreadable`: no signal for the session. `UnknownValue`: counts as working while the process
///   lives. Both want the notice (decision 37).
///
/// Each window includes its start and excludes its end.
pub fn decide(
    facts: &SessionFacts,
    alive: Alive,
    memory: &SessionMemory,
    now: Now,
) -> SessionAnswer {
    let memory = remember(facts, alive, *memory, now);
    let (holds, next) = match steady(facts, memory.alive) {
        Some(holds) => (holds, None),
        None => {
            let clocked = Clocked::new(facts, &memory);
            let holds = clocked.holds_at(now, 0);
            (holds, clocked.next_change(now, holds))
        }
    };
    SessionAnswer {
        holds,
        label: facts.label,
        update_notice: facts.format != FormatHealth::Ok,
        next,
        memory,
    }
}

fn remember(
    facts: &SessionFacts,
    alive: Alive,
    mut memory: SessionMemory,
    now: Now,
) -> SessionMemory {
    if alive != Alive::Unknown {
        memory.alive = alive;
    }
    // An unreadable file says nothing about the wait, so the part of the hour already used stays used.
    if !matches!(facts.format, FormatHealth::Unreadable(_)) {
        memory.waiting = match facts.activity {
            Activity::Waiting { since_wall_ms, .. } => match memory.waiting {
                Some(seen) if seen.since_wall_ms == since_wall_ms => Some(seen),
                _ => Some(WaitSeen {
                    since_wall_ms,
                    wall_ms: now.wall_ms,
                    mono_ms: now.mono_ms,
                }),
            },
            Activity::Working | Activity::Idle => None,
        };
    }
    memory
}

/// The answer when time alone cannot change it; `None` when it depends on timers or the waiting hour.
fn steady(facts: &SessionFacts, alive: Alive) -> Option<bool> {
    if alive != Alive::Yes {
        return Some(false);
    }
    match facts.format {
        FormatHealth::Unreadable(_) => Some(false),
        FormatHealth::UnknownValue(..) => Some(true),
        FormatHealth::Ok if facts.activity == Activity::Working || facts.open_tasks > 0 => {
            Some(true)
        }
        FormatHealth::Ok => None,
    }
}

/// The rules that change with time alone: timer windows on the wall clock, and the waiting hour on both clocks.
struct Clocked {
    windows: Vec<(i64, i64)>,
    wait_end: Option<(i64, u64)>,
}

impl Clocked {
    fn new(facts: &SessionFacts, memory: &SessionMemory) -> Self {
        let windows = facts
            .timers
            .iter()
            .map(|&due| {
                (
                    due.saturating_sub(TIMER_LEAD_MS),
                    due.saturating_add(TIMER_MARGIN_MS),
                )
            })
            .collect();
        let wait_end = match (facts.activity, memory.waiting) {
            (
                Activity::Waiting {
                    since_wall_ms,
                    remote: true,
                },
                Some(seen),
            ) => {
                let wall_end = since_wall_ms
                    .unwrap_or(seen.wall_ms)
                    .saturating_add(WAITING_LIMIT_MS);
                // The hour left at first sight, counted in run time: equal to the wall rule while the clock runs
                // true, and the bound that holds when it is set back.
                let left = wall_end
                    .saturating_sub(seen.wall_ms)
                    .clamp(0, WAITING_LIMIT_MS);
                Some((wall_end, seen.mono_ms.saturating_add(left.unsigned_abs())))
            }
            _ => None,
        };
        Self { windows, wait_end }
    }

    /// The answer `ahead` ms after `now`, as if neither clock jumps in between.
    fn holds_at(&self, now: Now, ahead: u64) -> bool {
        let wall = now.wall_ms.saturating_add_unsigned(ahead);
        let mono = now.mono_ms.saturating_add(ahead);
        self.windows
            .iter()
            .any(|&(start, end)| start <= wall && wall < end)
            || self
                .wait_end
                .is_some_and(|(wall_end, mono_end)| wall < wall_end && mono < mono_end)
    }

    /// The first edge after `now` where the answer differs from `holds`. Edges where it stays the same (a window
    /// that ends inside another) are passed over, so the caller gets no Tick that changes nothing.
    fn next_change(&self, now: Now, holds: bool) -> Option<Deadline> {
        let walls = self
            .windows
            .iter()
            .flat_map(|&(start, end)| [start, end])
            .chain(self.wait_end.map(|(wall_end, _)| wall_end))
            .map(Deadline::Wall);
        let monos = self.wait_end.map(|(_, mono_end)| Deadline::Mono(mono_end));
        let mut edges: Vec<(u64, Deadline)> = walls
            .chain(monos)
            .filter_map(|edge| ahead_of(now, edge).map(|ms| (ms, edge)))
            .collect();
        // On a tie the wall edge goes first: after a sleep it is due at resume, where the run-time edge is not.
        edges.sort_by_key(|&(ms, edge)| (ms, matches!(edge, Deadline::Mono(_))));
        edges
            .into_iter()
            .find(|&(ms, _)| self.holds_at(now, ms) != holds)
            .map(|(_, edge)| edge)
    }
}

/// How many ms after `now` an edge lies on its own clock; `None` when it is before `now`. An edge at `now` is
/// already in the answer, so `next_change` never picks it.
fn ahead_of(now: Now, edge: Deadline) -> Option<u64> {
    match edge {
        Deadline::Wall(wall_ms) => {
            u64::try_from(i128::from(wall_ms) - i128::from(now.wall_ms)).ok()
        }
        Deadline::Mono(mono_ms) => mono_ms.checked_sub(now.mono_ms),
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::process::FileTime;
    use crate::sources::{Liveness, SessionKey, SourceId};

    const MIN: i64 = 60_000;
    const HOUR: i64 = 60 * MIN;
    /// 2026-11-03 00:00 UTC; any time works, a real one reads better in a failure.
    const T: i64 = 1_793_750_400_000;
    /// Ten hours of run time, so tests can look back on the monotonic clock.
    const M: u64 = 36_000_000;
    const NOW: Now = Now {
        wall_ms: T,
        mono_ms: M,
    };

    fn session(activity: Activity) -> SessionFacts {
        SessionFacts {
            key: SessionKey {
                source: SourceId::ClaudeCode,
                id: "4242".to_owned(),
            },
            liveness: Liveness::Process {
                pid: 4242,
                start: FileTime(134_381_211_391_959_336),
            },
            activity,
            timers: Vec::new(),
            open_tasks: 0,
            label: "Claude Code",
            format: FormatHealth::Ok,
        }
    }

    fn idle() -> SessionFacts {
        session(Activity::Idle)
    }

    fn working() -> SessionFacts {
        session(Activity::Working)
    }

    fn timers(due: &[i64]) -> SessionFacts {
        SessionFacts {
            timers: due.to_vec(),
            ..idle()
        }
    }

    fn waiting(since_wall_ms: Option<i64>, remote: bool) -> SessionFacts {
        session(Activity::Waiting {
            since_wall_ms,
            remote,
        })
    }

    /// Both clocks moved on together, as when nothing jumps and nothing sleeps.
    fn later(now: Now, ms: i64) -> Now {
        Now {
            wall_ms: now.wall_ms + ms,
            mono_ms: now.mono_ms.checked_add_signed(ms).unwrap(),
        }
    }

    fn at(wall_ms: i64) -> Now {
        later(NOW, wall_ms - T)
    }

    /// A session seen for the first time with a live process.
    fn first(facts: &SessionFacts, now: Now) -> SessionAnswer {
        decide(facts, Alive::Yes, &SessionMemory::default(), now)
    }

    /// The next call for the same session, with what the last answer said to keep.
    fn again(facts: &SessionFacts, last: &SessionAnswer, now: Now) -> SessionAnswer {
        decide(facts, Alive::Yes, &last.memory, now)
    }

    fn known(alive: Alive) -> SessionMemory {
        SessionMemory {
            alive,
            waiting: None,
        }
    }

    // Activities.

    #[test]
    fn working_holds_with_no_deadline() {
        let a = first(&working(), NOW);
        assert!(a.holds);
        assert_eq!(a.next, None);
    }

    #[test]
    fn idle_does_not_hold() {
        let a = first(&idle(), NOW);
        assert!(!a.holds);
        assert_eq!(a.next, None);
    }

    #[test]
    fn open_task_holds_while_idle() {
        let facts = SessionFacts {
            open_tasks: 1,
            ..idle()
        };
        let a = first(&facts, NOW);
        assert!(a.holds);
        assert_eq!(a.next, None);
    }

    #[test]
    fn open_task_holds_while_waiting_without_remote() {
        let facts = SessionFacts {
            open_tasks: 2,
            ..waiting(Some(T), false)
        };
        assert!(first(&facts, NOW).holds);
    }

    #[test]
    fn working_with_a_timer_has_no_deadline() {
        let facts = SessionFacts {
            timers: vec![T + 30 * MIN],
            ..working()
        };
        let a = first(&facts, NOW);
        assert!(a.holds);
        assert_eq!(a.next, None);
    }

    #[test]
    fn label_comes_from_the_source() {
        let facts = SessionFacts {
            label: "Codex CLI",
            ..idle()
        };
        assert_eq!(first(&facts, NOW).label, "Codex CLI");
    }

    // Liveness.

    #[test]
    fn yes_is_remembered() {
        assert_eq!(first(&idle(), NOW).memory.alive, Alive::Yes);
    }

    #[test]
    fn gone_does_not_hold() {
        let a = decide(&working(), Alive::Gone, &SessionMemory::default(), NOW);
        assert!(!a.holds);
        assert_eq!(a.next, None);
        assert_eq!(a.memory.alive, Alive::Gone);
    }

    #[test]
    fn gone_after_yes_stops_holding() {
        let a = decide(&working(), Alive::Gone, &known(Alive::Yes), NOW);
        assert!(!a.holds);
    }

    #[test]
    fn unknown_keeps_the_last_yes() {
        let a = decide(&working(), Alive::Unknown, &known(Alive::Yes), NOW);
        assert!(a.holds);
        assert_eq!(a.memory.alive, Alive::Yes);
    }

    #[test]
    fn unknown_keeps_the_last_gone() {
        let a = decide(&working(), Alive::Unknown, &known(Alive::Gone), NOW);
        assert!(!a.holds);
        assert_eq!(a.memory.alive, Alive::Gone);
    }

    #[test]
    fn unknown_before_any_answer_does_not_hold() {
        let a = decide(&working(), Alive::Unknown, &SessionMemory::default(), NOW);
        assert!(!a.holds);
        assert_eq!(a.memory.alive, Alive::Unknown);
    }

    #[test]
    fn unknown_still_follows_new_facts() {
        let a = decide(&idle(), Alive::Unknown, &known(Alive::Yes), NOW);
        assert!(!a.holds);
    }

    // Timers.

    const DUE: i64 = T + 3 * HOUR;

    #[test]
    fn timer_just_before_its_window_does_not_hold() {
        let a = first(&timers(&[DUE]), at(DUE - HOUR - 1));
        assert!(!a.holds);
        assert_eq!(a.next, Some(Deadline::Wall(DUE - HOUR)));
    }

    #[test]
    fn timer_window_opens_one_hour_before_due() {
        let a = first(&timers(&[DUE]), at(DUE - HOUR));
        assert!(a.holds);
        assert_eq!(a.next, Some(Deadline::Wall(DUE + 2 * MIN)));
    }

    #[test]
    fn timer_holds_inside_its_window() {
        let a = first(&timers(&[DUE]), at(DUE));
        assert!(a.holds);
        assert_eq!(a.next, Some(Deadline::Wall(DUE + 2 * MIN)));
    }

    #[test]
    fn timer_holds_just_before_its_window_ends() {
        let a = first(&timers(&[DUE]), at(DUE + 2 * MIN - 1));
        assert!(a.holds);
        assert_eq!(a.next, Some(Deadline::Wall(DUE + 2 * MIN)));
    }

    #[test]
    fn timer_window_ends_two_minutes_after_due() {
        let a = first(&timers(&[DUE]), at(DUE + 2 * MIN));
        assert!(!a.holds);
        assert_eq!(a.next, None);
    }

    #[test]
    fn timer_past_its_window_does_not_hold() {
        let a = first(&timers(&[DUE]), at(DUE + HOUR));
        assert!(!a.holds);
        assert_eq!(a.next, None);
    }

    #[test]
    fn two_timers_apart_hold_in_turn() {
        let facts = timers(&[T + 3 * HOUR, T + HOUR]);
        let a = first(&facts, at(T + 10 * MIN));
        assert!(a.holds);
        assert_eq!(a.next, Some(Deadline::Wall(T + HOUR + 2 * MIN)));
        let b = again(&facts, &a, at(T + 90 * MIN));
        assert!(!b.holds);
        assert_eq!(b.next, Some(Deadline::Wall(T + 2 * HOUR)));
    }

    #[test]
    fn two_overlapping_timers_hold_until_the_later_ends() {
        let a = first(&timers(&[T + 30 * MIN, T + 50 * MIN]), NOW);
        assert!(a.holds);
        assert_eq!(a.next, Some(Deadline::Wall(T + 52 * MIN)));
    }

    #[test]
    fn timer_windows_that_touch_hold_through() {
        let second = T + HOUR + 2 * MIN + HOUR;
        let a = first(&timers(&[T + HOUR, second]), at(T + 10 * MIN));
        assert!(a.holds);
        assert_eq!(a.next, Some(Deadline::Wall(second + 2 * MIN)));
    }

    // Waiting.

    #[test]
    fn waiting_without_remote_does_not_hold() {
        let a = first(&waiting(Some(T), false), NOW);
        assert!(!a.holds);
        assert_eq!(a.next, None);
    }

    #[test]
    fn waiting_with_remote_holds_until_an_hour_after_since() {
        let a = first(&waiting(Some(T), true), at(T + 30 * MIN));
        assert!(a.holds);
        assert_eq!(a.next, Some(Deadline::Wall(T + HOUR)));
    }

    #[test]
    fn waiting_with_remote_holds_just_before_the_hour() {
        let a = first(&waiting(Some(T), true), at(T + HOUR - 1));
        assert!(a.holds);
        assert_eq!(a.next, Some(Deadline::Wall(T + HOUR)));
    }

    #[test]
    fn waiting_with_remote_ends_at_the_hour() {
        let a = first(&waiting(Some(T), true), at(T + HOUR));
        assert!(!a.holds);
        assert_eq!(a.next, None);
    }

    #[test]
    fn waiting_without_since_counts_from_first_seen() {
        let facts = waiting(None, true);
        let a = first(&facts, NOW);
        assert!(a.holds);
        assert_eq!(a.next, Some(Deadline::Wall(T + HOUR)));
        let seen = WaitSeen {
            since_wall_ms: None,
            wall_ms: T,
            mono_ms: M,
        };
        assert_eq!(a.memory.waiting, Some(seen));

        let b = again(&facts, &a, at(T + 59 * MIN));
        assert!(b.holds);
        assert_eq!(b.next, Some(Deadline::Wall(T + HOUR)));
        assert_eq!(b.memory.waiting, Some(seen));

        let c = again(&facts, &b, at(T + HOUR));
        assert!(!c.holds);
        assert_eq!(c.next, None);
    }

    #[test]
    fn waiting_seen_late_holds_only_the_rest_of_the_hour() {
        let a = first(&waiting(Some(T), true), at(T + 50 * MIN));
        assert!(a.holds);
        assert_eq!(a.next, Some(Deadline::Wall(T + HOUR)));
        let b = again(&waiting(Some(T), true), &a, at(T + HOUR));
        assert!(!b.holds);
    }

    #[test]
    fn a_new_wait_starts_a_new_hour() {
        let a = first(&waiting(Some(T), true), NOW);
        let next_wait = T + 50 * MIN;
        let b = again(&waiting(Some(next_wait), true), &a, at(next_wait));
        assert!(b.holds);
        assert_eq!(b.next, Some(Deadline::Wall(next_wait + HOUR)));
        let c = again(&waiting(Some(next_wait), true), &b, at(T + 61 * MIN));
        assert!(c.holds);
    }

    #[test]
    fn leaving_waiting_forgets_first_seen() {
        let a = first(&waiting(None, true), NOW);
        let b = again(&idle(), &a, at(T + 10 * MIN));
        assert_eq!(b.memory.waiting, None);
        let c = again(&waiting(None, true), &b, at(T + 20 * MIN));
        assert!(c.holds);
        assert_eq!(c.next, Some(Deadline::Wall(T + 20 * MIN + HOUR)));
    }

    #[test]
    fn turning_on_remote_control_does_not_restart_the_hour() {
        let a = first(&waiting(None, false), NOW);
        assert!(!a.holds);
        let b = again(&waiting(None, true), &a, at(T + 40 * MIN));
        assert!(b.holds);
        assert_eq!(b.next, Some(Deadline::Wall(T + HOUR)));
    }

    // Wall clock jumps.

    #[test]
    fn wall_jump_back_does_not_stretch_the_waiting_hour() {
        for since in [None, Some(T)] {
            let facts = waiting(since, true);
            let a = first(&facts, NOW);
            let back = Now {
                wall_ms: T - 3 * HOUR + 10 * MIN,
                mono_ms: M + 10 * MIN as u64,
            };
            let b = again(&facts, &a, back);
            assert!(b.holds, "since {since:?}");
            assert_eq!(
                b.next,
                Some(Deadline::Mono(M + HOUR as u64)),
                "since {since:?}"
            );
            let c = again(&facts, &b, later(back, 50 * MIN));
            assert!(!c.holds, "since {since:?}");
            assert_eq!(c.next, None, "since {since:?}");
        }
    }

    #[test]
    fn wait_since_ahead_of_the_clock_holds_at_most_an_hour() {
        let facts = waiting(Some(T + 2 * HOUR), true);
        let a = first(&facts, NOW);
        assert!(a.holds);
        assert_eq!(a.next, Some(Deadline::Mono(M + HOUR as u64)));
        assert!(!again(&facts, &a, later(NOW, HOUR)).holds);
    }

    #[test]
    fn wall_jump_forward_ends_the_waiting_hour_now() {
        let facts = waiting(None, true);
        let a = first(&facts, NOW);
        let ahead = Now {
            wall_ms: T + 2 * HOUR,
            mono_ms: M + MIN as u64,
        };
        let b = again(&facts, &a, ahead);
        assert!(!b.holds);
        assert_eq!(b.next, None);
    }

    #[test]
    fn wall_jump_forward_past_a_timer_window_ends_it() {
        let facts = timers(&[T + 30 * MIN]);
        assert!(first(&facts, NOW).holds);
        let ahead = Now {
            wall_ms: T + 4 * HOUR,
            mono_ms: M + MIN as u64,
        };
        let a = first(&facts, ahead);
        assert!(!a.holds);
        assert_eq!(a.next, None);
    }

    #[test]
    fn wall_jump_back_before_a_timer_window_waits_for_it() {
        let due = T + 30 * MIN;
        let back = Now {
            wall_ms: T - 2 * HOUR,
            mono_ms: M + MIN as u64,
        };
        let a = first(&timers(&[due]), back);
        assert!(!a.holds);
        assert_eq!(a.next, Some(Deadline::Wall(due - HOUR)));
    }

    // Format health.

    #[test]
    fn ok_format_wants_no_notice() {
        assert!(!first(&working(), NOW).update_notice);
    }

    #[test]
    fn unreadable_format_gives_no_signal_and_a_notice() {
        let facts = SessionFacts {
            format: FormatHealth::Unreadable("status file"),
            open_tasks: 1,
            timers: vec![T + 10 * MIN],
            ..working()
        };
        let a = first(&facts, NOW);
        assert!(!a.holds);
        assert!(a.update_notice);
        assert_eq!(a.next, None);
    }

    #[test]
    fn unreadable_format_wants_the_notice_whatever_the_liveness() {
        let facts = SessionFacts {
            format: FormatHealth::Unreadable("status file"),
            ..idle()
        };
        for alive in [Alive::Yes, Alive::Gone, Alive::Unknown] {
            let a = decide(&facts, alive, &SessionMemory::default(), NOW);
            assert!(a.update_notice, "{alive:?}");
        }
    }

    #[test]
    fn unreadable_format_keeps_the_waiting_first_seen() {
        let a = first(&waiting(None, true), NOW);
        let facts = SessionFacts {
            format: FormatHealth::Unreadable("status file"),
            ..idle()
        };
        let b = again(&facts, &a, at(T + 10 * MIN));
        assert_eq!(b.memory.waiting, a.memory.waiting);
    }

    #[test]
    fn unknown_value_holds_while_alive_with_a_notice() {
        let facts = SessionFacts {
            format: FormatHealth::UnknownValue("status", "pondering".to_owned()),
            ..idle()
        };
        let a = first(&facts, NOW);
        assert!(a.holds);
        assert!(a.update_notice);
        assert_eq!(a.next, None);
    }

    #[test]
    fn unknown_value_gone_does_not_hold_but_wants_the_notice() {
        let facts = SessionFacts {
            format: FormatHealth::UnknownValue("status", "pondering".to_owned()),
            ..idle()
        };
        let a = decide(&facts, Alive::Gone, &SessionMemory::default(), NOW);
        assert!(!a.holds);
        assert!(a.update_notice);
    }

    // Edges of the number range.

    #[test]
    fn extreme_times_do_not_overflow() {
        let facts = SessionFacts {
            timers: vec![i64::MIN, i64::MAX, 0],
            ..waiting(Some(i64::MAX), true)
        };
        let ends = [
            Now {
                wall_ms: i64::MIN,
                mono_ms: 0,
            },
            Now {
                wall_ms: i64::MAX,
                mono_ms: u64::MAX,
            },
            Now {
                wall_ms: 0,
                mono_ms: u64::MAX / 2,
            },
        ];
        for now in ends {
            let a = first(&facts, now);
            for later_now in ends {
                let _ = again(&facts, &a, later_now);
            }
            let other = SessionFacts {
                activity: Activity::Waiting {
                    since_wall_ms: Some(i64::MIN),
                    remote: true,
                },
                ..facts.clone()
            };
            let _ = first(&other, now);
        }
    }

    // The deadline is exact: the answer stays the same until it and changes at it.

    /// How far ahead of `now` a deadline is, on its own clock.
    fn ahead(d: Deadline, now: Now) -> i64 {
        match d {
            Deadline::Wall(w) => w - now.wall_ms,
            Deadline::Mono(m) => i64::try_from(m).unwrap() - i64::try_from(now.mono_ms).unwrap(),
        }
    }

    fn activity() -> impl Strategy<Value = Activity> {
        prop_oneof![
            Just(Activity::Working),
            Just(Activity::Idle),
            (proptest::option::of(-2 * HOUR..2 * HOUR), any::<bool>()).prop_map(
                |(since, remote)| {
                    Activity::Waiting {
                        since_wall_ms: since.map(|s| T + s),
                        remote,
                    }
                }
            ),
        ]
    }

    fn format() -> impl Strategy<Value = FormatHealth> {
        prop_oneof![
            4 => Just(FormatHealth::Ok),
            1 => Just(FormatHealth::Unreadable("status file")),
            1 => Just(FormatHealth::UnknownValue("status", "pondering".to_owned())),
        ]
    }

    fn alive() -> impl Strategy<Value = Alive> {
        prop_oneof![
            4 => Just(Alive::Yes),
            1 => Just(Alive::Gone),
            1 => Just(Alive::Unknown),
        ]
    }

    fn facts() -> impl Strategy<Value = SessionFacts> {
        (
            activity(),
            proptest::collection::vec(-3 * HOUR..3 * HOUR, 0..4),
            0u32..2,
            format(),
        )
            .prop_map(|(activity, timers, open_tasks, format)| SessionFacts {
                timers: timers.into_iter().map(|t| T + t).collect(),
                open_tasks,
                format,
                ..session(activity)
            })
    }

    fn memory() -> impl Strategy<Value = SessionMemory> {
        let seen = (
            proptest::option::of(-2 * HOUR..2 * HOUR),
            -3 * HOUR..3 * HOUR,
            0..3 * HOUR,
        )
            .prop_map(|(since, wall_back, mono_back)| WaitSeen {
                since_wall_ms: since.map(|s| T + s),
                wall_ms: T - wall_back,
                mono_ms: M - mono_back as u64,
            });
        (alive(), proptest::option::of(seen))
            .prop_map(|(alive, waiting)| SessionMemory { alive, waiting })
    }

    proptest! {
        #[test]
        fn next_deadline_is_where_the_answer_changes(
            facts in facts(),
            alive in alive(),
            memory in memory(),
            pick in any::<u64>(),
            far in 0..10 * HOUR,
        ) {
            let a = decide(&facts, alive, &memory, NOW);
            let same = decide(&facts, alive, &a.memory, NOW);
            prop_assert_eq!(same, a);
            match a.next {
                Some(d) => {
                    let gap = ahead(d, NOW);
                    prop_assert!(gap > 0, "deadline {:?} is not after now", d);
                    let before = i64::try_from(pick % gap.unsigned_abs()).unwrap();
                    let b = decide(&facts, alive, &a.memory, later(NOW, before));
                    prop_assert_eq!(b.holds, a.holds, "changed {} ms before {:?}", gap - before, d);
                    let c = decide(&facts, alive, &a.memory, later(NOW, gap));
                    prop_assert_ne!(c.holds, a.holds, "no change at {:?}", d);
                }
                None => {
                    let b = decide(&facts, alive, &a.memory, later(NOW, far));
                    prop_assert_eq!(b.holds, a.holds, "changed {} ms later with no deadline", far);
                }
            }
        }
    }
}
