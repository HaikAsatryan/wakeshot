//! The tray mode's wiring: the loop window, the dispatcher, the IO thread, startup and the composition root
//! (architecture section 4, "Tray threads" and "Tray startup").
//!
//! The seams here keep the parts apart: the window calls the dispatcher only through [`LoopHandler`], and the
//! dispatcher talks to the IO thread only through `io_queue::IoQueue` and [`Ack`].

pub mod dispatch;
pub mod io_thread;
pub mod startup;
pub mod tray_mode;
pub mod window;

use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::time::Duration;

use wakeshot_core::event::{Event, SessionChange};

/// Whether the loop goes on after a handler call. `Quit` ends the pump; the shutdown follows (architecture, "Tray
/// startup" step 8).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flow {
    Go,
    Quit,
}

/// Called by `app::window` for every message the loop window owns; `app::dispatch` implements it.
///
/// Every call comes on the loop thread only. A call can re-enter: a runner that pumps messages (the modal menu, a
/// sent message) lets the window call the handler again before the outer call returns. The handler then queues the
/// event and returns, and the outer call feeds the queue to the core before it returns, so the core is never called
/// while a call into it is on the stack. After every call the window re-arms its one timer with `next_timer_ms`.
///
/// Every method takes `&self`, and the window holds a `&dyn LoopHandler`: a re-entered call through `&mut self` would
/// make a second `&mut` while the outer one is still live, which is undefined behavior even when the inner call only
/// queues. So an implementation keeps its state in cells.
pub trait LoopHandler {
    /// An event posted through `WindowSink`.
    fn event(&self, event: Event) -> Flow;

    /// A change from `session::decode`, end-of-session messages included. Returns only after the end-of-session wait
    /// (the `Ack`, up to 4 s), also when the call re-enters: Windows may end the process once the window answers
    /// `WM_ENDSESSION` (architecture section 4, "Tray threads").
    fn session(&self, change: SessionChange) -> Flow;

    /// The one timer fired.
    fn timer(&self) -> Flow;

    /// Milliseconds until the timer should fire next: the nearer of the core's next deadline and the next safety
    /// tick, at least 10 (`USER_TIMER_MINIMUM`).
    fn next_timer_ms(&self) -> u32;
}

/// A one-shot signal: the IO thread sets it after the end-of-session restore; the loop waits on it.
///
/// Clones share one signal, so the IO thread and the loop each keep their own. A poisoned lock counts as set, also
/// after `reset`: a panicking IO thread aborts the process anyway (tray startup step 5), and a loop that waited on a
/// lock nobody can set would hold up the end of the session for nothing.
#[derive(Clone, Debug, Default)]
pub struct Ack(Arc<(Mutex<bool>, Condvar)>);

// The IO thread and the loop share it, so a field that is not `Send + Sync` must fail this crate's build.
const _: () = {
    const fn is_send_sync<T: Send + Sync>() {}
    is_send_sync::<Ack>();
};

impl Ack {
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the signal and wakes every waiter. It stays set until `reset`.
    pub fn set(&self) {
        let (lock, wake) = &*self.0;
        *lock.lock().unwrap_or_else(PoisonError::into_inner) = true;
        wake.notify_all();
    }

    /// Clears the signal, for an end of session that was cancelled.
    pub fn reset(&self) {
        let (lock, _) = &*self.0;
        if let Ok(mut set) = lock.lock() {
            *set = false;
        }
    }

    /// `true` when the signal is set, now or within `timeout_ms`; `false` after the timeout.
    pub fn wait(&self, timeout_ms: u32) -> bool {
        let (lock, wake) = &*self.0;
        let Ok(set) = lock.lock() else {
            return true;
        };
        let timeout = Duration::from_millis(u64::from(timeout_ms));
        // `wait_timeout_while` keeps the deadline across spurious wake-ups, and the value read last decides, so a set
        // that lands right at the timeout still counts.
        match wake.wait_timeout_while(set, timeout, |set| !*set) {
            Ok((set, _)) => *set,
            Err(_) => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::thread;
    use std::time::{Duration, Instant};

    use super::*;

    #[test]
    fn set_before_wait_returns_true_at_once() {
        let ack = Ack::new();
        ack.set();
        let start = Instant::now();
        assert!(ack.wait(10_000));
        assert!(start.elapsed() < Duration::from_secs(1));
        // It stays set: a second wait also returns at once.
        assert!(ack.wait(0));
    }

    #[test]
    fn wait_times_out_false() {
        let ack = Ack::new();
        let start = Instant::now();
        assert!(!ack.wait(50));
        assert!(start.elapsed() >= Duration::from_millis(50));
        assert!(!ack.wait(0));
    }

    #[test]
    fn set_from_another_thread_wakes_a_waiter() {
        let ack = Ack::new();
        let setter = ack.clone();
        let start = Instant::now();
        let handle = thread::spawn(move || {
            thread::sleep(Duration::from_millis(50));
            setter.set();
        });
        assert!(ack.wait(10_000));
        let took = start.elapsed();
        handle.join().expect("the setter thread ends");
        assert!(took >= Duration::from_millis(50));
        // Far below the 10 s timeout: the set woke the waiter, the timeout did not.
        assert!(took < Duration::from_secs(5));
    }

    #[test]
    fn reset_clears() {
        let ack = Ack::new();
        ack.set();
        ack.reset();
        assert!(!ack.wait(20));
        ack.set();
        assert!(ack.wait(0));
    }

    #[test]
    fn poisoned_lock_counts_as_set() {
        let ack = Ack::new();
        let poisoner = ack.clone();
        let died = thread::spawn(move || {
            let _held = poisoner.0.0.lock().expect("not poisoned yet");
            panic!("poisons the lock on purpose");
        })
        .join();
        assert!(died.is_err());
        assert!(ack.wait(0));
        ack.reset();
        assert!(ack.wait(0));
    }
}
