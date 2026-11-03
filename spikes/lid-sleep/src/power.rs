use windows::Win32::Foundation::{
    CloseHandle, ERROR_NO_MORE_ITEMS, ERROR_SUCCESS, GetLastError, HANDLE, HLOCAL, LPARAM,
    LocalFree, SetLastError, WIN32_ERROR, WPARAM,
};
use windows::Win32::System::Power::*;
use windows::Win32::System::Registry::HKEY;
use windows::Win32::System::SystemInformation::GetSystemTimeAsFileTime;
use windows::Win32::System::SystemServices::POWER_REQUEST_CONTEXT_VERSION;
use windows::Win32::System::Threading::{
    CREATE_WAITABLE_TIMER_MANUAL_RESET, CreateWaitableTimerExW,
    POWER_REQUEST_CONTEXT_SIMPLE_STRING, REASON_CONTEXT, REASON_CONTEXT_0, SetWaitableTimer,
    TIMER_ALL_ACCESS,
};
use windows::Win32::UI::WindowsAndMessaging::{
    HWND_BROADCAST, SC_MONITORPOWER, SMTO_ABORTIFHUNG, SendMessageTimeoutW, WM_SYSCOMMAND,
};
use windows::core::{GUID, PCWSTR, PWSTR};

pub const SUB_BUTTONS: GUID = GUID::from_u128(0x4f971e89_eebd_4455_a8de_9e59040e7347);
pub const LIDACTION: GUID = GUID::from_u128(0x5ca83367_6e45_459f_a27b_476b1d01c936);
pub const SUB_SLEEP: GUID = GUID::from_u128(0x238c9fa8_0aad_41ed_83f4_97be242c8f20);
pub const STANDBYIDLE: GUID = GUID::from_u128(0x29f6c1db_86da_48c5_9fdb_f2b67b1f44da);
pub const HIBERNATEIDLE: GUID = GUID::from_u128(0x9d7815a6_7ee4_497e_8888_515a05f02364);
pub const RTCWAKE: GUID = GUID::from_u128(0xbd3b718a_0680_4d9d_8ab2_e1d2b4ac806d);
pub const SUB_VIDEO: GUID = GUID::from_u128(0x7516b95f_f776_4464_8c53_06167f40cc99);
pub const VIDEOIDLE: GUID = GUID::from_u128(0x3c0bc021_c8a8_4e07_a973_6b14cbcb2b7e);
pub const SUB_NONE: GUID = GUID::from_u128(0xfea3413e_7e05_4911_9a71_700331f1c294);
pub const CONSOLELOCK: GUID = GUID::from_u128(0x0e796bdb_100d_47d6_a2d5_f7d2daa51f51);
pub const SUB_ENERGYSAVER: GUID = GUID::from_u128(0xde830923_a562_41af_a086_e3a2c6bad2da);
pub const ESBATTTHRESHOLD: GUID = GUID::from_u128(0xe69653ca_cf7f_4f05_aa73_cb833fa90ad4);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Ac,
    Dc,
}

impl Source {
    pub fn name(self) -> &'static str {
        match self {
            Source::Ac => "ac",
            Source::Dc => "dc",
        }
    }
}

pub const BOTH: [Source; 2] = [Source::Ac, Source::Dc];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Setting {
    pub name: &'static str,
    pub sub: GUID,
    pub guid: GUID,
}

pub const LID: Setting = Setting {
    name: "lid",
    sub: SUB_BUTTONS,
    guid: LIDACTION,
};
pub const SLEEP: Setting = Setting {
    name: "sleep",
    sub: SUB_SLEEP,
    guid: STANDBYIDLE,
};
pub const HIBERNATE_AFTER: Setting = Setting {
    name: "hibernate-after",
    sub: SUB_SLEEP,
    guid: HIBERNATEIDLE,
};
pub const WAKE_TIMERS: Setting = Setting {
    name: "wake-timers",
    sub: SUB_SLEEP,
    guid: RTCWAKE,
};
pub const DISPLAY_OFF: Setting = Setting {
    name: "display-off",
    sub: SUB_VIDEO,
    guid: VIDEOIDLE,
};
pub const SIGN_IN_AFTER_SLEEP: Setting = Setting {
    name: "consolelock",
    sub: SUB_NONE,
    guid: CONSOLELOCK,
};
pub const ENERGY_SAVER_THRESHOLD: Setting = Setting {
    name: "energy-saver-threshold",
    sub: SUB_ENERGYSAVER,
    guid: ESBATTTHRESHOLD,
};

