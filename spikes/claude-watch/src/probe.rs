//! One-shot checks for card `p0-unelevated-liveness`: what this process can read and open for each Claude Code
//! session, and at which integrity level each side runs. Every result comes from a real call; nothing is inferred.
//! File contents are never logged, only sizes and a few status fields.

use crate::names::{StatusFile, status_file_pid};
use crate::procs::filetime_u64;
use crate::watch::{Logger, find_transcript, first_object};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};
use windows::Wdk::System::SystemInformation::{NtQuerySystemInformation, SystemProcessInformation};
use windows::Win32::Foundation::{
    CloseHandle, FILETIME, HANDLE, HLOCAL, LocalFree, STATUS_INFO_LENGTH_MISMATCH, STILL_ACTIVE,
    WAIT_OBJECT_0, WAIT_TIMEOUT,
};
use windows::Win32::Security::Authorization::{
    ConvertSecurityDescriptorToStringSecurityDescriptorW, ConvertSidToStringSidW, GetSecurityInfo,
    SDDL_REVISION_1, SE_KERNEL_OBJECT,
};
use windows::Win32::Security::{
    DACL_SECURITY_INFORMATION, GetSidSubAuthority, GetSidSubAuthorityCount, GetTokenInformation,
    LABEL_SECURITY_INFORMATION, OWNER_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR, PSID,
    TOKEN_GROUPS, TOKEN_INFORMATION_CLASS, TOKEN_MANDATORY_LABEL, TOKEN_QUERY, TokenElevationType,
    TokenIntegrityLevel, TokenLogonSid,
};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OVERLAPPED, FILE_LIST_DIRECTORY,
    FILE_NOTIFY_CHANGE_FILE_NAME, FILE_NOTIFY_CHANGE_LAST_WRITE, FILE_NOTIFY_CHANGE_SIZE,
    FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING, ReadDirectoryChangesW,
};
use windows::Win32::System::IO::{CancelIoEx, GetOverlappedResult, OVERLAPPED};
use windows::Win32::System::Threading::{
    CreateEventW, GetCurrentProcess, GetExitCodeProcess, GetProcessTimes, OpenProcess,
    OpenProcessToken, PROCESS_ACCESS_RIGHTS, PROCESS_NAME_WIN32, PROCESS_QUERY_INFORMATION,
    PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_READ_CONTROL, PROCESS_SYNCHRONIZE, PROCESS_VM_READ,
    QueryFullProcessImageNameW, ResetEvent, WaitForSingleObject,
};
use windows::Win32::System::WindowsProgramming::SYSTEM_PROCESS_INFORMATION;
use windows::core::{HSTRING, PWSTR};

/// Closes the handle when dropped, so an early return cannot leak it.
struct Owned(HANDLE);

