//! The loop window: the hidden top-level window, the message pump, the one timer, `WindowSink` and `ask_quit`.
//!
//! `WindowSink` is the real `EventSink`; the window calls `LoopHandler`. The pump takes the handler shared: a nested
//! pump calls the handler again while an outer call runs, and only shared references make that sound (the
//! `LoopHandler` doc in `app.rs`).
//!
//! An event message carries a number, not a pointer: the event waits in the sink under that number until the window
//! takes it out. So a stray `WM_APP + 1` (another program's broadcast) can never make the window read or free a bad
//! pointer, and a quit drops every waiting event at once, wherever its message is in the queue.

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use wakeshot_core::event::{Event, SessionChange};
use wakeshot_core::view::MenuCommand;
use windows::Win32::Foundation::{
    ERROR_CLASS_ALREADY_EXISTS, ERROR_INVALID_WINDOW_HANDLE, ERROR_SUCCESS, GetLastError, HWND,
    LPARAM, LRESULT, SetLastError, WIN32_ERROR, WPARAM,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, FindWindowW, GWLP_USERDATA,
    GetMessageW, GetWindowLongPtrW, KillTimer, MSG, PM_REMOVE, PeekMessageW, PostMessageW,
    PostQuitMessage, RegisterClassExW, SetTimer, SetWindowLongPtrW, TranslateMessage,
    WINDOW_EX_STYLE, WM_APP, WM_CLOSE, WM_ENDSESSION, WM_QUERYENDSESSION, WM_TIMER,
    WM_WTSSESSION_CHANGE, WNDCLASSEXW, WS_OVERLAPPED,
};
use windows::core::{PCWSTR, w};

use super::{Flow, LoopHandler};
use crate::WinError;
use crate::session;
use crate::sink::EventSink;

/// An event from `WindowSink`.
const WM_EVENT: u32 = WM_APP + 1;

/// The `wparam` of every event message. A stray `WM_APP + 1` almost never carries it, so its `lparam` is never taken
/// for one of ours, and a waiting event never jumps the queue.
const EVENT_TAG: usize = 0x5741_4B45;

const TIMER_ID: usize = 1;

/// `USER_TIMER_MINIMUM`.
const TIMER_MIN_MS: u32 = 10;

thread_local! {
    /// This thread's loop window while it lives (0: none), for [`pass_close_to_loop`].
    static LOOP_WINDOW: Cell<isize> = const { Cell::new(0) };
}

/// The loop window's class: `Wakeshot.Loop`, or `Wakeshot.Loop.DryRun`, so a dry run and a real tray never find each
/// other.
fn class(dry_run: bool) -> PCWSTR {
    if dry_run {
        w!("Wakeshot.Loop.DryRun")
    } else {
        w!("Wakeshot.Loop")
    }
}

/// The real `EventSink`: posts `WM_APP + 1` to the loop window. Any thread may post, and `post` never waits on the
/// loop.
///
/// After the loop quits, or its window goes, every post fails with `PostMessageW` 1400 (`ERROR_INVALID_WINDOW_HANDLE`)
/// and drops its event. A full queue (about 10,000 posted messages) fails with 1816 (`ERROR_NOT_ENOUGH_QUOTA`).
#[derive(Debug)]
pub struct WindowSink {
    /// The loop window as a number: `HWND` is not `Send`, and every thread posts to it.
    hwnd: isize,
    ledger: Mutex<Ledger>,
}

/// The events posted and not yet taken by the window, under the number their message carries.
#[derive(Debug)]
struct Ledger {
    open: bool,
    next_id: usize,
    waiting: HashMap<usize, Event>,
}

impl WindowSink {
    fn new(hwnd: HWND) -> Self {
        Self {
            hwnd: hwnd.0 as isize,
            ledger: Mutex::new(Ledger {
                open: true,
                next_id: 1,
                waiting: HashMap::new(),
            }),
        }
    }

    fn ledger(&self) -> MutexGuard<'_, Ledger> {
        // A poster that panicked leaves a whole map behind; the loop should still get the other events.
        self.ledger.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn take(&self, id: usize) -> Option<Event> {
        self.ledger().waiting.remove(&id)
    }

    /// Fails every later post and drops every event still waiting.
    fn close(&self) {
        let dropped = {
            let mut ledger = self.ledger();
            ledger.open = false;
            std::mem::take(&mut ledger.waiting)
        };
        drop(dropped);
    }

    /// The events posted and not yet taken or dropped.
    #[cfg(test)]
    fn waiting(&self) -> usize {
        self.ledger().waiting.len()
    }
}

impl EventSink for WindowSink {
    fn post(&self, event: Event) -> Result<(), WinError> {
        let id = {
            let mut ledger = self.ledger();
            if !ledger.open {
                return Err(WinError {
                    call: "PostMessageW",
                    code: ERROR_INVALID_WINDOW_HANDLE.0,
                });
            }
            let id = ledger.next_id;
            ledger.next_id += 1;
            ledger.waiting.insert(id, event);
            id
        };
        // SAFETY: plain call; Windows checks the handle, and the message carries numbers only.
        let posted = unsafe {
            PostMessageW(
                Some(HWND(self.hwnd as *mut _)),
                WM_EVENT,
                WPARAM(EVENT_TAG),
                LPARAM(id as isize),
            )
        };
        posted.map_err(|e| {
            // Take the event back, so a lost post leaves nothing behind; `close` may have dropped it already.
            let _back = self.ledger().waiting.remove(&id);
            win_error("PostMessageW", &e)
        })
    }
}

/// One handler call the window makes.
#[derive(Debug)]
enum Call {
    Event(Event),
    Session(SessionChange),
    Timer,
}

