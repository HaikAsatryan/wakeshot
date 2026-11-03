//! Session notifications, the console check, `lock()`; real and dry run.
//!
//! Owner: `p3-session`.
//!
//! The loop window's owner (integration) calls `register` once its hidden top-level window exists and `decode` for
//! each message it gets; this module has no window procedure (architecture section 4, "Tray threads").

#[cfg(any(test, feature = "test-support"))]
use std::cell::Cell;
use std::sync::Arc;

use wakeshot_core::event::SessionChange;
use windows::Win32::Foundation::{HWND, LPARAM, WIN32_ERROR, WPARAM};
use windows::Win32::System::RemoteDesktop::{
    NOTIFY_FOR_THIS_SESSION, ProcessIdToSessionId, WTS_CURRENT_SESSION, WTS_SESSIONSTATE_LOCK,
    WTS_SESSIONSTATE_UNLOCK, WTSFreeMemory, WTSGetActiveConsoleSessionId, WTSINFOEXW,
    WTSQuerySessionInformationW, WTSRegisterSessionNotification, WTSSessionInfoEx,
    WTSUnRegisterSessionNotification,
};
use windows::Win32::System::Shutdown::LockWorkStation;
use windows::Win32::System::Threading::GetCurrentProcessId;
use windows::Win32::UI::WindowsAndMessaging::{
    WM_ENDSESSION, WM_QUERYENDSESSION, WM_WTSSESSION_CHANGE, WTS_CONSOLE_CONNECT,
    WTS_CONSOLE_DISCONNECT, WTS_SESSION_LOCK, WTS_SESSION_UNLOCK,
};
use windows::core::PWSTR;

use crate::WinError;
use crate::log::{Level, Log};

const AREA: &str = "session";

/// `WM_WTSSESSION_CHANGE` for this session, while it lives. Drop it before the window is destroyed: the docs ask for
/// the unregister first.
///
/// Owner: power track (Phase 3).
#[derive(Debug)]
pub struct SessionNotifications {
    hwnd: HWND,
    registered: bool,
}

/// `WTSRegisterSessionNotification(hwnd, NOTIFY_FOR_THIS_SESSION)`. `hwnd` is the loop's hidden top-level window.
///
/// The docs say a call made before the Remote Desktop services have started may fail with `RPC_S_INVALID_BINDING`
/// (1702); the caller logs any error, and the tray then runs without session changes.
pub fn register(hwnd: HWND) -> Result<SessionNotifications, WinError> {
    // SAFETY: plain call on a window handle; Windows checks it.
    unsafe { WTSRegisterSessionNotification(hwnd, NOTIFY_FOR_THIS_SESSION) }
        .map_err(|e| win_error("WTSRegisterSessionNotification", &e))?;
    Ok(SessionNotifications {
        hwnd,
        registered: true,
    })
}

impl SessionNotifications {
    /// Unregisters now and returns the result; a drop does the same and drops the error.
    pub fn unregister(mut self) -> Result<(), WinError> {
        self.registered = false;
        unregister(self.hwnd)
    }
}

impl Drop for SessionNotifications {
    fn drop(&mut self) {
        if self.registered {
            // The window goes next; nothing is left to act on a failure.
            let _ = unregister(self.hwnd);
        }
    }
}

fn unregister(hwnd: HWND) -> Result<(), WinError> {
    // SAFETY: plain call on a window handle; Windows checks it.
    unsafe { WTSUnRegisterSessionNotification(hwnd) }
        .map_err(|e| win_error("WTSUnRegisterSessionNotification", &e))
}