impl Drop for Owned {
    fn drop(&mut self) {
        // SAFETY: the handle came from a successful open and is closed exactly once.
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

fn hex(code: i32) -> String {
    format!("0x{:08X}", code as u32)
}

fn open(pid: u32, rights: PROCESS_ACCESS_RIGHTS) -> Result<Owned, i32> {
    // SAFETY: plain Win32 call; the handle is owned by the returned value.
    unsafe { OpenProcess(rights, false, pid) }
        .map(Owned)
        .map_err(|e| e.code().0)
}

pub fn integrity_name(rid: u32) -> String {
    match rid {
        0x0000 => "Untrusted".to_string(),
        0x1000 => "Low".to_string(),
        0x2000 => "Medium".to_string(),
        0x2100 => "Medium Plus".to_string(),
        0x3000 => "High".to_string(),
        0x4000 => "System".to_string(),
        0x5000 => "Protected".to_string(),
        other => format!("0x{other:04X}"),
    }
}

/// Token information comes back as a struct with pointers into the same buffer, so the buffer is u64 aligned.
fn token_bytes(token: HANDLE, class: TOKEN_INFORMATION_CLASS) -> Result<Vec<u64>, i32> {
    let mut len = 0u32;
    // SAFETY: a size query with no buffer; it fails by design and only sets `len`.
    let first = unsafe { GetTokenInformation(token, class, None, 0, &mut len) };
    if len == 0 {
        return Err(first.err().map_or(0, |e| e.code().0));
    }
    let mut buf = vec![0u64; (len as usize).div_ceil(8)];
    // SAFETY: the buffer holds at least `len` bytes.
    unsafe { GetTokenInformation(token, class, Some(buf.as_mut_ptr().cast()), len, &mut len) }
        .map_err(|e| e.code().0)?;
    Ok(buf)
}

fn sid_string(sid: PSID) -> Result<String, i32> {
    let mut text = PWSTR::null();
    // SAFETY: `sid` points into a live token buffer; the string is freed with LocalFree as documented.
    unsafe {
        ConvertSidToStringSidW(sid, &mut text).map_err(|e| e.code().0)?;
        let out = text.to_string().unwrap_or_default();
        let _ = LocalFree(Some(HLOCAL(text.0.cast())));
        Ok(out)
    }
}

pub struct TokenFacts {
    pub integrity: Result<u32, i32>,
    pub elevation: Result<i32, i32>,
    pub logon_sid: Result<String, i32>,
}

impl TokenFacts {
    fn read(token: HANDLE) -> Self {
        let integrity = token_bytes(token, TokenIntegrityLevel).map(|buf| {
            // SAFETY: the buffer holds a TOKEN_MANDATORY_LABEL whose SID points into the same buffer.
            unsafe {
                let label = &*(buf.as_ptr() as *const TOKEN_MANDATORY_LABEL);
                let count = *GetSidSubAuthorityCount(label.Label.Sid);
                *GetSidSubAuthority(label.Label.Sid, u32::from(count) - 1)
            }
        });
        let elevation = token_bytes(token, TokenElevationType).map(|buf| buf[0] as u32 as i32);
        let logon_sid = token_bytes(token, TokenLogonSid).and_then(|buf| {
            // SAFETY: the buffer holds a TOKEN_GROUPS with at least one entry when the call succeeds.
            let groups = unsafe { &*(buf.as_ptr() as *const TOKEN_GROUPS) };
            if groups.GroupCount == 0 {
                return Err(0);
            }
            sid_string(groups.Groups[0].Sid)
        });
        Self {
            integrity,
            elevation,
            logon_sid,
        }
    }

    pub fn of_self() -> Result<Self, i32> {
        let mut token = HANDLE::default();
        // SAFETY: the pseudo handle needs no close; the token handle is owned below.
        unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) }
            .map_err(|e| e.code().0)?;
        let token = Owned(token);
        Ok(Self::read(token.0))
    }

    fn of_process(process: HANDLE) -> Result<Self, i32> {
        let mut token = HANDLE::default();
        // SAFETY: `process` is a live handle; the token handle is owned below.
        unsafe { OpenProcessToken(process, TOKEN_QUERY, &mut token) }.map_err(|e| e.code().0)?;
        let token = Owned(token);
        Ok(Self::read(token.0))
    }

    pub fn describe(&self) -> String {
        let integrity = match self.integrity {
            Ok(rid) => format!("{} (S-1-16-{rid})", integrity_name(rid)),
            Err(e) => format!("unreadable {}", hex(e)),
        };
        let elevation = match self.elevation {
            Ok(1) => "Default (no split token)".to_string(),
            Ok(2) => "Full (elevated)".to_string(),
            Ok(3) => "Limited (filtered, not elevated)".to_string(),
            Ok(v) => format!("{v}"),
            Err(e) => format!("unreadable {}", hex(e)),
        };
        let logon = match &self.logon_sid {
            Ok(s) => s.clone(),
            Err(e) => format!("unreadable {}", hex(*e)),
        };
        format!("integrity {integrity}, elevation type {elevation}, logon SID {logon}")
    }
}

