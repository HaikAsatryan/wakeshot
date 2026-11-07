//! The data folder, atomic file writes, named mutexes, the Run and RunOnce values, and the Run value's
//! `StartupApproved` switch value (architecture sections 4 and 6).

use std::ffi::{OsStr, OsString};
use std::hash::{BuildHasher, RandomState};
use std::mem::offset_of;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use windows::Win32::Foundation::{
    DUPLICATE_SAME_ACCESS, DuplicateHandle, ERROR_ACCESS_DENIED, ERROR_FILE_EXISTS,
    ERROR_FILE_NOT_FOUND, ERROR_INVALID_DATA, ERROR_INVALID_PARAMETER, ERROR_MORE_DATA,
    ERROR_PATH_NOT_FOUND, ERROR_SUCCESS, ERROR_WRITE_FAULT, GENERIC_READ, GENERIC_WRITE,
    GetLastError, HANDLE, WAIT_ABANDONED, WAIT_OBJECT_0, WAIT_TIMEOUT, WIN32_ERROR,
};
use windows::Win32::Storage::FileSystem::{
    CREATE_NEW, CreateFileW, DELETE, DeleteFileW, FILE_ATTRIBUTE_NORMAL, FILE_RENAME_INFO,
    FILE_RENAME_INFO_0, FILE_SHARE_DELETE, FILE_SHARE_NONE, FILE_SHARE_READ, FILE_SHARE_WRITE,
    FileRenameInfoEx, FlushFileBuffers, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    MoveFileExW, OPEN_EXISTING, ReadFile, SetFileInformationByHandle, WriteFile,
};
use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_SET_VALUE, REG_OPTION_NON_VOLATILE, REG_SZ, RRF_RT_REG_SZ,
    RegCreateKeyExW, RegDeleteKeyValueW, RegGetValueW, RegSetValueExW,
};
use windows::Win32::System::Threading::{
    CreateMutexExW, GetCurrentProcess, MUTEX_MODIFY_STATE, ReleaseMutex,
    SYNCHRONIZATION_SYNCHRONIZE, WaitForSingleObject,
};
use windows::Win32::System::WindowsProgramming::{
    FILE_RENAME_FLAG_POSIX_SEMANTICS, FILE_RENAME_FLAG_REPLACE_IF_EXISTS,
};
use windows::Win32::UI::Shell::{FOLDERID_LocalAppData, KF_FLAG_DEFAULT, SHGetKnownFolderPath};
use windows::core::{Owned, PCWSTR};

use crate::WinError;

#[cfg(test)]
pub(crate) use scratch::{ScratchKey, sweep_scratch_keys};

/// One file in the data folder, by its fixed name (architecture section 6).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DataFile {
    Config,
    State,
    Journal,
    JournalCorrupt,
    JournalCorruptSeen,
    RestoreRequest,
    KeepAwake,
}

/// The data folder: `%LocalAppData%\Wakeshot`, its `dry-run` subfolder for `--dry-run`, or any folder for tests.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DataDir {
    root: PathBuf,
}

/// Read only in debug builds, so a release exe always uses the real folder.
const DATA_DIR_ENV: &str = "WAKESHOT_DATA_DIR";

impl DataDir {
    /// `%LocalAppData%\Wakeshot`; in debug builds `WAKESHOT_DATA_DIR` wins, so tests never touch the real folder.
    pub fn real() -> Result<Self, WinError> {
        let over = if cfg!(debug_assertions) {
            std::env::var_os(DATA_DIR_ENV)
        } else {
            None
        };
        real_root(over).map(Self::at)
    }

    /// `%LocalAppData%\Wakeshot\dry-run`.
    pub fn dry_run() -> Result<Self, WinError> {
        Self::real().map(|real| Self::at(real.root.join("dry-run")))
    }

    pub fn at(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn file(&self, file: DataFile) -> PathBuf {
        match file {
            DataFile::Config => self.root.join("config.json"),
            DataFile::State => self.root.join("state.json"),
            DataFile::Journal => self.root.join("journal.json"),
            DataFile::JournalCorrupt => self.root.join("journal.json.corrupt"),
            DataFile::JournalCorruptSeen => self.root.join("journal.json.corrupt.seen"),
            DataFile::RestoreRequest => self.root.join("restore-request.json"),
            DataFile::KeepAwake => self.holds().join("keep-awake.json"),
        }
    }

    pub fn holds(&self) -> PathBuf {
        self.root.join("holds")
    }

    pub fn logs(&self) -> PathBuf {
        self.root.join("logs")
    }
}

/// `over` is the `WAKESHOT_DATA_DIR` value. An empty one counts as unset: it would name the working folder.
fn real_root(over: Option<OsString>) -> Result<PathBuf, WinError> {
    match over {
        Some(root) if !root.is_empty() => Ok(PathBuf::from(root)),
        _ => Ok(local_app_data()?.join("Wakeshot")),
    }
}

/// `%LocalAppData%` from its known folder id (`FOLDERID_LocalAppData`), never from the environment, which the
/// process that started this one sets. The hook command finds an agent's install folder under it.
pub fn local_app_data() -> Result<PathBuf, WinError> {
    // SAFETY: the folder id is a static; no token means the calling user.
    let path = unsafe { SHGetKnownFolderPath(&FOLDERID_LocalAppData, KF_FLAG_DEFAULT, None) }
        .map_err(|e| core_error("SHGetKnownFolderPath", &e))?;
    // SAFETY: on success the shell gives a NUL-terminated string it allocated.
    let copy = unsafe { OsString::from_wide(path.as_wide()) };
    // SAFETY: the shell's string, freed once, after its one copy.
    unsafe { CoTaskMemFree(Some(path.0 as *const _)) };
    Ok(PathBuf::from(copy))
}

/// Makes each temp name unique inside one process; the pid in the name makes it unique across processes.
static TEMP_COUNT: AtomicU64 = AtomicU64::new(0);

/// How many temp names one write tries, each new, when the name it picked exists already.
const TEMP_TRIES: usize = 3;

/// Writes `bytes` to a temp file beside `path` and moves it over `path`, write-through. The temp name is
/// `<name>.<pid>.<n>.<16 hex digits>.tmp` ([`temp_target`]), never ending in `.json`, so the watch thread never reads
/// it as a hold. A missing folder is made first. On failure the temp file is removed and `path` is as it was.
///
/// The temp file is made with `CREATE_NEW`, so a name that exists is never opened: a hard link another process of the
/// user planted there cannot steer the write into its target (`docs/agent-hooks.md`, "The hook command"). The next try
/// takes a new name; after `TEMP_TRIES` the error is `ERROR_FILE_EXISTS`. Nothing is deleted to free a name, so a
/// temp file a crash left stays. A junction over the folder still redirects the write: only the hooks' Medium rule
/// closes that.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), WinError> {
    let target = wide("MoveFileExW", path.as_os_str())?;
    write_temp(path, bytes)?.replace(&target)
}

/// The target's file name when `name` is a temp file name of [`write_atomic`], `<target>.<pid>.<n>.<16 hex>.tmp`;
/// `None` for any other name, so a cleanup deletes only those (`docs/agent-hooks.md`, "The hook command" step 5).
pub fn temp_target(name: &str) -> Option<&str> {
    let mut parts = name.strip_suffix(".tmp")?.rsplitn(4, '.');
    let random = parts.next()?;
    let n = parts.next()?;
    let pid = parts.next()?;
    let target = parts.next().filter(|t| !t.is_empty())?;
    let digits = |s: &str| (1..=20).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_digit());
    let hex = random.len() == 16
        && random
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
    (hex && digits(n) && digits(pid)).then_some(target)
}

/// A fresh temp name for the file `name`. The random part keeps another process from guessing the next name to block
/// it; `CREATE_NEW` already makes a guessed name harmless. `RandomState` takes its keys from the system's random
/// source, so no crate or `windows` feature is needed (change it when a stronger source is wanted:
/// `BCryptGenRandom`, feature `Win32_Security_Cryptography`).
fn temp_name(name: &OsStr) -> OsString {
    let n = TEMP_COUNT.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    let random = RandomState::new().hash_one((pid, n));
    let mut temp = name.to_os_string();
    temp.push(format!(".{pid}.{n}.{random:016x}.tmp"));
    temp
}

/// A flushed temp file beside its target, not yet moved over it.
struct TempFile {
    name: Vec<u16>,
}

impl TempFile {
    fn replace(self, target: &[u16]) -> Result<(), WinError> {
        // SAFETY: both names are NUL-terminated and live for the call.
        let moved = unsafe {
            MoveFileExW(
                PCWSTR(self.name.as_ptr()),
                PCWSTR(target.as_ptr()),
                MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
            )
        };
        let Err(e) = moved else {
            return Ok(());
        };
        let err = core_error("MoveFileExW", &e);
        // `MoveFileExW` cannot replace a file that a reader has open, even with delete sharing (tested 2026-11-03).
        // A rename with POSIX semantics can, so a reader never makes a write fail. Rust's `std::fs::rename` does the
        // same, and like it the first error is the one reported.
        if err.code == ERROR_ACCESS_DENIED.0 && rename_posix(&self.name, target).is_ok() {
            return Ok(());
        }
        self.discard();
        Err(err)
    }

    /// A failed removal is not reported: the first error is the one the caller needs.
    fn discard(&self) {
        let _ = delete_file(&self.name);
    }
}

