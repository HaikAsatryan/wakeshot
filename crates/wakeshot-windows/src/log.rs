//! The text log in `logs\wakeshot.log` (architecture section 8).
//!
//! Owner: power track (`p3-log`). The trait, the types and `MemoryLog` are contract: the sources, tray and ship tracks
//! log through `Log` and test with `MemoryLog`.
//!
//! A line reads `2026-11-03T18:55:12.345+04:00 [1234.567] tray    INFO  power: text`: local wall time with its UTC
//! offset, then the monotonic clock in seconds (awake time since boot), process, level, area and text.

use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::path::{Path, PathBuf};
#[cfg(any(test, feature = "test-support"))]
use std::sync::Mutex;

use windows::Win32::Foundation::{
    ERROR_FILE_NOT_FOUND, ERROR_PATH_NOT_FOUND, FILETIME, HANDLE, WIN32_ERROR,
};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_APPEND_DATA, FILE_ATTRIBUTE_NORMAL, FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE,
    FILE_SHARE_MODE, FILE_SHARE_READ, FILE_SHARE_WRITE, FileTimeToLocalFileTime, GetFileSizeEx,
    MOVEFILE_REPLACE_EXISTING, MoveFileExW, OPEN_ALWAYS, OPEN_EXISTING, WriteFile,
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
const ROLLED_ONCE: &str = "wakeshot.1.log";
const ROLLED_TWICE: &str = "wakeshot.2.log";

/// "Over 1 MB" (architecture section 8), read as 1 MiB.
const ROLL_OVER_BYTES: u64 = 1024 * 1024;

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

/// Appends to the log file, shared by every process.
#[derive(Debug)]
pub struct LogWriter {
    file: OwnedHandle,
    proc: Proc,
}

impl LogWriter {
    /// Opens `wakeshot.log` in `logs_dir` (`DataDir::logs`), creating both if needed.
    pub fn open(logs_dir: &Path, proc: Proc) -> Result<Self, WinError> {
        std::fs::create_dir_all(logs_dir).map_err(|e| io_error("CreateDirectoryW", &e))?;
        let name = HSTRING::from(log_file(logs_dir).as_path());
        // Append rights without FILE_WRITE_DATA make every write land at the end of the file, so another process's
        // lines are never overwritten (Microsoft, "File Access Rights Constants"). std's append mode is not
        // documented to use exactly this mask, so the handle is opened here.
        // SAFETY: the name is a live, NUL-terminated wide string; no security attributes and no template are passed.
        let handle = unsafe {
            CreateFileW(
                &name,
                FILE_APPEND_DATA.0,
                SHARE_ALL,
                None,
                OPEN_ALWAYS,
                FILE_ATTRIBUTE_NORMAL,
                None,
            )
        }
        .map_err(|e| win_error("CreateFileW", &e))?;
        // SAFETY: CreateFileW just returned this handle and nothing else owns it, so OwnedHandle may close it.
        let file = unsafe { OwnedHandle::from_raw_handle(handle.0) };
        Ok(Self { file, proc })
    }

    /// Moves `wakeshot.log` to `wakeshot.1.log` and that one to `wakeshot.2.log`, when `wakeshot.log` in `logs_dir` is
    /// over 1 MB; the old `wakeshot.2.log` goes. A process that still has the log open keeps appending to the moved
    /// file, so no line is lost. An `Err` leaves every line in one of the three files.
    pub fn roll(logs_dir: &Path) -> Result<(), WinError> {
        let current = log_file(logs_dir);
        match file_size(&current)? {
            Some(size) if size > ROLL_OVER_BYTES => {}
            _ => return Ok(()),
        }
        let once = logs_dir.join(ROLLED_ONCE);
        // Oldest first, so a failure at either step drops nothing but the oldest file.
        match move_replacing(&once, &logs_dir.join(ROLLED_TWICE)) {
            Err(e) if is_not_found(e.code) => {}
            other => other?,
        }
        move_replacing(&current, &once)
    }
}