fn process_sddl(process: HANDLE) -> Result<String, i32> {
    let what = OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION | LABEL_SECURITY_INFORMATION;
    let mut sd = PSECURITY_DESCRIPTOR::default();
    // SAFETY: the descriptor and the string are freed with LocalFree as both APIs document.
    unsafe {
        let err = GetSecurityInfo(
            process,
            SE_KERNEL_OBJECT,
            what,
            None,
            None,
            None,
            None,
            Some(&mut sd),
        );
        if err.0 != 0 {
            return Err(err.to_hresult().0);
        }
        let mut text = PWSTR::null();
        let conv = ConvertSecurityDescriptorToStringSecurityDescriptorW(
            sd,
            SDDL_REVISION_1,
            what,
            &mut text,
            None,
        );
        let out = match conv {
            Ok(()) => Ok(text.to_string().unwrap_or_default()),
            Err(e) => Err(e.code().0),
        };
        if !text.is_null() {
            let _ = LocalFree(Some(HLOCAL(text.0.cast())));
        }
        let _ = LocalFree(Some(HLOCAL(sd.0)));
        out
    }
}

fn creation_time(process: HANDLE) -> Result<u64, i32> {
    let mut created = FILETIME::default();
    let mut exited = FILETIME::default();
    let mut kernel = FILETIME::default();
    let mut user = FILETIME::default();
    // SAFETY: `process` is a live handle and the out values are local.
    unsafe { GetProcessTimes(process, &mut created, &mut exited, &mut kernel, &mut user) }
        .map_err(|e| e.code().0)?;
    Ok(filetime_u64(created))
}

fn image_path(process: HANDLE) -> Result<String, i32> {
    let mut buf = vec![0u16; 1024];
    let mut size = buf.len() as u32;
    // SAFETY: `size` is the buffer length in characters, as the call expects.
    unsafe {
        QueryFullProcessImageNameW(
            process,
            PROCESS_NAME_WIN32,
            PWSTR(buf.as_mut_ptr()),
            &mut size,
        )
    }
    .map_err(|e| e.code().0)?;
    Ok(String::from_utf16_lossy(&buf[..size as usize]))
}

fn exit_code(process: HANDLE) -> Result<u32, i32> {
    let mut code = 0u32;
    // SAFETY: `process` is a live handle.
    unsafe { GetExitCodeProcess(process, &mut code) }.map_err(|e| e.code().0)?;
    Ok(code)
}

pub enum Waited {
    Signaled(Duration),
    TimedOut,
    Failed(u32),
}

pub fn wait(process: HANDLE, ms: u32) -> Waited {
    let start = Instant::now();
    // SAFETY: `process` is a live handle opened with SYNCHRONIZE.
    let r = unsafe { WaitForSingleObject(process, ms) };
    if r == WAIT_OBJECT_0 {
        Waited::Signaled(start.elapsed())
    } else if r == WAIT_TIMEOUT {
        Waited::TimedOut
    } else {
        Waited::Failed(r.0)
    }
}

/// Creation times of every process from the system process list. Needs no process handle, so it is the fallback
/// if `OpenProcess` is refused.
pub fn system_create_times() -> Result<HashMap<u32, u64>, i32> {
    let mut size: usize = 1 << 20;
    loop {
        let mut buf = vec![0u64; size / 8];
        let mut needed = 0u32;
        // SAFETY: the buffer holds `size` bytes; the call writes at most that much.
        let status = unsafe {
            NtQuerySystemInformation(
                SystemProcessInformation,
                buf.as_mut_ptr().cast(),
                size as u32,
                &mut needed,
            )
        };
        if status == STATUS_INFO_LENGTH_MISMATCH {
            // The list can grow between two calls, so leave room.
            size = (needed as usize).max(size) + (64 << 10);
            continue;
        }
        if status.is_err() {
            return Err(status.0);
        }
        let bytes: &[u8] =
            // SAFETY: a u64 buffer viewed as bytes, same length.
            unsafe { std::slice::from_raw_parts(buf.as_ptr().cast(), buf.len() * 8) };
        return Ok(parse_process_list(bytes));
    }
}

