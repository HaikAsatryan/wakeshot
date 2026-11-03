use std::collections::BTreeMap;
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0};
use windows::Win32::System::Power::*;
use windows::Win32::System::Registry::{
    HKEY, KEY_NOTIFY, KEY_READ, REG_NOTIFY_CHANGE_LAST_SET, REG_NOTIFY_CHANGE_NAME, RegCloseKey,
    RegNotifyChangeKeyValue, RegOpenKeyExW,
};
use windows::Win32::System::Threading::{CreateEventW, WaitForSingleObject};
use windows::Win32::UI::WindowsAndMessaging::{DEVICE_NOTIFY_CALLBACK, PBT_POWERSETTINGCHANGE};
use windows::core::{GUID, HSTRING};

use crate::power::{self, Source, guid_str};
use crate::sys;

#[derive(Clone, Debug)]
pub struct Event {
    pub at: Instant,
    pub source: &'static str,
    pub text: String,
}

static EVENTS: Mutex<Vec<Event>> = Mutex::new(Vec::new());

pub fn push(source: &'static str, text: String) {
    crate::log::line(source, &text);
    EVENTS.lock().unwrap().push(Event {
        at: Instant::now(),
        source,
        text,
    });
}

pub fn events_since(t: Instant) -> Vec<Event> {
    EVENTS
        .lock()
        .unwrap()
        .iter()
        .filter(|e| e.at >= t)
        .cloned()
        .collect()
}

pub type Snapshot = BTreeMap<String, String>;

pub fn snapshot(full: bool) -> Snapshot {
    let mut s = Snapshot::new();
    let active = power::active_scheme();
    s.insert(
        "active".into(),
        match &active {
            Ok(g) => guid_str(g),
            Err(rc) => format!("<rc {rc}>"),
        },
    );
    let mut settings = vec![power::LID, power::SLEEP];
    if full {
        settings.extend([
            power::ENERGY_SAVER_THRESHOLD,
            power::SIGN_IN_AFTER_SLEEP,
            power::DISPLAY_OFF,
        ]);
    }
    if let Ok(g) = active {
        for st in settings {
            for src in [Source::Ac, Source::Dc] {
                let v = match power::read(&g, st, src) {
                    Ok(v) => v.to_string(),
                    Err(rc) => format!("<rc {rc}>"),
                };
                s.insert(format!("{}.{}", st.name, src.name()), v);
            }
        }
    }
    if full {
        let opt = |o: Option<String>| o.unwrap_or_else(|| "<absent>".into());
        s.insert(
            "DelayLockInterval".into(),
            opt(
                sys::reg_dword(sys::hkcu(), sys::DESKTOP_KEY, "DelayLockInterval")
                    .map(|v| v.to_string()),
            ),
        );
        for (k, v) in [
            ("overlay.ac.reg", "ActiveOverlayAcPowerScheme"),
            ("overlay.dc.reg", "ActiveOverlayDcPowerScheme"),
        ] {
            s.insert(
                k.into(),
                opt(sys::reg_string(sys::hklm(), sys::SCHEMES_KEY, v)),
            );
        }
        s.insert(
            "overlay.actual".into(),
            power::overlay("PowerGetActualOverlayScheme"),
        );
        s.insert(
            "overlay.effective".into(),
            power::overlay("PowerGetEffectiveOverlayScheme"),
        );
    }
    s
}

pub fn diff(old: &Snapshot, new: &Snapshot) -> Vec<String> {
    let mut out = Vec::new();
    for (k, v) in new {
        match old.get(k) {
            Some(o) if o == v => {}
            Some(o) => out.push(format!("{k} {o} -> {v}")),
            None => out.push(format!("{k} = {v}")),
        }
    }
    out
}

fn label(g: &GUID) -> String {
    let named = [
        (power::LIDACTION, "lid"),
        (power::STANDBYIDLE, "sleep"),
        (power::ESBATTTHRESHOLD, "energy-saver-threshold"),
        (power::CONSOLELOCK, "consolelock"),
        (power::VIDEOIDLE, "display-off"),
        (power::GUID_ACTIVE_POWERSCHEME, "active-scheme"),
        (power::GUID_ACDC_POWER_SOURCE, "power-source"),
        (power::GUID_CONSOLE_DISPLAY_STATE, "display-state"),
        (power::GUID_ENERGY_SAVER_STATUS, "energy-saver-status"),
        (power::GUID_POWERSCHEME_PERSONALITY, "personality"),
    ];
    named
        .iter()
        .find(|(n, _)| n == g)
        .map(|(_, s)| s.to_string())
        .unwrap_or_else(|| guid_str(g))
}

