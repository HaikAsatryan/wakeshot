//! The IO thread's queue of commands, with the end-of-session rule (architecture section 4, "Tray threads").
//!
//! Owner: power track (`p3-power-io`). The signatures are contract: the loop (integration) pushes, the IO thread pops.

use wakeshot_core::command::Command;

/// Commands for the IO thread, in order. At `WM_ENDSESSION` the loop pushes the restore with `push_front` and calls
/// `session_ending`, after which queued notices and `Apply` commands are dropped; `session_resumed` lifts that.
#[derive(Debug, Default)]
pub struct IoQueue {
    _private: (),
}

impl IoQueue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&self, command: Command) {
        let _ = command;
        unimplemented!("p3-power-io")
    }

    pub fn push_front(&self, command: Command) {
        let _ = command;
        unimplemented!("p3-power-io")
    }

    /// Drops every queued command `drop` picks; returns how many.
    pub fn drop_queued(&self, drop: &dyn Fn(&Command) -> bool) -> usize {
        let _ = drop;
        unimplemented!("p3-power-io")
    }

    /// The next command, or `None` after `timeout_ms`.
    pub fn pop(&self, timeout_ms: u32) -> Option<Command> {
        let _ = timeout_ms;
        unimplemented!("p3-power-io")
    }

    pub fn session_ending(&self) {
        unimplemented!("p3-power-io")
    }

    pub fn session_resumed(&self) {
        unimplemented!("p3-power-io")
    }
}