/// The session change a window message carries, or `None` for any other message or code.
///
/// `lparam` is never read: with `NOTIFY_FOR_THIS_SESSION` it always names this session, and the end-session flags
/// (log off, Restart Manager close, forced) all mean the tray is about to end, so they map the same.
pub fn decode(msg: u32, wparam: WPARAM, _lparam: LPARAM) -> Option<SessionChange> {
    match msg {
        WM_WTSSESSION_CHANGE => match u32::try_from(wparam.0).ok()? {
            WTS_SESSION_LOCK => Some(SessionChange::Locked),
            WTS_SESSION_UNLOCK => Some(SessionChange::Unlocked),
            WTS_CONSOLE_DISCONNECT => Some(SessionChange::ConsoleLeft),
            WTS_CONSOLE_CONNECT => Some(SessionChange::ConsoleBack),
            _ => None,
        },
        WM_QUERYENDSESSION => Some(SessionChange::Ending),
        // Any nonzero wparam is TRUE.
        WM_ENDSESSION if wparam.0 != 0 => Some(SessionChange::Ended),
        WM_ENDSESSION => Some(SessionChange::EndCancelled),
        _ => None,
    }
}

/// This session is the one on the PC's own screen and keyboard. `false` while no session is attached to the console
/// (`WTSGetActiveConsoleSessionId` gives 0xFFFFFFFF during an attach or detach, Microsoft docs), and in a Remote
/// Desktop session.
///
/// An `Err` should count as `false`, the safe side: the core then holds requests only and leaves the user's lid and
/// sleep settings alone.
pub fn on_console() -> Result<bool, WinError> {
    let mut own = 0;
    // SAFETY: `own` is a valid out pointer for the call.
    unsafe { ProcessIdToSessionId(GetCurrentProcessId(), &mut own) }
        .map_err(|e| win_error("ProcessIdToSessionId", &e))?;
    // SAFETY: no arguments; the call cannot fail.
    let console = unsafe { WTSGetActiveConsoleSessionId() };
    Ok(console == own)
}

/// This session's lock state now (`WTSSessionInfoEx`, `SessionFlags`; a port of the lid spike's `session_locked`).
/// `None` when the query fails or the state is unknown. Integration feeds `Session(Locked)` right after `Started` on
/// `Some(true)`.
pub fn locked_now() -> Option<bool> {
    let mut buffer = PWSTR::null();
    let mut bytes = 0u32;
    // SAFETY: both out pointers are valid locals; WTS allocates the buffer, freed below.
    unsafe {
        WTSQuerySessionInformationW(
            None,
            WTS_CURRENT_SESSION,
            WTSSessionInfoEx,
            &mut buffer,
            &mut bytes,
        )
    }
    .ok()?;
    if buffer.is_null() {
        return None;
    }
    let flags = if bytes as usize >= size_of::<WTSINFOEXW>() {
        // SAFETY: the buffer holds `bytes` bytes, enough for the struct; an unaligned read needs no alignment.
        let info = unsafe { buffer.0.cast::<WTSINFOEXW>().read_unaligned() };
        // SAFETY: level 1 is the union's only member, and every bit pattern is a valid `i32`.
        let flags = unsafe { info.Data.WTSInfoExLevel1.SessionFlags };
        (info.Level == 1).then_some(flags)
    } else {
        None
    };
    // SAFETY: the buffer came from `WTSQuerySessionInformationW` and is freed once.
    unsafe { WTSFreeMemory(buffer.0.cast()) };
    lock_state(flags?)
}

/// `SessionFlags` as a lock state. The docs' lock and unlock values are swapped only on Windows 7 and Server 2008 R2,
/// which Wakeshot does not run on.
fn lock_state(flags: i32) -> Option<bool> {
    match flags as u32 {
        WTS_SESSIONSTATE_LOCK => Some(true),
        WTS_SESSIONSTATE_UNLOCK => Some(false),
        _ => None,
    }
}

/// Locks this session, for `Command::Lock`; the loop thread runs it and answers `Outcome::Locked(result.map_err(
/// Failure::from))`.
///
/// Owner: power track (Phase 3).
pub trait Locker {
    fn lock(&self) -> Result<(), WinError>;
}