pub const WATCHED: [Setting; 7] = [
    LID,
    SLEEP,
    DISPLAY_OFF,
    HIBERNATE_AFTER,
    WAKE_TIMERS,
    SIGN_IN_AFTER_SLEEP,
    ENERGY_SAVER_THRESHOLD,
];

pub fn guid_str(g: &GUID) -> String {
    format!("{g:?}").to_lowercase()
}

pub fn parse_guid(s: &str) -> Option<GUID> {
    let hex: String = s.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    if hex.len() != 32 {
        return None;
    }
    u128::from_str_radix(&hex, 16).ok().map(GUID::from_u128)
}

pub fn active_scheme() -> Result<GUID, u32> {
    let mut p: *mut GUID = std::ptr::null_mut();
    let rc = unsafe { PowerGetActiveScheme(None, &mut p) };
    if rc != ERROR_SUCCESS || p.is_null() {
        return Err(rc.0);
    }
    let g = unsafe { *p };
    unsafe { LocalFree(Some(HLOCAL(p as _))) };
    Ok(g)
}

pub fn read(scheme: &GUID, s: Setting, src: Source) -> Result<u32, u32> {
    let mut v = 0u32;
    let rc = unsafe {
        match src {
            Source::Ac => {
                PowerReadACValueIndex(None, Some(scheme), Some(&s.sub), Some(&s.guid), &mut v).0
            }
            Source::Dc => {
                PowerReadDCValueIndex(None, Some(scheme), Some(&s.sub), Some(&s.guid), &mut v)
            }
        }
    };
    if rc == 0 { Ok(v) } else { Err(rc) }
}

pub fn write(scheme: &GUID, s: Setting, src: Source, value: u32) -> u32 {
    unsafe {
        match src {
            Source::Ac => {
                PowerWriteACValueIndex(None, scheme, Some(&s.sub), Some(&s.guid), value).0
            }
            Source::Dc => PowerWriteDCValueIndex(None, scheme, Some(&s.sub), Some(&s.guid), value),
        }
    }
}

pub fn set_active(scheme: &GUID) -> u32 {
    unsafe { PowerSetActiveScheme(None::<HKEY>, Some(scheme)).0 }
}

pub fn schemes() -> Vec<GUID> {
    let mut out = Vec::new();
    for index in 0.. {
        let mut buf = [0u8; 16];
        let mut size = buf.len() as u32;
        let rc = unsafe {
            PowerEnumerate(
                None,
                None,
                None,
                ACCESS_SCHEME,
                index,
                Some(buf.as_mut_ptr()),
                &mut size,
            )
        };
        if rc == ERROR_NO_MORE_ITEMS || rc != ERROR_SUCCESS {
            break;
        }
        out.push(unsafe { std::ptr::read_unaligned(buf.as_ptr() as *const GUID) });
    }
    out
}

pub fn friendly_name(scheme: &GUID) -> String {
    let mut size = 0u32;
    let rc = unsafe { PowerReadFriendlyName(None, Some(scheme), None, None, None, &mut size) };
    if rc != ERROR_SUCCESS || size == 0 {
        return format!("<name rc {}>", rc.0);
    }
    let mut buf = vec![0u8; size as usize];
    let rc = unsafe {
        PowerReadFriendlyName(
            None,
            Some(scheme),
            None,
            None,
            Some(buf.as_mut_ptr()),
            &mut size,
        )
    };
    if rc != ERROR_SUCCESS {
        return format!("<name rc {}>", rc.0);
    }
    let wide: Vec<u16> = buf
        .as_chunks::<2>()
        .0
        .iter()
        .map(|c| u16::from_le_bytes(*c))
        .take_while(|&c| c != 0)
        .collect();
    String::from_utf16_lossy(&wide)
}

pub fn write_friendly_name(scheme: &GUID, name: &str) -> u32 {
    let bytes: Vec<u8> = name
        .encode_utf16()
        .chain(std::iter::once(0))
        .flat_map(|c| c.to_le_bytes())
        .collect();
    unsafe { PowerWriteFriendlyName(None, scheme, None, None, &bytes).0 }
}

