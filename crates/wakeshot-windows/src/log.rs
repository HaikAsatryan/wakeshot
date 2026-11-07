//! The text log in `logs\wakeshot.log` (architecture section 8).
//!
//! Owner: power track (`p3-log`); rolling while running, the folder cap and `log_files` by `p6-logs-diagnostics`. The
//! trait, the types and `MemoryLog` are contract: the sources, tray and ship tracks log through `Log` and test with
//! `MemoryLog`.
//!
//! A line reads `2026-11-03T18:55:12.345+04:00 [1234.567] tray    INFO  power: text`: local wall time with its UTC
//! offset, then the monotonic clock in seconds (awake time since boot), process, level, area and text.
//!
//! Every line opens the file, appends and closes it, so a roll by any process strands no writer on the old file. A
//! writer that finds the file over 1 MiB after its line rolls it to `wakeshot.<UTC time>.log` and prunes the folder:
//! rolled files older than 30 days go, and the oldest go while the logs take more than 8 MiB.

use std::os::windows::ffi::OsStrExt;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::path::{Path, PathBuf};
#[cfg(any(test, feature = "test-support"))]
use std::sync::Mutex;

use windows::Win32::Foundation::{
    ERROR_ALREADY_EXISTS, ERROR_FILE_EXISTS, ERROR_FILE_NOT_FOUND, ERROR_INVALID_PARAMETER,
    ERROR_PATH_NOT_FOUND, FILETIME, HANDLE, WIN32_ERROR,
};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, DELETE, FILE_APPEND_DATA, FILE_ATTRIBUTE_NORMAL, FILE_READ_ATTRIBUTES,
    FILE_RENAME_INFO, FILE_RENAME_INFO_0, FILE_SHARE_DELETE, FILE_SHARE_MODE, FILE_SHARE_READ,
    FILE_SHARE_WRITE, FileRenameInfo, FileTimeToLocalFileTime, GetFileSizeEx, OPEN_ALWAYS,
    OPEN_EXISTING, SetFileInformationByHandle, WriteFile,
};
use windows::core::HSTRING;

use crate::{WinError, clock};

/// Which process wrote a line.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Proc {
    Tray,
    Guard,
    Restore,
    Cli,
    Hook,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Level {
    Info,
    Warn,
    Error,
}

/// One log line. `area` names the part that writes it (`core`, `power`, `watch`, ...); `text` is plain English and
/// never holds prompts or file content (architecture section 8). Callers may rely on `line` never failing: a write
/// that fails is dropped.
pub trait Log: Send + Sync {
    fn line(&self, level: Level, area: &'static str, text: &str);
}

/// The file names of track note 9: the `.log` extension keeps "Open log" on the user's default app.
const LOG_FILE: &str = "wakeshot.log";

/// What builds before `p6-logs-diagnostics` rolled to. The prune counts them as the oldest rolled files, so an update
/// cleans them up like its own.
const LEGACY_TWICE: &str = "wakeshot.2.log";
const LEGACY_ONCE: &str = "wakeshot.1.log";

/// A rolled file is `wakeshot.` + this shape + `.log`, the UTC time of its roll: `d` a digit, anything else itself.
/// UTC and a fixed width, so name order is roll order, also across a daylight saving change.
const STAMP_SHAPE: &str = "dddd-dd-ddTdd-dd-dd.dddZ";

/// The sizes and age the folder keeps (architecture section 8). Tests pass smaller ones.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Limits {
    /// "Over 1 MB", read as 1 MiB: a file over this rolls.
    pub roll_over: u64,
    /// The logs, the current file included, take at most this; the oldest rolled files go first.
    pub folder_max: u64,
    /// A rolled file last written longer ago than this goes.
    pub max_age_ms: i64,
}

pub(crate) const LIMITS: Limits = Limits {
    roll_over: 1024 * 1024,
    folder_max: 8 * 1024 * 1024,
    max_age_ms: 30 * MS_PER_DAY,
};

/// Every process opens the file with this, so no open, write or roll in one process locks out another.
const SHARE_ALL: FILE_SHARE_MODE =
    FILE_SHARE_MODE(FILE_SHARE_READ.0 | FILE_SHARE_WRITE.0 | FILE_SHARE_DELETE.0);

/// 100 ns FILETIME ticks per millisecond.
const TICKS_PER_MS: i64 = 10_000;

/// Milliseconds from 1601-01-01 (the FILETIME start) to 1970-01-01.
const FILETIME_TO_UNIX_MS: i64 = 11_644_473_600_000;

const MS_PER_DAY: i64 = 86_400_000;

/// The file every process appends to: `wakeshot.log` in `logs_dir` (`DataDir::logs`). Open log, the balloon click and
/// the uninstall hook's last delete name it through this, so they never miss the file `LogWriter` writes.
pub fn log_file(logs_dir: &Path) -> PathBuf {
    logs_dir.join(LOG_FILE)
}

/// Every log file Wakeshot wrote in `logs_dir`, oldest first: the rolled files in roll order, then `wakeshot.log`.
/// Other files in the folder are never listed. For the diagnostics bundle.
pub fn log_files(logs_dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = rolled_files(logs_dir)
        .into_iter()
        .map(|f| logs_dir.join(f.name))
        .collect();
    let current = log_file(logs_dir);
    if current.is_file() {
        files.push(current);
    }
    files
}

/// Appends to the log file, shared by every process.
#[derive(Debug)]
pub struct LogWriter {
    dir: PathBuf,
    proc: Proc,
    limits: Limits,
}

impl LogWriter {
    /// Opens `wakeshot.log` in `logs_dir` (`DataDir::logs`) once, creating both if needed, so a folder that takes no
    /// file fails here and the caller can choose a log that drops every line.
    pub fn open(logs_dir: &Path, proc: Proc) -> Result<Self, WinError> {
        Self::open_with(logs_dir, proc, LIMITS)
    }

    pub(crate) fn open_with(logs_dir: &Path, proc: Proc, limits: Limits) -> Result<Self, WinError> {
        std::fs::create_dir_all(logs_dir).map_err(|e| io_error("CreateDirectoryW", &e))?;
        open_append(&log_file(logs_dir))?;
        Ok(Self {
            dir: logs_dir.to_path_buf(),
            proc,
            limits,
        })
    }

