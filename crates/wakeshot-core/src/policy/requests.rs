//! Which power requests the holds need.
//!
//! Owner: `p2-policy-power`.

use crate::command::Command;

/// What the request rule looks at. Named fields, so two flags cannot swap places at a call.
///
/// Owner: core track (Phase 2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RequestFacts {
    /// At least one hold counts.
    pub holding: bool,
    /// Keep screen on is asked for (the Keep awake hold's `screen`).
    pub keep_screen_on: bool,
    /// On battery under the floor (`policy::floor`).
    pub stepped_back: bool,
    /// This session is on the PC's own screen and keyboard.
    pub on_console: bool,
}

/// The power requests to hold. `system` means SYSTEM plus EXECUTION (decision 15).
///
/// Owner: core track (Phase 2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Requests {
    pub system: bool,
    pub display: bool,
}

impl Requests {
    pub const NONE: Self = Self {
        system: false,
        display: false,
    };

    /// Any request is on, so a resume needs `Retake` (decision 16).
    pub fn any(self) -> bool {
        self.system || self.display
    }

    pub fn command(self) -> Command {
        Command::Requests {
            system: self.system,
            display: self.display,
        }
    }
}

/// No request with no hold, and none while stepped back, so Windows' own rules apply. The display only with Keep
/// screen on and on the console: when another user has the PC's screen, it must still turn off so Windows locks their
/// session (architecture section 3, the console bullet).
pub fn wanted(facts: RequestFacts) -> Requests {
    let system = facts.holding && !facts.stepped_back;
    Requests {
        system,
        display: system && facts.keep_screen_on && facts.on_console,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(
        holding: bool,
        keep_screen_on: bool,
        stepped_back: bool,
        on_console: bool,
    ) -> RequestFacts {
        RequestFacts {
            holding,
            keep_screen_on,
            stepped_back,
            on_console,
        }
    }

    const SYSTEM_ONLY: Requests = Requests {
        system: true,
        display: false,
    };

    #[test]
    fn no_hold_takes_no_request() {
        assert_eq!(wanted(facts(false, false, false, true)), Requests::NONE);
        assert_eq!(wanted(facts(false, true, false, true)), Requests::NONE);
    }

    #[test]
    fn a_hold_takes_the_system_request_only() {
        assert_eq!(wanted(facts(true, false, false, true)), SYSTEM_ONLY);
    }

    #[test]
    fn keep_screen_on_adds_the_display_on_the_console() {
        let both = Requests {
            system: true,
            display: true,
        };
        assert_eq!(wanted(facts(true, true, false, true)), both);
    }

    #[test]
    fn off_the_console_keep_screen_on_holds_the_system_request_only() {
        assert_eq!(wanted(facts(true, true, false, false)), SYSTEM_ONLY);
        assert_eq!(wanted(facts(true, false, false, false)), SYSTEM_ONLY);
    }

    #[test]
    fn stepped_back_takes_no_request_even_with_keep_screen_on() {
        assert_eq!(wanted(facts(true, false, true, true)), Requests::NONE);
        assert_eq!(wanted(facts(true, true, true, true)), Requests::NONE);
    }

    #[test]
    fn every_combination_follows_the_rule() {
        for bits in 0..16u8 {
            let f = facts(bits & 1 != 0, bits & 2 != 0, bits & 4 != 0, bits & 8 != 0);
            let r = wanted(f);
            assert_eq!(r.system, f.holding && !f.stepped_back, "{f:?}");
            assert_eq!(
                r.display,
                r.system && f.keep_screen_on && f.on_console,
                "{f:?}"
            );
        }
    }

    #[test]
    fn any_request_on_means_a_retake_after_resume() {
        assert!(!Requests::NONE.any());
        assert!(SYSTEM_ONLY.any());
    }

    #[test]
    fn requests_become_the_command() {
        let r = Requests {
            system: true,
            display: true,
        };
        assert_eq!(
            r.command(),
            Command::Requests {
                system: true,
                display: true
            }
        );
        assert_eq!(
            Requests::NONE.command(),
            Command::Requests {
                system: false,
                display: false
            }
        );
    }
}