pub fn duplicate(src: &GUID) -> Result<GUID, u32> {
    let mut p: *mut GUID = std::ptr::null_mut();
    let rc: WIN32_ERROR = unsafe { PowerDuplicateScheme(None, src, &mut p) };
    if rc != ERROR_SUCCESS || p.is_null() {
        return Err(rc.0);
    }
    let g = unsafe { *p };
    unsafe { LocalFree(Some(HLOCAL(p as _))) };
    Ok(g)
}

pub fn delete(scheme: &GUID) -> u32 {
    unsafe { PowerDeleteScheme(None, scheme).0 }
}

pub fn capabilities() -> Option<SYSTEM_POWER_CAPABILITIES> {
    let mut c = SYSTEM_POWER_CAPABILITIES::default();
    if unsafe { GetPwrCapabilities(&mut c) } {
        Some(c)
    } else {
        None
    }
}

pub fn hibernate_allowed() -> bool {
    unsafe { IsPwrHibernateAllowed() }
}

pub fn suspend_allowed() -> bool {
    unsafe { IsPwrSuspendAllowed() }
}

/// Calls SetSuspendState and returns (returned TRUE, last error). On success the call returns
/// only after the PC wakes again, so `ms` is the time spent inside it.
pub fn set_suspend_state(hibernate: bool) -> (bool, u32, u128) {
    let t = std::time::Instant::now();
    unsafe { SetLastError(WIN32_ERROR(0)) };
    let ok = unsafe { SetSuspendState(hibernate, false, false) };
    let e = unsafe { GetLastError().0 };
    (ok, e, t.elapsed().as_millis())
}

/// The classic way to turn the screen off; on Modern Standby it starts a standby session.
pub fn screen_off() -> (isize, u32) {
    let mut result = 0usize;
    unsafe { SetLastError(WIN32_ERROR(0)) };
    let r = unsafe {
        SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_SYSCOMMAND,
            WPARAM(SC_MONITORPOWER as usize),
            LPARAM(2),
            SMTO_ABORTIFHUNG,
            2000,
            Some(&mut result),
        )
    };
    (r.0, unsafe { GetLastError().0 })
}

fn reason(text: &str) -> (Vec<u16>, REASON_CONTEXT) {
    let mut wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    let ctx = REASON_CONTEXT {
        Version: POWER_REQUEST_CONTEXT_VERSION,
        Flags: POWER_REQUEST_CONTEXT_SIMPLE_STRING,
        Reason: REASON_CONTEXT_0 {
            SimpleReasonString: PWSTR(wide.as_mut_ptr()),
        },
    };
    (wide, ctx)
}

/// What a test holds while "work" runs. Drop releases it, so an early return cannot leave a
/// request behind.
pub enum Hold {
    /// The reason text stays alive with the handle, as Windows may read it later.
    Request {
        handle: HANDLE,
        _reason: Vec<u16>,
    },
    Stes,
}

impl Hold {
    pub fn request(text: &str) -> Result<Hold, String> {
        let (wide, ctx) = reason(text);
        let h =
            unsafe { PowerCreateRequest(&ctx) }.map_err(|e| format!("PowerCreateRequest {e}"))?;
        let r1 = unsafe { PowerSetRequest(h, PowerRequestSystemRequired) };
        let r2 = unsafe { PowerSetRequest(h, PowerRequestExecutionRequired) };
        crate::log!(
            "hold",
            "PowerSetRequest SystemRequired {r1:?}, ExecutionRequired {r2:?}"
        );
        Ok(Hold::Request {
            handle: h,
            _reason: wide,
        })
    }

