use std::os::windows::process::CommandExt;
use std::process::Command;

use windows::Win32::Foundation::{
    CloseHandle, GetLastError, HANDLE, LUID, SetLastError, WAIT_OBJECT_0, WIN32_ERROR,
};
use windows::Win32::Security::{
    AdjustTokenPrivileges, GetSidSubAuthority, GetSidSubAuthorityCount, GetTokenInformation,
    LUID_AND_ATTRIBUTES, LookupPrivilegeValueW, SE_PRIVILEGE_ENABLED, SE_SHUTDOWN_NAME,
    TOKEN_ADJUST_PRIVILEGES, TOKEN_ELEVATION, TOKEN_MANDATORY_LABEL, TOKEN_PRIVILEGES, TOKEN_QUERY,
    TokenElevation, TokenIntegrityLevel,
};
use windows::Win32::System::Power::{
    CallNtPowerInformation, GetSystemPowerStatus, LastSleepTime, LastWakeTime,
    POWER_INFORMATION_LEVEL, SYSTEM_POWER_INFORMATION, SYSTEM_POWER_STATUS, SystemExecutionState,
    SystemPowerInformation,
};
use windows::Win32::System::Registry::{HKEY, HKEY_LOCAL_MACHINE, RRF_RT_REG_DWORD, RegGetValueW};
use windows::Win32::System::RemoteDesktop::{
    WTS_CURRENT_SESSION, WTSFreeMemory, WTSINFOEXW, WTSQuerySessionInformationW, WTSSessionInfoEx,
};
use windows::Win32::System::SystemInformation::GetTickCount;
use windows::Win32::System::Threading::*;
use windows::Win32::System::WindowsProgramming::{QueryInterruptTime, QueryUnbiasedInterruptTime};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};
use windows::Win32::UI::WindowsAndMessaging::{GetShellWindow, GetWindowThreadProcessId};
use windows::core::{HSTRING, PWSTR};

pub struct TokenInfo {
    pub elevated: bool,
    pub integrity_rid: u32,
}

impl TokenInfo {
    pub fn integrity_name(&self) -> &'static str {
        match self.integrity_rid {
            0x1000 => "Low",
            0x2000 => "Medium",
            0x2100 => "Medium Plus",
            0x3000 => "High",
            0x4000 => "System",
            _ => "other",
        }
    }
}

pub fn token_info() -> TokenInfo {
    unsafe {
        let mut token = HANDLE::default();
        OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).expect("OpenProcessToken");
        let mut elevation = TOKEN_ELEVATION::default();
        let mut len = 0u32;
        let _ = GetTokenInformation(
            token,
            TokenElevation,
            Some(&mut elevation as *mut _ as *mut _),
            size_of::<TOKEN_ELEVATION>() as u32,
            &mut len,
        );
        let mut buf = vec![0u8; 256];
        let mut rid = 0;
        if GetTokenInformation(
            token,
            TokenIntegrityLevel,
            Some(buf.as_mut_ptr() as *mut _),
            buf.len() as u32,
            &mut len,
        )
        .is_ok()
        {
            let label = &*(buf.as_ptr() as *const TOKEN_MANDATORY_LABEL);
            let sid = label.Label.Sid;
            let count = *GetSidSubAuthorityCount(sid);
            rid = *GetSidSubAuthority(sid, (count - 1) as u32);
        }
        let _ = CloseHandle(token);
        TokenInfo {
            elevated: elevation.TokenIsElevated != 0,
            integrity_rid: rid,
        }
    }
}