impl Log for LogWriter {
    fn line(&self, level: Level, area: &'static str, text: &str) {
        let line = format_line(stamp_now(), self.proc, level, area, text);
        // WriteFile panics on a buffer over 4 GiB, and a panic aborts the caller.
        if u32::try_from(line.len()).is_err() {
            return;
        }
        let mut written = 0u32;
        // A failed or short write drops the line: a retry could split it, and the log must never stop its caller.
        // SAFETY: the handle stays open while `self` lives; the buffer and the count are live locals.
        let _ = unsafe {
            WriteFile(
                HANDLE(self.file.as_raw_handle()),
                Some(line.as_bytes()),
                Some(&mut written),
                None,
            )
        };
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

/// The size from an open handle, the file's own, while other processes may be appending. `None` when the file does not
/// exist.
fn file_size(path: &Path) -> Result<Option<u64>, WinError> {
    let name = HSTRING::from(path);
    // Attribute rights and every share mode, so reading the size never locks out a writer or a reader.
    // SAFETY: the name is a live, NUL-terminated wide string; no security attributes and no template are passed.
    let opened = unsafe {
        CreateFileW(
            &name,
            FILE_READ_ATTRIBUTES.0,
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
                Ok(None)
            } else {
                Err(error)
            };
        }
    };
    // SAFETY: CreateFileW just returned this handle and nothing else owns it, so OwnedHandle may close it.
    let file = unsafe { OwnedHandle::from_raw_handle(handle.0) };
    let mut size = 0i64;
    // SAFETY: the handle is open for the call; the pointer is to a live local.
    unsafe { GetFileSizeEx(HANDLE(file.as_raw_handle()), &mut size) }
        .map_err(|e| win_error("GetFileSizeEx", &e))?;
    Ok(Some(u64::try_from(size).unwrap_or(0)))
}

fn move_replacing(from: &Path, to: &Path) -> Result<(), WinError> {
    let (from, to) = (HSTRING::from(from), HSTRING::from(to));
    // SAFETY: both names are live, NUL-terminated wide strings.
    unsafe { MoveFileExW(&from, &to, MOVEFILE_REPLACE_EXISTING) }
        .map_err(|e| win_error("MoveFileExW", &e))
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
        fs::write(dir.file(LOG_FILE), b"").unwrap();
        // A read-only handle makes every WriteFile fail.
        let file = OwnedHandle::from(File::open(dir.file(LOG_FILE)).unwrap());
        let writer = LogWriter {
            file,
            proc: Proc::Tray,
        };
        writer.line(Level::Info, "core", "dropped");
        writer.line(Level::Info, "core", "dropped too");
        assert_eq!(fs::read(dir.file(LOG_FILE)).unwrap(), b"");
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
        let mut parts = role.splitn(3, '|');
        let child: usize = parts.next().unwrap().parse().unwrap();
        let start_ms: i64 = parts.next().unwrap().parse().unwrap();
        let dir = PathBuf::from(parts.next().unwrap());
        let log = LogWriter::open(&dir, CHILD_PROCS[child]).unwrap();
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

    #[test]
    fn appends_from_many_processes_never_mix() {
        if let Ok(role) = std::env::var(CHILD_ROLE) {
            run_append_child(&role);
            return;
        }
        let dir = TempDir::new("mix");
        let start_ms = clock::now().wall_ms + 1500;
        let exe = std::env::current_exe().unwrap();
        let children: Vec<_> = (0..CHILDREN)
            .map(|child| {
                Command::new(&exe)
                    .args([
                        "log::tests::appends_from_many_processes_never_mix",
                        "--exact",
                    ])
                    .env(
                        CHILD_ROLE,
                        format!("{child}|{start_ms}|{}", dir.path().display()),
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

        let bytes = fs::read(dir.file(LOG_FILE)).unwrap();
        let text = String::from_utf8(bytes).expect("a line was cut inside a character");
        assert!(text.ends_with("\r\n"), "the last line is cut");
        let mut next = [0usize; CHILDREN];
        let (mut switches, mut last) = (0, None);
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
        assert_eq!(next, [LINES_PER_CHILD; CHILDREN]);
        // Without one switch more than the child count, the children never wrote at the same time.
        assert!(
            switches > CHILDREN,
            "only {switches} switches between children"
        );
        println!("{} bytes, {switches} switches between children", text.len());
    }

    fn write_big(path: &Path, fill: u8) {
        fs::write(path, vec![fill; ROLL_OVER_BYTES as usize + 1]).unwrap();
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

    #[test]
    fn roll_keeps_three_files() {
        let dir = TempDir::new("roll");
        LogWriter::roll(&dir.file("missing")).unwrap();
        LogWriter::roll(dir.path()).unwrap();
        assert!(names(&dir).is_empty());

        fs::write(dir.file(LOG_FILE), vec![b'0'; ROLL_OVER_BYTES as usize]).unwrap();
        LogWriter::roll(dir.path()).unwrap();
        assert_eq!(names(&dir), [LOG_FILE], "exactly 1 MiB is not over");

        write_big(&dir.file(LOG_FILE), b'A');
        LogWriter::roll(dir.path()).unwrap();
        assert_eq!(names(&dir), [ROLLED_ONCE]);
        assert_eq!(first_byte(&dir.file(ROLLED_ONCE)), Some(b'A'));

        write_big(&dir.file(LOG_FILE), b'B');
        LogWriter::roll(dir.path()).unwrap();
        assert_eq!(names(&dir), [ROLLED_ONCE, ROLLED_TWICE]);
        assert_eq!(first_byte(&dir.file(ROLLED_ONCE)), Some(b'B'));
        assert_eq!(first_byte(&dir.file(ROLLED_TWICE)), Some(b'A'));

        write_big(&dir.file(LOG_FILE), b'C');
        LogWriter::roll(dir.path()).unwrap();
        LogWriter::open(dir.path(), Proc::Tray)
            .unwrap()
            .line(Level::Info, "core", "start");
        assert_eq!(names(&dir), [ROLLED_ONCE, ROLLED_TWICE, LOG_FILE]);
        assert_eq!(first_byte(&dir.file(ROLLED_ONCE)), Some(b'C'));
        assert_eq!(first_byte(&dir.file(ROLLED_TWICE)), Some(b'B'));
        assert!(
            fs::read_to_string(dir.file(LOG_FILE))
                .unwrap()
                .ends_with(" core: start\r\n")
        );
    }

    /// Every line of the three files, in file order oldest first.
    fn all_lines(dir: &TempDir) -> Vec<String> {
        [ROLLED_TWICE, ROLLED_ONCE, LOG_FILE]
            .iter()
            .filter_map(|name| fs::read_to_string(dir.file(name)).ok())
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
        let count = ROLL_OVER_BYTES as usize / line.len() + 1;
        fs::write(dir.file(LOG_FILE), line.repeat(count)).unwrap();
        count
    }

    #[test]
    fn roll_with_file_open_elsewhere() {
        // Another process's writer is open: the roll works, and its lines follow the moved file.
        let dir = TempDir::new("open-writer");
        let fillers = big_log(&dir, "filler");
        let other = LogWriter::open(dir.path(), Proc::Guard).unwrap();
        other.line(Level::Info, "core", "before roll");
        let rolled = LogWriter::roll(dir.path());
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
        assert_eq!(rolled, Ok(()));
        let current = fs::read_to_string(dir.file(LOG_FILE)).unwrap();
        assert!(
            current.ends_with(" core: new writer\r\n") && current.lines().count() == 1,
            "{current:?}"
        );
        println!("open writer: {rolled:?}");

        // A reader without delete sharing: the move of `wakeshot.log` fails, and no line is lost.
        let dir = TempDir::new("open-reader");
        fs::write(dir.file(ROLLED_ONCE), "older\r\n").unwrap();
        let fillers = big_log(&dir, "filler");
        let reader = OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ.0 | FILE_SHARE_WRITE.0)
            .open(dir.file(LOG_FILE))
            .unwrap();
        let rolled = LogWriter::roll(dir.path());
        LogWriter::open(dir.path(), Proc::Tray)
            .unwrap()
            .line(Level::Info, "core", "new writer");
        drop(reader);
        let lines = all_lines(&dir);
        assert_eq!(count_ending(&lines, "filler"), fillers);
        assert_eq!(count_ending(&lines, "older"), 1);
        assert_eq!(count_ending(&lines, " core: new writer"), 1);
        println!("reader without delete sharing: {rolled:?}");

        // A reader on `wakeshot.2.log`, the file the roll replaces.
        let dir = TempDir::new("open-oldest");
        fs::write(dir.file(ROLLED_TWICE), "oldest\r\n").unwrap();
        fs::write(dir.file(ROLLED_ONCE), "older\r\n").unwrap();
        let fillers = big_log(&dir, "filler");
        let reader = File::open(dir.file(ROLLED_TWICE)).unwrap();
        let rolled = LogWriter::roll(dir.path());
        LogWriter::open(dir.path(), Proc::Tray)
            .unwrap()
            .line(Level::Info, "core", "new writer");
        drop(reader);
        let lines = all_lines(&dir);
        assert_eq!(count_ending(&lines, "filler"), fillers);
        assert_eq!(count_ending(&lines, "older"), 1);
        assert_eq!(count_ending(&lines, " core: new writer"), 1);
        if rolled.is_err() {
            assert_eq!(
                count_ending(&lines, "oldest"),
                1,
                "a failed roll dropped the oldest file"
            );
        }
        println!("reader on the oldest file: {rolled:?}");
    }
}