/// Renames `temp` over `target` by handle, with POSIX semantics: the name points at the new file at once, and a
/// handle open on the old file keeps reading the old file.
fn rename_posix(temp: &[u16], target: &[u16]) -> Result<(), WinError> {
    // SAFETY: `temp` is NUL-terminated and lives for the call.
    let file = unsafe {
        CreateFileW(
            PCWSTR(temp.as_ptr()),
            DELETE.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            None,
        )
    }
    .map_err(|e| core_error("CreateFileW", &e))?;
    // SAFETY: the handle is new and owned only here.
    let file = unsafe { Owned::new(file) };
    let too_long = || win32("SetFileInformationByHandle", ERROR_INVALID_PARAMETER);
    // `target` ends in its NUL, which the length leaves out but the buffer keeps.
    let name_bytes = (target.len() - 1) * size_of::<u16>();
    let size = offset_of!(FILE_RENAME_INFO, FileName) + name_bytes + size_of::<u16>();
    let name_len = u32::try_from(name_bytes).map_err(|_| too_long())?;
    let size_len = u32::try_from(size).map_err(|_| too_long())?;
    // u64 units keep the struct's handle field aligned.
    let mut buf = vec![0u64; size.div_ceil(size_of::<u64>())];
    let info = buf.as_mut_ptr().cast::<FILE_RENAME_INFO>();
    // SAFETY: `buf` is zeroed, aligned and at least `size` bytes long, so the header and the whole name fit in it.
    unsafe {
        (&raw mut (*info).Anonymous).write(FILE_RENAME_INFO_0 {
            Flags: FILE_RENAME_FLAG_REPLACE_IF_EXISTS | FILE_RENAME_FLAG_POSIX_SEMANTICS,
        });
        (&raw mut (*info).FileNameLength).write(name_len);
        target
            .as_ptr()
            .copy_to_nonoverlapping((&raw mut (*info).FileName).cast::<u16>(), target.len());
    }
    // SAFETY: an open handle with `DELETE`; `info` points at `size_len` filled bytes.
    unsafe { SetFileInformationByHandle(*file, FileRenameInfoEx, info.cast(), size_len) }
        .map_err(|e| core_error("SetFileInformationByHandle", &e))
}

/// Step one of `write_atomic`. A stop after it leaves `path` as it was.
fn write_temp(path: &Path, bytes: &[u8]) -> Result<TempFile, WinError> {
    let name = path
        .file_name()
        .ok_or_else(|| win32("CreateFileW", ERROR_INVALID_PARAMETER))?;
    write_temp_named(path, bytes, || path.with_file_name(temp_name(name)))
}

/// [`write_temp`] with the temp names from `next`, so a test can plant a file at the names it will try.
fn write_temp_named(
    path: &Path,
    bytes: &[u8],
    mut next: impl FnMut() -> PathBuf,
) -> Result<TempFile, WinError> {
    let mut taken = win32("CreateFileW", ERROR_FILE_EXISTS);
    for _ in 0..TEMP_TRIES {
        let temp = TempFile {
            name: wide("CreateFileW", next().as_os_str())?,
        };
        let created = match create_temp(&temp.name) {
            // The first write of a fresh install, or the CLI before any tray ran, finds no folder yet.
            Err(e) if e.code == ERROR_PATH_NOT_FOUND.0 => {
                if let Some(folder) = path.parent() {
                    std::fs::create_dir_all(folder)
                        .map_err(|e| io_error("CreateDirectoryW", &e))?;
                }
                create_temp(&temp.name)
            }
            other => other,
        };
        let file = match created {
            Ok(file) => file,
            Err(e) if e.code == ERROR_FILE_EXISTS.0 => {
                taken = e;
                continue;
            }
            Err(e) => return Err(e),
        };
        let written = write_all(*file, bytes).and_then(|()| flush(*file));
        // Closed before the move: the temp file is open with no sharing, so the move would fail on it.
        drop(file);
        return match written {
            Ok(()) => Ok(temp),
            Err(e) => {
                temp.discard();
                Err(e)
            }
        };
    }
    Err(taken)
}

/// `CREATE_NEW`: see [`write_atomic`]. A temp file left by a stopped process is never overwritten, nor the file a
/// link at that name points to.
fn create_temp(name: &[u16]) -> Result<Owned<HANDLE>, WinError> {
    // SAFETY: `name` is NUL-terminated and lives for the call.
    let file = unsafe {
        CreateFileW(
            PCWSTR(name.as_ptr()),
            GENERIC_WRITE.0,
            FILE_SHARE_NONE,
            None,
            CREATE_NEW,
            FILE_ATTRIBUTE_NORMAL,
            None,
        )
    }
    .map_err(|e| core_error("CreateFileW", &e))?;
    // SAFETY: the handle is new and owned only here.
    Ok(unsafe { Owned::new(file) })
}

fn write_all(file: HANDLE, mut bytes: &[u8]) -> Result<(), WinError> {
    while !bytes.is_empty() {
        let chunk = &bytes[..bytes.len().min(u32::MAX as usize)];
        let mut written = 0u32;
        // SAFETY: a synchronous handle and a live buffer and count.
        unsafe { WriteFile(file, Some(chunk), Some(&raw mut written), None) }
            .map_err(|e| core_error("WriteFile", &e))?;
        if written == 0 {
            // A disk file never writes nothing without an error; stop rather than loop forever.
            return Err(win32("WriteFile", ERROR_WRITE_FAULT));
        }
        bytes = &bytes[written as usize..];
    }
    Ok(())
}

fn flush(file: HANDLE) -> Result<(), WinError> {
    // SAFETY: an open handle with write access.
    unsafe { FlushFileBuffers(file) }.map_err(|e| core_error("FlushFileBuffers", &e))
}

/// The whole file, or `None` when it does not exist.
pub fn read_whole(path: &Path) -> Result<Option<Vec<u8>>, WinError> {
    match open_shared(path)? {
        Some(file) => read_to_end(*file).map(Some),
        None => Ok(None),
    }
}

/// Read, write and delete sharing, so a writer's replace or a delete never fails because a reader has the file open.
/// Only a missing file or folder is `None`; a folder path, a denied open or a bad name is an error.
fn open_shared(path: &Path) -> Result<Option<Owned<HANDLE>>, WinError> {
    let name = wide("CreateFileW", path.as_os_str())?;
    // SAFETY: `name` is NUL-terminated and lives for the call.
    let opened = unsafe {
        CreateFileW(
            PCWSTR(name.as_ptr()),
            GENERIC_READ.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            None,
        )
    };
    match opened {
        // SAFETY: the handle is new and owned only here.
        Ok(file) => Ok(Some(unsafe { Owned::new(file) })),
        Err(e) => missing_is_none(core_error("CreateFileW", &e)),
    }
}

/// The handle keeps the file it opened, so a replace during the read never mixes old and new bytes.
fn read_to_end(file: HANDLE) -> Result<Vec<u8>, WinError> {
    let mut out = Vec::new();
    let mut buf = [0u8; 16 * 1024];
    loop {
        let mut read = 0u32;
        // SAFETY: a synchronous handle and a live buffer and count.
        unsafe { ReadFile(file, Some(&mut buf[..]), Some(&raw mut read), None) }
            .map_err(|e| core_error("ReadFile", &e))?;
        if read == 0 {
            return Ok(out);
        }
        out.extend_from_slice(&buf[..read as usize]);
    }
}

/// `true`: the file existed and is gone now.
pub fn remove(path: &Path) -> Result<bool, WinError> {
    match delete_file(&wide("DeleteFileW", path.as_os_str())?) {
        Ok(()) => Ok(true),
        Err(e) => missing_is_none::<()>(e).map(|_| false),
    }
}

fn delete_file(name: &[u16]) -> Result<(), WinError> {
    // SAFETY: `name` is NUL-terminated and lives for the call.
    unsafe { DeleteFileW(PCWSTR(name.as_ptr())) }.map_err(|e| core_error("DeleteFileW", &e))
}

/// The three mutex names (architecture section 4, "Processes"). A dry run uses its own names, so it runs beside the
/// real tray; tests use their own, so they never block a real tray.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Names {
    pub tray: String,
    pub settings: String,
    pub journal: String,
}

/// Makes `Names::test` unique per call, so two tests with one tag still get their own mutexes.
static TEST_NAMES: AtomicU64 = AtomicU64::new(0);

impl Names {
    pub fn real() -> Self {
        Self {
            tray: r"Local\Wakeshot.Tray".to_owned(),
            settings: r"Global\Wakeshot.Settings".to_owned(),
            journal: r"Local\Wakeshot.Journal".to_owned(),
        }
    }

    pub fn dry_run() -> Self {
        Self {
            tray: r"Local\Wakeshot.Tray.DryRun".to_owned(),
            settings: r"Global\Wakeshot.Settings.DryRun".to_owned(),
            journal: r"Local\Wakeshot.Journal.DryRun".to_owned(),
        }
    }

    /// Names no real or dry-run tray uses, with `tag` in them. The settings name stays in `Global\`, so a test takes
    /// the same path as the real one.
    pub fn test(tag: &str) -> Self {
        // A backslash would end the namespace part of the name; 64 characters keep it far below `MAX_PATH`.
        let tag: String = tag
            .chars()
            .map(|c| if c == '\\' { '_' } else { c })
            .take(64)
            .collect();
        let n = TEST_NAMES.fetch_add(1, Ordering::Relaxed);
        let id = format!("Wakeshot.Test.{tag}.{}.{n}", std::process::id());
        Self {
            tray: format!(r"Local\{id}.Tray"),
            settings: format!(r"Global\{id}.Settings"),
            journal: format!(r"Local\{id}.Journal"),
        }
    }
}

/// A named mutex, opened or created.
///
/// A thread that holds it takes it again at once: Windows counts a mutex per owning thread.
#[derive(Debug)]
pub struct NamedMutex {
    handle: Owned<HANDLE>,
}

// SAFETY: any thread may wait on or duplicate a mutex handle. Ownership is per thread, and it lives in `HeldMutex`,
// which stays on the thread that took it.
unsafe impl Send for NamedMutex {}
// SAFETY: as above.
unsafe impl Sync for NamedMutex {}

impl NamedMutex {
    /// Creates the mutex, or opens it when it exists. A mutex this user may not open is an error
    /// (`ERROR_ACCESS_DENIED`).
    pub fn open(name: &str) -> Result<Self, WinError> {
        let name = wide("CreateMutexExW", OsStr::new(name))?;
        // Only the rights a wait and a release need, so a mutex whose security grants just those still opens.
        let access = (SYNCHRONIZATION_SYNCHRONIZE | MUTEX_MODIFY_STATE).0;
        // SAFETY: `name` is NUL-terminated and lives for the call; no security attributes means the default ones.
        let handle = unsafe { CreateMutexExW(None, PCWSTR(name.as_ptr()), 0, access) }
            .map_err(|e| core_error("CreateMutexExW", &e))?;
        Ok(Self {
            // SAFETY: the handle is new and owned only here.
            handle: unsafe { Owned::new(handle) },
        })
    }