/// `LockWorkStation`. `Ok` means the lock has started, not that the session is locked: the call is asynchronous
/// (Microsoft docs; tested: it returned in 0.5 ms and the session read locked 1 s later, research "Lock").
#[derive(Clone, Copy, Debug, Default)]
pub struct Win32Locker;

impl Locker for Win32Locker {
    fn lock(&self) -> Result<(), WinError> {
        // SAFETY: no arguments.
        unsafe { LockWorkStation() }.map_err(|e| win_error("LockWorkStation", &e))
    }
}

/// Logs the lock it would make and never locks (track note 8).
pub struct DryRunLocker {
    log: Arc<dyn Log>,
}

impl DryRunLocker {
    pub fn new(log: Arc<dyn Log>) -> Self {
        Self { log }
    }
}

impl std::fmt::Debug for DryRunLocker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DryRunLocker").finish_non_exhaustive()
    }
}

impl Locker for DryRunLocker {
    fn lock(&self) -> Result<(), WinError> {
        self.log.line(Level::Info, AREA, "dry run: LockWorkStation");
        Ok(())
    }
}

/// Counts every `lock` call, failed ones too; `fail_with` makes every call fail.
#[cfg(any(test, feature = "test-support"))]
#[derive(Debug, Default)]
pub struct FakeLocker {
    pub locks: Cell<u32>,
    pub fail_with: Option<WinError>,
}