unsafe extern "system" fn on_setting(
    _ctx: *const c_void,
    kind: u32,
    setting: *const c_void,
) -> u32 {
    if kind != PBT_POWERSETTINGCHANGE || setting.is_null() {
        push("notify", format!("callback type {kind}"));
        return 0;
    }
    unsafe {
        let s = setting as *const POWERBROADCAST_SETTING;
        let guid = (*s).PowerSetting;
        let len = (*s).DataLength as usize;
        let data = std::slice::from_raw_parts(std::ptr::addr_of!((*s).Data) as *const u8, len);
        let value = match len {
            4 => u32::from_le_bytes([data[0], data[1], data[2], data[3]]).to_string(),
            16 => guid_str(&std::ptr::read_unaligned(data.as_ptr() as *const GUID)),
            _ => format!("{data:02x?}"),
        };
        push("notify", format!("{} = {value}", label(&guid)));
    }
    0
}

unsafe extern "system" fn on_suspend_resume(
    _ctx: *const c_void,
    kind: u32,
    _s: *const c_void,
) -> u32 {
    let name = match kind {
        4 => "PBT_APMSUSPEND",
        7 => "PBT_APMRESUMESUSPEND",
        18 => "PBT_APMRESUMEAUTOMATIC",
        _ => "other",
    };
    push("suspend", format!("{name} ({kind})"));
    0
}

unsafe extern "system" fn on_effective_mode(mode: EFFECTIVE_POWER_MODE, _ctx: *const c_void) {
    push(
        "effmode",
        format!("{} ({})", power::effective_mode_name(mode.0), mode.0),
    );
}

/// Windows keeps the pointer for the whole registration, so the parameters must never move or drop.
fn leaked_params(cb: PDEVICE_NOTIFY_CALLBACK_ROUTINE) -> HANDLE {
    let p: &'static mut DEVICE_NOTIFY_SUBSCRIBE_PARAMETERS =
        Box::leak(Box::new(DEVICE_NOTIFY_SUBSCRIBE_PARAMETERS {
            Callback: cb,
            Context: std::ptr::null_mut(),
        }));
    HANDLE(p as *mut _ as *mut _)
}

pub struct Options {
    pub notify: Vec<GUID>,
    pub registry: bool,
    pub poll_ms: Option<u64>,
    pub full: bool,
    pub effective_mode: bool,
    pub suspend_resume: bool,
}

pub struct Detectors {
    stop: Arc<AtomicBool>,
    threads: Vec<JoinHandle<()>>,
    setting_handles: Vec<*mut c_void>,
    effmode_handle: Option<*mut c_void>,
    suspend_handle: Option<*mut c_void>,
}

impl Detectors {
    pub fn start(o: Options) -> Detectors {
        let stop = Arc::new(AtomicBool::new(false));
        let mut d = Detectors {
            stop: stop.clone(),
            threads: Vec::new(),
            setting_handles: Vec::new(),
            effmode_handle: None,
            suspend_handle: None,
        };
        let setting_params = leaked_params(Some(on_setting));
        for g in &o.notify {
            let mut h: *mut c_void = std::ptr::null_mut();
            let rc = unsafe {
                PowerSettingRegisterNotification(g, DEVICE_NOTIFY_CALLBACK, setting_params, &mut h)
            };
            crate::log!(
                "setup",
                "PowerSettingRegisterNotification({}) rc {}",
                label(g),
                rc.0
            );
            if rc.0 == 0 {
                d.setting_handles.push(h);
            }
        }
        if o.effective_mode {
            let mut h: *mut c_void = std::ptr::null_mut();
            let r = unsafe {
                PowerRegisterForEffectivePowerModeNotifications(
                    EFFECTIVE_POWER_MODE_V2,
                    Some(on_effective_mode),
                    None,
                    &mut h,
                )
            };
            crate::log!(
                "setup",
                "PowerRegisterForEffectivePowerModeNotifications {r:?}"
            );
            if r.is_ok() {
                d.effmode_handle = Some(h);
            }
        }
        if o.suspend_resume {
            let mut h: *mut c_void = std::ptr::null_mut();
            let rc = unsafe {
                PowerRegisterSuspendResumeNotification(
                    DEVICE_NOTIFY_CALLBACK,
                    leaked_params(Some(on_suspend_resume)),
                    &mut h,
                )
            };
            crate::log!(
                "setup",
                "PowerRegisterSuspendResumeNotification rc {}",
                rc.0
            );
            if rc.0 == 0 {
                d.suspend_handle = Some(h);
            }
        }
        if o.registry {
            let mut keys = vec![(sys::hklm(), sys::SCHEMES_KEY, "reg-hklm")];
            if o.full {
                keys.push((sys::hkcu(), sys::DESKTOP_KEY, "reg-hkcu"));
            }
            for (root, path, tag) in keys {
                let stop = stop.clone();
                let full = o.full;
                let root_raw = root.0 as usize;
                d.threads.push(std::thread::spawn(move || {
                    registry_loop(HKEY(root_raw as _), path, tag, full, &stop)
                }));
            }
        }
        if let Some(ms) = o.poll_ms {
            let stop = stop.clone();
            let full = o.full;
            d.threads.push(std::thread::spawn(move || {
                let mut last = snapshot(full);
                while !stop.load(Ordering::Relaxed) {
                    std::thread::sleep(Duration::from_millis(ms));
                    let now = snapshot(full);
                    let changes = diff(&last, &now);
                    if !changes.is_empty() {
                        push("poll", changes.join("; "));
                        last = now;
                    }
                }
            }));
        }
        d
    }