    /// `None`: not taken within `timeout_ms`.
    pub fn take(&self, timeout_ms: u32) -> Result<Option<HeldMutex>, WinError> {
        // The guard gets its own handle, so it can outlive this `NamedMutex`.
        let mut own = HANDLE::default();
        // SAFETY: the source handle is open; the copy is written to a live local.
        unsafe {
            DuplicateHandle(
                GetCurrentProcess(),
                *self.handle,
                GetCurrentProcess(),
                &raw mut own,
                0,
                false,
                DUPLICATE_SAME_ACCESS,
            )
        }
        .map_err(|e| core_error("DuplicateHandle", &e))?;
        // SAFETY: the copy is new and owned only here.
        let own = unsafe { Owned::new(own) };
        // SAFETY: an open mutex handle with `SYNCHRONIZE`.
        let abandoned = match unsafe { WaitForSingleObject(*own, timeout_ms) } {
            WAIT_OBJECT_0 => false,
            WAIT_ABANDONED => true,
            WAIT_TIMEOUT => return Ok(None),
            // SAFETY: read right after the failed wait, on the same thread.
            _ => return Err(win32("WaitForSingleObject", unsafe { GetLastError() })),
        };
        Ok(Some(HeldMutex {
            handle: own,
            abandoned,
        }))
    }
}

/// A taken mutex. It owns its handle and releases on drop.
///
/// Not `Send`: Windows lets only the thread that took a mutex release it.
#[derive(Debug)]
pub struct HeldMutex {
    handle: Owned<HANDLE>,
    abandoned: bool,
}

impl HeldMutex {
    /// The previous owner ended while holding it (`WAIT_ABANDONED`); it counts as taken (architecture section 4).
    pub fn abandoned(&self) -> bool {
        self.abandoned
    }
}

impl Drop for HeldMutex {
    fn drop(&mut self) {
        // A failed release has no fallback: the handle closes next, and the mutex counts as abandoned once this thread
        // ends.
        // SAFETY: an open mutex handle this thread owns.
        let _ = unsafe { ReleaseMutex(*self.handle) };
    }
}

/// `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` or `RunOnce`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StartKey {
    Run,
    RunOnce,
}

/// Under an `AutoStart` base: where Windows keeps the Settings > Apps > Startup switch, one value per Run value, with
/// the same name (research "Start at login switch check").
const APPROVED_RUN: &str = r"Explorer\StartupApproved\Run";

/// The Run and RunOnce values under one base key, the real one or a test key, and the Run value's switch value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AutoStart {
    base: String,
}

impl AutoStart {
    pub fn real() -> Self {
        Self::at(r"Software\Microsoft\Windows\CurrentVersion")
    }

    /// `base` is a key under `HKCU`; `Run` and `RunOnce` are its subkeys.
    pub fn at(base: &str) -> Self {
        Self {
            base: base.to_owned(),
        }
    }

    /// A `REG_SZ` value; the key is made when it is missing.
    pub fn set(&self, key: StartKey, name: &str, command: &str) -> Result<(), WinError> {
        let subkey = self.subkey("RegCreateKeyExW", key)?;
        let name = wide("RegSetValueExW", OsStr::new(name))?;
        let data: Vec<u8> = wide("RegSetValueExW", OsStr::new(command))?
            .iter()
            .flat_map(|unit| unit.to_le_bytes())
            .collect();
        let mut opened = HKEY::default();
        // SAFETY: `subkey` is NUL-terminated and lives for the call; the key is written to a live local.
        let created = unsafe {
            RegCreateKeyExW(
                HKEY_CURRENT_USER,
                PCWSTR(subkey.as_ptr()),
                None,
                PCWSTR::null(),
                REG_OPTION_NON_VOLATILE,
                KEY_SET_VALUE,
                None,
                &raw mut opened,
                None,
            )
        };
        reg_ok("RegCreateKeyExW", created)?;
        // SAFETY: the key is new and owned only here.
        let opened = unsafe { Owned::new(opened) };
        // SAFETY: an open key with `KEY_SET_VALUE`; `name` is NUL-terminated, `data` holds the terminator too.
        let set = unsafe {
            RegSetValueExW(
                *opened,
                PCWSTR(name.as_ptr()),
                None,
                REG_SZ,
                Some(data.as_slice()),
            )
        };
        reg_ok("RegSetValueExW", set)
    }

    /// `None`: no such value, or no such key. A value of another type than `REG_SZ` is an error.
    pub fn get(&self, key: StartKey, name: &str) -> Result<Option<String>, WinError> {
        let subkey = self.subkey("RegGetValueW", key)?;
        let name = wide("RegGetValueW", OsStr::new(name))?;
        // The value can grow between the size query and the read; a few tries cover that.
        for _ in 0..4 {
            let mut size = 0u32;
            // SAFETY: names are NUL-terminated and live for the call; a size query writes only `size`.
            let query = unsafe {
                RegGetValueW(
                    HKEY_CURRENT_USER,
                    PCWSTR(subkey.as_ptr()),
                    PCWSTR(name.as_ptr()),
                    RRF_RT_REG_SZ,
                    None,
                    None,
                    Some(&raw mut size),
                )
            };
            if query != ERROR_SUCCESS {
                return missing_is_none(win32("RegGetValueW", query));
            }
            let mut buf = vec![0u16; (size as usize).div_ceil(2)];
            let mut got = u32::try_from(buf.len() * 2)
                .map_err(|_| win32("RegGetValueW", ERROR_INVALID_DATA))?;
            // SAFETY: `buf` holds `got` bytes and lives for the call.
            let read = unsafe {
                RegGetValueW(
                    HKEY_CURRENT_USER,
                    PCWSTR(subkey.as_ptr()),
                    PCWSTR(name.as_ptr()),
                    RRF_RT_REG_SZ,
                    None,
                    Some(buf.as_mut_ptr().cast()),
                    Some(&raw mut got),
                )
            };
            if read == ERROR_MORE_DATA {
                continue;
            }
            if read != ERROR_SUCCESS {
                return missing_is_none(win32("RegGetValueW", read));
            }
            buf.truncate(got as usize / 2);
            while buf.last() == Some(&0) {
                buf.pop();
            }
            return String::from_utf16(&buf)
                .map(Some)
                .map_err(|_| win32("RegGetValueW", ERROR_INVALID_DATA));
        }
        Err(win32("RegGetValueW", ERROR_MORE_DATA))
    }

    /// `true`: the value existed and is gone now.
    pub fn remove(&self, key: StartKey, name: &str) -> Result<bool, WinError> {
        Self::delete(&self.subkey("RegDeleteKeyValueW", key)?, name)
    }

    /// The switch value of the Run value `name`, which Windows writes once the user has used Settings > Apps > Startup.
    /// Only Windows writes it, so this only deletes it. `true`: the value existed and is gone now; a missing value or
    /// key is not an error, as in `remove`.
    pub fn remove_approval(&self, name: &str) -> Result<bool, WinError> {
        let subkey = wide("RegDeleteKeyValueW", OsStr::new(&self.approval_key()))?;
        Self::delete(&subkey, name)
    }

    fn approval_key(&self) -> String {
        format!(r"{}\{APPROVED_RUN}", self.base)
    }

    fn delete(subkey: &[u16], name: &str) -> Result<bool, WinError> {
        let name = wide("RegDeleteKeyValueW", OsStr::new(name))?;
        // SAFETY: names are NUL-terminated and live for the call.
        let deleted = unsafe {
            RegDeleteKeyValueW(
                HKEY_CURRENT_USER,
                PCWSTR(subkey.as_ptr()),
                PCWSTR(name.as_ptr()),
            )
        };
        if deleted == ERROR_SUCCESS {
            return Ok(true);
        }
        missing_is_none::<()>(win32("RegDeleteKeyValueW", deleted)).map(|_| false)
    }

    fn subkey(&self, call: &'static str, key: StartKey) -> Result<Vec<u16>, WinError> {
        let leaf = match key {
            StartKey::Run => "Run",
            StartKey::RunOnce => "RunOnce",
        };
        wide(call, OsStr::new(&format!(r"{}\{leaf}", self.base)))
    }
}

/// The switch value as Windows writes it, for tests only: Wakeshot never writes it.
#[cfg(test)]
impl AutoStart {
    /// A `REG_BINARY` value; the key is made when it is missing.
    pub(crate) fn set_approval(&self, name: &str, data: &[u8]) -> Result<(), WinError> {
        use windows::Win32::System::Registry::REG_BINARY;
        let subkey = wide("RegCreateKeyExW", OsStr::new(&self.approval_key()))?;
        let name = wide("RegSetValueExW", OsStr::new(name))?;
        let mut opened = HKEY::default();
        // SAFETY: `subkey` is NUL-terminated and lives for the call; the key is written to a live local.
        let created = unsafe {
            RegCreateKeyExW(
                HKEY_CURRENT_USER,
                PCWSTR(subkey.as_ptr()),
                None,
                PCWSTR::null(),
                REG_OPTION_NON_VOLATILE,
                KEY_SET_VALUE,
                None,
                &raw mut opened,
                None,
            )
        };
        reg_ok("RegCreateKeyExW", created)?;
        // SAFETY: the key is new and owned only here.
        let opened = unsafe { Owned::new(opened) };
        // SAFETY: an open key with `KEY_SET_VALUE`; `name` is NUL-terminated and `data` lives for the call.
        let set =
            unsafe { RegSetValueExW(*opened, PCWSTR(name.as_ptr()), None, REG_BINARY, Some(data)) };
        reg_ok("RegSetValueExW", set)
    }
}

/// The switch value read back, for tests and for the dev tools' guard (`RealPlaces`).
#[cfg(any(test, feature = "test-support"))]
impl AutoStart {
    /// The value's bytes; `None`: no such value, or no such key.
    pub(crate) fn approval(&self, name: &str) -> Result<Option<Vec<u8>>, WinError> {
        use windows::Win32::System::Registry::RRF_RT_REG_BINARY;
        let subkey = wide("RegGetValueW", OsStr::new(&self.approval_key()))?;
        let name = wide("RegGetValueW", OsStr::new(name))?;
        // The value is 12 bytes as Windows writes it; 64 leaves room for any test value.
        let mut buf = vec![0u8; 64];
        let mut got = buf.len() as u32;
        // SAFETY: names are NUL-terminated and live for the call; `buf` holds `got` bytes.
        let read = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                PCWSTR(subkey.as_ptr()),
                PCWSTR(name.as_ptr()),
                RRF_RT_REG_BINARY,
                None,
                Some(buf.as_mut_ptr().cast()),
                Some(&raw mut got),
            )
        };
        if read != ERROR_SUCCESS {
            return missing_is_none(win32("RegGetValueW", read));
        }
        buf.truncate(got as usize);
        Ok(Some(buf))
    }
}

