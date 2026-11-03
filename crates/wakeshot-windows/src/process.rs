//! Process identity and liveness, exit waits, the system process list, and spawns outside the tray's process tree
//! (architecture section 5, "Liveness" and "Guard").
//!
//! Owner: power track (`p3-process`). The signatures and `FakeLiveness` are contract: the sources track checks
//! sessions and hold files through `Liveness`, and the ship track's hooks check the journal owner.

use std::ffi::{OsStr, OsString, c_void};
use std::mem::offset_of;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::io::AsRawHandle;
use std::path::Path;
#[cfg(any(test, feature = "test-support"))]
use std::sync::Mutex;

use wakeshot_core::process::{Alive, FileTime, Lookup, ProcessRef};
use windows::Wdk::System::SystemInformation::{NtQuerySystemInformation, SystemProcessInformation};
use windows::Win32::Foundation::{
    CloseHandle, DUPLICATE_SAME_ACCESS, DuplicateHandle, E_INVALIDARG, FILETIME, HANDLE,
    INVALID_HANDLE_VALUE, STATUS_INFO_LENGTH_MISMATCH, UNICODE_STRING, WAIT_OBJECT_0, WAIT_TIMEOUT,
};
use windows::Win32::System::RemoteDesktop::ProcessIdToSessionId;
use windows::Win32::System::Threading::{
    CREATE_NO_WINDOW, CreateProcessW, DeleteProcThreadAttributeList, EXTENDED_STARTUPINFO_PRESENT,
    GetCurrentProcess, GetCurrentProcessId, GetProcessTimes, INFINITE,
    InitializeProcThreadAttributeList, LPPROC_THREAD_ATTRIBUTE_LIST, OpenProcess,
    PROC_THREAD_ATTRIBUTE_PARENT_PROCESS, PROCESS_ACCESS_RIGHTS, PROCESS_CREATE_PROCESS,
    PROCESS_INFORMATION, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE,
    RegisterWaitForSingleObject, STARTUPINFOEXW, STARTUPINFOW, TerminateProcess, UnregisterWaitEx,
    UpdateProcThreadAttribute, WT_EXECUTEONLYONCE, WaitForSingleObject,
};
use windows::Win32::System::WindowsProgramming::SYSTEM_PROCESS_INFORMATION;
use windows::Win32::UI::WindowsAndMessaging::{GetShellWindow, GetWindowThreadProcessId};
use windows::core::{PCWSTR, PWSTR};

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

    /// Any failed open goes to the system list, like `liveness`: the list needs no rights, so an elevated or
    /// otherwise closed process is still found.
    fn start_of_pid(&self, pid: u32) -> Lookup {
        match open_pid(pid) {
            Ok(Opened::Running(_, start)) => Lookup::Running(start),
            Ok(Opened::Ended) => Lookup::NotRunning,
            Err(_) => match system_list() {
                Ok(list) => list
                    .iter()
                    .find(|e| e.pid == pid)
                    .map_or(Lookup::NotRunning, |e| Lookup::Running(e.start)),
                Err(_) => Lookup::Unknown,
            },
        }
    }

    fn list(&self) -> Result<Vec<ProcessEntry>, WinError> {
        Ok(system_list()?
            .into_iter()
            .map(|e| ProcessEntry {
                pid: e.pid,
                start: e.start,
                image: e.image,
            })
            .collect())
    }
}

/// One liveness check of a pid and its start time.
///
/// A failed call on an opened handle falls back to the list too, so only a failed list gives `Unknown`.
pub fn liveness(process: ProcessRef) -> Alive {
    match open_pid(process.pid) {
        Ok(Opened::Running(_, start)) if start == process.start => Alive::Yes,
        Ok(_) => Alive::Gone,
        Err(_) => listed(process),
    }
}

/// This process.
pub fn current() -> Result<ProcessRef, WinError> {
    // SAFETY: plain calls; the pseudo handle needs no close.
    let (pid, me) = unsafe { (GetCurrentProcessId(), GetCurrentProcess()) };
    Ok(ProcessRef {
        pid,
        start: creation_time(me)?,
    })
}

/// This process's session id.
pub fn current_session() -> Result<u32, WinError> {
    let mut session = 0;
    // SAFETY: plain call with a local out value.
    unsafe { ProcessIdToSessionId(GetCurrentProcessId(), &mut session) }
        .map_err(|e| failed("ProcessIdToSessionId", &e))?;
    Ok(session)
}

/// The start time of a child this process started.
pub fn start_of(child: &std::process::Child) -> Result<FileTime, WinError> {
    creation_time(HANDLE(child.as_raw_handle()))
}

/// An open process handle with the rights to wait on it and read its start time.
#[derive(Debug)]
pub struct ProcessHandle {
    handle: Owned,
    process: ProcessRef,
}

// SAFETY: a process handle is a kernel handle that any thread of this process may use or close.
unsafe impl Send for ProcessHandle {}
// SAFETY: every method takes `&self` and only passes the handle to thread-safe kernel calls.
unsafe impl Sync for ProcessHandle {}