    /// Rolls `wakeshot.log` in `logs_dir` when it is over 1 MiB, then prunes the folder, so a start after a long break
    /// drops old files at once. Every line makes the same size check, so a running process rolls too. An `Err` leaves
    /// every line in a file; the next line over the size tries again.
    pub fn roll(logs_dir: &Path) -> Result<(), WinError> {
        roll_with(logs_dir, LIMITS, clock::now().wall_ms, true)
    }
}

impl Log for LogWriter {
    fn line(&self, level: Level, area: &'static str, text: &str) {
        let line = format_line(stamp_now(), self.proc, level, area, text);
        // WriteFile panics on a buffer over 4 GiB, and a panic aborts the caller.
        if u32::try_from(line.len()).is_err() {
            return;
        }
        // A folder deleted under the writer (the uninstall hook) drops the line: the open never creates the folder.
        let Ok(file) = open_append(&log_file(&self.dir)) else {
            return;
        };
        let handle = HANDLE(file.as_raw_handle());
        let mut written = 0u32;
        // A failed or short write drops the line: a retry could split it, and the log must never stop its caller.
        // SAFETY: the handle is open for the call; the buffer and the count are live locals.
        let wrote = unsafe { WriteFile(handle, Some(line.as_bytes()), Some(&mut written), None) };
        let size = handle_size(handle);
        drop(file);
        if wrote.is_ok() && size.is_ok_and(|size| size > self.limits.roll_over) {
            // No prune when the roll did not happen: a reader that blocks the rename would make every line list the
            // folder.
            let _ = roll_with(&self.dir, self.limits, clock::now().wall_ms, false);
        }
    }
}

/// Opens for append only: append rights without FILE_WRITE_DATA make every write land at the end of the file, so
/// another process's lines are never overwritten (Microsoft, "File Access Rights Constants"). std's append mode is not
/// documented to use exactly this mask, so the handle is opened here. Attribute rights read the size for the roll.
fn open_append(path: &Path) -> Result<OwnedHandle, WinError> {
    let name = HSTRING::from(path);
    // SAFETY: the name is a live, NUL-terminated wide string; no security attributes and no template are passed.
    let handle = unsafe {
        CreateFileW(
            &name,
            FILE_APPEND_DATA.0 | FILE_READ_ATTRIBUTES.0,
            SHARE_ALL,
            None,
            OPEN_ALWAYS,
            FILE_ATTRIBUTE_NORMAL,
            None,
        )
    }
    .map_err(|e| win_error("CreateFileW", &e))?;
    // SAFETY: CreateFileW just returned this handle and nothing else owns it, so OwnedHandle may close it.
    Ok(unsafe { OwnedHandle::from_raw_handle(handle.0) })
}

/// The roll, then the prune when `always_prune` or the roll moved a file. The size is checked and the file renamed
/// through one handle, so two processes that both saw the file over the size never roll twice: the second finds the
/// new, small `wakeshot.log`, or renames the same file its handle holds. No lock is needed.
fn roll_with(dir: &Path, limits: Limits, now_ms: i64, always_prune: bool) -> Result<(), WinError> {
    let rolled = roll_current(dir, limits, now_ms);
    if always_prune || rolled == Ok(true) {
        prune(dir, limits, now_ms);
    }
    rolled.map(|_| ())
}

/// `Ok(true)`: `wakeshot.log` was over the size and now has its rolled name.
fn roll_current(dir: &Path, limits: Limits, now_ms: i64) -> Result<bool, WinError> {
    let name = HSTRING::from(log_file(dir).as_path());
    // SAFETY: the name is a live, NUL-terminated wide string; no security attributes and no template are passed.
    let opened = unsafe {
        CreateFileW(
            &name,
            DELETE.0 | FILE_READ_ATTRIBUTES.0,
            SHARE_ALL,
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            None,
        )
    };
    let handle = match opened {
        Ok(handle) => handle,
        Err(e) => {
            let error = win_error("CreateFileW", &e);
            return if is_not_found(error.code) {
                Ok(false)
            } else {
                Err(error)
            };
        }
    };
    // SAFETY: CreateFileW just returned this handle and nothing else owns it, so OwnedHandle may close it.
    let file = unsafe { OwnedHandle::from_raw_handle(handle.0) };
    let handle = HANDLE(file.as_raw_handle());
    if handle_size(handle)? <= limits.roll_over {
        return Ok(false);
    }
    let target = dir.join(rolled_name(now_ms));
    match rename_no_replace(handle, &target) {
        Ok(()) => Ok(true),
        // Another process rolled to the same name in the same millisecond.
        Err(e) if e.code == ERROR_ALREADY_EXISTS.0 || e.code == ERROR_FILE_EXISTS.0 => Ok(false),
        Err(e) => Err(e),
    }
}

/// `wakeshot.<UTC time of the roll>.log`, such as `wakeshot.2026-11-07T00-12-33.512Z.log`.
fn rolled_name(utc_ms: i64) -> String {
    let (year, month, day) = civil_from_days(utc_ms.div_euclid(MS_PER_DAY));
    let of_day = utc_ms.rem_euclid(MS_PER_DAY);
    format!(
        "wakeshot.{year:04}-{month:02}-{day:02}T{:02}-{:02}-{:02}.{:03}Z.log",
        of_day / 3_600_000,
        of_day / 60_000 % 60,
        of_day / 1000 % 60,
        of_day % 1000
    )
}

/// The rolled file's place in roll order: the legacy names first, the older one before the newer, then stamps by
/// name. `None`: not a file this module rolled, so it is never listed or deleted.
fn roll_order(name: &str) -> Option<(u8, String)> {
    if name.eq_ignore_ascii_case(LEGACY_TWICE) {
        return Some((0, String::new()));
    }
    if name.eq_ignore_ascii_case(LEGACY_ONCE) {
        return Some((1, String::new()));
    }
    let stamp = name.strip_prefix("wakeshot.")?.strip_suffix(".log")?;
    let shaped = stamp.len() == STAMP_SHAPE.len()
        && stamp
            .bytes()
            .zip(STAMP_SHAPE.bytes())
            .all(|(c, s)| match s {
                b'd' => c.is_ascii_digit(),
                _ => c == s,
            });
    shaped.then(|| (2, stamp.to_owned()))
}

