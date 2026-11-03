//! How threads off the loop hand events to the core: the IO thread, system callbacks, the hibernate, watch and sound
//! threads, and the menu (architecture section 4, "Tray threads").
//!
//! Owner: contracts card. The real sink (boxed events posted to the loop's window) comes with the integration step,
//! which owns the loop and its window, so one place boxes and unboxes an `Event`.

#[cfg(any(test, feature = "test-support"))]
use std::sync::Mutex;

use wakeshot_core::event::Event;

use crate::WinError;

/// Posts one event to the loop. Callers may rely on: `post` never blocks on the loop; an error means the event is
/// lost (the loop's message queue is full or gone), so the caller logs it and the next safety tick recovers.
pub trait EventSink: Send + Sync {
    fn post(&self, event: Event) -> Result<(), WinError>;
}

/// Records every event posted; `fail_with` makes every post fail.
///
/// Owner: contracts card.
#[cfg(any(test, feature = "test-support"))]
#[derive(Debug, Default)]
pub struct FakeEventSink {
    pub posted: Mutex<Vec<Event>>,
    pub fail_with: Option<WinError>,
}

#[cfg(any(test, feature = "test-support"))]
impl FakeEventSink {
    /// The events posted so far, also after a poster panicked.
    pub fn events(&self) -> Vec<Event> {
        self.posted
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}

#[cfg(any(test, feature = "test-support"))]
impl EventSink for FakeEventSink {
    fn post(&self, event: Event) -> Result<(), WinError> {
        if let Some(e) = self.fail_with {
            return Err(e);
        }
        self.posted
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(event);
        Ok(())
    }
}