    pub fn stop(mut self) {
        self.stop.store(true, Ordering::Relaxed);
        for t in self.threads.drain(..) {
            let _ = t.join();
        }
        unsafe {
            for h in self.setting_handles.drain(..) {
                let _ = PowerSettingUnregisterNotification(HPOWERNOTIFY(h as isize));
            }
            if let Some(h) = self.effmode_handle.take() {
                let _ = PowerUnregisterFromEffectivePowerModeNotifications(h);
            }
            if let Some(h) = self.suspend_handle.take() {
                let _ = PowerUnregisterSuspendResumeNotification(HPOWERNOTIFY(h as isize));
            }
        }
    }
}

fn registry_loop(root: HKEY, path: &'static str, tag: &'static str, full: bool, stop: &AtomicBool) {
    let mut key = HKEY::default();
    let rc = unsafe {
        RegOpenKeyExW(
            root,
            &HSTRING::from(path),
            Some(0),
            KEY_NOTIFY | KEY_READ,
            &mut key,
        )
    };
    crate::log!("setup", "RegOpenKeyExW({path}) rc {}", rc.0);
    if rc.is_err() {
        return;
    }
    let Ok(event) = (unsafe { CreateEventW(None, false, false, None) }) else {
        return;
    };
    let mut last = snapshot(full);
    while !stop.load(Ordering::Relaxed) {
        let rc = unsafe {
            RegNotifyChangeKeyValue(
                key,
                true,
                REG_NOTIFY_CHANGE_NAME | REG_NOTIFY_CHANGE_LAST_SET,
                Some(event),
                true,
            )
        };
        if rc.is_err() {
            crate::log!(tag, "RegNotifyChangeKeyValue rc {}", rc.0);
            break;
        }
        loop {
            if stop.load(Ordering::Relaxed) {
                break;
            }
            if unsafe { WaitForSingleObject(event, 100) } == WAIT_OBJECT_0 {
                let now = snapshot(full);
                let changes = diff(&last, &now);
                push(
                    tag,
                    if changes.is_empty() {
                        "key changed, no watched value differs".into()
                    } else {
                        format!("key changed: {}", changes.join("; "))
                    },
                );
                last = now;
                break;
            }
        }
    }
    unsafe {
        let _ = CloseHandle(event);
        let _ = RegCloseKey(key);
    }
}

/// First event from each detector after `t0`, as ms after `t0`.
pub fn first_per_source(t0: Instant, until: Instant) -> BTreeMap<&'static str, (u128, String)> {
    let mut out = BTreeMap::new();
    for e in events_since(t0) {
        if e.at > until {
            break;
        }
        out.entry(e.source)
            .or_insert(((e.at - t0).as_millis(), e.text.clone()));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_lists_only_changes() {
        let a: Snapshot = [
            ("lid.ac".to_string(), "1".to_string()),
            ("x".into(), "y".into()),
        ]
        .into();
        let mut b = a.clone();
        b.insert("lid.ac".into(), "0".into());
        b.insert("new".into(), "1".into());
        assert_eq!(diff(&a, &b), vec!["lid.ac 1 -> 0", "new = 1"]);
        assert!(diff(&a, &a).is_empty());
    }
}
