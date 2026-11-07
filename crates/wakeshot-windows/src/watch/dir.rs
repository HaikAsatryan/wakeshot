//! Folder change notices: overlapped `ReadDirectoryChangesW` on several folders, waited on together with the watch
//! thread's own events, so no folder is polled (architecture section 4, "Tray threads").
//!
//! [`FolderWatch::wait`] gives the names Windows reported, relative to the watched folder, and never opens a file:
//! which names matter is the caller's call, so a `.key` name is only a name here. A burst that overflows a folder's
//! buffer, a folder that goes and a folder that comes back give [`Changed::Rescan`]. A folder that cannot be opened (an
//! agent never run, a folder deleted later) waits on a retry list for [`FolderWatch::retry_missing`], which the watch
//! thread calls on its tick.
//!
//! A `FolderWatch` is not `Send`: make it on the thread that waits on it. Windows cancels a pending read when the
//! thread that issued it ends (`docs/research.md`, "Folder watch with `ReadDirectoryChangesW`").

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fmt;
use std::os::windows::ffi::OsStringExt;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{
    ERROR_INVALID_PARAMETER, ERROR_IO_INCOMPLETE, ERROR_NOTIFY_ENUM_DIR, ERROR_OPERATION_ABORTED,
    HANDLE, STATUS_PENDING, WAIT_ABANDONED_0, WAIT_OBJECT_0, WAIT_TIMEOUT,
};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OVERLAPPED, FILE_LIST_DIRECTORY,
    FILE_NOTIFY_CHANGE, FILE_NOTIFY_CHANGE_DIR_NAME, FILE_NOTIFY_CHANGE_FILE_NAME,
    FILE_NOTIFY_CHANGE_LAST_WRITE, FILE_NOTIFY_CHANGE_SIZE, FILE_SHARE_DELETE, FILE_SHARE_READ,
    FILE_SHARE_WRITE, OPEN_EXISTING, ReadDirectoryChangesW,
};
use windows::Win32::System::IO::{CancelIoEx, GetOverlappedResult, OVERLAPPED};
use windows::Win32::System::Threading::{
    CreateEventW, INFINITE, ResetEvent, WaitForMultipleObjects,
};
use windows::core::HSTRING;

use crate::WinError;

/// Bytes of notices one folder holds between two reads. 64 KB is the most a read of a network folder takes
/// (Microsoft, `ReadDirectoryChangesW`), and a burst beyond it costs one `Rescan`, never a lost change. Change it when
/// the log shows rescans of a folder in normal use.
const BUFFER_BYTES: usize = 64 * 1024;

/// Handles one [`FolderWatch::wait`] takes beside the folders: the watch thread passes its wake and stop events.
pub const MAX_EXTRA: usize = 4;

/// The most folders one watch holds: `WaitForMultipleObjects` takes at most 64 handles (`MAXIMUM_WAIT_OBJECTS`), and
/// the extras need their room.
pub const MAX_FOLDERS: usize = 64 - MAX_EXTRA;

/// Names and sizes, not attributes or access times: a status write changes the size or the write time, an atomic
/// write is a name change, and a new session or day folder is a folder name change.
const FILTER: FILE_NOTIFY_CHANGE = FILE_NOTIFY_CHANGE(
    FILE_NOTIFY_CHANGE_FILE_NAME.0
        | FILE_NOTIFY_CHANGE_DIR_NAME.0
        | FILE_NOTIFY_CHANGE_LAST_WRITE.0
        | FILE_NOTIFY_CHANGE_SIZE.0,
);

/// What changed in one watched folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Changed {
    /// The names Windows reported, relative to `folder`, each once and sorted; a rename gives the old and the new
    /// name. In a subtree watch a deeper name has its folders in front (`<session>\events.jsonl`).
    Names {
        folder: PathBuf,
        names: Vec<PathBuf>,
    },
    /// Notices were lost, or the folder came back or went: read it in full.
    Rescan(PathBuf),
}

impl Changed {
    pub fn folder(&self) -> &Path {
        match self {
            Changed::Names { folder, .. } | Changed::Rescan(folder) => folder,
        }
    }
}

/// Why [`FolderWatch::wait`] returned.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Woke {
    /// One entry per folder that changed, in folder order; never empty.
    Changed(Vec<Changed>),
    /// This handle of `extra` fired (an abandoned mutex counts too). It wins over folder changes, so a stop is never
    /// held up by a busy folder.
    Extra(usize),
    Timeout,
}

/// Where one folder of the watch stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Watching,
    /// On the retry list. The error (an HRESULT) is why the last open or read failed: `CreateFileW` with
    /// `ERROR_FILE_NOT_FOUND` or `ERROR_PATH_NOT_FOUND` for a folder not made yet, `ReadDirectoryChangesW` with
    /// `ERROR_ACCESS_DENIED` for a watched folder that was deleted, and with `ERROR_INVALID_PARAMETER` for a file.
    Missing(WinError),
}

/// Change notices for a set of folders, each one level or its whole subtree.
///
/// Each folder is opened with `FILE_LIST_DIRECTORY` and full sharing, so a rename or delete in it, or of it, still
/// works while it is watched. Like any open handle, a watch blocks renaming a folder above the watched one (deleting
/// it works). Folders are keyed by the path as given: the same folder by two spellings is watched twice.
pub struct FolderWatch {
    folders: BTreeMap<PathBuf, Folder>,
    buffer_bytes: usize,
}

struct Folder {
    subtree: bool,
    /// `Err` while the folder is on the retry list.
    watch: Result<Open, WinError>,
}

