//! Time as the core sees it. The core never reads a clock: each event comes with a `Now`, so tests can use any time.

/// Both clocks, read once per event, so every rule in one step sees the same time.
///
/// Owner: core track (Phase 2). The tray loop reads it through [`Clock`] before each event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Now {
    /// Milliseconds since 1970-01-01 UTC. For times a person reads as clock time, such as a Keep awake end.
    pub wall_ms: i64,
    /// Milliseconds the PC has run, suspended time left out. For how long something ran, such as the closed-lid
    /// report; a clock change does not move it.
    pub mono_ms: u64,
}

/// When the core next needs a `Tick`, on the clock that fits the rule (architecture section 3, "Clocks").
///
/// Owner: core track (Phase 2). The tray loop may rely on this: it sets one timer to the nearer of this and the next
/// safety tick, and a `Wall` deadline can come early or late after a clock change, so the core re-checks on `Tick`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Deadline {
    Wall(i64),
    Mono(u64),
}

/// The one place the tray reads the time, so a fake clock can drive the loop in tests.
///
/// Owner: core track (Phase 2) for the trait; the real clock is `wakeshot_windows::clock::SystemClock` (Phase 3).
/// Callers may rely on `mono_ms` never going back; `wall_ms` may jump either way.
pub trait Clock {
    fn now(&self) -> Now;
}

#[cfg(test)]
mod tests {
    use super::Now;

    #[test]
    fn readings_compare_by_value() {
        let a = Now {
            wall_ms: 1_767_225_600_000,
            mono_ms: 42,
        };
        let b = a;
        assert_eq!(a, b);
        assert_ne!(a, Now { mono_ms: 43, ..a });
    }
}
