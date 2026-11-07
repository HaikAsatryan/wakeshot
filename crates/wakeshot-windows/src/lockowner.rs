//! Which processes hold a file open, through Restart Manager: ties a Codex or Copilot lock file to the process that
//! holds it (architecture section 7). Asking does not disturb the holder: with a file held the way Codex holds its
//! lock files, the lock and the holder's writes stay (tested below), so Codex never sees a second writer.

use std::os::windows::ffi::OsStrExt;
use std::path::Path;

use wakeshot_core::process::{FileTime, ProcessRef};
use windows::Win32::Foundation::{ERROR_MORE_DATA, ERROR_SUCCESS, WIN32_ERROR};
use windows::Win32::System::RestartManager::{
    CCH_RM_SESSION_KEY, RM_PROCESS_INFO, RmEndSession, RmGetList, RmRegisterResources,
    RmStartSession,
};
use windows::core::{PCWSTR, PWSTR};

use crate::WinError;

/// Who holds a file open, behind a trait so the watch thread's tests give the answer.
///
/// Callers may rely on: `Ok` with an empty list means no process has the file open now; every `ProcessRef` carries
/// the start time Windows reports for that pid, the one `process::Liveness` compares.
pub trait LockOwners: Send {
    fn holders(&self, path: &Path) -> Result<Vec<ProcessRef>, WinError>;
}

/// The real answer, from Restart Manager.
#[derive(Clone, Copy, Debug, Default)]
pub struct RestartManager;

impl LockOwners for RestartManager {
    fn holders(&self, path: &Path) -> Result<Vec<ProcessRef>, WinError> {
        holders(path)
    }
}

/// `RmGetList` calls before giving up while the list keeps growing between the call that sized it and the next.
const TRIES: usize = 4;

/// Room for processes that open the file between two `RmGetList` calls.
const SLACK: usize = 4;

/// The processes that have `path` open now. A missing file has none. Each call is its own Restart Manager session,
/// ended before it returns, so no session outlives a call (Restart Manager has a session limit,
/// `ERROR_MAX_SESSIONS_REACHED`).
pub fn holders(path: &Path) -> Result<Vec<ProcessRef>, WinError> {
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain([0]).collect();
    let session = Session::start()?;
    let files = [PCWSTR(wide.as_ptr())];
    // SAFETY: `files` holds one pointer to `wide`, which ends in a nul and outlives the call; `session.0` is a live
    // session handle.
    let code = unsafe { RmRegisterResources(session.0, Some(&files), None, None) };
    check("RmRegisterResources", code)?;
    let mut list: Vec<RM_PROCESS_INFO> = Vec::new();
    for _ in 0..TRIES {
        let mut needed = 0u32;
        let mut count = u32::try_from(list.len()).unwrap_or(u32::MAX);
        let mut reasons = 0u32;
        let buffer = (!list.is_empty()).then_some(list.as_mut_ptr());
        // SAFETY: `count` is the length of `list`, the buffer `buffer` points at, or 0 with no buffer; the other
        // pointers are locals that live through the call.
        let code = unsafe { RmGetList(session.0, &mut needed, &mut count, buffer, &mut reasons) };
        if code == ERROR_SUCCESS {
            list.truncate(count as usize);
            return Ok(list.iter().map(process_ref).collect());
        }
        if code != ERROR_MORE_DATA {
            return Err(WinError {
                call: "RmGetList",
                code: code.0,
            });
        }
        list.resize(needed as usize + SLACK, RM_PROCESS_INFO::default());
    }
    Err(WinError {
        call: "RmGetList",
        code: ERROR_MORE_DATA.0,
    })
}

/// A Restart Manager session, ended on drop so every return path ends it.
struct Session(u32);

impl Session {
    fn start() -> Result<Self, WinError> {
        let mut handle = 0u32;
        let mut key = [0u16; CCH_RM_SESSION_KEY as usize + 1];
        // SAFETY: `key` has room for the session key and its nul, as the call requires; `handle` is a local.
        let code = unsafe { RmStartSession(&mut handle, None, PWSTR(key.as_mut_ptr())) };
        check("RmStartSession", code)?;
        Ok(Self(handle))
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        // SAFETY: the handle came from `RmStartSession` and is ended once, here. A failed end leaves nothing to do:
        // the session ends with this process anyway.
        let _ = unsafe { RmEndSession(self.0) };
    }
}

