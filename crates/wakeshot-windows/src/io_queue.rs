//! The IO thread's queue of commands, with the end-of-session rule (architecture section 4, "Tray threads").
//!
//! Owner: power track (`p3-power-io`). The signatures are contract: the loop (integration) pushes, the IO thread pops.

use std::collections::VecDeque;
use std::sync::{Condvar, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use wakeshot_core::command::{Command, RestoreWhy};

/// Commands for the IO thread, in order. The loop and the IO thread share one (`Arc<IoQueue>`).
///
/// At `WM_ENDSESSION` the loop calls `session_ending`: it queues `Restore { plan: None, why: SessionEnd }` first,
/// drops the queued `Notify` and `Apply` commands, and refuses new ones until `session_resumed` (a cancelled end). The
/// loop needs no `push_front` of its own for that restore; one it already pushed is not doubled.
///
/// Callers may rely on: commands come out in the order they went in, except that `push_front` and `session_ending` put
/// theirs first; a command the queue drops or refuses gets no answer; each push wakes a waiting `pop`. The queue's own
/// restore answers as `Done(Restored { why: SessionEnd })` like any other, so the core may get that answer before it
/// issues its own `SessionEnd` restore.
#[derive(Debug, Default)]
pub struct IoQueue {
    state: Mutex<Queued>,
    ready: Condvar,
}

#[derive(Debug, Default)]
struct Queued {
    commands: VecDeque<Command>,
    /// Between `session_ending` and `session_resumed`.
    ending: bool,
}

// The loop and the IO thread share it, so a field that is not `Send + Sync` must fail this crate's build.
const _: () = {
    const fn is_send_sync<T: Send + Sync>() {}
    is_send_sync::<IoQueue>();
};

/// The restore an ending session runs before anything else queued.
const SESSION_END_RESTORE: Command = Command::Restore {
    plan: None,
    why: RestoreWhy::SessionEnd,
};

/// Dropped and refused while the session ends: a notice nobody will see, and an apply that would override settings
/// right before Windows ends the process (architecture section 4).
fn held_back(command: &Command) -> bool {
    matches!(command, Command::Notify(_) | Command::Apply { .. })
}

impl IoQueue {
    pub fn new() -> Self {
        Self::default()
    }

    /// Queues `command` last. While the session ends, a `Notify` or `Apply` is refused.
    pub fn push(&self, command: Command) {
        self.add(command, |commands, command| commands.push_back(command));
    }

    /// Queues `command` first, so the IO thread runs it next. Refused like `push`.
    pub fn push_front(&self, command: Command) {
        self.add(command, |commands, command| commands.push_front(command));
    }

    /// Drops every queued command `pick` returns `true` for; returns how many.
    pub fn drop_queued(&self, pick: &dyn Fn(&Command) -> bool) -> usize {
        let mut state = self.lock();
        let before = state.commands.len();
        state.commands.retain(|command| !pick(command));
        before - state.commands.len()
    }

    /// The next command, or `None` after `timeout_ms`.
    pub fn pop(&self, timeout_ms: u32) -> Option<Command> {
        let timeout = Duration::from_millis(u64::from(timeout_ms));
        // `wait_timeout_while` keeps the deadline across spurious wake-ups.
        let (mut state, _) = self
            .ready
            .wait_timeout_while(self.lock(), timeout, |s| s.commands.is_empty())
            .unwrap_or_else(PoisonError::into_inner);
        state.commands.pop_front()
    }

    /// The end of the session (`WM_ENDSESSION`): the restore first, queued notices and applies dropped, new ones
    /// refused.
    pub fn session_ending(&self) {
        let mut state = self.lock();
        state.ending = true;
        // One restore at the front, also when the loop already pushed one: a second would only delay what follows.
        state
            .commands
            .retain(|command| !held_back(command) && *command != SESSION_END_RESTORE);
        state.commands.push_front(SESSION_END_RESTORE);
        drop(state);
        self.ready.notify_one();
    }

    /// The end of the session was cancelled: notices and applies are taken again. What was dropped stays dropped.
    pub fn session_resumed(&self) {
        self.lock().ending = false;
    }

    fn add(&self, command: Command, put: impl FnOnce(&mut VecDeque<Command>, Command)) {
        let mut state = self.lock();
        if state.ending && held_back(&command) {
            return;
        }
        put(&mut state.commands, command);
        drop(state);
        self.ready.notify_one();
    }

    /// The queue stays usable after a panic elsewhere: a `pick` closure that panicked leaves the commands whole (a
    /// subset), and a panicking IO thread aborts the tray anyway (tray startup step 5).
    fn lock(&self) -> MutexGuard<'_, Queued> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::thread;
    use std::time::Instant;

    use wakeshot_core::settings::{Guid, SettingKey};
    use wakeshot_core::view::Notice;

    use super::*;

    const BALANCED: Guid = Guid(0x381b4222_f694_41f0_9685_ff5bb260df2e);

    fn apply() -> Command {
        Command::Apply {
            plan: BALANCED,
            writes: vec![(SettingKey::LidDc, 0)],
        }
    }

    fn restore(why: RestoreWhy) -> Command {
        Command::Restore { plan: None, why }
    }

    fn notify() -> Command {
        Command::Notify(Notice::RestoreFailed)
    }

    fn log(text: &str) -> Command {
        Command::Log(text.to_owned())
    }

    /// Everything queued now, in order.
    fn drain(queue: &IoQueue) -> Vec<Command> {
        std::iter::from_fn(|| queue.pop(0)).collect()
    }

    #[test]
    fn queue_keeps_order() {
        let queue = IoQueue::new();
        queue.push(Command::ReadPlan);
        queue.push(apply());
        queue.push(log("a"));
        queue.push(notify());
        queue.push(restore(RestoreWhy::WorkEnded));
        assert_eq!(
            drain(&queue),
            [
                Command::ReadPlan,
                apply(),
                log("a"),
                notify(),
                restore(RestoreWhy::WorkEnded)
            ]
        );
        assert_eq!(queue.pop(0), None);
    }

    #[test]
    fn push_front_runs_next() {
        let queue = IoQueue::new();
        queue.push(Command::ReadPlan);
        queue.push(log("a"));
        queue.push_front(restore(RestoreWhy::Quit));
        assert_eq!(queue.pop(0), Some(restore(RestoreWhy::Quit)));
        queue.push_front(log("b"));
        queue.push_front(log("c"));
        assert_eq!(
            drain(&queue),
            [log("c"), log("b"), Command::ReadPlan, log("a")]
        );
    }

    #[test]
    fn drop_queued_drops_only_what_it_picks() {
        let queue = IoQueue::new();
        for command in [log("a"), Command::ReadPlan, log("b"), apply()] {
            queue.push(command);
        }
        let dropped = queue.drop_queued(&|c| matches!(c, Command::Log(_)));
        assert_eq!(dropped, 2);
        assert_eq!(queue.drop_queued(&|_| false), 0);
        assert_eq!(drain(&queue), [Command::ReadPlan, apply()]);
    }

    #[test]
    fn session_ending_drops_notices_and_applies() {
        let queue = IoQueue::new();
        for command in [
            Command::ReadPlan,
            notify(),
            apply(),
            log("kept"),
            Command::WriteConfig(wakeshot_core::files::Config::new(true)),
            apply(),
            restore(RestoreWhy::WorkEnded),
            notify(),
        ] {
            queue.push(command);
        }
        queue.session_ending();
        assert_eq!(
            drain(&queue),
            [
                restore(RestoreWhy::SessionEnd),
                Command::ReadPlan,
                log("kept"),
                Command::WriteConfig(wakeshot_core::files::Config::new(true)),
                restore(RestoreWhy::WorkEnded),
            ]
        );
    }

    #[test]
    fn session_ending_keeps_one_restore_first() {
        let queue = IoQueue::new();
        queue.push(log("a"));
        // The loop pushed its own before the call, as the old stub doc told it to.
        queue.push_front(restore(RestoreWhy::SessionEnd));
        queue.push(restore(RestoreWhy::SessionEnd));
        queue.session_ending();
        queue.session_ending();
        assert_eq!(drain(&queue), [restore(RestoreWhy::SessionEnd), log("a")]);
    }

    #[test]
    fn session_ending_refuses_new_applies() {
        let queue = IoQueue::new();
        queue.session_ending();
        queue.push(apply());
        queue.push(notify());
        queue.push_front(apply());
        queue.push_front(notify());
        queue.push(restore(RestoreWhy::SessionEnd));
        queue.push(Command::ReadPlan);
        queue.push_front(log("first"));
        assert_eq!(
            drain(&queue),
            [
                log("first"),
                restore(RestoreWhy::SessionEnd),
                restore(RestoreWhy::SessionEnd),
                Command::ReadPlan,
            ]
        );
    }

    #[test]
    fn session_resumed_accepts_again() {
        let queue = IoQueue::new();
        queue.push(apply());
        queue.session_ending();
        queue.push(apply());
        queue.session_resumed();
        queue.push(apply());
        queue.push(notify());
        queue.push_front(notify());
        assert_eq!(
            drain(&queue),
            [notify(), restore(RestoreWhy::SessionEnd), apply(), notify()]
        );
    }

    #[test]
    fn pop_times_out() {
        let queue = IoQueue::new();
        let start = Instant::now();
        assert_eq!(queue.pop(50), None);
        assert!(start.elapsed() >= Duration::from_millis(50));
        assert_eq!(queue.pop(0), None);
        queue.push(log("a"));
        let start = Instant::now();
        assert_eq!(queue.pop(10_000), Some(log("a")));
        assert!(start.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn queue_works_across_threads() {
        const COUNT: usize = 2_000;
        let queue = Arc::new(IoQueue::new());

        // The IO thread side: it waits first, so the first push must wake it.
        let popper = Arc::clone(&queue);
        let io = thread::spawn(move || {
            let mut got = Vec::new();
            while let Some(command) = popper.pop(10_000) {
                if command == Command::Quit {
                    break;
                }
                got.push(command);
            }
            got
        });
        thread::sleep(Duration::from_millis(50));

        // Two loop-side threads push at once; each one's own order must hold.
        let pushers: Vec<_> = ["a", "b"]
            .into_iter()
            .map(|tag| {
                let pusher = Arc::clone(&queue);
                thread::spawn(move || {
                    for n in 0..COUNT {
                        pusher.push(log(&format!("{tag}{n}")));
                    }
                })
            })
            .collect();
        for pusher in pushers {
            pusher.join().expect("a pusher ends");
        }
        queue.push(Command::Quit);
        let got = io.join().expect("the popper ends");

        assert_eq!(got.len(), 2 * COUNT);
        for tag in ["a", "b"] {
            let mine: Vec<Command> = got
                .iter()
                .filter(|c| matches!(c, Command::Log(text) if text.starts_with(tag)))
                .cloned()
                .collect();
            let sent: Vec<Command> = (0..COUNT).map(|n| log(&format!("{tag}{n}"))).collect();
            assert_eq!(mine, sent);
        }

        // A push, a push to the front and `session_ending` each wake a pop that waits, long before its timeout.
        type Waker = (Command, fn(&IoQueue));
        let wakers: [Waker; 3] = [
            (log("pushed"), |q| q.push(log("pushed"))),
            (log("in front"), |q| q.push_front(log("in front"))),
            (restore(RestoreWhy::SessionEnd), IoQueue::session_ending),
        ];
        for (expected, wake) in wakers {
            let popper = Arc::clone(&queue);
            let io = thread::spawn(move || {
                let start = Instant::now();
                (popper.pop(10_000), start.elapsed())
            });
            thread::sleep(Duration::from_millis(50));
            wake(&queue);
            let (got, took) = io.join().expect("the popper ends");
            assert_eq!(got, Some(expected));
            assert!(took < Duration::from_secs(5), "{took:?}");
        }
    }
}
