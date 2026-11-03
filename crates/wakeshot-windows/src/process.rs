//! Process identity and liveness, exit waits, the system process list, and spawns outside the tray's process tree
//! (architecture section 5, "Liveness" and "Guard").
//!
//! Owner: power track (`p3-process`). The signatures and `FakeLiveness` are contract: the sources track checks
//! sessions and hold files through `Liveness`, and the ship track's hooks check the journal owner.

use std::ffi::OsString;
use std::path::Path;
#[cfg(any(test, feature = "test-support"))]
use std::sync::Mutex;

use wakeshot_core::process::{Alive, FileTime, Lookup, ProcessRef};
use windows::Win32::Foundation::HANDLE;

use crate::WinError;

/// One process from the system process list. `image` is the exe file name only, never a path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcessEntry {
    pub pid: u32,
    pub start: FileTime,
    pub image: String,
}

/// Liveness checks behind one trait, so the watch thread and the hooks test with `FakeLiveness`.
///
/// Callers may rely on: `check` answers `Unknown` only when the check itself failed; `start_of_pid` is for hold files
/// that give only a pid; `list` is the whole list or an error.
pub trait Liveness: Send + Sync {
    fn check(&self, process: ProcessRef) -> Alive;
    fn start_of_pid(&self, pid: u32) -> Lookup;
    fn list(&self) -> Result<Vec<ProcessEntry>, WinError>;
}

/// The real checks.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemLiveness;

impl Liveness for SystemLiveness {
    fn check(&self, process: ProcessRef) -> Alive {
        liveness(process)
    }

    fn start_of_pid(&self, pid: u32) -> Lookup {
        let _ = pid;
        unimplemented!("p3-process")
    }

    fn list(&self) -> Result<Vec<ProcessEntry>, WinError> {
        unimplemented!("p3-process")
    }
}

/// One liveness check of a pid and its start time.
pub fn liveness(process: ProcessRef) -> Alive {
    let _ = process;
    unimplemented!("p3-process")
}

/// This process.
pub fn current() -> Result<ProcessRef, WinError> {
    unimplemented!("p3-process")
}

/// This process's session id.
pub fn current_session() -> Result<u32, WinError> {
    unimplemented!("p3-process")
}

/// The start time of a child this process started.
pub fn start_of(child: &std::process::Child) -> Result<FileTime, WinError> {
    let _ = child;
    unimplemented!("p3-process")
}

/// An open process handle with the rights to wait on it and read its start time.
#[derive(Debug)]
pub struct ProcessHandle {
    _private: (),
}

impl ProcessHandle {
    /// `None`: no process with that pid and start time.
    pub fn open(process: ProcessRef) -> Result<Option<Self>, WinError> {
        let _ = process;
        unimplemented!("p3-process")
    }

    pub fn process(&self) -> ProcessRef {
        unimplemented!("p3-process")
    }

    /// `true`: the process ended within `timeout_ms`.
    pub fn wait(&self, timeout_ms: u32) -> Result<bool, WinError> {
        let _ = timeout_ms;
        unimplemented!("p3-process")
    }

    pub fn raw(&self) -> HANDLE {
        unimplemented!("p3-process")
    }
}

/// A process exit wait on the thread pool (`RegisterWaitForSingleObject`). The callback only wakes its owner; the
/// wait ends on drop.
#[derive(Debug)]
pub struct ExitWait {
    _private: (),
}

impl ExitWait {
    pub fn register(
        process: &ProcessHandle,
        on_exit: Box<dyn Fn() + Send + Sync>,
    ) -> Result<Self, WinError> {
        let _ = (process, on_exit);
        unimplemented!("p3-process")
    }
}

/// A started process. `outside_tree`: it was started outside this process's tree, so ending this process's tree
/// does not end it (architecture section 5, "Guard").
#[derive(Debug)]
pub struct Spawned {
    pub handle: ProcessHandle,
    pub outside_tree: bool,
}

/// Starts `exe` with `args` outside this process's tree (the guard), or as its own child when that is not possible.
pub fn spawn_outside_tree(exe: &Path, args: &[OsString]) -> Result<Spawned, WinError> {
    let _ = (exe, args);
    unimplemented!("p3-process")
}

/// Liveness answers the test sets, and a record of every check. A process not in `alive` is `Gone`; a pid not in
/// `starts` is `NotRunning`; `list` returns `processes`, or `list_error` when it is set.
///
/// Owner: power track (`p3-process`); first version from the contracts card.
#[cfg(any(test, feature = "test-support"))]
#[derive(Debug, Default)]
pub struct FakeLiveness {
    pub alive: Mutex<Vec<(ProcessRef, Alive)>>,
    pub starts: Mutex<Vec<(u32, Lookup)>>,
    pub processes: Mutex<Vec<ProcessEntry>>,
    pub list_error: Mutex<Option<WinError>>,
    pub checked: Mutex<Vec<ProcessRef>>,
}

#[cfg(any(test, feature = "test-support"))]
fn locked<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(any(test, feature = "test-support"))]
impl Liveness for FakeLiveness {
    fn check(&self, process: ProcessRef) -> Alive {
        locked(&self.checked).push(process);
        locked(&self.alive)
            .iter()
            .find(|(p, _)| *p == process)
            .map_or(Alive::Gone, |&(_, a)| a)
    }

    fn start_of_pid(&self, pid: u32) -> Lookup {
        locked(&self.starts)
            .iter()
            .find(|(p, _)| *p == pid)
            .map_or(Lookup::NotRunning, |&(_, l)| l)
    }

    fn list(&self) -> Result<Vec<ProcessEntry>, WinError> {
        match *locked(&self.list_error) {
            Some(e) => Err(e),
            None => Ok(locked(&self.processes).clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fake_liveness_answers_as_scripted() {
        let tray = ProcessRef {
            pid: 9120,
            start: FileTime(5),
        };
        let fake = FakeLiveness::default();
        let live: &dyn Liveness = &fake;
        assert_eq!(live.check(tray), Alive::Gone);
        fake.alive.lock().unwrap().push((tray, Alive::Unknown));
        assert_eq!(live.check(tray), Alive::Unknown);
        assert_eq!(*fake.checked.lock().unwrap(), [tray, tray]);

        assert_eq!(live.start_of_pid(7), Lookup::NotRunning);
        fake.starts
            .lock()
            .unwrap()
            .push((7, Lookup::Running(FileTime(9))));
        assert_eq!(live.start_of_pid(7), Lookup::Running(FileTime(9)));

        assert_eq!(live.list(), Ok(vec![]));
        let error = WinError {
            call: "CreateToolhelp32Snapshot",
            code: 5,
        };
        *fake.list_error.lock().unwrap() = Some(error);
        assert_eq!(live.list(), Err(error));
    }
}