/// winternl.h hides `CreateTime` inside `Reserved1`: it is the LARGE_INTEGER 24 bytes into that field (after
/// WorkingSetPrivateSize, HardFaultCount, NumberOfThreadsHighWatermark and CycleTime in the NT layout). The test
/// `system_list_create_time_matches_process_times` checks this offset against `GetProcessTimes`.
const CREATE_TIME_IN_RESERVED1: usize = 24;

fn parse_process_list(bytes: &[u8]) -> HashMap<u32, u64> {
    let reserved1 = std::mem::offset_of!(SYSTEM_PROCESS_INFORMATION, Reserved1);
    let pid_at = std::mem::offset_of!(SYSTEM_PROCESS_INFORMATION, UniqueProcessId);
    let time_at = reserved1 + CREATE_TIME_IN_RESERVED1;
    let read_u64 = |at: usize| -> Option<u64> {
        bytes
            .get(at..at + 8)
            .map(|b| u64::from_le_bytes(b.try_into().unwrap_or([0; 8])))
    };
    let mut out = HashMap::new();
    let mut at = 0usize;
    while let (Some(pid), Some(time)) = (read_u64(at + pid_at), read_u64(at + time_at)) {
        out.insert(pid as u32, time);
        let next = bytes
            .get(at..at + 4)
            .map(|b| u32::from_le_bytes(b.try_into().unwrap_or([0; 4])))
            .unwrap_or(0);
        if next == 0 {
            break;
        }
        at += next as usize;
    }
    out
}

pub fn proc_start_of(fields: &Map<String, Value>) -> Option<u64> {
    fields
        .get("procStartFt")
        .or_else(|| fields.get("procStart"))
        .and_then(Value::as_str)
        .and_then(|s| s.parse().ok())
}

fn compare(label: &str, real: u64, expected: Option<u64>) -> String {
    match expected {
        Some(e) if e == real => format!("{label} {real}, equals procStart"),
        Some(e) => format!("{label} {real}, DIFFERS from procStart {e}"),
        None => format!("{label} {real}, no procStart to compare"),
    }
}

/// Every check one process needs. `expected` is `procStart` from its status file, if it has one.
pub fn check_process(log: &mut Logger, pid: u32, expected: Option<u64>, wait_ms: u32) {
    let tag = format!("[{pid}]");
    let rights = [
        (
            "QUERY_LIMITED_INFORMATION",
            PROCESS_QUERY_LIMITED_INFORMATION,
        ),
        ("SYNCHRONIZE", PROCESS_SYNCHRONIZE),
        (
            "QUERY_LIMITED_INFORMATION|SYNCHRONIZE",
            PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE,
        ),
        ("QUERY_INFORMATION", PROCESS_QUERY_INFORMATION),
        ("VM_READ", PROCESS_VM_READ),
        ("READ_CONTROL", PROCESS_READ_CONTROL),
    ];
    for (name, mask) in rights {
        let shown = match open(pid, mask) {
            Ok(_) => "ok".to_string(),
            Err(e) => format!("FAILED {}", hex(e)),
        };
        log.line(&format!("{tag} OpenProcess {name}: {shown}"));
    }
    match open(pid, PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE) {
        Ok(h) => {
            let created = match creation_time(h.0) {
                Ok(t) => compare("creation time", t, expected),
                Err(e) => format!("creation time FAILED {}", hex(e)),
            };
            log.line(&format!("{tag} GetProcessTimes: {created}"));
            let image = image_path(h.0).unwrap_or_else(|e| format!("FAILED {}", hex(e)));
            log.line(&format!("{tag} QueryFullProcessImageNameW: {image}"));
            let code = match exit_code(h.0) {
                Ok(c) if c == STILL_ACTIVE.0 as u32 => "STILL_ACTIVE".to_string(),
                Ok(c) => format!("{c}"),
                Err(e) => format!("FAILED {}", hex(e)),
            };
            log.line(&format!("{tag} GetExitCodeProcess: {code}"));
            let waited = match wait(h.0, wait_ms) {
                Waited::TimedOut => format!("timed out after {wait_ms} ms (still running)"),
                Waited::Signaled(d) => format!("signaled after {} ms (exited)", d.as_millis()),
                Waited::Failed(r) => format!("FAILED return {r}"),
            };
            log.line(&format!("{tag} WaitForSingleObject: {waited}"));
            let token = match TokenFacts::of_process(h.0) {
                Ok(t) => t.describe(),
                Err(e) => format!("OpenProcessToken TOKEN_QUERY FAILED {}", hex(e)),
            };
            log.line(&format!("{tag} target token: {token}"));
        }
        Err(e) => log.line(&format!(
            "{tag} no QUERY_LIMITED_INFORMATION|SYNCHRONIZE handle ({}), handle checks skipped",
            hex(e)
        )),
    }
    let sddl = match open(pid, PROCESS_READ_CONTROL) {
        Ok(h) => process_sddl(h.0).unwrap_or_else(|e| format!("GetSecurityInfo FAILED {}", hex(e))),
        Err(e) => format!("no READ_CONTROL handle {}", hex(e)),
    };
    log.line(&format!("{tag} process security descriptor: {sddl}"));
    let listed = match system_create_times() {
        Ok(map) => match map.get(&pid) {
            Some(t) => compare("create time", *t, expected),
            None => "pid not in the list".to_string(),
        },
        Err(status) => format!("NtQuerySystemInformation FAILED {}", hex(status)),
    };
    log.line(&format!("{tag} system process list: {listed}"));
    let proc_start = expected.map(|v| v.to_string());
    log.line(&format!(
        "{tag} watcher verdict: {}",
        crate::procs::verdict(pid, proc_start.as_deref())
    ));
}

