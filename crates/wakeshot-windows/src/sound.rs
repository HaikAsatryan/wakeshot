//! The sound seam (architecture section 2, `SoundPlayer`): Wakeshot's chimes, the device rule, the WASAPI player.

mod chime;
mod device;
mod player;

#[cfg(any(test, feature = "test-support"))]
use std::sync::{Arc, Mutex};

pub use device::{Endpoint, PickWhy, form_factor_name, pick, picked_endpoint};
pub use player::{WasapiPlayer, current_scheme};

use wakeshot_core::command::SoundEnd;
use wakeshot_core::view::Cue;

use crate::WinError;
#[cfg(any(test, feature = "test-support"))]
use crate::sink::EventSink;

/// Plays one of Wakeshot's chimes on the device the device rule picks.
///
/// Callers may rely on:
///
/// - `play` never blocks, so the lid-close lock is never held up by a sound; its result comes back as
///   `Event::Done(Outcome::Sound(..))` through the event sink, for the log only;
/// - `play_and_wait` blocks until the cue played out or 3 s passed; only the hibernate thread calls it, after its
///   last check (`Command::Hibernate`).
pub trait SoundPlayer: Send + Sync {
    fn play(&self, cue: Cue);
    fn play_and_wait(&self, cue: Cue) -> Result<SoundEnd, WinError>;
}

/// Records each cue in order and plays nothing. With `sink`, `play` posts `Done(Sound(Ok(Played)))` as the real
/// player does; `wait_error` makes `play_and_wait` fail.
#[cfg(any(test, feature = "test-support"))]
#[derive(Default)]
pub struct FakeSoundPlayer {
    pub played: Mutex<Vec<Cue>>,
    pub sink: Option<Arc<dyn EventSink>>,
    pub wait_error: Option<WinError>,
}

#[cfg(any(test, feature = "test-support"))]
impl SoundPlayer for FakeSoundPlayer {
    fn play(&self, cue: Cue) {
        self.played
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(cue);
        if let Some(sink) = &self.sink {
            use wakeshot_core::command::Outcome;
            use wakeshot_core::event::Event;
            // The real player logs a failed post; the fake has nobody to tell.
            let _ = sink.post(Event::Done(Outcome::Sound(Ok(SoundEnd::Played))));
        }
    }

    fn play_and_wait(&self, cue: Cue) -> Result<SoundEnd, WinError> {
        self.played
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(cue);
        self.wait_error.map_or(Ok(SoundEnd::Played), Err)
    }
}