/// Velopack's pack id: the install folder's and the uninstall key's name (`packaging/pack.ps1`).
#[cfg(any(test, feature = "test-support"))]
const PACK_ID: &str = "HaikAsatryan.Wakeshot";

/// One mark of a real Wakeshot on this PC, which a dev tool that changes power settings would fight with.
#[cfg(any(test, feature = "test-support"))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RealSign {
    /// The tray's lock name exists: a real tray runs in this session.
    TrayRuns { lock: String },
    /// Velopack's install folder.
    InstallFolder { path: PathBuf },
    /// Velopack's uninstall key, under `HKCU`.
    UninstallKey { key: String },
    /// The Run value, and no switch value that says off: Windows starts it at the next sign-in.
    StartsAtLogin { command: String },
    /// `journal.json` in the real data folder: a real Wakeshot may own power values now.
    Journal { path: PathBuf },
    /// The RunOnce value: restore mode runs at the next sign-in.
    RunOnce { command: String },
    /// A read failed, so whether a real Wakeshot is here is unknown.
    Unread { what: String, error: WinError },
}

/// Where a real Wakeshot leaves its marks: the real places, or scratch ones in tests. Only for dev tools, so no shipped
/// exe carries it: `examples/power_check.rs` reads it before it changes anything, and `packaging/e2e.ps1` reads the
/// same places in PowerShell.
#[cfg(any(test, feature = "test-support"))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RealPlaces {
    pub tray_lock: String,
    pub install_dir: PathBuf,
    /// Under `HKCU`.
    pub uninstall_key: String,
    pub data_dir: DataDir,
    /// The Run value `Wakeshot`, its switch value and the RunOnce value `WakeshotRestore`.
    pub auto: AutoStart,
}

#[cfg(any(test, feature = "test-support"))]
impl RealPlaces {
    /// The installed Wakeshot's places. Never `WAKESHOT_DATA_DIR`: it moves only a debug build's own folder, and a
    /// dev shell may have it set.
    pub fn real() -> Result<Self, WinError> {
        let local = local_app_data()?;
        Ok(Self {
            tray_lock: Names::real().tray,
            install_dir: local.join(PACK_ID),
            uninstall_key: format!(
                r"Software\Microsoft\Windows\CurrentVersion\Uninstall\{PACK_ID}"
            ),
            data_dir: DataDir::at(local.join("Wakeshot")),
            auto: AutoStart::real(),
        })
    }

    /// Every mark found, in the order of `RealSign`; empty when there is none. It only reads: the lock is opened and
    /// closed, never created or taken.
    pub fn signs(&self) -> Vec<RealSign> {
        use crate::hooks::RUN_NAME;
        use crate::power::journal::RUN_ONCE_NAME;

        let mut signs = Vec::new();
        let unread = |what: String, error: WinError| RealSign::Unread { what, error };
        match mutex_exists(&self.tray_lock) {
            Ok(true) => signs.push(RealSign::TrayRuns {
                lock: self.tray_lock.clone(),
            }),
            Ok(false) => {}
            Err(e) => signs.push(unread(format!("the tray lock {}", self.tray_lock), e)),
        }
        match exists(&self.install_dir) {
            Ok(true) => signs.push(RealSign::InstallFolder {
                path: self.install_dir.clone(),
            }),
            Ok(false) => {}
            Err(e) => signs.push(unread(
                format!("the install folder {}", self.install_dir.display()),
                e,
            )),
        }
        match reg_key_exists(&self.uninstall_key) {
            Ok(true) => signs.push(RealSign::UninstallKey {
                key: self.uninstall_key.clone(),
            }),
            Ok(false) => {}
            Err(e) => signs.push(unread(
                format!(r"the uninstall key HKCU\{}", self.uninstall_key),
                e,
            )),
        }
        match self.auto.get(StartKey::Run, RUN_NAME) {
            Ok(None) => {}
            Ok(Some(command)) => match self.auto.approval(RUN_NAME) {
                Ok(value) if switched_off(value.as_deref()) => {}
                Ok(_) => signs.push(RealSign::StartsAtLogin { command }),
                Err(e) => signs.push(unread(format!("the switch value {RUN_NAME}"), e)),
            },
            Err(e) => signs.push(unread(format!("the Run value {RUN_NAME}"), e)),
        }
        let journal = self.data_dir.file(DataFile::Journal);
        match exists(&journal) {
            Ok(true) => signs.push(RealSign::Journal { path: journal }),
            Ok(false) => {}
            Err(e) => signs.push(unread(format!("the journal {}", journal.display()), e)),
        }
        match self.auto.get(StartKey::RunOnce, RUN_ONCE_NAME) {
            Ok(Some(command)) => signs.push(RealSign::RunOnce { command }),
            Ok(None) => {}
            Err(e) => signs.push(unread(format!("the RunOnce value {RUN_ONCE_NAME}"), e)),
        }
        signs
    }
}

/// Off when the state, the value's first DWORD, is 1 (what the Settings > Apps > Startup switch wrote on the test
/// laptop) or 3 (what web sources name); research "Where Windows keeps the switch". No value, or any other form,
/// counts as on, so an unknown form stops a dev tool rather than miss a tray that starts at the next sign-in.
#[cfg(any(test, feature = "test-support"))]
fn switched_off(value: Option<&[u8]>) -> bool {
    let state = value
        .and_then(|bytes| bytes.get(..4))
        .and_then(|head| <[u8; 4]>::try_from(head).ok())
        .map(u32::from_le_bytes);
    matches!(state, Some(1 | 3))
}

/// `SYNCHRONIZE` only, and closed at once, so the open neither creates the mutex nor takes it. One this user may not
/// open exists too.
#[cfg(any(test, feature = "test-support"))]
fn mutex_exists(name: &str) -> Result<bool, WinError> {
    use windows::Win32::System::Threading::OpenMutexW;

    let name = wide("OpenMutexW", OsStr::new(name))?;
    // SAFETY: `name` is NUL-terminated and lives for the call.
    match unsafe { OpenMutexW(SYNCHRONIZATION_SYNCHRONIZE, false, PCWSTR(name.as_ptr())) } {
        Ok(handle) => {
            // SAFETY: the handle is new and owned only here; the drop closes it.
            drop(unsafe { Owned::new(handle) });
            Ok(true)
        }
        Err(e) => match core_error("OpenMutexW", &e) {
            e if e.code == ERROR_ACCESS_DENIED.0 => Ok(true),
            e => missing_is_none::<()>(e).map(|_| false),
        },
    }
}

/// A key this user may not open exists too.
#[cfg(any(test, feature = "test-support"))]
fn reg_key_exists(path: &str) -> Result<bool, WinError> {
    use windows::Win32::System::Registry::{KEY_QUERY_VALUE, RegOpenKeyExW};

    let name = wide("RegOpenKeyExW", OsStr::new(path))?;
    let mut key = HKEY::default();
    // SAFETY: `name` is NUL-terminated and lives for the call; the key is written to a live local.
    let opened = unsafe {
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(name.as_ptr()),
            None,
            KEY_QUERY_VALUE,
            &raw mut key,
        )
    };
    match opened {
        ERROR_SUCCESS => {
            // SAFETY: the key is new and owned only here; the drop closes it.
            drop(unsafe { Owned::new(key) });
            Ok(true)
        }
        ERROR_ACCESS_DENIED => Ok(true),
        other => missing_is_none::<()>(win32("RegOpenKeyExW", other)).map(|_| false),
    }
}

/// A file or folder; one this user may not read is an error, not a "no".
#[cfg(any(test, feature = "test-support"))]
fn exists(path: &Path) -> Result<bool, WinError> {
    path.try_exists()
        .map_err(|e| io_error("Path::try_exists", &e))
}

/// NUL-terminated UTF-16. An inner NUL would cut the name short and name another file or value, so it is refused.
fn wide(call: &'static str, text: &OsStr) -> Result<Vec<u16>, WinError> {
    let mut units: Vec<u16> = text.encode_wide().collect();
    if units.contains(&0) {
        return Err(win32(call, ERROR_INVALID_PARAMETER));
    }
    units.push(0);
    Ok(units)
}

fn win32(call: &'static str, code: WIN32_ERROR) -> WinError {
    WinError { call, code: code.0 }
}

/// Win32 errors stay Win32 codes (2, 5, 32), so callers compare them with `ERROR_*`; other failures keep the HRESULT.
fn core_error(call: &'static str, e: &windows::core::Error) -> WinError {
    let hresult = e.code().0 as u32;
    let code = if hresult & 0xFFFF_0000 == 0x8007_0000 {
        hresult & 0xFFFF
    } else {
        hresult
    };
    WinError { call, code }
}

fn io_error(call: &'static str, e: &std::io::Error) -> WinError {
    WinError {
        call,
        code: e
            .raw_os_error()
            .map_or(ERROR_INVALID_PARAMETER.0, |code| code as u32),
    }
}

fn reg_ok(call: &'static str, result: WIN32_ERROR) -> Result<(), WinError> {
    if result == ERROR_SUCCESS {
        Ok(())
    } else {
        Err(win32(call, result))
    }
}

/// A missing file, folder, key or value is `Ok(None)`; any other error stays an error.
fn missing_is_none<T>(e: WinError) -> Result<Option<T>, WinError> {
    if e.code == ERROR_FILE_NOT_FOUND.0 || e.code == ERROR_PATH_NOT_FOUND.0 {
        Ok(None)
    } else {
        Err(e)
    }
}

/// Test registry keys under `HKCU\Software\Wakeshot-test\<pid>`, one rule for every test of this crate.
/// Tests run in parallel threads and inside other workers' runs, so each test deletes only its own key, and the sweep
/// deletes only keys of processes that ended.
#[cfg(test)]
mod scratch {
    use std::ffi::OsStr;
    use std::sync::{Mutex, MutexGuard};

    use windows::Win32::Foundation::{
        ERROR_INVALID_PARAMETER, ERROR_KEY_DELETED, ERROR_SUCCESS, HANDLE, WAIT_TIMEOUT,
        WIN32_ERROR,
    };
    use windows::Win32::System::Registry::{
        HKEY, HKEY_CURRENT_USER, KEY_ENUMERATE_SUB_KEYS, KEY_SET_VALUE, REG_OPTION_NON_VOLATILE,
        RegCreateKeyExW, RegDeleteKeyW, RegDeleteTreeW, RegEnumKeyExW, RegOpenKeyExW,
    };
    use windows::Win32::System::Threading::{
        OpenProcess, PROCESS_SYNCHRONIZE, WaitForSingleObject,
    };
    use windows::core::{Owned, PCWSTR, PWSTR};

