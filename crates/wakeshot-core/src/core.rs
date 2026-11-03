//! The core's entry point. Its state and rules come with the core track; until then it answers nothing, so the loop
//! can be wired and tested against the real signatures.

use crate::command::Command;
use crate::event::Event;
use crate::files::Config;
use crate::time::{Deadline, Now};

/// Wakeshot's decisions: events in, commands out, no I/O and no clock reads.
///
/// Owner: core track (Phase 2) builds the state and every rule behind these three calls; their signatures are the
/// contract. The loop may rely on: `handle` never blocks and never fails, and `next_deadline` is read after each
/// `handle` to set the one timer. The config in `Event::Started` wins over the one given to `new`.
#[derive(Debug)]
pub struct Core {
    _private: (),
}

impl Core {
    pub fn new(config: Config) -> Self {
        let _ = config;
        Self { _private: () }
    }

    pub fn handle(&mut self, now: Now, event: Event) -> Vec<Command> {
        let _ = (now, event);
        Vec::new()
    }

    pub fn next_deadline(&self) -> Option<Deadline> {
        None
    }
}