/// One open folder with its read.
struct Open {
    dir: OwnedHandle,
    /// Manual reset, the kind Microsoft names for overlapped reads: a wait that consumed an auto-reset event would make
    /// the waiting `GetOverlappedResult` in `drop` hang.
    event: OwnedHandle,
    /// Boxed so the `OVERLAPPED` and the buffer stay where Windows writes them, wherever the `Open` moves. `None` only
    /// in `drop`, which leaks it if the read cannot be ended.
    io: Option<Box<Io>>,
    /// A read is queued, so `io` is Windows' to write until it completes.
    pending: bool,
}

struct Io {
    overlapped: OVERLAPPED,
    /// `u32`: the notice records are DWORD aligned (Microsoft, `ReadDirectoryChangesW`).
    buffer: Box<[u32]>,
}

/// What one completed (or not yet completed) read gave.
enum Taken {
    Pending,
    Names(Vec<PathBuf>),
    Rescan,
    /// The read failed: the folder went, most likely.
    Lost(WinError),
}

impl FolderWatch {
    pub fn new() -> Self {
        Self::with_buffer(BUFFER_BYTES)
    }

    fn with_buffer(buffer_bytes: usize) -> Self {
        Self {
            folders: BTreeMap::new(),
            buffer_bytes,
        }
    }

    /// Watches `dir`, one level or with `subtree` its whole subtree. A folder that cannot be opened now is kept on the
    /// retry list and comes back as [`State::Missing`] with the reason. Adding a folder again opens it again only when
    /// it is missing or the flag differs.
    ///
    /// Fails only when the watch already holds [`MAX_FOLDERS`]; the folder is then not added.
    pub fn add(&mut self, dir: &Path, subtree: bool) -> Result<State, WinError> {
        if let Some(folder) = self.folders.get_mut(dir) {
            if folder.subtree != subtree || folder.watch.is_err() {
                folder.subtree = subtree;
                // The old read ends first, so a folder never has two reads queued at once.
                folder.watch = Err(not_open());
                folder.watch = open(dir, subtree, self.buffer_bytes);
            }
            return Ok(folder.state());
        }
        if self.folders.len() >= MAX_FOLDERS {
            return Err(too_many());
        }
        let folder = Folder {
            subtree,
            watch: open(dir, subtree, self.buffer_bytes),
        };
        let state = folder.state();
        self.folders.insert(dir.to_path_buf(), folder);
        Ok(state)
    }

    /// Stops watching `dir`: its read is cancelled and waited for, then its handles are closed. False when it was not
    /// watched.
    pub fn remove(&mut self, dir: &Path) -> bool {
        self.folders.remove(dir).is_some()
    }

    /// `None` when `dir` was never added or was removed.
    pub fn state(&self, dir: &Path) -> Option<State> {
        self.folders.get(dir).map(Folder::state)
    }

    /// Opens each folder of the retry list again; one `Rescan` for each that opened, since its files may have changed
    /// while it was not watched. A folder that still fails keeps its place with the new reason.
    pub fn retry_missing(&mut self) -> Vec<Changed> {
        let mut back = Vec::new();
        for (dir, folder) in &mut self.folders {
            if folder.watch.is_err() {
                folder.watch = open(dir, folder.subtree, self.buffer_bytes);
                if folder.watch.is_ok() {
                    back.push(Changed::Rescan(dir.clone()));
                }
            }
        }
        back
    }

    /// Waits until a watched folder changes, a handle of `extra` fires, or `timeout` (`None`: no limit) runs out.
    /// `extra` takes at most [`MAX_EXTRA`] waitable handles; auto-reset events suit it, since one that stays set wins
    /// every wait. With no folder open and no extra handle the wait only sleeps out its timeout.
    ///
    /// Fails when `extra` is too long, when there is nothing to wait on and no timeout, or when the wait itself fails.
    pub fn wait(&mut self, timeout: Option<Duration>, extra: &[HANDLE]) -> Result<Woke, WinError> {
        if extra.len() > MAX_EXTRA {
            return Err(too_many());
        }
        let deadline = timeout.and_then(|t| Instant::now().checked_add(t));
        loop {
            let left = deadline.map(|d| d.saturating_duration_since(Instant::now()));
            let mut handles = extra.to_vec();
            handles.extend(
                self.folders
                    .values()
                    .filter_map(|folder| folder.watch.as_ref().ok())
                    .map(Open::event),
            );
            if handles.is_empty() {
                let Some(left) = left else {
                    return Err(too_many());
                };
                std::thread::sleep(left);
                return Ok(Woke::Timeout);
            }
            // SAFETY: every handle stays open until the wait returns: the folder events belong to `self`, which this
            // call borrows mutably, and the caller lends `extra` for the call.
            let woke = unsafe { WaitForMultipleObjects(&handles, false, wait_ms(left)) };
            let count = handles.len() as u32;
            let index = if (WAIT_OBJECT_0.0..WAIT_OBJECT_0.0 + count).contains(&woke.0) {
                (woke.0 - WAIT_OBJECT_0.0) as usize
            } else if (WAIT_ABANDONED_0.0..WAIT_ABANDONED_0.0 + count).contains(&woke.0) {
                (woke.0 - WAIT_ABANDONED_0.0) as usize
            } else if woke == WAIT_TIMEOUT {
                return Ok(Woke::Timeout);
            } else {
                return Err(failed(
                    "WaitForMultipleObjects",
                    &windows::core::Error::from_thread(),
                ));
            };
            if index < extra.len() {
                return Ok(Woke::Extra(index));
            }
            let changed = self.take_all();
            if !changed.is_empty() {
                return Ok(Woke::Changed(changed));
            }
            // A folder event woke the wait with no read done, which a private manual-reset event never does: wait
            // again for what is left of the timeout.
        }
    }