    use super::{WinError, core_error, reg_ok, wide, win32};

    /// Under `HKCU`.
    pub(crate) const ROOT: &str = r"Software\Wakeshot-test";

    /// Serializes this process's creates and parent deletes, so one test's clean-up never deletes the `<pid>` key
    /// that another test is creating under.
    static LOCK: Mutex<()> = Mutex::new(());

    fn lock() -> MutexGuard<'static, ()> {
        // A test that panicked while holding it left no half-done state behind: each step is one registry call.
        LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// `HKCU\Software\Wakeshot-test\<pid>\<test name>`, deleted on drop with its `<pid>` parent when that is empty.
    pub(crate) struct ScratchKey {
        path: String,
    }

    impl ScratchKey {
        /// `test_name` must be unique in the test binary (the test's path is a good one): two keys with one name are
        /// one key. Runs the sweep first, so keys of crashed runs go.
        pub(crate) fn new(test_name: &str) -> Self {
            sweep_scratch_keys();
            let leaf: String = test_name
                .chars()
                .map(|c| if c == '\\' { '_' } else { c })
                .collect();
            let path = format!(r"{}\{leaf}", pid_key(std::process::id()));
            let _lock = lock();
            if let Err(e) = create_key(&path) {
                panic!("scratch key {path}: {e:?}");
            }
            Self { path }
        }

        /// The key under `HKCU`, for `AutoStart::at`.
        pub(crate) fn path(&self) -> &str {
            &self.path
        }
    }

    impl Drop for ScratchKey {
        fn drop(&mut self) {
            let _lock = lock();
            let _ = delete_tree(&self.path);
            // `RegDeleteKeyW` refuses a key with subkeys, so a parent goes only when no other test still uses it.
            let _ = delete_key(&pid_key(std::process::id()));
            let _ = delete_key(ROOT);
        }
    }

    /// Deletes the `<pid>` keys whose process is not running. A reused pid only delays the delete.
    pub(crate) fn sweep_scratch_keys() {
        let me = std::process::id();
        for name in subkeys(ROOT) {
            let Ok(pid) = name.parse::<u32>() else {
                continue;
            };
            if pid != me && !running(pid) {
                let _ = delete_tree(&pid_key(pid));
            }
        }
    }

    pub(super) fn pid_key(pid: u32) -> String {
        format!(r"{ROOT}\{pid}")
    }

    /// An open and a zero wait. A denied open still means a process holds the pid.
    fn running(pid: u32) -> bool {
        // SAFETY: plain call; the handle is owned below.
        match unsafe { OpenProcess(PROCESS_SYNCHRONIZE, false, pid) } {
            Ok(process) => {
                // SAFETY: the handle is new and owned only here.
                let process: Owned<HANDLE> = unsafe { Owned::new(process) };
                // SAFETY: an open process handle with `SYNCHRONIZE`.
                unsafe { WaitForSingleObject(*process, 0) == WAIT_TIMEOUT }
            }
            Err(e) => core_error("OpenProcess", &e).code != ERROR_INVALID_PARAMETER.0,
        }
    }

    pub(super) fn create_key(path: &str) -> Result<(), WinError> {
        let name = wide("RegCreateKeyExW", OsStr::new(path))?;
        let mut last = ERROR_SUCCESS;
        for _ in 0..5 {
            let mut key = HKEY::default();
            // SAFETY: `name` is NUL-terminated and lives for the call; the key is written to a live local.
            let result = unsafe {
                RegCreateKeyExW(
                    HKEY_CURRENT_USER,
                    PCWSTR(name.as_ptr()),
                    None,
                    PCWSTR::null(),
                    REG_OPTION_NON_VOLATILE,
                    KEY_SET_VALUE,
                    None,
                    &raw mut key,
                    None,
                )
            };
            if result == ERROR_SUCCESS {
                // SAFETY: the key is new and owned only here; it closes at once.
                drop(unsafe { Owned::new(key) });
                return Ok(());
            }
            // Another test process deleted an empty parent between the steps of this create; the next try makes it
            // again.
            if result != ERROR_KEY_DELETED {
                return Err(win32("RegCreateKeyExW", result));
            }
            last = result;
        }
        Err(win32("RegCreateKeyExW", last))
    }

    /// The key, its subkeys and its values. A missing key is not an error.
    pub(super) fn delete_tree(path: &str) -> Result<(), WinError> {
        let name = wide("RegDeleteTreeW", OsStr::new(path))?;
        // SAFETY: `name` is NUL-terminated and lives for the call.
        let result = unsafe { RegDeleteTreeW(HKEY_CURRENT_USER, PCWSTR(name.as_ptr())) };
        not_found_ok("RegDeleteTreeW", result)?;
        delete_key(path)
    }

    /// The key only, when it has no subkeys. A missing key is not an error.
    fn delete_key(path: &str) -> Result<(), WinError> {
        let name = wide("RegDeleteKeyW", OsStr::new(path))?;
        // SAFETY: `name` is NUL-terminated and lives for the call.
        let result = unsafe { RegDeleteKeyW(HKEY_CURRENT_USER, PCWSTR(name.as_ptr())) };
        not_found_ok("RegDeleteKeyW", result)
    }

    fn not_found_ok(call: &'static str, result: WIN32_ERROR) -> Result<(), WinError> {
        if result == windows::Win32::Foundation::ERROR_FILE_NOT_FOUND {
            return Ok(());
        }
        reg_ok(call, result)
    }

    /// Subkey names; a missing or unreadable key has none.
    pub(super) fn subkeys(path: &str) -> Vec<String> {
        let Ok(name) = wide("RegOpenKeyExW", OsStr::new(path)) else {
            return Vec::new();
        };
        let mut key = HKEY::default();
        // SAFETY: `name` is NUL-terminated and lives for the call; the key is written to a live local.
        let opened = unsafe {
            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                PCWSTR(name.as_ptr()),
                None,
                KEY_ENUMERATE_SUB_KEYS,
                &raw mut key,
            )
        };
        if opened != ERROR_SUCCESS {
            return Vec::new();
        }
        // SAFETY: the key is new and owned only here.
        let key = unsafe { Owned::new(key) };
        let mut names = Vec::new();
        for index in 0.. {
            // Key names are at most 255 characters (Microsoft, "Registry element size limits").
            let mut buf = [0u16; 256];
            let mut len = buf.len() as u32;
            // SAFETY: an open key with `KEY_ENUMERATE_SUB_KEYS`; `buf` holds `len` characters.
            let result = unsafe {
                RegEnumKeyExW(
                    *key,
                    index,
                    Some(PWSTR(buf.as_mut_ptr())),
                    &raw mut len,
                    None,
                    None,
                    None,
                    None,
                )
            };
            if result != ERROR_SUCCESS {
                break;
            }
            names.push(String::from_utf16_lossy(&buf[..len as usize]));
        }
        names
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::windows::fs::OpenOptionsExt;
    use std::process::Command;
    use std::thread;

    use windows::Win32::Security::{
        GetSidSubAuthority, GetSidSubAuthorityCount, GetTokenInformation, TOKEN_MANDATORY_LABEL,
        TOKEN_QUERY, TokenIntegrityLevel,
    };
    use windows::Win32::System::Registry::{KEY_QUERY_VALUE, REG_DWORD, RegOpenKeyExW};
    use windows::Win32::System::Threading::OpenProcessToken;

    use super::scratch::{ROOT, create_key, delete_tree, pid_key, subkeys};
    use super::*;

    /// A fresh folder under `%TEMP%`, removed with everything in it when the test ends.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(tag: &str) -> Self {
            static N: AtomicU64 = AtomicU64::new(0);
            let n = N.fetch_add(1, Ordering::Relaxed);
            let dir = std::env::temp_dir()
                .join(format!("wakeshot-store-{tag}-{}-{n}", std::process::id()));
            fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn names_in(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    fn key_exists(path: &str) -> bool {
        let name = wide("RegOpenKeyExW", OsStr::new(path)).unwrap();
        let mut key = HKEY::default();
        // SAFETY: `name` is NUL-terminated; the key is written to a live local and owned below.
        let result = unsafe {
            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                PCWSTR(name.as_ptr()),
                None,
                KEY_QUERY_VALUE,
                &raw mut key,
            )
        };
        if result == ERROR_SUCCESS {
            // SAFETY: the key is new and owned only here.
            drop(unsafe { Owned::new(key) });
        }
        result == ERROR_SUCCESS
    }

    /// A test key the test made by hand; deleted even when an assert fails first.
    struct HandKey(String);

    impl Drop for HandKey {
        fn drop(&mut self) {
            let _ = delete_tree(&self.0);
        }
    }