/// One rolled file as the prune sees it.
#[derive(Debug)]
struct Rolled {
    name: String,
    size: u64,
    /// Last write as Unix ms; `None` when it does not read, and then age never deletes it.
    written_ms: Option<i64>,
}

/// The rolled files in `dir`, oldest first. Links and folders are left out: the prune deletes only files it rolled.
fn rolled_files(dir: &Path) -> Vec<Rolled> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<((u8, String), Rolled)> = entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            let order = roll_order(&name)?;
            // By path, not from the folder listing, which can lag for a file another process still writes.
            let meta = std::fs::symlink_metadata(entry.path()).ok()?;
            if !meta.is_file() {
                return None;
            }
            let written_ms = meta.modified().ok().and_then(|t| {
                let since = t.duration_since(std::time::UNIX_EPOCH).ok()?;
                i64::try_from(since.as_millis()).ok()
            });
            Some((
                order,
                Rolled {
                    name,
                    size: meta.len(),
                    written_ms,
                },
            ))
        })
        .collect();
    files.sort_by(|a, b| a.0.cmp(&b.0));
    files.into_iter().map(|(_, file)| file).collect()
}

/// Deletes the rolled files that are too old, and the oldest while the logs take more than the folder size, newest
/// kept first. The current file always stays. A file that cannot go now (a reader without delete sharing) is tried at
/// the next roll; two processes that prune at once only find a file already gone.
fn prune(dir: &Path, limits: Limits, now_ms: i64) {
    let current = std::fs::metadata(log_file(dir)).map_or(0, |m| m.len());
    let mut total = current;
    let mut full = false;
    for file in rolled_files(dir).into_iter().rev() {
        let old = file
            .written_ms
            .is_some_and(|at| now_ms.saturating_sub(at) > limits.max_age_ms);
        full = full || total.saturating_add(file.size) > limits.folder_max;
        if old || full {
            let _ = std::fs::remove_file(dir.join(&file.name));
        } else {
            total += file.size;
        }
    }
}

/// When a line was written. Local time and its offset come from one conversion, so local time minus the offset is
/// always the exact UTC instant, even if the offset were a moment late after a daylight saving switch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Stamp {
    /// Local wall time, counted like Unix milliseconds.
    local_ms: i64,
    /// Local minus UTC.
    offset_min: i32,
    mono_ms: u64,
}

/// Reads the clocks for every line: a start time plus a counter drifted 16 s in the tray spike (research).
fn stamp_now() -> Stamp {
    let now = clock::now();
    let utc_ticks = (now.wall_ms + FILETIME_TO_UNIX_MS) * TICKS_PER_MS;
    // Without the local time the line still names the right instant, in UTC.
    let offset_min = to_local_ticks(utc_ticks).map_or(0, |local| offset_minutes(utc_ticks, local));
    Stamp {
        local_ms: now.wall_ms + i64::from(offset_min) * 60_000,
        offset_min,
        mono_ms: now.mono_ms,
    }
}

/// A UTC instant as local wall time, for texts outside log lines (the diagnostics bundle). The offset is today's, as
/// for a line (`to_local_ticks`); without one the time shows in UTC.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LocalTime {
    local_ms: i64,
    offset_min: i32,
}

impl LocalTime {
    pub(crate) fn of(utc_ms: i64) -> Self {
        let utc_ticks = (utc_ms + FILETIME_TO_UNIX_MS) * TICKS_PER_MS;
        let offset_min =
            to_local_ticks(utc_ticks).map_or(0, |local| offset_minutes(utc_ticks, local));
        Self {
            local_ms: utc_ms + i64::from(offset_min) * 60_000,
            offset_min,
        }
    }

    /// `2026-11-07T04:12:33+04:00`, the log's form without the milliseconds.
    pub(crate) fn text(self) -> String {
        let line = format_line(
            Stamp {
                local_ms: self.local_ms,
                offset_min: self.offset_min,
                mono_ms: 0,
            },
            Proc::Cli,
            Level::Info,
            "",
            "",
        );
        format!("{}{}", &line[..19], &line[23..29])
    }

    /// `20261107-041233`, for a file name.
    pub(crate) fn compact(self) -> String {
        let text = self.text();
        let digits: String = text[..19].chars().filter(char::is_ascii_digit).collect();
        format!("{}-{}", &digits[..8], &digits[8..])
    }
}

/// Microsoft points to `SystemTimeToTzSpecificLocalTime` for daylight saving, which matters for a past time; a line
/// converts the time now. If the bias were stale for a moment after a switch, the line still names the right instant
/// (`Stamp`).
fn to_local_ticks(utc_ticks: i64) -> Option<i64> {
    let utc = FILETIME {
        dwLowDateTime: utc_ticks as u32,
        dwHighDateTime: (utc_ticks >> 32) as u32,
    };
    let mut local = FILETIME::default();
    // SAFETY: both pointers are to live locals, and they differ, as the call requires.
    unsafe { FileTimeToLocalFileTime(&utc, &mut local) }.ok()?;
    Some((i64::from(local.dwHighDateTime) << 32) | i64::from(local.dwLowDateTime))
}

fn offset_minutes(utc_ticks: i64, local_ticks: i64) -> i32 {
    let minutes = (local_ticks - utc_ticks) / (TICKS_PER_MS * 60_000);
    // Real offsets are within 14 hours; anything else is a broken clock, shown as UTC.
    i32::try_from(minutes).unwrap_or(0)
}

fn format_line(stamp: Stamp, proc: Proc, level: Level, area: &str, text: &str) -> String {
    let (year, month, day) = civil_from_days(stamp.local_ms.div_euclid(MS_PER_DAY));
    let of_day = stamp.local_ms.rem_euclid(MS_PER_DAY);
    let (hour, minute) = (of_day / 3_600_000, of_day / 60_000 % 60);
    let (second, milli) = (of_day / 1000 % 60, of_day % 1000);
    let sign = if stamp.offset_min < 0 { '-' } else { '+' };
    let offset = stamp.offset_min.unsigned_abs();
    let mut line = format!(
        "{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{milli:03}{sign}{:02}:{:02} [{}.{:03}] {:<7} {:<5} ",
        offset / 60,
        offset % 60,
        stamp.mono_ms / 1000,
        stamp.mono_ms % 1000,
        proc_name(proc),
        level_name(level),
    );
    push_on_one_line(&mut line, area);
    line.push_str(": ");
    push_on_one_line(&mut line, text);
    // CRLF: "Open log" opens the user's default app for `.log`, which may be an editor that shows only CRLF breaks.
    line.push_str("\r\n");
    line
}