    pub fn stes() -> Hold {
        let prev = unsafe { SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED) };
        crate::log!(
            "hold",
            "SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED) previous {:#x}",
            prev.0
        );
        Hold::Stes
    }

    /// The PowerToys Awake fix: clear and set again after a resume.
    pub fn retake(&self) {
        match self {
            Hold::Request { handle: h, .. } => unsafe {
                let c1 = PowerClearRequest(*h, PowerRequestSystemRequired);
                let c2 = PowerClearRequest(*h, PowerRequestExecutionRequired);
                let s1 = PowerSetRequest(*h, PowerRequestSystemRequired);
                let s2 = PowerSetRequest(*h, PowerRequestExecutionRequired);
                crate::log!("hold", "retake: clear {c1:?} {c2:?}, set {s1:?} {s2:?}");
            },
            Hold::Stes => unsafe {
                SetThreadExecutionState(ES_CONTINUOUS);
                let prev = SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED);
                crate::log!("hold", "retake: STES again, previous {:#x}", prev.0);
            },
        }
    }
}

impl Drop for Hold {
    fn drop(&mut self) {
        match self {
            Hold::Request { handle: h, .. } => unsafe {
                let c1 = PowerClearRequest(*h, PowerRequestSystemRequired);
                let c2 = PowerClearRequest(*h, PowerRequestExecutionRequired);
                let _ = CloseHandle(*h);
                crate::log!("hold", "released: clear {c1:?} {c2:?}, handle closed");
            },
            Hold::Stes => unsafe {
                SetThreadExecutionState(ES_CONTINUOUS);
                crate::log!("hold", "released: STES cleared");
            },
        }
    }
}

/// A waitable timer that asks Windows to wake the PC when it fires.
pub struct WakeTimer {
    pub handle: HANDLE,
    pub set_ok: bool,
    /// 50 (ERROR_NOT_SUPPORTED) after a successful set means the PC cannot be woken by it.
    pub last_error: u32,
}

impl WakeTimer {
    pub fn arm(after_s: u64) -> Result<WakeTimer, String> {
        let h = unsafe {
            CreateWaitableTimerExW(
                None,
                PCWSTR::null(),
                CREATE_WAITABLE_TIMER_MANUAL_RESET,
                TIMER_ALL_ACCESS.0,
            )
        }
        .map_err(|e| format!("CreateWaitableTimerExW {e}"))?;
        // Absolute UTC time: since Windows 8 a relative due time does not count down while the
        // PC sleeps, so it could never wake it on time.
        let now = unsafe { GetSystemTimeAsFileTime() };
        let now = ((now.dwHighDateTime as i64) << 32) | now.dwLowDateTime as i64;
        let due: i64 = now + (after_s as i64) * 10_000_000;
        unsafe { SetLastError(WIN32_ERROR(0)) };
        let r = unsafe { SetWaitableTimer(h, &due, 0, None, None, true) };
        let e = unsafe { GetLastError().0 };
        Ok(WakeTimer {
            handle: h,
            set_ok: r.is_ok(),
            last_error: e,
        })
    }

    pub fn fired(&self) -> bool {
        use windows::Win32::Foundation::WAIT_OBJECT_0;
        use windows::Win32::System::Threading::WaitForSingleObject;
        unsafe { WaitForSingleObject(self.handle, 0) == WAIT_OBJECT_0 }
    }
}

impl Drop for WakeTimer {
    fn drop(&mut self) {
        unsafe {
            let _ = windows::Win32::System::Threading::CancelWaitableTimer(self.handle);
            let _ = CloseHandle(self.handle);
        }
    }
}

pub fn lid_action_name(v: u32) -> &'static str {
    match v {
        0 => "Do nothing",
        1 => "Sleep",
        2 => "Hibernate",
        3 => "Shut down",
        _ => "unknown",
    }
}

pub fn wake_timers_name(v: u32) -> &'static str {
    match v {
        0 => "Disable",
        1 => "Enable",
        2 => "Important wake timers only",
        _ => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guid_round_trip() {
        let g = GUID::from_u128(0x381b4222_f694_41f0_9685_ff5bb260df2e);
        let s = guid_str(&g);
        assert_eq!(s, "381b4222-f694-41f0-9685-ff5bb260df2e");
        assert_eq!(parse_guid(&s), Some(g));
        assert_eq!(
            parse_guid("{381B4222-F694-41F0-9685-FF5BB260DF2E}"),
            Some(g)
        );
        assert_eq!(parse_guid("nope"), None);
    }

    #[test]
    fn names() {
        assert_eq!(lid_action_name(0), "Do nothing");
        assert_eq!(lid_action_name(1), "Sleep");
        assert_eq!(wake_timers_name(2), "Important wake timers only");
    }
}