impl ProcessHandle {
    /// `None`: no process with that pid and start time.
    ///
    /// A denied open is an error, not `None`: the process may well run, and only the list can tell (`liveness`).
    pub fn open(process: ProcessRef) -> Result<Option<Self>, WinError> {
        match open_pid(process.pid) {
            Ok(Opened::Running(handle, start)) if start == process.start => {
                Ok(Some(Self { handle, process }))
            }
            Ok(_) => Ok(None),
            Err(e) if e.call == "OpenProcess" && e.code == hresult(E_INVALIDARG) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn process(&self) -> ProcessRef {
        self.process
    }

    /// `true`: the process ended within `timeout_ms`.
    pub fn wait(&self, timeout_ms: u32) -> Result<bool, WinError> {
        signaled(self.handle.0, timeout_ms)
    }

    pub fn raw(&self) -> HANDLE {
        self.handle.0
    }
}

/// A process exit wait on the thread pool (`RegisterWaitForSingleObject`). The callback only wakes its owner; the
/// wait ends on drop.
///
/// Drop waits until no callback runs, so after it the callback never runs again. Never drop it inside its own
/// callback: that drop would wait for itself. The callback runs on a pool thread behind a system call frame, so a
/// panic in it ends the process.
#[derive(Debug)]
pub struct ExitWait {
    wait: HANDLE,
    on_exit: *mut OnExit,
    // A handle of its own: the pool must not see the waited handle close while the wait is registered (Microsoft,
    // RegisterWaitForSingleObject), whatever the caller does with theirs.
    process: HANDLE,
}

type OnExit = Box<dyn Fn() + Send + Sync>;

// SAFETY: the wait and the process handle are kernel handles any thread may unregister or close, and the callback
// is `Send + Sync`.
unsafe impl Send for ExitWait {}
// SAFETY: there is no method that takes `&self`.
unsafe impl Sync for ExitWait {}

impl ExitWait {
    pub fn register(
        process: &ProcessHandle,
        on_exit: Box<dyn Fn() + Send + Sync>,
    ) -> Result<Self, WinError> {
        let process = duplicate(process.raw())?;
        let mut this = Self {
            wait: HANDLE::default(),
            on_exit: Box::into_raw(Box::new(on_exit)),
            process,
        };
        // SAFETY: `fire` reads `on_exit` only while the wait is registered, and drop unregisters before freeing it.
        // Once: a process handle stays signaled, so a repeating wait would call back without end.
        unsafe {
            RegisterWaitForSingleObject(
                &mut this.wait,
                this.process,
                Some(fire),
                Some(this.on_exit.cast_const().cast()),
                INFINITE,
                WT_EXECUTEONLYONCE,
            )
        }
        .map_err(|e| failed("RegisterWaitForSingleObject", &e))?;
        Ok(this)
    }
}

impl Drop for ExitWait {
    fn drop(&mut self) {
        if !self.wait.is_invalid() {
            // INVALID_HANDLE_VALUE: return only once no callback runs, so `on_exit` may be freed.
            // SAFETY: `wait` came from RegisterWaitForSingleObject and is unregistered once.
            if unsafe { UnregisterWaitEx(self.wait, Some(INVALID_HANDLE_VALUE)) }.is_err() {
                // A callback may still come, so leak what it reads rather than free it.
                return;
            }
        }
        // SAFETY: `on_exit` came from Box::into_raw in `register`, and no callback can run now; the duplicate is
        // closed once.
        unsafe {
            drop(Box::from_raw(self.on_exit));
            let _ = CloseHandle(self.process);
        }
    }
}

unsafe extern "system" fn fire(context: *mut c_void, _timed_out: bool) {
    // SAFETY: `context` is the `OnExit` boxed in `register`, alive until drop has unregistered the wait.
    let on_exit = unsafe { &*context.cast_const().cast::<OnExit>() };
    on_exit();
}

/// A started process. `outside_tree`: it was started outside this process's tree, so ending this process's tree
/// does not end it (architecture section 5, "Guard").
#[derive(Debug)]
pub struct Spawned {
    pub handle: ProcessHandle,
    pub outside_tree: bool,
}

/// Starts `exe` with `args` outside this process's tree (the guard), or as its own child when that is not possible.
///
/// `exe` is a full path: it is passed as the application name too, so a space in it never runs another exe. The
/// parent is the shell's Explorer (`GetShellWindow`); the child then also takes Explorer's token (checked: an
/// elevated caller gets a Medium child) and job (Microsoft docs; not tested). No shell
/// window, a failed open of Explorer, or a failed start under it: a normal child, `outside_tree: false`, so the caller
/// logs it (track note 9). The child gets no console window and inherits no handles.
pub fn spawn_outside_tree(exe: &Path, args: &[OsString]) -> Result<Spawned, WinError> {
    let line = command_line(exe.as_os_str(), args)?;
    if let Some(shell) = shell_process()
        && let Ok(handle) = create(exe, line.clone(), Some(&shell))
    {
        return Ok(Spawned {
            handle,
            outside_tree: true,
        });
    }
    Ok(Spawned {
        handle: create(exe, line, None)?,
        outside_tree: false,
    })
}

/// Exactly these rights: an elevated process denies a Medium caller every read right beyond them, and one extra right
/// fails the whole open (research, "Reading elevated Claude Code sessions").
const RIGHTS: PROCESS_ACCESS_RIGHTS =
    PROCESS_ACCESS_RIGHTS(PROCESS_SYNCHRONIZE.0 | PROCESS_QUERY_LIMITED_INFORMATION.0);

/// Closes the handle on drop.
#[derive(Debug)]
struct Owned(HANDLE);

impl Drop for Owned {
    fn drop(&mut self) {
        // SAFETY: the handle was opened for this value and is closed once.
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

/// What one open with `RIGHTS` found.
#[derive(Debug)]
enum Opened {
    Running(Owned, FileTime),
    /// Open, but already ended: an ended process stays open while anyone holds a handle to it.
    Ended,
}

/// An error from the open or from a call on its handle.
fn open_pid(pid: u32) -> Result<Opened, WinError> {
    // SAFETY: plain call; the handle is owned from here on.
    let handle =
        Owned(unsafe { OpenProcess(RIGHTS, false, pid) }.map_err(|e| failed("OpenProcess", &e))?);
    let start = creation_time(handle.0)?;
    if signaled(handle.0, 0)? {
        return Ok(Opened::Ended);
    }
    Ok(Opened::Running(handle, start))
}

/// The list path of `liveness`.
fn listed(process: ProcessRef) -> Alive {
    let Ok(list) = system_list() else {
        return Alive::Unknown;
    };
    let found = list
        .iter()
        .any(|e| e.pid == process.pid && e.start == process.start);
    if found { Alive::Yes } else { Alive::Gone }
}

fn creation_time(process: HANDLE) -> Result<FileTime, WinError> {
    let [mut created, mut exited, mut kernel, mut user] = [FILETIME::default(); 4];
    // SAFETY: `process` has at least QUERY_LIMITED_INFORMATION; the out values are local.
    unsafe { GetProcessTimes(process, &mut created, &mut exited, &mut kernel, &mut user) }
        .map_err(|e| failed("GetProcessTimes", &e))?;
    Ok(FileTime(
        (u64::from(created.dwHighDateTime) << 32) | u64::from(created.dwLowDateTime),
    ))
}

fn signaled(process: HANDLE, timeout_ms: u32) -> Result<bool, WinError> {
    // SAFETY: `process` has SYNCHRONIZE.
    let waited = unsafe { WaitForSingleObject(process, timeout_ms) };
    if waited == WAIT_OBJECT_0 {
        Ok(true)
    } else if waited == WAIT_TIMEOUT {
        Ok(false)
    } else {
        Err(failed(
            "WaitForSingleObject",
            &windows::core::Error::from_thread(),
        ))
    }
}

fn duplicate(handle: HANDLE) -> Result<HANDLE, WinError> {
    let mut copy = HANDLE::default();
    // SAFETY: both process arguments are this process's pseudo handle; the caller owns `copy`.
    unsafe {
        DuplicateHandle(
            GetCurrentProcess(),
            handle,
            GetCurrentProcess(),
            &mut copy,
            0,
            false,
            DUPLICATE_SAME_ACCESS,
        )
    }
    .map_err(|e| failed("DuplicateHandle", &e))?;
    Ok(copy)
}

fn failed(call: &'static str, e: &windows::core::Error) -> WinError {
    WinError {
        call,
        code: hresult(e.code()),
    }
}

fn hresult(code: windows::core::HRESULT) -> u32 {
    code.0 as u32
}

/// One record of the system process list, with the fields this module reads. An ended process is not listed even
/// while a handle to it is open (`docs/research.md`, "Process module checks"), so a listed pid runs.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Listed {
    pid: u32,
    parent: u32,
    start: FileTime,
    image: String,
}

/// How often the list call may say "too small" before the answer is an error.
const LIST_TRIES: usize = 8;

/// Every process, from `NtQuerySystemInformation(SystemProcessInformation)`: it needs no process handle or right.
/// The error code is the call's NTSTATUS.
fn system_list() -> Result<Vec<Listed>, WinError> {
    const CALL: &str = "NtQuerySystemInformation";
    // Room for a few hundred processes; the call says when it needs more.
    let mut size = 1usize << 20;
    for _ in 0..LIST_TRIES {
        // u64 items keep every record's 8 byte fields aligned.
        let mut buf = vec![0u64; size.div_ceil(8)];
        let bytes = buf.len() * 8;
        let mut needed = 0u32;
        // SAFETY: the buffer holds `bytes` bytes and the call writes at most the length it is given.
        let status = unsafe {
            NtQuerySystemInformation(
                SystemProcessInformation,
                buf.as_mut_ptr().cast(),
                u32::try_from(bytes).unwrap_or(u32::MAX),
                &mut needed,
            )
        };
        if status == STATUS_INFO_LENGTH_MISMATCH {
            // The list can grow between two calls.
            size = (needed as usize).max(bytes) + (64 << 10);
            continue;
        }
        if status.is_err() {
            return Err(WinError {
                call: CALL,
                code: status.0 as u32,
            });
        }
        // SAFETY: the same buffer, viewed as bytes.
        let view = unsafe { std::slice::from_raw_parts(buf.as_ptr().cast::<u8>(), bytes) };
        return Ok(parse_list(view));
    }
    Err(WinError {
        call: CALL,
        code: STATUS_INFO_LENGTH_MISMATCH.0 as u32,
    })
}

/// winternl.h hides `CreateTime` in `Reserved1`, 24 bytes in (after WorkingSetPrivateSize, HardFaultCount,
/// NumberOfThreadsHighWatermark and CycleTime in the NT layout), and the parent pid in `Reserved2`
/// (`InheritedFromUniqueProcessId`). Tests check both against `GetProcessTimes` and a child's real parent.
const CREATE_TIME_IN_RESERVED1: usize = 24;

/// Reads records by bounds-checked offsets, never by casting pointers into the buffer: a record or a name that does
/// not fit ends the list or gives an empty name.
fn parse_list(bytes: &[u8]) -> Vec<Listed> {
    const RECORD: usize = size_of::<SYSTEM_PROCESS_INFORMATION>();
    let start_at = offset_of!(SYSTEM_PROCESS_INFORMATION, Reserved1) + CREATE_TIME_IN_RESERVED1;
    let name_at = offset_of!(SYSTEM_PROCESS_INFORMATION, ImageName);
    let name_len_at = name_at + offset_of!(UNICODE_STRING, Length);
    let name_ptr_at = name_at + offset_of!(UNICODE_STRING, Buffer);
    let pid_at = offset_of!(SYSTEM_PROCESS_INFORMATION, UniqueProcessId);
    let parent_at = offset_of!(SYSTEM_PROCESS_INFORMATION, Reserved2);
    let base = bytes.as_ptr() as usize;

    let mut out = Vec::new();
    let mut at = 0usize;
    while let Some(record) = bytes.get(at..).and_then(|rest| rest.get(..RECORD)) {
        let name_ptr = usize::from_le_bytes(field(record, name_ptr_at));
        let name_len = usize::from(u16::from_le_bytes(field(record, name_len_at)));
        out.push(Listed {
            pid: usize::from_le_bytes(field(record, pid_at)) as u32,
            parent: usize::from_le_bytes(field(record, parent_at)) as u32,
            start: FileTime(u64::from_le_bytes(field(record, start_at))),
            image: image_name(bytes, name_ptr.wrapping_sub(base), name_len),
        });
        let next = u32::from_le_bytes(field(record, 0)) as usize;
        if next == 0 {
            break;
        }
        at += next;
    }
    out
}

fn field<const N: usize>(record: &[u8], at: usize) -> [u8; N] {
    record
        .get(at..at + N)
        .and_then(|b| b.try_into().ok())
        .unwrap_or([0; N])
}

/// The name's text is in the same buffer; `at` is its offset there.
fn image_name(bytes: &[u8], at: usize, len: usize) -> String {
    let Some(raw) = at.checked_add(len).and_then(|end| bytes.get(at..end)) else {
        return String::new();
    };
    let (pairs, _) = raw.as_chunks::<2>();
    let wide: Vec<u16> = pairs.iter().map(|&c| u16::from_le_bytes(c)).collect();
    String::from_utf16_lossy(&wide)
}

/// Explorer, the shell of this desktop, opened to be a parent: `PROCESS_CREATE_PROCESS` is the right the parent
/// attribute needs (Microsoft, UpdateProcThreadAttribute).
fn shell_process() -> Option<Owned> {
    let pid = shell_pid()?;
    // SAFETY: plain call; the handle is owned from here on.
    unsafe { OpenProcess(PROCESS_CREATE_PROCESS, false, pid) }
        .ok()
        .map(Owned)
}

fn shell_pid() -> Option<u32> {
    // SAFETY: plain call; no shell gives a null window.
    let window = unsafe { GetShellWindow() };
    if window.is_invalid() {
        return None;
    }
    let mut pid = 0;
    // SAFETY: `pid` is a local out value; a window that is gone leaves it 0.
    unsafe { GetWindowThreadProcessId(window, Some(&mut pid)) };
    (pid != 0).then_some(pid)
}

/// `line` from `command_line`. A process whose start time cannot be read is ended again: the caller could never
/// check it.
fn create(
    exe: &Path,
    mut line: Vec<u16>,
    parent: Option<&Owned>,
) -> Result<ProcessHandle, WinError> {
    let app: Vec<u16> = exe.as_os_str().encode_wide().chain([0]).collect();
    let attribute = parent.map(|p| ParentAttribute::new(p.0)).transpose()?;
    let mut info = STARTUPINFOEXW::default();
    let mut flags = CREATE_NO_WINDOW;
    if let Some(attribute) = &attribute {
        info.StartupInfo.cb = size_of::<STARTUPINFOEXW>() as u32;
        info.lpAttributeList = attribute.list;
        flags |= EXTENDED_STARTUPINFO_PRESENT;
    } else {
        info.StartupInfo.cb = size_of::<STARTUPINFOW>() as u32;
    }
    let mut created = PROCESS_INFORMATION::default();
    // SAFETY: every buffer outlives the call, `line` is writable as the call requires, and `cb` says which startup
    // struct `info` is.
    unsafe {
        CreateProcessW(
            PCWSTR(app.as_ptr()),
            Some(PWSTR(line.as_mut_ptr())),
            None,
            None,
            false,
            flags,
            None,
            PCWSTR::null(),
            std::ptr::from_ref(&info).cast::<STARTUPINFOW>(),
            &mut created,
        )
    }
    .map_err(|e| failed("CreateProcessW", &e))?;
    let handle = Owned(created.hProcess);
    drop(Owned(created.hThread));
    match creation_time(handle.0) {
        Ok(start) => Ok(ProcessHandle {
            handle,
            process: ProcessRef {
                pid: created.dwProcessId,
                start,
            },
        }),
        Err(e) => {
            // SAFETY: the handle from CreateProcessW has every right.
            unsafe {
                let _ = TerminateProcess(handle.0, 1);
            }
            Err(e)
        }
    }
}

/// A one-entry attribute list that names the parent. The list and the handle value it points to must outlive
/// CreateProcessW (Microsoft, UpdateProcThreadAttribute), so both live here.
struct ParentAttribute {
    list: LPPROC_THREAD_ATTRIBUTE_LIST,
    _buffer: Vec<u64>,
    parent: Box<HANDLE>,
}

impl ParentAttribute {
    fn new(parent: HANDLE) -> Result<Self, WinError> {
        const INIT: &str = "InitializeProcThreadAttributeList";
        let mut size = 0usize;
        // SAFETY: a size query; it fails by design and only sets `size`.
        let query = unsafe { InitializeProcThreadAttributeList(None, 1, None, &mut size) };
        if size == 0 {
            return Err(query.err().map_or(
                WinError {
                    call: INIT,
                    code: 0,
                },
                |e| failed(INIT, &e),
            ));
        }
        let mut buffer = vec![0u64; size.div_ceil(8)];
        let list = LPPROC_THREAD_ATTRIBUTE_LIST(buffer.as_mut_ptr().cast());
        // SAFETY: `buffer` holds at least `size` bytes.
        unsafe { InitializeProcThreadAttributeList(Some(list), 1, None, &mut size) }
            .map_err(|e| failed(INIT, &e))?;
        // From here on drop deletes the list.
        let this = Self {
            list,
            _buffer: buffer,
            parent: Box::new(parent),
        };
        // SAFETY: the value points at the boxed handle, which lives as long as the list.
        unsafe {
            UpdateProcThreadAttribute(
                this.list,
                0,
                PROC_THREAD_ATTRIBUTE_PARENT_PROCESS as usize,
                Some(std::ptr::from_ref::<HANDLE>(&*this.parent).cast()),
                size_of::<HANDLE>(),
                None,
                None,
            )
        }
        .map_err(|e| failed("UpdateProcThreadAttribute", &e))?;
        Ok(this)
    }
}

impl Drop for ParentAttribute {
    fn drop(&mut self) {
        // SAFETY: initialized in `new` and deleted once; the buffer is freed after this, with the fields.
        unsafe { DeleteProcThreadAttributeList(self.list) };
    }
}

const QUOTE: u16 = b'"' as u16;
const BACKSLASH: u16 = b'\\' as u16;

/// The command line for CreateProcessW, with its closing NUL: the exe in quotes (a path holds no quote), then each
/// argument quoted so the C runtime splits it back to the same text (Microsoft, "Parsing C command-line
/// arguments"). A NUL inside would cut the line short, so it is refused.
fn command_line(exe: &OsStr, args: &[OsString]) -> Result<Vec<u16>, WinError> {
    let mut line = vec![QUOTE];
    line.extend(exe.encode_wide());
    line.push(QUOTE);
    for arg in args {
        line.push(u16::from(b' '));
        push_arg(&mut line, arg);
    }
    if line.contains(&0) {
        return Err(WinError {
            call: "CreateProcessW",
            code: hresult(E_INVALIDARG),
        });
    }
    line.push(0);
    Ok(line)
}

fn push_arg(line: &mut Vec<u16>, arg: &OsStr) {
    let wide: Vec<u16> = arg.encode_wide().collect();
    let plain = !wide.is_empty()
        && !wide
            .iter()
            .any(|&c| matches!(c, 0x09..=0x0B | 0x20 | QUOTE));
    if plain {
        line.extend(wide);
        return;
    }
    line.push(QUOTE);
    let mut backslashes = 0;
    for c in wide {
        if c == BACKSLASH {
            backslashes += 1;
        } else {
            if c == QUOTE {
                // Backslashes before a quote escape, so double them and escape the quote.
                line.extend(std::iter::repeat_n(BACKSLASH, backslashes + 1));
            }
            backslashes = 0;
        }
        line.push(c);
    }
    // The same for the closing quote.
    line.extend(std::iter::repeat_n(BACKSLASH, backslashes));
    line.push(QUOTE);
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
    use std::process::{Child, Command, Stdio};
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::{Arc, mpsc};
    use std::time::Duration;

    use windows::Win32::Foundation::{E_ACCESSDENIED, HLOCAL, LocalFree};
    use windows::Win32::Security::Authorization::{
        ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
    };
    use windows::Win32::Security::{
        CreateRestrictedToken, DISABLE_MAX_PRIVILEGE, GetSidSubAuthority, GetSidSubAuthorityCount,
        GetTokenInformation, ImpersonateLoggedOnUser, PSECURITY_DESCRIPTOR, RevertToSelf,
        SECURITY_ATTRIBUTES, TOKEN_DUPLICATE, TOKEN_IMPERSONATE, TOKEN_INFORMATION_CLASS,
        TOKEN_MANDATORY_LABEL, TOKEN_QUERY, TokenElevationType, TokenIntegrityLevel,
    };
    use windows::Win32::System::Threading::{GetExitCodeProcess, OpenProcessToken};
    use windows::Win32::UI::Shell::CommandLineToArgvW;
    use windows::core::w;

    use super::*;

    const SLEEP_ARG: &str = "wakeshot-test-sleep-ms=";
    /// A child that outlives its test (a crashed run) ends on its own after this.
    const SLEEP_MAX_MS: u64 = 120_000;

    /// The child role. Started as `<this exe> process::tests::sleep_role --exact --ignored wakeshot-test-sleep-ms=<ms>`
    /// it sleeps; started any other way it ends at once. The last argument is a test filter that matches nothing, so
    /// the harness takes it.
    #[test]
    #[ignore = "a child role for the other tests of this module"]
    fn sleep_role() {
        let ms = std::env::args().find_map(|a| a.strip_prefix(SLEEP_ARG)?.parse::<u64>().ok());
        if let Some(ms) = ms {
            std::thread::sleep(Duration::from_millis(ms.min(SLEEP_MAX_MS)));
        }
    }

    fn role_args(sleep_ms: Option<u64>) -> Vec<OsString> {
        let mut args: Vec<OsString> = [
            "process::tests::sleep_role",
            "--exact",
            "--ignored",
            "--test-threads=1",
            "--quiet",
        ]
        .map(OsString::from)
        .into();
        if let Some(ms) = sleep_ms {
            args.push(format!("{SLEEP_ARG}{ms}").into());
        }
        args
    }

    fn this_exe() -> std::path::PathBuf {
        std::env::current_exe().expect("test exe path")
    }

    /// A child from `std::process::Command`, ended and reaped on drop, so a failed assert leaves no process.
    struct Kid(Child);

    impl Kid {
        fn start(sleep_ms: Option<u64>) -> Self {
            let child = Command::new(this_exe())
                .args(role_args(sleep_ms))
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .expect("start child");
            println!("child pid {}", child.id());
            Self(child)
        }

        fn process(&self) -> ProcessRef {
            ProcessRef {
                pid: self.0.id(),
                start: start_of(&self.0).expect("child start"),
            }
        }

        /// Ends it and keeps its handle open.
        fn end(&mut self) {
            let _ = self.0.kill();
            self.0.wait().expect("reap child");
        }

        /// Ends it and closes its handle, so the pid may be reused.
        fn end_and_close(mut self) -> ProcessRef {
            let process = self.process();
            self.end();
            process
        }
    }

    impl Drop for Kid {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }

    /// A child from CreateProcessW, ended through its handle on drop.
    struct Created(ProcessHandle);

    impl Created {
        fn end_and_close(self) -> ProcessRef {
            self.0.process()
        }
    }

    impl Drop for Created {
        fn drop(&mut self) {
            // SAFETY: the handle came from CreateProcessW, so it has PROCESS_TERMINATE.
            unsafe {
                let _ = TerminateProcess(self.0.raw(), 1);
            }
            // No assert: a panic here, during a failed test's unwind, would abort the run.
            let _ = self.0.wait(10_000);
        }
    }

    /// A sleeping child whose security descriptor grants nobody anything, so an open by pid is denied to any caller
    /// without SeDebugPrivilege (`without_privileges`). The handle from CreateProcessW still has every right.
    fn start_locked() -> Created {
        let mut sd = PSECURITY_DESCRIPTOR::default();
        // SAFETY: a constant string; `sd` is freed below with LocalFree, as the call documents.
        unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                w!("D:"),
                SDDL_REVISION_1,
                &mut sd,
                None,
            )
        }
        .expect("empty DACL");
        let security = SECURITY_ATTRIBUTES {
            nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: sd.0,
            bInheritHandle: false.into(),
        };
        let exe = this_exe();
        let app: Vec<u16> = exe.as_os_str().encode_wide().chain([0]).collect();
        let mut line = command_line(exe.as_os_str(), &role_args(Some(SLEEP_MAX_MS))).unwrap();
        let info = STARTUPINFOW {
            cb: size_of::<STARTUPINFOW>() as u32,
            ..Default::default()
        };
        let mut created = PROCESS_INFORMATION::default();
        // SAFETY: every buffer outlives the call.
        let result = unsafe {
            CreateProcessW(
                PCWSTR(app.as_ptr()),
                Some(PWSTR(line.as_mut_ptr())),
                Some(std::ptr::from_ref(&security)),
                None,
                false,
                CREATE_NO_WINDOW,
                None,
                PCWSTR::null(),
                &info,
                &mut created,
            )
        };
        // SAFETY: allocated by the conversion above, freed once.
        unsafe {
            let _ = LocalFree(Some(HLOCAL(sd.0)));
        }
        result.expect("start the locked child");
        drop(Owned(created.hThread));
        let handle = Owned(created.hProcess);
        let start = creation_time(handle.0).expect("locked child start");
        println!("locked child pid {}", created.dwProcessId);
        Created(ProcessHandle {
            handle,
            process: ProcessRef {
                pid: created.dwProcessId,
                start,
            },
        })
    }

    /// Runs `f` with this thread on this process's token minus every privilege but SeChangeNotify, so the opens in
    /// `f` meet the DACL as an ordinary caller's do: a runner may hold SeDebugPrivilege turned on (this laptop's
    /// elevated shell does), and that passes any DACL.
    fn without_privileges<T>(f: impl FnOnce() -> T) -> T {
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
        .expect("own token");
        let token = Owned(token);
        let mut bare = HANDLE::default();
        // SAFETY: `token` has TOKEN_DUPLICATE; the new token is closed by `Owned`.
        unsafe {
            CreateRestrictedToken(token.0, DISABLE_MAX_PRIVILEGE, None, None, None, &mut bare)
        }
        .expect("token without privileges");
        let bare = Owned(bare);
        // SAFETY: a token of this same user; `Revert` ends the impersonation even if `f` panics.
        unsafe { ImpersonateLoggedOnUser(bare.0) }.expect("impersonate");
        let _revert = Revert;
        f()
    }

    fn token_info(process: HANDLE, class: TOKEN_INFORMATION_CLASS) -> Vec<u64> {
        let mut token = HANDLE::default();
        // SAFETY: `process` is a live handle; the token is closed by `Owned`.
        unsafe { OpenProcessToken(process, TOKEN_QUERY, &mut token) }.expect("open token");
        let token = Owned(token);
        let mut len = 0u32;
        // SAFETY: a size query; it fails by design and only sets `len`.
        let _ = unsafe { GetTokenInformation(token.0, class, None, 0, &mut len) };
        // u64 items: the answer holds pointers into itself.
        let mut buf = vec![0u64; (len as usize).div_ceil(8)];
        // SAFETY: the buffer holds at least `len` bytes.
        unsafe {
            GetTokenInformation(token.0, class, Some(buf.as_mut_ptr().cast()), len, &mut len)
        }
        .expect("token information");
        buf
    }

    /// The token's integrity level and elevation type, as text for the record.
    fn token_level(process: HANDLE) -> String {
        let label = token_info(process, TokenIntegrityLevel);
        // SAFETY: the buffer holds a TOKEN_MANDATORY_LABEL whose SID points into the same buffer.
        let rid = unsafe {
            let sid = (*label.as_ptr().cast::<TOKEN_MANDATORY_LABEL>()).Label.Sid;
            *GetSidSubAuthority(sid, u32::from(*GetSidSubAuthorityCount(sid)) - 1)
        };
        let level = match rid {
            0x1000 => "Low",
            0x2000 => "Medium",
            0x2100 => "Medium Plus",
            0x3000 => "High",
            0x4000 => "System",
            _ => "other",
        };
        let elevation = match token_info(process, TokenElevationType)[0] as u32 {
            1 => "Default",
            2 => "Full",
            3 => "Limited",
            _ => "other",
        };
        format!("{level} (S-1-16-{rid}), elevation type {elevation}")
    }

    fn opened_text(opened: &Result<Opened, WinError>) -> String {
        match opened {
            Ok(Opened::Running(_, start)) => format!("ok, running, start {start}"),
            Ok(Opened::Ended) => "ok, ended".to_string(),
            Err(e) => format!("{} 0x{:08X}", e.call, e.code),
        }
    }

    #[test]
    fn own_process_is_alive() {
        let me = current().unwrap();
        assert_eq!(me.pid, std::process::id());
        assert_eq!(liveness(me), Alive::Yes);
        assert_eq!(SystemLiveness.check(me), Alive::Yes);
    }

    #[test]
    fn killed_child_is_gone() {
        let kid = Kid::start(Some(SLEEP_MAX_MS));
        assert_eq!(liveness(kid.process()), Alive::Yes);
        let child = kid.end_and_close();
        assert_eq!(liveness(child), Alive::Gone);
        assert!(ProcessHandle::open(child).unwrap().is_none());
    }

    #[test]
    fn wrong_start_time_is_gone() {
        let me = current().unwrap();
        let other = ProcessRef {
            start: FileTime(me.start.0 + 1),
            ..me
        };
        assert_eq!(liveness(other), Alive::Gone);
        assert_eq!(listed(other), Alive::Gone);
        assert!(ProcessHandle::open(other).unwrap().is_none());
    }

    #[test]
    fn exited_child_with_open_handle_is_gone() {
        let mut kid = Kid::start(None);
        let child = kid.process();
        kid.0.wait().unwrap();
        // The open works while `kid` holds its handle, so this is the wait, not the list.
        assert!(matches!(open_pid(child.pid), Ok(Opened::Ended)));
        assert_eq!(liveness(child), Alive::Gone);
        assert!(ProcessHandle::open(child).unwrap().is_none());
        assert_eq!(SystemLiveness.start_of_pid(child.pid), Lookup::NotRunning);
    }

    #[test]
    fn process_list_agrees_with_open() {
        let me = current().unwrap();
        assert_eq!(listed(me), liveness(me));
        assert_eq!(listed(me), Alive::Yes);

        let mut kid = Kid::start(Some(SLEEP_MAX_MS));
        let child = kid.process();
        assert_eq!(listed(child), Alive::Yes);
        assert_eq!(liveness(child), Alive::Yes);

        kid.end();
        let held = system_list()
            .unwrap()
            .into_iter()
            .find(|e| e.pid == child.pid);
        println!("ended child, its handle still open, in the list: {held:?}");
        // `Listed` relies on this: an ended process leaves the list at once, open handles or not.
        assert_eq!(held, None);
        assert_eq!(listed(child), Alive::Gone);
        assert_eq!(liveness(child), Alive::Gone);

        let child = kid.end_and_close();
        assert_eq!(listed(child), Alive::Gone);
        assert_eq!(liveness(child), Alive::Gone);
    }

    #[test]
    fn process_list_has_start_and_parent() {
        let me = current().unwrap();
        let kid = Kid::start(Some(SLEEP_MAX_MS));
        let child = kid.process();
        let list = system_list().unwrap();
        let entry = list
            .iter()
            .find(|e| e.pid == child.pid)
            .expect("child listed");
        assert_eq!(entry.start, child.start);
        assert_eq!(entry.parent, me.pid);
        let own = list.iter().find(|e| e.pid == me.pid).expect("self listed");
        assert_eq!(own.start, me.start);
    }

    #[test]
    fn denied_open_falls_back_to_list() {
        let denied = WinError {
            call: "OpenProcess",
            code: hresult(E_ACCESSDENIED),
        };
        let locked = start_locked();
        let child = locked.0.process();
        println!(
            "locked child open, runner's token: {}",
            opened_text(&open_pid(child.pid))
        );
        without_privileges(|| {
            let opened = open_pid(child.pid);
            println!("locked child open, no privileges: {}", opened_text(&opened));
            assert!(matches!(opened, Err(e) if e == denied));
            assert_eq!(liveness(child), Alive::Yes);
            assert_eq!(
                SystemLiveness.start_of_pid(child.pid),
                Lookup::Running(child.start)
            );
            assert_eq!(ProcessHandle::open(child).unwrap_err(), denied);
        });
        let child = locked.end_and_close();
        assert_eq!(without_privileges(|| liveness(child)), Alive::Gone);

        // The card's example, csrss.exe: recorded with both tokens; alive either way.
        let csrss: Vec<Listed> = system_list()
            .unwrap()
            .into_iter()
            .filter(|e| e.image.eq_ignore_ascii_case("csrss.exe"))
            .collect();
        assert!(!csrss.is_empty());
        for e in csrss {
            let process = ProcessRef {
                pid: e.pid,
                start: e.start,
            };
            let bare = without_privileges(|| opened_text(&open_pid(e.pid)));
            println!(
                "csrss.exe pid {} open, runner's token: {}; no privileges: {bare}",
                e.pid,
                opened_text(&open_pid(e.pid))
            );
            assert_eq!(liveness(process), Alive::Yes);
            assert_eq!(without_privileges(|| liveness(process)), Alive::Yes);
        }
    }

    #[test]
    fn exit_wait_fires_once() {
        let mut kid = Kid::start(Some(SLEEP_MAX_MS));
        let handle = ProcessHandle::open(kid.process()).unwrap().unwrap();
        let fired = Arc::new(AtomicUsize::new(0));
        let count = Arc::clone(&fired);
        let (tx, rx) = mpsc::channel();
        let wait = ExitWait::register(
            &handle,
            Box::new(move || {
                count.fetch_add(1, Ordering::SeqCst);
                let _ = tx.send(());
            }),
        )
        .unwrap();
        // The wait keeps its own handle, so the caller's may close first.
        drop(handle);
        std::thread::sleep(Duration::from_millis(100));
        assert_eq!(fired.load(Ordering::SeqCst), 0);

        kid.end();
        rx.recv_timeout(Duration::from_secs(10))
            .expect("the exit wait fired");
        std::thread::sleep(Duration::from_millis(300));
        drop(wait);
        assert_eq!(fired.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn exit_wait_dropped_never_fires() {
        let mut kid = Kid::start(Some(SLEEP_MAX_MS));
        let handle = ProcessHandle::open(kid.process()).unwrap().unwrap();
        let fired = Arc::new(AtomicUsize::new(0));
        let count = Arc::clone(&fired);
        let wait = ExitWait::register(
            &handle,
            Box::new(move || {
                count.fetch_add(1, Ordering::SeqCst);
            }),
        )
        .unwrap();
        drop(wait);
        kid.end();
        assert!(handle.wait(10_000).unwrap());
        std::thread::sleep(Duration::from_millis(300));
        assert_eq!(fired.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn exit_wait_drop_waits_for_a_running_callback() {
        let mut kid = Kid::start(Some(SLEEP_MAX_MS));
        let handle = ProcessHandle::open(kid.process()).unwrap().unwrap();
        let done = Arc::new(AtomicBool::new(false));
        let finished = Arc::clone(&done);
        let (tx, rx) = mpsc::channel();
        let wait = ExitWait::register(
            &handle,
            Box::new(move || {
                let _ = tx.send(());
                std::thread::sleep(Duration::from_millis(300));
                finished.store(true, Ordering::SeqCst);
            }),
        )
        .unwrap();
        kid.end();
        rx.recv_timeout(Duration::from_secs(10))
            .expect("the callback started");
        drop(wait);
        assert!(done.load(Ordering::SeqCst));
    }

    #[test]
    fn process_handle_waits_for_its_process() {
        let me = current().unwrap();
        let own = ProcessHandle::open(me).unwrap().unwrap();
        assert_eq!(own.process(), me);
        assert!(!own.raw().is_invalid());
        assert!(!own.wait(0).unwrap());

        let mut kid = Kid::start(Some(SLEEP_MAX_MS));
        let child = kid.process();
        let handle = ProcessHandle::open(child).unwrap().unwrap();
        assert!(!handle.wait(50).unwrap());
        kid.end();
        assert!(handle.wait(10_000).unwrap());
    }

    #[test]
    fn start_of_pid_self() {
        assert_eq!(
            SystemLiveness.start_of_pid(std::process::id()),
            Lookup::Running(current().unwrap().start)
        );
    }

    #[test]
    fn start_of_pid_ended_child() {
        let child = Kid::start(Some(SLEEP_MAX_MS)).end_and_close();
        match SystemLiveness.start_of_pid(child.pid) {
            Lookup::NotRunning => {}
            // The pid was taken again at once: still not the child.
            Lookup::Running(start) => assert_ne!(start, child.start),
            Lookup::Unknown => panic!("the lookup failed"),
        }
    }

    #[test]
    fn list_has_self_with_image() {
        let me = current().unwrap();
        let exe = this_exe();
        let image = exe.file_name().unwrap().to_string_lossy().into_owned();
        let list = SystemLiveness.list().unwrap();
        assert!(list.contains(&ProcessEntry {
            pid: me.pid,
            start: me.start,
            image,
        }));
    }

    #[test]
    fn current_session_is_inherited_by_a_child() {
        let session = current_session().unwrap();
        let kid = Kid::start(Some(SLEEP_MAX_MS));
        let mut child_session = u32::MAX;
        // SAFETY: plain call with a local out value.
        unsafe { ProcessIdToSessionId(kid.0.id(), &mut child_session) }.unwrap();
        assert_eq!(child_session, session);
    }

    #[test]
    fn normal_child_has_this_process_as_parent() {
        let me = current().unwrap();
        let line = command_line(this_exe().as_os_str(), &role_args(Some(SLEEP_MAX_MS))).unwrap();
        let created = Created(create(&this_exe(), line, None).unwrap());
        let child = created.0.process();
        println!("normal child pid {}", child.pid);
        let entry = system_list()
            .unwrap()
            .into_iter()
            .find(|e| e.pid == child.pid)
            .expect("child listed");
        assert_eq!(entry.parent, me.pid);
        assert_eq!(entry.start, child.start);
        assert_eq!(liveness(child), Alive::Yes);
        let child = created.end_and_close();
        assert_eq!(liveness(child), Alive::Gone);
    }

    /// Run with `cargo test --locked -p wakeshot-windows -- --ignored process::`. Needs Explorer as the shell.
    #[test]
    #[ignore = "needs Explorer as the shell (track note 3)"]
    fn real_spawn_outside_tree_has_explorer_as_parent() {
        let explorer = shell_pid().expect("a shell window");
        let me = current().unwrap();
        let spawned = spawn_outside_tree(&this_exe(), &role_args(Some(SLEEP_MAX_MS))).unwrap();
        let outside = spawned.outside_tree;
        let created = Created(spawned.handle);
        let child = created.0.process();
        println!(
            "outside-tree child pid {}, explorer pid {explorer}",
            child.pid
        );
        // SAFETY: the pseudo handle needs no close.
        let runner = token_level(unsafe { GetCurrentProcess() });
        let level = token_level(created.0.raw());
        println!("runner token: {runner}; child token: {level}");
        assert!(outside);
        let entry = system_list()
            .unwrap()
            .into_iter()
            .find(|e| e.pid == child.pid)
            .expect("child listed");
        assert_eq!(entry.parent, explorer);
        assert_ne!(entry.parent, me.pid);
        assert_eq!(entry.start, child.start);
        assert_eq!(liveness(child), Alive::Yes);
        let child = created.end_and_close();
        assert_eq!(liveness(child), Alive::Gone);
        if let Some(path) =
            std::env::args().find_map(|a| a.strip_prefix(REPORT_ARG).map(String::from))
        {
            let report = format!("passed; runner token: {runner}; child token: {level}");
            std::fs::write(path, report).expect("write the report");
        }
    }

    /// The nested run of the test above writes its result to this file, so the outer run knows it ran and passed.
    const REPORT_ARG: &str = "wakeshot-test-report=";

    /// The test above, run by a process with Explorer's token (Medium, as the tray normally runs): this test exe,
    /// started under Explorer, runs it, and its exit code is the answer.
    #[test]
    #[ignore = "needs Explorer as the shell (track note 3)"]
    fn real_spawn_outside_tree_works_with_explorers_token() {
        let report =
            std::env::temp_dir().join(format!("wakeshot-process-test-{}.txt", std::process::id()));
        let _ = std::fs::remove_file(&report);
        let mut args: Vec<OsString> = [
            "process::tests::real_spawn_outside_tree_has_explorer_as_parent",
            "--exact",
            "--ignored",
            "--test-threads=1",
            "--quiet",
        ]
        .map(OsString::from)
        .into();
        let mut report_arg = OsString::from(REPORT_ARG);
        report_arg.push(&report);
        args.push(report_arg);
        let spawned = spawn_outside_tree(&this_exe(), &args).unwrap();
        assert!(spawned.outside_tree);
        let nested = Created(spawned.handle);
        println!(
            "nested runner pid {}, token: {}",
            nested.0.process().pid,
            token_level(nested.0.raw())
        );
        assert!(nested.0.wait(60_000).unwrap(), "the nested run ended");
        let mut code = 0;
        // SAFETY: a live handle with every right, from CreateProcessW.
        unsafe { GetExitCodeProcess(nested.0.raw(), &mut code) }.unwrap();
        // A filter that matched no test would also exit 0, so the report is the proof.
        let text = std::fs::read_to_string(&report).expect("the nested run wrote its report");
        let _ = std::fs::remove_file(&report);
        println!("nested run exit code {code}, report: {text}");
        assert_eq!(code, 0, "the nested run passed");
        assert!(text.starts_with("passed"));
    }

    #[test]
    fn command_line_quotes_as_the_runtime_splits() {
        let args: Vec<OsString> = [
            "--guard",
            "a b",
            "",
            "say \"hi\"",
            r"C:\dir\",
            r"C:\my dir\",
            "tab\there",
            r#"back\\"quote"#,
        ]
        .map(OsString::from)
        .into();
        let exe = OsStr::new(r"C:\Program Files\Wakeshot\wakeshot-tray.exe");
        let line = command_line(exe, &args).unwrap();
        let text = String::from_utf16(&line[..line.len() - 1]).unwrap();
        assert_eq!(
            text,
            r#""C:\Program Files\Wakeshot\wakeshot-tray.exe" --guard "a b" "" "say \"hi\"" C:\dir\ "C:\my dir\\" "tab	here" "back\\\\\"quote""#
        );

        let mut count = 0;
        // SAFETY: `line` ends in NUL; the result is freed with LocalFree, as the call documents.
        let split: Vec<String> = unsafe {
            let argv = CommandLineToArgvW(PCWSTR(line.as_ptr()), &mut count);
            assert!(!argv.is_null());
            let out = (0..count as usize)
                .map(|i| (*argv.add(i)).to_string().unwrap())
                .collect();
            let _ = LocalFree(Some(HLOCAL(argv.cast())));
            out
        };
        let mut expected = vec![exe.to_string_lossy().into_owned()];
        expected.extend(args.iter().map(|a| a.to_string_lossy().into_owned()));
        assert_eq!(split, expected);

        let nul = [OsString::from("a\0b")];
        assert_eq!(
            command_line(exe, &nul).unwrap_err().code,
            hresult(E_INVALIDARG)
        );
    }

    #[test]
    fn parse_list_stops_at_a_short_record_and_drops_a_name_outside() {
        const RECORD: usize = size_of::<SYSTEM_PROCESS_INFORMATION>();
        let pid_at = offset_of!(SYSTEM_PROCESS_INFORMATION, UniqueProcessId);
        let name_at = offset_of!(SYSTEM_PROCESS_INFORMATION, ImageName);
        let name_ptr_at = name_at + offset_of!(UNICODE_STRING, Buffer);
        let name: Vec<u8> = "a.exe".encode_utf16().flat_map(u16::to_le_bytes).collect();
        // Record one, its name after it, then record two, whose name points outside, then half a record three.
        let second = (RECORD + name.len()).next_multiple_of(8);
        let mut buf = vec![0u8; second + RECORD + RECORD / 2];
        let base = buf.as_ptr() as usize;
        buf[0..4].copy_from_slice(&(second as u32).to_le_bytes());
        buf[pid_at..pid_at + 8].copy_from_slice(&7usize.to_le_bytes());
        buf[name_at..name_at + 2].copy_from_slice(&(name.len() as u16).to_le_bytes());
        buf[name_ptr_at..name_ptr_at + 8].copy_from_slice(&(base + RECORD).to_le_bytes());
        buf[RECORD..RECORD + name.len()].copy_from_slice(&name);
        let two = second;
        buf[two..two + 4].copy_from_slice(&(RECORD as u32).to_le_bytes());
        buf[two + pid_at..two + pid_at + 8].copy_from_slice(&9usize.to_le_bytes());
        buf[two + name_at..two + name_at + 2].copy_from_slice(&64u16.to_le_bytes());
        buf[two + name_ptr_at..two + name_ptr_at + 8].copy_from_slice(&1usize.to_le_bytes());

        let list = parse_list(&buf);
        assert_eq!(list.len(), 2);
        assert_eq!((list[0].pid, list[0].image.as_str()), (7, "a.exe"));
        assert_eq!((list[1].pid, list[1].image.as_str()), (9, ""));
    }

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
