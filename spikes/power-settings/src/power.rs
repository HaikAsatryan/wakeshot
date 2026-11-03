use windows::Win32::Foundation::{
    ERROR_NO_MORE_ITEMS, ERROR_SUCCESS, HLOCAL, LocalFree, WIN32_ERROR,
};
use windows::Win32::System::Power::*;
use windows::Win32::System::Registry::{HKEY, KEY_READ, KEY_WRITE, REG_SAM_FLAGS};
use windows::core::GUID;

pub const SCHEME_BALANCED: GUID = GUID::from_u128(0x381b4222_f694_41f0_9685_ff5bb260df2e);

pub const SUB_BUTTONS: GUID = GUID::from_u128(0x4f971e89_eebd_4455_a8de_9e59040e7347);
pub const LIDACTION: GUID = GUID::from_u128(0x5ca83367_6e45_459f_a27b_476b1d01c936);
pub const SUB_SLEEP: GUID = GUID::from_u128(0x238c9fa8_0aad_41ed_83f4_97be242c8f20);
pub const STANDBYIDLE: GUID = GUID::from_u128(0x29f6c1db_86da_48c5_9fdb_f2b67b1f44da);
pub const SUB_ENERGYSAVER: GUID = GUID::from_u128(0xde830923_a562_41af_a086_e3a2c6bad2da);
pub const ESBATTTHRESHOLD: GUID = GUID::from_u128(0xe69653ca_cf7f_4f05_aa73_cb833fa90ad4);
pub const SUB_NONE: GUID = GUID::from_u128(0xfea3413e_7e05_4911_9a71_700331f1c294);
pub const CONSOLELOCK: GUID = GUID::from_u128(0x0e796bdb_100d_47d6_a2d5_f7d2daa51f51);
pub const SUB_VIDEO: GUID = GUID::from_u128(0x7516b95f_f776_4464_8c53_06167f40cc99);
pub const VIDEOIDLE: GUID = GUID::from_u128(0x3c0bc021_c8a8_4e07_a973_6b14cbcb2b7e);

// Notification-only GUIDs (winnt.h).
pub const GUID_ACTIVE_POWERSCHEME: GUID = GUID::from_u128(0x31f9f286_5084_42fe_b720_2b0264993763);
pub const GUID_ACDC_POWER_SOURCE: GUID = GUID::from_u128(0x5d3e9a59_e9d5_4b00_a6bd_ff34ff516548);
pub const GUID_CONSOLE_DISPLAY_STATE: GUID =
    GUID::from_u128(0x6fe69556_704a_47a0_8f24_c28d936fda47);
pub const GUID_ENERGY_SAVER_STATUS: GUID = GUID::from_u128(0x550e8400_e29b_41d4_a716_446655440000);
pub const GUID_POWERSCHEME_PERSONALITY: GUID =
    GUID::from_u128(0x245d8541_3943_4422_b025_13a784f679b7);

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
    pub fn parse(s: &str) -> Option<Source> {
        match s {
            "ac" => Some(Source::Ac),
            "dc" => Some(Source::Dc),
            _ => None,
        }
    }
}

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
pub const ENERGY_SAVER_THRESHOLD: Setting = Setting {
    name: "energy-saver-threshold",
    sub: SUB_ENERGYSAVER,
    guid: ESBATTTHRESHOLD,
};
pub const SIGN_IN_AFTER_SLEEP: Setting = Setting {
    name: "consolelock",
    sub: SUB_NONE,
    guid: CONSOLELOCK,
};
pub const DISPLAY_OFF: Setting = Setting {
    name: "display-off",
    sub: SUB_VIDEO,
    guid: VIDEOIDLE,
};

pub fn setting_by_name(name: &str) -> Option<Setting> {
    [
        LID,
        SLEEP,
        ENERGY_SAVER_THRESHOLD,
        SIGN_IN_AFTER_SLEEP,
        DISPLAY_OFF,
    ]
    .into_iter()
    .find(|s| s.name == name)
}

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