#[cfg(any(test, feature = "test-support"))]
impl Locker for FakeLocker {
    fn lock(&self) -> Result<(), WinError> {
        self.locks.set(self.locks.get() + 1);
        match self.fail_with {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }
}

/// Win32 errors keep their Win32 code (5, 1702), so a log line matches the docs; other failures keep the HRESULT.
fn win_error(call: &'static str, error: &windows::core::Error) -> WinError {
    WinError {
        call,
        code: WIN32_ERROR::from_error(error).map_or(error.code().0 as u32, |e| e.0),
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DestroyWindow, WINDOW_EX_STYLE, WM_APP, WM_POWERBROADCAST, WM_TIMER,
        WS_OVERLAPPED, WTS_REMOTE_CONNECT, WTS_REMOTE_DISCONNECT, WTS_SESSION_LOGOFF,
        WTS_SESSION_LOGON,
    };
    use windows::core::w;

    use super::*;
    use crate::log::MemoryLog;

    const ENDSESSION_LOGOFF: isize = 0x8000_0000;
    const ENDSESSION_CLOSEAPP: isize = 0x1;

    fn session(code: u32) -> Option<SessionChange> {
        decode(WM_WTSSESSION_CHANGE, WPARAM(code as usize), LPARAM(1))
    }

    #[test]
    fn decode_session_messages() {
        assert_eq!(session(WTS_SESSION_LOCK), Some(SessionChange::Locked));
        assert_eq!(session(WTS_SESSION_UNLOCK), Some(SessionChange::Unlocked));
        assert_eq!(
            session(WTS_CONSOLE_DISCONNECT),
            Some(SessionChange::ConsoleLeft)
        );
        assert_eq!(
            session(WTS_CONSOLE_CONNECT),
            Some(SessionChange::ConsoleBack)
        );
        assert_eq!(
            decode(WM_QUERYENDSESSION, WPARAM(0), LPARAM(ENDSESSION_LOGOFF)),
            Some(SessionChange::Ending)
        );
        assert_eq!(
            decode(WM_QUERYENDSESSION, WPARAM(0), LPARAM(0)),
            Some(SessionChange::Ending),
            "shutdown or restart"
        );
        assert_eq!(
            decode(WM_ENDSESSION, WPARAM(1), LPARAM(ENDSESSION_LOGOFF)),
            Some(SessionChange::Ended)
        );
        assert_eq!(
            decode(WM_ENDSESSION, WPARAM(2), LPARAM(ENDSESSION_CLOSEAPP)),
            Some(SessionChange::Ended),
            "any nonzero wparam is TRUE"
        );
        assert_eq!(
            decode(WM_ENDSESSION, WPARAM(0), LPARAM(ENDSESSION_LOGOFF)),
            Some(SessionChange::EndCancelled)
        );

        // Codes the core has no event for.
        for code in [
            0,
            WTS_REMOTE_CONNECT,
            WTS_REMOTE_DISCONNECT,
            WTS_SESSION_LOGON,
            WTS_SESSION_LOGOFF,
            9,
            0xF,
            99,
        ] {
            assert_eq!(session(code), None, "code {code}");
        }
        assert_eq!(
            decode(
                WM_WTSSESSION_CHANGE,
                WPARAM(WTS_SESSION_LOCK as usize | 1 << 32),
                LPARAM(1)
            ),
            None,
            "a wparam past u32 is no code"
        );

        // Other messages.
        for msg in [WM_TIMER, WM_POWERBROADCAST, WM_APP, 0] {
            assert_eq!(
                decode(msg, WPARAM(WTS_SESSION_LOCK as usize), LPARAM(1)),
                None
            );
        }
    }

    #[test]
    fn session_flags_read_as_lock_state() {
        assert_eq!(lock_state(0), Some(true));
        assert_eq!(lock_state(1), Some(false));
        assert_eq!(lock_state(-1), None, "WTS_SESSIONSTATE_UNKNOWN");
        assert_eq!(lock_state(2), None);
    }

    /// Real, read only: a hidden top-level window made here; register and unregister give no error, and a drop
    /// unregisters too, so a second registration of the same window works.
    #[test]
    fn register_on_hidden_window() {
        // SAFETY: a plain window of the system STATIC class, never shown, destroyed below on this thread.
        let hwnd = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("STATIC"),
                w!("Wakeshot session test"),
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
        .expect("a hidden window");

        let first = register(hwnd);
        assert!(first.is_ok(), "{first:?}");
        drop(first);
        let second = register(hwnd).expect("registers again after the drop");
        assert_eq!(second.unregister(), Ok(()));

        // SAFETY: the window made above, on this thread.
        unsafe { DestroyWindow(hwnd) }.expect("the window goes");
    }

    #[test]
    fn fake_and_dry_run_lock_never_lock() {
        let log = Arc::new(MemoryLog::default());
        let dry: &dyn Locker = &DryRunLocker::new(log.clone());
        assert_eq!(dry.lock(), Ok(()));
        assert_eq!(
            *log.lines.lock().unwrap(),
            [(Level::Info, AREA, "dry run: LockWorkStation".to_owned())]
        );

        let error = WinError {
            call: "LockWorkStation",
            code: 5,
        };
        let mut fake = FakeLocker::default();
        assert_eq!(fake.lock(), Ok(()));
        fake.fail_with = Some(error);
        assert_eq!(fake.lock(), Err(error));
        assert_eq!(fake.locks.get(), 2);
    }

    /// Real: true when run from Haik's desktop. Ignored, since a CI runner's console session is not verified (review
    /// M9).
    #[test]
    #[ignore = "real: needs this session on the PC's own screen"]
    fn real_on_console_here() {
        assert_eq!(on_console(), Ok(true));
    }

    /// Real: `Some(false)` when run from an unlocked desktop.
    #[test]
    #[ignore = "real: needs an unlocked desktop session"]
    fn real_locked_now_here() {
        assert_eq!(locked_now(), Some(false));
    }

    /// Real: locks Haik's screen. Only `p3-haik-checks` runs it, with Haik there to sign in again.
    #[test]
    #[ignore = "real: locks the screen; p3-haik-checks only"]
    fn real_lock_locks() {
        assert_eq!(Win32Locker.lock(), Ok(()));
        let start = Instant::now();
        while locked_now() != Some(true) {
            assert!(
                start.elapsed() < Duration::from_secs(5),
                "not locked 5 s after LockWorkStation"
            );
            std::thread::sleep(Duration::from_millis(100));
        }
    }
}
