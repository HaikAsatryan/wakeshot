use std::ffi::c_void;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use windows::Win32::Foundation::HANDLE;
use windows::Win32::System::Console::SetConsoleCtrlHandler;
use windows::Win32::System::Power::*;
use windows::Win32::System::SystemServices::{
    GUID_ACDC_POWER_SOURCE, GUID_BATTERY_PERCENTAGE_REMAINING, GUID_CONSOLE_DISPLAY_STATE,
    GUID_LIDSWITCH_STATE_CHANGE, GUID_SESSION_DISPLAY_STATUS,
};
use windows::Win32::UI::WindowsAndMessaging::{DEVICE_NOTIFY_CALLBACK, PBT_POWERSETTINGCHANGE};
use windows::core::{BOOL, GUID};

use crate::power::guid_str;

pub const GUID_ACTIVE_POWERSCHEME: GUID = GUID::from_u128(0x31f9f286_5084_42fe_b720_2b0264993763);

#[derive(Clone, Debug, PartialEq)]
pub enum Ev {
    Setting { name: &'static str, value: u32 },
    Scheme(GUID),
    Suspend(u32),
}

static QUEUE: Mutex<Vec<(Instant, Ev)>> = Mutex::new(Vec::new());

fn push(ev: Ev) {
    QUEUE.lock().unwrap().push((Instant::now(), ev));
}

/// Everything the callbacks saw since the last call, oldest first.
pub fn drain() -> Vec<(Instant, Ev)> {
    std::mem::take(&mut *QUEUE.lock().unwrap())
}

pub fn setting_name(g: &GUID) -> &'static str {
    match *g {
        GUID_LIDSWITCH_STATE_CHANGE => "lid",
        GUID_CONSOLE_DISPLAY_STATE => "display",
        GUID_ACDC_POWER_SOURCE => "power-source",
        GUID_BATTERY_PERCENTAGE_REMAINING => "battery",
        GUID_ACTIVE_POWERSCHEME => "active-scheme",
        GUID_SESSION_DISPLAY_STATUS => "session-display",
        _ => "other",
    }
}

pub fn value_text(name: &str, v: u32) -> String {
    let label = match (name, v) {
        ("lid", 0) => "closed",
        ("lid", 1) => "open",
        ("display", 0) => "off",
        ("display", 1) => "on",
        ("display", 2) => "dimmed",
        ("session-display", 0) => "off",
        ("session-display", 1) => "on",
        ("session-display", 2) => "dimmed",
        ("power-source", 0) => "AC",
        ("power-source", 1) => "DC",
        ("power-source", 2) => "short-term DC (UPS)",
        _ => "",
    };
    if label.is_empty() {
        v.to_string()
    } else {
        format!("{v} ({label})")
    }
}

pub fn suspend_name(kind: u32) -> &'static str {
    match kind {
        4 => "PBT_APMSUSPEND",
        7 => "PBT_APMRESUMESUSPEND",
        18 => "PBT_APMRESUMEAUTOMATIC",
        _ => "other",
    }
}

unsafe extern "system" fn on_setting(
    _ctx: *const c_void,
    kind: u32,
    setting: *const c_void,
) -> u32 {
    if kind != PBT_POWERSETTINGCHANGE || setting.is_null() {
        crate::log!("notify", "callback type {kind}");
        return 0;
    }
    unsafe {
        let s = setting as *const POWERBROADCAST_SETTING;
        let guid = (*s).PowerSetting;
        let len = (*s).DataLength as usize;
        let data = std::slice::from_raw_parts(std::ptr::addr_of!((*s).Data) as *const u8, len);
        let name = setting_name(&guid);
        match len {
            4 => {
                let v = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
                crate::log!("notify", "{name} = {}", value_text(name, v));
                push(Ev::Setting { name, value: v });
            }
            16 => {
                let g = std::ptr::read_unaligned(data.as_ptr() as *const GUID);
                crate::log!("notify", "{name} = {}", guid_str(&g));
                push(Ev::Scheme(g));
            }
            _ => crate::log!("notify", "{name} = {data:02x?}"),
        }
    }
    0
}

unsafe extern "system" fn on_suspend(_ctx: *const c_void, kind: u32, _s: *const c_void) -> u32 {
    crate::log!("suspend", "{} ({kind})", suspend_name(kind));
    push(Ev::Suspend(kind));
    0
}

/// Windows keeps the pointer for the whole registration, so the parameters must never move.
fn leaked_params(cb: PDEVICE_NOTIFY_CALLBACK_ROUTINE) -> HANDLE {
    let p: &'static mut DEVICE_NOTIFY_SUBSCRIBE_PARAMETERS =
        Box::leak(Box::new(DEVICE_NOTIFY_SUBSCRIBE_PARAMETERS {
            Callback: cb,
            Context: std::ptr::null_mut(),
        }));
    HANDLE(p as *mut _ as *mut _)
}

pub struct Subscriptions {
    settings: Vec<*mut c_void>,
    suspend: Option<*mut c_void>,
}

pub const DEFAULT_SETTINGS: [GUID; 6] = [
    GUID_LIDSWITCH_STATE_CHANGE,
    GUID_CONSOLE_DISPLAY_STATE,
    GUID_SESSION_DISPLAY_STATUS,
    GUID_ACDC_POWER_SOURCE,
    GUID_BATTERY_PERCENTAGE_REMAINING,
    GUID_ACTIVE_POWERSCHEME,
];