    /// Every completed read, so one busy folder never starves another; each taken folder gets its next read at once,
    /// since Windows keeps the notices that come in between (Microsoft, `ReadDirectoryChangesW`).
    fn take_all(&mut self) -> Vec<Changed> {
        let mut changed = Vec::new();
        for (dir, folder) in &mut self.folders {
            let Ok(open) = &mut folder.watch else {
                continue;
            };
            let taken = open.take();
            let next = match taken {
                Taken::Pending => continue,
                Taken::Names(_) | Taken::Rescan => open.issue(folder.subtree),
                Taken::Lost(e) => Err(e),
            };
            let entry = match (taken, next) {
                (Taken::Names(names), Ok(())) => Changed::Names {
                    folder: dir.clone(),
                    names,
                },
                (_, Ok(())) => Changed::Rescan(dir.clone()),
                (_, Err(e)) => {
                    folder.watch = Err(e);
                    Changed::Rescan(dir.clone())
                }
            };
            changed.push(entry);
        }
        changed
    }
}

impl Default for FolderWatch {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for FolderWatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map()
            .entries(
                self.folders
                    .iter()
                    .map(|(dir, folder)| (dir, (folder.subtree, folder.state()))),
            )
            .finish()
    }
}

impl Folder {
    fn state(&self) -> State {
        match &self.watch {
            Ok(_) => State::Watching,
            Err(e) => State::Missing(*e),
        }
    }
}

/// Opens `dir` and queues its first read.
fn open(dir: &Path, subtree: bool, buffer_bytes: usize) -> Result<Open, WinError> {
    let name = HSTRING::from(dir);
    // SAFETY: the name is a live, NUL-terminated wide string; no security attributes and no template are passed.
    // Backup semantics is what opens a folder; full sharing keeps every rename and delete working.
    let raw = unsafe {
        CreateFileW(
            &name,
            FILE_LIST_DIRECTORY.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            None,
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OVERLAPPED,
            None,
        )
    }
    .map_err(|e| failed("CreateFileW", &e))?;
    // SAFETY: CreateFileW just returned this handle and nothing else owns it, so OwnedHandle may close it.
    let dir = unsafe { OwnedHandle::from_raw_handle(raw.0) };
    // SAFETY: no security attributes and no name are passed.
    let raw =
        unsafe { CreateEventW(None, true, false, None) }.map_err(|e| failed("CreateEventW", &e))?;
    // SAFETY: CreateEventW just returned this handle and nothing else owns it, so OwnedHandle may close it.
    let event = unsafe { OwnedHandle::from_raw_handle(raw.0) };
    let mut open = Open {
        dir,
        event,
        io: Some(Box::new(Io {
            overlapped: OVERLAPPED::default(),
            buffer: vec![0u32; buffer_bytes.div_ceil(4)].into_boxed_slice(),
        })),
        pending: false,
    };
    open.issue(subtree)?;
    Ok(open)
}

impl Open {
    fn dir(&self) -> HANDLE {
        HANDLE(self.dir.as_raw_handle())
    }

    fn event(&self) -> HANDLE {
        HANDLE(self.event.as_raw_handle())
    }

    /// Queues the next read. Only called with no read pending.
    fn issue(&mut self, subtree: bool) -> Result<(), WinError> {
        let (dir, event) = (self.dir(), self.event());
        let Some(io) = self.io.as_mut() else {
            return Err(not_open());
        };
        // SAFETY: the event is open for as long as `self`.
        unsafe { ResetEvent(event) }.map_err(|e| failed("ResetEvent", &e))?;
        io.overlapped = OVERLAPPED {
            hEvent: event,
            ..OVERLAPPED::default()
        };
        let bytes = u32::try_from(io.buffer.len() * 4).unwrap_or(!3);
        // SAFETY: the buffer and the `OVERLAPPED` live in the box, whose address holds until `drop` has cancelled the
        // read and waited for it (or leaked the box); `bytes` is within the buffer; no read is pending on them now.
        unsafe {
            ReadDirectoryChangesW(
                dir,
                io.buffer.as_mut_ptr().cast(),
                bytes,
                subtree,
                FILTER,
                None,
                Some(&mut io.overlapped),
                None,
            )
        }
        .map_err(|e| failed("ReadDirectoryChangesW", &e))?;
        self.pending = true;
        Ok(())
    }

    /// The pending read's result, if it completed.
    fn take(&mut self) -> Taken {
        let dir = self.dir();
        let Some(io) = self.io.as_ref() else {
            return Taken::Lost(not_open());
        };
        if !self.pending {
            return Taken::Rescan;
        }
        let mut bytes = 0u32;
        // SAFETY: the `OVERLAPPED` is the one the pending read on this handle was queued with; no wait is asked for.
        let done = unsafe { GetOverlappedResult(dir, &io.overlapped, &mut bytes, false) };
        match done {
            Err(e) if e.code() == ERROR_IO_INCOMPLETE.to_hresult() => Taken::Pending,
            Ok(()) => {
                self.pending = false;
                let len = (bytes as usize).min(io.buffer.len() * 4);
                // SAFETY: the read completed, so Windows no longer writes the buffer; `len` bytes are within it, and
                // a `u32` slice may be read as bytes.
                let raw =
                    unsafe { std::slice::from_raw_parts(io.buffer.as_ptr().cast::<u8>(), len) };
                // Zero bytes means the buffer overflowed (Microsoft, `ReadDirectoryChangesW`); a result that does
                // not parse loses its names the same way.
                match parse(raw) {
                    Some(names) if !names.is_empty() => Taken::Names(names.into_iter().collect()),
                    _ => Taken::Rescan,
                }
            }
            Err(e) => {
                self.pending = false;
                let code = e.code();
                // `ERROR_NOTIFY_ENUM_DIR` is the other overflow result (Microsoft, `ReadDirectoryChangesW`); a cancel
                // from outside (the thread that queued the read ended) loses notices the same way.
                if code == ERROR_NOTIFY_ENUM_DIR.to_hresult()
                    || code == ERROR_OPERATION_ABORTED.to_hresult()
                {
                    Taken::Rescan
                } else {
                    Taken::Lost(failed("ReadDirectoryChangesW", &e))
                }
            }
        }
    }
}

