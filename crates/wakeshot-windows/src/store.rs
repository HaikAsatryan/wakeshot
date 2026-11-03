//! The data folder, atomic file writes, named mutexes, and the Run and RunOnce values (architecture sections 4 and 6).
//!
//! Owner: power track (`p3-store`). The signatures are contract: the sources and ship tracks use them.

use std::ffi::{OsStr, OsString};
use std::mem::offset_of;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use windows::Win32::Foundation::{
    DUPLICATE_SAME_ACCESS, DuplicateHandle, ERROR_ACCESS_DENIED, ERROR_FILE_NOT_FOUND,
    ERROR_INVALID_DATA, ERROR_INVALID_PARAMETER, ERROR_MORE_DATA, ERROR_PATH_NOT_FOUND,
    ERROR_SUCCESS, ERROR_WRITE_FAULT, GENERIC_READ, GENERIC_WRITE, GetLastError, HANDLE,
    WAIT_ABANDONED, WAIT_OBJECT_0, WAIT_TIMEOUT, WIN32_ERROR,
};
use windows::Win32::Storage::FileSystem::{
    CREATE_ALWAYS, CreateFileW, DELETE, DeleteFileW, FILE_ATTRIBUTE_NORMAL, FILE_RENAME_INFO,
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

fn local_app_data() -> Result<PathBuf, WinError> {
    // SAFETY: the folder id is a static; no token means the calling user.
    let path = unsafe { SHGetKnownFolderPath(&FOLDERID_LocalAppData, KF_FLAG_DEFAULT, None) }
        .map_err(|e| core_error("SHGetKnownFolderPath", &e))?;
    // SAFETY: on success the shell gives a NUL-terminated string it allocated; it is copied once, then freed once.
    let copy = unsafe { OsString::from_wide(path.as_wide()) };
    unsafe { CoTaskMemFree(Some(path.0 as *const _)) };
    Ok(PathBuf::from(copy))
}

/// Makes each temp name unique inside one process; the pid in the name makes it unique across processes.
static TEMP_COUNT: AtomicU64 = AtomicU64::new(0);

/// Writes `bytes` to a temp file beside `path` and moves it over `path`, write-through. The temp name is
/// `<name>.<pid>.<n>.tmp`, never ending in `.json`, so the watch thread never reads it as a hold. A missing folder is
/// made first. On failure the temp file is removed and `path` is as it was.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), WinError> {
    let target = wide("MoveFileExW", path.as_os_str())?;
    write_temp(path, bytes)?.replace(&target)
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
    let mut temp = path
        .file_name()
        .ok_or_else(|| win32("CreateFileW", ERROR_INVALID_PARAMETER))?
        .to_os_string();
    let n = TEMP_COUNT.fetch_add(1, Ordering::Relaxed);
    temp.push(format!(".{}.{n}.tmp", std::process::id()));
    let temp = TempFile {
        name: wide("CreateFileW", path.with_file_name(temp).as_os_str())?,
    };
    let file = match create_temp(&temp.name) {
        // The first write of a fresh install, or the CLI before any tray ran, finds no folder yet.
        Err(e) if e.code == ERROR_PATH_NOT_FOUND.0 => {
            if let Some(folder) = path.parent() {
                std::fs::create_dir_all(folder).map_err(|e| io_error("CreateDirectoryW", &e))?;
            }
            create_temp(&temp.name)?
        }
        other => other?,
    };
    let written = write_all(*file, bytes).and_then(|()| flush(*file));
    // Closed before the move: the temp file is open with no sharing, so the move would fail on it.
    drop(file);
    match written {
        Ok(()) => Ok(temp),
        Err(e) => {
            temp.discard();
            Err(e)
        }
    }
}

/// `CREATE_ALWAYS`: a temp file left by a stopped process whose pid came back is overwritten, not an error.
fn create_temp(name: &[u16]) -> Result<Owned<HANDLE>, WinError> {
    // SAFETY: `name` is NUL-terminated and lives for the call.
    let file = unsafe {
        CreateFileW(
            PCWSTR(name.as_ptr()),
            GENERIC_WRITE.0,
            FILE_SHARE_NONE,
            None,
            CREATE_ALWAYS,
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
        // SAFETY: the handle is new and owned only here.
        Ok(Self {
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

/// The Run and RunOnce values under one base key: the real one, or a test key.
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
        let subkey = self.subkey("RegDeleteKeyValueW", key)?;
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

/// Test registry keys under `HKCU\Software\Wakeshot-test\<pid>`, one rule for every test of this crate (review M8).
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
        assert_eq!(fs::read(dir.0.join(temp)).unwrap(), b"new");
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
}