impl Subscriptions {
    /// Callbacks with no window (LidGuard's way). Each setting fires once at once with its current
    /// value.
    pub fn start(settings: &[GUID], suspend: bool) -> Subscriptions {
        let mut s = Subscriptions {
            settings: Vec::new(),
            suspend: None,
        };
        let params = leaked_params(Some(on_setting));
        for g in settings {
            let mut h: *mut c_void = std::ptr::null_mut();
            let t = Instant::now();
            let rc = unsafe {
                PowerSettingRegisterNotification(g, DEVICE_NOTIFY_CALLBACK, params, &mut h)
            };
            crate::log!(
                "setup",
                "PowerSettingRegisterNotification({}) rc {} in {} us",
                setting_name(g),
                rc.0,
                t.elapsed().as_micros()
            );
            if rc.0 == 0 {
                s.settings.push(h);
            }
        }
        if suspend {
            let mut h: *mut c_void = std::ptr::null_mut();
            let rc = unsafe {
                PowerRegisterSuspendResumeNotification(
                    DEVICE_NOTIFY_CALLBACK,
                    leaked_params(Some(on_suspend)),
                    &mut h,
                )
            };
            crate::log!(
                "setup",
                "PowerRegisterSuspendResumeNotification rc {}",
                rc.0
            );
            if rc.0 == 0 {
                s.suspend = Some(h);
            }
        }
        s
    }
}

impl Drop for Subscriptions {
    fn drop(&mut self) {
        unsafe {
            for h in self.settings.drain(..) {
                let _ = PowerSettingUnregisterNotification(HPOWERNOTIFY(h as isize));
            }
            if let Some(h) = self.suspend.take() {
                let _ = PowerUnregisterSuspendResumeNotification(HPOWERNOTIFY(h as isize));
            }
        }
    }
}

static STOP: AtomicBool = AtomicBool::new(false);
static CLEAN: AtomicBool = AtomicBool::new(false);

unsafe extern "system" fn on_ctrl(kind: u32) -> BOOL {
    crate::log!(
        "ctrl",
        "console control event {kind}: stopping and restoring"
    );
    STOP.store(true, Ordering::SeqCst);
    // Close ends the process when this returns, so give the main loop a few seconds to put the
    // plan back first. Logoff and shutdown never arrive here: this exe loads user32, and Windows
    // sends those two only to console apps that do not. A restart or sign-out mid-run therefore
    // leaves the copy active until `restore`.
    if kind >= 2 {
        let t = Instant::now();
        while !CLEAN.load(Ordering::SeqCst) && t.elapsed() < Duration::from_millis(4500) {
            std::thread::sleep(Duration::from_millis(50));
        }
    }
    BOOL(1)
}

pub fn install_ctrl_handler() {
    let r = unsafe { SetConsoleCtrlHandler(Some(on_ctrl), true) };
    crate::log!("setup", "SetConsoleCtrlHandler {r:?}");
}

pub fn stop_requested() -> bool {
    STOP.load(Ordering::SeqCst)
}

pub fn mark_clean() {
    CLEAN.store(true, Ordering::SeqCst);
}

/// Sleeps in short steps; false when a stop was requested.
pub fn nap(ms: u64) -> bool {
    let end = Instant::now() + Duration::from_millis(ms);
    while Instant::now() < end {
        if stop_requested() {
            return false;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    !stop_requested()
}

/// Power events from the System log since `since` (local time "yyyy-MM-dd HH:mm:ss").
pub fn dump_power_events(since: &str) {
    let script = format!(
        "$ErrorActionPreference='SilentlyContinue'; \
         Get-WinEvent -FilterHashtable @{{LogName='System'; ProviderName='Microsoft-Windows-Kernel-Power','Microsoft-Windows-Power-Troubleshooter'; StartTime=[datetime]'{since}'}} | \
         Sort-Object TimeCreated | ForEach-Object {{ \
           $m = ($_.Message -replace '\\s+',' '); if ($m.Length -gt 220) {{ $m = $m.Substring(0,220) }}; \
           $x = ''; if ($_.Id -eq 187) {{ $x = ' caller ' + (([xml]$_.ToXml()).Event.EventData.Data | Where-Object Name -eq 'ApiCallerName').'#text' }}; \
           '{{0:HH:mm:ss.fff}} {{1}} {{2}}: {{3}}{{4}}' -f $_.TimeCreated, $_.ProviderName.Replace('Microsoft-Windows-',''), $_.Id, $m, $x }}"
    );
    let (rc, out) = crate::sys::powershell(&script);
    if out.is_empty() {
        crate::log!("events", "none since {since} (rc {rc})");
    }
    for l in out.lines() {
        crate::log!("events", "{l}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels() {
        assert_eq!(value_text("lid", 0), "0 (closed)");
        assert_eq!(value_text("display", 2), "2 (dimmed)");
        assert_eq!(value_text("battery", 87), "87");
        assert_eq!(setting_name(&GUID_LIDSWITCH_STATE_CHANGE), "lid");
        assert_eq!(suspend_name(18), "PBT_APMRESUMEAUTOMATIC");
    }
}