pub fn err_text(code: u32) -> String {
    match code {
        0 => "0 ERROR_SUCCESS".into(),
        5 => "5 ERROR_ACCESS_DENIED".into(),
        2 => "2 ERROR_FILE_NOT_FOUND".into(),
        87 => "87 ERROR_INVALID_PARAMETER".into(),
        1260 => "1260 ERROR_ACCESS_DISABLED_BY_POLICY".into(),
        other => format!("{other}"),
    }
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

pub fn access_check(accessor: POWER_DATA_ACCESSOR, guid: Option<&GUID>, sam: REG_SAM_FLAGS) -> u32 {
    let g = guid.map(|g| g as *const GUID);
    unsafe { PowerSettingAccessCheckEx(accessor, g, sam).0 }
}

pub fn access_check_setting(s: Setting, src: Source, write: bool) -> u32 {
    let accessor = match src {
        Source::Ac => ACCESS_AC_POWER_SETTING_INDEX,
        Source::Dc => ACCESS_DC_POWER_SETTING_INDEX,
    };
    access_check(
        accessor,
        Some(&s.guid),
        if write { KEY_WRITE } else { KEY_READ },
    )
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

type OverlayFn = unsafe extern "system" fn(*mut GUID) -> u32;

/// Undocumented powrprof exports behind the Windows 11 Power mode slider.
pub fn overlay(name: &str) -> String {
    use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
    use windows::core::{PCSTR, w};
    unsafe {
        let Ok(dll) = LoadLibraryW(w!("powrprof.dll")) else {
            return "<no powrprof>".into();
        };
        let cname = format!("{name}\0");
        let Some(f) = GetProcAddress(dll, PCSTR(cname.as_ptr())) else {
            return "<not exported>".into();
        };
        let f: OverlayFn = std::mem::transmute(f);
        let mut g = GUID::zeroed();
        let rc = f(&mut g);
        if rc == 0 {
            overlay_name(&g)
        } else {
            format!("<rc {rc}>")
        }
    }
}

type SetOverlayFn = unsafe extern "system" fn(GUID) -> u32;

/// Undocumented powrprof export the Power mode slider uses. Only for putting the user's mode back.
pub fn set_overlay(g: GUID) -> String {
    use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
    use windows::core::{s, w};
    unsafe {
        let Ok(dll) = LoadLibraryW(w!("powrprof.dll")) else {
            return "<no powrprof>".into();
        };
        let Some(f) = GetProcAddress(dll, s!("PowerSetActiveOverlayScheme")) else {
            return "<not exported>".into();
        };
        let f: SetOverlayFn = std::mem::transmute(f);
        format!("rc {}", f(g))
    }
}

pub fn overlay_name(g: &GUID) -> String {
    let s = guid_str(g);
    // Labels come from community lists of overlay GUIDs, not from Microsoft docs.
    let label = match s.as_str() {
        "00000000-0000-0000-0000-000000000000" => "balanced, no overlay?",
        "961cc777-2547-4f9d-8174-7d86181b8a7a" => "best power efficiency?",
        "3af9b8d9-7c97-431d-ad78-34a8bfea439f" => "better battery?",
        "ded574b5-45a0-4f42-8737-46345c09c238" => "best performance?",
        _ => "unknown",
    };
    format!("{s} ({label})")
}

pub fn effective_mode_name(m: i32) -> &'static str {
    match m {
        0 => "BatterySaver",
        1 => "BetterBattery",
        2 => "Balanced",
        3 => "HighPerformance",
        4 => "MaxPerformance",
        5 => "GameMode",
        6 => "MixedReality",
        _ => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guid_round_trip() {
        let s = guid_str(&SCHEME_BALANCED);
        assert_eq!(s, "381b4222-f694-41f0-9685-ff5bb260df2e");
        assert_eq!(parse_guid(&s), Some(SCHEME_BALANCED));
        assert_eq!(
            parse_guid("{381B4222-F694-41F0-9685-FF5BB260DF2E}"),
            Some(SCHEME_BALANCED)
        );
        assert_eq!(parse_guid("nope"), None);
    }

    #[test]
    fn settings_by_name() {
        assert_eq!(setting_by_name("lid"), Some(LID));
        assert_eq!(setting_by_name("sleep"), Some(SLEEP));
        assert_eq!(setting_by_name("x"), None);
    }
}