/// What the window procedure reads through `GWLP_USERDATA`, on the loop thread only.
#[derive(Debug)]
struct WindowState {
    sink: Arc<WindowSink>,
    /// While `run` pumps: points at `run`'s own `&dyn LoopHandler`. Null before and after.
    handler: Cell<*const ()>,
    /// Calls that came before `run` (startup code that pumped messages), for `run` to make first, in order.
    early: RefCell<VecDeque<Call>>,
    /// Set once a call returned `Quit`, or the window is going: no handler call after it.
    quitting: Cell<bool>,
}

impl WindowState {
    fn handler(&self) -> Option<&dyn LoopHandler> {
        let slot = self.handler.get().cast::<&dyn LoopHandler>();
        // SAFETY: null, or `run`'s own `&dyn LoopHandler`, which lives through every message `run` pumps; `run` clears
        // the pointer before that reference goes.
        unsafe { slot.as_ref() }.copied()
    }

    fn call(&self, hwnd: HWND, call: Call) {
        if self.quitting.get() {
            return;
        }
        let Some(handler) = self.handler() else {
            // The timer is armed only by `run`, so no timer call can be lost here.
            if !matches!(call, Call::Timer) {
                self.early.borrow_mut().push_back(call);
            }
            return;
        };
        let flow = match call {
            Call::Event(event) => handler.event(event),
            Call::Session(change) => handler.session(change),
            Call::Timer => handler.timer(),
        };
        // A nested call may have quit while this one ran; its quit wins.
        if self.quitting.get() {
            return;
        }
        match flow {
            Flow::Go => self.arm(hwnd, handler),
            Flow::Quit => self.quit(hwnd),
        }
    }

    fn arm(&self, hwnd: HWND, handler: &dyn LoopHandler) {
        let ms = handler.next_timer_ms().max(TIMER_MIN_MS);
        // SAFETY: plain call on this thread's own window. A failed arm is left alone: the next handler call arms
        // again, and there is nothing else to do about it.
        unsafe { SetTimer(Some(hwnd), TIMER_ID, ms, None) };
    }

    fn quit(&self, hwnd: HWND) {
        self.quitting.set(true);
        // Closed at once, not when `run` returns: a busy poster could otherwise keep the queue full and `WM_QUIT`
        // from ever coming.
        self.sink.close();
        // SAFETY: plain calls on this thread's own window and queue.
        unsafe {
            let _ = KillTimer(Some(hwnd), TIMER_ID);
            PostQuitMessage(0);
        }
    }
}

/// The loop thread's hidden top-level window and its pump. Not `Send`: a window belongs to the thread that made it,
/// and only that thread's pump gets its messages.
#[derive(Debug)]
pub struct LoopWindow {
    hwnd: HWND,
    state: Rc<WindowState>,
}

impl LoopWindow {
    /// Registers the class (`Wakeshot.Loop`, or `Wakeshot.Loop.DryRun`) and makes the window on the calling thread.
    /// Top-level, not message-only: Windows sends `WM_QUERYENDSESSION` and `WM_ENDSESSION` to top-level windows only.
    /// Never shown.
    pub fn new(dry_run: bool) -> Result<LoopWindow, WinError> {
        Self::with_class(class(dry_run))
    }

    fn with_class(class: PCWSTR) -> Result<LoopWindow, WinError> {
        // SAFETY: plain call; `None` names this exe.
        let module =
            unsafe { GetModuleHandleW(None) }.map_err(|e| win_error("GetModuleHandleW", &e))?;
        let wc = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(window_proc),
            hInstance: module.into(),
            lpszClassName: class,
            ..Default::default()
        };
        // SAFETY: `wc` and the class name it points at outlive the call.
        if unsafe { RegisterClassExW(&wc) } == 0 {
            // SAFETY: no arguments.
            let error = unsafe { GetLastError() };
            // A second window of the class in this process (tests) finds it registered.
            if error != ERROR_CLASS_ALREADY_EXISTS {
                return Err(WinError {
                    call: "RegisterClassExW",
                    code: error.0,
                });
            }
        }
        // SAFETY: the class is registered above; the window is never shown and is destroyed in `Drop`, on this thread.
        let hwnd = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                class,
                w!("Wakeshot"),
                WS_OVERLAPPED,
                0,
                0,
                0,
                0,
                None,
                None,
                Some(module.into()),
                None,
            )
        }
        .map_err(|e| win_error("CreateWindowExW", &e))?;
        let state = Rc::new(WindowState {
            sink: Arc::new(WindowSink::new(hwnd)),
            handler: Cell::new(std::ptr::null()),
            early: RefCell::default(),
            quitting: Cell::new(false),
        });
        // SAFETY: the window made above; `Drop` clears the slot before the state can go.
        unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, Rc::as_ptr(&state) as isize) };
        LOOP_WINDOW.with(|w| w.set(hwnd.0 as isize));
        Ok(LoopWindow { hwnd, state })
    }

    /// The sink every other thread posts through.
    pub fn sink(&self) -> Arc<WindowSink> {
        Arc::clone(&self.state.sink)
    }

    /// For `session::register`, and for tests.
    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }

    /// The pump: `GetMessageW`, `TranslateMessage`, `DispatchMessageW`, so the tray icon's and the menu's windows on
    /// this thread get their messages too. When a handler call returns `Flow::Quit`, the events still waiting are
    /// dropped and every later post fails; `run` then destroys the window and returns.
    ///
    /// The window procedure hands the handler: `WM_APP + 1` to `event`; its one timer to `timer`, re-armed after every
    /// call with `next_timer_ms` (at least 10 ms); `WM_WTSSESSION_CHANGE`, `WM_QUERYENDSESSION` and `WM_ENDSESSION`
    /// through `session::decode` to `session`, and `WM_QUERYENDSESSION` answers TRUE; `WM_CLOSE` to
    /// `event(Menu(Quit))`, never the default destroy, also when it came to the tray icon's or the menu's window
    /// (`pass_close_to_loop`). A `WM_QUIT` the window did not post goes the same way as `WM_CLOSE`, so the loop never
    /// ends without the handler's quit. Calls that came before `run` are made first.
    ///
    /// A handler that panics inside a window message aborts the process: Rust does not unwind out of a window
    /// procedure (the tray aborts on panic anyway). `Err` only when `GetMessageW` fails.
    pub fn run(self, handler: &dyn LoopHandler) -> Result<(), WinError> {
        let state = &*self.state;
        let slot: &dyn LoopHandler = handler;
        state.handler.set((&raw const slot).cast());
        // Declared after `slot`, so it drops first: the pointer is cleared before `slot` goes, also on unwind.
        let _clear = ClearHandler(&state.handler);
        state.arm(self.hwnd, handler);
        loop {
            let early = state.early.borrow_mut().pop_front();
            let Some(call) = early else { break };
            state.call(self.hwnd, call);
        }
        let mut msg = MSG::default();
        loop {
            // SAFETY: `msg` is a valid out pointer.
            match unsafe { GetMessageW(&mut msg, None, 0, 0) }.0 {
                -1 => return Err(last_error("GetMessageW")),
                0 if state.quitting.get() => return Ok(()),
                0 => state.call(self.hwnd, Call::Event(Event::Menu(MenuCommand::Quit))),
                // SAFETY: `msg` came from `GetMessageW` just above.
                _ => unsafe {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                },
            }
        }
    }
}

