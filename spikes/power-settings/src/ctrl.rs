use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use windows::Win32::System::Console::SetConsoleCtrlHandler;
use windows::core::BOOL;

static STOP: AtomicBool = AtomicBool::new(false);
static CLEAN: AtomicBool = AtomicBool::new(false);

unsafe extern "system" fn on_ctrl(kind: u32) -> BOOL {
    crate::log!(
        "ctrl",
        "console control event {kind}: stopping and restoring"
    );
    STOP.store(true, Ordering::SeqCst);
    // Close ends the process when this returns, so give the main thread a few seconds to put the
    // plan back first. Logoff and shutdown never arrive here: this exe loads user32, and Windows
    // sends those two only to console apps that do not. A restart or sign-out mid-run therefore
    // leaves the copy active until `restore`.
    if kind >= 2 {
        let t = Instant::now();
        while !CLEAN.load(Ordering::SeqCst) && t.elapsed() < Duration::from_millis(4500) {
            std::thread::sleep(Duration::from_millis(50));
        }
    }
    BOOL(1)
}

pub fn install() {
    let r = unsafe { SetConsoleCtrlHandler(Some(on_ctrl), true) };
    crate::log!("setup", "SetConsoleCtrlHandler {r:?}");
}

pub fn stop_requested() -> bool {
    STOP.load(Ordering::SeqCst)
}

/// Tells a pending close event that the plan is back, so the process may end now.
pub fn mark_clean() {
    CLEAN.store(true, Ordering::SeqCst);
}

/// Sleeps in short steps; false when a stop was requested.
pub fn nap(ms: u64) -> bool {
    let end = Instant::now() + Duration::from_millis(ms);
    while Instant::now() < end {
        if stop_requested() {
            return false;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    !stop_requested()
}