fn check(call: &'static str, code: WIN32_ERROR) -> Result<(), WinError> {
    if code == ERROR_SUCCESS {
        Ok(())
    } else {
        Err(WinError { call, code: code.0 })
    }
}

fn process_ref(info: &RM_PROCESS_INFO) -> ProcessRef {
    let start = info.Process.ProcessStartTime;
    ProcessRef {
        pid: info.Process.dwProcessId,
        start: FileTime((u64::from(start.dwHighDateTime) << 32) | u64::from(start.dwLowDateTime)),
    }
}

#[cfg(test)]
mod tests {
    use std::fs::{File, OpenOptions, TryLockError};
    use std::io::{BufRead, BufReader, Write};
    use std::path::PathBuf;
    use std::process::{Child, Command, Stdio};
    use std::time::Duration;

    use super::*;
    use crate::process::{self, Liveness, SystemLiveness};
    use crate::watch::read::TempDir;

    const HOLD_ARG: &str = "wakeshot-test-hold=";
    /// A child that outlives its test (a crashed run) lets go on its own after this.
    const HOLD_MAX: Duration = Duration::from_secs(120);

    /// Opens and locks `path` the way Codex 0.167.0 takes a thread lock file (`rollout/src/writer_lock.rs`): read
    /// and write, created if missing, not truncated, then an exclusive lock on the handle.
    fn hold_like_codex(path: &Path) -> File {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)
            .unwrap();
        file.try_lock().unwrap();
        file
    }

    /// The child role. Started as `<this exe> lockowner::tests::hold_role --exact --ignored wakeshot-test-hold=<path>`
    /// it holds the file, says `held` on stdout and lets go when stdin closes; started any other way it ends at once.
    /// The last argument is a test filter that matches nothing, so the harness takes it.
    #[test]
    #[ignore = "a child role for the holder test"]
    fn hold_role() {
        let Some(path) = std::env::args().find_map(|a| a.strip_prefix(HOLD_ARG).map(PathBuf::from))
        else {
            return;
        };
        let file = hold_like_codex(&path);
        // Straight to the handle: the harness captures only the print macros.
        let mut out = std::io::stdout().lock();
        out.write_all(b"held\n").unwrap();
        out.flush().unwrap();
        drop(out);
        let (done, wait) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = std::io::stdin().read_line(&mut String::new());
            let _ = done.send(());
        });
        let _ = wait.recv_timeout(HOLD_MAX);
        drop(file);
    }

    /// A holder child, ended and reaped on drop, so a failed assert leaves no process.
    struct Holder(Child);

    impl Holder {
        fn start(path: &Path) -> Self {
            let mut arg = std::ffi::OsString::from(HOLD_ARG);
            arg.push(path);
            let mut child = Command::new(std::env::current_exe().unwrap())
                .args([
                    "lockowner::tests::hold_role",
                    "--exact",
                    "--ignored",
                    "--test-threads=1",
                    "--quiet",
                ])
                .arg(arg)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
                .unwrap();
            let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
            let holder = Self(child);
            assert!(
                lines.any(|line| line.is_ok_and(|l| l.trim() == "held")),
                "the child never held the file"
            );
            holder
        }

        fn process(&self) -> ProcessRef {
            ProcessRef {
                pid: self.0.id(),
                start: process::start_of(&self.0).unwrap(),
            }
        }

        /// Closes its stdin, so it lets go and ends by itself.
        fn release(&mut self) {
            drop(self.0.stdin.take());
            self.0.wait().unwrap();
        }
    }

    impl Drop for Holder {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }

    #[test]
    fn a_file_this_process_holds_names_this_process_and_stays_locked() {
        let dir = TempDir::new("lockowner-self");
        let path = dir.path().join("0199aaaa-0000-7000-8000-000000000001.lock");
        let mut held = hold_like_codex(&path);
        let me = process::current().unwrap();

        assert_eq!(holders(&path).unwrap(), [me]);
        assert_eq!(RestartManager.holders(&path).unwrap(), [me]);

        // The start Restart Manager gives is the one the system process list gives, so the watch thread can match
        // a holder with its image.
        let listed = SystemLiveness.list().unwrap();
        let entry = listed.iter().find(|e| e.pid == me.pid).unwrap();
        assert_eq!(entry.start, me.start);

        // Still locked, still writable: the question did not disturb the holder.
        let other = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .unwrap();
        assert!(matches!(other.try_lock(), Err(TryLockError::WouldBlock)));
        held.write_all(b"x").unwrap();
        drop(other);

        drop(held);
        assert_eq!(holders(&path).unwrap(), []);
    }

    #[test]
    fn a_file_nobody_holds_or_that_is_missing_has_no_holders() {
        let dir = TempDir::new("lockowner-free");
        let free = dir.write("free.lock", b"");
        assert_eq!(holders(&free).unwrap(), []);
        assert_eq!(holders(&dir.path().join("missing.lock")).unwrap(), []);
    }

    #[test]
    fn another_process_holding_the_file_is_named_with_its_start() {
        let dir = TempDir::new("lockowner-child");
        let path = dir.path().join("0199aaaa-0000-7000-8000-000000000002.lock");
        let mut holder = Holder::start(&path);
        let child = holder.process();

        assert_eq!(holders(&path).unwrap(), [child]);
        assert_eq!(
            SystemLiveness.check(child),
            wakeshot_core::process::Alive::Yes
        );
        // A test runner may be elevated with `SeDebugPrivilege`; the tray is not, so the answer must not come from a
        // privilege.
        assert_eq!(without_privileges(|| holders(&path)).unwrap(), [child]);

        holder.release();
        assert_eq!(holders(&path).unwrap(), []);
    }

    /// Runs `f` on this thread as this user's token with every privilege removed.
    fn without_privileges<T>(f: impl FnOnce() -> T) -> T {
        use windows::Win32::Foundation::{CloseHandle, HANDLE};
        use windows::Win32::Security::{
            CreateRestrictedToken, DISABLE_MAX_PRIVILEGE, ImpersonateLoggedOnUser, RevertToSelf,
            TOKEN_DUPLICATE, TOKEN_IMPERSONATE, TOKEN_QUERY,
        };
        use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

        struct Owned(HANDLE);
        impl Drop for Owned {
            fn drop(&mut self) {
                // SAFETY: the handle was opened here and is closed once.
                let _ = unsafe { CloseHandle(self.0) };
            }
        }
        struct Revert;
        impl Drop for Revert {
            fn drop(&mut self) {
                // SAFETY: plain call; it ends this thread's impersonation.
                let _ = unsafe { RevertToSelf() };
            }
        }
        let mut token = HANDLE::default();
        // SAFETY: the pseudo handle needs no close; the token is closed by `Owned`.
        unsafe {
            OpenProcessToken(
                GetCurrentProcess(),
                TOKEN_DUPLICATE | TOKEN_QUERY | TOKEN_IMPERSONATE,
                &mut token,
            )
        }
        .unwrap();
        let token = Owned(token);
        let mut bare = HANDLE::default();
        // SAFETY: `token` has TOKEN_DUPLICATE; the new token is closed by `Owned`.
        unsafe {
            CreateRestrictedToken(token.0, DISABLE_MAX_PRIVILEGE, None, None, None, &mut bare)
        }
        .unwrap();
        let bare = Owned(bare);
        // SAFETY: a token of this same user; `Revert` ends the impersonation even if `f` panics.
        unsafe { ImpersonateLoggedOnUser(bare.0) }.unwrap();
        let _revert = Revert;
        f()
    }

    #[test]
    fn many_calls_leave_no_session_behind() {
        let dir = TempDir::new("lockowner-many");
        let path = dir.write("a.lock", b"");
        // A leaked session per call would reach Restart Manager's session limit long before the end.
        for _ in 0..200 {
            assert_eq!(holders(&path).unwrap(), []);
        }
    }
}