    /// The test runner's integrity level, the last sub-authority of its mandatory label (0x2000 Medium, 0x3000 High).
    fn integrity_rid() -> u32 {
        let mut token = HANDLE::default();
        // SAFETY: the pseudo handle needs no close; the token is written to a live local and owned below.
        unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &raw mut token) }.unwrap();
        // SAFETY: the token is new and owned only here.
        let token = unsafe { Owned::new(token) };
        let mut len = 0u32;
        // SAFETY: a size query with no buffer; it fails by design and only sets `len`.
        let _ = unsafe { GetTokenInformation(*token, TokenIntegrityLevel, None, 0, &raw mut len) };
        // u64 units keep the label's pointer field aligned.
        let mut buf = vec![0u64; (len as usize).div_ceil(8)];
        // SAFETY: `buf` holds at least `len` bytes.
        unsafe {
            GetTokenInformation(
                *token,
                TokenIntegrityLevel,
                Some(buf.as_mut_ptr().cast()),
                len,
                &raw mut len,
            )
        }
        .unwrap();
        // SAFETY: the call filled `buf` with a TOKEN_MANDATORY_LABEL whose SID lives inside `buf`.
        unsafe {
            let label = &*buf.as_ptr().cast::<TOKEN_MANDATORY_LABEL>();
            let count = *GetSidSubAuthorityCount(label.Label.Sid);
            *GetSidSubAuthority(label.Label.Sid, u32::from(count) - 1)
        }
    }

    #[test]
    fn data_dir_paths() {
        let root = real_root(None).unwrap();
        assert_eq!(root, local_app_data().unwrap().join("Wakeshot"));
        assert!(root.is_absolute());
        assert!(root.ends_with("Wakeshot"));
        assert!(
            !root
                .components()
                .any(|c| c.as_os_str() == "HaikAsatryan.Wakeshot")
        );
        let dry = DataDir::at(root.join("dry-run"));
        assert!(dry.root().ends_with(r"Wakeshot\dry-run"));
        // `dry_run` follows `real`, override or not.
        assert_eq!(
            DataDir::dry_run().unwrap().root(),
            DataDir::real().unwrap().root().join("dry-run")
        );

        assert_eq!(
            real_root(Some(OsString::from(r"C:\elsewhere"))).unwrap(),
            Path::new(r"C:\elsewhere")
        );
        assert_eq!(real_root(Some(OsString::new())).unwrap(), root);

        let dir = DataDir::at(PathBuf::from(r"C:\data"));
        let files = [
            (DataFile::Config, r"C:\data\config.json"),
            (DataFile::State, r"C:\data\state.json"),
            (DataFile::Journal, r"C:\data\journal.json"),
            (DataFile::JournalCorrupt, r"C:\data\journal.json.corrupt"),
            (
                DataFile::JournalCorruptSeen,
                r"C:\data\journal.json.corrupt.seen",
            ),
            (DataFile::RestoreRequest, r"C:\data\restore-request.json"),
            (DataFile::KeepAwake, r"C:\data\holds\keep-awake.json"),
        ];
        for (file, path) in files {
            assert_eq!(dir.file(file), Path::new(path), "{file:?}");
        }
        assert_eq!(dir.holds(), Path::new(r"C:\data\holds"));
        assert_eq!(dir.logs(), Path::new(r"C:\data\logs"));
    }

    #[test]
    fn names_stay_apart() {
        let real = Names::real();
        let dry = Names::dry_run();
        let a = Names::test("store::names");
        let b = Names::test("store::names");
        for names in [&real, &dry, &a, &b] {
            assert!(names.tray.starts_with(r"Local\"));
            assert!(names.settings.starts_with(r"Global\"));
            assert!(names.journal.starts_with(r"Local\"));
        }
        assert_ne!(real.journal, dry.journal);
        assert_ne!(a, b);
        assert!(a.tray.contains("store::names") && a.tray != real.tray && a.tray != dry.tray);
        let odd = Names::test(r"a\b");
        assert_eq!(odd.journal.matches('\\').count(), 1);
    }

    #[test]
    fn atomic_write_replaces_whole_file() {
        let dir = TempDir::new("replace");
        let path = dir.0.join("state.json");
        write_atomic(
            &path,
            b"{ \"version\": 1, \"old\": \"a much longer old file\" }",
        )
        .unwrap();
        write_atomic(&path, b"{ \"version\": 1 }").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"{ \"version\": 1 }");
        assert_eq!(names_in(&dir.0), ["state.json"]);

        // A first write into a folder that does not exist yet.
        let hold = dir.0.join("holds").join("keep-awake.json");
        write_atomic(&hold, b"{}").unwrap();
        assert_eq!(fs::read(&hold).unwrap(), b"{}");
        assert_eq!(names_in(&dir.0.join("holds")), ["keep-awake.json"]);
    }

    #[test]
    fn atomic_write_keeps_old_file_on_failure() {
        let dir = TempDir::new("fail");
        let path = dir.0.join("journal.json");
        write_atomic(&path, b"old").unwrap();
        // Held open without delete sharing, so nothing can replace it.
        let blocker = fs::OpenOptions::new()
            .read(true)
            .share_mode((FILE_SHARE_READ | FILE_SHARE_WRITE).0)
            .open(&path)
            .unwrap();
        let err = write_atomic(&path, b"new").unwrap_err();
        println!("replace of a file open without delete sharing: {err:?}");
        assert_eq!((err.call, err.code), ("MoveFileExW", ERROR_ACCESS_DENIED.0));
        assert_eq!(read_whole(&path).unwrap().as_deref(), Some(&b"old"[..]));
        assert_eq!(names_in(&dir.0), ["journal.json"]);
        drop(blocker);
    }

    #[test]
    fn stop_before_rename_keeps_old_file() {
        let dir = TempDir::new("stop");
        let path = dir.0.join("journal.json");
        write_atomic(&path, b"old").unwrap();
        // The process stops here: the temp file is written and flushed, the move never runs.
        let _stopped = write_temp(&path, b"new").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"old");
        let names = names_in(&dir.0);
        assert_eq!(names.len(), 2, "{names:?}");
        let temp = &names[1];
        assert!(
            temp.starts_with(&format!("journal.json.{}.", std::process::id())),
            "{temp}"
        );
        assert!(temp.ends_with(".tmp") && !temp.ends_with(".json"), "{temp}");
        assert_eq!(temp_target(temp), Some("journal.json"), "{temp}");
        assert_eq!(fs::read(dir.0.join(temp)).unwrap(), b"new");
    }

    #[test]
    fn temp_names_have_one_shape_and_a_random_part() {
        let a = temp_name(OsStr::new("4f1c.json")).into_string().unwrap();
        let b = temp_name(OsStr::new("4f1c.json")).into_string().unwrap();
        assert_eq!(temp_target(&a), Some("4f1c.json"), "{a}");
        assert_eq!(temp_target(&b), Some("4f1c.json"), "{b}");
        let random = |name: &str| name.rsplit('.').nth(1).unwrap().to_owned();
        assert_ne!(random(&a), random(&b));

        let pid = std::process::id();
        for (name, target) in [
            (
                format!("x.json.{pid}.0.0123456789abcdef.tmp"),
                Some("x.json"),
            ),
            ("a.b.c.1.2.ffffffffffffffff.tmp".to_owned(), Some("a.b.c")),
            // The old shape, upper case, a short or long random part, a missing part, other ends.
            (format!("x.json.{pid}.0.tmp"), None),
            ("x.json.1.2.0123456789ABCDEF.tmp".to_owned(), None),
            ("x.json.1.2.0123456789abcde.tmp".to_owned(), None),
            ("x.json.1.2.0123456789abcdef0.tmp".to_owned(), None),
            (".1.2.0123456789abcdef.tmp".to_owned(), None),
            ("1.2.0123456789abcdef.tmp".to_owned(), None),
            ("x.json.a.2.0123456789abcdef.tmp".to_owned(), None),
            ("x.json.1..0123456789abcdef.tmp".to_owned(), None),
            ("x.json.1.2.0123456789abcdef.TMP".to_owned(), None),
            ("x.json.1.2.0123456789abcdef.tmp.json".to_owned(), None),
            ("x.json".to_owned(), None),
            (
                "x.json.111111111111111111111.2.0123456789abcdef.tmp".to_owned(),
                None,
            ),
        ] {
            assert_eq!(temp_target(&name), target, "{name}");
        }
    }

    /// Hard-link support is per file system: NTFS has it (`%TEMP%` was on NTFS on the test laptop).
    #[test]
    fn a_planted_hard_link_at_a_temp_name_is_never_written_through() {
        let dir = TempDir::new("planted");
        let victim = dir.0.join("victim.txt");
        fs::write(&victim, b"secret").unwrap();
        let path = dir.0.join("4f1c.json");
        let planted = dir.0.join("4f1c.json.1.1.0000000000000001.tmp");
        fs::hard_link(&victim, &planted).unwrap();
        let fresh = dir.0.join("4f1c.json.1.2.0000000000000002.tmp");
        let mut names = [planted.clone(), fresh].into_iter();

        let target = wide("MoveFileExW", path.as_os_str()).unwrap();
        write_temp_named(&path, b"new", || names.next().unwrap())
            .unwrap()
            .replace(&target)
            .unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"new");
        assert_eq!(fs::read(&victim).unwrap(), b"secret", "the link's target");
        assert_eq!(fs::read(&planted).unwrap(), b"secret", "the link stays");
        assert_eq!(
            names_in(&dir.0),
            [
                "4f1c.json",
                "4f1c.json.1.1.0000000000000001.tmp",
                "victim.txt"
            ]
        );
    }

    #[test]
    fn a_write_whose_names_all_exist_fails_and_deletes_nothing() {
        let dir = TempDir::new("taken");
        let path = dir.0.join("4f1c.json");
        fs::write(&path, b"old").unwrap();
        let taken: Vec<PathBuf> = (0..TEMP_TRIES)
            .map(|i| {
                dir.0
                    .join(format!("4f1c.json.1.{i}.000000000000000{i}.tmp"))
            })
            .collect();
        for name in &taken {
            fs::write(name, b"left").unwrap();
        }
        let mut names = taken.clone().into_iter();
        let err = write_temp_named(&path, b"new", || names.next().unwrap())
            .err()
            .unwrap();
        assert_eq!((err.call, err.code), ("CreateFileW", ERROR_FILE_EXISTS.0));
        assert_eq!(names.next(), None, "every try took a new name");
        assert_eq!(fs::read(&path).unwrap(), b"old");
        for name in &taken {
            assert_eq!(fs::read(name).unwrap(), b"left");
        }
    }

    #[test]
    fn atomic_write_replaces_a_file_a_reader_has_open() {
        let dir = TempDir::new("reader");
        let path = dir.0.join("config.json");
        write_atomic(&path, b"old").unwrap();
        let reader = open_shared(&path).unwrap().unwrap();
        write_atomic(&path, b"new").unwrap();
        // The open handle still reads the file it opened, whole.
        assert_eq!(read_to_end(*reader).unwrap(), b"old");
        drop(reader);
        assert_eq!(read_whole(&path).unwrap().as_deref(), Some(&b"new"[..]));
        assert_eq!(names_in(&dir.0), ["config.json"]);
    }

    #[test]
    fn read_whole_tells_missing_from_error() {
        let dir = TempDir::new("read");
        assert_eq!(read_whole(&dir.0.join("none.json")), Ok(None));
        assert_eq!(
            read_whole(&dir.0.join("no-folder").join("none.json")),
            Ok(None)
        );
        let err = read_whole(&dir.0).unwrap_err();
        assert_eq!(err.call, "CreateFileW");
        let empty = dir.0.join("empty.json");
        fs::write(&empty, b"").unwrap();
        assert_eq!(read_whole(&empty), Ok(Some(Vec::new())));
        let big: Vec<u8> = (0..100_000u32).map(|i| (i % 251) as u8).collect();
        let file = dir.0.join("big.json");
        fs::write(&file, &big).unwrap();
        assert_eq!(read_whole(&file), Ok(Some(big)));
    }

    #[test]
    fn remove_tells_missing_from_error() {
        let dir = TempDir::new("remove");
        let path = dir.0.join("restore-request.json");
        assert_eq!(remove(&path), Ok(false));
        assert_eq!(remove(&dir.0.join("no-folder").join("x.json")), Ok(false));
        fs::write(&path, b"{}").unwrap();
        assert_eq!(remove(&path), Ok(true));
        assert!(!path.exists());
        assert_eq!(remove(&dir.0).unwrap_err().call, "DeleteFileW");
    }

    #[test]
    fn remove_and_rewrite_while_a_reader_has_the_file_open() {
        let dir = TempDir::new("remove-reader");
        let path = dir.0.join("run-4242.json");
        write_atomic(&path, b"old").unwrap();
        let reader = open_shared(&path).unwrap().unwrap();
        assert_eq!(remove(&path), Ok(true));
        // The name is free at once, so the next write of the same file works while the reader still reads.
        assert_eq!(read_whole(&path), Ok(None));
        write_atomic(&path, b"new").unwrap();
        assert_eq!(read_to_end(*reader).unwrap(), b"old");
        drop(reader);
        assert_eq!(read_whole(&path).unwrap().as_deref(), Some(&b"new"[..]));
        assert_eq!(names_in(&dir.0), ["run-4242.json"]);
    }

    #[test]
    fn mutex_excludes_and_releases() {
        let name = Names::test("store::excludes").journal;
        // The guard keeps its own handle, so it holds after the `NamedMutex` is gone.
        let held = NamedMutex::open(&name)
            .unwrap()
            .take(0)
            .unwrap()
            .expect("a new mutex is free");
        assert!(!held.abandoned());

        let other = name.clone();
        let blocked = thread::spawn(move || {
            NamedMutex::open(&other)
                .unwrap()
                .take(50)
                .unwrap()
                .is_none()
        });
        assert!(
            blocked.join().unwrap(),
            "another thread must not take a held mutex"
        );

        drop(held);
        // A `NamedMutex` moves to the thread that takes it (the IO thread takes the journal mutex).
        let mutex = NamedMutex::open(&name).unwrap();
        let taken = thread::spawn(move || {
            let held = mutex.take(1000).unwrap();
            held.map(|held| held.abandoned())
        });
        assert_eq!(taken.join().unwrap(), Some(false));
    }

    #[test]
    fn abandoned_mutex_counts_as_taken() {
        let name = Names::test("store::abandoned").journal;
        let mutex = NamedMutex::open(&name).unwrap();
        let other = name.clone();
        thread::spawn(move || {
            let held = NamedMutex::open(&other).unwrap().take(0).unwrap().unwrap();
            // The thread ends while it holds the mutex, as a killed process would.
            std::mem::forget(held);
        })
        .join()
        .unwrap();

        let held = mutex
            .take(1000)
            .unwrap()
            .expect("an abandoned mutex counts as taken");
        assert!(held.abandoned());
        drop(held);
        let again = mutex.take(0).unwrap().expect("released on drop");
        assert!(!again.abandoned());
    }

    #[test]
    fn global_mutex_creates() {
        let rid = integrity_rid();
        let level = match rid {
            0x1000 => "Low",
            0x2000 => "Medium",
            0x2100 => "Medium Plus",
            0x3000 => "High",
            0x4000 => "System",
            _ => "other",
        };
        println!("test runner integrity level: 0x{rid:04x} ({level})");
        let name = Names::test("store::global").settings;
        assert!(name.starts_with(r"Global\"));
        let mutex = NamedMutex::open(&name).expect("a Global mutex needs no privilege");
        let held = mutex.take(0).unwrap().expect("a new mutex is free");
        assert!(!held.abandoned());
    }

    #[test]
    fn auto_start_round_trip() {
        let key = ScratchKey::new("store::auto_start_round_trip");
        let auto = AutoStart::at(key.path());
        // A Unicode path (Armenian letters) must come back as it went in.
        let command = "\"C:\\Users\\\u{540}\u{561}\u{575}\u{56f}\\Wakeshot\\current\\wakeshot-tray.exe\" --restore";
        for start in [StartKey::Run, StartKey::RunOnce] {
            assert_eq!(auto.get(start, "WakeshotRestore"), Ok(None));
            assert_eq!(auto.remove(start, "WakeshotRestore"), Ok(false));
            auto.set(start, "WakeshotRestore", "old").unwrap();
            auto.set(start, "WakeshotRestore", command).unwrap();
            assert_eq!(
                auto.get(start, "WakeshotRestore"),
                Ok(Some(command.to_owned()))
            );
        }
        assert_eq!(auto.remove(StartKey::Run, "WakeshotRestore"), Ok(true));
        assert_eq!(auto.get(StartKey::Run, "WakeshotRestore"), Ok(None));
        // Run and RunOnce are two keys.
        assert_eq!(
            auto.get(StartKey::RunOnce, "WakeshotRestore"),
            Ok(Some(command.to_owned()))
        );
        assert_eq!(auto.remove(StartKey::RunOnce, "WakeshotRestore"), Ok(true));
        assert_eq!(auto.remove(StartKey::RunOnce, "WakeshotRestore"), Ok(false));
        assert_eq!(
            auto.set(StartKey::Run, "bad\0name", "x").unwrap_err().code,
            ERROR_INVALID_PARAMETER.0
        );

        let path = key.path().to_owned();
        drop(key);
        assert!(!key_exists(&path));
    }

    #[test]
    fn auto_start_refuses_other_types() {
        let key = ScratchKey::new("store::auto_start_refuses_other_types");
        let auto = AutoStart::at(key.path());
        auto.set(StartKey::Run, "Other", "x").unwrap();
        let name = wide(
            "RegCreateKeyExW",
            OsStr::new(&format!(r"{}\Run", key.path())),
        )
        .unwrap();
        let mut opened = HKEY::default();
        // SAFETY: `name` is NUL-terminated; the key is written to a live local and owned below.
        let created = unsafe {
            RegCreateKeyExW(
                HKEY_CURRENT_USER,
                PCWSTR(name.as_ptr()),
                None,
                PCWSTR::null(),
                REG_OPTION_NON_VOLATILE,
                KEY_SET_VALUE,
                None,
                &raw mut opened,
                None,
            )
        };
        assert_eq!(created, ERROR_SUCCESS);
        // SAFETY: the key is new and owned only here.
        let opened = unsafe { Owned::new(opened) };
        let value = wide("RegSetValueExW", OsStr::new("Other")).unwrap();
        // SAFETY: an open key with `KEY_SET_VALUE`; the data is a live 4-byte DWORD.
        let set = unsafe {
            RegSetValueExW(
                *opened,
                PCWSTR(value.as_ptr()),
                None,
                REG_DWORD,
                Some(&7u32.to_le_bytes()[..]),
            )
        };
        assert_eq!(set, ERROR_SUCCESS);
        let err = auto.get(StartKey::Run, "Other").unwrap_err();
        println!("a DWORD read as REG_SZ: {err:?}");
        assert_eq!(err.call, "RegGetValueW");
    }

    #[test]
    fn auto_start_removes_the_switch_value() {
        assert_eq!(
            AutoStart::real().approval_key(),
            r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run"
        );
        let key = ScratchKey::new("store::auto_start_removes_the_switch_value");
        let auto = AutoStart::at(key.path());
        auto.set(StartKey::Run, "Wakeshot", "x").unwrap();
        // No switch key yet, as on a PC where nobody used the switch.
        assert_eq!(auto.remove_approval("Wakeshot"), Ok(false));
        assert_eq!(auto.approval("Wakeshot"), Ok(None));

        // As the real switch wrote them (research "Where Windows keeps the switch"): off is state 1, then the FILETIME
        // of the switch-off; on is 12 zero bytes.
        let off = [1, 0, 0, 0, 0x9D, 0xAA, 0x39, 0x37, 0xA4, 0x6C, 0xDD, 0x01];
        let on = [0; 12];
        auto.set_approval("Wakeshot", &off).unwrap();
        auto.set_approval("Other", &on).unwrap();
        assert_eq!(auto.approval("Wakeshot"), Ok(Some(off.to_vec())));
        // The Run value of the same name lives in another key, and each delete leaves the other.
        assert_eq!(auto.remove(StartKey::Run, "Wakeshot"), Ok(true));
        assert_eq!(auto.approval("Wakeshot"), Ok(Some(off.to_vec())));
        auto.set(StartKey::Run, "Wakeshot", "x").unwrap();
        assert_eq!(auto.remove_approval("Wakeshot"), Ok(true));
        assert_eq!(auto.approval("Wakeshot"), Ok(None));
        assert_eq!(
            auto.get(StartKey::Run, "Wakeshot"),
            Ok(Some("x".to_owned()))
        );
        assert_eq!(auto.remove_approval("Wakeshot"), Ok(false));
        assert_eq!(auto.approval("Other"), Ok(Some(on.to_vec())));
        assert_eq!(
            auto.remove_approval("bad\0name").unwrap_err().code,
            ERROR_INVALID_PARAMETER.0
        );

        let path = key.path().to_owned();
        drop(key);
        assert!(!key_exists(&path));
    }

    #[test]
    fn sweep_skips_live_pids() {
        let mine = ScratchKey::new("store::sweep_skips_live_pids");
        let mut child = Command::new("cmd.exe")
            .args(["/c", "exit", "0"])
            .spawn()
            .unwrap();
        child.wait().unwrap();
        // `child` keeps its process handle open, so its pid cannot be reused before the sweep.
        let dead = HandKey(pid_key(child.id()));
        create_key(&format!(r"{}\left", dead.0)).unwrap();
        assert!(subkeys(ROOT).contains(&child.id().to_string()));

        sweep_scratch_keys();
        assert!(key_exists(mine.path()), "a live pid's key stays");
        assert!(!key_exists(&dead.0), "an ended pid's key goes");
        drop(child);
    }

    /// Scratch places that start clean: a lock name nobody made, and a folder and keys with nothing in them yet. The
    /// real lock, folders and keys are never touched.
    struct RealRig {
        places: RealPlaces,
        _dir: TempDir,
        _key: ScratchKey,
    }

    impl RealRig {
        fn new(test_name: &str) -> Self {
            let dir = TempDir::new("real");
            let key = ScratchKey::new(test_name);
            let places = RealPlaces {
                tray_lock: Names::test(test_name).tray,
                install_dir: dir.0.join(PACK_ID),
                uninstall_key: format!(r"{}\Uninstall\{PACK_ID}", key.path()),
                data_dir: DataDir::at(dir.0.join("Wakeshot")),
                auto: AutoStart::at(key.path()),
            };
            Self {
                places,
                _dir: dir,
                _key: key,
            }
        }
    }

    /// A `REG_DWORD`, which `AutoStart::get` refuses to read as a command.
    fn set_dword(key: &str, name: &str) {
        create_key(key).unwrap();
        let path = wide("RegOpenKeyExW", OsStr::new(key)).unwrap();
        let mut opened = HKEY::default();
        // SAFETY: `path` is NUL-terminated; the key is written to a live local and owned below.
        let result = unsafe {
            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                PCWSTR(path.as_ptr()),
                None,
                KEY_SET_VALUE,
                &raw mut opened,
            )
        };
        assert_eq!(result, ERROR_SUCCESS);
        // SAFETY: the key is new and owned only here.
        let opened = unsafe { Owned::new(opened) };
        let name = wide("RegSetValueExW", OsStr::new(name)).unwrap();
        // SAFETY: an open key with `KEY_SET_VALUE`; the data is a live 4-byte DWORD.
        let set = unsafe {
            RegSetValueExW(
                *opened,
                PCWSTR(name.as_ptr()),
                None,
                REG_DWORD,
                Some(&1u32.to_le_bytes()[..]),
            )
        };
        assert_eq!(set, ERROR_SUCCESS);
    }

    const NONE: [RealSign; 0] = [];
    const TRAY_COMMAND: &str =
        r#""C:\Users\Test User\AppData\Local\HaikAsatryan.Wakeshot\current\wakeshot-tray.exe""#;
    const RESTORE_COMMAND: &str = r#""C:\Users\Test User\AppData\Local\HaikAsatryan.Wakeshot\current\wakeshot-tray.exe" --restore"#;

    #[test]
    fn real_places_name_the_installed_app() {
        let real = RealPlaces::real().unwrap();
        let local = local_app_data().unwrap();
        assert_eq!(real.tray_lock, Names::real().tray);
        assert_eq!(real.tray_lock, r"Local\Wakeshot.Tray");
        assert_eq!(real.install_dir, local.join("HaikAsatryan.Wakeshot"));
        assert_eq!(
            real.uninstall_key,
            r"Software\Microsoft\Windows\CurrentVersion\Uninstall\HaikAsatryan.Wakeshot"
        );
        // Not `DataDir::real`, which a debug build moves with `WAKESHOT_DATA_DIR`.
        assert_eq!(real.data_dir.root(), local.join("Wakeshot"));
        assert_eq!(real.auto, AutoStart::real());
    }

    #[test]
    fn real_signs_none_on_a_clean_pc() {
        let rig = RealRig::new("store::real_signs_none_on_a_clean_pc");
        assert_eq!(rig.places.signs(), NONE);
        // The check opened no lock that a second look would find.
        assert_eq!(rig.places.signs(), NONE);
    }

    #[test]
    fn real_signs_a_running_tray() {
        let rig = RealRig::new("store::real_signs_a_running_tray");
        let lock = rig.places.tray_lock.clone();
        let mutex = NamedMutex::open(&lock).unwrap();
        let runs = [RealSign::TrayRuns { lock: lock.clone() }];
        assert_eq!(rig.places.signs(), runs, "made, not taken");
        let held = mutex.take(0).unwrap().expect("a new mutex is free");
        assert_eq!(rig.places.signs(), runs, "taken");
        // The look never takes the lock, so another thread still finds it held, never free or abandoned.
        let other = thread::spawn(move || {
            let mutex = NamedMutex::open(&lock).unwrap();
            mutex.take(0).unwrap().is_none()
        });
        assert!(other.join().unwrap(), "still held by this thread");
        drop(held);
        drop(mutex);
        assert_eq!(rig.places.signs(), NONE, "gone with its last handle");
    }

    #[test]
    fn real_signs_an_install() {
        let rig = RealRig::new("store::real_signs_an_install");
        let places = &rig.places;
        fs::create_dir_all(&places.install_dir).unwrap();
        assert_eq!(
            places.signs(),
            [RealSign::InstallFolder {
                path: places.install_dir.clone()
            }]
        );
        fs::remove_dir(&places.install_dir).unwrap();
        create_key(&places.uninstall_key).unwrap();
        assert_eq!(
            places.signs(),
            [RealSign::UninstallKey {
                key: places.uninstall_key.clone()
            }]
        );
    }

    #[test]
    fn real_signs_start_at_login_by_the_switch() {
        let rig = RealRig::new("store::real_signs_start_at_login_by_the_switch");
        let (places, auto) = (&rig.places, &rig.places.auto);
        auto.set(StartKey::Run, "Wakeshot", TRAY_COMMAND).unwrap();
        let on = [RealSign::StartsAtLogin {
            command: TRAY_COMMAND.to_owned(),
        }];
        assert_eq!(places.signs(), on, "no switch value: on");
        let filetime = [0x9D, 0xAA, 0x39, 0x37, 0xA4, 0x6C, 0xDD, 0x01];
        // 1 and 0 are what the real switch wrote; 3, 2 and 6 are what web sources name.
        for (state, off) in [(1u8, true), (3, true), (0, false), (2, false), (6, false)] {
            let mut value = vec![state, 0, 0, 0];
            value.extend(if off { filetime } else { [0; 8] });
            auto.set_approval("Wakeshot", &value).unwrap();
            let want: &[RealSign] = if off { &NONE } else { &on };
            assert_eq!(places.signs(), want, "state {state}");
        }
        // A switch value with no Run value starts nothing.
        auto.set_approval("Wakeshot", &[0; 12]).unwrap();
        assert_eq!(auto.remove(StartKey::Run, "Wakeshot"), Ok(true));
        assert_eq!(places.signs(), NONE);
    }

    #[test]
    fn switch_forms() {
        assert!(!switched_off(None), "no value is on");
        assert!(switched_off(Some(&[
            1, 0, 0, 0, 0x9D, 0xAA, 0x39, 0x37, 0xA4, 0x6C, 0xDD, 0x01
        ])));
        assert!(switched_off(Some(&[3, 0, 0, 0])));
        assert!(!switched_off(Some(&[0; 12])));
        assert!(!switched_off(Some(&[2, 0, 0, 0])));
        assert!(!switched_off(Some(&[1, 1, 0, 0])), "state 257 is unknown");
        assert!(!switched_off(Some(&[1, 0, 0])), "too short for a state");
        assert!(!switched_off(Some(&[])));
    }

    #[test]
    fn real_signs_a_journal_and_a_run_once_value() {
        let rig = RealRig::new("store::real_signs_a_journal_and_a_run_once_value");
        let places = &rig.places;
        fs::create_dir_all(places.data_dir.root()).unwrap();
        assert_eq!(places.signs(), NONE, "a data folder alone is no sign");
        let journal = places.data_dir.file(DataFile::Journal);
        fs::write(&journal, b"{}").unwrap();
        assert_eq!(
            places.signs(),
            [RealSign::Journal {
                path: journal.clone()
            }]
        );
        fs::remove_file(&journal).unwrap();
        places
            .auto
            .set(StartKey::RunOnce, "WakeshotRestore", RESTORE_COMMAND)
            .unwrap();
        assert_eq!(
            places.signs(),
            [RealSign::RunOnce {
                command: RESTORE_COMMAND.to_owned()
            }]
        );
    }

    #[test]
    fn real_signs_all_in_order() {
        let rig = RealRig::new("store::real_signs_all_in_order");
        let places = &rig.places;
        let _mutex = NamedMutex::open(&places.tray_lock).unwrap();
        fs::create_dir_all(&places.install_dir).unwrap();
        create_key(&places.uninstall_key).unwrap();
        places
            .auto
            .set(StartKey::Run, "Wakeshot", TRAY_COMMAND)
            .unwrap();
        fs::create_dir_all(places.data_dir.root()).unwrap();
        fs::write(places.data_dir.file(DataFile::Journal), b"{}").unwrap();
        places
            .auto
            .set(StartKey::RunOnce, "WakeshotRestore", RESTORE_COMMAND)
            .unwrap();
        assert_eq!(
            places.signs(),
            [
                RealSign::TrayRuns {
                    lock: places.tray_lock.clone()
                },
                RealSign::InstallFolder {
                    path: places.install_dir.clone()
                },
                RealSign::UninstallKey {
                    key: places.uninstall_key.clone()
                },
                RealSign::StartsAtLogin {
                    command: TRAY_COMMAND.to_owned()
                },
                RealSign::Journal {
                    path: places.data_dir.file(DataFile::Journal)
                },
                RealSign::RunOnce {
                    command: RESTORE_COMMAND.to_owned()
                },
            ]
        );
    }

    #[test]
    fn real_signs_what_does_not_read() {
        use windows::Win32::Foundation::ERROR_INVALID_HANDLE;
        use windows::Win32::System::Threading::CreateEventW;

        let rig = RealRig::new("store::real_signs_what_does_not_read");
        let places = &rig.places;
        // An event of the lock's name: the mutex open fails with ERROR_INVALID_HANDLE, so the guard cannot tell.
        let name = wide("CreateEventW", OsStr::new(&places.tray_lock)).unwrap();
        // SAFETY: `name` is NUL-terminated and lives for the call; the event is owned below.
        let event = unsafe { CreateEventW(None, false, false, PCWSTR(name.as_ptr())) }.unwrap();
        // SAFETY: the handle is new and owned only here.
        let event = unsafe { Owned::new(event) };
        // A Run value of another type does not read as a command.
        set_dword(&format!(r"{}\Run", places.auto.base), "Wakeshot");
        let signs = places.signs();
        println!("what does not read: {signs:?}");
        match signs.as_slice() {
            [
                RealSign::Unread {
                    what: lock,
                    error: a,
                },
                RealSign::Unread {
                    what: run,
                    error: b,
                },
            ] => {
                assert_eq!(*lock, format!("the tray lock {}", places.tray_lock));
                assert_eq!(a.call, "OpenMutexW");
                assert_eq!(a.code, ERROR_INVALID_HANDLE.0);
                assert_eq!(run, "the Run value Wakeshot");
                assert_eq!(b.call, "RegGetValueW");
            }
            other => panic!("two unread signs, got {other:?}"),
        }
        drop(event);
        assert_eq!(places.signs().len(), 1, "the lock reads again");
    }
}