impl Drop for LoopWindow {
    fn drop(&mut self) {
        self.state.quitting.set(true);
        self.state.sink.close();
        LOOP_WINDOW.with(|w| {
            if w.get() == self.hwnd.0 as isize {
                w.set(0);
            }
        });
        let mut msg = MSG::default();
        // Their events are gone with `close`; take the messages out too, so none waits for a window that is gone.
        // SAFETY: `msg` is a valid out pointer; the filter names this thread's own window.
        while unsafe { PeekMessageW(&mut msg, Some(self.hwnd), WM_EVENT, WM_EVENT, PM_REMOVE) }
            .as_bool()
        {}
        // SAFETY: this thread's own window. The slot is cleared first, so the messages `DestroyWindow` sends never
        // see the state, which goes after this.
        unsafe {
            SetWindowLongPtrW(self.hwnd, GWLP_USERDATA, 0);
            // A window that will not go is reclaimed with its thread; nothing else is left to do.
            let _ = DestroyWindow(self.hwnd);
        }
    }
}

struct ClearHandler<'a>(&'a Cell<*const ()>);

impl Drop for ClearHandler<'_> {
    fn drop(&mut self) {
        self.0.set(std::ptr::null());
    }
}

extern "system" fn window_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // SAFETY: plain read of this window's own slot.
    let state = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) } as *const WindowState;
    // SAFETY: null, or the state of the `LoopWindow` that owns this window; its `Drop` clears the slot before the state
    // can go, and only this thread runs this procedure.
    let Some(state) = (unsafe { state.as_ref() }) else {
        // SAFETY: the default handling, with the message as it came.
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    };
    match msg {
        WM_EVENT => {
            if wparam.0 == EVENT_TAG
                && let Some(event) = state.sink.take(lparam.0 as usize)
            {
                state.call(hwnd, Call::Event(event));
            }
            LRESULT(0)
        }
        WM_TIMER if wparam.0 == TIMER_ID => {
            state.call(hwnd, Call::Timer);
            LRESULT(0)
        }
        // Never the default: it destroys the window, and a tray without its loop window restores nothing.
        WM_CLOSE => {
            state.call(hwnd, Call::Event(Event::Menu(MenuCommand::Quit)));
            LRESULT(0)
        }
        WM_WTSSESSION_CHANGE | WM_QUERYENDSESSION | WM_ENDSESSION => {
            if let Some(change) = session::decode(msg, wparam, lparam) {
                state.call(hwnd, Call::Session(change));
            }
            // Wakeshot never holds up a sign-out or a shutdown.
            LRESULT(isize::from(msg == WM_QUERYENDSESSION))
        }
        // SAFETY: the default handling, with the message as it came.
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

/// For the tray's other top-level windows (the icon's and the menu's): posts `WM_CLOSE` to this thread's loop window,
/// so a `WM_CLOSE` to any of them ends in the same Quit. `taskkill /PID` without `/F` posts one `WM_CLOSE`, to the
/// process's first top-level window in z-order, and that is seldom the loop window (research, "Plain taskkill and the
/// tray's windows"). `false` when this thread has no loop window (a demo, a test) or the post failed.
pub(crate) fn pass_close_to_loop() -> bool {
    let hwnd = LOOP_WINDOW.with(Cell::get);
    if hwnd == 0 {
        return false;
    }
    // SAFETY: plain call; Windows checks the handle.
    unsafe { PostMessageW(Some(HWND(hwnd as *mut _)), WM_CLOSE, WPARAM(0), LPARAM(0)) }.is_ok()
}

/// Finds this session's loop window by class and posts `WM_CLOSE`, which the tray handles as Quit from the menu.
/// `false` when no tray of that kind runs in this session.
pub fn ask_quit(dry_run: bool) -> Result<bool, WinError> {
    ask_quit_class(class(dry_run))
}

fn ask_quit_class(class: PCWSTR) -> Result<bool, WinError> {
    // SAFETY: plain calls. `FindWindowW` searches this desktop only, so it never finds another session's window.
    let found = unsafe {
        SetLastError(ERROR_SUCCESS);
        FindWindowW(class, PCWSTR::null())
    };
    let hwnd = match found {
        Ok(hwnd) => hwnd,
        Err(e) => {
            let error = win_error("FindWindowW", &e);
            return if error.code == ERROR_SUCCESS.0 {
                Ok(false)
            } else {
                Err(error)
            };
        }
    };
    // SAFETY: plain call; Windows checks the handle.
    match unsafe { PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0)) } {
        Ok(()) => Ok(true),
        // The window went between the find and the post.
        Err(e) if WIN32_ERROR::from_error(&e) == Some(ERROR_INVALID_WINDOW_HANDLE) => Ok(false),
        Err(e) => Err(win_error("PostMessageW", &e)),
    }
}