fn shown_field(fields: &Map<String, Value>, key: &str) -> String {
    match fields.get(key) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Null) => "null".to_string(),
        Some(v) => v.to_string(),
        None => "(none)".to_string(),
    }
}

fn check_transcript(log: &mut Logger, home: &Path, pid: u32, sid: &str) {
    let tag = format!("[{pid}]");
    let Some(path) = find_transcript(home, sid) else {
        log.line(&format!(
            "{tag} transcript {sid}.jsonl: not found under projects"
        ));
        return;
    };
    let shown = match fs::read(&path) {
        Ok(bytes) => {
            let lines = bytes.iter().filter(|&&b| b == b'\n').count();
            format!("read ok, {} bytes, {lines} lines", bytes.len())
        }
        Err(e) => format!("read FAILED {e}"),
    };
    log.line(&format!("{tag} transcript {}: {shown}", path.display()));
}

/// Hands each change notification for one folder to `on_event` as it arrives, for `for_ms`, so a log shows when
/// each one came. Returns how many arrived.
pub fn watch_dir(
    dir: &Path,
    for_ms: u64,
    on_event: &mut dyn FnMut(u32, &str),
) -> Result<usize, String> {
    let wide = HSTRING::from(dir.as_os_str());
    // SAFETY: plain Win32 calls; both handles are owned below, and the pending read is cancelled and finished
    // before the buffer and the OVERLAPPED go out of scope.
    unsafe {
        let handle = CreateFileW(
            &wide,
            FILE_LIST_DIRECTORY.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            None,
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OVERLAPPED,
            None,
        )
        .map_err(|e| format!("CreateFileW FILE_LIST_DIRECTORY FAILED {}", hex(e.code().0)))?;
        let handle = Owned(handle);
        let event = Owned(
            CreateEventW(None, true, false, None)
                .map_err(|e| format!("CreateEventW FAILED {}", hex(e.code().0)))?,
        );
        let mut buf = vec![0u64; 8192];
        let filter =
            FILE_NOTIFY_CHANGE_FILE_NAME | FILE_NOTIFY_CHANGE_LAST_WRITE | FILE_NOTIFY_CHANGE_SIZE;
        let deadline = Instant::now() + Duration::from_millis(for_ms);
        let mut count = 0usize;
        loop {
            let mut ov = OVERLAPPED {
                hEvent: event.0,
                ..Default::default()
            };
            let _ = ResetEvent(event.0);
            ReadDirectoryChangesW(
                handle.0,
                buf.as_mut_ptr().cast(),
                (buf.len() * 8) as u32,
                false,
                filter,
                None,
                Some(&mut ov),
                None,
            )
            .map_err(|e| format!("ReadDirectoryChangesW FAILED {}", hex(e.code().0)))?;
            let left = deadline.saturating_duration_since(Instant::now());
            let mut got = 0u32;
            if wait(event.0, left.as_millis() as u32).is_signaled() {
                GetOverlappedResult(handle.0, &ov, &mut got, false)
                    .map_err(|e| format!("GetOverlappedResult FAILED {}", hex(e.code().0)))?;
                let bytes: &[u8] = std::slice::from_raw_parts(buf.as_ptr().cast(), got as usize);
                for (action, name) in parse_notify(bytes) {
                    on_event(action, &name);
                    count += 1;
                }
                if Instant::now() < deadline {
                    continue;
                }
                return Ok(count);
            }
            let _ = CancelIoEx(handle.0, Some(&ov));
            let _ = GetOverlappedResult(handle.0, &ov, &mut got, true);
            return Ok(count);
        }
    }
}