impl Drop for Open {
    fn drop(&mut self) {
        let Some(io) = self.io.take() else {
            return;
        };
        if self.pending {
            let dir = self.dir();
            let mut bytes = 0u32;
            // SAFETY: the `OVERLAPPED` is the one the pending read on this handle was queued with, and the box, the
            // handle and the event all stay alive until the wait returns (fields drop after this body). A cancel that
            // finds the read already done is harmless. `Internal` is read through a raw pointer, volatile, as the
            // `HasOverlappedIoCompleted` macro does.
            let still_pending = unsafe {
                let _ = CancelIoEx(dir, Some(&io.overlapped));
                let _ = GetOverlappedResult(dir, &io.overlapped, &mut bytes, true);
                std::ptr::read_volatile(&raw const io.overlapped.Internal)
                    == STATUS_PENDING.0 as usize
            };
            if still_pending {
                // Only a failed wait gets here. Windows may still write the box, so it is never freed.
                std::mem::forget(io);
                return;
            }
        }
        drop(io);
    }
}

/// The names in a run of `FILE_NOTIFY_INFORMATION` records (`NextEntryOffset`, `Action`, `FileNameLength` in bytes,
/// then the UTF-16 name). `None` when a record runs past the end or the next one is not DWORD aligned, which a
/// well-formed result never does. Names stay as Windows gave them, unpaired surrogates included.
fn parse(bytes: &[u8]) -> Option<BTreeSet<PathBuf>> {
    let u32_at = |at: usize| {
        let field = bytes.get(at..at.checked_add(4)?)?;
        Some(u32::from_le_bytes(field.try_into().ok()?))
    };
    let mut names = BTreeSet::new();
    let mut at = 0usize;
    loop {
        let next = u32_at(at)? as usize;
        let len = u32_at(at + 8)? as usize;
        let start = at + 12;
        let raw = bytes.get(start..start.checked_add(len)?)?;
        if !len.is_multiple_of(2) {
            return None;
        }
        let units: Vec<u16> = raw
            .as_chunks::<2>()
            .0
            .iter()
            .map(|unit| u16::from_le_bytes(*unit))
            .collect();
        if !units.is_empty() {
            names.insert(PathBuf::from(OsString::from_wide(&units)));
        }
        if next == 0 {
            return Some(names);
        }
        if !next.is_multiple_of(4) {
            return None;
        }
        at = at.checked_add(next)?;
    }
}

/// The timeout in whole milliseconds, rounded up so a wait never ends early, and kept below `INFINITE`.
fn wait_ms(left: Option<Duration>) -> u32 {
    match left {
        None => INFINITE,
        Some(left) => {
            let ms = left.as_nanos().div_ceil(1_000_000);
            u32::try_from(ms).unwrap_or(INFINITE - 1).min(INFINITE - 1)
        }
    }
}

fn failed(call: &'static str, e: &windows::core::Error) -> WinError {
    WinError {
        call,
        code: e.code().0 as u32,
    }
}

/// The wait would take more handles than `WaitForMultipleObjects` does, or none at all with no timeout: the error that
/// call gives for both.
fn too_many() -> WinError {
    WinError {
        call: "WaitForMultipleObjects",
        code: ERROR_INVALID_PARAMETER.to_hresult().0 as u32,
    }
}

/// A folder between its old and its new open; never seen outside this module.
fn not_open() -> WinError {
    WinError {
        call: "FolderWatch",
        code: ERROR_INVALID_PARAMETER.to_hresult().0 as u32,
    }
}

#[cfg(test)]
mod tests {
    use std::fs::{self, OpenOptions};
    use std::io::Write;
    use std::process::{Command, Stdio};