fn last_error(call: &'static str) -> WinError {
    // SAFETY: no arguments.
    let code = unsafe { GetLastError() }.0;
    WinError { call, code }
}

/// Win32 errors keep their Win32 code (1400, 1816), so a log line matches the docs; other failures keep the HRESULT.
fn win_error(call: &'static str, error: &windows::core::Error) -> WinError {
    WinError {
        call,
        code: WIN32_ERROR::from_error(error).map_or(error.code().0 as u32, |e| e.0),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;
    use std::thread::{self, JoinHandle};
    use std::time::{Duration, Instant};

    use windows::Win32::Foundation::ERROR_NOT_ENOUGH_QUOTA;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowThreadProcessId, IsWindow, PostThreadMessageW, SMTO_NORMAL, SendMessageTimeoutW,
        WM_NULL, WM_QUIT, WTS_SESSION_LOCK,
    };

    use super::*;

    /// Tests never use the real classes, so `ask_quit` from a tool never finds a test window, and a test never closes
    /// a real tray.
    fn test_class() -> PCWSTR {
        w!("Wakeshot.Loop.Test")
    }

    const WAIT: Duration = Duration::from_secs(10);

    /// Windows rounds a timer's due time to its clock tick (about 15.6 ms), so a fire may come a little early.
    const EARLY_MS: i64 = 50;

    /// `WM_TIMER` comes only when the loop thread runs and its queue is empty, so a busy laptop makes it late. With the
    /// whole crate's tests starting at once a fire came 209 ms late (2026-11-04), and a 50 ms bound failed 2 runs in 5.
    /// The arms in a test lie further apart than this, so even a fire this late tells one arm from another.
    const LATE_MS: i64 = 1_000;

    #[derive(Clone, Debug, PartialEq, Eq)]
    enum Seen {
        Event(Event),
        Session(SessionChange),
        Timer,
    }

    #[derive(Clone, Debug)]
    struct Record {
        call: Seen,
        /// 1 for a call from the pump, 2 for one from a pump inside a call.
        depth: u32,
        at: Instant,
        returned: Option<Instant>,
    }

    type React = Box<dyn Fn(&Recorder, &Seen) -> Flow>;
    type NextMs = Box<dyn Fn(&Recorder) -> u32>;

    /// Records every call. `react` must not panic: a panic inside a window message aborts the test binary.
    struct Recorder {
        hwnd: HWND,
        react: React,
        next_ms: NextMs,
        records: RefCell<Vec<Record>>,
        arms: RefCell<Vec<(Instant, u32)>>,
        depth: Cell<u32>,
    }

    impl Recorder {
        fn record(&self, call: Seen) -> Flow {
            let depth = self.depth.get() + 1;
            self.depth.set(depth);
            let index = {
                let mut records = self.records.borrow_mut();
                records.push(Record {
                    call: call.clone(),
                    depth,
                    at: Instant::now(),
                    returned: None,
                });
                records.len() - 1
            };
            let flow = (self.react)(self, &call);
            self.records.borrow_mut()[index].returned = Some(Instant::now());
            self.depth.set(depth - 1);
            flow
        }

        fn count(&self) -> usize {
            self.records.borrow().len()
        }

        fn timers(&self) -> usize {
            self.records
                .borrow()
                .iter()
                .filter(|r| r.call == Seen::Timer)
                .count()
        }
    }

    impl LoopHandler for Recorder {
        fn event(&self, event: Event) -> Flow {
            self.record(Seen::Event(event))
        }

        fn session(&self, change: SessionChange) -> Flow {
            self.record(Seen::Session(change))
        }

        fn timer(&self) -> Flow {
            self.record(Seen::Timer)
        }

        fn next_timer_ms(&self) -> u32 {
            let ms = (self.next_ms)(self);
            self.arms.borrow_mut().push((Instant::now(), ms));
            ms
        }
    }

    struct Finished {
        result: Result<(), WinError>,
        records: Vec<Record>,
        arms: Vec<(Instant, u32)>,
    }

    struct Running {
        hwnd: isize,
        sink: Arc<WindowSink>,
        thread: JoinHandle<Finished>,
    }

    impl Running {
        fn hwnd(&self) -> HWND {
            HWND(self.hwnd as *mut _)
        }

        fn finish(self) -> Finished {
            self.thread.join().expect("the loop thread ends")
        }
    }

    /// A loop window of `class` on its own thread: `before` runs there first, then `run` with a recorder.
    fn start_with(
        class: fn() -> PCWSTR,
        before: impl FnOnce(&LoopWindow) + Send + 'static,
        react: impl Fn(&Recorder, &Seen) -> Flow + Send + 'static,
        next_ms: impl Fn(&Recorder) -> u32 + Send + 'static,
    ) -> Running {
        let (made_tx, made_rx) = mpsc::channel();
        let thread = thread::spawn(move || {
            let window = LoopWindow::with_class(class()).expect("the loop window");
            made_tx
                .send((window.hwnd().0 as isize, window.sink()))
                .expect("the test waits");
            before(&window);
            let recorder = Recorder {
                hwnd: window.hwnd(),
                react: Box::new(react),
                next_ms: Box::new(next_ms),
                records: RefCell::default(),
                arms: RefCell::default(),
                depth: Cell::new(0),
            };
            let result = window.run(&recorder);
            Finished {
                result,
                records: recorder.records.take(),
                arms: recorder.arms.take(),
            }
        });
        let (hwnd, sink) = made_rx.recv_timeout(WAIT).expect("the window is made");
        Running { hwnd, sink, thread }
    }

    fn start(
        react: impl Fn(&Recorder, &Seen) -> Flow + Send + 'static,
        next_ms: impl Fn(&Recorder) -> u32 + Send + 'static,
    ) -> Running {
        start_with(test_class, |_| {}, react, next_ms)
    }

    /// Holds the loop thread before `run` until the test sends on the returned channel, so the test can fill the
    /// queue first.
    fn gate() -> (mpsc::Sender<()>, impl FnOnce(&LoopWindow) + Send + 'static) {
        let (go_tx, go_rx) = mpsc::channel::<()>();
        (go_tx, move |_: &LoopWindow| {
            go_rx.recv_timeout(WAIT).expect("the test opens the gate")
        })
    }

    fn quit_on_menu_quit(_: &Recorder, call: &Seen) -> Flow {
        if *call == Seen::Event(quit_event()) {
            Flow::Quit
        } else {
            Flow::Go
        }
    }

    fn far_timer(_: &Recorder) -> u32 {
        60_000
    }

    fn idle(ms: u64) -> Event {
        Event::InputIdle { ms }
    }

    fn quit_event() -> Event {
        Event::Menu(MenuCommand::Quit)
    }

    fn events(records: &[Record]) -> Vec<Event> {
        records
            .iter()
            .filter_map(|r| match &r.call {
                Seen::Event(event) => Some(event.clone()),
                _ => None,
            })
            .collect()
    }

    fn calls(records: &[Record]) -> Vec<Seen> {
        records.iter().map(|r| r.call.clone()).collect()
    }

    fn post_raw(hwnd: HWND, msg: u32, wparam: usize, lparam: isize) {
        // SAFETY: plain call; the message carries numbers only.
        unsafe { PostMessageW(Some(hwnd), msg, WPARAM(wparam), LPARAM(lparam)) }
            .expect("the post goes");
    }

    /// Sends from this thread and waits for the answer; the times are just before and just after the send.
    fn send(hwnd: HWND, msg: u32, wparam: usize, lparam: isize) -> (usize, Instant, Instant) {
        let mut answer = 0usize;
        let sent = Instant::now();
        // SAFETY: `answer` is a valid out pointer; the message carries numbers only.
        let ok = unsafe {
            SendMessageTimeoutW(
                hwnd,
                msg,
                WPARAM(wparam),
                LPARAM(lparam),
                SMTO_NORMAL,
                5_000,
                Some(&raw mut answer),
            )
        };
        assert_ne!(ok.0, 0, "the send of {msg:#x} failed or timed out");
        (answer, sent, Instant::now())
    }

    /// Dispatches one message waiting on this thread; false when none waits.
    fn pump_one() -> bool {
        let mut msg = MSG::default();
        // SAFETY: `msg` is a valid out pointer, and it is dispatched as it came.
        unsafe {
            if !PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                return false;
            }
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        true
    }

    fn is_window(hwnd: HWND) -> bool {
        // SAFETY: plain call; any handle value is fine.
        unsafe { IsWindow(Some(hwnd)) }.as_bool()
    }

    #[test]
    fn class_names() {
        // SAFETY: both point at static, nul-terminated strings.
        unsafe {
            assert_eq!(
                class(false).to_string().ok().as_deref(),
                Some("Wakeshot.Loop")
            );
            assert_eq!(
                class(true).to_string().ok().as_deref(),
                Some("Wakeshot.Loop.DryRun")
            );
        }
    }

    #[test]
    fn events_from_four_threads_all_arrive_in_each_posters_order() {
        let running = start(quit_on_menu_quit, far_timer);
        let posters: Vec<_> = (0..4u64)
            .map(|poster| {
                let sink = Arc::clone(&running.sink);
                thread::spawn(move || {
                    for seq in 0..250 {
                        sink.post(idle(poster * 1_000_000 + seq))
                            .expect("the post goes");
                    }
                })
            })
            .collect();
        for poster in posters {
            poster.join().expect("the poster ends");
        }
        running.sink.post(quit_event()).expect("the post goes");
        let sink = Arc::clone(&running.sink);
        let done = running.finish();

        assert_eq!(done.result, Ok(()));
        let got = events(&done.records);
        assert_eq!(got.len(), 1_001);
        assert_eq!(got.last(), Some(&quit_event()));
        let mut next = [0u64; 4];
        for event in &got[..1_000] {
            let Event::InputIdle { ms } = *event else {
                panic!("not a posted event: {event:?}");
            };
            let (poster, seq) = ((ms / 1_000_000) as usize, ms % 1_000_000);
            assert_eq!(seq, next[poster], "poster {poster} out of order");
            next[poster] += 1;
        }
        assert_eq!(next, [250; 4]);
        assert!(done.records.iter().all(|r| r.depth == 1));
        assert_eq!(sink.waiting(), 0);
    }

    #[test]
    fn a_post_after_run_returned_fails_and_leaves_nothing() {
        let running = start(quit_on_menu_quit, far_timer);
        running.sink.post(quit_event()).expect("the post goes");
        let sink = Arc::clone(&running.sink);
        let hwnd = running.hwnd();
        let done = running.finish();

        assert_eq!(done.result, Ok(()));
        assert!(!is_window(hwnd), "run destroys the window");
        let closed = WinError {
            call: "PostMessageW",
            code: ERROR_INVALID_WINDOW_HANDLE.0,
        };
        assert_eq!(sink.post(idle(1)), Err(closed));
        assert_eq!(sink.post(idle(2)), Err(closed));
        assert_eq!(sink.waiting(), 0);
    }

    #[test]
    fn a_failed_post_takes_its_event_back() {
        // SAFETY: a plain window of the system STATIC class, never shown, destroyed at once on this thread.
        let hwnd = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("STATIC"),
                w!("Wakeshot window test"),
                WS_OVERLAPPED,
                0,
                0,
                0,
                0,
                None,
                None,
                None,
                None,
            )
        }
        .expect("a window");
        // SAFETY: the window made above, on this thread.
        unsafe { DestroyWindow(hwnd) }.expect("the window goes");
        // A sink still open on a window that is gone: only `PostMessageW` itself can fail the post.
        let sink = WindowSink::new(hwnd);

        let got = sink.post(idle(1));

        assert_eq!(
            got,
            Err(WinError {
                call: "PostMessageW",
                code: ERROR_INVALID_WINDOW_HANDLE.0,
            })
        );
        assert_eq!(sink.waiting(), 0);
    }

    #[test]
    fn quit_drops_every_event_still_queued() {
        let (go, before) = gate();
        let running = start_with(test_class, before, |_, _| Flow::Quit, far_timer);
        for ms in 0..100 {
            running.sink.post(idle(ms)).expect("the post goes");
        }
        assert_eq!(running.sink.waiting(), 100);
        go.send(()).expect("the loop thread waits");
        let sink = Arc::clone(&running.sink);
        let done = running.finish();

        assert_eq!(done.result, Ok(()));
        assert_eq!(events(&done.records), [idle(0)]);
        assert_eq!(sink.waiting(), 0);
    }

    #[test]
    fn a_full_queue_fails_the_post_and_quit_frees_the_rest() {
        let (go, before) = gate();
        let running = start_with(test_class, before, |_, _| Flow::Quit, far_timer);
        let mut posted = 0usize;
        let failure = loop {
            match running.sink.post(idle(posted as u64)) {
                Ok(()) => posted += 1,
                Err(e) => break e,
            }
            assert!(
                posted < 1_000_000,
                "a full queue fails a post well before this"
            );
        };
        eprintln!("posts before the queue was full: {posted}");
        assert_eq!(
            failure,
            WinError {
                call: "PostMessageW",
                code: ERROR_NOT_ENOUGH_QUOTA.0,
            }
        );
        assert!(posted >= 1_000, "only {posted} posts went");
        assert_eq!(running.sink.waiting(), posted);
        go.send(()).expect("the loop thread waits");
        let sink = Arc::clone(&running.sink);
        let done = running.finish();

        assert_eq!(done.result, Ok(()));
        assert_eq!(events(&done.records), [idle(0)]);
        assert_eq!(sink.waiting(), 0);
    }

    #[test]
    fn a_stray_event_message_is_ignored_and_takes_no_event() {
        let (go, before) = gate();
        let running = start_with(test_class, before, quit_on_menu_quit, far_timer);
        let hwnd = running.hwnd();
        // Queued ahead of the real messages: without the tag check it would take the second event first.
        post_raw(hwnd, WM_EVENT, 0, 2);
        running.sink.post(idle(1)).expect("the post goes");
        running.sink.post(idle(2)).expect("the post goes");
        post_raw(hwnd, WM_EVENT, EVENT_TAG, 999);
        running.sink.post(quit_event()).expect("the post goes");
        go.send(()).expect("the loop thread waits");
        let done = running.finish();

        assert_eq!(events(&done.records), [idle(1), idle(2), quit_event()]);
    }

    #[test]
    fn the_timer_fires_after_next_timer_ms_and_again_after_each_re_arm() {
        // The middle arm is more than `LATE_MS` past the others, so no single fixed delay, and no arm that kept the
        // value before, passes all three bounds.
        const PLAN: [u32; 3] = [150, 1_500, 80];
        let running = start(
            |rec, call| {
                if *call == Seen::Timer && rec.timers() == PLAN.len() {
                    Flow::Quit
                } else {
                    Flow::Go
                }
            },
            |rec| PLAN[rec.timers().min(PLAN.len() - 1)],
        );
        let done = running.finish();

        assert_eq!(done.result, Ok(()));
        assert_eq!(
            calls(&done.records),
            [Seen::Timer, Seen::Timer, Seen::Timer]
        );
        let armed: Vec<u32> = done.arms.iter().map(|(_, ms)| *ms).collect();
        assert_eq!(armed, PLAN);
        for (fire, (at, ms)) in done.records.iter().zip(&done.arms) {
            let took = fire.at.duration_since(*at).as_millis() as i64;
            eprintln!("a {ms} ms timer fired after {took} ms");
            let ms = i64::from(*ms);
            assert!(
                (ms - EARLY_MS..=ms + LATE_MS).contains(&took),
                "a {ms} ms timer fired after {took} ms"
            );
        }
    }

    #[test]
    fn an_event_re_arms_the_timer() {
        // Far past `AGAIN + LATE_MS`, so the event always comes before the first arm is due, however late the loop
        // thread runs, and the one fire can only be the re-arm's. A broken re-arm fails after 5 s instead of hanging.
        const FIRST: u32 = 5_000;
        const AGAIN: u32 = 200;
        let (armed_tx, armed_rx) = mpsc::channel();
        let running = start(
            |_, call| {
                if *call == Seen::Timer {
                    Flow::Quit
                } else {
                    Flow::Go
                }
            },
            move |rec| {
                if rec.count() == 0 {
                    let _ = armed_tx.send(());
                    FIRST
                } else {
                    AGAIN
                }
            },
        );
        armed_rx.recv_timeout(WAIT).expect("run arms the timer");
        running.sink.post(idle(1)).expect("the post goes");
        let done = running.finish();

        assert_eq!(calls(&done.records), [Seen::Event(idle(1)), Seen::Timer]);
        let [(first, FIRST), (second, AGAIN)] = done.arms[..] else {
            panic!("arms: {:?}", done.arms);
        };
        let fire = done.records[1].at;
        let since_event = fire.duration_since(second).as_millis() as i64;
        let since_start = fire.duration_since(first).as_millis() as i64;
        eprintln!(
            "re-armed {AGAIN} ms timer fired after {since_event} ms, {since_start} ms after the first arm"
        );
        let again = i64::from(AGAIN);
        assert!(
            (again - EARLY_MS..=again + LATE_MS).contains(&since_event),
            "fired {since_event} ms after the re-arm"
        );
        assert!(
            since_start < i64::from(FIRST) - EARLY_MS,
            "the first arm's {FIRST} ms still ran: {since_start} ms"
        );
    }

    #[test]
    fn session_messages_reach_the_handler_and_the_sender_waits() {
        let running = start(
            |rec, call| {
                if *call == Seen::Session(SessionChange::Ended) {
                    thread::sleep(Duration::from_millis(300));
                }
                quit_on_menu_quit(rec, call)
            },
            far_timer,
        );
        let hwnd = running.hwnd();

        let (query, _, _) = send(hwnd, WM_QUERYENDSESSION, 0, 0);
        let (_, sent, back) = send(hwnd, WM_ENDSESSION, 1, 0);
        let _ = send(hwnd, WM_ENDSESSION, 0, 0);
        let _ = send(hwnd, WM_WTSSESSION_CHANGE, WTS_SESSION_LOCK as usize, 1);
        post_raw(hwnd, WM_CLOSE, 0, 0);
        let done = running.finish();

        assert_eq!(query, 1, "WM_QUERYENDSESSION answers TRUE");
        assert_eq!(
            calls(&done.records),
            [
                Seen::Session(SessionChange::Ending),
                Seen::Session(SessionChange::Ended),
                Seen::Session(SessionChange::EndCancelled),
                Seen::Session(SessionChange::Locked),
                Seen::Event(quit_event()),
            ]
        );
        let ended = done.records[1].returned.expect("the call returned");
        assert!(
            back >= ended,
            "the sender came back before the handler returned"
        );
        let waited = back.duration_since(sent);
        eprintln!("the WM_ENDSESSION sender waited {} ms", waited.as_millis());
        assert!(
            waited >= Duration::from_millis(300),
            "the sender waited only {waited:?}"
        );
    }

    #[test]
    fn close_reaches_the_handler_as_quit_and_the_window_stays_until_quit() {
        let (alive_tx, alive_rx) = mpsc::channel();
        let running = start(
            move |rec, _| {
                let _ = alive_tx.send(is_window(rec.hwnd));
                // The first close goes on, the second quits.
                if rec.count() >= 2 {
                    Flow::Quit
                } else {
                    Flow::Go
                }
            },
            far_timer,
        );
        let hwnd = running.hwnd();

        post_raw(hwnd, WM_CLOSE, 0, 0);
        assert_eq!(alive_rx.recv_timeout(WAIT), Ok(true));
        // A sent message is handled only after the close's call returned, so the window outlived a call that went on.
        let _ = send(hwnd, WM_NULL, 0, 0);
        assert!(is_window(hwnd));
        post_raw(hwnd, WM_CLOSE, 0, 0);
        let done = running.finish();

        assert_eq!(alive_rx.recv_timeout(WAIT), Ok(true));
        assert_eq!(done.result, Ok(()));
        assert_eq!(events(&done.records), [quit_event(), quit_event()]);
        assert!(!is_window(hwnd));
    }

    /// A plain `taskkill` posts its `WM_CLOSE` to the icon's or the menu's window, whichever is first in z-order.
    #[test]
    fn close_on_the_icon_or_the_menu_window_reaches_the_handler_as_quit() {
        use crate::sink::FakeEventSink;
        use crate::tray::icon::{IconKind, TrayIcon};
        use crate::tray::menu::OwnerDrawnMenu;
        use windows::Win32::UI::WindowsAndMessaging::{SC_CLOSE, WM_SYSCOMMAND};

        let running = start_with(
            test_class,
            |window| {
                let menu = OwnerDrawnMenu::new(Arc::new(FakeEventSink::default()))
                    .expect("the menu window");
                let menu_hwnd = menu.hwnd();
                let icon = TrayIcon::new(IconKind::Demo, Box::new(menu)).expect("the icon window");
                let icon_hwnd = HWND(icon.identity().hwnd as *mut _);
                // An Alt+F4 on the menu's window, then the two closes, posted as taskkill posts them.
                let _ = send(menu_hwnd, WM_SYSCOMMAND, SC_CLOSE as usize, 0);
                post_raw(menu_hwnd, WM_CLOSE, 0, 0);
                post_raw(icon_hwnd, WM_CLOSE, 0, 0);
                // Each window passes its close on while it lives; the loop window's closes wait for `run`.
                while pump_one() {}
                assert!(is_window(menu_hwnd) && is_window(icon_hwnd));
                drop(icon);
                window.sink().post(idle(7)).expect("the post goes");
            },
            |_, call| {
                if *call == Seen::Event(idle(7)) {
                    Flow::Quit
                } else {
                    Flow::Go
                }
            },
            far_timer,
        );
        let done = running.finish();

        assert_eq!(done.result, Ok(()));
        assert_eq!(events(&done.records), [quit_event(), quit_event(), idle(7)]);
    }

    #[test]
    fn no_close_is_passed_on_without_a_loop_window() {
        let passed = thread::spawn(|| {
            let before = pass_close_to_loop();
            let window = LoopWindow::with_class(test_class()).expect("the loop window");
            drop(window);
            (before, pass_close_to_loop())
        })
        .join()
        .expect("the thread ends");
        assert_eq!(passed, (false, false));
    }

    #[test]
    fn a_quit_message_from_elsewhere_reaches_the_handler_as_quit() {
        let running = start(quit_on_menu_quit, far_timer);
        // SAFETY: plain calls; the window's thread has a queue, as it made the window.
        unsafe {
            let thread = GetWindowThreadProcessId(running.hwnd(), None);
            PostThreadMessageW(thread, WM_QUIT, WPARAM(0), LPARAM(0)).expect("the post goes");
        }
        let done = running.finish();

        assert_eq!(done.result, Ok(()));
        assert_eq!(events(&done.records), [quit_event()]);
    }

    #[test]
    fn ask_quit_finds_the_window_by_class_and_false_when_none_runs() {
        fn class() -> PCWSTR {
            w!("Wakeshot.Loop.Test.AskQuit")
        }
        assert_eq!(
            ask_quit_class(class()),
            Ok(false),
            "before any window of the class"
        );
        let running = start_with(class, |_| {}, quit_on_menu_quit, far_timer);
        let hwnd = running.hwnd();

        assert_eq!(ask_quit_class(class()), Ok(true));
        let done = running.finish();

        assert_eq!(events(&done.records), [quit_event()]);
        assert!(!is_window(hwnd));
        assert_eq!(ask_quit_class(class()), Ok(false), "after the window went");
        assert_eq!(ask_quit_class(w!("Wakeshot.Loop.Test.Nobody")), Ok(false));
    }

    #[test]
    fn a_nested_pump_gets_the_nested_events_in_order() {
        let (go, before) = gate();
        let running = start_with(
            test_class,
            before,
            |rec, call| {
                if *call != Seen::Event(idle(0)) {
                    return quit_on_menu_quit(rec, call);
                }
                // A runner that pumps, as the modal menu does, until the nine events behind this one came through.
                let deadline = Instant::now() + WAIT;
                while rec.count() < 10 && Instant::now() < deadline {
                    if !pump_one() {
                        thread::sleep(Duration::from_millis(1));
                    }
                }
                Flow::Go
            },
            far_timer,
        );
        for ms in 0..10 {
            running.sink.post(idle(ms)).expect("the post goes");
        }
        running.sink.post(quit_event()).expect("the post goes");
        go.send(()).expect("the loop thread waits");
        let done = running.finish();

        let got: Vec<(Seen, u32)> = done
            .records
            .iter()
            .map(|r| (r.call.clone(), r.depth))
            .collect();
        let mut want = vec![(Seen::Event(idle(0)), 1)];
        want.extend((1..10).map(|ms| (Seen::Event(idle(ms)), 2)));
        want.push((Seen::Event(quit_event()), 1));
        assert_eq!(got, want);
    }

    #[test]
    fn calls_that_come_before_run_are_made_first_in_order() {
        let (posted_tx, posted_rx) = mpsc::channel::<()>();
        let (pumped_tx, pumped_rx) = mpsc::channel::<()>();
        let running = start_with(
            test_class,
            move |_| {
                posted_rx.recv_timeout(WAIT).expect("the test posts");
                // Startup code that pumps messages before `run`.
                while pump_one() {}
                pumped_tx.send(()).expect("the test waits");
            },
            |_, call| {
                if *call == Seen::Event(idle(3)) {
                    Flow::Quit
                } else {
                    Flow::Go
                }
            },
            far_timer,
        );
        let hwnd = running.hwnd();
        running.sink.post(idle(0)).expect("the post goes");
        running.sink.post(idle(1)).expect("the post goes");
        post_raw(hwnd, WM_WTSSESSION_CHANGE, WTS_SESSION_LOCK as usize, 1);
        running.sink.post(idle(2)).expect("the post goes");
        posted_tx.send(()).expect("the loop thread waits");
        pumped_rx
            .recv_timeout(WAIT)
            .expect("the loop thread pumped");
        assert_eq!(running.sink.waiting(), 0, "the window took the events");
        running.sink.post(idle(3)).expect("the post goes");
        let done = running.finish();

        assert_eq!(done.result, Ok(()));
        assert_eq!(
            calls(&done.records),
            [
                Seen::Event(idle(0)),
                Seen::Event(idle(1)),
                Seen::Session(SessionChange::Locked),
                Seen::Event(idle(2)),
                Seen::Event(idle(3)),
            ]
        );
    }

    #[test]
    fn a_window_dropped_without_run_drops_its_events_and_goes() {
        let window = LoopWindow::with_class(test_class()).expect("the loop window");
        let sink = window.sink();
        let hwnd = window.hwnd();
        sink.post(idle(1)).expect("the post goes");
        sink.post(idle(2)).expect("the post goes");
        assert_eq!(sink.waiting(), 2);

        drop(window);

        assert_eq!(sink.waiting(), 0);
        assert!(!is_window(hwnd));
        assert!(sink.post(idle(3)).is_err());
    }
}
