use std::ffi::c_void;

use windows::{
    Win32::{
        Foundation::{LPARAM, RECT},
        Graphics::Gdi::{EnumDisplayMonitors, HDC, HMONITOR},
        System::{
            LibraryLoader::{GetProcAddress, LOAD_LIBRARY_SEARCH_SYSTEM32, LoadLibraryExW},
            Registry::{
                HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, RRF_RT_REG_DWORD, RRF_RT_REG_SZ,
                RegGetValueW,
            },
        },
        UI::{
            Accessibility::{HCF_HIGHCONTRASTON, HIGHCONTRASTW},
            HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI},
            WindowsAndMessaging::{
                SPI_GETHIGHCONTRAST, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SystemParametersInfoW,
            },
        },
    },
    core::{BOOL, PCSTR, PCWSTR, w},
};

/// `PreferredAppMode` from uxtheme, undocumented. Values from ysc3839/win32-darkmode (1903+).
#[derive(Clone, Copy, Debug)]
#[repr(i32)]
#[allow(dead_code)]
pub enum AppMode {
    Default = 0,
    AllowDark = 1,
    ForceDark = 2,
    ForceLight = 3,
}

type SetPreferredAppModeFn = unsafe extern "system" fn(i32) -> i32;
type VoidFn = unsafe extern "system" fn();
type BoolFn = unsafe extern "system" fn() -> bool;

/// Undocumented uxtheme exports, loaded by ordinal. Any of them may be missing on a future build,
/// so every call is optional and the menu simply stays light.
#[derive(Clone, Copy)]
pub struct UxTheme {
    set_preferred_app_mode: Option<SetPreferredAppModeFn>,
    flush_menu_themes: Option<VoidFn>,
    refresh_policy: Option<VoidFn>,
    should_apps_use_dark: Option<BoolFn>,
}

impl UxTheme {
    pub fn load() -> Self {
        unsafe {
            let Ok(dll) = LoadLibraryExW(w!("uxtheme.dll"), None, LOAD_LIBRARY_SEARCH_SYSTEM32)
            else {
                return Self {
                    set_preferred_app_mode: None,
                    flush_menu_themes: None,
                    refresh_policy: None,
                    should_apps_use_dark: None,
                };
            };
            let by_ordinal = |n: usize| GetProcAddress(dll, PCSTR(n as *const u8));
            Self {
                set_preferred_app_mode: by_ordinal(135).map(|f| std::mem::transmute(f)),
                flush_menu_themes: by_ordinal(136).map(|f| std::mem::transmute(f)),
                refresh_policy: by_ordinal(104).map(|f| std::mem::transmute(f)),
                should_apps_use_dark: by_ordinal(132).map(|f| std::mem::transmute(f)),
            }
        }
    }

    pub fn found(&self) -> String {
        format!(
            "SetPreferredAppMode(135)={} FlushMenuThemes(136)={} RefreshImmersiveColorPolicyState(104)={} ShouldAppsUseDarkMode(132)={}",
            self.set_preferred_app_mode.is_some(),
            self.flush_menu_themes.is_some(),
            self.refresh_policy.is_some(),
            self.should_apps_use_dark.is_some()
        )
    }

    /// Returns the previous mode as uxtheme reports it.
    pub fn set_mode(&self, mode: AppMode) -> Option<i32> {
        self.set_preferred_app_mode
            .map(|f| unsafe { f(mode as i32) })
    }

    pub fn flush(&self) {
        if let Some(f) = self.refresh_policy {
            unsafe { f() };
        }
        if let Some(f) = self.flush_menu_themes {
            unsafe { f() };
        }
    }

    pub fn apps_dark(&self) -> Option<bool> {
        self.should_apps_use_dark.map(|f| unsafe { f() })
    }
}

const PERSONALIZE: PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize");

pub fn reg_dword(root_hklm: bool, key: PCWSTR, value: PCWSTR) -> Option<u32> {
    let mut data = 0u32;
    let mut size = 4u32;
    let root = if root_hklm {
        HKEY_LOCAL_MACHINE
    } else {
        HKEY_CURRENT_USER
    };
    let r = unsafe {
        RegGetValueW(
            root,
            key,
            value,
            RRF_RT_REG_DWORD,
            None,
            Some(&mut data as *mut u32 as *mut c_void),
            Some(&mut size),
        )
    };
    r.is_ok().then_some(data)
}

pub fn reg_string(root_hklm: bool, key: PCWSTR, value: PCWSTR) -> Option<String> {
    let mut buf = [0u16; 1024];
    let mut size = (buf.len() * 2) as u32;
    let root = if root_hklm {
        HKEY_LOCAL_MACHINE
    } else {
        HKEY_CURRENT_USER
    };
    let r = unsafe {
        RegGetValueW(
            root,
            key,
            value,
            RRF_RT_REG_SZ | windows::Win32::System::Registry::RRF_RT_REG_EXPAND_SZ,
            None,
            Some(buf.as_mut_ptr() as *mut c_void),
            Some(&mut size),
        )
    };
    if r.is_err() {
        return None;
    }
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    Some(String::from_utf16_lossy(&buf[..len]))
}

/// `AppsUseLightTheme`: drives menus and windows.
pub fn apps_light() -> Option<bool> {
    reg_dword(false, PERSONALIZE, w!("AppsUseLightTheme")).map(|v| v != 0)
}

/// `SystemUsesLightTheme`: drives the taskbar, so it picks the tray glyph colour.
pub fn taskbar_light() -> Option<bool> {
    reg_dword(false, PERSONALIZE, w!("SystemUsesLightTheme")).map(|v| v != 0)
}

pub fn high_contrast() -> Option<bool> {
    let mut hc = HIGHCONTRASTW {
        cbSize: size_of::<HIGHCONTRASTW>() as u32,
        ..Default::default()
    };
    unsafe {
        SystemParametersInfoW(
            SPI_GETHIGHCONTRAST,
            hc.cbSize,
            Some(&mut hc as *mut HIGHCONTRASTW as *mut c_void),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
        .ok()?;
    }
    Some(hc.dwFlags.contains(HCF_HIGHCONTRASTON))
}

pub fn os_build() -> String {
    let key = w!("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion");
    format!(
        "{} build {}.{}",
        reg_string(true, key, w!("DisplayVersion")).unwrap_or_default(),
        reg_string(true, key, w!("CurrentBuild")).unwrap_or_default(),
        reg_dword(true, key, w!("UBR")).unwrap_or_default()
    )
}

/// (monitor rect, effective DPI) for every monitor.
pub fn monitors() -> Vec<(RECT, u32)> {
    unsafe extern "system" fn cb(m: HMONITOR, _: HDC, r: *mut RECT, data: LPARAM) -> BOOL {
        let list = unsafe { &mut *(data.0 as *mut Vec<(RECT, u32)>) };
        let (mut x, mut y) = (0u32, 0u32);
        let dpi = unsafe { GetDpiForMonitor(m, MDT_EFFECTIVE_DPI, &mut x, &mut y) }
            .map(|_| x)
            .unwrap_or(0);
        list.push((unsafe { *r }, dpi));
        BOOL(1)
    }
    let mut list: Vec<(RECT, u32)> = Vec::new();
    unsafe {
        let _ = EnumDisplayMonitors(
            None,
            None,
            Some(cb),
            LPARAM(&mut list as *mut Vec<(RECT, u32)> as isize),
        );
    }
    list
}