/// SetSuspendState needs SeShutdownPrivilege enabled. Standard tokens hold it disabled.
pub fn enable_shutdown_privilege() -> (bool, String) {
    unsafe {
        let mut token = HANDLE::default();
        if let Err(e) = OpenProcessToken(
            GetCurrentProcess(),
            TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY,
            &mut token,
        ) {
            return (
                false,
                format!("OpenProcessToken(ADJUST_PRIVILEGES) failed: {e}"),
            );
        }
        let mut luid = LUID::default();
        if let Err(e) = LookupPrivilegeValueW(None, SE_SHUTDOWN_NAME, &mut luid) {
            let _ = CloseHandle(token);
            return (false, format!("LookupPrivilegeValueW failed: {e}"));
        }
        let tp = TOKEN_PRIVILEGES {
            PrivilegeCount: 1,
            Privileges: [LUID_AND_ATTRIBUTES {
                Luid: luid,
                Attributes: SE_PRIVILEGE_ENABLED,
            }],
        };
        SetLastError(WIN32_ERROR(0));
        let r = AdjustTokenPrivileges(token, false, Some(&tp), 0, None, None);
        // Success with 1300 (ERROR_NOT_ALL_ASSIGNED) means the token does not hold it.
        let e = GetLastError().0;
        let _ = CloseHandle(token);
        match r {
            Ok(()) => (e == 0, format!("AdjustTokenPrivileges ok, last error {e}")),
            Err(err) => (false, format!("AdjustTokenPrivileges failed: {err}")),
        }
    }
}

pub fn idle_ms() -> u32 {
    let mut lii = LASTINPUTINFO {
        cbSize: size_of::<LASTINPUTINFO>() as u32,
        dwTime: 0,
    };
    unsafe {
        if !GetLastInputInfo(&mut lii).as_bool() {
            return u32::MAX;
        }
        GetTickCount().wrapping_sub(lii.dwTime)
    }
}

pub struct PowerStatus {
    pub on_ac: bool,
    pub battery_percent: u8,
    pub energy_saver_on: bool,
}

pub fn power_status() -> PowerStatus {
    let mut s = SYSTEM_POWER_STATUS::default();
    let _ = unsafe { GetSystemPowerStatus(&mut s) };
    PowerStatus {
        on_ac: s.ACLineStatus == 1,
        battery_percent: s.BatteryLifePercent,
        energy_saver_on: s.SystemStatusFlag == 1,
    }
}

/// Does not count time in sleep, so a jump against the wall clock shows a sleep.
pub fn unbiased_ms() -> u64 {
    let mut t = 0u64;
    let _ = unsafe { QueryUnbiasedInterruptTime(&mut t) };
    t / 10_000
}

/// Counts time in sleep too, unlike `unbiased_ms`.
pub fn interrupt_ms() -> u64 {
    unsafe { QueryInterruptTime() / 10_000 }
}

fn nt_power<T: Default>(level: POWER_INFORMATION_LEVEL) -> Result<T, i32> {
    let mut v = T::default();
    let st = unsafe {
        CallNtPowerInformation(
            level,
            None,
            0,
            Some(&mut v as *mut T as *mut _),
            size_of::<T>() as u32,
        )
    };
    if st.0 == 0 { Ok(v) } else { Err(st.0) }
}

/// Interrupt time (100 ns units) of the last sleep and wake, as CallNtPowerInformation gives them.
pub fn last_sleep_wake() -> (Result<u64, i32>, Result<u64, i32>) {
    (
        nt_power::<u64>(LastSleepTime),
        nt_power::<u64>(LastWakeTime),
    )
}

/// The ES_* flags the system reports as in force. Lets a normal user see a held request without
/// `powercfg /requests`, if it works.
pub fn system_execution_state() -> Result<u32, i32> {
    nt_power::<u32>(SystemExecutionState)
}

/// TimeRemaining is the idle sleep countdown in seconds.
pub fn system_power_information() -> Result<SYSTEM_POWER_INFORMATION, i32> {
    nt_power::<SYSTEM_POWER_INFORMATION>(SystemPowerInformation)
}

/// Some(true) when the console session is locked. WTS SessionFlags: 0 lock, 1 unlock.
pub fn session_locked() -> Option<bool> {
    unsafe {
        let mut buf = PWSTR::null();
        let mut bytes = 0u32;
        if WTSQuerySessionInformationW(
            None,
            WTS_CURRENT_SESSION,
            WTSSessionInfoEx,
            &mut buf,
            &mut bytes,
        )
        .is_err()
            || buf.is_null()
        {
            return None;
        }
        let info = &*(buf.0 as *const WTSINFOEXW);
        let flags = if info.Level == 1 {
            Some(info.Data.WTSInfoExLevel1.SessionFlags)
        } else {
            None
        };
        WTSFreeMemory(buf.0 as *mut _);
        match flags {
            Some(0) => Some(true),
            Some(1) => Some(false),
            _ => None,
        }
    }
}