impl Waited {
    fn is_signaled(&self) -> bool {
        matches!(self, Waited::Signaled(_))
    }
}

fn parse_notify(bytes: &[u8]) -> Vec<(u32, String)> {
    let u32_at = |at: usize| -> Option<u32> {
        bytes
            .get(at..at + 4)
            .map(|b| u32::from_le_bytes(b.try_into().unwrap_or([0; 4])))
    };
    let mut out = Vec::new();
    let mut at = 0usize;
    while let (Some(next), Some(action), Some(len)) = (u32_at(at), u32_at(at + 4), u32_at(at + 8)) {
        let name_at = at + 12;
        let Some(raw) = bytes.get(name_at..name_at + len as usize) else {
            break;
        };
        let units: Vec<u16> = raw
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| u16::from_le_bytes(*c))
            .collect();
        out.push((action, String::from_utf16_lossy(&units)));
        if next == 0 {
            break;
        }
        at += next as usize;
    }
    out
}

fn action_name(action: u32) -> &'static str {
    match action {
        1 => "added",
        2 => "removed",
        3 => "modified",
        4 => "renamed from",
        5 => "renamed to",
        _ => "other",
    }
}

pub fn log_self(log: &mut Logger) {
    let facts = match TokenFacts::of_self() {
        Ok(t) => t.describe(),
        Err(e) => format!("OpenProcessToken FAILED {}", hex(e)),
    };
    log.line(&format!("self pid {}: {facts}", std::process::id()));
}

pub struct ProbeOptions<'a> {
    pub home: &'a Path,
    /// Processes to check that have no status file, for example an elevated `wakeshot run` stand-in.
    pub extra_pids: &'a [u32],
    pub wait_ms: u32,
    pub notify_ms: u64,
}