/// One call is one line: CR and LF become spaces, and so does every other control character and the Unicode line and
/// paragraph separators, because some editors break lines at NEL, VT, FF or U+2028. So a hold's `reason` cannot forge
/// a line, and an escape sequence cannot hide text in a terminal.
fn push_on_one_line(line: &mut String, text: &str) {
    line.extend(text.chars().map(|c| {
        if c.is_control() || c == '\u{2028}' || c == '\u{2029}' {
            ' '
        } else {
            c
        }
    }));
}

fn proc_name(proc: Proc) -> &'static str {
    match proc {
        Proc::Tray => "tray",
        Proc::Guard => "guard",
        Proc::Restore => "restore",
        Proc::Cli => "cli",
        Proc::Hook => "hook",
    }
}

fn level_name(level: Level) -> &'static str {
    match level {
        Level::Info => "INFO",
        Level::Warn => "WARN",
        Level::Error => "ERROR",
    }
}

/// Year, month and day of a day count from 1970-01-01 (Howard Hinnant's `civil_from_days`), so the log needs no date
/// crate.
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

/// The size of the file a handle holds, while other processes may be appending.
fn handle_size(handle: HANDLE) -> Result<u64, WinError> {
    let mut size = 0i64;
    // SAFETY: the caller keeps the handle open for the call; the pointer is to a live local.
    unsafe { GetFileSizeEx(handle, &mut size) }.map_err(|e| win_error("GetFileSizeEx", &e))?;
    Ok(u64::try_from(size).unwrap_or(0))
}

/// Renames the file `handle` holds to `target`, failing when `target` exists. The handle needs `DELETE`.
fn rename_no_replace(handle: HANDLE, target: &Path) -> Result<(), WinError> {
    const CALL: &str = "SetFileInformationByHandle";
    let too_long = || WinError {
        call: CALL,
        code: ERROR_INVALID_PARAMETER.0,
    };
    let name: Vec<u16> = target.as_os_str().encode_wide().collect();
    let name_bytes = name.len() * size_of::<u16>();
    // The name's NUL, which the length leaves out, stays in the buffer.
    let size = std::mem::offset_of!(FILE_RENAME_INFO, FileName) + name_bytes + size_of::<u16>();
    let name_len = u32::try_from(name_bytes).map_err(|_| too_long())?;
    let size_len = u32::try_from(size).map_err(|_| too_long())?;
    // u64 units keep the struct's handle field aligned.
    let mut buf = vec![0u64; size.div_ceil(size_of::<u64>())];
    let info = buf.as_mut_ptr().cast::<FILE_RENAME_INFO>();
    // SAFETY: `buf` is zeroed, aligned and at least `size` bytes long, so the header and the whole name fit in it. A
    // zero union is `ReplaceIfExists` false.
    unsafe {
        (&raw mut (*info).Anonymous).write(FILE_RENAME_INFO_0 { Flags: 0 });
        (&raw mut (*info).FileNameLength).write(name_len);
        name.as_ptr()
            .copy_to_nonoverlapping((&raw mut (*info).FileName).cast::<u16>(), name.len());
    }
    // SAFETY: an open handle with `DELETE`; `info` points at `size_len` filled bytes.
    unsafe { SetFileInformationByHandle(handle, FileRenameInfo, info.cast(), size_len) }
        .map_err(|e| win_error(CALL, &e))
}

fn is_not_found(code: u32) -> bool {
    code == ERROR_FILE_NOT_FOUND.0 || code == ERROR_PATH_NOT_FOUND.0
}

/// The Win32 error when the HRESULT wraps one, so codes read the same as Microsoft's error tables.
fn win_error(call: &'static str, error: &windows::core::Error) -> WinError {
    WinError {
        call,
        code: WIN32_ERROR::from_error(error).map_or(error.code().0 as u32, |e| e.0),
    }
}

/// A std error without an OS code (a NUL in the path) gets `u32::MAX`, which no Win32 error uses.
fn io_error(call: &'static str, error: &std::io::Error) -> WinError {
    WinError {
        call,
        code: error.raw_os_error().map_or(u32::MAX, |c| c as u32),
    }
}

/// Keeps every line in memory, for tests.
#[cfg(any(test, feature = "test-support"))]
#[derive(Debug, Default)]
pub struct MemoryLog {
    pub lines: Mutex<Vec<(Level, &'static str, String)>>,
}

#[cfg(any(test, feature = "test-support"))]
impl Log for MemoryLog {
    fn line(&self, level: Level, area: &'static str, text: &str) {
        self.lines
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push((level, area, text.to_owned()));
    }
}

#[cfg(test)]
mod tests {
    use std::fs::{self, File, OpenOptions};
    use std::os::windows::fs::OpenOptionsExt;
    use std::path::PathBuf;
    use std::process::{Command, Stdio};
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::time::Duration;

    use windows::Win32::System::SystemServices::{TIME_ZONE_ID_DAYLIGHT, TIME_ZONE_ID_STANDARD};
    use windows::Win32::System::Time::{
        DYNAMIC_TIME_ZONE_INFORMATION, GetDynamicTimeZoneInformation,
    };

    use super::*;

    /// A fresh folder under the temp folder, removed on drop.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(name: &str) -> Self {
            static NEXT: AtomicU32 = AtomicU32::new(0);
            let n = NEXT.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "wakeshot-log-{name}-{}-{n}-{}",
                std::process::id(),
                clock::now().mono_ms
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }

        fn file(&self, name: &str) -> PathBuf {
            self.0.join(name)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn stamp(utc_ms: i64, offset_min: i32, mono_ms: u64) -> Stamp {
        Stamp {
            local_ms: utc_ms + i64::from(offset_min) * 60_000,
            offset_min,
            mono_ms,
        }
    }

    /// Fixtures from Python's `datetime`: UTC Unix ms, offset, expected local time.
    const TIMES: [(i64, i32, &str); 7] = [
        (1_793_717_712_345, 240, "2026-11-03T18:55:12.345+04:00"),
        (1_767_232_800_007, -210, "2025-12-31T22:30:00.007-03:30"),
        (1_835_461_200_000, 345, "2028-03-01T00:05:00.000+05:45"),
        (1_709_269_200_999, -600, "2024-02-29T19:00:00.999-10:00"),
        (1_767_225_600_000, 0, "2026-01-01T00:00:00.000+00:00"),
        (1_767_225_600_000, -30, "2025-12-31T23:30:00.000-00:30"),
        (4_107_540_600_000, 60, "2100-03-01T00:30:00.000+01:00"),
    ];

    #[test]
    fn line_format() {
        for (utc_ms, offset, local) in TIMES {
            let line = format_line(
                stamp(utc_ms, offset, 1_234_567),
                Proc::Tray,
                Level::Info,
                "power",
                "hold",
            );
            assert_eq!(
                line,
                format!("{local} [1234.567] tray    INFO  power: hold\r\n")
            );
        }

        let at = stamp(1_793_717_712_345, 240, 5);
        let cases = [
            (Proc::Guard, Level::Warn, "[0.005] guard   WARN "),
            (Proc::Restore, Level::Error, "[0.005] restore ERROR"),
            (Proc::Cli, Level::Info, "[0.005] cli     INFO "),
            (Proc::Hook, Level::Info, "[0.005] hook    INFO "),
        ];
        for (proc, level, middle) in cases {
            let line = format_line(at, proc, level, "core", "x");
            assert_eq!(
                line,
                format!("2026-11-03T18:55:12.345+04:00 {middle} core: x\r\n")
            );
        }

        let forged =
            "nightly backup\r\n2026-11-03T18:55:12.345+04:00 [1.000] tray    ERROR power: fake";
        let line = format_line(at, Proc::Cli, Level::Info, "watch", forged);
        assert_eq!(
            line,
            "2026-11-03T18:55:12.345+04:00 [0.005] cli     INFO  watch: nightly backup  \
             2026-11-03T18:55:12.345+04:00 [1.000] tray    ERROR power: fake\r\n"
        );
        let line = format_line(
            at,
            Proc::Cli,
            Level::Info,
            "a\nb",
            "c\rd\ne\tf\u{b}g\u{c}h\u{85}i\u{2028}j\u{2029}k\u{1b}l",
        );
        assert!(line.ends_with(" a b: c d e f g h i j k l\r\n"), "{line:?}");
        let breaks = line.char_indices().filter(|&(_, c)| c == '\r' || c == '\n');
        assert_eq!(
            breaks.map(|(i, _)| i).collect::<Vec<_>>(),
            [line.len() - 2, line.len() - 1]
        );
        let line = format_line(at, Proc::Cli, Level::Info, "core", "");
        assert!(line.ends_with(" INFO  core: \r\n"), "{line:?}");
    }

    #[test]
    fn local_time_texts() {
        let at = LocalTime {
            local_ms: 1_793_717_712_345 + 240 * 60_000,
            offset_min: 240,
        };
        assert_eq!(at.text(), "2026-11-03T18:55:12+04:00");
        assert_eq!(at.compact(), "20261103-185512");
        let west = LocalTime {
            local_ms: 1_767_232_800_007 - 210 * 60_000,
            offset_min: -210,
        };
        assert_eq!(west.text(), "2025-12-31T22:30:00-03:30");
    }

    #[test]
    fn offset_comes_from_the_two_file_times() {
        let utc = 134_381_913_120_000_000;
        let hour = TICKS_PER_MS * 3_600_000;
        assert_eq!(offset_minutes(utc, utc + 4 * hour), 240);
        assert_eq!(offset_minutes(utc, utc - 3 * hour - hour / 2), -210);
        assert_eq!(offset_minutes(utc, utc), 0);
    }

    #[test]
    fn local_offset_matches_windows() {
        let mut zone = DYNAMIC_TIME_ZONE_INFORMATION::default();
        // SAFETY: the pointer is to a live local.
        let id = unsafe { GetDynamicTimeZoneInformation(&mut zone) };
        let bias = match id {
            TIME_ZONE_ID_DAYLIGHT => zone.Bias + zone.DaylightBias,
            TIME_ZONE_ID_STANDARD => zone.Bias + zone.StandardBias,
            _ => zone.Bias,
        };
        let now = stamp_now();
        assert_eq!(now.offset_min, -bias, "zone id {id}");
        let line = format_line(now, Proc::Cli, Level::Info, "test", "offset");
        // Compared by hand with `Get-Date -Format zzz` (`--nocapture`).
        println!("log offset {}", &line[23..29]);
    }

    #[test]
    fn open_creates_the_folder_and_appends() {
        let dir = TempDir::new("open");
        let logs = dir.file("data").join("logs");
        LogWriter::open(&logs, Proc::Tray)
            .unwrap()
            .line(Level::Info, "core", "first");
        LogWriter::open(&logs, Proc::Cli)
            .unwrap()
            .line(Level::Warn, "core", "second");
        // The file the other modules name through `log_file` is the one written here.
        let path = log_file(&logs);
        assert_eq!(path, logs.join("wakeshot.log"));
        let text = fs::read_to_string(path).unwrap();
        let lines: Vec<_> = text.split_terminator("\r\n").collect();
        assert_eq!(lines.len(), 2, "{text:?}");
        assert!(lines[0].ends_with(" tray    INFO  core: first"), "{text:?}");
        assert!(
            lines[1].ends_with(" cli     WARN  core: second"),
            "{text:?}"
        );
    }

    #[test]
    fn failed_write_is_dropped() {
        let dir = TempDir::new("fail");
        let writer = LogWriter::open(dir.path(), Proc::Tray).unwrap();
        // A handle that shares nothing makes every open of the line fail.
        let locked = OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(dir.file(LOG_FILE))
            .unwrap();
        writer.line(Level::Info, "core", "dropped");
        writer.line(Level::Info, "core", "dropped too");
        drop(locked);
        assert_eq!(fs::read(dir.file(LOG_FILE)).unwrap(), b"");

        // A folder deleted under the writer, as the uninstall hook does: the line goes nowhere and brings nothing back.
        fs::remove_dir_all(dir.path()).unwrap();
        writer.line(Level::Info, "core", "after the delete");
        assert!(!dir.path().exists());
    }

    const CHILD_ROLE: &str = "WAKESHOT_LOG_TEST_CHILD";
    const CHILDREN: usize = 4;
    const LINES_PER_CHILD: usize = 2000;
    const CHILD_PROCS: [Proc; CHILDREN] = [Proc::Tray, Proc::Guard, Proc::Restore, Proc::Cli];

    /// Mostly short lines, and every 50th one long enough to cross a 4 KiB page.
    fn payload_len(child: usize, seq: usize) -> usize {
        if seq % 50 == 7 {
            3000 + (seq * 37 + child * 11) % 2200
        } else {
            (seq * 31 + child * 17) % 180
        }
    }

    fn child_mark(child: usize) -> char {
        char::from(b'a' + child as u8)
    }

    /// `d` is a digit, `s` a sign; anything else must match itself.
    fn has_shape(text: &str, shape: &str) -> bool {
        text.len() == shape.len()
            && text.bytes().zip(shape.bytes()).all(|(t, s)| match s {
                b'd' => t.is_ascii_digit(),
                b's' => t == b'+' || t == b'-',
                _ => t == s,
            })
    }

    /// The child and its sequence number, when the line is whole and well formed.
    fn parse_child_line(line: &str) -> Option<(usize, usize)> {
        let stamp = line.get(..29)?;
        if !has_shape(stamp, "dddd-dd-ddTdd:dd:dd.dddsdd:dd") {
            return None;
        }
        let (mono, rest) = line.get(29..)?.strip_prefix(" [")?.split_once("] ")?;
        let (secs, millis) = mono.split_once('.')?;
        if secs.is_empty() || !secs.bytes().all(|b| b.is_ascii_digit()) || !has_shape(millis, "ddd")
        {
            return None;
        }
        let (proc, rest) = (rest.get(..8)?, rest.get(8..)?);
        let rest = rest.strip_prefix("INFO  mix: child=")?;
        let (child, rest) = rest.split_once(" seq=")?;
        let child: usize = child.parse().ok().filter(|&c| c < CHILDREN)?;
        let (seq, rest) = rest.split_once(" len=")?;
        let seq: usize = seq.parse().ok()?;
        let (len, rest) = rest.split_once(' ')?;
        let len: usize = len.parse().ok()?;
        let payload = rest.strip_suffix(" end")?;
        let whole = proc == format!("{:<7} ", proc_name(CHILD_PROCS[child]))
            && len == payload_len(child, seq)
            && payload.len() == len
            && payload.chars().all(|c| c == child_mark(child));
        whole.then_some((child, seq))
    }

    fn run_append_child(role: &str) {
        let mut parts = role.splitn(4, '|');
        let child: usize = parts.next().unwrap().parse().unwrap();
        let start_ms: i64 = parts.next().unwrap().parse().unwrap();
        let roll_over: u64 = parts.next().unwrap().parse().unwrap();
        let dir = PathBuf::from(parts.next().unwrap());
        let limits = Limits {
            roll_over,
            ..LIMITS
        };
        let log = LogWriter::open_with(&dir, CHILD_PROCS[child], limits).unwrap();
        // One start time for all children, so their writes overlap: sleep, then spin the last 20 ms.
        while clock::now().wall_ms < start_ms - 20 {
            std::thread::sleep(Duration::from_millis(1));
        }
        while clock::now().wall_ms < start_ms {
            std::hint::spin_loop();
        }
        let mark = child_mark(child).to_string();
        for seq in 0..LINES_PER_CHILD {
            let len = payload_len(child, seq);
            let text = format!("child={child} seq={seq} len={len} {} end", mark.repeat(len));
            log.line(Level::Info, "mix", &text);
        }
    }

    /// Four writer processes at once, each rolling at `roll_over`; every line must come back whole, once and in its
    /// writer's order from the files in `log_files` order. Gives the number of files.
    fn four_writers(test: &str, roll_over: u64) -> usize {
        if let Ok(role) = std::env::var(CHILD_ROLE) {
            run_append_child(&role);
            return 0;
        }
        let dir = TempDir::new("mix");
        let start_ms = clock::now().wall_ms + 1500;
        let exe = std::env::current_exe().unwrap();
        let children: Vec<_> = (0..CHILDREN)
            .map(|child| {
                Command::new(&exe)
                    .args([test, "--exact"])
                    .env(
                        CHILD_ROLE,
                        format!("{child}|{start_ms}|{roll_over}|{}", dir.path().display()),
                    )
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()
                    .unwrap()
            })
            .collect();
        for child in children {
            let out = child.wait_with_output().unwrap();
            assert!(
                out.status.success(),
                "child failed: {}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
        }

        let files = log_files(dir.path());
        let mut next = [0usize; CHILDREN];
        let (mut switches, mut last, mut bytes) = (0, None, 0);
        for file in &files {
            let text = String::from_utf8(fs::read(file).unwrap())
                .expect("a line was cut inside a character");
            assert!(text.ends_with("\r\n"), "a last line is cut in {file:?}");
            bytes += text.len();
            for line in text.split_terminator("\r\n") {
                let shown: String = line.chars().take(120).collect();
                let (child, seq) =
                    parse_child_line(line).unwrap_or_else(|| panic!("broken line: {shown}"));
                assert_eq!(seq, next[child], "child {child} out of order at: {shown}");
                next[child] += 1;
                if last != Some(child) {
                    switches += 1;
                    last = Some(child);
                }
            }
        }
        assert_eq!(next, [LINES_PER_CHILD; CHILDREN]);
        // Without one switch more than the child count, the children never wrote at the same time.
        assert!(
            switches > CHILDREN,
            "only {switches} switches between children"
        );
        println!(
            "{bytes} bytes in {} files, {switches} switches between children",
            files.len()
        );
        files.len()
    }

    #[test]
    fn appends_from_many_processes_never_mix() {
        four_writers(
            "log::tests::appends_from_many_processes_never_mix",
            LIMITS.roll_over,
        );
    }

    /// The four-writer rule with a roll every 64 KiB: about 20 rolls while all four write.
    #[test]
    fn four_writers_roll_often_and_lose_nothing() {
        let files = four_writers(
            "log::tests::four_writers_roll_often_and_lose_nothing",
            64 * 1024,
        );
        if std::env::var(CHILD_ROLE).is_err() {
            assert!(files > 10, "only {files} files");
        }
    }

    fn write_big(path: &Path, fill: u8) {
        fs::write(path, vec![fill; LIMITS.roll_over as usize + 1]).unwrap();
    }

    fn first_byte(path: &Path) -> Option<u8> {
        fs::read(path).ok()?.first().copied()
    }

    fn names(dir: &TempDir) -> Vec<String> {
        let mut names: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        names
    }

    /// The rolled names in `dir`, in roll order.
    fn rolled(dir: &TempDir) -> Vec<String> {
        rolled_files(dir.path())
            .into_iter()
            .map(|f| f.name)
            .collect()
    }

    #[test]
    fn rolled_names_sort_in_roll_order() {
        assert_eq!(
            rolled_name(1_793_717_712_345),
            "wakeshot.2026-11-03T14-55-12.345Z.log"
        );
        assert_eq!(
            rolled_name(1_767_225_600_000),
            "wakeshot.2026-01-01T00-00-00.000Z.log"
        );
        let mut names = [
            rolled_name(1_793_717_712_346),
            "wakeshot.1.log".to_owned(),
            rolled_name(1_767_225_600_000),
            "WAKESHOT.2.LOG".to_owned(),
            rolled_name(1_793_717_712_345),
        ];
        names.sort_by_key(|n| roll_order(n).unwrap());
        assert_eq!(
            names,
            [
                "WAKESHOT.2.LOG".to_owned(),
                "wakeshot.1.log".to_owned(),
                rolled_name(1_767_225_600_000),
                rolled_name(1_793_717_712_345),
                rolled_name(1_793_717_712_346),
            ]
        );
        for other in [
            LOG_FILE,
            "wakeshot.3.log",
            "wakeshot.notes.log",
            "wakeshot.2026-11-03T14-55-12.345Z.txt",
            "wakeshot.2026-11-03T14-55-12.34Z.log",
            "other.log",
        ] {
            assert_eq!(roll_order(other), None, "{other}");
        }
    }

    #[test]
    fn roll_moves_the_big_file_and_keeps_the_rest() {
        let dir = TempDir::new("roll");
        LogWriter::roll(&dir.file("missing")).unwrap();
        LogWriter::roll(dir.path()).unwrap();
        assert!(names(&dir).is_empty());

        fs::write(dir.file(LOG_FILE), vec![b'0'; LIMITS.roll_over as usize]).unwrap();
        LogWriter::roll(dir.path()).unwrap();
        assert_eq!(names(&dir), [LOG_FILE], "exactly 1 MiB is not over");

        write_big(&dir.file(LOG_FILE), b'A');
        LogWriter::roll(dir.path()).unwrap();
        assert_eq!(rolled(&dir).len(), 1);
        assert!(!dir.file(LOG_FILE).exists());

        // A millisecond apart, so the second roll gets its own name.
        std::thread::sleep(Duration::from_millis(2));
        write_big(&dir.file(LOG_FILE), b'B');
        LogWriter::roll(dir.path()).unwrap();
        LogWriter::open(dir.path(), Proc::Tray)
            .unwrap()
            .line(Level::Info, "core", "start");
        let files = log_files(dir.path());
        assert_eq!(files.len(), 3, "{files:?}");
        assert_eq!(first_byte(&files[0]), Some(b'A'));
        assert_eq!(first_byte(&files[1]), Some(b'B'));
        assert_eq!(files[2], dir.file(LOG_FILE));
        assert!(
            fs::read_to_string(dir.file(LOG_FILE))
                .unwrap()
                .ends_with(" core: start\r\n")
        );
    }

    #[test]
    fn rolls_while_running() {
        let dir = TempDir::new("live-roll");
        let log = LogWriter::open(dir.path(), Proc::Tray).unwrap();
        let filler = "x".repeat(1000);
        for n in 0..1100 {
            log.line(Level::Info, "core", &format!("{n} {filler}"));
        }
        log.line(Level::Info, "core", "after the roll");
        let rolled: Vec<_> = names(&dir).into_iter().filter(|n| n != LOG_FILE).collect();
        assert_eq!(rolled.len(), 1, "{rolled:?}");
        let current = fs::read_to_string(dir.file(LOG_FILE)).unwrap();
        assert!(
            current.ends_with(" core: after the roll\r\n"),
            "{current:?}"
        );
        // Every line is in one of the two files, in order.
        let lines = all_lines(&dir);
        assert_eq!(lines.len(), 1101);
        for (n, line) in lines.iter().take(1100).enumerate() {
            assert!(
                line.contains(&format!(" core: {n} x")),
                "line {n}: {line:.80}"
            );
        }
    }

    /// Plants a rolled file of `size` bytes, last written `days_ago` days before `now_ms`.
    fn plant(dir: &TempDir, name: &str, size: usize, now_ms: i64, days_ago: i64) {
        let path = dir.file(name);
        fs::write(&path, vec![b'x'; size]).unwrap();
        let at = now_ms - days_ago * MS_PER_DAY;
        let time = std::time::UNIX_EPOCH + Duration::from_millis(at as u64);
        File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_modified(time)
            .unwrap();
    }

    #[test]
    fn prune_keeps_the_folder_under_its_cap_and_drops_old_files() {
        let limits = Limits {
            roll_over: 1000,
            folder_max: 10_000,
            max_age_ms: 30 * MS_PER_DAY,
        };
        let now = clock::now().wall_ms;
        let dir = TempDir::new("prune");
        // Twelve rolled files of 1000 bytes, an hour apart, the oldest first, and the two legacy names.
        let stamped: Vec<String> = (0..12)
            .map(|n| rolled_name(now - (12 - n) * 3_600_000))
            .collect();
        for name in &stamped {
            plant(&dir, name, 1000, now, 0);
        }
        plant(&dir, LEGACY_TWICE, 1000, now, 1);
        plant(&dir, LEGACY_ONCE, 1000, now, 1);
        // Not Wakeshot's: never deleted, never counted.
        plant(&dir, "wakeshot.notes.log", 50_000, now, 90);
        plant(&dir, "other.txt", 50_000, now, 90);
        fs::write(dir.file(LOG_FILE), vec![b'c'; 1500]).unwrap();

        roll_with(dir.path(), limits, now, false).unwrap();
        // Over the roll size: it rolled, and the prune ran. 10 000 bytes fit the new 1500-byte rolled file and the
        // eight newest older ones.
        let left = rolled(&dir);
        assert_eq!(left.len(), 9, "{left:?}");
        assert_eq!(left[..8], stamped[4..]);
        assert!(!dir.file(LOG_FILE).exists());
        let kept: u64 = rolled_files(dir.path()).iter().map(|f| f.size).sum();
        assert!(kept <= limits.folder_max, "{kept}");
        assert!(dir.file("wakeshot.notes.log").exists());
        assert!(dir.file("other.txt").exists());

        // Age: a file last written 31 days ago goes even with room to spare; one 29 days old stays.
        let dir = TempDir::new("prune-age");
        plant(&dir, &rolled_name(now - 40 * MS_PER_DAY), 10, now, 31);
        plant(&dir, &rolled_name(now - 35 * MS_PER_DAY), 10, now, 29);
        fs::write(dir.file(LOG_FILE), b"small").unwrap();
        roll_with(dir.path(), limits, now, false).unwrap();
        assert_eq!(rolled(&dir).len(), 2, "no roll, so no prune");
        LogWriter::roll(dir.path()).unwrap();
        assert_eq!(rolled(&dir), [rolled_name(now - 35 * MS_PER_DAY)]);
        assert!(dir.file(LOG_FILE).exists(), "the current file always stays");

        // A current file over the cap drops every rolled file, and stays itself.
        let dir = TempDir::new("prune-big");
        plant(&dir, &rolled_name(now - 1000), 10, now, 0);
        fs::write(dir.file(LOG_FILE), vec![b'c'; 900]).unwrap();
        prune(
            dir.path(),
            Limits {
                folder_max: 800,
                ..limits
            },
            now,
        );
        assert!(rolled(&dir).is_empty());
        assert!(dir.file(LOG_FILE).exists());
    }

    #[test]
    fn a_second_roller_never_rolls_twice() {
        let dir = TempDir::new("twice");
        write_big(&dir.file(LOG_FILE), b'A');
        let now = clock::now().wall_ms;
        assert_eq!(roll_current(dir.path(), LIMITS, now), Ok(true));
        // The second process saw the same big file a moment ago; by now `wakeshot.log` is new and small.
        LogWriter::open(dir.path(), Proc::Guard)
            .unwrap()
            .line(Level::Info, "core", "new");
        assert_eq!(roll_current(dir.path(), LIMITS, now + 1), Ok(false));
        assert_eq!(rolled(&dir).len(), 1);
        // The same name in the same millisecond: no roll, no error.
        write_big(&dir.file(LOG_FILE), b'B');
        assert_eq!(roll_current(dir.path(), LIMITS, now), Ok(false));
        assert_eq!(first_byte(&dir.file(LOG_FILE)), Some(b'B'));
    }

    /// Every line of every log file, oldest first.
    fn all_lines(dir: &TempDir) -> Vec<String> {
        log_files(dir.path())
            .iter()
            .filter_map(|path| fs::read_to_string(path).ok())
            .flat_map(|text| {
                text.split_terminator("\r\n")
                    .map(str::to_owned)
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    fn count_ending(lines: &[String], end: &str) -> usize {
        lines.iter().filter(|line| line.ends_with(end)).count()
    }

    /// A log over the limit, made of `filler` lines.
    fn big_log(dir: &TempDir, filler: &str) -> usize {
        let line = format!("{filler}\r\n");
        let count = LIMITS.roll_over as usize / line.len() + 1;
        fs::write(dir.file(LOG_FILE), line.repeat(count)).unwrap();
        count
    }

    #[test]
    fn roll_with_file_open_elsewhere() {
        // Another process's writer exists: the roll works, and its next line goes to the new file.
        let dir = TempDir::new("open-writer");
        let fillers = big_log(&dir, "filler");
        let other = LogWriter::open(dir.path(), Proc::Guard).unwrap();
        // This line finds the file over the size and rolls it.
        other.line(Level::Info, "core", "before roll");
        let rolled_now = LogWriter::roll(dir.path());
        other.line(Level::Info, "core", "after roll");
        LogWriter::open(dir.path(), Proc::Tray)
            .unwrap()
            .line(Level::Info, "core", "new writer");
        let lines = all_lines(&dir);
        assert_eq!(count_ending(&lines, "filler"), fillers);
        for end in [
            " core: before roll",
            " core: after roll",
            " core: new writer",
        ] {
            assert_eq!(count_ending(&lines, end), 1, "{end}");
        }
        assert_eq!(rolled_now, Ok(()));
        let current = fs::read_to_string(dir.file(LOG_FILE)).unwrap();
        assert!(
            current.ends_with(" core: new writer\r\n") && current.lines().count() == 2,
            "{current:?}"
        );

        // A reader without delete sharing: the rename of `wakeshot.log` fails, and no line is lost.
        let dir = TempDir::new("open-reader");
        fs::write(dir.file(LEGACY_ONCE), "older\r\n").unwrap();
        let fillers = big_log(&dir, "filler");
        let reader = OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ.0 | FILE_SHARE_WRITE.0)
            .open(dir.file(LOG_FILE))
            .unwrap();
        let rolled_now = LogWriter::roll(dir.path());
        LogWriter::open(dir.path(), Proc::Tray)
            .unwrap()
            .line(Level::Info, "core", "new writer");
        drop(reader);
        let lines = all_lines(&dir);
        assert_eq!(count_ending(&lines, "filler"), fillers);
        assert_eq!(count_ending(&lines, "older"), 1);
        assert_eq!(count_ending(&lines, " core: new writer"), 1);
        assert!(rolled_now.is_err(), "{rolled_now:?}");
        println!("reader without delete sharing: {rolled_now:?}");

        // A reader on a rolled file the prune wants gone: the file stays, the roll still works.
        let dir = TempDir::new("open-oldest");
        let now = clock::now().wall_ms;
        plant(&dir, LEGACY_TWICE, 10, now, 40);
        let reader = OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ.0)
            .open(dir.file(LEGACY_TWICE))
            .unwrap();
        let fillers = big_log(&dir, "filler");
        let rolled_now = LogWriter::roll(dir.path());
        assert_eq!(rolled_now, Ok(()));
        assert!(dir.file(LEGACY_TWICE).exists());
        drop(reader);
        let lines = all_lines(&dir);
        assert_eq!(count_ending(&lines, "filler"), fillers);
        // The next prune takes it.
        LogWriter::roll(dir.path()).unwrap();
        assert!(!dir.file(LEGACY_TWICE).exists());
    }
}