pub fn reg_dword(root: HKEY, key: &str, value: &str) -> Option<u32> {
    let mut v = 0u32;
    let mut size = 4u32;
    let rc = unsafe {
        RegGetValueW(
            root,
            &HSTRING::from(key),
            &HSTRING::from(value),
            RRF_RT_REG_DWORD,
            None,
            Some(&mut v as *mut _ as *mut _),
            Some(&mut size),
        )
    };
    if rc.is_err() { None } else { Some(v) }
}

pub fn hklm() -> HKEY {
    HKEY_LOCAL_MACHINE
}

pub const POWER_KEY: &str = r"SYSTEM\CurrentControlSet\Control\Power";

/// Runs a command without a window and returns its trimmed output.
pub fn run(program: &str, args: &[&str]) -> (i32, String) {
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    match Command::new(program)
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
    {
        Ok(o) => {
            let mut text = String::from_utf8_lossy(&o.stdout).into_owned();
            text.push_str(&String::from_utf8_lossy(&o.stderr));
            (o.status.code().unwrap_or(-1), text.trim().to_string())
        }
        Err(e) => (-1, format!("spawn failed: {e}")),
    }
}

/// Runs a PowerShell 5.1 snippet with this process's token, so a `limited` run checks WMI as a
/// normal user.
pub fn powershell(script: &str) -> (i32, String) {
    run(
        "powershell.exe",
        &["-NoProfile", "-NonInteractive", "-Command", script],
    )
}

/// Starts `exe args` with Explorer as the parent, so it gets Explorer's unelevated token, and waits
/// for it. Lets an elevated session test what a normal user can do.
pub fn run_as_explorer_child(exe: &str, args: &str, cwd: &str) -> Result<u32, String> {
    unsafe {
        let shell = GetShellWindow();
        let mut pid = 0u32;
        GetWindowThreadProcessId(shell, Some(&mut pid));
        if pid == 0 {
            return Err("no shell window".into());
        }
        let parent = OpenProcess(PROCESS_CREATE_PROCESS, false, pid)
            .map_err(|e| format!("OpenProcess explorer {pid}: {e}"))?;
        let mut size = 0usize;
        let _ = InitializeProcThreadAttributeList(None, 1, None, &mut size);
        let mut attr_buf = vec![0u8; size];
        let list = LPPROC_THREAD_ATTRIBUTE_LIST(attr_buf.as_mut_ptr() as *mut _);
        InitializeProcThreadAttributeList(Some(list), 1, None, &mut size)
            .map_err(|e| format!("InitializeProcThreadAttributeList: {e}"))?;
        UpdateProcThreadAttribute(
            list,
            0,
            PROC_THREAD_ATTRIBUTE_PARENT_PROCESS as usize,
            Some(&parent as *const HANDLE as *const _),
            size_of::<HANDLE>(),
            None,
            None,
        )
        .map_err(|e| format!("UpdateProcThreadAttribute: {e}"))?;
        let mut si = STARTUPINFOEXW::default();
        si.StartupInfo.cb = size_of::<STARTUPINFOEXW>() as u32;
        si.lpAttributeList = list;
        let mut cmdline: Vec<u16> = format!("\"{exe}\" {args}")
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let mut pi = PROCESS_INFORMATION::default();
        let created = CreateProcessW(
            &HSTRING::from(exe),
            Some(PWSTR(cmdline.as_mut_ptr())),
            None,
            None,
            false,
            EXTENDED_STARTUPINFO_PRESENT | CREATE_NO_WINDOW,
            None,
            &HSTRING::from(cwd),
            &si.StartupInfo,
            &mut pi,
        );
        DeleteProcThreadAttributeList(list);
        let _ = CloseHandle(parent);
        created.map_err(|e| format!("CreateProcessW: {e}"))?;
        crate::log!(
            "limited",
            "child pid {} started under explorer pid {pid}",
            pi.dwProcessId
        );
        let wait = WaitForSingleObject(pi.hProcess, INFINITE);
        let mut code = 0u32;
        let _ = GetExitCodeProcess(pi.hProcess, &mut code);
        let _ = CloseHandle(pi.hThread);
        let _ = CloseHandle(pi.hProcess);
        if wait != WAIT_OBJECT_0 {
            return Err(format!("wait failed {wait:?}"));
        }
        Ok(code)
    }
}