pub fn run(log: &mut Logger, opts: &ProbeOptions) {
    log_self(log);
    let dir = opts.home.join(".claude").join("sessions");
    let names: Vec<String> = match fs::read_dir(&dir) {
        Ok(entries) => entries
            .flatten()
            .filter_map(|e| e.file_name().to_str().map(str::to_string))
            .collect(),
        Err(e) => {
            log.line(&format!("list {}: FAILED {e}", dir.display()));
            Vec::new()
        }
    };
    log.line(&format!(
        "list {}: {} entries, {} status files",
        dir.display(),
        names.len(),
        names
            .iter()
            .filter(|n| status_file_pid(n).is_some())
            .count()
    ));
    for name in &names {
        let Some(file) = StatusFile::from_name(&dir, name) else {
            continue;
        };
        let pid = file.pid();
        let fields = match file.read() {
            Ok(bytes) => {
                let fields = first_object(&bytes);
                let shown = match &fields {
                    Some(f) => format!(
                        "read ok, {} bytes: status={} kind={} entrypoint={} procStart={}",
                        bytes.len(),
                        shown_field(f, "status"),
                        shown_field(f, "kind"),
                        shown_field(f, "entrypoint"),
                        shown_field(f, "procStart"),
                    ),
                    None => format!("read ok, {} bytes, not JSON", bytes.len()),
                };
                log.line(&format!("[{pid}] status file {name}: {shown}"));
                fields
            }
            Err(e) => {
                log.line(&format!("[{pid}] status file {name}: read FAILED {e}"));
                None
            }
        };
        let expected = fields.as_ref().and_then(proc_start_of);
        check_process(log, pid, expected, opts.wait_ms);
        if let Some(sid) = fields
            .as_ref()
            .and_then(|f| f.get("sessionId"))
            .and_then(Value::as_str)
        {
            check_transcript(log, opts.home, pid, sid);
        }
    }
    for &pid in opts.extra_pids {
        log.line(&format!("[{pid}] extra process, no status file"));
        check_process(log, pid, None, opts.wait_ms);
    }
    if opts.notify_ms > 0 {
        log.line(&format!(
            "change notifications on {} for {} ms",
            dir.display(),
            opts.notify_ms
        ));
        let mut status_events = 0;
        let result = watch_dir(&dir, opts.notify_ms, &mut |action, name| {
            // Only status file names are shown; other names in that folder belong to `.key` files.
            if status_file_pid(name).is_some() {
                status_events += 1;
                log.line(&format!("  {} {name}", action_name(action)));
            } else {
                log.line(&format!("  {} (other file)", action_name(action)));
            }
        });
        match result {
            Ok(n) => log.line(&format!(
                "change notifications: {n} events, {status_events} on status files"
            )),
            Err(e) => log.line(&format!("change notifications: {e}")),
        }
    }
    log.line("probe done");
}