    use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_PATH_NOT_FOUND};
    use windows::Win32::System::Threading::{GetCurrentProcess, GetProcessHandleCount, SetEvent};

    use super::*;
    use crate::watch::read::TempDir;

    /// Generous for a notice that comes within milliseconds, so a busy test run does not fail.
    const WITHIN: Duration = Duration::from_secs(10);

    /// What the waits gave, merged per folder.
    #[derive(Debug, Default)]
    struct Seen {
        names: BTreeMap<PathBuf, BTreeSet<PathBuf>>,
        rescans: BTreeMap<PathBuf, usize>,
        first: Option<Instant>,
    }

    impl Seen {
        fn add(&mut self, changed: Vec<Changed>) {
            assert!(!changed.is_empty(), "an empty Changed came back");
            self.first.get_or_insert_with(Instant::now);
            for entry in changed {
                match entry {
                    Changed::Names { folder, names } => {
                        assert!(!names.is_empty(), "no names for {}", folder.display());
                        self.names.entry(folder).or_default().extend(names);
                    }
                    Changed::Rescan(folder) => *self.rescans.entry(folder).or_default() += 1,
                }
            }
        }

        fn has(&self, folder: &Path, name: impl AsRef<Path>) -> bool {
            self.names
                .get(folder)
                .is_some_and(|names| names.contains(name.as_ref()))
        }

        fn rescanned(&self, folder: &Path) -> bool {
            self.rescans.contains_key(folder)
        }
    }

    /// Waits until `done` holds for what came back; panics with what came back if it never does.
    fn wait_until(watch: &mut FolderWatch, done: impl Fn(&Seen) -> bool) -> Seen {
        let mut seen = Seen::default();
        let deadline = Instant::now() + WITHIN;
        while !done(&seen) {
            let left = deadline.saturating_duration_since(Instant::now());
            assert!(!left.is_zero(), "never came; came: {seen:?}");
            match watch.wait(Some(left), &[]).unwrap() {
                Woke::Changed(changed) => seen.add(changed),
                Woke::Timeout => {}
                Woke::Extra(i) => panic!("extra handle {i} fired, but none was given"),
            }
        }
        seen
    }

    /// Everything that comes within `ms`.
    fn drain(watch: &mut FolderWatch, ms: u64) -> Seen {
        let mut seen = Seen::default();
        let deadline = Instant::now() + Duration::from_millis(ms);
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            match watch.wait(Some(left), &[]).unwrap() {
                Woke::Changed(changed) => seen.add(changed),
                Woke::Timeout => return seen,
                Woke::Extra(i) => panic!("extra handle {i} fired, but none was given"),
            }
        }
    }

    /// Prints the notice latency, shown with `--nocapture`; no assert on it.
    fn latency(what: &str, since: Instant, seen: &Seen) {
        let ms = seen
            .first
            .map_or(f64::NAN, |at| (at - since).as_secs_f64() * 1000.0);
        println!("latency {what}: {ms:.2} ms");
    }

    fn auto_event() -> OwnedHandle {
        // SAFETY: no security attributes and no name are passed.
        let raw = unsafe { CreateEventW(None, false, false, None) }.unwrap();
        // SAFETY: CreateEventW just returned this handle and nothing else owns it.
        unsafe { OwnedHandle::from_raw_handle(raw.0) }
    }

    fn handle(event: &OwnedHandle) -> HANDLE {
        HANDLE(event.as_raw_handle())
    }

    fn set(event: &OwnedHandle) {
        // SAFETY: the event is open for the call.
        unsafe { SetEvent(handle(event)) }.unwrap();
    }

    fn not_found(e: &WinError) -> bool {
        e.call == "CreateFileW"
            && [ERROR_FILE_NOT_FOUND, ERROR_PATH_NOT_FOUND]
                .iter()
                .any(|code| e.code == code.to_hresult().0 as u32)
    }

    #[test]
    fn a_new_file_a_write_and_a_delete_are_named() {
        let dir = TempDir::new("dir-basic");
        let mut watch = FolderWatch::new();
        assert_eq!(watch.add(dir.path(), false).unwrap(), State::Watching);
        let file = dir.path().join("1234.json");

        let at = Instant::now();
        fs::write(&file, b"{}").unwrap();
        let seen = wait_until(&mut watch, |s| s.has(dir.path(), "1234.json"));
        latency("create", at, &seen);

        drain(&mut watch, 100);
        let at = Instant::now();
        let mut open = OpenOptions::new().append(true).open(&file).unwrap();
        open.write_all(b"\n").unwrap();
        drop(open);
        let seen = wait_until(&mut watch, |s| s.has(dir.path(), "1234.json"));
        latency("append", at, &seen);

        drain(&mut watch, 100);
        let at = Instant::now();
        fs::remove_file(&file).unwrap();
        let seen = wait_until(&mut watch, |s| s.has(dir.path(), "1234.json"));
        latency("delete", at, &seen);
        assert!(seen.rescans.is_empty(), "{seen:?}");
    }

    #[test]
    fn a_rename_into_place_names_the_old_and_the_new_name() {
        let dir = TempDir::new("dir-rename");
        dir.write("5678.json", b"{\"old\":1}");
        let mut watch = FolderWatch::new();
        watch.add(dir.path(), false).unwrap();

        let at = Instant::now();
        let temp = dir.write("5678.json.tmp.42", b"{\"new\":1}");
        fs::rename(&temp, dir.path().join("5678.json")).unwrap();
        let seen = wait_until(&mut watch, |s| {
            s.has(dir.path(), "5678.json.tmp.42") && s.has(dir.path(), "5678.json")
        });
        latency("rename into place", at, &seen);
    }

    #[test]
    fn names_come_back_as_given() {
        let dir = TempDir::new("dir-names");
        let mut watch = FolderWatch::new();
        watch.add(dir.path(), false).unwrap();
        // A name the caller would refuse, Armenian and Russian names, and an unpaired surrogate, which NTFS takes.
        let odd = PathBuf::from(OsString::from_wide(&[0xD800, u16::from(b'x')]));
        let names = [
            PathBuf::from("4242.key"),
            PathBuf::from("\u{0561}\u{0580}\u{0569}\u{0576}.json"),
            PathBuf::from("\u{0441}\u{0435}\u{0441}\u{0441}\u{0438}\u{044f}.json"),
            odd,
        ];
        for name in &names {
            fs::write(dir.path().join(name), b"x").unwrap();
        }
        let seen = wait_until(&mut watch, |s| names.iter().all(|n| s.has(dir.path(), n)));
        assert!(seen.rescans.is_empty(), "{seen:?}");
    }

    #[test]
    fn a_subtree_watch_names_deeper_files_relative_to_the_folder() {
        let dir = TempDir::new("dir-subtree");
        let mut watch = FolderWatch::new();
        watch.add(dir.path(), true).unwrap();

        let at = Instant::now();
        let events = dir.write("sess-1/events.jsonl", b"{}\n");
        let deep = Path::new("sess-1").join("events.jsonl");
        let seen = wait_until(&mut watch, |s| {
            s.has(dir.path(), "sess-1") && s.has(dir.path(), &deep)
        });
        latency("subtree create", at, &seen);

        drain(&mut watch, 100);
        let at = Instant::now();
        let mut open = OpenOptions::new().append(true).open(&events).unwrap();
        open.write_all(b"{}\n").unwrap();
        drop(open);
        let seen = wait_until(&mut watch, |s| s.has(dir.path(), &deep));
        latency("subtree append", at, &seen);
    }

    #[test]
    fn a_one_level_watch_never_names_deeper_files() {
        let dir = TempDir::new("dir-one-level");
        fs::create_dir(dir.path().join("sub")).unwrap();
        let mut watch = FolderWatch::new();
        watch.add(dir.path(), false).unwrap();

        dir.write("sub/deep.json", b"{}");
        dir.write("top.json", b"{}");
        let seen = wait_until(&mut watch, |s| s.has(dir.path(), "top.json"));
        let later = drain(&mut watch, 300);
        let deep = Path::new("sub").join("deep.json");
        assert!(
            !seen.has(dir.path(), &deep) && !later.has(dir.path(), &deep),
            "{seen:?} {later:?}"
        );
    }

    #[test]
    fn adding_again_with_the_subtree_flag_watches_deeper() {
        let dir = TempDir::new("dir-reflag");
        fs::create_dir(dir.path().join("sub")).unwrap();
        let mut watch = FolderWatch::new();
        assert_eq!(watch.add(dir.path(), false).unwrap(), State::Watching);
        assert_eq!(watch.add(dir.path(), false).unwrap(), State::Watching);
        assert_eq!(watch.add(dir.path(), true).unwrap(), State::Watching);

        dir.write("sub/deep.json", b"{}");
        let deep = Path::new("sub").join("deep.json");
        wait_until(&mut watch, |s| s.has(dir.path(), &deep));
    }

    #[test]
    fn two_folders_are_watched_at_once() {
        let (a, b) = (TempDir::new("dir-two-a"), TempDir::new("dir-two-b"));
        let mut watch = FolderWatch::new();
        watch.add(a.path(), false).unwrap();
        watch.add(b.path(), true).unwrap();

        let at = Instant::now();
        a.write("a.json", b"{}");
        b.write("b.json", b"{}");
        let seen = wait_until(&mut watch, |s| {
            s.has(a.path(), "a.json") && s.has(b.path(), "b.json")
        });
        latency("two folders", at, &seen);
        assert!(!seen.has(a.path(), "b.json") && !seen.has(b.path(), "a.json"));

        assert!(watch.remove(a.path()));
        assert!(!watch.remove(a.path()));
        assert_eq!(watch.state(a.path()), None);
        a.write("a2.json", b"{}");
        b.write("b2.json", b"{}");
        let seen = wait_until(&mut watch, |s| s.has(b.path(), "b2.json"));
        assert!(!seen.names.contains_key(a.path()), "{seen:?}");
    }

    #[test]
    fn a_folder_made_after_the_watch_starts_is_opened_by_retry_missing() {
        let root = TempDir::new("dir-late");
        let late = root.path().join("sessions");
        let deeper = root.path().join("not").join("yet");
        let mut watch = FolderWatch::new();
        for dir in [&late, &deeper] {
            match watch.add(dir, false).unwrap() {
                State::Missing(e) => assert!(not_found(&e), "{e:?}"),
                State::Watching => panic!("{} does not exist yet", dir.display()),
            }
        }
        assert_eq!(watch.retry_missing(), []);
        assert_eq!(
            watch.wait(Some(Duration::from_millis(50)), &[]).unwrap(),
            Woke::Timeout
        );

        fs::create_dir(&late).unwrap();
        assert_eq!(watch.retry_missing(), [Changed::Rescan(late.clone())]);
        assert_eq!(watch.retry_missing(), []);
        assert_eq!(watch.state(&late), Some(State::Watching));
        assert!(matches!(watch.state(&deeper), Some(State::Missing(e)) if not_found(&e)));

        let at = Instant::now();
        fs::write(late.join("1.json"), b"{}").unwrap();
        let seen = wait_until(&mut watch, |s| s.has(&late, "1.json"));
        latency("late folder", at, &seen);
    }

    #[test]
    fn the_watched_folder_deleted_and_made_again_is_watched_again() {
        let root = TempDir::new("dir-gone");
        let sessions = root.path().join("sessions");
        fs::create_dir(&sessions).unwrap();
        fs::write(sessions.join("1.json"), b"{}").unwrap();
        let mut watch = FolderWatch::new();
        watch.add(&sessions, false).unwrap();

        // A plain delete (`DeleteFileW`, then `RemoveDirectoryW`), as an agent cleaning up would do it.
        fs::remove_file(sessions.join("1.json")).unwrap();
        fs::remove_dir(&sessions).unwrap();
        let seen = wait_until(&mut watch, |s| s.rescanned(&sessions));
        println!(
            "after the delete: {seen:?}, state {:?}",
            watch.state(&sessions)
        );
        assert!(matches!(watch.state(&sessions), Some(State::Missing(_))));
        assert_eq!(watch.retry_missing(), []);

        fs::create_dir(&sessions).unwrap();
        assert_eq!(watch.retry_missing(), [Changed::Rescan(sessions.clone())]);
        fs::write(sessions.join("2.json"), b"{}").unwrap();
        wait_until(&mut watch, |s| s.has(&sessions, "2.json"));
    }

    #[test]
    fn a_burst_that_overflows_the_buffer_gives_rescan() {
        let dir = TempDir::new("dir-burst");
        let mut watch = FolderWatch::with_buffer(4096);
        watch.add(dir.path(), false).unwrap();

        let at = Instant::now();
        for n in 0..5000 {
            fs::write(dir.path().join(format!("burst-{n:05}.json")), b"{}").unwrap();
        }
        println!("5,000 files written in {} ms", at.elapsed().as_millis());
        let seen = wait_until(&mut watch, |s| s.rescanned(dir.path()));
        let names = seen.names.get(dir.path()).map_or(0, BTreeSet::len);
        println!("before the rescan: {names} names");
        assert!(names < 5000, "the buffer never overflowed");
        assert_eq!(watch.state(dir.path()), Some(State::Watching));

        // The watch goes on after the overflow.
        drain(&mut watch, 300);
        dir.write("after.json", b"{}");
        wait_until(&mut watch, |s| s.has(dir.path(), "after.json"));
    }

    #[test]
    fn the_wake_and_stop_events_end_the_wait() {
        let dir = TempDir::new("dir-extra");
        let mut watch = FolderWatch::new();
        watch.add(dir.path(), false).unwrap();
        let (wake, stop) = (auto_event(), auto_event());
        let extra = [handle(&wake), handle(&stop)];

        let from_thread = stop.try_clone().unwrap();
        let setter = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(100));
            set(&from_thread);
        });
        let at = Instant::now();
        assert_eq!(watch.wait(Some(WITHIN), &extra).unwrap(), Woke::Extra(1));
        println!("stop ended the wait after {} ms", at.elapsed().as_millis());
        setter.join().unwrap();

        // An auto-reset event fires once.
        let short = Some(Duration::from_millis(50));
        assert_eq!(watch.wait(short, &extra).unwrap(), Woke::Timeout);
        set(&wake);
        assert_eq!(watch.wait(short, &extra).unwrap(), Woke::Extra(0));
    }

    #[test]
    fn an_extra_handle_wins_over_changes_that_wait() {
        let dir = TempDir::new("dir-extra-first");
        let mut watch = FolderWatch::new();
        watch.add(dir.path(), false).unwrap();
        let (wake, stop) = (auto_event(), auto_event());
        let extra = [handle(&wake), handle(&stop)];

        dir.write("1.json", b"{}");
        std::thread::sleep(Duration::from_millis(200));
        set(&stop);
        assert_eq!(watch.wait(Some(WITHIN), &extra).unwrap(), Woke::Extra(1));
        match watch.wait(Some(WITHIN), &extra).unwrap() {
            Woke::Changed(changed) => assert_eq!(
                changed,
                [Changed::Names {
                    folder: dir.path().to_path_buf(),
                    names: vec![PathBuf::from("1.json")],
                }]
            ),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn waits_time_out() {
        let dir = TempDir::new("dir-timeout");
        let mut watch = FolderWatch::new();
        watch.add(dir.path(), false).unwrap();
        let at = Instant::now();
        assert_eq!(
            watch.wait(Some(Duration::from_millis(150)), &[]).unwrap(),
            Woke::Timeout
        );
        let waited = at.elapsed();
        println!("a 150 ms wait took {} ms", waited.as_millis());
        assert!(waited >= Duration::from_millis(120), "{waited:?}");
        assert_eq!(
            watch.wait(Some(Duration::ZERO), &[]).unwrap(),
            Woke::Timeout
        );

        // Nothing to wait on: the wait sleeps out its timeout, or fails with none.
        let mut empty = FolderWatch::new();
        assert_eq!(
            empty.wait(Some(Duration::from_millis(20)), &[]).unwrap(),
            Woke::Timeout
        );
        assert_eq!(empty.wait(None, &[]), Err(too_many()));
    }

    #[test]
    fn too_many_folders_or_extra_handles_fail() {
        let root = TempDir::new("dir-many");
        let mut watch = FolderWatch::new();
        for n in 0..MAX_FOLDERS {
            let state = watch.add(&root.path().join(n.to_string()), false).unwrap();
            assert!(matches!(state, State::Missing(_)));
        }
        assert_eq!(
            watch.add(&root.path().join("one-more"), false),
            Err(too_many())
        );
        assert_eq!(watch.state(&root.path().join("one-more")), None);
        assert!(watch.add(&root.path().join("0"), false).is_ok());

        let events: Vec<OwnedHandle> = (0..=MAX_EXTRA).map(|_| auto_event()).collect();
        let extra: Vec<HANDLE> = events.iter().map(handle).collect();
        let short = Some(Duration::from_millis(10));
        assert_eq!(watch.wait(short, &extra), Err(too_many()));
        assert_eq!(
            watch.wait(short, &extra[..MAX_EXTRA]).unwrap(),
            Woke::Timeout
        );
    }

    #[test]
    fn a_path_that_is_a_file_is_missing() {
        let dir = TempDir::new("dir-file");
        let file = dir.write("plain.txt", b"x");
        let mut watch = FolderWatch::new();
        let state = watch.add(&file, false).unwrap();
        println!("a file path gives {state:?}");
        assert!(matches!(state, State::Missing(_)));
    }

    /// The fact behind the module doc: a read ends when the thread that queued it ends. The watch then reports
    /// `Rescan` and queues a new read, so it goes on.
    #[test]
    fn a_read_cancelled_by_its_thread_ending_gives_rescan_and_goes_on() {
        struct Moved(FolderWatch);
        // SAFETY: test only; moving the watch to another thread is the point, and nothing else holds it.
        unsafe impl Send for Moved {}

        let dir = TempDir::new("dir-thread");
        let path = dir.path().to_path_buf();
        let moved = std::thread::spawn(move || {
            let mut watch = FolderWatch::new();
            watch.add(&path, false).unwrap();
            Moved(watch)
        })
        .join()
        .unwrap();
        let mut watch = moved.0;

        let seen = drain(&mut watch, 300);
        println!("after the queuing thread ended: {seen:?}");
        assert!(seen.rescanned(dir.path()), "{seen:?}");
        dir.write("1.json", b"{}");
        wait_until(&mut watch, |s| s.has(dir.path(), "1.json"));
    }

    #[test]
    fn the_watch_never_blocks_a_rename_or_a_delete() {
        let root = TempDir::new("dir-share");
        let watched = root.path().join("watched");
        fs::create_dir(&watched).unwrap();
        let mut watch = FolderWatch::new();
        watch.add(&watched, true).unwrap();

        let file = watched.join("1.json");
        fs::write(&file, b"{}").unwrap();
        fs::rename(&file, watched.join("2.json")).unwrap();
        fs::remove_file(watched.join("2.json")).unwrap();
        fs::create_dir(watched.join("sub")).unwrap();
        fs::write(watched.join("sub").join("3.json"), b"{}").unwrap();
        fs::rename(watched.join("sub"), watched.join("sub2")).unwrap();
        fs::remove_dir_all(watched.join("sub2")).unwrap();
        let moved = root.path().join("moved");
        fs::rename(&watched, &moved).unwrap();
        fs::rename(&moved, &watched).unwrap();
        // Deleted and made again while the old read is still queued: the name is free at once.
        fs::remove_dir(&watched).unwrap();
        fs::create_dir(&watched).unwrap();

        wait_until(&mut watch, |s| s.rescanned(&watched));
        assert!(matches!(watch.state(&watched), Some(State::Missing(_))));
        assert_eq!(watch.retry_missing(), [Changed::Rescan(watched.clone())]);
    }

    /// The one limit, which any open handle has: a folder above the watched one cannot be renamed while it is
    /// watched (`docs/research.md`, "Folder watch with `ReadDirectoryChangesW`"). Deleting it works, and the watch
    /// reports it.
    #[test]
    fn a_watch_blocks_renaming_a_folder_above_but_never_deleting_it() {
        let root = TempDir::new("dir-above");
        let parent = root.path().join("parent");
        let watched = parent.join("watched");
        fs::create_dir_all(&watched).unwrap();
        let mut watch = FolderWatch::new();
        watch.add(&watched, false).unwrap();

        let rename = fs::rename(&parent, root.path().join("parent2"));
        assert_eq!(rename.map_err(|e| e.raw_os_error()), Err(Some(5)));
        fs::remove_dir_all(&parent).unwrap();
        wait_until(&mut watch, |s| s.rescanned(&watched));
        assert!(matches!(watch.state(&watched), Some(State::Missing(_))));

        // A plain `RemoveDirectoryW` of the watched folder, then of its parent, while the read is still queued.
        fs::create_dir_all(&watched).unwrap();
        assert_eq!(watch.retry_missing(), [Changed::Rescan(watched.clone())]);
        fs::remove_dir(&watched).unwrap();
        fs::remove_dir(&parent).unwrap();
    }

    #[test]
    fn parse_reads_every_record_and_refuses_a_broken_one() {
        fn record(next: u32, name: &str) -> Vec<u8> {
            let units: Vec<u16> = name.encode_utf16().collect();
            let mut out = Vec::new();
            out.extend(next.to_le_bytes());
            out.extend(1u32.to_le_bytes());
            out.extend(((units.len() * 2) as u32).to_le_bytes());
            out.extend(units.iter().flat_map(|u| u.to_le_bytes()));
            while out.len() % 4 != 0 {
                out.push(0);
            }
            out
        }
        let first = record(0, "a.json");
        let mut two = record(first.len() as u32, "a.json");
        two.extend(record(0, "sub\\b.json"));
        let names = parse(&two).unwrap();
        assert_eq!(
            names.into_iter().collect::<Vec<_>>(),
            [PathBuf::from("a.json"), PathBuf::from("sub\\b.json")]
        );
        let mut same = record(first.len() as u32, "a.json");
        same.extend(record(0, "a.json"));
        assert_eq!(parse(&same).unwrap().len(), 1);

        assert_eq!(parse(&[]), None);
        assert_eq!(parse(&first[..first.len() - 4]), None);
        let mut past_end = record(64, "a.json");
        past_end.truncate(first.len());
        assert_eq!(parse(&past_end), None);
        let odd = record(2, "a.json");
        assert_eq!(parse(&odd), None);
    }

    const LEAK_CHILD: &str = "WAKESHOT_TEST_FOLDER_WATCH_LEAK";
    const CYCLES: usize = 1000;

    #[test]
    fn a_thousand_add_and_remove_cycles_leak_no_handle() {
        if let Ok(dir) = std::env::var(LEAK_CHILD) {
            leak_child(Path::new(&dir));
            return;
        }
        // In a child process of its own, so the other tests' handles do not move the count.
        let dir = TempDir::new("dir-leak");
        let out = Command::new(std::env::current_exe().unwrap())
            .args([
                "watch::dir::tests::a_thousand_add_and_remove_cycles_leak_no_handle",
                "--exact",
                "--test-threads=1",
                "--nocapture",
            ])
            .env(LEAK_CHILD, dir.path())
            .stdin(Stdio::null())
            .output()
            .unwrap();
        let text = String::from_utf8_lossy(&out.stdout);
        println!("{text}");
        assert!(
            out.status.success(),
            "child failed: {text}{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(
            text.contains("handles before"),
            "the child never ran the cycles"
        );
    }

    fn leak_child(dir: &Path) {
        let mut watch = FolderWatch::new();
        let missing = dir.join("missing");
        let mut cycle = |n: usize| {
            assert_eq!(
                watch.add(dir, n.is_multiple_of(2)).unwrap(),
                State::Watching
            );
            assert!(matches!(
                watch.add(&missing, false).unwrap(),
                State::Missing(_)
            ));
            if n.is_multiple_of(10) {
                // A completed read that nobody took, and every 20th cycle one taken and queued again.
                fs::write(dir.join("x.json"), n.to_string()).unwrap();
                if n.is_multiple_of(20) {
                    let woke = watch.wait(Some(WITHIN), &[]).unwrap();
                    assert!(matches!(woke, Woke::Changed(_)), "{woke:?}");
                }
            }
            assert!(watch.remove(dir));
            assert!(watch.remove(&missing));
        };
        for n in 0..10 {
            cycle(n);
        }
        let before = handle_count();
        for n in 0..CYCLES {
            cycle(n);
        }
        let after = handle_count();
        println!("handles before {before}, after {after} ({CYCLES} cycles)");
        assert_eq!(before, after);
    }

    fn handle_count() -> u32 {
        let mut count = 0u32;
        // SAFETY: the pseudo handle of this process needs no close, and `count` outlives the call.
        unsafe { GetProcessHandleCount(GetCurrentProcess(), &mut count) }.unwrap();
        count
    }
}