/// Waits on one process handle until it exits or `timeout_ms` passes.
pub fn wait_exit(log: &mut Logger, pid: u32, timeout_ms: u32) {
    log_self(log);
    let h = match open(pid, PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE) {
        Ok(h) => h,
        Err(e) => {
            log.line(&format!(
                "[{pid}] OpenProcess QUERY_LIMITED_INFORMATION|SYNCHRONIZE FAILED {}",
                hex(e)
            ));
            return;
        }
    };
    let created =
        creation_time(h.0).map_or_else(|e| format!("FAILED {}", hex(e)), |t| t.to_string());
    log.line(&format!(
        "[{pid}] handle open, creation time {created}; waiting up to {timeout_ms} ms"
    ));
    match wait(h.0, timeout_ms) {
        Waited::Signaled(d) => {
            let code =
                exit_code(h.0).map_or_else(|e| format!("FAILED {}", hex(e)), |c| c.to_string());
            log.line(&format!(
                "[{pid}] signaled after {} ms, exit code {code}",
                d.as_millis()
            ));
        }
        Waited::TimedOut => log.line(&format!("[{pid}] still running after {timeout_ms} ms")),
        Waited::Failed(r) => log.line(&format!("[{pid}] wait FAILED return {r}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::names::opened;
    use crate::procs::{Liveness, check};

    fn own_start() -> u64 {
        match check(std::process::id()) {
            Liveness::Running(t) => t,
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn integrity_names_cover_the_usual_levels() {
        assert_eq!(integrity_name(0x2000), "Medium");
        assert_eq!(integrity_name(0x3000), "High");
        assert_eq!(integrity_name(0x1234), "0x1234");
    }

    #[test]
    fn own_token_reads_the_same_through_the_process_handle() {
        let own = TokenFacts::of_self().expect("own token");
        let rid = own.integrity.expect("own integrity");
        assert!(rid >= 0x1000, "{rid:#x}");
        let h = open(std::process::id(), PROCESS_QUERY_LIMITED_INFORMATION).expect("open self");
        let via_handle = TokenFacts::of_process(h.0).expect("token via handle");
        assert_eq!(via_handle.integrity, Ok(rid));
        assert!(own.describe().contains(&integrity_name(rid)));
    }

    #[test]
    fn system_list_create_time_matches_process_times() {
        let map = system_create_times().expect("process list");
        assert_eq!(map.get(&std::process::id()), Some(&own_start()));
        assert!(map.len() > 10);
    }

    #[test]
    fn notify_records_are_parsed() {
        let mut bytes = Vec::new();
        let name: Vec<u8> = "42.json"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect();
        let first_len = (12 + name.len()).next_multiple_of(4);
        bytes.extend_from_slice(&(first_len as u32).to_le_bytes());
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&(name.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&name);
        bytes.resize(first_len, 0);
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&2u32.to_le_bytes());
        bytes.extend_from_slice(&(name.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&name);
        assert_eq!(
            parse_notify(&bytes),
            vec![(1, "42.json".to_string()), (2, "42.json".to_string())]
        );
    }

    #[test]
    fn dir_watch_sees_a_file_written_by_another_thread() {
        let dir = std::env::temp_dir().join(format!("claude-watch-notify-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let target = dir.join("4242.json");
        let writer = {
            let target = target.clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(200));
                fs::write(&target, b"{}").unwrap();
            })
        };
        let mut events = Vec::new();
        let count =
            watch_dir(&dir, 1200, &mut |a, n| events.push((a, n.to_string()))).expect("watch");
        writer.join().unwrap();
        let _ = fs::remove_dir_all(&dir);
        assert_eq!(count, events.len());
        assert!(
            events.iter().any(|(a, n)| *a == 1 && n == "4242.json"),
            "{events:?}"
        );
    }

    #[test]
    fn wait_sees_an_exited_child_and_times_out_on_self() {
        let mut child = std::process::Command::new("cmd.exe")
            .args(["/c", "exit 3"])
            .spawn()
            .expect("spawn cmd");
        let h = open(
            child.id(),
            PROCESS_SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION,
        )
        .expect("open child");
        assert!(wait(h.0, 5000).is_signaled());
        assert_eq!(exit_code(h.0), Ok(3));
        child.wait().unwrap();
        let me = open(std::process::id(), PROCESS_SYNCHRONIZE).expect("open self");
        assert!(matches!(wait(me.0, 0), Waited::TimedOut));
    }

    #[test]
    fn probe_opens_only_pid_json_and_checks_the_process() {
        let root = std::env::temp_dir().join(format!("claude-watch-probe-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let sessions = root.join(".claude").join("sessions");
        fs::create_dir_all(&sessions).unwrap();
        fs::create_dir_all(root.join(".claude").join("projects").join("C--x")).unwrap();
        let pid = std::process::id();
        let sid = "bbbbbbbb-2222-3333-4444-555555555555";
        fs::write(
            sessions.join(format!("{pid}.json")),
            format!(
                r#"{{"pid":{pid},"sessionId":"{sid}","procStart":"{}","status":"busy"}}"#,
                own_start()
            ),
        )
        .unwrap();
        fs::write(sessions.join("777.abcdef.key"), r#"{"pid":777}"#).unwrap();
        fs::write(
            root.join(".claude")
                .join("projects")
                .join("C--x")
                .join(format!("{sid}.jsonl")),
            "{}\n{}\n",
        )
        .unwrap();
        let mut log = Logger::capture();
        opened::take();
        run(
            &mut log,
            &ProbeOptions {
                home: &root,
                extra_pids: &[],
                wait_ms: 10,
                notify_ms: 0,
            },
        );
        let opened_names = opened::take();
        let _ = fs::remove_dir_all(&root);
        assert_eq!(opened_names, vec![format!("{pid}.json")]);
        let out = log.captured.unwrap();
        let has = |s: &str| out.iter().any(|l| l.contains(s));
        assert!(has("status=busy"), "{out:#?}");
        assert!(has("OpenProcess QUERY_LIMITED_INFORMATION|SYNCHRONIZE: ok"));
        assert!(has("creation time") && has("equals procStart"));
        assert!(has("timed out after 10 ms (still running)"));
        assert!(has("watcher verdict: alive, start time matches"));
        assert!(has("read ok, 6 bytes, 2 lines"));
        assert!(!out.iter().any(|l| l.contains("abcdef")));
    }
}
